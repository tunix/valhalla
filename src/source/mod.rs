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
}

impl Source {
    pub const ALL: [Source; 2] = [Source::Wallhaven, Source::Unsplash];

    pub fn id(self) -> &'static str {
        match self {
            Source::Wallhaven => "wallhaven",
            Source::Unsplash => "unsplash",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Source::Wallhaven => "Wallhaven",
            Source::Unsplash => "Unsplash",
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
        }
    }
}
