//! Off-thread thumbnail decoding for favorites, with a process-wide cache.
//!
//! The Preferences dialog (and thus the favorites grid) is rebuilt on every
//! open, and removals rebuild the whole grid — without a cache every rebuild
//! would re-decode every image. GdkTexture is immutable and thread-safe
//! (gdk-rs marks it `Send`), so one process-wide map serves all rebuilds.
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

/// Drop cache entries whose path is not among `kept` (e.g. the favorite
/// was just removed), so the stored files' memory is reclaimed promptly.
pub fn retain_only(kept: &[String]) {
    cache()
        .lock()
        .unwrap()
        .retain(|path, _| kept.contains(path));
}

/// Paint `picture` with the thumbnail for `path`: instantly from the
/// cache, or decoded off-thread on a miss (and stored for next time).
pub fn load_thumb(path: &str, picture: &gtk::Picture) {
    if let Some(tex) = cache().lock().unwrap().get(path).cloned() {
        picture.set_paintable(Some(tex.as_ref()));
        return;
    }
    let rx = spawn_decode(Path::new(path), THUMB_SIZE);
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
    if cache().lock().unwrap().contains_key(path) {
        return;
    }
    let rx = spawn_decode(Path::new(path), THUMB_SIZE);
    let key = path.to_string();
    glib::spawn_future_local(async move {
        let _ = await_texture(key, rx).await;
    });
}

/// Kick off a blocking decode; results arrive as JPEG bytes on `rx`.
fn spawn_decode(path: &Path, max_size: u32) -> async_channel::Receiver<Result<Vec<u8>, String>> {
    let (tx, rx) = async_channel::unbounded::<Result<Vec<u8>, String>>();
    let path = path.to_path_buf();
    crate::tokio_handle().spawn_blocking(move || {
        let _ = tx.send_blocking(load_image_jpeg(&path, max_size));
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
