//! Manual source: folders the user picked — every image inside (one level,
//! non-recursive) becomes a candidate. Persisted as an absolute path list;
//! candidates cycle from local files, no network. Theme-matching runs on
//! the local image via the scheduler's brightness analysis, same as
//! Favorites — against a small cached thumbnail (one per file version), so
//! a refresh never full-decodes the actual wallpapers and stays instant.
//! Single-file entries are not supported: only directories are accepted,
//! and file entries that survive from very old builds are pruned.

use gio::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Settings key storing the picked paths as a GSettings string array
/// (`as`) — one path per element, the native dconf representation.
const PATHS_KEY: &str = "manual-folders";

/// Older builds stored the paths in `manual-paths` as a single
/// newline-separated string. That key is kept in the schema (type `s`) so
/// existing user data stays readable — a type change would make GSettings
/// silently drop the old value — and is migrated to [`PATHS_KEY`] on first
/// read, then reset to its default.
const LEGACY_KEY: &str = "manual-paths";

/// Same settings instance the app/daemon uses.
pub fn settings() -> gio::Settings {
    crate::config::settings()
}

/// One-time conversion of the legacy `manual-paths` string into the
/// `manual-folders` array. Idempotent: the legacy key is reset after a
/// successful migration, so a second call finds no user value and does
/// nothing. Never clobbers newer `manual-folders` data.
fn migrate_legacy(s: &gio::Settings) {
    if s.user_value(PATHS_KEY).is_some() {
        return; // already using the array key — leave it alone
    }
    let Some(user) = s.user_value(LEGACY_KEY) else {
        return; // nothing to migrate
    };
    // Old format: one string, paths joined by newlines.
    let legacy: Vec<String> = user
        .str()
        .unwrap_or("")
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    if legacy.is_empty() {
        return;
    }
    if s.set_strv(PATHS_KEY, legacy).is_ok() {
        s.reset(LEGACY_KEY); // migration complete: drop the old value
    }
}

pub fn paths_with(s: &gio::Settings) -> Vec<PathBuf> {
    migrate_legacy(s);
    s.strv(PATHS_KEY)
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Replace the picked set (folders + legacy file entries). Persists as a
/// GSettings string array.
pub fn store_with(s: &gio::Settings, paths: &[PathBuf]) -> Result<(), String> {
    let strv: Vec<String> = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    s.set_strv(PATHS_KEY, strv).map_err(|e| e.to_string())
}

/// Convenience wrappers: open the default settings themselves (UI callers
/// pass no widget state, keeping closures Send).
pub fn paths() -> Vec<PathBuf> {
    paths_with(&settings())
}

pub fn store(paths: &[PathBuf]) -> Result<(), String> {
    store_with(&settings(), paths)
}

/// Image extensions we accept when expanding a directory.
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "avif"];

fn is_image_file(p: &Path) -> bool {
    p.is_file()
        && p.extension()
            .and_then(|e| e.to_str())
            .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

/// Images directly inside `dir` (one level, sorted deterministically).
fn images_in_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("read_dir {}: {e}", dir.display()))?
        .filter_map(|r| r.ok())
        .map(|r| r.path())
        .filter(|f| is_image_file(f))
        .collect();
    entries.sort();
    Ok(entries)
}

/// Add folders to the stored list. Duplicates are skipped; order is
/// preserved. Returns the entries actually added. Single files are not
/// accepted — the manual source cycles folders.
pub fn add(picked: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut current = paths();
    let mut added = Vec::new();
    for p in picked {
        if !p.is_dir() {
            continue; // files and nonexistent paths are not tracked
        }
        if current.contains(p) || added.contains(p) {
            continue; // folder already tracked
        }
        added.push(p.clone());
    }
    current.extend(added.iter().cloned());
    store(&current)?;
    Ok(added)
}

/// Drop a tracked folder from the list.
pub fn remove(path: &Path) -> Result<(), String> {
    let current = paths();
    let remaining: Vec<PathBuf> = current
        .iter()
        .filter(|p| p.as_path() != path)
        .cloned()
        .collect();
    if remaining.len() != current.len() {
        store(&remaining)?;
    }
    Ok(())
}

