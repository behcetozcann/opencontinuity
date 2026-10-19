import Foundation
import UIKit

/// Uygulamanın kullandığı dizinler ve ağ ayarları.
enum Paths {
    /// Kimlik ve eş listesi; kullanıcıya görünmez.
    static var config: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("OpenContinuity", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    /// Gelen dosyalar; Dosyalar › iPhone'umda › OpenContinuity altında görünür.
    static var incoming: URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    /// Gönderilecek dosyaların kopyası (güvenlik kapsamlı URL'ler motor okurken kapanmasın diye).
    static var outgoing: URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("outgoing", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    // Simülatör Mac'in ağ yığınını paylaşır; Mac'teki OpenContinuity 47800'ü tutar.
    #if targetEnvironment(simulator)
    static let port: UInt16 = 47801
    #else
    static let port: UInt16 = 47800
    #endif

    static var deviceName: String { UIDevice.current.name }
}
