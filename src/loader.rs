use image::{ImageDecoder, ImageReader};
use std::path::Path;

pub const EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jfif", "gif", "webp", "bmp", "tif", "tiff", "ico",
];

pub struct DecodedImage {
    pub pixels: Vec<u8>,
    pub size: [usize; 2],
    pub original_size: [u32; 2],
}

/// Decode away from the UI thread, honor EXIF orientation, and respect GPU limits.
/// Animated formats display their first frame.
pub fn decode(path: &Path, max_side: u32) -> Result<DecodedImage, String> {
    let mut reader = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    decoded.apply_orientation(orientation);
    let original_size = [decoded.width(), decoded.height()];
    let max_side = max_side.max(1);
    if decoded.width() > max_side || decoded.height() > max_side {
        decoded = decoded.thumbnail(max_side, max_side);
    }
    let rgba = decoded.into_rgba8();
    Ok(DecodedImage {
        size: [rgba.width() as usize, rgba.height() as usize],
        pixels: rgba.into_raw(),
        original_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_unicode_path_and_scales_without_stretching() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("picture with spaces 日本.png");
        image::RgbaImage::from_pixel(80, 40, image::Rgba([25, 50, 100, 128]))
            .save(&path)
            .unwrap();
        let decoded = decode(&path, 20).unwrap();
        assert_eq!(decoded.original_size, [80, 40]);
        assert_eq!(decoded.size, [20, 10]);
        assert_eq!(decoded.pixels.len(), 20 * 10 * 4);
        assert_eq!(&decoded.pixels[..4], &[25, 50, 100, 128]);
    }

    #[test]
    fn corrupt_and_missing_files_return_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.png");
        std::fs::write(&path, b"not an image").unwrap();
        assert!(decode(&path, 4096).is_err());
        assert!(decode(&dir.path().join("missing.png"), 4096).is_err());
    }
}