/// Present the manual set as refreshable candidates (file:// URLs like
/// Favorites — the scheduler short-circuits download + analysis). Every
/// tracked folder contributes all images inside it (one level), so
/// multiple folders loop together. Single-file entries are not served:
/// leftovers from very old builds are pruned from the store on the first
/// refresh (they cannot come back — nothing writes them anymore).
/// Vanished folders are left in place: a temporarily unmounted folder
/// must not lose its slot, it just contributes nothing until it returns.
pub fn candidates() -> Result<Vec<crate::source::Candidate>, String> {
    let stored = paths();
    if stored.is_empty() {
        return Err("No wallpapers selected — pick a folder in Preferences".into());
    }
    let mut cands = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stale_files = Vec::new();
    for p in stored {
        let items: Vec<PathBuf> = if p.is_dir() {
            match images_in_dir(&p) {
                Ok(items) => items,
                Err(e) => {
                    tracing::warn!("manual: {e}");
                    Vec::new()
                }
            }
        } else {
            // Not a directory: a leftover file entry (pruned below) or a
            // vanished/unreadable path (kept — it may return).
            if p.is_file() {
                tracing::debug!("manual: pruning file entry {p:?}");
                stale_files.push(p);
            }
            Vec::new()
        };
        for image in items {
            if seen.insert(image.clone()) {
                cands.push(candidate_for(&image));
            }
        }
    }
    if !stale_files.is_empty() {
        let remaining: Vec<PathBuf> = paths()
            .into_iter()
            .filter(|p| !stale_files.contains(p))
            .collect();
        if let Err(e) = store(&remaining) {
            tracing::debug!("manual: pruning stale file entries failed: {e}");
        }
    }
    if cands.is_empty() {
        return Err("No images found in the selected folders".into());
    }
    Ok(cands)
}

/// Analysis-thumbnail cache key: path + file size + mtime. Editing or
/// replacing a folder's image produces a fresh key, so stale thumbnails
/// never serve analysis; unchanged files hit the cached copy instantly.
fn thumb_key(p: &Path) -> String {
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
    let mut hasher = DefaultHasher::new();
    p.hash(&mut hasher);
    hasher.write_u64(size);
    hasher.write_u128(mtime);
    format!("{:016x}", hasher.finish())
}

/// Small pre-scaled analysis copy for a manual image, shared with the
/// Favorites thumbnail store (`thumbs/`), keyed by path+size+mtime —
/// generated once per file version, reused on every refresh. Mirrors
/// `favorites::ensure_thumb`: without it the scheduler would full-decode
/// every wallpaper on each refresh, seconds of work that feels exactly
/// like a network download.
fn ensure_thumb(p: &Path) -> PathBuf {
    let key = thumb_key(p);
    let dest = crate::favorites::thumb_path(&format!("manual-{key}"));
    if dest.exists() {
        return dest;
    }
    let generated = (|| -> Result<PathBuf, String> {
        // Same bound as favorites: brightness sampling downscales to 64×64
        // anyway, and small sources are analyzed directly (no decode).
        let dims = image::ImageReader::open(p)
            .and_then(|r| r.with_guessed_format())
            .map_err(|e| e.to_string())?
            .into_dimensions()
            .map_err(|e| e.to_string())?;
        if dims.0.max(dims.1) <= crate::favorites::THUMB_MAX_EDGE {
            return Ok(p.to_path_buf());
        }
        let img = image::ImageReader::open(p)
            .and_then(|r| r.with_guessed_format())
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;
        let small = img.thumbnail(
            crate::favorites::THUMB_MAX_EDGE,
            crate::favorites::THUMB_MAX_EDGE,
        );
        // tmp + rename so a concurrent refresh never sees a half-written
        // thumbnail (same pattern as favorites::ensure_thumb).
        let tmp = crate::favorites::thumbs_dir().join(format!("manual-{key}.tmp"));
        small
            .write_to(
                &mut std::io::BufWriter::new(
                    std::fs::File::create(&tmp).map_err(|e| e.to_string())?,
                ),
                image::ImageFormat::Jpeg,
            )
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
        Ok(dest)
    })();
    match generated {
        Ok(path) => path,
        Err(e) => {
            tracing::debug!("thumbnail for manual {p:?} not generated: {e}");
            p.to_path_buf() // scheduler falls back to the full-size decode
        }
    }
}

