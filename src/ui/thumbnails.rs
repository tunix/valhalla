//! Off-thread thumbnail decoding for favorites, with a three-tier cache:
//! in-memory textures, on-disk thumbnails and the stored wallpaper itself.
//!
//! The Preferences dialog (and thus the favorites grid) is rebuilt on every
//! open, and removals rebuild the whole grid — without a cache every rebuild
//! would re-decode every image. GdkTexture is immutable and thread-safe
//! (gdk-rs marks it `Send`), so one process-wide map serves all rebuilds.
//! Thumbnails are also persisted under `$XDG_CACHE_HOME/valhalla/thumbs/`,
//! so the grid paints instantly even on the first open after a restart.
//! New likes can pre-warm the cache via [`prewarm`] so the tile paints
//! instantly on the next grid open.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use adw::prelude::*;
use gtk::{gdk, glib};

/// Thumbnail budget for favorites tiles (2× the 168×94 tile for HiDPI).
pub const THUMB_SIZE: u32 = 336;

/// Decoded thumbnail textures keyed by the favorite's stored path.
fn cache() -> &'static Mutex<HashMap<String, Arc<gdk::Texture>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<gdk::Texture>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Directory for on-disk thumbnails: `$XDG_CACHE_HOME/valhalla/thumbs/`.
fn disk_dir() -> std::path::PathBuf {
    let dir = glib::user_cache_dir().join("valhalla").join("thumbs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// On-disk thumbnail for a stored favorite: favorites live at
/// `<id>.<ext>`, thumbnails at `thumbs/<id>.jpg` (stable across runs).
fn disk_path_for(stored: &Path) -> std::path::PathBuf {
    let stem = stored
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    disk_dir().join(format!("{stem}.jpg"))
}

/// Thumbnail texture from the disk cache, if readable/decodable. Also
/// seeds the in-memory cache so the next lookup is a plain hit.
fn texture_from_disk(path: &str) -> Option<Arc<gdk::Texture>> {
    let bytes = std::fs::read(disk_path_for(Path::new(path))).ok()?;
    let tex = gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).ok()?;
    let tex = Arc::new(tex);
    cache()
        .lock()
        .unwrap()
        .insert(path.to_string(), tex.clone());
    Some(tex)
}

/// Drop cache entries whose path is not among `kept` (e.g. the favorite
/// was just removed): memory is reclaimed promptly and orphan on-disk
/// thumbnails are unlinked so the cache dir stays honest.
pub fn retain_only(kept: &[String]) {
    cache()
        .lock()
        .unwrap()
        .retain(|path, _| kept.contains(path));
    let kept_stems: std::collections::HashSet<&str> = kept
        .iter()
        .filter_map(|p| Path::new(p).file_stem().and_then(|s| s.to_str()))
        .collect();
    if let Ok(entries) = std::fs::read_dir(disk_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if !kept_stems.contains(stem) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Paint `picture` with the thumbnail for `path`: instantly from the
/// in-memory cache or the disk cache, or decoded off-thread on a full
/// miss (persisted for future runs, stored for next time).
pub fn load_thumb(path: &str, picture: &gtk::Picture) {
    if let Some(tex) = cache()
        .lock()
        .unwrap()
        .get(path)
        .cloned()
        .or_else(|| texture_from_disk(path))
    {
        picture.set_paintable(Some(tex.as_ref()));
        return;
    }
    let disk = disk_path_for(Path::new(path));
    let rx = spawn_decode(Path::new(path), THUMB_SIZE, Some(disk));
    let pic = picture.downgrade();
    let key = path.to_string();
    glib::spawn_future_local(async move {
        if let Some(tex) = await_texture(key, rx).await {
            if let Some(pic) = pic.upgrade() {
                pic.set_paintable(Some(tex.as_ref()));
            }
        }
    });
}

/// Pre-decode a thumbnail in the background so a later grid open paints
/// instantly. Called when a wallpaper is liked; a cache hit is a no-op.
pub fn prewarm(path: &str) {
    if cache().lock().unwrap().contains_key(path) || texture_from_disk(path).is_some() {
        return;
    }
    let disk = disk_path_for(Path::new(path));
    let rx = spawn_decode(Path::new(path), THUMB_SIZE, Some(disk));
    let key = path.to_string();
    glib::spawn_future_local(async move {
        let _ = await_texture(key, rx).await;
    });
}

/// Kick off a blocking decode; results arrive as JPEG bytes on `rx`.
/// `persist_to` writes the bytes out (the disk thumbnail cache) so
/// restarts skip the decode entirely.
fn spawn_decode(
    path: &Path,
    max_size: u32,
    persist_to: Option<std::path::PathBuf>,
) -> async_channel::Receiver<Result<Vec<u8>, String>> {
    let (tx, rx) = async_channel::unbounded::<Result<Vec<u8>, String>>();
    let path = path.to_path_buf();
    crate::tokio_handle().spawn_blocking(move || {
        let result = load_image_jpeg(&path, max_size).inspect(|bytes| {
            if let Some(dest) = &persist_to {
                let _ = std::fs::write(dest, bytes);
            }
        });
        let _ = tx.send_blocking(result);
    });
    rx
}

/// Await decoded bytes, turn them into a texture and store it in the
/// cache. Returns the texture so the caller can paint immediately.
async fn await_texture(
    key: String,
    rx: async_channel::Receiver<Result<Vec<u8>, String>>,
) -> Option<Arc<gdk::Texture>> {
    let Ok(loaded) = rx.recv().await else {
        return None;
    };
    match loaded.and_then(|bytes| {
        gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).map_err(|e| e.to_string())
    }) {
        Ok(tex) => {
            let tex = Arc::new(tex);
            cache().lock().unwrap().insert(key, tex.clone());
            Some(tex)
        }
        Err(e) => {
            tracing::warn!("thumbnail decode failed: {e}");
            None
        }
    }
}

/// Decode an image off the UI thread and re-encode it as JPEG bytes that
/// `gdk::Texture::from_bytes` accepts (it reads PNG/JPEG/TIFF only, while
/// some sources hand out WebP). `max_size` bounds the larger dimension;
/// 0 keeps the original size.
pub fn load_image_jpeg(path: &Path, max_size: u32) -> Result<Vec<u8>, String> {
    let img = image::ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| format!("open failed: {e}"))?
        .decode()
        .map_err(|e| format!("decode failed: {e}"))?;
    let img = if max_size > 0 {
        img.thumbnail(max_size, max_size)
    } else {
        img
    };
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}
