import SwiftUI

@main
struct OpenContinuityApp: App {
    @StateObject private var store = EngineStore()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            HomeView()
                .environmentObject(store)
        }
        // iOS arka planda uygulamayı askıya alır; motor yalnız öndeyken çalışır.
        .onChange(of: scenePhase, initial: true) { _, phase in
            switch phase {
            case .active: store.start()
            case .background: store.stop()
            default: break
            }
        }
    }
}
