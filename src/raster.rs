//! Rasterizing image formats the `image` crate cannot decode: SVG (via
//! `resvg`) and JPEG XL (via `jxl-oxide`).
//!
//! Manual-folder wallpapers may legitimately be SVG or JXL — Bluefin's
//! `bluefin-wallpapers-extra` pack ships both — but every downstream
//! consumer needs raster pixels: analysis thumbnails are JPEG copies, the
//! fit check decodes headers with the `image` crate, and the file the
//! desktop finally renders must be readable by gdk-pixbuf (which has no
//! JXL loader on most distros). Both formats are therefore rasterized
//! here, once, and re-encoded as plain PNG/JPEG by the callers — the
//! applied wallpaper and every thumbnail are always files the `image`
//! crate can read again.

use std::hash::Hasher;
use std::path::{Path, PathBuf};

use image::DynamicImage;
// resvg re-exports its usvg and tiny-skia engines at matching versions.
use resvg::tiny_skia;
use resvg::usvg;

/// Extensions that cannot be decoded by the `image` crate and need
/// rasterizing first.
pub const RASTERIZED_EXTS: &[&str] = &["svg", "jxl"];

/// True when the (already lowercased) extension needs [`decode_path`]'s
/// special handling.
pub fn ext_is_rasterized(ext: &str) -> bool {
    RASTERIZED_EXTS.contains(&ext.to_ascii_lowercase().as_str())
}

/// True when `path` is a format the `image` crate cannot decode.
pub fn needs_rasterize(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(ext_is_rasterized)
        .unwrap_or(false)
}

/// Upper bound for any rendered edge: keeps a huge SVG viewBox (or a
/// screen-target scale on a tiny drawing) from allocating gigabytes of
/// pixels, the same spirit as the fit check's 4× upscale cap.
const MAX_EDGE: u32 = 8192;

/// Rasterize SVG bytes to an RGBA image. `target` is the output box the
/// drawing should cover (e.g. the screen size); without a target the SVG's
/// intrinsic size is used. The rendering never exceeds [`MAX_EDGE`] on
/// either side.
pub fn rasterize_svg(data: &[u8], target: Option<(u32, u32)>) -> Result<DynamicImage, String> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let opt = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(data, &opt).map_err(|e| format!("svg parse failed: {e}"))?;
    let size = tree.size();
    let (iw, ih) = (size.width(), size.height());
    if iw <= 0.0 || ih <= 0.0 {
        return Err("svg has no drawable size".into());
    }
    let scale = match target {
        Some((tw, th)) if tw > 0 && th > 0 => (tw as f32 / iw).max(th as f32 / ih),
        _ => 1.0,
    };
    let scale = scale.min(MAX_EDGE as f32 / iw.max(ih)).max(0.001);
    let w = ((iw * scale).ceil() as u32).clamp(1, MAX_EDGE);
    let h = ((ih * scale).ceil() as u32).clamp(1, MAX_EDGE);
    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("svg raster {w}x{h} failed"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied alpha; `image` wants straight alpha.
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let img = image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| "svg raster buffer mismatch".to_string())?;
    Ok(DynamicImage::ImageRgba8(img))
}

/// Decode a JPEG XL file with jxl-oxide (the `image` crate has no JXL
/// support). Animations decode as their first frame.
pub fn decode_jxl(reader: impl std::io::Read) -> Result<DynamicImage, String> {
    let decoder = jxl_oxide::integration::JxlDecoder::new(reader)
        .map_err(|e| format!("jxl open failed: {e}"))?;
    DynamicImage::from_decoder(decoder).map_err(|e| format!("jxl decode failed: {e}"))
}

/// [`decode_jxl`] for a file path.
pub fn decode_jxl_file(p: &Path) -> Result<DynamicImage, String> {
    let f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    decode_jxl(std::io::BufReader::new(f))
}

