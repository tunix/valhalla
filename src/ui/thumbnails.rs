//! Off-thread thumbnail decoding for favorites, with a three-tier cache:
//! in-memory textures, on-disk thumbnails and the stored wallpaper itself.
//! Full previews (for the favorites preview window) use the same disk dir
//! under a `.preview.jpg` suffix and a single-slot memory cache.
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

/// On-disk cache file for a stored favorite: favorites live at
/// `<id>.<ext>`, thumbnails at `thumbs/<id>.jpg` (stable across runs).
/// `variant` is a suffix between stem and `.jpg` — `""` for thumbnails,
/// `.preview` for full previews (`thumbs/<id>.preview.jpg`).
fn disk_path_for(stored: &Path, variant: &str) -> std::path::PathBuf {
    let stem = stored
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    disk_dir().join(format!("{stem}{variant}.jpg"))
}

/// On-disk preview for a stored favorite: `thumbs/<id>.preview.jpg`.
fn preview_disk_path(stored: &Path) -> std::path::PathBuf {
    disk_path_for(stored, ".preview")
}

/// Decode budget for previews: the primary monitor's long edge in device
/// pixels. A screen-sized decode is fast and looks identical on screen —
/// the full source resolution only costs time. Falls back to 2560 when no
/// monitor geometry is available yet.
fn preview_budget() -> u32 {
    const FALLBACK: u32 = 2560;
    let Some(display) = gdk::Display::default() else {
        return FALLBACK;
    };
    let monitor = display.monitors().item(0).and_downcast::<gdk::Monitor>();
    let Some(monitor) = monitor else {
        return FALLBACK;
    };
    let geometry = monitor.geometry();
    // Geometry is in logical pixels; decode in device pixels so HiDPI
    // screens don't get an upscaled-looking preview.
    let scale = monitor.scale_factor().max(1);
    let long_edge = geometry.width().max(geometry.height()).max(1);
    long_edge as u32 * scale as u32
}

/// Single-slot memory cache for the LAST preview only: full-size textures
/// are memory-heavy, so unlike the thumbnail map this holds at most one.
type PreviewSlot = Option<(String, Arc<gdk::Texture>)>;

fn preview_slot() -> &'static Mutex<PreviewSlot> {
    static SLOT: OnceLock<Mutex<PreviewSlot>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Blocking: the preview texture for `path` — bytes from the on-disk
/// preview cache when present, else a decode of the source at `budget`
/// that is persisted for future runs. GdkTexture is immutable and usable
/// off the main thread, so the heavy decode+`from_bytes` work never
/// blocks the UI.
fn preview_texture(path: &str, budget: u32) -> Result<Arc<gdk::Texture>, String> {
    let disk = preview_disk_path(Path::new(path));
    if let Ok(bytes) = std::fs::read(&disk) {
        if let Ok(tex) = gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)) {
            return Ok(Arc::new(tex));
        }
    }
    let bytes = load_image_jpeg(Path::new(path), budget)?;
    let _ = std::fs::write(&disk, &bytes);
    let tex =
        gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).map_err(|e| e.to_string())?;
    Ok(Arc::new(tex))
}

/// Generate the on-disk preview file if it doesn't exist yet (blocking).
fn generate_preview_file(path: &str, budget: u32) -> Result<(), String> {
    let disk = preview_disk_path(Path::new(path));
    if disk.exists() {
        return Ok(());
    }
    let bytes = load_image_jpeg(Path::new(path), budget)?;
    std::fs::write(&disk, &bytes).map_err(|e| e.to_string())
}

