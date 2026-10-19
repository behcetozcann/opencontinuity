import Foundation

/// Motorun geri çağrıları kendi iş parçacıklarından gelir; ana kuyruğa aktarılır.
final class ListenerBridge: EngineListener, @unchecked Sendable {
    private weak var store: EngineStore?

    init(store: EngineStore) {
        self.store = store
    }

    func onState(state: SnapshotRecord) {
        DispatchQueue.main.async { self.store?.apply(state: state) }
    }

    func onEvent(event: EventRecord) {
        DispatchQueue.main.async { self.store?.apply(event: event) }
    }

    func writeClipboard(clip: ClipRecord) {
        DispatchQueue.main.async { self.store?.pasteboard.write(clip) }
    }

    func setAdvertisePairing(pairing: Bool) {
        DispatchQueue.main.async { self.store?.bonjour?.setPairing(pairing) }
    }

    func onStopped(error: String?) {
        DispatchQueue.main.async { self.store?.engineStopped(error: error) }
    }
}
