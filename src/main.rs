mod app;
mod applier;
mod brightness;
mod config;
mod credentials;
mod fit;
mod http;
mod scheduler;
mod screen;
mod source;
mod state;
mod systemd;
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
    let resource = gio::Resource::load(concat!(env!("OUT_DIR"), "/valhalla.gresource"))
        .expect("failed to load compiled GResource");
    gio::resources_register(&resource);
}
