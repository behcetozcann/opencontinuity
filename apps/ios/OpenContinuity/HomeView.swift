import SwiftUI
import UniformTypeIdentifiers

struct HomeView: View {
    @EnvironmentObject private var store: EngineStore
    @State private var pairingShown = false
    @State private var devicesShown = false
    @State private var importerShown = false

    var body: some View {
        NavigationStack {
            List {
                Section { StatusCard(pairingShown: $pairingShown) }
                if !store.peers.isEmpty {
                    Section {
                        PasteButton(supportedContentTypes: [.plainText, .png, .image]) { providers in
                            send(providers)
                        }
                        .labelStyle(.titleAndIcon)
                        Button { importerShown = true } label: {
                            Label("Dosya gönder", systemImage: "doc.badge.arrow.up")
                        }
                    } footer: {
                        Text("Uygulama açıkken kopyaladıklarınız kendiliğinden gider; başka uygulamada kopyaladıysanız buraya dönmeniz yeter.")
                    }
                }
                if let active = store.active {
                    Section("Sürüyor") { TransferRow(transfer: active, inProgress: true) }
                }
                if !store.transfers.isEmpty {
                    Section("Son aktarımlar") {
                        ForEach(store.transfers) { TransferRow(transfer: $0, inProgress: false) }
                    }
                }
            }
            .navigationTitle("OpenContinuity")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { devicesShown = true } label: { Image(systemName: "gearshape") }
                        .accessibilityLabel("Cihazlar")
                }
            }
            .sheet(isPresented: $pairingShown) { PairingView() }
            .sheet(isPresented: $devicesShown) { DevicesView(pairingShown: $pairingShown) }
            .fileImporter(isPresented: $importerShown, allowedContentTypes: [.item, .folder], allowsMultipleSelection: true) { result in
                if case let .success(urls) = result { store.send(files: urls) }
            }
            .overlay(alignment: .bottom) { NoticeBanner() }
        }
    }

    private func send(_ providers: [NSItemProvider]) {
        guard let provider = providers.first else { return }
        if provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
            _ = provider.loadObject(ofClass: String.self) { text, _ in
                guard let text else { return }
                Task { @MainActor in store.push(.text(text: text)) }
            }
        } else {
            provider.loadDataRepresentation(forTypeIdentifier: UTType.image.identifier) { data, _ in
                guard let data, let png = UIImage(data: data)?.pngData() else { return }
                Task { @MainActor in store.push(.png(data: png)) }
            }
        }
    }
}

private struct StatusCard: View {
    @EnvironmentObject private var store: EngineStore
    @Binding var pairingShown: Bool

    var body: some View {
        if store.peers.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Label("Henüz eşleşmiş bilgisayar yok", systemImage: "laptopcomputer.and.iphone")
                    .font(.headline)
                Text("Bilgisayarda OpenContinuity'de Ayarlar › Cihaz ekle'ye basın, sonra burada eşleştirmeye başlayın.")
                    .font(.subheadline).foregroundStyle(.secondary)
                Button("Eşleştirmeye başla") { pairingShown = true }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 4)
            }
            .padding(.vertical, 4)
        } else {
            ForEach(store.peers, id: \.fingerprintPrefix) { peer in
                HStack {
                    Image(systemName: "desktopcomputer")
                        .foregroundStyle(peer.connected ? Color.green : Color.secondary)
                    VStack(alignment: .leading) {
                        Text(peer.name)
                        Text(peer.connected ? "Bağlı" : (peer.linkError ?? "Bağlanılıyor…"))
                            .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                }
            }
        }
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
                .transition(.opacity)
                .task(id: notice) {
                    try? await Task.sleep(for: .seconds(3))
                    if store.notice == notice { store.notice = nil }
                }
        }
    }
}
