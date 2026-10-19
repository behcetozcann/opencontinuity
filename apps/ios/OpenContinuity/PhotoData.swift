import PhotosUI
import SwiftUI

/// Fotoğraflar'dan seçilen bir görselin baytları ve dosya uzantısı.
struct PhotoData {
    let data: Data
    let fileExtension: String

    /// `.compatible` kodlamayla HEIC görseller JPEG gelir; Windows/Linux açabilsin diye.
    static func load(_ items: [PhotosPickerItem]) async -> [PhotoData] {
        var photos: [PhotoData] = []
        for item in items {
            guard let data = try? await item.loadTransferable(type: Data.self) else { continue }
            photos.append(PhotoData(data: data, fileExtension: fileExtension(of: data, item: item)))
        }
        return photos
    }

    private static func fileExtension(of data: Data, item: PhotosPickerItem) -> String {
        if data.starts(with: [0xFF, 0xD8]) { return "jpg" }
        if data.starts(with: [0x89, 0x50, 0x4E, 0x47]) { return "png" }
        return item.supportedContentTypes.first?.preferredFilenameExtension ?? "jpg"
    }
}
