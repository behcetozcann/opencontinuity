import SwiftUI

/// Masaüstündekiyle aynı akış: diğer cihaz aranır, iki ekranda aynı 6 haneli
/// kod görünür, iki taraf da onaylayınca eşleşilir.
struct PairingView: View {
    @EnvironmentObject private var store: EngineStore
    @Environment(\.dismiss) private var dismiss
    @State private var address = ""

    var body: some View {
        NavigationStack {
            VStack(spacing: 20) {
                content
                Spacer()
            }
            .padding(24)
            .navigationTitle("Eşleştir")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Vazgeç") {
                        store.cancelPairing()
                        dismiss()
                    }
                }
            }
        }
        .onAppear { store.startPairing() }
        .onChange(of: store.peers.count) { old, new in
            if new > old { dismiss() }
        }
    }

    @ViewBuilder private var content: some View {
        switch store.snapshot?.pairing {
        case let .confirm(peer, code):
            Text("\(peer) ile eşleşiliyor").font(.headline)
            Text(code)
                .font(.system(size: 44, weight: .semibold, design: .monospaced))
                .tracking(6)
            Text("İki ekranda da aynı 6 haneli kod görünmeli.")
                .foregroundStyle(.secondary).multilineTextAlignment(.center)
            HStack {
                Button("Farklı", role: .destructive) { store.confirmCode(false) }
                    .buttonStyle(.bordered)
                Button("Kodlar aynı") { store.confirmCode(true) }
                    .buttonStyle(.borderedProminent)
            }
        case let .waitingPeer(peer):
            ProgressView()
            Text("\(peer) cihazının onayı bekleniyor").foregroundStyle(.secondary)
        default:
            if let failure = store.pairingFailure {
                Label(failure, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                Button("Tekrar dene") { store.startPairing() }.buttonStyle(.borderedProminent)
            } else {
                ProgressView()
                Text("Diğer bilgisayar aranıyor").font(.headline)
                Text("Bilgisayarda OpenContinuity'de Ayarlar › Cihaz ekle'ye basın. İki cihaz aynı Wi-Fi ağında olmalı.")
                    .foregroundStyle(.secondary).multilineTextAlignment(.center)
            }
            addressFallback
        }
    }

    private var addressFallback: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Bulunamıyor mu? Bilgisayarın adresiyle bağlanın:").font(.footnote).foregroundStyle(.secondary)
            HStack {
                TextField("192.168.1.20", text: $address)
                    .keyboardType(.numbersAndPunctuation)
                    .textInputAutocapitalization(.never)
                    .textFieldStyle(.roundedBorder)
                Button("Bağlan") {
                    store.cancelPairing()
                    store.startPairing(address: address.trimmingCharacters(in: .whitespaces))
                }
                .disabled(address.isEmpty)
            }
        }
        .padding(.top, 12)
    }
}