/// Composite `img` over white into an RGB image. Semi-transparent pixels
/// (common in SVG drawings) would otherwise silently drop or darken their
/// alpha channel in the RGB conversion, skewing brightness analysis.
pub fn flatten_white(img: &DynamicImage) -> image::RgbImage {
    let rgba = img.to_rgba8();
    let mut out = image::RgbImage::new(rgba.width(), rgba.height());
    for (x, y, px) in out.enumerate_pixels_mut() {
        let p = rgba.get_pixel(x, y);
        let a = p.0[3] as u32;
        let mix = |fg: u8, bg: u8| ((fg as u32 * a + bg as u32 * (255 - a)) / 255) as u8;
        *px = image::Rgb([mix(p.0[0], 255), mix(p.0[1], 255), mix(p.0[2], 255)]);
    }
    out
}

/// Cache key for a rasterized copy: path + size + mtime, mirroring the
/// manual thumbnail key so a replaced file gets a fresh copy and an
/// unchanged file reuses the existing one.
fn raster_key(p: &Path) -> String {
    let (size, mtime) = match std::fs::metadata(p) {
        Ok(m) => (
            m.len(),
            m.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ),
        Err(_) => (0, 0),
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&p, &mut hasher);
    hasher.write_u64(size);
    hasher.write_u128(mtime);
    format!("{:016x}", hasher.finish())
}

/// Rasterize `src` (SVG or JXL) into `dir` as a plain PNG the desktop can
/// render, and return its path. The copy is named by content-identity
/// (path+size+mtime), so repeated refreshes reuse an existing file and a
/// replaced source gets a fresh one. `target` (the screen size) is honored
/// for SVG only — a vector is rendered to cover it directly; JXL decodes
/// at its native size and the regular fit check handles the rest.
pub fn rasterize_into(
    src: &Path,
    dir: &Path,
    target: Option<(u32, u32)>,
) -> Result<PathBuf, String> {
    let dest = dir.join(format!("raster-{}.png", raster_key(src)));
    if dest.exists() {
        return Ok(dest);
    }
    let img = match src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => {
            let data = std::fs::read(src).map_err(|e| e.to_string())?;
            rasterize_svg(&data, target)?
        }
        _ => decode_jxl_file(src)?,
    };
    // tmp + rename so a concurrent refresh never sees a half-written file
    // (same pattern as the thumbnail caches).
    let tmp = dir.join(format!("raster-{}.tmp", raster_key(src)));
    img.save_with_format(&tmp, image::ImageFormat::Png)
        .map_err(|e| format!("png encode failed: {e}"))?;
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Tiny in-code test images (no image files in the repo — see AGENTS.md).
#[cfg(test)]
pub(crate) mod test_images {
    /// 40x30 red rectangle with a blue circle.
    pub const SVG: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' width='40' height='30' viewBox='0 0 40 30'><rect width='40' height='30' fill='#ff0000'/><circle cx='20' cy='15' r='8' fill='#0000ff'/></svg>";

    /// 16x12 solid red JPEG XL codestream (71 bytes, encoded with
    /// `cjxl -q 90` from a 16x12 `xc:red` PNG).
    pub const JXL: &[u8] = &[
        255, 10, 88, 48, 144, 80, 92, 8, 0, 18, 136, 2, 0, 224, 0, 93, 229, 18, 0, 0, 21, 42, 163,
        140, 27, 188, 156, 235, 249, 242, 67, 135, 197, 180, 141, 235, 12, 109, 181, 109, 97, 73,
        162, 200, 56, 54, 73, 39, 112, 72, 18, 139, 32, 238, 38, 199, 3, 0, 0, 66, 204, 195, 112,
        2, 118, 130, 161, 41, 42, 73, 0,
    ];
}

#[cfg(test)]
mod tests {
    use super::test_images;
    use super::*;
    use image::{GenericImageView, Rgba};

