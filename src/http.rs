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

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(color: [u8; 3]) -> Vec<u8> {
        let mut img = image::RgbImage::new(4, 4);
        for px in img.pixels_mut() {
            *px = image::Rgb(color);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    fn cached_path_keeps_short_extensions() {
        let dir = Path::new("/tmp");
        let p = cached_path("https://example.com/a/b.jpg", dir);
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("jpg"));
        let p = cached_path("https://example.com/img.PNG", dir);
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("PNG"));
    }

    #[test]
    fn cached_path_falls_back_to_jpg() {
        let dir = Path::new("/tmp");
        // No dot at all -> extensionless fallback.
        let p = cached_path("https://example.com/image", dir);
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("jpg"));
    }

    #[test]
    fn cached_path_rejects_querystring_as_extension() {
        let dir = Path::new("/tmp");
        // The "extension" candidate contains '/', so it must not be used.
        let p = cached_path("https://example.com/img?x=jpeg", dir);
        assert_eq!(
            p.extension().and_then(|e| e.to_str()),
            Some("jpg")
        );
    }

    #[test]
    fn cached_path_rejects_long_extensions() {
        let dir = Path::new("/tmp");
        // 6+ chars is not a plausible image extension.
        let p = cached_path("https://example.com/img.abcdef", dir);
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("jpg"));
    }

    #[test]
    fn same_url_maps_to_same_file() {
        let dir = Path::new("/tmp");
        let a = cached_path("https://example.com/w.jpg", dir);
        let b = cached_path("https://example.com/w.jpg", dir);
        assert_eq!(a, b);
        let c = cached_path("https://example.com/other.jpg", dir);
        assert_ne!(a, c);
    }

    #[test]
    fn cache_subdirs_exist() {
        // glib user cache dir resolves under XDG_CACHE_HOME; point it at a
        // temp dir to exercise the real dir-creation paths.
        let base = std::env::temp_dir().join(format!("valhalla-http-{}", std::process::id()));
        std::env::set_var("XDG_CACHE_HOME", &base);
        let cd = cache_dir();
        assert!(cd.ends_with("valhalla"));
        assert!(thumbs_dir().is_absolute());
        assert!(wallpapers_dir().is_absolute());
        assert!(thumbs_dir().starts_with(cache_dir()));
        assert!(wallpapers_dir().starts_with(cache_dir()));
        assert!(thumbs_dir().exists());
        assert!(wallpapers_dir().exists());
    }

    #[tokio::test]
    async fn download_cached_serves_local_http() {
        // Tiny local HTTP server: verifies the hit path + download path
        // without any external network.
        let dir = std::env::temp_dir().join(format!("valhalla-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let payload = png_bytes([10, 20, 30]);
        let payload_for_thread = payload.clone();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let payload = payload_for_thread;
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&stream);
            use std::io::Read as _;
            let mut buf = [0u8; 1024];
            let _ = reader.read(&mut buf);
            let mut stream = stream;
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            use std::io::Write as _;
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&payload).unwrap();
        });

        let url = format!("http://127.0.0.1:{port}/img.png");
        let p1 = download_cached(&url, &dir).await.expect("first download");
        server.join().unwrap();
        assert_eq!(std::fs::read(&p1).unwrap(), payload);

        // Second call hits the cache (no server running anymore).
        let p2 = download_cached(&url, &dir).await.expect("cached hit");
        assert_eq!(p1, p2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn download_image_rejects_non_image() {
        let dir = std::env::temp_dir().join(format!("valhalla-dl2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = b"plain text, not an image".to_vec();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&stream);
            use std::io::Read as _;
            let mut buf = [0u8; 1024];
            let _ = reader.read(&mut buf);
            let mut stream = stream;
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            use std::io::Write as _;
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });

        let url = format!("http://127.0.0.1:{port}/fake.png");
        let result = download_image(&url, &dir).await;
        server.join().unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a valid image"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
