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
    let mut url =
        reqwest::Url::parse("https://wallhaven.cc/api/v1/search").map_err(|e| e.to_string())?;
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
