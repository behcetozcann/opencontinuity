import PhotosUI
import SwiftUI

struct HomeView: View {
    @EnvironmentObject private var store: EngineStore
    @State private var pairingShown = false
    @State private var devicesShown = false
    @State private var importerShown = false
    @State private var photosShown = false
    @State private var photoSelection: [PhotosPickerItem] = []

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    if store.peers.isEmpty {
                        PairPrompt { pairingShown = true }
                    } else {
                        DeviceGrid(pairingShown: $pairingShown)
                        SendActions(connected: store.connectedCount, photos: { photosShown = true }, files: { importerShown = true })
                    }
                    ActivityList()
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 12)
            }
            .background(Color(.systemBackground))
            .navigationTitle("OpenContinuity")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { devicesShown = true } label: { Image(systemName: "gearshape") }
                        .accessibilityLabel("Ayarlar")
                }
            }
            .sheet(isPresented: $pairingShown) { PairingView() }
            .sheet(isPresented: $devicesShown) { DevicesView(pairingShown: $pairingShown) }
            .fileImporter(isPresented: $importerShown, allowedContentTypes: [.item, .folder], allowsMultipleSelection: true) { result in
                if case let .success(urls) = result { store.send(files: urls) }
            }
            .photosPicker(isPresented: $photosShown, selection: $photoSelection, matching: .images, preferredItemEncoding: .compatible)
            .onChange(of: photoSelection) { _, items in
                guard !items.isEmpty else { return }
                photoSelection = []
                Task { store.send(photos: await PhotoData.load(items)) }
            }
            .overlay(alignment: .bottom) { NoticeBanner() }
        }
    }
}

private struct PairPrompt: View {
    let action: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "laptopcomputer.and.iphone")
                .font(.system(size: 52, weight: .light))
                .foregroundStyle(Color.accentColor)
                .padding(.bottom, 4)
            Text("Bilgisayarınla eşleştir")
                .font(.title2.weight(.semibold))
            Text("Bilgisayarda OpenContinuity'yi açıp Ayarlar › Cihaz ekle'ye bas, sonra burada başla.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            Button(action: action) {
                Text("Eşleştirmeye başla")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
            .buttonBorderShape(.capsule)
            .padding(.top, 8)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 48)
    }
}

private struct DeviceGrid: View {
    @EnvironmentObject private var store: EngineStore
    @Binding var pairingShown: Bool

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: 12)]

    var body: some View {
        LazyVGrid(columns: columns, spacing: 12) {
            ForEach(store.peers, id: \.fingerprintPrefix) { peer in
                DeviceCard(peer: peer, activeDirection: direction(for: peer))
            }
            AddDeviceCard { pairingShown = true }
        }
    }

    /// Gelen aktarım gönderen kartta, giden aktarım bağlı tüm kartlarda görünür (motor gruba gönderir).
    private func direction(for peer: PeerRecord) -> Transfer.Direction? {
        guard let active = store.active, peer.connected else { return nil }
        if active.direction == .incoming && active.peer != peer.name { return nil }
        return active.direction
    }
}

private struct SendActions: View {
    let connected: Int
    let photos: () -> Void
    let files: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("Gönder")
                    .font(.title3.weight(.semibold))
                Spacer()
                Text(connected > 0 ? "\(connected) bilgisayara" : "Bağlı bilgisayar yok")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            HStack(spacing: 12) {
                SendTile(title: "Fotoğraf", subtitle: "Galeriden", symbol: "photo.on.rectangle.angled", colors: [.pink, .orange], action: photos)
                SendTile(title: "Dosya", subtitle: "Dosyalar'dan", symbol: "folder.fill", colors: [.blue, .cyan], action: files)
            }
            .disabled(connected == 0)
        }
    }
}

private struct SendTile: View {
    @Environment(\.isEnabled) private var isEnabled
    let title: String
    let subtitle: String
    let symbol: String
    let colors: [Color]
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 18) {
                Image(systemName: symbol)
                    .font(.system(size: 22, weight: .semibold))
                    .foregroundStyle(.white)
                    .frame(width: 46, height: 46)
                    .background(
                        Circle().fill(LinearGradient(colors: isEnabled ? colors : [Color(.systemGray3)], startPoint: .topLeading, endPoint: .bottomTrailing))
                    )
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.headline)
                        .foregroundStyle(.primary)
                    Text(subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
            .background(RoundedRectangle(cornerRadius: 22, style: .continuous).fill(Color(.secondarySystemBackground)))
        }
        .buttonStyle(PressableStyle())
        .opacity(isEnabled ? 1 : 0.6)
    }
}

/// Basınca hafifçe küçülür.
private struct PressableStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.96 : 1)
            .animation(.spring(duration: 0.25), value: configuration.isPressed)
            .sensoryFeedback(.impact(weight: .light), trigger: configuration.isPressed) { _, pressed in pressed }
    }
}

private struct ActivityList: View {
    @EnvironmentObject private var store: EngineStore

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Son aktarımlar")
                .font(.title3.weight(.semibold))
                .padding(.bottom, 8)
            if let active = store.active {
                TransferRow(transfer: active, inProgress: true)
                Divider().padding(.leading, 52)
            }
            if store.transfers.isEmpty && store.active == nil {
                Text("Bilgisayarda kopyaladığın metin ve görseller iPhone panosuna gelir, burada da görünür.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            ForEach(store.transfers) { transfer in
                TransferRow(transfer: transfer, inProgress: false)
                if transfer.id != store.transfers.last?.id {
                    Divider().padding(.leading, 52)
                }
            }
        }
        .animation(.smooth, value: store.transfers.count)
    }
}

private struct NoticeBanner: View {
    @EnvironmentObject private var store: EngineStore

    var body: some View {
        if let notice = store.notice {
            Text(notice)
                .font(.subheadline)
                .padding(.horizontal, 16).padding(.vertical, 10)
                .background(.thinMaterial, in: Capsule())
                .padding(.bottom, 12)
                .padding(.horizontal, 20)
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .task(id: notice) {
                    try? await Task.sleep(for: .seconds(3))
                    if store.notice == notice { store.notice = nil }
                }
        }
    }
}
