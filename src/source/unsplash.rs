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

    let mut url =
        reqwest::Url::parse("https://api.unsplash.com/photos/random").map_err(|e| e.to_string())?;
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
