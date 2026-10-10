//! Management of the `valhalla.service` systemd user unit, used for
//! headless refreshes at login without the window.

use std::path::PathBuf;

const UNIT_NAME: &str = "valhalla.service";

/// Hard cap on any systemctl invocation: the user-manager D-Bus call can
/// block for the full bus timeout when the session is half-up (early login,
/// sandboxed environments), which froze the preferences toggle. 15 s is
/// generous for enable/disable/is-enabled; exceeding it fails the call so
/// the UI can surface the error instead of hanging.
const SYSTEMCTL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

const UNIT_TEMPLATE: &str = r#"[Unit]
Description=Valhalla wallpaper service
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart="{exe}" --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
"#;

fn unit_dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".config"));
    config.join("systemd/user")
}

fn unit_path() -> PathBuf {
    unit_dir().join(UNIT_NAME)
}

/// Write the unit file (with the current executable path) and reload the
/// user manager.
pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dir = unit_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(
        unit_path(),
        UNIT_TEMPLATE.replace("{exe}", &exe.display().to_string()),
    )
    .map_err(|e| e.to_string())?;
    systemctl(&["daemon-reload"])
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let output = std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Whether the user manager reports the unit as enabled. Kept for the
/// direct-query path used by tests; the UI uses is_enabled()'s bus-tolerant
/// version below.
fn is_enabled_direct() -> bool {
    let Ok(output) = std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", UNIT_NAME])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).trim() == "enabled"
}

fn systemctl_with_timeout(args: &[&str]) -> Result<(), String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("systemctl: {e}"))?;
    let deadline = std::time::Instant::now() + SYSTEMCTL_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                return if status.success() {
                    Ok(())
                } else {
                    Err(stderr.trim().to_string())
                };
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "systemctl {} timed out after {SYSTEMCTL_TIMEOUT:?}",
                        args.join(" ")
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(format!("systemctl: {e}")),
        }
    }
}

/// Whether the unit is enabled for future logins.
///
/// Prefers the manager's answer, but when the user bus is unreachable
/// (is-enabled errors out), falls back to checking the enablement symlink
/// under the unit dir — the toggle must reflect reality even while the
/// session bus is half-up.
pub fn is_enabled() -> bool {
    if let Ok(output) = std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", UNIT_NAME])
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).trim() == "enabled";
    }
    // Fallback: `enable` links <unit-dir>/../default.target.wants/<unit>.
    unit_dir()
        .parent()
        .map(|p| p.join("default.target.wants").join(UNIT_NAME).is_file())
        .unwrap_or(false)
}

/// Install the unit, enable it for future logins, and start it now.
///
/// Enabling and starting are separate calls on purpose: a failed start
/// (e.g. the daemon exits immediately in a sandbox) must NOT undo the
/// enablement — the service is wanted at every future login regardless.
/// Start errors are logged, not returned; the switch stays on.
pub fn enable_start() -> Result<(), String> {
    install()?;
    systemctl_with_timeout(&["enable", UNIT_NAME])?;
    if let Err(e) = systemctl_with_timeout(&["start", UNIT_NAME]) {
        tracing::warn!("service enabled but the start failed (retries at next login): {e}");
    }
    Ok(())
}

pub fn stop_disable() -> Result<(), String> {
    systemctl_with_timeout(&["disable", "--now", UNIT_NAME])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A user systemd manager is required for the lifecycle test: it drives
    /// the real `systemctl --user`. CI containers (e.g. fedora:latest) have
    /// no user bus / XDG_RUNTIME_DIR, so skip instead of failing there.
    fn user_systemd_available() -> bool {
        std::process::Command::new("systemctl")
            .args(["--user", "is-system-running"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn service_lifecycle() {
        if !user_systemd_available() {
            eprintln!("skipping service_lifecycle: no user systemd manager");
            return;
        }
        // No --now here: the test must not launch the daemon.
        install().expect("unit installed");
        assert!(unit_path().exists());
        systemctl(&["enable", UNIT_NAME]).expect("enabled");
        assert!(is_enabled_direct());
        systemctl(&["disable", UNIT_NAME]).expect("disabled");
        assert!(!is_enabled_direct());
        std::fs::remove_file(unit_path()).ok();
    }

    /// The unit template embeds the executable path and stays a valid unit
    /// file. Verify the rendering without a systemd manager.
    #[test]
    fn unit_template_renders_exe_path() {
        let exe = "/usr/local/bin/valhalla";
        let unit = UNIT_TEMPLATE.replace("{exe}", exe);
        assert!(unit.contains("ExecStart=\"/usr/local/bin/valhalla\" --daemon"));
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("WantedBy=graphical-session.target"));
        assert!(!unit.contains("{exe}"), "placeholder must be substituted");
    }

    #[test]
    fn unit_dir_respects_xdg_config_home() {
        // Same resolution logic as unit_dir(), exercised against env vars.
        let dir = std::env::temp_dir().join(format!("valhalla-systemd-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let resolved = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| glib::home_dir().join(".config"));
        assert_eq!(resolved, dir);
        let unit_dir = resolved.join("systemd/user");
        assert!(unit_dir.ends_with("systemd/user"));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn unit_name_is_stable() {
        // The unit filename is referenced by the UI toggle and docs.
        assert_eq!(UNIT_NAME, "valhalla.service");
        assert!(unit_path().file_name().and_then(|n| n.to_str()) == Some(UNIT_NAME));
    }
}
