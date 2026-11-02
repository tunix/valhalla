//! Manual source: folders the user picked — every image inside (one level,
//! non-recursive) becomes a candidate. Persisted as an absolute path list;
//! candidates cycle from local files, no network. Theme-matching runs on
//! the local image via the scheduler's brightness analysis, same as
//! Favorites. Legacy `manual-paths` entries that point at files keep
//! working: they are kept in the list and still served.

use gio::prelude::*;
use std::path::{Path, PathBuf};

/// Settings key storing the picked paths (newline-separated). GSettings
/// string arrays would be nicer but a plain string survives the simple
/// schema and the few-path scale we target.
const PATHS_KEY: &str = "manual-paths";

/// Same settings instance the app/daemon uses.
pub fn settings() -> gio::Settings {
    crate::config::settings()
}

pub fn paths_with(s: &gio::Settings) -> Vec<PathBuf> {
    s.string(PATHS_KEY)
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Replace the picked set (files + directories already expanded by the
/// caller). Persists as newline-separated absolute paths.
pub fn store_with(s: &gio::Settings, paths: &[PathBuf]) -> Result<(), String> {
    let joined = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    s.set_string(PATHS_KEY, &joined).map_err(|e| e.to_string())
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

/// Add folders (and, for legacy callers, individual image files) to the
/// stored list. Duplicates are skipped; order is preserved. Returns the
/// entries actually added.
pub fn add(picked: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut current = paths();
    let mut added = Vec::new();
    for p in picked {
        if p.is_dir() {
            if current.contains(p) || added.contains(p) {
                continue; // folder already tracked
            }
            added.push(p.clone());
        } else if is_image_file(p) && !current.contains(p) && !added.contains(p) {
            added.push(p.clone());
        }
        // Legacy file entries are only queued in `added` here — they are
        // appended to `current` once below, so they are never duplicated.
    }
    current.extend(added.iter().cloned());
    store(&current)?;
    Ok(added)
}

/// Drop a path (a tracked folder or a legacy file entry) from the list.
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
/// multiple folders loop together; legacy file entries are served too.
/// Vanished folders/files are pruned lazily.
pub fn candidates() -> Result<Vec<crate::source::Candidate>, String> {
    let stored = paths();
    if stored.is_empty() {
        return Err("No wallpapers selected — pick a folder in Preferences".into());
    }
    let mut cands = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in stored {
        let items: Vec<PathBuf> = if p.is_dir() {
            match images_in_dir(&p) {
                Ok(items) => items,
                Err(e) => {
                    tracing::warn!("manual: {e}");
                    Vec::new()
                }
            }
        } else if is_image_file(&p) {
            vec![p]
        } else {
            Vec::new() // vanished file or unreadable path — prune lazily
        };
        for image in items {
            if seen.insert(image.clone()) {
                cands.push(candidate_for(&image));
            }
        }
    }
    if cands.is_empty() {
        return Err("No images found in the selected folders".into());
    }
    Ok(cands)
}

fn candidate_for(p: &Path) -> crate::source::Candidate {
    crate::source::Candidate {
        id: format!("manual:{}", p.display()),
        source: "manual",
        image_url: format!("file://{}", p.display()),
        thumb_url: format!("file://{}", p.display()),
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
        // Memory backend persists across tests in one process — start clean.
        settings().set_string(PATHS_KEY, "").unwrap();
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
        s.set_string(PATHS_KEY, "").unwrap();
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

        // 5) legacy file entries still work alongside folders
        let added = add(std::slice::from_ref(&a)).expect("add legacy file");
        assert_eq!(added, vec![a.clone()]);
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 3);
        let cand_a = cands.iter().find(|c| c.id.contains("a.jpg")).unwrap();
        assert!(cand_a.image_url.starts_with("file://"));
        assert_eq!(cand_a.title.as_deref(), Some("a"));

        // 6) duplicates are skipped (folder and file)
        let added = add(&[tmp.join("folder"), a.clone()]).expect("re-add");
        assert!(added.is_empty());
        assert_eq!(paths().len(), 3);

        // 7) remove drops a tracked folder; the other folder + file remain
        remove(&tmp.join("folder")).expect("remove");
        let all = paths();
        assert_eq!(all.len(), 2);
        assert!(!all.contains(&tmp.join("folder")));
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 2, "folder2 + legacy file");

        // 8) vanished folder pruned lazily at candidates()
        let ghost = tmp.join("gone");
        fs::create_dir_all(&ghost).unwrap();
        add(std::slice::from_ref(&ghost)).unwrap();
        fs::remove_dir(&ghost).unwrap();
        let cands = candidates().expect("candidates");
        assert!(!cands.iter().any(|c| c.id.contains("gone")));

        // 9) non-image file add is skipped (not a dir, not an image ext)
        let added = add(std::slice::from_ref(&txt)).expect("add txt");
        assert!(added.is_empty(), "non-image file must be skipped");

        // 10) empty-but-tracked folders give a graceful error, not a panic
        s.set_string(PATHS_KEY, "").unwrap();
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

        // whitespace/empty lines in the stored string are skipped
        s.set_string(PATHS_KEY, "\n/tmp/a.jpg\n\n/tmp/b.jpg\n")
            .unwrap();
        let read = paths_with(&s);
        assert_eq!(read.len(), 2, "empty lines must be skipped");
    }
}
