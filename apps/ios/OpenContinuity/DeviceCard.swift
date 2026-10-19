import SwiftUI

/// Eşleşmiş bilgisayarın durumu: bağlı, bağlanıyor ya da hata.
struct DeviceCard: View {
    let peer: PeerRecord
    /// Bu cihazla süren aktarım varsa yönü.
    let activeDirection: Transfer.Direction?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                Image(systemName: "desktopcomputer")
                    .font(.system(size: 26, weight: .regular))
                    .foregroundStyle(peer.connected ? Color.accentColor : Color.secondary)
                Spacer()
                if let activeDirection {
                    Image(systemName: activeDirection == .incoming ? "arrow.down" : "arrow.up")
                        .font(.footnote.weight(.bold))
                        .foregroundStyle(.white)
                        .frame(width: 24, height: 24)
                        .background(Circle().fill(Color.accentColor))
                        .symbolEffect(.pulse, options: .repeating)
                        .transition(.scale.combined(with: .opacity))
                }
            }
            VStack(alignment: .leading, spacing: 4) {
                Text(peer.name)
                    .font(.headline)
                    .lineLimit(2)
                    .minimumScaleFactor(0.85)
                StatusLabel(state: state)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, minHeight: 124, alignment: .topLeading)
        .background(
            RoundedRectangle(cornerRadius: 22, style: .continuous)
                .fill(peer.connected ? Color.accentColor.opacity(0.14) : Color(.secondarySystemBackground))
        )
        .animation(.smooth, value: peer.connected)
        .animation(.smooth, value: activeDirection)
        .accessibilityElement(children: .combine)
    }

    private var state: StatusLabel.State {
        if peer.connected { return .connected }
        if peer.linkError != nil { return .unreachable }
        return .connecting
    }
}

private struct StatusLabel: View {
    enum State {
        case connected, connecting, unreachable
    }

    let state: State

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(color)
                .frame(width: 7, height: 7)
            Text(text)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
    }

    private var color: Color {
        switch state {
        case .connected: return .green
        case .connecting: return .orange
        case .unreachable: return .red
        }
    }

    private var text: String {
        switch state {
        case .connected: return "Bağlı"
        case .connecting: return "Bağlanıyor…"
        case .unreachable: return "Ulaşılamıyor"
        }
    }
}

/// Izgaradaki son kart: yeni cihaz eşleştirmeyi açar.
struct AddDeviceCard: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: "plus")
                    .font(.system(size: 22, weight: .medium))
                Text("Cihaz ekle")
                    .font(.subheadline.weight(.medium))
            }
            .foregroundStyle(Color.accentColor)
            .frame(maxWidth: .infinity, minHeight: 124)
            .background(
                RoundedRectangle(cornerRadius: 22, style: .continuous)
                    .strokeBorder(Color.accentColor.opacity(0.35), style: StrokeStyle(lineWidth: 1.5, dash: [6, 5]))
            )
        }
        .buttonStyle(.plain)
    }
}
