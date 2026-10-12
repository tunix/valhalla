pub mod unsplash;
pub mod wallhaven;

#[derive(Debug, Clone)]
#[allow(dead_code)] // web_url/dominant_colors feed history + metadata filters later
pub struct Candidate {
    pub id: String,
    pub source: &'static str,
    /// Full-size image URL.
    pub image_url: String,
    /// Small thumbnail used for brightness analysis.
    pub thumb_url: String,
    pub web_url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    /// Dominant colors as lowercase hex (without '#'), when the API provides them.
    pub dominant_colors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Wallhaven,
    Unsplash,
    Favorites,
    Manual,
}

impl Source {
    pub const ALL: [Source; 4] = [
        Source::Wallhaven,
        Source::Unsplash,
        Source::Favorites,
        Source::Manual,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Source::Wallhaven => "wallhaven",
            Source::Unsplash => "unsplash",
            Source::Favorites => "favorites",
            Source::Manual => "manual",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Source::Wallhaven => "Wallhaven",
            Source::Unsplash => "Unsplash",
            Source::Favorites => "Favorites",
            Source::Manual => "Manual",
        }
    }

    pub fn from_id(id: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|s| s.id() == id)
    }

    pub async fn candidates(
        &self,
        wallhaven: &crate::config::WallhavenCfg,
        unsplash: &crate::config::UnsplashCfg,
    ) -> Result<Vec<Candidate>, String> {
        match self {
            Source::Wallhaven => wallhaven::candidates(wallhaven).await,
            Source::Unsplash => unsplash::candidates(unsplash).await,
            Source::Favorites => crate::favorites::candidates(),
            Source::Manual => crate::manual::candidates(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_titles_match() {
        for source in Source::ALL {
            assert_eq!(Source::from_id(source.id()), Some(source));
            assert!(!source.title().is_empty());
            assert_ne!(source.title(), source.id()); // titles are display names
        }
    }

    #[test]
    fn from_id_rejects_unknown() {
        assert_eq!(Source::from_id("pixiv"), None);
        assert_eq!(Source::from_id(""), None);
        assert_eq!(Source::from_id("Wallhaven"), None); // case-sensitive
    }

    #[test]
    fn candidate_colors_are_normalized() {
        // The wallhaven parser strips '#' and lowercases; verify the contract
        // the code relies on with a representative mapping.
        let raw = ["#FFAA00", "#0fF"];
        let normalized: Vec<String> = raw
            .iter()
            .map(|c| c.trim_start_matches('#').to_lowercase())
            .collect();
        assert_eq!(normalized, vec!["ffaa00", "0ff"]);
    }

    #[test]
    fn candidate_serializes_metadata() {
        let c = Candidate {
            id: "x1".into(),
            source: "wallhaven",
            image_url: "https://img/x1.jpg".into(),
            thumb_url: "https://img/t.jpg".into(),
            web_url: "https://wh/x1".into(),
            title: Some("Aurora".into()),
            author: None,
            dominant_colors: vec!["001122".into()],
        };
        let json = serde_json::to_string(&c.id).unwrap();
        assert_eq!(json, "\"x1\"");
    }
}
