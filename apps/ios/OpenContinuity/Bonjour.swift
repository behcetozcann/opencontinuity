import Foundation

/// `_opencontinuity._tcp` yayını ve araması. Masaüstündeki mDNS ile aynı TXT
/// kayıtları: `fp` (parmak izi hex), `name`, `pair` (eşleştirme modu).
/// Güven her zaman TLS'teki parmak izine dayanır; burası yalnız adres bulur.
@MainActor
final class Bonjour: NSObject {
    struct Found {
        let name: String
        let fingerprint: String
        let pairing: Bool
        /// "ip:port"
        let addresses: [String]
    }

    private static let type = "_opencontinuity._tcp."
    private let name: String
    private let fingerprint: String
    private let port: UInt16
    private let onFound: (Found) -> Void
    private var published: NetService?
    private let browser = NetServiceBrowser()
    private var resolving: Set<NetService> = []
    private var pairing = false

    init(name: String, fingerprint: String, port: UInt16, onFound: @escaping (Found) -> Void) {
        self.name = name
        self.fingerprint = fingerprint
        self.port = port
        self.onFound = onFound
        super.init()
        browser.delegate = self
    }

    func start() {
        publish()
        browser.searchForServices(ofType: Self.type, inDomain: "local.")
    }

    func stop() {
        browser.stop()
        published?.stop()
        published = nil
        resolving.forEach { $0.stop() }
        resolving.removeAll()
    }

    func setPairing(_ on: Bool) {
        pairing = on
        published?.setTXTRecord(txtRecord())
    }

    private func publish() {
        let service = NetService(domain: "local.", type: Self.type, name: "\(name) \(fingerprint.prefix(8))", port: Int32(port))
        service.setTXTRecord(txtRecord())
        service.publish()
        published = service
    }

    private func txtRecord() -> Data {
        NetService.data(fromTXTRecord: [
            "fp": Data(fingerprint.utf8),
            "name": Data(name.utf8),
            "pair": Data((pairing ? "1" : "0").utf8),
        ])
    }

    fileprivate func resolved(_ service: NetService) {
        guard let txt = service.txtRecordData() else { return }
        let fields = NetService.dictionary(fromTXTRecord: txt).mapValues { String(decoding: $0, as: UTF8.self) }
        guard let fp = fields["fp"], fp != fingerprint else { return }
        let addresses = (service.addresses ?? []).compactMap { Self.ipv4($0) }.map { "\($0):\(service.port)" }
        guard !addresses.isEmpty else { return }
        onFound(Found(name: fields["name"] ?? service.name, fingerprint: fp, pairing: fields["pair"] == "1", addresses: addresses))
    }

    /// Masaüstü yalnız IPv4 yayınlıyor ve kullanıyor.
    private static func ipv4(_ data: Data) -> String? {
        data.withUnsafeBytes { raw -> String? in
            guard let base = raw.baseAddress, raw.count >= MemoryLayout<sockaddr_in>.size else { return nil }
            let family = base.assumingMemoryBound(to: sockaddr.self).pointee.sa_family
            guard family == sa_family_t(AF_INET) else { return nil }
            var address = base.assumingMemoryBound(to: sockaddr_in.self).pointee.sin_addr
            var buffer = [CChar](repeating: 0, count: Int(INET_ADDRSTRLEN))
            guard inet_ntop(AF_INET, &address, &buffer, socklen_t(INET_ADDRSTRLEN)) != nil else { return nil }
            let text = String(cString: buffer)
            return text.hasPrefix("169.254.") || text.hasPrefix("127.") ? nil : text
        }
    }
}

extension Bonjour: NetServiceBrowserDelegate, NetServiceDelegate {
    nonisolated func netServiceBrowser(_ browser: NetServiceBrowser, didFind service: NetService, moreComing: Bool) {
        MainActor.assumeIsolated {
            service.delegate = self
            resolving.insert(service)
            service.resolve(withTimeout: 5)
            // TXT değişimi (eşleştirme modu açılıp kapanınca) için izlemeye devam.
            service.startMonitoring()
        }
    }

    nonisolated func netServiceBrowser(_ browser: NetServiceBrowser, didRemove service: NetService, moreComing: Bool) {
        MainActor.assumeIsolated {
            service.stop()
            resolving.remove(service)
        }
    }

    nonisolated func netServiceDidResolveAddress(_ sender: NetService) {
        MainActor.assumeIsolated { resolved(sender) }
    }

    nonisolated func netService(_ sender: NetService, didUpdateTXTRecord data: Data) {
        MainActor.assumeIsolated { resolved(sender) }
    }
}
