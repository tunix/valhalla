use gio::prelude::*;

use crate::screen::{self, ScreenInfo};

pub fn settings() -> gio::Settings {
    gio::Settings::new("io.github.tunix.valhalla")
}

fn screen_or_default(screen: Option<ScreenInfo>) -> ScreenInfo {
    screen.unwrap_or(ScreenInfo {
        width: 1920,
        height: 1080,
    })
}

#[derive(Debug, Clone)]
pub struct WallhavenCfg {
    pub query: String,
    pub categories: String,
    pub purity: String,
    pub sorting: String,
    pub atleast: String,
    pub ratios: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnsplashCfg {
    pub query: String,
    pub orientation: String,
    pub content_filter: String,
    pub access_key: Option<String>,
}

/// Resolution and aspect ratio are always derived from the screen.
pub fn wallhaven(
    s: &gio::Settings,
    api_key: Option<String>,
    screen: Option<ScreenInfo>,
) -> WallhavenCfg {
    let sc = screen_or_default(screen);
    WallhavenCfg {
        query: s.string("wallhaven-query").to_string(),
        categories: s.string("wallhaven-categories").to_string(),
        purity: s.string("wallhaven-purity").to_string(),
        sorting: s.string("wallhaven-sorting").to_string(),
        atleast: format!("{}x{}", sc.width, sc.height),
        ratios: screen::wallhaven_ratios(sc).join(","),
        api_key,
    }
}

pub fn unsplash(
    s: &gio::Settings,
    access_key: Option<String>,
    screen: Option<ScreenInfo>,
) -> UnsplashCfg {
    let sc = screen_or_default(screen);
    UnsplashCfg {
        query: s.string("unsplash-query").to_string(),
        orientation: screen::unsplash_orientation(sc).to_string(),
        content_filter: s.string("unsplash-content-filter").to_string(),
        access_key,
    }
}

/// The probed screen geometry persisted for fallback when probing is
/// unavailable (e.g. sandboxed daemon before any GUI run).
pub fn screen_geometry(s: &gio::Settings) -> Option<ScreenInfo> {
    let raw = s.string("screen-geometry").to_string();
    let (w, h) = raw.split_once('x')?;
    let (w, h) = (w.trim().parse::<i32>().ok()?, h.trim().parse::<i32>().ok()?);
    (w > 0 && h > 0).then_some(ScreenInfo {
        width: w,
        height: h,
    })
}

pub fn store_screen_geometry(s: &gio::Settings, screen: ScreenInfo) {
    let _ = s.set_string(
        "screen-geometry",
        &format!("{}x{}", screen.width, screen.height),
    );
}