    #[test]
    fn svg_rasterizes_at_intrinsic_size() {
        let img = rasterize_svg(b"<svg xmlns='http://www.w3.org/2000/svg' width='40' height='30'><rect width='40' height='30' fill='#ff0000'/></svg>", None).unwrap();
        assert_eq!(img.dimensions(), (40, 30));
        assert_eq!(img.get_pixel(20, 15), Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn svg_rasterizes_to_cover_target() {
        let img = rasterize_svg(b"<svg xmlns='http://www.w3.org/2000/svg' width='40' height='20'><rect width='40' height='20' fill='#00ff00'/></svg>", Some((120, 50))).unwrap();
        // 40x20 covering 120x50 scales by 3.0 -> 120x60.
        assert_eq!(img.dimensions(), (120, 60));
    }

    #[test]
    fn svg_transparency_flattens_to_white() {
        let img = rasterize_svg(b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><rect width='10' height='10' fill='#0000ff' fill-opacity='0.5'/></svg>", None).unwrap();
        let flat = flatten_white(&img);
        // 50% blue over white -> (127, 127, 255)-ish, not raw (0,0,255).
        let px = flat.get_pixel(5, 5).0;
        assert!(px[2] > 200 && px[0] < 160 && px[1] < 160, "got {px:?}");
    }

    #[test]
    fn svg_rejects_garbage() {
        assert!(rasterize_svg(b"not an svg", None).is_err());
        assert!(rasterize_svg(b"", None).is_err());
    }

    #[test]
    fn jxl_fixture_decodes() {
        let img = decode_jxl(std::io::Cursor::new(test_images::JXL)).expect("decode jxl");
        assert_eq!(img.dimensions(), (16, 12));
        assert_eq!(img.get_pixel(8, 6), Rgba([255, 0, 0, 255]));
        assert!(decode_jxl(std::io::Cursor::new(b"not a jxl".to_vec())).is_err());
    }

    #[test]
    fn needs_rasterize_follows_extensions() {
        assert!(needs_rasterize(Path::new("/x/a.svg")));
        assert!(needs_rasterize(Path::new("/x/A.JXL")));
        assert!(!needs_rasterize(Path::new("/x/a.jpg")));
        assert!(!needs_rasterize(Path::new("/x/no-ext")));
        assert!(ext_is_rasterized("SVG"));
        assert!(!ext_is_rasterized("png"));
    }

    #[test]
    fn rasterize_into_caches_and_reuses() {
        let dir = std::env::temp_dir().join(format!("valhalla-raster-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("art.svg");
        std::fs::write(&src, test_images::SVG).unwrap();

        let png = rasterize_into(&src, &dir, Some((80, 40))).expect("rasterize");
        assert!(png.is_file());
        assert!(png.extension().and_then(|e| e.to_str()) == Some("png"));
        let img = image::open(&png).expect("converted file must be a plain PNG");
        // Cover keeps the aspect: 40x30 covering 80x40 -> 80x60.
        assert_eq!(img.dimensions(), (80, 60), "screen target is covered");

        // Second call with an unchanged source reuses the same file.
        let mtime = std::fs::metadata(&png).unwrap().modified().unwrap();
        assert_eq!(rasterize_into(&src, &dir, Some((80, 40))).unwrap(), png);
        assert_eq!(
            std::fs::metadata(&png).unwrap().modified().unwrap(),
            mtime,
            "cached copy must be reused"
        );

        // Replacing the source invalidates the cache.
        std::fs::write(
            &src,
            b"<svg xmlns='http://www.w3.org/2000/svg' width='60' height='40'><rect width='60' height='40' fill='#0000ff'/></svg>",
        )
        .unwrap();
        let png2 = rasterize_into(&src, &dir, Some((80, 40))).expect("rasterize after edit");
        assert_ne!(png2, png, "edited source must get a new copy");

        // JXL sources convert at native size (and the file-path decoder
        // reads the same bytes the in-memory one does).
        let src = dir.join("art.jxl");
        std::fs::write(&src, test_images::JXL).unwrap();
        assert_eq!(decode_jxl_file(&src).unwrap().dimensions(), (16, 12));
        let png = rasterize_into(&src, &dir, None).expect("rasterize jxl");
        assert_eq!(image::open(&png).unwrap().dimensions(), (16, 12));

        std::fs::remove_dir_all(&dir).ok();
    }
}
