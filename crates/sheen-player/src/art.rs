//! Cover thumbnails. Decoding a 1500 px album cover for a 40 px list row would
//! waste memory and CPU in the webview, so covers are downscaled once,
//! re-encoded as small JPEGs and cached on disk.

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Downscale an encoded image so its longer side is at most `max` px and
/// return it as JPEG. Images already small enough are re-encoded anyway, so
/// every cached thumbnail has the same, predictable format.
pub fn thumbnail_jpeg(encoded: &[u8], max: u32) -> Option<Vec<u8>> {
    let img = ImageReader::new(Cursor::new(encoded)).with_guessed_format().ok()?.decode().ok()?;
    encode(&shrink(img, max))
}

fn shrink(img: DynamicImage, max: u32) -> DynamicImage {
    if img.width() <= max && img.height() <= max {
        img
    } else if img.width().max(img.height()) > max * 4 {
        // Two passes: a cheap nearest-neighbour cut first, then a good filter.
        img.resize(max * 2, max * 2, FilterType::Nearest).resize(max, max, FilterType::Triangle)
    } else {
        img.resize(max, max, FilterType::Triangle)
    }
}

fn encode(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 82).encode_image(&img.to_rgb8()).ok()?;
    Some(out)
}

/// On-disk thumbnail cache: `<dir>/<key>-<size>.jpg`, plus an in-memory set of
/// keys known to have no art so misses are not re-probed on every request.
pub struct ArtCache {
    dir: PathBuf,
    missing: std::sync::Mutex<std::collections::HashSet<(String, u32)>>,
}

impl ArtCache {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Self { dir, missing: Default::default() }
    }

    /// JPEG thumbnail for `key`, generated from `source` on first use.
    pub fn get(&self, key: &str, size: u32, source: &Path) -> Option<Vec<u8>> {
        let id = (key.to_string(), size);
        if self.missing.lock().unwrap().contains(&id) {
            return None;
        }
        let file = self.dir.join(format!("{key}-{size}.jpg"));
        if let Ok(bytes) = std::fs::read(&file) {
            return Some(bytes);
        }
        let thumb = crate::library::cover_art(source).and_then(|(_, b)| thumbnail_jpeg(&b, size));
        match &thumb {
            Some(bytes) => {
                let _ = std::fs::write(&file, bytes);
            }
            None => {
                self.missing.lock().unwrap().insert(id);
            }
        }
        thumb
    }

    /// Forget negative results (e.g. after a rescan found new art).
    pub fn reset_misses(&self) {
        self.missing.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbImage};

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, image::Rgb([200, 30, 30])))
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn shrinks_and_keeps_aspect() {
        let thumb = thumbnail_jpeg(&png(1200, 600), 96).unwrap();
        let img = image::load_from_memory(&thumb).unwrap();
        assert_eq!((img.width(), img.height()), (96, 48));
    }

    #[test]
    fn rejects_non_images() {
        assert!(thumbnail_jpeg(b"definitely not an image", 64).is_none());
    }

    #[test]
    fn caches_misses_and_hits() {
        let dir = std::env::temp_dir().join(format!("sheen-artcache-{}", std::process::id()));
        let cache = ArtCache::new(dir.clone());
        assert!(cache.get("k", 64, &dir.join("nope.mp3")).is_none());
        assert!(cache.missing.lock().unwrap().contains(&("k".to_string(), 64)));

        std::fs::write(dir.join("cover.png"), png(300, 300)).unwrap();
        cache.reset_misses();
        let first = cache.get("k", 64, &dir.join("song.mp3")).unwrap();
        assert!(dir.join("k-64.jpg").exists());
        assert_eq!(cache.get("k", 64, &dir.join("song.mp3")).unwrap(), first);
        let _ = std::fs::remove_dir_all(dir);
    }
}