/// Thumbnail texture from the disk cache, if readable/decodable. Also
/// seeds the in-memory cache so the next lookup is a plain hit.
fn texture_from_disk(path: &str) -> Option<Arc<gdk::Texture>> {
    let bytes = std::fs::read(disk_path_for(Path::new(path), "")).ok()?;
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
/// thumbnails and previews are unlinked so the cache dir stays honest.
pub fn retain_only(kept: &[String]) {
    cache()
        .lock()
        .unwrap()
        .retain(|path, _| kept.contains(path));
    // The preview slot holds at most one entry — drop it if its favorite
    // is gone.
    let mut slot = preview_slot().lock().unwrap();
    if let Some((path, _)) = slot.as_ref() {
        if !kept.contains(path) {
            *slot = None;
        }
    }
    drop(slot);
    let kept_stems: std::collections::HashSet<&str> = kept
        .iter()
        .filter_map(|p| Path::new(p).file_stem().and_then(|s| s.to_str()))
        .collect();
    if let Ok(entries) = std::fs::read_dir(disk_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            // `file_stem` of "abc.preview.jpg" is "abc.preview", so strip
            // both cache-file suffixes explicitly before comparing with
            // the favorites' stems.
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let stem = name
                .strip_suffix(".preview.jpg")
                .or_else(|| name.strip_suffix(".jpg"))
                .unwrap_or(name);
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
    // Split lookups so the mutex guard is dropped before `texture_from_disk`
    // re-locks the cache (a guard temporary in an `if let` scrutinee would
    // otherwise outlive into the `or_else` and deadlock).
    let cached = cache().lock().unwrap().get(path).cloned();
    if let Some(tex) = cached.or_else(|| texture_from_disk(path)) {
        picture.set_paintable(Some(tex.as_ref()));
        return;
    }
    let disk = disk_path_for(Path::new(path), "");
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
    // Same guard-vs-relock hazard as in `load_thumb`: the two lookups must
    // not share one condition expression.
    if cache().lock().unwrap().contains_key(path) {
        return;
    }
    if texture_from_disk(path).is_some() {
        return;
    }
    let disk = disk_path_for(Path::new(path), "");
    let rx = spawn_decode(Path::new(path), THUMB_SIZE, Some(disk));
    let key = path.to_string();
    glib::spawn_future_local(async move {
        let _ = await_texture(key, rx).await;
    });
}

/// Paint `picture` with a screen-sized preview of `path` and report
/// completion via `on_done` (exactly once, success or failure — the caller
/// must use it to stop its spinner).
///
/// Order of speed: single-slot memory hit (instant) → on-disk preview
/// cache → off-thread decode at the monitor-sized budget (persisted to
/// `thumbs/<id>.preview.jpg` for future runs). All disk/decode work runs
/// off the main thread.
pub fn load_preview(path: &str, picture: &gtk::Picture, on_done: impl Fn(bool) + 'static) {
    // Lookup is split from the paint on purpose: a guard temporary in an
    // `if let` scrutinee lives until the end of the whole expression, so
    // chaining a lookup into a call that re-locks the same mutex deadlocks.
    let cached = preview_slot().lock().unwrap().clone();
    if let Some((key, tex)) = cached {
        if key == path {
            picture.set_paintable(Some(tex.as_ref()));
            on_done(true);
            return;
        }
    }
    let budget = preview_budget(); // main-thread gdk access, then off-thread work
    let (tx, rx) = async_channel::unbounded::<Result<Arc<gdk::Texture>, String>>();
    let src = path.to_string();
    crate::tokio_handle().spawn_blocking(move || {
        let _ = tx.send_blocking(preview_texture(&src, budget));
    });
    let pic = picture.downgrade();
    let key = path.to_string();
    glib::spawn_future_local(async move {
        let tex = rx.recv().await.ok().and_then(|r| r.ok());
        if let Some(tex) = &tex {
            *preview_slot().lock().unwrap() = Some((key, tex.clone()));
            if let Some(pic) = pic.upgrade() {
                pic.set_paintable(Some(tex.as_ref()));
            }
        }
        on_done(tex.is_some());
    });
}

/// Pre-decode the preview of a freshly liked wallpaper in the background
/// so its first preview click paints instantly. No-op when the on-disk
/// preview already exists.
pub fn prewarm_preview(path: &str) {
    if preview_disk_path(Path::new(path)).exists() {
        return;
    }
    let budget = preview_budget(); // main-thread gdk access
    let src = path.to_string();
    crate::tokio_handle().spawn_blocking(move || {
        if let Err(e) = generate_preview_file(&src, budget) {
            tracing::debug!("preview prewarm failed for {src}: {e}");
        }
    });
}

/// Generate missing on-disk previews for many favorites, sequentially on
/// one blocking thread — the grid calls this on open so every preview
/// click finds its file ready. Existing files are skipped in O(1).
pub fn prewarm_previews(paths: Vec<String>) {
    let budget = preview_budget(); // main-thread gdk access
    crate::tokio_handle().spawn_blocking(move || {
        for path in paths {
            if let Err(e) = generate_preview_file(&path, budget) {
                tracing::debug!("preview prewarm failed for {path}: {e}");
            }
        }
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