fn candidate_for(p: &Path) -> crate::source::Candidate {
    crate::source::Candidate {
        id: format!("manual:{}", p.display()),
        source: "manual",
        image_url: format!("file://{}", p.display()),
        thumb_url: format!("file://{}", ensure_thumb(p).display()),
        web_url: String::new(),
        title: p
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|n| n.to_string()),
        author: None,
        dominant_colors: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_settings(_tag: &str) -> gio::Settings {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        gio::Settings::new("io.github.tunix.valhalla")
    }

    fn make_image(dir: &Path, name: &str) -> PathBuf {
        use image::Rgb;
        let mut img = image::RgbImage::new(4, 4);
        for px in img.pixels_mut() {
            *px = Rgb([10, 20, 30]);
        }
        let path = dir.join(name);
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::BufWriter::new(fs::File::create(&path).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        path
    }

    // All manual tests in one serial body: GSETTINGS_BACKEND is process-global.
    #[test]
    fn manual_lifecycle_serial() {
        // GSETTINGS_BACKEND and the shared memory backend are process-global:
        // hold ENV_LOCK for the whole body (see crate::http::test_env).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let s = temp_settings("manual");
        // Memory backend persists across tests in one process — start clean
        // on BOTH keys (the legacy one would otherwise migrate into runs).
        settings().reset(PATHS_KEY);
        settings().reset(LEGACY_KEY);
        let tmp = std::env::temp_dir().join(format!("valhalla-manual-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("folder")).unwrap();
        fs::create_dir_all(tmp.join("folder2")).unwrap();

        let a = make_image(&tmp, "a.jpg");
        let _b = make_image(&tmp, "b.png");
        let _c = make_image(&tmp.join("folder"), "c.jpg");
        let _d = make_image(&tmp.join("folder2"), "d.jpg");
        let txt = tmp.join("notes.txt");
        fs::write(&txt, b"not an image").unwrap();

        // 1) empty state (reset may race other tests on the shared memory
        // backend, so assert on the reset itself rather than isolation)
        s.reset(PATHS_KEY);
        assert!(paths().is_empty());
        assert!(candidates().is_err()); // graceful empty error

        // 2) folders are stored as folders, not expanded at add time
        let added = add(&[tmp.join("folder")]).expect("add dir");
        assert_eq!(added, vec![tmp.join("folder")]);

        // 3) candidates expand the folder: images only, sorted, no notes.txt
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 1);
        assert!(cands[0].image_url.ends_with("c.jpg"));
        assert_eq!(cands[0].source, "manual");
        assert_eq!(cands[0].title.as_deref(), Some("c"));

        // 4) a second folder loops together with the first; duplicates
        // across folders are served once
        let added = add(&[tmp.join("folder2")]).expect("add second dir");
        assert_eq!(added.len(), 1);
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 2, "both folders must contribute");

        // 5) single files are not accepted at all
        let added = add(std::slice::from_ref(&a)).expect("add file rejected");
        assert!(added.is_empty(), "single files must not be tracked");
        assert_eq!(paths().len(), 2);

        // 6) duplicate folders are skipped
        let added = add(&[tmp.join("folder"), tmp.join("folder2")]).expect("re-add");
        assert!(added.is_empty());
        assert_eq!(paths().len(), 2);

        // 7) remove drops a tracked folder; the other folder remains
        remove(&tmp.join("folder")).expect("remove");
        let all = paths();
        assert_eq!(all.len(), 1);
        assert!(!all.contains(&tmp.join("folder")));
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 1, "folder2 only");

        // 8) vanished folder pruned lazily at candidates()
        let ghost = tmp.join("gone");
        fs::create_dir_all(&ghost).unwrap();
        add(std::slice::from_ref(&ghost)).unwrap();
        fs::remove_dir(&ghost).unwrap();
        let cands = candidates().expect("candidates");
        assert!(!cands.iter().any(|c| c.id.contains("gone")));

        // 9) non-image file add is skipped (not a dir)
        let added = add(std::slice::from_ref(&txt)).expect("add txt");
        assert!(added.is_empty(), "non-image file must be skipped");

        // 10) file entries left by very old builds are pruned from the
        // store on the first refresh and never served
        store_with(&s, &[tmp.join("folder2"), a.clone(), txt.clone()]).unwrap();
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 1, "only the folder contributes");
        let all = paths();
        assert_eq!(all, vec![tmp.join("folder2")], "file entries pruned");

        // 11) empty-but-tracked folders give a graceful error, not a panic
        s.reset(PATHS_KEY);
        // nonexistent path is neither dir nor file -> skipped silently
        let added = add(&[tmp.join("empty-never-created")]).expect("nonexistent skipped");
        assert!(added.is_empty());
        fs::create_dir_all(tmp.join("empty")).unwrap();
        add(&[tmp.join("empty")]).unwrap();
        assert!(
            candidates().is_err(),
            "folder with no images must error gracefully"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn store_roundtrip_persists() {
        // Writes PATHS_KEY in the same memory backend as
        // manual_lifecycle_serial — a concurrent store from it lands between
        // the set and the read-back here and flakily fails the assert
        // (run 38472595982 died on exactly this). Must not interleave.
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let s = temp_settings("persist");
        let p1 = PathBuf::from("/tmp/x/one.jpg");
        let p2 = PathBuf::from("/tmp/x/two.png");
        store_with(&s, &[p1.clone(), p2.clone()]).expect("store");
        let read = paths_with(&s);
        assert_eq!(read, vec![p1, p2]);
    }

    #[test]
    fn legacy_string_value_migrates_to_array() {
        // Pre-array builds stored manual-paths as one newline-separated
        // string; the first read migrates it into manual-folders (a real
        // string array — dconf shows @as [...]) and resets the old key.
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let s = temp_settings("migrate");
        s.reset(PATHS_KEY);
        s.set_string(LEGACY_KEY, "/tmp/a.jpg\n/tmp/b dir\n")
            .unwrap();
        let read = paths_with(&s);
        assert_eq!(
            read,
            vec![PathBuf::from("/tmp/a.jpg"), PathBuf::from("/tmp/b dir")]
        );
        // The migration persisted: the new key holds a real array (dconf
        // shows @as [...]) with both paths, and the legacy key is back at
        // its default — no lingering user data.
        assert_eq!(
            s.user_value(PATHS_KEY).unwrap().type_().as_str(),
            "as",
            "migrated value must be a string array"
        );
        assert!(s.user_value(LEGACY_KEY).is_none());
        // Idempotent: a second read is a no-op and keeps the data.
        assert_eq!(paths_with(&s), read);

        // New data written after a migration is never clobbered by the
        // legacy key.
        store_with(&s, &[PathBuf::from("/tmp/new.png")]).unwrap();
        assert_eq!(paths_with(&s), vec![PathBuf::from("/tmp/new.png")]);
        s.reset(PATHS_KEY);
        s.reset(LEGACY_KEY);
    }

    #[test]
    fn thumb_cache_reuses_and_invalidates() {
        // Analysis thumbnails are cached under the favorites thumbs store,
        // keyed by path+size+mtime: unchanged files reuse the cached copy,
        // an edited file gets a fresh one. XDG_DATA_HOME is process-global —
        // held under ENV_LOCK like every other env-mutating test.
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp =
            std::env::temp_dir().join(format!("valhalla-manual-thumb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // A large image (> 256px) forces a real thumbnail generation.
        let big = tmp.join("big.jpg");
        let img = image::RgbImage::from_pixel(512, 512, image::Rgb([200, 100, 50]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::BufWriter::new(fs::File::create(&big).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();

        // First candidates() call generates the thumbnail; the served
        // thumb_url points at the small cached copy, not the full image.
        // (The manual source tracks folders, so the image is stored inside
        // the tracked directory.)
        store(std::slice::from_ref(&tmp)).unwrap();
        let cands = candidates().expect("candidates");
        let thumb = PathBuf::from(cands[0].thumb_url.strip_prefix("file://").unwrap());
        assert!(thumb.is_file(), "cached thumbnail must exist");
        assert_ne!(thumb, big, "thumb_url must be the cached copy");
        let dims = image::ImageReader::open(&thumb)
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert!(dims.0.max(dims.1) <= crate::favorites::THUMB_MAX_EDGE);

        // Second call with an unchanged file reuses the same file (mtime
        // untouched) — the instant-refresh path.
        let mtime = fs::metadata(&thumb).unwrap().modified().unwrap();
        candidates().expect("second candidates");
        assert_eq!(
            fs::metadata(&thumb).unwrap().modified().unwrap(),
            mtime,
            "thumbnail must be reused, not regenerated"
        );

        // Replacing the file (new content -> new size) invalidates the
        // cache: a fresh thumbnail is generated for the new version.
        let img2 = image::RgbImage::from_pixel(600, 400, image::Rgb([10, 200, 80]));
        image::DynamicImage::ImageRgb8(img2)
            .write_to(
                &mut std::io::BufWriter::new(fs::File::create(&big).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        candidates().expect("candidates after edit");
        let cands = candidates().expect("candidates");
        let thumb2 = PathBuf::from(cands[0].thumb_url.strip_prefix("file://").unwrap());
        assert_ne!(thumb2, thumb, "edited file must get a new thumbnail");

        // A corrupt source falls back gracefully: the full-size path is
        // handed to the scheduler (which then skips the candidate), no panic.
        let corrupt = tmp.join("corrupt.jpg");
        fs::write(&corrupt, b"not an image").unwrap();
        let cands = candidates().expect("candidates with corrupt file");
        let c = cands.iter().find(|c| c.id.contains("corrupt")).unwrap();
        assert!(c.thumb_url.ends_with("corrupt.jpg"));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn thumb_analysis_is_far_cheaper_than_full_decode() {
        // The reason for the thumbnail cache, stated as a budget: analyzing
        // a cache-hitting manual candidate must cost a fraction of a full
        // wallpaper decode. Real-size synthetic images (2560×1440 JPEG).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp =
            std::env::temp_dir().join(format!("valhalla-manual-bench-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let big = tmp.join("wide.jpg");
        let img = image::RgbImage::from_pixel(2560, 1440, image::Rgb([120, 130, 140]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::BufWriter::new(fs::File::create(&big).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();

        // Cold: first candidates() generates the thumbnail.
        let cold_start = std::time::Instant::now();
        store(std::slice::from_ref(&tmp)).unwrap();
        let cands = candidates().expect("candidates");
        let cold = cold_start.elapsed();
        let thumb_bytes = fs::read(
            cands[0]
                .thumb_url
                .strip_prefix("file://")
                .expect("file thumb"),
        )
        .unwrap();
        assert!(
            thumb_bytes.len() < 100 * 1024,
            "thumb is small, not the full image"
        );

        // Warm: cache hit — analyze from the cached thumb bytes.
        let warm_start = std::time::Instant::now();
        let cands = candidates().expect("warm candidates");
        let bytes = fs::read(
            cands[0]
                .thumb_url
                .strip_prefix("file://")
                .expect("file thumb"),
        )
        .unwrap();
        crate::brightness::analyze(&bytes).expect("analyze thumb");
        let warm = warm_start.elapsed();

        // What the cache saves: a full-size decode + analysis.
        let full_start = std::time::Instant::now();
        let full_bytes = fs::read(&big).unwrap();
        crate::brightness::analyze(&full_bytes).expect("analyze full");
        let full = full_start.elapsed();

        eprintln!(
            "manual refresh analysis budget: warm(cached)={warm:?}, cold(gen+analyze)={cold:?}, full-decode={full:?}"
        );
        assert!(
            warm < full / 3,
            "cached analysis must be far cheaper than full decode (warm={warm:?}, full={full:?})"
        );

        fs::remove_dir_all(&tmp).ok();
    }
}
