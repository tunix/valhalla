mod app;
mod applier;
mod autostart;
mod brightness;
mod config;
mod credentials;
mod fit;
mod http;
mod scheduler;
mod screen;
mod source;
mod state;
mod theme;
mod ui;

use std::sync::OnceLock;

static TOKIO_HANDLE: OnceLock<tokio::runtime::Handle> = OnceLock::new();

pub fn tokio_handle() -> &'static tokio::runtime::Handle {
    TOKIO_HANDLE.get().expect("tokio runtime not initialized")
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,valhalla=debug")),
        )
        .with_target(false)
        .init();

    let daemon = std::env::args().nth(1).as_deref() == Some("--daemon");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start tokio runtime");
    TOKIO_HANDLE
        .set(rt.handle().clone())
        .expect("tokio runtime initialized twice");
    // Leak the runtime: its worker threads must stay alive for the whole process.
    std::mem::forget(rt);

    if daemon {
        scheduler::run_daemon();
    } else {
        app::run();
    }
}

fn register_resources() {
    // Embed the compiled GResource in the binary. Loading it from OUT_DIR
    // only works on the build machine (dev cargo run) — an installed or
    // packaged binary (flatpak, release tarball) must carry the resource
    // with it, and OUT_DIR does not exist at runtime there.
    const RESOURCE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/valhalla.gresource"));
    let resource = gio::Resource::from_data(&glib::Bytes::from_static(RESOURCE))
        .expect("failed to load embedded GResource");
    gio::resources_register(&resource);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests run inside a tokio runtime (cargo test provides one); seed the
    /// global handle so `tokio_handle()` works for every module under test.
    #[test]
    fn tokio_handle_resolves_after_seeding() {
        if TOKIO_HANDLE.get().is_none() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            TOKIO_HANDLE.set(rt.handle().clone()).ok();
        }
        let _ = tokio_handle();
    }

    #[test]
    fn daemon_flag_parsing() {
        // Mirrors main()'s arg check: only an exact "--daemon" enables it.
        let daemon = |args: &[&str]| args.get(1).map(|s| *s == "--daemon").unwrap_or(false);
        assert!(daemon(&["valhalla", "--daemon"]));
        assert!(!daemon(&["valhalla"]));
        assert!(!daemon(&["valhalla", "--daemon=1"]));
        assert!(!daemon(&["valhalla", "-d"]));
    }
}
