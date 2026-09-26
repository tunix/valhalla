use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use reqwest::Client;

static CLIENT: OnceLock<Client> = OnceLock::new();

pub fn client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(concat!("valhalla/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub fn cache_dir() -> PathBuf {
    let dir = glib::user_cache_dir().join("valhalla");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn thumbs_dir() -> PathBuf {
    let dir = cache_dir().join("thumbs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn wallpapers_dir() -> PathBuf {
    let dir = cache_dir().join("wallpapers");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn url_hash(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn cached_path(url: &str, dir: &Path) -> PathBuf {
    let name = url_hash(url);
    let ext = url
        .rsplit('.')
        .next()
        .filter(|e| !e.is_empty() && e.len() <= 5 && !e.contains('/'))
        .unwrap_or("jpg");
    dir.join(format!("{name}.{ext}"))
}

/// Download `url` into the cache, skipping the request when already present.
pub async fn download_cached(url: &str, dir: &Path) -> Result<PathBuf, String> {
    let dest = cached_path(url, dir);
    if dest.exists() {
        return Ok(dest);
    }
    let tmp = dest.with_extension("part");
    let bytes = client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Download an image and verify it actually decodes (guards against error pages).
pub async fn download_image(url: &str, dir: &Path) -> Result<PathBuf, String> {
    let dest = download_cached(url, dir).await?;
    let bytes = std::fs::read(&dest).map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| format!("not a valid image: {e}"))?;
    Ok(dest)
}
