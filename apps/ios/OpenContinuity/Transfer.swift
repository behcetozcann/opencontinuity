import Foundation

struct Transfer: Identifiable {
    enum Direction { case incoming, outgoing }

    let id = UUID()
    let direction: Direction
    let content: ContentKindRecord
    let bytes: UInt64
    let items: UInt64
    /// Gelen aktarımda gönderen cihaz.
    let peer: String?
    let date = Date()
    var savedTo: URL?

    var contentTitle: String {
        switch content {
        case .text: return "Metin"
        case .image: return "Görsel"
        case .files: return items == 1 ? "1 dosya" : "\(items) öğe"
        }
    }

    static func format(_ bytes: UInt64) -> String {
        ByteCountFormatter.string(fromByteCount: Int64(bytes), countStyle: .file)
    }
}

extension ContentKindRecord {
    var label: String {
        switch self {
        case .text: return "metin"
        case .image: return "görsel"
        case .files: return "dosya"
        }
    }

    var symbol: String {
        switch self {
        case .text: return "text.alignleft"
        case .image: return "photo"
        case .files: return "doc"
        }
    }
}
