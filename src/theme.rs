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
