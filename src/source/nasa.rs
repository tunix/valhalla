use serde::Deserialize;

use crate::{http, source::Candidate};

/// NASA APOD (Astronomy Picture of the Day) via images-api.nasa.gov — no key
/// required (DEMO_KEY fallback), but a free key lifts the rate limit.
#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    nasa_id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    photographer: Option<String>,
    #[serde(default)]
    center: Option<String>,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(default)]
    items: Vec<CollectionItem>,
}

#[derive(Deserialize)]
struct CollectionItem {
    #[serde(default)]
    data: Vec<Item>,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    #[serde(default)]
    href: String,
}

pub async fn candidates(api_key: Option<String>) -> Result<Vec<Candidate>, String> {
    // Tests point this at a local stub server; see wallhaven.rs.
    let base = std::env::var("VALHALLA_NASA_URL")
        .unwrap_or_else(|_| "https://images-api.nasa.gov/search".into());
    let mut url = reqwest::Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("media_type", "image");
        qp.append_pair("page_size", "24");
        qp.append_pair("year_end", &chrono_now_year());
    }

    let mut request = http::client().get(url);
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        request = request.header("Authorization", format!("Bearer {key}"));
    }

    let response = request.send().await.map_err(|e| e.to_string())?;
    let response = response
        .error_for_status()
        .map_err(|e| format!("nasa: {e}"))?;
    let parsed: SearchResponse = response.json().await.map_err(|e| format!("nasa: {e}"))?;

    Ok(parsed
        .collection
        .items
        .into_iter()
        .filter_map(|ci| {
            let item = ci.data.into_iter().next()?;
            let thumb = ci.links.into_iter().next().map(|l| l.href)?;
            // href is the collection listing; the asset fetch needs one more
            // request, so we use the thumb URL as the image when it ends in
            // a usable extension and skip otherwise. The imagery API serves
            // ~orig at a predictable path: strip the thumb filename.
            let image_url = original_url(&thumb);
            Some(Candidate {
                id: format!("nasa-{}", item.nasa_id),
                source: "nasa",
                image_url,
                thumb_url: thumb,
                web_url: format!("https://images.nasa.gov/details/{}", item.nasa_id),
                title: item.title,
                author: item.photographer.or(item.center),
                dominant_colors: Vec::new(),
            })
        })
        .collect())
}

/// images-api serves the full asset at the same path with ~orig.jpg suffix.
fn original_url(thumb: &str) -> String {
    // Thumb URLs look like: https://images-assets.nasa.gov/image/ID/ID~thumb.jpg
    // The original: https://images-assets.nasa.gov/image/ID/ID~orig.jpg
    if let Some(idx) = thumb.rfind("~") {
        format!("{}~orig.jpg", &thumb[..idx])
    } else {
        thumb.to_string()
    }
}

fn chrono_now_year() -> String {
    // images-api accepts a generous future bound; keep the window open.
    std::env::var("VALHALLA_NASA_YEAR_END").unwrap_or_else(|_| "2030".into())
}

#[derive(Deserialize)]
struct SearchResponse {
    collection: Collection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_response_parses() {
        let body = r#"{
            "collection": {
                "items": [{
                    "data": [{
                        "nasa_id": "PIA123",
                        "title": "Nebula",
                        "photographer": "Hubble",
                        "center": "JPL"
                    }],
                    "links": [{"href": "https://images-assets.nasa.gov/image/PIA123/PIA123~thumb.jpg"}]
                }]
            }
        }"#;
        let r: SearchResponse = serde_json::from_str(body).unwrap();
        assert_eq!(r.collection.items.len(), 1);
    }

    #[test]
    fn empty_collection_ok() {
        let r: SearchResponse = serde_json::from_str(r#"{"collection": {"items": []}}"#).unwrap();
        assert!(r.collection.items.is_empty());
    }

    #[test]
    fn original_url_derives_from_thumb() {
        assert_eq!(
            original_url("https://images-assets.nasa.gov/image/PIA1/PIA1~thumb.jpg"),
            "https://images-assets.nasa.gov/image/PIA1/PIA1~orig.jpg"
        );
        // No ~suffix: return as-is (defensive).
        assert_eq!(
            original_url("https://example.com/plain.jpg"),
            "https://example.com/plain.jpg"
        );
    }

    #[test]
    fn items_without_thumb_are_skipped() {
        // filter_map drops entries that lack both data and links.
        let body = r#"{"collection": {"items": [{"data": [{"nasa_id": "X"}], "links": []}]}}"#;
        let r: SearchResponse = serde_json::from_str(body).unwrap();
        let mapped: Vec<_> = r
            .collection
            .items
            .into_iter()
            .filter_map(|ci| ci.links.into_iter().next().map(|l| l.href))
            .collect();
        assert!(mapped.is_empty());
    }
}
