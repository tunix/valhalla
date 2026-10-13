use serde::Deserialize;

use crate::{config::PexelsCfg, http, source::Candidate};

#[derive(Deserialize)]
struct SearchResponse {
    photos: Vec<Photo>,
}

#[derive(Deserialize)]
struct Photo {
    id: u64,
    #[serde(default)]
    url: Option<String>,
    src: Src,
    #[serde(default)]
    photographer: Option<String>,
    #[serde(default)]
    alt: Option<String>,
    #[serde(default)]
    avg_color: Option<String>,
}

#[derive(Deserialize)]
struct Src {
    original: String,
    #[serde(default)]
    medium: Option<String>,
}

pub async fn candidates(cfg: &PexelsCfg) -> Result<Vec<Candidate>, String> {
    let Some(key) = cfg.api_key.clone().filter(|k| !k.is_empty()) else {
        return Err("Pexels requires an API key (set it in Preferences)".into());
    };

    // Tests point this at a local stub server; see wallhaven.rs.
    let base = std::env::var("VALHALLA_PEXELS_URL")
        .unwrap_or_else(|_| "https://api.pexels.com/v1/search".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("per_page", "24");
        if !cfg.query.is_empty() {
            qp.append_pair("query", &cfg.query);
        }
        if !cfg.orientation.is_empty() {
            qp.append_pair("orientation", &cfg.orientation);
        }
    }

    let response = http::client()
        .get(url)
        .header("Authorization", key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("pexels: API key rejected (401)".into());
    }

    let response = response
        .error_for_status()
        .map_err(|e| format!("pexels: {e}"))?;
    let parsed: SearchResponse = response
        .json()
        .await
        .map_err(|e| format!("pexels: {e}"))?;

    Ok(parsed
        .photos
        .into_iter()
        .map(|p| {
            // original has no size limit; large2x caps at 1880px wide. Prefer
            // original so the full-resolution image lands on the screen.
            let image_url = p.src.original;
            let thumb_url = p.src.medium.unwrap_or_else(|| image_url.clone());
            Candidate {
                id: format!("pexels-{}", p.id),
                source: "pexels",
                image_url,
                thumb_url,
                web_url: p.url.unwrap_or_default(),
                title: p.alt.filter(|a| !a.is_empty()),
                author: p.photographer,
                dominant_colors: p
                    .avg_color
                    .map(|c| vec![c.trim_start_matches('#').to_lowercase()])
                    .unwrap_or_default(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(api_key: Option<&str>) -> PexelsCfg {
        PexelsCfg {
            query: "nature".into(),
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
    fn photo_shape_parses() {
        let body = r##"{
            "photos": [{
                "id": 1,
                "url": "https://pexels.com/ph/1",
                "src": {"original": "https://img/o.jpg", "large2x": "https://img/l.jpg", "medium": "https://img/m.jpg"},
                "photographer": "Ada",
                "alt": "A forest",
                "avg_color": "#102030"
            }]
        }"##;
        let r: SearchResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.photos.len(), 1);
        let p = &r.photos[0];
        assert_eq!(p.src.original, "https://img/o.jpg");
        assert_eq!(p.photographer.as_deref(), Some("Ada"));
        assert_eq!(p.avg_color.as_deref(), Some("#102030")); 
        // ^ hex inside a raw string literal; no escaping needed
    }

    #[test]
    fn missing_optional_fields_default() {
        let body = r#"{"photos": [{"id": 2, "src": {"original": "https://img/o2.jpg"}}]}"#;
        let r: SearchResponse = serde_json::from_str(body).unwrap();
        let p = &r.photos[0];
        assert_eq!(p.url, None);
        assert_eq!(p.photographer, None);
        assert_eq!(p.alt, None);
        assert_eq!(p.avg_color, None);
        assert!(p.src.medium.is_none());
    }

    #[tokio::test]
    async fn local_api_maps_photos() {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("VALHALLA_PEXELS_URL");
            }
        }
        let guard = Guard;
        let payload = r##"{"photos": [{
            "id": 7,
            "url": "https://pexels.com/ph/7",
            "src": {"original": "https://img/o7.jpg", "medium": "https://img/m7.jpg"},
            "photographer": "Grace",
            "alt": "Sea cliffs",
            "avg_color": "#0a0b0c"
        }]}"##;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var("VALHALLA_PEXELS_URL", format!("http://127.0.0.1:{port}/search"));
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
        assert_eq!(c.id, "pexels-7");
        assert_eq!(c.source, "pexels");
        assert_eq!(c.image_url, "https://img/o7.jpg");
        assert_eq!(c.thumb_url, "https://img/m7.jpg");
        assert_eq!(c.web_url, "https://pexels.com/ph/7");
        assert_eq!(c.author.as_deref(), Some("Grace"));
        assert_eq!(c.title.as_deref(), Some("Sea cliffs"));
        assert_eq!(c.dominant_colors, vec!["0a0b0c"]);
    }

    #[tokio::test]
    async fn unauthorized_key_is_reported() {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("VALHALLA_PEXELS_URL");
            }
        }
        let guard = Guard;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var("VALHALLA_PEXELS_URL", format!("http://127.0.0.1:{port}/search"));
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
        drop(guard);
        let err = result.unwrap_err();
        assert!(err.contains("401"), "error: {err}");
    }

}
