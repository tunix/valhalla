//! Favorite wallpapers: persisted list + local storage under
//! `$XDG_DATA_HOME/valhalla/favorites/`.
//!
//! A favorite is a *copy* of the wallpaper file, not a cache reference:
//! caches are pruned, favorites must survive. Metadata (source, author,
//! title, brightness scheme at like-time) travels with the list entry so
//! the Favorites source can cycle through them without re-fetching.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::theme::ColorScheme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub id: String,
    pub source: String,
    /// Absolute path of the *stored copy* under favorites/.
    pub path: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// Tone at like-time (kept for information; Favorites respects the
    /// current theme when applying, like every other source).
    #[serde(default)]
    pub scheme: Option<String>,
}

pub fn favorites_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local").join("share"));
    let dir = base.join("valhalla").join("favorites");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn list_path() -> PathBuf {
    favorites_dir().join("favorites.json")
}

/// All favorites, oldest first (insertion order = stable cycle order).
pub fn list() -> Vec<Favorite> {
    let text = match std::fs::read_to_string(list_path()) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<Favorite>>(&text).unwrap_or_default()
}

fn save_list(favs: &[Favorite]) -> Result<(), String> {
    let tmp = list_path().with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(favs).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, list_path()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn contains(id: &str) -> bool {
    list().iter().any(|f| f.id == id)
}

/// Copy `src` into favorites/ as `<id>.<ext>` and append to the list.
/// Existing favorite with the same id is a no-op (already liked).
pub fn add(
    id: &str,
    source: &str,
    src: &Path,
    title: Option<String>,
    author: Option<String>,
    scheme: Option<ColorScheme>,
) -> Result<(), String> {
    if contains(id) {
        return Ok(());
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let dest = favorites_dir().join(format!("{}.{}", sanitize_id(id), ext));
    // Copy, not rename: the cache may prune the original later.
    std::fs::copy(src, &dest).map_err(|e| format!("copy failed: {e}"))?;
    let mut favs = list();
    favs.push(Favorite {
        id: id.to_string(),
        source: source.to_string(),
        path: dest.to_string_lossy().into_owned(),
        title,
        author,
        scheme: scheme.map(|s| s.label().to_string()),
    });
    save_list(&favs)
}

/// Remove from the list and delete the stored copy.
pub fn remove(id: &str) -> Result<(), String> {
    let mut favs = list();
    let Some(pos) = favs.iter().position(|f| f.id == id) else {
        return Ok(());
    };
    let fav = favs.remove(pos);
    save_list(&favs)?;
    let _ = std::fs::remove_file(&fav.path); // list entry already gone
    Ok(())
}

/// Present favorites as refreshable candidates. Each entry cycles from the
/// local copy — no network, theme-matching happens on the already-stored
/// image via brightness analysis in the scheduler.
pub fn candidates() -> Result<Vec<crate::source::Candidate>, String> {
    let favs = list();
    if favs.is_empty() {
        return Err("No favorites yet — like a wallpaper with the heart button".into());
    }
    Ok(favs
        .into_iter()
        .map(|f| crate::source::Candidate {
            id: f.id.clone(),
            source: "favorites",
            // The stored copy is the image itself; scheduler's download step
            // short-circuits when the file already exists in cache.
            image_url: format!("file://{}", f.path),
            thumb_url: format!("file://{}", f.path),
            web_url: String::new(),
            title: f.title,
            author: f.author,
            dominant_colors: Vec::new(),
        })
        .collect())
}

/// Ids may come from third-party APIs — keep only filesystem-safe chars.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ColorScheme;

    fn temp_data(tag: &str) {
        let dir = std::env::temp_dir().join(format!("valhalla-fav-{tag}-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &dir);
        let _ = std::fs::remove_dir_all(&dir);
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
                &mut std::io::BufWriter::new(std::fs::File::create(&path).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        path
    }

    #[test]
    fn lifecycle_serial() {
        // All in one test: XDG_DATA_HOME is process-global; parallel tests
        // would race it. Serialized against the other env users via ENV_LOCK
        // (see crate::http::test_env).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        temp_data("life");

        let tmp = std::env::temp_dir().join(format!("valhalla-fav-src-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = make_image(&tmp, "wall.jpg");

        // empty at first
        assert!(list().is_empty());
        assert!(!contains("w1"));

        // add
        add(
            "w1",
            "wallhaven",
            &src,
            Some("Title".into()),
            Some("Ada".into()),
            Some(ColorScheme::Dark),
        )
        .expect("add");
        assert!(contains("w1"));
        let favs = list();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, "w1");
        assert_eq!(favs[0].source, "wallhaven");
        assert_eq!(favs[0].title.as_deref(), Some("Title"));
        assert_eq!(favs[0].author.as_deref(), Some("Ada"));
        assert_eq!(favs[0].scheme.as_deref(), Some("dark"));
        // stored copy exists and is a COPY (original removal doesn't matter)
        let stored = PathBuf::from(&favs[0].path);
        assert!(stored.exists());
        assert!(stored.extension().and_then(|e| e.to_str()) == Some("jpg"));
        assert!(stored.starts_with(favorites_dir()));
        assert_ne!(stored, src, "stored copy, not the cache file");

        // double-add is a no-op
        add("w1", "wallhaven", &src, None, None, None).expect("re-add no-op");
        assert_eq!(list().len(), 1);

        // multiple favorites keep insertion order
        add("w2", "unsplash", &src, None, None, Some(ColorScheme::Light)).expect("add w2");
        let favs = list();
        assert_eq!(favs.len(), 2);
        assert_eq!(favs[0].id, "w1"); // insertion order preserved
        assert_eq!(favs[1].id, "w2");

        // remove deletes list entry AND the stored copy
        remove("w1").expect("remove");
        assert!(!contains("w1"));
        assert!(
            !stored.exists(),
            "stored copy must be deleted with the entry"
        );
        // w2 untouched
        assert!(contains("w2"));
        remove("w2").expect("remove w2");
        assert!(list().is_empty());

        // remove of unknown id is a no-op
        remove("ghost").expect("remove ghost");

        // sanitize: unsafe filename chars replaced
        add("photo/with:weird*chars", "unsplash", &src, None, None, None).expect("add");
        let favs = list();
        let stored = PathBuf::from(&favs.last().unwrap().path);
        let name = stored.file_name().and_then(|n| n.to_str()).unwrap();
        assert!(!name.contains(['/', ':', '*']), "sanitized name: {name}");
        assert!(name.starts_with("photo_with_weird_chars"));
        remove(&favs.last().unwrap().id).ok();

        std::fs::remove_dir_all(&tmp).ok();
        std::env::remove_var("XDG_DATA_HOME");
    }
}
