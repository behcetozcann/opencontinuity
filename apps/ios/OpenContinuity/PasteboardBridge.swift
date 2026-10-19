import UIKit
import UniformTypeIdentifiers

/// Bilgisayardan gelen metin ve görseli panoya yazar. iPhone'daki kopyalar
/// gönderilmez: Mac'e Apple'ın Evrensel Pano'su zaten taşıyor.
@MainActor
enum PasteboardBridge {
    static func write(_ clip: ClipRecord) {
        let item: [String: Any]
        switch clip {
        case let .text(text):
            item = [UTType.utf8PlainText.identifier: text]
        case let .png(data):
            item = [UTType.png.identifier: data]
        case .files:
            // Dosyalar Belgeler'e kaydedildi; panoya dosya yazılmaz.
            return
        }
        // Evrensel Pano gelen içeriği Mac'e taşımasın; taşırsa Mac onu yeni
        // kopya sanıp gruba yeniden gönderir.
        UIPasteboard.general.setItems([item], options: [.localOnly: true])
    }
}
