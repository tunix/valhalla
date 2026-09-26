//! Management of the `valhalla.service` systemd user unit, used for
//! headless refreshes at login without the window.

use std::path::PathBuf;

const UNIT_NAME: &str = "valhalla.service";

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

/// Whether the user manager reports the unit as enabled.
pub fn is_enabled() -> bool {
    let Ok(output) = std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", UNIT_NAME])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).trim() == "enabled"
}

/// Install the unit and start it right away.
pub fn enable_start() -> Result<(), String> {
    install()?;
    systemctl(&["enable", "--now", UNIT_NAME])
}

pub fn stop_disable() -> Result<(), String> {
    systemctl(&["disable", "--now", UNIT_NAME])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_lifecycle() {
        // No --now here: the test must not launch the daemon.
        install().expect("unit installed");
        assert!(unit_path().exists());
        systemctl(&["enable", UNIT_NAME]).expect("enabled");
        assert!(is_enabled());
        systemctl(&["disable", UNIT_NAME]).expect("disabled");
        assert!(!is_enabled());
        std::fs::remove_file(unit_path()).ok();
    }
}
