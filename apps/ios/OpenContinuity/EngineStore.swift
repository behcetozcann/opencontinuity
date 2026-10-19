import Foundation

/// Motorun durumunu arayüze taşır; motoru, Bonjour'u ve pano köprüsünü yönetir.
@MainActor
final class EngineStore: ObservableObject {
    @Published private(set) var snapshot: SnapshotRecord?
    @Published private(set) var transfers: [Transfer] = []
    /// Süren görsel/dosya aktarımı.
    @Published private(set) var active: Transfer?
    @Published var notice: String?
    @Published private(set) var pairingFailure: String?

    private(set) var bonjour: Bonjour?
    private var engine: OcEngine?
    private var stopping = false
    private var startWhenStopped = false
    #if DEBUG
    private var debugPushed = false
    #endif

    var peers: [PeerRecord] { snapshot?.peers ?? [] }
    var connectedCount: Int { peers.filter(\.connected).count }

    func start() {
        guard engine == nil else { return }
        if stopping {
            startWhenStopped = true
            return
        }
        let configDir = Paths.config.path
        guard let fingerprint = deviceFingerprint(configDir: configDir, deviceName: Paths.deviceName) else {
            notice = "Cihaz kimliği oluşturulamadı"
            return
        }
        let bridge = ListenerBridge(store: self)
        let config = StartConfig(configDir: configDir, filesDir: Paths.incoming.path, deviceName: Paths.deviceName, port: Paths.port)
        let engine = OcEngine.start(config: config, listener: bridge)
        self.engine = engine
        bonjour = Bonjour(name: Paths.deviceName, fingerprint: fingerprint, port: Paths.port) { found in
            engine.bonjourFound(name: found.name, fingerprintHex: found.fingerprint, pairing: found.pairing, addresses: found.addresses)
        }
        bonjour?.start()
        #if DEBUG
        // Simülatör testi: `simctl launch … -ocAutoPair YES` (kod kendiliğinden onaylanır).
        if UserDefaults.standard.bool(forKey: "ocAutoPair") { startPairing() }
        #endif
    }

    func stop() {
        guard let engine else { return }
        bonjour?.stop()
        bonjour = nil
        engine.shutdown()
        self.engine = nil
        stopping = true
        active = nil
    }

    // MARK: Komutlar

    func startPairing(address: String? = nil) {
        pairingFailure = nil
        engine?.startPairing(address: address)
    }

    func confirmCode(_ accepted: Bool) { engine?.confirmCode(accepted: accepted) }
    func cancelPairing() { engine?.cancelPairing() }
    func unpair(_ peer: PeerRecord) { engine?.unpair(fingerprintPrefix: peer.fingerprintPrefix) }

    private func push(_ clip: ClipRecord) {
        guard let engine else { return }
        if !engine.pushClip(clip: clip) {
            notice = "Önceki aktarım sürüyor, biraz sonra tekrar deneyin"
        }
    }

    /// Seçilen dosyaları geçici klasöre kopyalayıp gönderir.
    func send(files urls: [URL]) {
        sendBatch { batch in
            try urls.map { url in
                let scoped = url.startAccessingSecurityScopedResource()
                defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                let target = batch.appendingPathComponent(url.lastPathComponent)
                try FileManager.default.copyItem(at: url, to: target)
                return target.path
            }
        }
    }

    /// Fotoğraflar'dan seçilenleri (uyumlu biçimde, HEIC yerine JPEG) dosya olarak gönderir.
    func send(photos: [PhotoData]) {
        sendBatch { batch in
            try photos.enumerated().map { index, photo in
                let target = batch.appendingPathComponent("Fotoğraf \(index + 1).\(photo.fileExtension)")
                try photo.data.write(to: target)
                return target.path
            }
        }
    }

    private func sendBatch(_ prepare: (URL) throws -> [String]) {
        let batch = Paths.outgoing.appendingPathComponent(UUID().uuidString, isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: batch, withIntermediateDirectories: true)
            push(.files(paths: try prepare(batch)))
        } catch {
            notice = "Dosya hazırlanamadı: \(error.localizedDescription)"
        }
    }

    // MARK: Motordan gelenler

    func apply(state: SnapshotRecord) {
        snapshot = state
        #if DEBUG
        if UserDefaults.standard.bool(forKey: "ocAutoPair"), case .confirm = state.pairing { confirmCode(true) }
        // `-ocPushText <metin>`: bağlanınca bir kez gönderir (simülatörde pano okuma izni verilemiyor).
        if let text = UserDefaults.standard.string(forKey: "ocPushText"), !debugPushed, state.peers.contains(where: \.connected) {
            debugPushed = true
            push(.text(text: text))
        }
        #endif
    }

    func apply(event: EventRecord) {
        switch event {
        case let .sending(content, bytes, items):
            active = Transfer(direction: .outgoing, content: content, bytes: bytes, items: items, peer: nil)
        case let .receiving(content, bytes, items, from):
            active = Transfer(direction: .incoming, content: content, bytes: bytes, items: items, peer: from)
        case let .sent(content, bytes, items):
            active = nil
            record(Transfer(direction: .outgoing, content: content, bytes: bytes, items: items, peer: nil))
        case let .received(content, bytes, items, from, savedTo):
            active = nil
            var transfer = Transfer(direction: .incoming, content: content, bytes: bytes, items: items, peer: from)
            transfer.savedTo = savedTo.map { URL(fileURLWithPath: $0) }
            record(transfer)
            notice = content == .files ? "\(from) cihazından \(items) öğe geldi (Dosyalar › OpenContinuity)" : "\(from) cihazından \(content.label) panoya kopyalandı"
        case let .tooLarge(content, bytes, limit):
            notice = "\(content.label.capitalized) çok büyük: \(Transfer.format(bytes)) (sınır \(Transfer.format(limit)))"
        case let .disconnected(_, reason):
            if active != nil { notice = "Aktarım yarıda kaldı: \(reason)" }
            active = nil
        case let .pairingFailed(reason):
            pairingFailure = reason
        case let .paired(peer):
            notice = "\(peer) ile eşleşildi"
        case let .peerAdded(peer, via):
            notice = "\(via) gruba \(peer) cihazını ekledi"
        case .skipped, .diagnostic:
            break
        }
    }

    func engineStopped(error: String?) {
        stopping = false
        if let error, engine != nil {
            notice = "Senkron durdu: \(error)"
            engine = nil
        }
        if startWhenStopped {
            startWhenStopped = false
            start()
        }
    }

    private func record(_ transfer: Transfer) {
        transfers.insert(transfer, at: 0)
        if transfers.count > 20 { transfers.removeLast() }
    }
}
