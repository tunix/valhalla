//! Manual source: wallpapers the user picked — N files and/or a directory
//! (all images inside). Persisted as an absolute path list; candidates
//! cycle from local files, no network. Theme-matching runs on the local
//! image via the scheduler's brightness analysis, same as Favorites.

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

/// Expand `paths` (files kept, directories expanded one level — non-
/// recursive by design: the user picked a folder, not a tree) into the
/// stored list. Skips non-images and duplicates, preserves order.
pub fn add(picked: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut current = paths();
    let mut added = Vec::new();
    for p in picked {
        if p.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(p)
                .map_err(|e| format!("read_dir {}: {e}", p.display()))?
                .filter_map(|r| r.ok())
                .map(|r| r.path())
                .filter(|f| {
                    f.is_file()
                        && f.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
                            .unwrap_or(false)
                })
                .collect();
            entries.sort(); // deterministic order within a directory
            for e in entries {
                if !current.contains(&e) && !added.contains(&e) {
                    current.push(e.clone());
                    added.push(e);
                }
            }
        } else if p.is_file()
            && p.extension()
                .and_then(|e| e.to_str())
                .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
                .unwrap_or(false)
            && !current.contains(p)
            && !added.contains(p)
        {
            current.push(p.clone());
            added.push(p.clone());
        }
    }
    store(&current)?;
    Ok(added)
}

/// Drop a path from the stored list.
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
/// Favorites — the scheduler short-circuits download + analysis).
pub fn candidates() -> Result<Vec<crate::source::Candidate>, String> {
    let paths = paths();
    if paths.is_empty() {
        return Err("No wallpapers selected — pick files or a folder in Preferences".into());
    }
    Ok(paths
        .into_iter()
        .filter(|p| p.is_file()) // prune vanished files lazily
        .map(|p| {
            let id = format!("manual:{}", p.display());
            crate::source::Candidate {
                id: id.clone(),
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
        })
        .collect())
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
        let s = temp_settings("manual");
        // Memory backend persists across tests in one process — start clean.
        settings().set_string(PATHS_KEY, "").unwrap();
        let tmp = std::env::temp_dir().join(format!("valhalla-manual-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("folder")).unwrap();

        let a = make_image(&tmp, "a.jpg");
        let b = make_image(&tmp, "b.png");
        let c = make_image(&tmp.join("folder"), "c.jpg");
        let txt = tmp.join("notes.txt");
        fs::write(&txt, b"not an image").unwrap();

        // 1) empty state (reset may race other tests on the shared memory
        // backend, so assert on the reset itself rather than isolation)
        s.set_string(PATHS_KEY, "").unwrap();
        assert!(paths().is_empty());
        assert!(candidates().is_err()); // graceful empty error

        // 2) add files
        let added = add(&[a.clone(), b.clone()]).expect("add files");
        assert_eq!(added.len(), 2);
        assert_eq!(paths().len(), 2);

        // 3) add a directory — images only, sorted, no notes.txt
        let added = add(&[tmp.join("folder")]).expect("add dir");
        assert_eq!(added, vec![c.clone()]);
        let all = paths();
        assert_eq!(all.len(), 3);
        assert!(!all.contains(&txt), "non-image must be skipped");

        // 4) duplicates are skipped
        let added = add(&[a.clone(), tmp.join("folder")]).expect("re-add");
        assert!(added.is_empty());
        assert_eq!(paths().len(), 3);

        // 5) candidates carry file:// URLs and stem titles
        let cands = candidates().expect("candidates");
        assert_eq!(cands.len(), 3);
        let cand_a = cands.iter().find(|c| c.id.contains("a.jpg")).unwrap();
        assert!(cand_a.image_url.starts_with("file://"));
        assert!(cand_a.image_url.ends_with("a.jpg"));
        assert_eq!(cand_a.source, "manual");
        assert_eq!(cand_a.title.as_deref(), Some("a"));

        // 6) remove drops the entry
        remove(&a).expect("remove");
        let all = paths();
        assert_eq!(all.len(), 2);
        assert!(!all.contains(&a));

        // 7) vanished file pruned lazily at candidates()
        let stale = make_image(&tmp, "stale.jpg");
        let mut all = paths();
        all.push(stale.clone());
        store_with(&s, &all).expect("store");
        fs::remove_file(&stale).unwrap();
        let cands = candidates().expect("candidates");
        assert!(!cands.iter().any(|c| c.id.contains("stale")));

        // 8) non-image file add is skipped (not a dir, but not an image ext)
        let added = add(std::slice::from_ref(&txt)).expect("add txt");
        assert!(added.is_empty(), "non-image file must be skipped");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn store_roundtrip_persists() {
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
