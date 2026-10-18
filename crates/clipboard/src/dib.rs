//! Windows `CF_DIB` (BITMAPINFO + pikseller) ile PNG arasında dönüşüm.

use std::io::Cursor;

use anyhow::bail;
use image::codecs::bmp::BmpDecoder;
use image::codecs::png::PngDecoder;
use image::{DynamicImage, ImageDecoder, ImageFormat};

/// RGBA'ya açılınca ~200 MB; daha büyüğü belleği şişirmesin diye dönüştürülmez.
const MAX_PIXELS: u64 = 50_000_000;
const BITMAPINFOHEADER_BYTES: u32 = 40;

pub fn dib_to_png(dib: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decoder = BmpDecoder::new_without_file_header(Cursor::new(dib))?;
    check_size(decoder.dimensions())?;
    let image = DynamicImage::from_decoder(decoder)?;
    let mut png = Vec::new();
    image.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    Ok(png)
}

/// 32 bit, alttan üste satırlı DIB. Saydamlığı bilmeyen uygulamalar için
/// saydam pikseller beyaz zemine karıştırılır; saydamlık PNG biçiminde korunur.
pub fn png_to_dib(png: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decoder = PngDecoder::new(Cursor::new(png))?;
    let (width, height) = decoder.dimensions();
    check_size((width, height))?;
    let rgba = DynamicImage::from_decoder(decoder)?.to_rgba8();
    let pixel_bytes = width * height * 4;
    let mut dib = Vec::with_capacity((BITMAPINFOHEADER_BYTES + pixel_bytes) as usize);
    for field in [BITMAPINFOHEADER_BYTES, width, height] {
        dib.extend_from_slice(&field.to_le_bytes());
    }
    dib.extend_from_slice(&1u16.to_le_bytes()); // düzlem
    dib.extend_from_slice(&32u16.to_le_bytes()); // bit/piksel
    for field in [0u32, pixel_bytes, 0, 0, 0, 0] {
        // BI_RGB, görüntü boyutu, çözünürlükler, palet
        dib.extend_from_slice(&field.to_le_bytes());
    }
    for row in rgba.rows().rev() {
        for pixel in row {
            let [red, green, blue, alpha] = pixel.0;
            let over_white = |channel: u8| ((u32::from(channel) * u32::from(alpha) + 255 * (255 - u32::from(alpha))) / 255) as u8;
            dib.extend_from_slice(&[over_white(blue), over_white(green), over_white(red), 255]);
        }
    }
    Ok(dib)
}

fn check_size((width, height): (u32, u32)) -> anyhow::Result<()> {
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        bail!("görsel çok büyük: {width}×{height}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn sample_png() -> Vec<u8> {
        let image = RgbaImage::from_fn(3, 2, |x, y| Rgba([x as u8 * 80, y as u8 * 120, 200, 255]));
        let mut png = Vec::new();
        image.write_to(&mut Cursor::new(&mut png), ImageFormat::Png).unwrap();
        png
    }

    #[test]
    fn png_survives_dib_round_trip() {
        let original = image::load_from_memory(&sample_png()).unwrap().to_rgba8();
        let png = dib_to_png(&png_to_dib(&sample_png()).unwrap()).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        assert_eq!(image::load_from_memory(&png).unwrap().to_rgba8(), original);
    }

    #[test]
    fn transparent_pixels_become_white_in_dib() {
        let mut png = Vec::new();
        RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])).write_to(&mut Cursor::new(&mut png), ImageFormat::Png).unwrap();
        let dib = png_to_dib(&png).unwrap();
        assert_eq!(&dib[40..], &[255, 255, 255, 255]);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(dib_to_png(&[1, 2, 3]).is_err());
        assert!(png_to_dib(&[1, 2, 3]).is_err());
    }
}
