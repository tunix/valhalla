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
//! entry where the session manager looks. The Exec line is
//! `flatpak run <app-id> --daemon`: the host session cannot see the
//! sandbox-internal binary, only the Flatpak launcher.

use std::path::PathBuf;

const APP_ID: &str = "io.github.tunix.valhalla";

/// Exec resolution by packaging mode:
///   - Native/deb/rpm: `valhalla --daemon` via PATH (binary installed to
///     /usr/bin or ~/.cargo/bin).
///   - Flatpak: the sandbox-internal binary is invisible to the host
///     session, so the entry must go through the Flatpak launcher:
///     `flatpak run <app-id> --daemon`. Detection is /.flatpak-info
///     (see [`crate::applier::is_flatpak`]), which only exists inside
///     the sandbox — exactly where this entry gets written.
///   - AppImage: the binary only exists under a transient /tmp/.mount_*
///     mount, so PATH is not an option — the entry must point at the
///     AppImage file itself (the path is stable as long as the user keeps
///     the file where it is, which is the normal case).
fn exec_command() -> String {
    exec_command_for(crate::applier::is_flatpak())
}

/// [`exec_command`] with Flatpak detection injected, so tests can exercise
/// each packaging mode without a real /.flatpak-info.
fn exec_command_for(flatpak: bool) -> String {
    // AppImage runtime sets $APPIMAGE to the absolute path of the file.
    if let Ok(path) = std::env::var("APPIMAGE") {
        if !path.is_empty() && std::path::Path::new(&path).is_file() {
            return format!("{path:?} --daemon"); // quoted: paths may contain spaces
        }
    }
    if flatpak {
        return format!("flatpak run {APP_ID} --daemon");
    }
    "valhalla --daemon".to_string()
}

const ENTRY_TEMPLATE: &str = r#"[Desktop Entry]
Type=Application
Name=Valhalla (background)
Comment=Theme-aware wallpaper refresh daemon
Exec={exec}
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

/// Write (or re-point) the autostart entry. The Exec line is resolved by
/// packaging mode — see [`exec_command`].
pub fn enable() -> Result<(), String> {
    let dir = autostart_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let entry = ENTRY_TEMPLATE
        .replace("{exec}", &exec_command())
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
        // Uniform rule: any test touching process-global state (env vars)
        // holds ENV_LOCK for its whole body (see crate::http::test_env).
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("valhalla-autostart-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let _ = std::fs::remove_dir_all(&dir);

        // 1) starts disabled
        assert!(!is_enabled());

        // 2) enable writes a valid entry with the packaging-appropriate Exec
        enable().expect("enable");
        assert!(is_enabled());
        let text = std::fs::read_to_string(entry_path()).unwrap();
        // Test env has no $APPIMAGE -> PATH-resolved Exec.
        assert!(text.contains("Exec=valhalla --daemon"));
        assert!(text.contains(&format!("Icon={APP_ID}")));
        assert!(!text.contains("{exe}"), "placeholder substituted");
        assert!(!text.contains("{exec}"), "placeholder substituted");

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
            text.replace(&format!("Icon={APP_ID}"), "Icon=stale"),
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

        // 7) Exec resolution per packaging mode (same thread: APPIMAGE is
        // process-global and parallel tests would race the env).
        // 7a) AppImage mode: $APPIMAGE pointing at a real file
        let fake =
            std::env::temp_dir().join(format!("fake-appimage-{}.AppImage", std::process::id()));
        std::fs::write(&fake, b"magma").unwrap();
        std::env::set_var("APPIMAGE", &fake);
        let cmd = exec_command();
        std::env::remove_var("APPIMAGE");
        std::fs::remove_file(&fake).ok();
        // Quoted path + daemon flag.
        assert!(cmd.starts_with('"'), "path must be quoted: {cmd}");
        assert!(cmd.ends_with("\" --daemon"), "cmd: {cmd}");

        // 7b) AppImage path not a real file -> PATH fallback
        std::env::set_var("APPIMAGE", "/nonexistent/path.AppImage");
        let cmd = exec_command();
        std::env::remove_var("APPIMAGE");
        assert_eq!(cmd, "valhalla --daemon");

        // 7c) Flatpak mode: the host session must launch through the
        // Flatpak launcher, not the invisible sandbox binary.
        let cmd = exec_command_for(true);
        assert_eq!(cmd, format!("flatpak run {APP_ID} --daemon"));

        // 7d) AppImage wins over Flatpak (order-of-precedence guard).
        let fake =
            std::env::temp_dir().join(format!("fake-appimage-{}.AppImage", std::process::id()));
        std::fs::write(&fake, b"magma").unwrap();
        std::env::set_var("APPIMAGE", &fake);
        let cmd = exec_command_for(true);
        std::env::remove_var("APPIMAGE");
        std::fs::remove_file(&fake).ok();
        assert!(cmd.ends_with("\" --daemon"), "cmd: {cmd}");
    }
}
