import UIKit
import UniformTypeIdentifiers

/// Uygulama öndeyken panoyu izler. iOS arka planda panoya erişim vermez;
/// uygulamaya dönülünce son kopya gönderilir (iOS bir kez "Yapıştır" izni sorar).
@MainActor
final class PasteboardBridge {
    var onCopy: ((ClipRecord) -> Void)?

    private var timer: Timer?
    private let seenKey = "lastSeenChangeCount"

    /// Son işlenen sayaç; uygulama kapanıp açılınca aynı kopya tekrar gitmesin.
    private var lastSeen: Int {
        get { UserDefaults.standard.integer(forKey: seenKey) }
        set { UserDefaults.standard.set(newValue, forKey: seenKey) }
    }

    func start() {
        check()
        timer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.check() }
        }
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func write(_ clip: ClipRecord) {
        let pasteboard = UIPasteboard.general
        switch clip {
        case let .text(text):
            pasteboard.string = text
        case let .png(data):
            pasteboard.setData(data, forPasteboardType: UTType.png.identifier)
        case .files:
            // Dosyalar Belgeler'e kaydedildi; panoya dosya yazılmaz.
            return
        }
        lastSeen = pasteboard.changeCount
    }

    private func check() {
        let pasteboard = UIPasteboard.general
        guard pasteboard.changeCount != lastSeen else { return }
        lastSeen = pasteboard.changeCount
        // has* sorguları izin uyarısı göstermez; yalnız içerik varsa okunur.
        if pasteboard.hasStrings, let text = pasteboard.string, !text.isEmpty {
            onCopy?(.text(text: text))
        } else if pasteboard.hasImages, let png = Self.png(from: pasteboard) {
            onCopy?(.png(data: png))
        }
    }

    private static func png(from pasteboard: UIPasteboard) -> Data? {
        if let data = pasteboard.data(forPasteboardType: UTType.png.identifier) {
            return data
        }
        return pasteboard.image?.pngData()
    }
}
