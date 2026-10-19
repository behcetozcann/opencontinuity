import SwiftUI

struct TransferRow: View {
    let transfer: Transfer
    let inProgress: Bool

    var body: some View {
        HStack(spacing: 12) {
            ZStack(alignment: .bottomTrailing) {
                Image(systemName: transfer.content.symbol)
                    .font(.title3)
                    .frame(width: 32, height: 32)
                Image(systemName: transfer.direction == .incoming ? "arrow.down.circle.fill" : "arrow.up.circle.fill")
                    .font(.caption)
                    .foregroundStyle(transfer.direction == .incoming ? Color.blue : Color.green)
                    .background(Circle().fill(Color(.systemBackground)))
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.subheadline)
                Text(transfer.summary).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if inProgress {
                ProgressView()
            } else {
                Text(transfer.date, style: .time).font(.caption).foregroundStyle(.secondary)
            }
        }
        .contextMenu {
            if let folder = transfer.savedTo, let url = filesAppURL(folder) {
                Link(destination: url) { Label("Dosyalar'da göster", systemImage: "folder") }
            }
        }
    }

    private var title: String {
        switch (transfer.direction, inProgress) {
        case (.incoming, true): return "\(transfer.peer ?? "Eş") cihazından alınıyor"
        case (.outgoing, true): return "Gönderiliyor"
        case (.incoming, false): return "\(transfer.peer ?? "Eş") cihazından geldi"
        case (.outgoing, false): return "Gönderildi"
        }
    }

    /// Dosyalar uygulamasında klasörü açan adres (`shareddocuments://`).
    private func filesAppURL(_ folder: URL) -> URL? {
        var components = URLComponents(url: folder, resolvingAgainstBaseURL: false)
        components?.scheme = "shareddocuments"
        return components?.url
    }
}
