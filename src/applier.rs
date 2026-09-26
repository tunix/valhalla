use std::path::Path;

use ashpd::desktop::wallpaper::{SetOn, WallpaperOptions, WallpaperProxy};
use gio::prelude::*;

use crate::theme::ColorScheme;

pub fn is_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

pub fn gsettings_background_available() -> bool {
    gio::SettingsSchemaSource::default()
        .and_then(|src| src.lookup("org.gnome.desktop.background", true))
        .is_some()
}

/// How wallpapers get applied on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applier {
    /// XDG wallpaper portal: works on GNOME, KDE and other fd.o desktops.
    Portal,
    /// Direct GSettings with independent light/dark keys (GNOME only).
    /// Preferred natively because the portal overwrites both keys with a
    /// single image, defeating the per-mode strategy.
    GSettings,
}

pub fn detect() -> Applier {
    if !is_flatpak() && gsettings_background_available() {
        Applier::GSettings
    } else {
        Applier::Portal
    }
}

/// Apply wallpapers. `light` and `dark` may point at the same file when only
/// one mode has a wallpaper yet; whichever is set for the current scheme wins.
pub async fn apply(
    applier: Applier,
    light: Option<&Path>,
    dark: Option<&Path>,
    current: ColorScheme,
    lockscreen: bool,
) -> Result<(), String> {
    match applier {
        Applier::GSettings => apply_gsettings(light, dark),
        Applier::Portal => {
            let path = match current {
                ColorScheme::Light => light,
                ColorScheme::Dark => dark,
            }
            .ok_or("no wallpaper to apply")?;
            apply_portal(path, lockscreen).await
        }
    }
}

fn apply_gsettings(light: Option<&Path>, dark: Option<&Path>) -> Result<(), String> {
    let bg = gio::Settings::new("org.gnome.desktop.background");
    for (key, path) in [("picture-uri", light), ("picture-uri-dark", dark)] {
        let Some(path) = path else { continue };
        let uri = glib::filename_to_uri(path, None).map_err(|e| e.to_string())?;
        bg.set_string(key, uri.as_str())
            .map_err(|e| format!("gsettings: {e}"))?;
    }
    Ok(())
}

async fn apply_portal(path: &Path, lockscreen: bool) -> Result<(), String> {
    let proxy = WallpaperProxy::new().await.map_err(|e| e.to_string())?;
    let uri = glib::filename_to_uri(path, None).map_err(|e| e.to_string())?;
    let uri = ashpd::Uri::parse(uri.as_str()).map_err(|e| e.to_string())?;

    let options = WallpaperOptions::default()
        .set_show_preview(false)
        .set_set_on(if lockscreen {
            SetOn::Both
        } else {
            SetOn::Background
        });

    proxy
        .set_wallpaper_uri(None, &uri, options)
        .await
        .map_err(|e| format!("wallpaper portal: {e}"))?
        .response()
        .map_err(|e| format!("wallpaper portal: {e}"))?;
    Ok(())
}
