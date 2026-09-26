use crate::theme::ColorScheme;
use image::DynamicImage;

/// Perceptual (gamma-encoded) brightness metrics of an image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Analysis {
    /// Average luma in [0, 1].
    pub avg_luma: f32,
    /// Fraction of very dark pixels (luma < 0.15).
    pub dark_frac: f32,
    /// Fraction of very light pixels (luma > 0.85).
    pub light_frac: f32,
}

/// Decode an in-memory image and analyze its brightness.
pub fn analyze(bytes: &[u8]) -> Result<Analysis, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("image decode failed: {e}"))?;
    Ok(analyze_image(&img))
}

pub fn analyze_image(img: &DynamicImage) -> Analysis {
    // Downscale aggressively: thumbnails are plenty and it keeps this cheap.
    let small = img.thumbnail(64, 64);
    let rgb = small.to_rgb8();
    let (w, h) = rgb.dimensions();
    let n = (w * h).max(1);

    let mut sum = 0.0f32;
    let mut dark = 0u32;
    let mut light = 0u32;
    for px in rgb.pixels() {
        let [r, g, b] = px.0;
        // Perceptual luma on gamma-encoded values: mid-gray maps to ~0.5.
        let luma = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
        sum += luma;
        if luma < 0.15 {
            dark += 1;
        }
        if luma > 0.85 {
            light += 1;
        }
    }

    Analysis {
        avg_luma: sum / n as f32,
        dark_frac: dark as f32 / n as f32,
        light_frac: light as f32 / n as f32,
    }
}

/// Classify an analyzed image against the dark threshold.
pub fn classify(a: &Analysis, dark_threshold: f32) -> ColorScheme {
    if a.avg_luma < dark_threshold {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn solid(color: [u8; 3]) -> DynamicImage {
        let mut img = RgbImage::new(32, 32);
        for px in img.pixels_mut() {
            *px = Rgb(color);
        }
        DynamicImage::ImageRgb8(img)
    }

    fn from_bytes(img: &DynamicImage) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn pure_white_is_light() {
        let a = analyze_image(&solid([255, 255, 255]));
        assert!(a.avg_luma > 0.99);
        assert_eq!(classify(&a, 0.45), ColorScheme::Light);
    }

    #[test]
    fn pure_black_is_dark() {
        let a = analyze_image(&solid([0, 0, 0]));
        assert!(a.avg_luma < 0.01);
        assert_eq!(classify(&a, 0.45), ColorScheme::Dark);
    }

    #[test]
    fn mid_gray_is_around_half() {
        let a = analyze_image(&solid([128, 128, 128]));
        assert!((a.avg_luma - 0.5).abs() < 0.02);
        assert_eq!(classify(&a, 0.45), ColorScheme::Light);
    }

    #[test]
    fn dark_image_with_bright_spot_is_dark() {
        let mut img = RgbImage::new(64, 64);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = if x < 8 && y < 8 {
                Rgb([240, 240, 240])
            } else {
                Rgb([12, 12, 16])
            };
        }
        let a = analyze_image(&DynamicImage::ImageRgb8(img));
        assert_eq!(classify(&a, 0.45), ColorScheme::Dark);
        assert!(a.dark_frac > 0.7);
    }

    #[test]
    fn decode_roundtrip_matches_direct_analysis() {
        let img = solid([30, 34, 40]);
        let bytes = from_bytes(&img);
        let a1 = analyze_image(&img);
        let a2 = analyze(&bytes).expect("decode");
        assert!((a1.avg_luma - a2.avg_luma).abs() < 0.01);
    }

    #[test]
    fn junk_bytes_are_rejected() {
        assert!(analyze(b"this is not an image").is_err());
    }
}
