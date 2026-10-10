//! "Start at login" via the XDG autostart mechanism.
//!
//! Writes a desktop entry that launches the headless daemon (`valhalla
//! --daemon`) into the user's autostart directory. This is the portable,
//! non-invasive path: the entry only takes effect because the user asked for
//! it in Preferences, and every desktop (GNOME, KDE, hyprland/Sway via their
//! portal stacks) honors `~/.config/autostart`.
//!
//! Flatpak note: inside the sandbox `XDG_CONFIG_HOME` maps to
//! `~/.var/app/<app-id>/config`, and the manifest must pass
//! `--filesystem=xdg-config/autostart:create` so the host autostart
//! directory is the real target. Writing through that mapping lands the
//! entry where the session manager looks.

use std::path::PathBuf;

const APP_ID: &str = "io.github.tunix.valhalla";

/// The entry launches the same executable that is running now, headless.
const ENTRY_TEMPLATE: &str = r#"[Desktop Entry]
Type=Application
Name=Valhalla (background)
Comment=Theme-aware wallpaper refresh daemon
Exec={exe} --daemon
Icon={app_id}
Terminal=false
X-GNOME-Autostart-enabled=true
NoDisplay=false
Categories=GNOME;GTK;Utility;
"#;

fn autostart_dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".config"));
    config.join("autostart")
}

fn entry_path() -> PathBuf {
    autostart_dir().join(format!("{APP_ID}.desktop"))
}

/// Whether the autostart entry exists and is enabled.
pub fn is_enabled() -> bool {
    match std::fs::read_to_string(entry_path()) {
        Ok(text) => !text.contains("X-GNOME-Autostart-enabled=false"),
        Err(_) => false,
    }
}

/// Write (or re-point) the autostart entry at the current executable.
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dir = autostart_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let entry = ENTRY_TEMPLATE
        .replace("{exe}", &exe.display().to_string())
        .replace("{app_id}", APP_ID);
    std::fs::write(entry_path(), entry).map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove the autostart entry. Missing file is fine (already disabled).
pub fn disable() -> Result<(), String> {
    match std::fs::remove_file(entry_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests mutate XDG_CONFIG_HOME, which is process-global: run them
    // together, serially, so parallel tests cannot race the env.
    #[test]
    fn autostart_lifecycle_serial() {
        let dir = std::env::temp_dir().join(format!(
            "valhalla-autostart-{}",
            std::process::id()
        ));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let _ = std::fs::remove_dir_all(&dir);

        // 1) starts disabled
        assert!(!is_enabled());

        // 2) enable writes a valid entry
        enable().expect("enable");
        assert!(is_enabled());
        let text = std::fs::read_to_string(entry_path()).unwrap();
        assert!(text.contains("--daemon"));
        assert!(text.contains(&format!("Icon={APP_ID}")));
        assert!(!text.contains("{exe}"), "placeholder substituted");

        // 3) disabled flag in the file counts as off
        let text = std::fs::read_to_string(entry_path()).unwrap();
        std::fs::write(
            entry_path(),
            text.replace(
                "X-GNOME-Autostart-enabled=true",
                "X-GNOME-Autostart-enabled=false",
            ),
        )
        .unwrap();
        assert!(!is_enabled(), "entry present but disabled flag set");

        // 4) re-enable repoints a stale Exec
        let text = std::fs::read_to_string(entry_path()).unwrap();
        std::fs::write(
            entry_path(),
            text.replace(
                &format!("Icon={APP_ID}"),
                "Icon=stale",
            ),
        )
        .unwrap();
        enable().expect("re-enable");
        let updated = std::fs::read_to_string(entry_path()).unwrap();
        assert!(updated.contains(&format!("Icon={APP_ID}")));
        assert!(!updated.contains("Icon=stale"));

        // 5) path shape
        assert!(entry_path().ends_with("autostart/io.github.tunix.valhalla.desktop"));

        // 6) disable; disabling again is fine
        disable().expect("disable");
        assert!(!is_enabled());
        disable().expect("disable again");

        std::env::remove_var("XDG_CONFIG_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

