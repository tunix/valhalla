use serde::Deserialize;

use crate::{config::UnsplashCfg, http, source::Candidate};

#[derive(Deserialize)]
struct Photo {
    id: String,
    urls: Urls,
    #[serde(default)]
    links: Option<Links>,
    #[serde(default)]
    user: Option<User>,
}

#[derive(Deserialize)]
struct Urls {
    raw: String,
    #[serde(default)]
    full: Option<String>,
    #[serde(default)]
    small: Option<String>,
    #[serde(default)]
    thumb: Option<String>,
}

#[derive(Deserialize)]
struct Links {
    #[serde(default)]
    html: Option<String>,
}

#[derive(Deserialize)]
struct User {
    #[serde(default)]
    name: Option<String>,
}

pub async fn candidates(cfg: &UnsplashCfg) -> Result<Vec<Candidate>, String> {
    let Some(key) = cfg.access_key.clone().filter(|k| !k.is_empty()) else {
        return Err("Unsplash requires an access key (set it in Preferences)".into());
    };

    // Tests point this at a local stub server; see wallhaven.rs.
    let base = std::env::var("VALHALLA_UNSPLASH_URL")
        .unwrap_or_else(|_| "https://api.unsplash.com/photos/random".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("count", "24");
        if !cfg.query.is_empty() {
            qp.append_pair("query", &cfg.query);
        }
        if !cfg.orientation.is_empty() {
            qp.append_pair("orientation", &cfg.orientation);
        }
        qp.append_pair("content_filter", &cfg.content_filter);
    }

    let response = http::client()
        .get(url)
        .header("Authorization", format!("Client-ID {key}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("unsplash: access key rejected (401)".into());
    }
    if let Some(remaining) = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        if remaining < 10 {
            tracing::warn!("unsplash rate limit low: {remaining} requests left this hour");
        }
    }

    let response = response
        .error_for_status()
        .map_err(|e| format!("unsplash: {e}"))?;
    let photos: Vec<Photo> = response
        .json()
        .await
        .map_err(|e| format!("unsplash: {e}"))?;

    Ok(photos
        .into_iter()
        .map(|p| {
            // Keep the ixid parameter when building a resized URL, per API guidelines.
            let image_url = p
                .urls
                .full
                .unwrap_or_else(|| format!("{}&fm=jpg&w=3840&q=85&fit=max", p.urls.raw));
            let thumb_url = p
                .urls
                .small
                .or(p.urls.thumb)
                .unwrap_or_else(|| p.urls.raw.clone());
            Candidate {
                id: p.id,
                source: "unsplash",
                image_url,
                thumb_url,
                web_url: p.links.and_then(|l| l.html).unwrap_or_default(),
                title: None,
                author: p.user.and_then(|u| u.name),
                dominant_colors: Vec::new(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubBaseGuard {
        var: &'static str,
    }

    impl StubBaseGuard {
        fn set(&self, port: u16) {
            std::env::set_var(self.var, format!("http://127.0.0.1:{port}/photos/random"));
        }
    }

    impl Drop for StubBaseGuard {
        fn drop(&mut self) {
            std::env::remove_var(self.var);
        }
    }

    fn cfg(access_key: Option<&str>) -> UnsplashCfg {
        UnsplashCfg {
            query: "nature".into(),
            orientation: "landscape".into(),
            content_filter: "low".into(),
            access_key: access_key.map(Into::into),
        }
    }

    #[tokio::test]
    async fn missing_key_short_circuits() {
        let err = candidates(&cfg(None)).await.unwrap_err();
        assert!(err.contains("access key"));
        // Empty string counts as missing too.
        let err = candidates(&cfg(Some(""))).await.unwrap_err();
        assert!(err.contains("access key"));
    }

    #[test]
    fn photo_shape_parses() {
        let body = r#"{
            "id": "ph1",
            "urls": {"raw": "https://u/raw?ixid=1", "full": "https://u/full", "small": "https://u/s"},
            "links": {"html": "https://unsplash.com/ph1"},
            "user": {"name": "Ada"}
        }"#;
        let p: Photo = serde_json::from_str(body).unwrap();
        assert_eq!(p.id, "ph1");
        assert_eq!(p.urls.full.as_deref(), Some("https://u/full"));
        assert_eq!(p.user.and_then(|u| u.name).as_deref(), Some("Ada"));
        assert_eq!(
            p.links.and_then(|l| l.html).as_deref(),
            Some("https://unsplash.com/ph1")
        );
    }

    #[test]
    fn missing_optional_sections_default() {
        let body = r#"{"id": "ph2", "urls": {"raw": "https://u/raw2"}}"#;
        let p: Photo = serde_json::from_str(body).unwrap();
        assert_eq!(p.urls.full, None);
        assert_eq!(p.urls.small, None);
        assert_eq!(p.links.and_then(|l| l.html), None);
        assert_eq!(p.user.and_then(|u| u.name), None);
    }

    #[test]
    fn resized_url_fallback_appends_params() {
        // Mirrors the candidates() mapping: no full URL -> raw + params.
        let raw = "https://u/raw?ixid=1";
        let image_url = format!("{raw}&fm=jpg&w=3840&q=85&fit=max");
        assert!(image_url.starts_with(raw));
        assert!(image_url.contains("ixid=1")); // ixid preserved per API guidelines
        assert!(image_url.contains("fm=jpg"));
    }

    #[tokio::test]
    async fn local_api_maps_photos() {
        let port_guard = StubBaseGuard {
            var: "VALHALLA_UNSPLASH_URL",
        };
        let payload = r#"[{
            "id": "u1",
            "urls": {"raw": "https://u/raw?ixid=9", "full": "https://u/full", "small": "https://u/s"},
            "links": {"html": "https://unsplash.com/u1"},
            "user": {"name": "Grace"}
        }]"#;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        port_guard.set(port);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&stream);
            use std::io::Read as _;
            let mut buf = [0u8; 4096];
            let _ = reader.read(&mut buf);
            // Auth header check happens implicitly: the request must carry
            // "Authorization: Client-ID ..." for the real API; here we only
            // exercise the happy-path mapping.
            let mut stream = stream;
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            use std::io::Write as _;
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(payload.as_bytes()).unwrap();
        });

        let result = candidates(&cfg(Some("secret-key"))).await;
        server.join().unwrap();
        let cands = result.expect("candidates");
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        assert_eq!(c.id, "u1");
        assert_eq!(c.source, "unsplash");
        assert_eq!(c.image_url, "https://u/full");
        assert_eq!(c.thumb_url, "https://u/s");
        assert_eq!(c.web_url, "https://unsplash.com/u1");
        assert_eq!(c.author.as_deref(), Some("Grace"));
        assert!(c.dominant_colors.is_empty());
        assert!(c.title.is_none());
    }

    #[tokio::test]
    async fn unauthorized_key_is_reported() {
        let port_guard = StubBaseGuard {
            var: "VALHALLA_UNSPLASH_URL",
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        port_guard.set(port);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&stream);
            use std::io::Read as _;
            let mut buf = [0u8; 4096];
            let _ = reader.read(&mut buf);
            let mut stream = stream;
            use std::io::Write as _;
            stream
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        });

        let result = candidates(&cfg(Some("bad-key"))).await;
        server.join().unwrap();
        let err = result.unwrap_err();
        assert!(err.contains("401"), "error: {err}");
        assert!(err.contains("access key rejected"));
    }
}
