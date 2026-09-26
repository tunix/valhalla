use std::path::Path;

use image::imageops::FilterType;

/// Scale the image at `path` up so it covers the screen when it is smaller.
/// Returns `Ok(true)` when the file was re-encoded in place.
pub fn upscale_to_cover(path: &Path, screen_w: i32, screen_h: i32) -> Result<bool, String> {
    let img = image::open(path).map_err(|e| format!("decode failed: {e}"))?;
    let (w, h) = (img.width() as i32, img.height() as i32);
    if w >= screen_w && h >= screen_h {
        return Ok(false);
    }

    let scale = (screen_w as f32 / w as f32).max(screen_h as f32 / h as f32);
    if scale > 4.0 {
        tracing::debug!("wallpaper too small to upscale ({w}x{h} for {screen_w}x{screen_h})");
        return Ok(false);
    }

    let target_w = ((w as f32 * scale).ceil() as u32).max(1);
    let target_h = ((h as f32 * scale).ceil() as u32).max(1);
    let resized = img.resize_exact(target_w, target_h, FilterType::Triangle);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let jpeg = matches!(ext.as_str(), "jpg" | "jpeg");
    let tmp = path.with_extension("valhalla.tmp");
    {
        let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        if jpeg {
            let rgb = image::DynamicImage::ImageRgb8(resized.to_rgb8());
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&file, 92);
            encoder
                .encode_image(&rgb)
                .map_err(|e| format!("encode failed: {e}"))?;
        } else {
            let format = if ext == "png" {
                image::ImageFormat::Png
            } else {
                image::ImageFormat::Jpeg
            };
            resized
                .write_to(&mut std::io::BufWriter::new(&file), format)
                .map_err(|e| format!("encode failed: {e}"))?;
        }
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    tracing::info!("upscaled wallpaper {w}x{h} -> {target_w}x{target_h}");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgb, RgbImage};

    fn write_test_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = Rgb([(x % 256) as u8, (y % 256) as u8, 64]);
        }
        image::DynamicImage::ImageRgb8(img)
            .save_with_format(path, image::ImageFormat::Jpeg)
            .unwrap();
    }

    #[test]
    fn upscales_to_cover() {
        let dir = std::env::temp_dir().join("valhalla-fit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("small.jpg");
        write_test_image(&path, 400, 300);

        let changed = upscale_to_cover(&path, 800, 600).unwrap();
        assert!(changed);
        let img = image::open(&path).unwrap();
        assert_eq!(img.dimensions(), (800, 600));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn skips_when_large_enough() {
        let dir = std::env::temp_dir().join("valhalla-fit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("large.jpg");
        write_test_image(&path, 1920, 1080);

        let changed = upscale_to_cover(&path, 1280, 720).unwrap();
        assert!(!changed);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn skips_when_far_too_small() {
        let dir = std::env::temp_dir().join("valhalla-fit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tiny.jpg");
        write_test_image(&path, 100, 100);

        let changed = upscale_to_cover(&path, 4000, 2000).unwrap();
        assert!(!changed);
        std::fs::remove_file(&path).ok();
    }
}
