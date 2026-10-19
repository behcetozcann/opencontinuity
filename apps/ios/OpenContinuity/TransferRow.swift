import SwiftUI

struct TransferRow: View {
    let transfer: Transfer
    let inProgress: Bool

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: transfer.content.symbol)
                .font(.body.weight(.medium))
                .foregroundStyle(tint)
                .frame(width: 40, height: 40)
                .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(tint.opacity(0.12)))
                .overlay(alignment: .bottomTrailing) {
                    Image(systemName: transfer.direction == .incoming ? "arrow.down.circle.fill" : "arrow.up.circle.fill")
                        .font(.caption)
                        .foregroundStyle(tint)
                        .background(Circle().fill(Color(.systemBackground)))
                        .offset(x: 4, y: 4)
                }
            VStack(alignment: .leading, spacing: 2) {
                Text(transfer.contentTitle).font(.subheadline.weight(.medium))
                Text(detail).font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer()
            if inProgress {
                ProgressView()
            } else {
                Text(transfer.date, style: .time).font(.caption).foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .contextMenu {
            if let folder = transfer.savedTo, let url = filesAppURL(folder) {
                Link(destination: url) { Label("Dosyalar'da göster", systemImage: "folder") }
            }
        }
    }

    private var tint: Color {
        transfer.direction == .incoming ? .accentColor : .green
    }

    private var detail: String {
        let size = Transfer.format(transfer.bytes)
        switch (transfer.direction, inProgress) {
        case (.incoming, true): return "\(transfer.peer ?? "Eş") cihazından alınıyor · \(size)"
        case (.outgoing, true): return "Gönderiliyor · \(size)"
        case (.incoming, false): return "\(transfer.peer ?? "Eş") · \(size)"
        case (.outgoing, false): return "Gönderildi · \(size)"
        }
    }

    /// Dosyalar uygulamasında klasörü açan adres (`shareddocuments://`).
    private func filesAppURL(_ folder: URL) -> URL? {
        var components = URLComponents(url: folder, resolvingAgainstBaseURL: false)
        components?.scheme = "shareddocuments"
        return components?.url
    }
}
