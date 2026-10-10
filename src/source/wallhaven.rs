use serde::Deserialize;

use crate::{config::WallhavenCfg, http, source::Candidate};

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    url: String,
    path: String,
    #[serde(default)]
    colors: Vec<String>,
    thumbs: Thumbs,
}

#[derive(Deserialize)]
struct Thumbs {
    #[serde(default)]
    small: Option<String>,
    #[serde(default)]
    large: Option<String>,
}

fn random_seed() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..6)
        .map(|_| CHARS[fastrand::usize(..CHARS.len())] as char)
        .collect()
}

pub async fn candidates(cfg: &WallhavenCfg) -> Result<Vec<Candidate>, String> {
    // Tests point this at a local stub server; production always resolves to
    // the real API because the env var is unset there.
    let base = std::env::var("VALHALLA_WALLHAVEN_URL")
        .unwrap_or_else(|_| "https://wallhaven.cc/api/v1/search".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("categories", &cfg.categories);
        qp.append_pair("purity", &cfg.purity);
        qp.append_pair("sorting", &cfg.sorting);
        if cfg.sorting == "random" {
            qp.append_pair("seed", &random_seed());
        }
        if !cfg.query.is_empty() {
            qp.append_pair("q", &cfg.query);
        }
        if !cfg.ratios.is_empty() {
            qp.append_pair("ratios", &cfg.ratios);
        }
        if !cfg.atleast.is_empty() {
            qp.append_pair("atleast", &cfg.atleast);
        }
        if let Some(key) = &cfg.api_key {
            qp.append_pair("apikey", key);
        }
    }

    let response = http::client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let response = response
        .error_for_status()
        .map_err(|e| format!("wallhaven: {e}"))?;
    let parsed: SearchResponse = response.json().await.map_err(|e| e.to_string())?;

    Ok(parsed
        .data
        .into_iter()
        .map(|item| {
            let thumb_url = item
                .thumbs
                .small
                .or(item.thumbs.large)
                .unwrap_or_else(|| item.url.clone());
            Candidate {
                id: item.id,
                source: "wallhaven",
                image_url: item.path,
                thumb_url,
                web_url: item.url,
                title: None,
                author: None,
                dominant_colors: item
                    .colors
                    .iter()
                    .map(|c| c.trim_start_matches('#').to_lowercase())
                    .collect(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Points the env-var base override at a local stub and restores the
    /// previous value on drop (tests run in parallel; env is global).
    fn set_stub_base(var: &'static str) -> StubBaseGuard {
        StubBaseGuard { var }
    }

    struct StubBaseGuard {
        var: &'static str,
    }

    impl StubBaseGuard {
        fn set(&self, port: u16) {
            std::env::set_var(self.var, format!("http://127.0.0.1:{port}/search"));
        }
    }

    impl Drop for StubBaseGuard {
        fn drop(&mut self) {
            std::env::remove_var(self.var);
        }
    }

    fn cfg(sorting: &str) -> WallhavenCfg {
        WallhavenCfg {
            query: "mountains".into(),
            categories: "110".into(),
            purity: "100".into(),
            sorting: sorting.into(),
            atleast: "1920x1080".into(),
            ratios: "16x9".into(),
            api_key: None,
        }
    }

    #[test]
    fn seed_is_six_alnum_chars() {
        for _ in 0..20 {
            let seed = random_seed();
            assert_eq!(seed.len(), 6);
            assert!(seed.chars().all(|c| c.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn seeds_vary_across_calls() {
        let seeds: Vec<String> = (0..8).map(|_| random_seed()).collect();
        let unique: std::collections::HashSet<_> = seeds.iter().collect();
        assert!(
            unique.len() > 1,
            "random_seed() produced identical seeds 8x in a row"
        );
    }

    /// The JSON the parser consumes and the fields it must produce.
    #[test]
    fn response_shape_maps_to_candidates() {
        let body = r##"{
            "data": [{
                "id": "abc123",
                "url": "https://wallhaven.cc/w/abc123",
                "path": "https://wh/path/full.jpg",
                "colors": ["#000000", "#ffffff"],
                "thumbs": {"small": "https://wh/t/s.jpg", "large": "https://wh/t/l.jpg"}
            }]
        }"##;
        let parsed: SearchResponse = serde_json::from_str(body).unwrap();
        let item = &parsed.data[0];
        assert_eq!(item.id, "abc123");
        assert_eq!(item.colors.len(), 2);
        assert_eq!(item.thumbs.small.as_deref(), Some("https://wh/t/s.jpg"));
    }

    #[test]
    fn thumbs_without_small_falls_back_to_large() {
        let body = r#"{"id":"i1","url":"u","path":"p","thumbs":{"large":"L"}}"#;
        let item: Item = serde_json::from_str(body).unwrap();
        assert_eq!(item.thumbs.small, None);
        assert_eq!(item.thumbs.large.as_deref(), Some("L"));
        // The candidates() mapping prefers small, else large, else the page URL.
        let thumb = item
            .thumbs
            .small
            .or(item.thumbs.large)
            .unwrap_or_else(|| item.url.clone());
        assert_eq!(thumb, "L");
    }

    #[test]
    fn thumbs_missing_entirely_falls_back_to_page_url() {
        let body = r#"{"id":"i2","url":"PAGE","path":"p","thumbs":{}}"#;
        let item: Item = serde_json::from_str(body).unwrap();
        let thumb = item
            .thumbs
            .small
            .or(item.thumbs.large)
            .unwrap_or_else(|| item.url.clone());
        assert_eq!(thumb, "PAGE");
    }

    #[test]
    fn colors_are_stripped_and_lowercased() {
        let raw = ["#AABBCC", "#001122"];
        let out: Vec<String> = raw
            .iter()
            .map(|c| c.trim_start_matches('#').to_lowercase())
            .collect();
        assert_eq!(out, vec!["aabbcc", "001122"]);
    }

    #[tokio::test]
    async fn local_api_server_is_parsed() {
        // Full candidates() round trip against a local wallhaven-shaped API.
        let port_guard = set_stub_base("VALHALLA_WALLHAVEN_URL");
        let payload = r##"{"data":[
            {"id":"w9","url":"https://wh/w/w9","path":"https://wh/f/w9.jpg",
             "colors":["#112233"],
             "thumbs":{"small":"https://wh/s/w9.jpg","large":"https://wh/l/w9.jpg"}}
        ]}"##;
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
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            use std::io::Write as _;
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(payload.as_bytes()).unwrap();
        });

        let result = candidates(&cfg("toplist")).await;
        server.join().unwrap();
        let cands = result.expect("candidates");
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        assert_eq!(c.id, "w9");
        assert_eq!(c.source, "wallhaven");
        assert_eq!(c.image_url, "https://wh/f/w9.jpg");
        assert_eq!(c.thumb_url, "https://wh/s/w9.jpg");
        assert_eq!(c.web_url, "https://wh/w/w9");
        assert_eq!(c.dominant_colors, vec!["112233"]);
        assert!(c.title.is_none());
        assert!(c.author.is_none());
    }

    #[tokio::test]
    async fn http_error_status_is_wrapped() {
        let port_guard = set_stub_base("VALHALLA_WALLHAVEN_URL");
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
                .write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
        });

        let result = candidates(&cfg("random")).await;
        server.join().unwrap();
        let err = result.unwrap_err();
        assert!(err.starts_with("wallhaven:"), "wrapped error: {err}");
        assert!(err.contains("429"));
    }
}
