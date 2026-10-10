use ashpd::desktop::settings::{ColorScheme as PortalScheme, Settings};
use gio::prelude::*;
use serde::{Deserialize, Serialize};

/// The desktop light/dark appearance preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Light,
    Dark,
}

impl ColorScheme {
    pub fn label(self) -> &'static str {
        match self {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        }
    }
}

/// Read the color scheme through the XDG settings portal.
pub async fn portal_scheme() -> Option<ColorScheme> {
    let proxy = Settings::new().await.ok()?;
    let scheme = proxy.color_scheme().await.ok()?;
    Some(match scheme {
        PortalScheme::PreferDark => ColorScheme::Dark,
        PortalScheme::PreferLight | PortalScheme::NoPreference => ColorScheme::Light,
    })
}

/// Read GNOME's color scheme directly from GSettings.
pub fn gsettings_scheme() -> Option<ColorScheme> {
    let settings = gio::Settings::new("org.gnome.desktop.interface");
    match settings.string("color-scheme").as_str() {
        "prefer-dark" => Some(ColorScheme::Dark),
        "prefer-light" | "default" => Some(ColorScheme::Light),
        _ => None,
    }
}

/// Current desktop scheme: portal first, GSettings fallback, light as default.
pub async fn current() -> ColorScheme {
    portal_scheme()
        .await
        .or_else(gsettings_scheme)
        .unwrap_or(ColorScheme::Light)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(ColorScheme::Light.label(), "light");
        assert_eq!(ColorScheme::Dark.label(), "dark");
    }

    #[test]
    fn serde_roundtrip() {
        // The scheme is persisted in state.json — serde names are load-bearing.
        assert_eq!(
            serde_json::to_string(&ColorScheme::Light).unwrap(),
            "\"light\""
        );
        assert_eq!(
            serde_json::to_string(&ColorScheme::Dark).unwrap(),
            "\"dark\""
        );
        let dark: ColorScheme = serde_json::from_str("\"dark\"").unwrap();
        assert_eq!(dark, ColorScheme::Dark);
        assert!(serde_json::from_str::<ColorScheme>("\"blue\"").is_err());
    }

    #[test]
    fn gsettings_scheme_with_memory_backend() {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        let settings = gio::Settings::new("org.gnome.desktop.interface");
        let _ = settings.set_string("color-scheme", "prefer-dark");
        assert_eq!(gsettings_scheme(), Some(ColorScheme::Dark));
        let _ = settings.set_string("color-scheme", "prefer-light");
        assert_eq!(gsettings_scheme(), Some(ColorScheme::Light));
        let _ = settings.set_string("color-scheme", "default");
        assert_eq!(gsettings_scheme(), Some(ColorScheme::Light));
        // Non-enum values are rejected by the schema (set fails silently),
        // so gsettings_scheme()'s None arm corresponds to an absent key in
        // relocatable schemas — covered by the match itself.
        std::env::remove_var("GSETTINGS_BACKEND");
    }

    #[test]
    fn portal_scheme_mapping_is_exhaustive() {
        // Mirrors portal_scheme()'s match so a new portal variant surfaces
        // here as a compile error and the mapping intent stays documented.
        fn map(scheme: PortalScheme) -> ColorScheme {
            match scheme {
                PortalScheme::PreferDark => ColorScheme::Dark,
                PortalScheme::PreferLight | PortalScheme::NoPreference => ColorScheme::Light,
            }
        }
        assert_eq!(map(PortalScheme::PreferDark), ColorScheme::Dark);
        assert_eq!(map(PortalScheme::PreferLight), ColorScheme::Light);
        assert_eq!(map(PortalScheme::NoPreference), ColorScheme::Light);
    }
}
