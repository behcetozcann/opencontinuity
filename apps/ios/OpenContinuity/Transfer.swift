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

    var summary: String {
        switch content {
        case .text: return "Metin · \(Transfer.format(bytes))"
        case .image: return "Görsel · \(Transfer.format(bytes))"
        case .files: return "\(items) öğe · \(Transfer.format(bytes))"
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
