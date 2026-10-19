import SwiftUI

struct DevicesView: View {
    @EnvironmentObject private var store: EngineStore
    @Environment(\.dismiss) private var dismiss
    @Binding var pairingShown: Bool
    @State private var removing: PeerRecord?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(store.peers, id: \.fingerprintPrefix) { peer in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(peer.name)
                            Text(peer.connected ? "Bağlı · \(peer.address ?? "")" : "Bağlı değil")
                                .font(.caption).foregroundStyle(.secondary)
                            Text("Parmak izi \(peer.fingerprintPrefix)")
                                .font(.caption2.monospaced()).foregroundStyle(.tertiary)
                        }
                        .swipeActions {
                            Button("Kaldır", role: .destructive) { removing = peer }
                        }
                    }
                    Button {
                        dismiss()
                        pairingShown = true
                    } label: {
                        Label("Cihaz ekle", systemImage: "plus")
                    }
                } header: {
                    Text("Cihazlar")
                } footer: {
                    if let snapshot = store.snapshot {
                        Text("Bu cihaz: \(snapshot.deviceName) · \(snapshot.fingerprintPrefix)")
                    }
                }
                Section("Gelen dosyalar") {
                    Text("Dosyalar › iPhone'umda › OpenContinuity")
                    Text("Bilgisayardan gelen görsel ve metin panoya kopyalanır.")
                        .font(.footnote).foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Ayarlar")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Bitti") { dismiss() } }
            }
            .confirmationDialog(
                "\(removing?.name ?? "") kaldırılsın mı?",
                isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }),
                titleVisibility: .visible
            ) {
                Button("Kaldır", role: .destructive) {
                    if let removing { store.unpair(removing) }
                }
            } message: {
                Text("Bu cihazla paylaşım durur. Yeniden eşleşmeden geri gelmez.")
            }
        }
    }
}
