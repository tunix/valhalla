use serde::Deserialize;

use crate::{http, source::Candidate};

/// Bing's daily wallpaper: no API key, one fresh image per day.
///
/// The `bing.com/HPImageArchive.aspx` endpoint returns a JSON list; the
/// `uhd` variant of the base URL serves the full-resolution copy.
#[derive(Deserialize)]
struct ArchiveResponse {
    #[serde(default)]
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    urlbase: String,
    #[serde(default)]
    copyrightonly: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

pub async fn candidates() -> Result<Vec<Candidate>, String> {
    // Tests point this at a local stub server; see wallhaven.rs.
    let base = std::env::var("VALHALLA_BING_URL")
        .unwrap_or_else(|_| "https://www.bing.com/HPImageArchive.aspx".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("format", "js");
        qp.append_pair("idx", "0");
        qp.append_pair("n", "8");
        qp.append_pair("mkt", "en-US");
    }

    let response = http::client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let response = response
        .error_for_status()
        .map_err(|e| format!("bing: {e}"))?;
    let parsed: ArchiveResponse = response.json().await.map_err(|e| format!("bing: {e}"))?;

    Ok(parsed
        .images
        .into_iter()
        .map(|img| {
            // url is capped at 1920px; urlbase + "1920x1080" style params
            // give more control. The uhd endpoint serves the native copy.
            let image_url = format!("https://www.bing.com{}UHD&qlt=100", img.urlbase);
            // Small copy for the brightness analysis.
            let thumb_url = format!("https://www.bing.com{}_1920x1080", img.urlbase);
            Candidate {
                id: format!(
                    "bing-{}",
                    img.urlbase.rsplit('/').next().unwrap_or("unknown")
                ),
                source: "bing",
                image_url,
                thumb_url,
                web_url: "https://www.bing.com/gallery".into(),
                title: img.title.or(img.copyrightonly),
                author: None,
                dominant_colors: Vec::new(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_shape_parses() {
        let body = r#"{"images": [{
            "url": "/th?id=OHR.TreeDay_EN-US1234_1920x1080.jpg",
            "urlbase": "/th?id=OHR.TreeDay_EN-US1234",
            "copyright": "A forest (© Someone)",
            "copyrightonly": "A forest",
            "title": "Forest title"
        }]}"#;
        let r: ArchiveResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.images.len(), 1);
        let img = &r.images[0];
        assert_eq!(img.urlbase, "/th?id=OHR.TreeDay_EN-US1234");
        assert_eq!(img.title.as_deref(), Some("Forest title"));
    }

    #[test]
    fn missing_images_defaults_to_empty() {
        let r: ArchiveResponse = serde_json::from_str("{}").unwrap();
        assert!(r.images.is_empty());
    }

    /// Runs candidates() against a local stub server on a dedicated single
    /// threaded runtime (a nested runtime inside #[tokio::test] is illegal).
    fn run_local(payload: &'static str) -> Vec<Candidate> {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var("VALHALLA_BING_URL");
            }
        }
        let _guard = Guard;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var("VALHALLA_BING_URL", format!("http://127.0.0.1:{port}/HPImageArchive.aspx"));
        let _server = std::thread::spawn(move || {
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

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(candidates())
            .expect("candidates")
    }

    #[test]
    fn local_api_maps_images() {
        let payload = r#"{"images": [{
            "url": "/th?id=OHR.Test_EN-US9_1920x1080.jpg&rf=",
            "urlbase": "/th?id=OHR.Test_EN-US9",
            "copyrightonly": "Misty mountains",
            "title": "Morning fog"
        }]}"#;
        let cands = run_local(payload);
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        // urlbase is "/th?id=OHR.Test_EN-US9": the last '/'-separated chunk
        // is "th?id=OHR.Test_EN-US9", which is what the id derives from.
        assert_eq!(c.id, "bing-th?id=OHR.Test_EN-US9");
        assert_eq!(c.source, "bing");
        assert_eq!(
            c.image_url,
            "https://www.bing.com/th?id=OHR.Test_EN-US9UHD&qlt=100"
        );
        assert_eq!(
            c.thumb_url,
            "https://www.bing.com/th?id=OHR.Test_EN-US9_1920x1080"
        );
        assert_eq!(c.title.as_deref(), Some("Morning fog"));
    }
}
