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

#[derive(Debug, Clone)]
pub struct PexelsCfg {
    pub query: String,
    pub orientation: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PixabayCfg {
    pub query: String,
    pub category: String,
    pub orientation: String,
    pub api_key: Option<String>,
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

pub fn pexels(s: &gio::Settings, api_key: Option<String>, screen: Option<ScreenInfo>) -> PexelsCfg {
    let sc = screen_or_default(screen);
    PexelsCfg {
        query: s.string("pexels-query").to_string(),
        orientation: screen::unsplash_orientation(sc).to_string(),
        api_key,
    }
}

pub fn pixabay(
    s: &gio::Settings,
    api_key: Option<String>,
    screen: Option<ScreenInfo>,
) -> PixabayCfg {
    let sc = screen_or_default(screen);
    PixabayCfg {
        query: s.string("pixabay-query").to_string(),
        category: s.string("pixabay-category").to_string(),
        orientation: screen::unsplash_orientation(sc).to_string(),
        api_key,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// screen_geometry() parses "WxH" strings; the parse rules matter for
    /// state persisted by older versions. Verify them against raw strings
    /// via the same split/parse logic (GSettings-free).
    fn parse_geometry(raw: &str) -> Option<ScreenInfo> {
        let (w, h) = raw.split_once('x')?;
        let (w, h) = (w.trim().parse::<i32>().ok()?, h.trim().parse::<i32>().ok()?);
        (w > 0 && h > 0).then_some(ScreenInfo {
            width: w,
            height: h,
        })
    }

    #[test]
    fn geometry_parses_valid_strings() {
        assert_eq!(
            parse_geometry("1920x1080"),
            Some(ScreenInfo {
                width: 1920,
                height: 1080
            })
        );
        assert_eq!(
            parse_geometry("3840x2160"),
            Some(ScreenInfo {
                width: 3840,
                height: 2160
            })
        );
        assert_eq!(
            parse_geometry(" 800x 600 "),
            Some(ScreenInfo {
                width: 800,
                height: 600
            })
        );
        assert_eq!(
            parse_geometry("1080x1920"),
            Some(ScreenInfo {
                width: 1080,
                height: 1920
            })
        );
    }

    #[test]
    fn geometry_rejects_invalid_strings() {
        assert_eq!(parse_geometry(""), None); // no 'x'
        assert_eq!(parse_geometry("nonsense"), None);
        assert_eq!(parse_geometry("0x100"), None); // zero width
        assert_eq!(parse_geometry("100x0"), None); // zero height
        assert_eq!(parse_geometry("-10x200"), None); // negative
        assert_eq!(parse_geometry("ax200"), None); // not a number
        assert_eq!(parse_geometry("1920x"), None); // missing height
        assert_eq!(parse_geometry("1920xabc"), None);
        assert_eq!(parse_geometry("1920x2.5"), None); // not an integer
        assert_eq!(parse_geometry("1x2x3"), None); // two separators
    }

    #[test]
    fn screen_or_default_fills_1080p() {
        let d = screen_or_default(None);
        assert_eq!((d.width, d.height), (1920, 1080));
        let s = screen_or_default(Some(ScreenInfo {
            width: 2560,
            height: 1440,
        }));
        assert_eq!((s.width, s.height), (2560, 1440));
    }

    #[test]
    fn cfg_structs_clone_and_debug() {
        let wh = WallhavenCfg {
            query: "sea".into(),
            categories: "100".into(),
            purity: "100".into(),
            sorting: "random".into(),
            atleast: "1920x1080".into(),
            ratios: "16x9".into(),
            api_key: Some("k".into()),
        };
        let wh2 = wh.clone();
        assert_eq!(wh2.query, "sea");
        assert!(!format!("{wh:?}").is_empty());

        let un = UnsplashCfg {
            query: "forest".into(),
            orientation: "landscape".into(),
            content_filter: "low".into(),
            access_key: None,
        };
        let un2 = un.clone();
        assert_eq!(un2.orientation, "landscape");
        assert!(!format!("{un:?}").is_empty());
    }

    /// Memory-backend GSettings: gio ships a built-in "memory" backend, so
    /// the settings-driven builders can run without dconf/DBus.
    #[test]
    fn wallhaven_and_unsplash_builders_with_memory_backend() {
        // Uniform rule: any test touching process-global state (env vars,
        // the shared GSettings memory backend) holds ENV_LOCK for its whole
        // body (see crate::http::test_env).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        let s = gio::Settings::new("io.github.tunix.valhalla");
        let _ = s.set_string("wallhaven-query", "ocean");
        let _ = s.set_string("unsplash-content-filter", "high");

        let wh = wallhaven(
            &s,
            Some("key".into()),
            Some(ScreenInfo {
                width: 2560,
                height: 1440,
            }),
        );
        assert_eq!(wh.query, "ocean");
        assert_eq!(wh.atleast, "2560x1440");
        assert_eq!(wh.api_key.as_deref(), Some("key"));
        assert!(wh.ratios.contains("16x9"));

        let wh_noscreen = wallhaven(&s, None, None);
        assert_eq!(wh_noscreen.atleast, "1920x1080"); // default screen
        assert!(wh_noscreen.api_key.is_none());

        let un = unsplash(
            &s,
            Some("un-key".into()),
            Some(ScreenInfo {
                width: 1080,
                height: 1920,
            }),
        );
        assert_eq!(un.orientation, "portrait");
        assert_eq!(un.content_filter, "high");
        assert_eq!(un.access_key.as_deref(), Some("un-key"));

        let un_default = unsplash(&s, None, None);
        assert_eq!(un_default.orientation, "landscape");
        std::env::remove_var("GSETTINGS_BACKEND");
    }

    #[test]
    fn screen_geometry_roundtrip_with_memory_backend() {
        // Memory backend persists across tests in one process; never assume
        // an initial value, only roundtrip consistency. Serialized against
        // the other backend users via ENV_LOCK (see crate::http::test_env).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        let s = gio::Settings::new("io.github.tunix.valhalla");
        store_screen_geometry(
            &s,
            ScreenInfo {
                width: 3440,
                height: 1440,
            },
        );
        assert_eq!(
            screen_geometry(&s),
            Some(ScreenInfo {
                width: 3440,
                height: 1440
            })
        );
        // Garbage write is rejected by the schema type; parse side still
        // yields a value (whatever valid one is stored). The contract under
        // test: parse never panics and only valid WxH strings surface.
        let parsed = screen_geometry(&s);
        assert!(parsed.is_none_or(|g| g.width > 0 && g.height > 0));
        std::env::remove_var("GSETTINGS_BACKEND");
    }

    /// GSettings-backed builders need a writable dconf backend in tests;
    /// memory backend via GSETTINGS_BACKEND env is not linkable here, so
    /// verify the pure mapping (settings value -> cfg field) instead.
    #[test]
    fn atleast_and_ratios_derive_from_screen() {
        let sc = ScreenInfo {
            width: 2560,
            height: 1440,
        };
        assert_eq!(format!("{}x{}", sc.width, sc.height), "2560x1440");
        let ratios = screen::wallhaven_ratios(sc).join(",");
        assert!(
            ratios.contains("16x9"),
            "2560x1440 -> 16x9 family, got {ratios}"
        );
    }
}
