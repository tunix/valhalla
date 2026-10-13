use serde::Deserialize;

use crate::{config::PixabayCfg, http, source::Candidate};

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    id: u64,
    #[serde(default)]
    page_url: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    user: Option<String>,
    large_image_url: String,
    #[serde(default)]
    webformat_url: Option<String>,
}

pub async fn candidates(cfg: &PixabayCfg) -> Result<Vec<Candidate>, String> {
    let Some(key) = cfg.api_key.clone().filter(|k| !k.is_empty()) else {
        return Err("Pixabay requires an API key (set it in Preferences)".into());
    };

    // Tests point this at a local stub server; see wallhaven.rs.
    let base =
        std::env::var("VALHALLA_PIXABAY_URL").unwrap_or_else(|_| "https://pixabay.com/api/".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("per_page", "24");
        qp.append_pair("safesearch", "true");
        if !cfg.query.is_empty() {
            qp.append_pair("q", &cfg.query);
        }
        if !cfg.category.is_empty() {
            qp.append_pair("category", &cfg.category);
        }
        match cfg.orientation.as_str() {
            "landscape" | "portrait" => {
                qp.append_pair("orientation", &cfg.orientation);
            }
            _ => {}
        }
        qp.append_pair("key", &key);
    }

    let response = http::client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // Pixabay reports credential problems as 400 with a plain-text body.
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::BAD_REQUEST
    {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let lower = body.to_lowercase();
        if lower.contains("invalid") || lower.contains("key") {
            return Err("pixabay: API key rejected (check Preferences)".into());
        }
        return Err(format!("pixabay: HTTP {status}"));
    }

    let response = response
        .error_for_status()
        .map_err(|e| format!("pixabay: {e}"))?;
    let parsed: SearchResponse = response.json().await.map_err(|e| format!("pixabay: {e}"))?;

    Ok(parsed
        .hits
        .into_iter()
        .map(|h| {
            // large_image_url is capped at 1280px; webformat is smaller. Use
            // large for the applied image (the fit checker upscales to cover).
            let image_url = h.large_image_url;
            let thumb_url = h.webformat_url.unwrap_or_else(|| image_url.clone());
            Candidate {
                id: format!("pixabay-{}", h.id),
                source: "pixabay",
                image_url,
                thumb_url,
                web_url: h.page_url.unwrap_or_default(),
                title: h.tags,
                author: h.user,
                dominant_colors: Vec::new(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(api_key: Option<&str>) -> PixabayCfg {
        PixabayCfg {
            query: "nature".into(),
            category: String::new(),
            orientation: "landscape".into(),
            api_key: api_key.map(Into::into),
        }
    }

    #[tokio::test]
    async fn missing_key_short_circuits() {
        let err = candidates(&cfg(None)).await.unwrap_err();
        assert!(err.contains("API key"));
        let err = candidates(&cfg(Some(""))).await.unwrap_err();
        assert!(err.contains("API key"));
    }

    #[test]
    fn hit_shape_parses() {
        let body = r#"{"hits": [{
            "id": 1,
            "page_url": "https://pixabay.com/photos/1/",
            "tags": "forest, fog",
            "user": "Ada",
            "large_image_url": "https://img/l.jpg",
            "webformat_url": "https://img/w.jpg"
        }]}"#;
        let r: SearchResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.hits.len(), 1);
        let h = &r.hits[0];
        assert_eq!(h.large_image_url, "https://img/l.jpg");
        assert_eq!(h.tags.as_deref(), Some("forest, fog"));
    }

    #[test]
    fn empty_hits_ok() {
        let r: SearchResponse = serde_json::from_str(r#"{"totalHits": 0, "hits": []}"#).unwrap();
        assert!(r.hits.is_empty());
    }

    #[tokio::test]
    async fn local_api_maps_hits() {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("VALHALLA_PIXABAY_URL");
            }
        }
        let guard = Guard;
        let payload = r#"{"hits": [{
            "id": 9,
            "page_url": "https://pixabay.com/photos/9/",
            "tags": "sea, cliffs",
            "user": "Grace",
            "large_image_url": "https://img/l9.jpg",
            "webformat_url": "https://img/w9.jpg"
        }]}"#;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "VALHALLA_PIXABAY_URL",
            format!("http://127.0.0.1:{port}/api/"),
        );
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(&stream);
            use std::io::Read as _;
            let mut buf = [0u8; 4096];
            let _ = reader.read(&mut buf);
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
        drop(guard);
        let cands = result.expect("candidates");
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        assert_eq!(c.id, "pixabay-9");
        assert_eq!(c.source, "pixabay");
        assert_eq!(c.image_url, "https://img/l9.jpg");
        assert_eq!(c.thumb_url, "https://img/w9.jpg");
        assert_eq!(c.author.as_deref(), Some("Grace"));
        assert_eq!(c.title.as_deref(), Some("sea, cliffs"));
    }

    #[tokio::test]
    async fn bad_key_is_reported() {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("VALHALLA_PIXABAY_URL");
            }
        }
        let guard = Guard;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "VALHALLA_PIXABAY_URL",
            format!("http://127.0.0.1:{port}/api/"),
        );
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
                    b"HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 22\r\nConnection: close\r\n\r\n[ERROR 400] invalid key",
                )
                .unwrap();
        });

        let result = candidates(&cfg(Some("bad-key"))).await;
        server.join().unwrap();
        drop(guard);
        let err = result.unwrap_err();
        assert!(err.contains("API key"), "error: {err}");
    }
}
