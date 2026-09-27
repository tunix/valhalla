use std::sync::Arc;

use gtk::{gdk, gio, glib, prelude::*};

use crate::{config, scheduler, ui};

pub fn run() {
    crate::register_resources();

    let app = adw::Application::new(
        Some("io.github.tunix.valhalla"),
        gio::ApplicationFlags::empty(),
    );

    app.connect_activate(|app| {
        load_css();
        register_icon_theme();

        if let Some(existing) = app.active_window() {
            existing.present();
            return;
        }

        let settings = config::settings();
        let (tx, rx) = async_channel::unbounded::<scheduler::Msg>();

        // Keep the persisted screen geometry in sync with the real monitors.
        if let Some(s) = crate::screen::probe() {
            config::store_screen_geometry(&settings, s);
        }

        let core = Arc::new(scheduler::Core::new(settings.clone(), tx));
        let trig = core.clone().spawn();
        let _ = trig.send(scheduler::Trigger::Startup);

        let window = ui::window::Window::new(app, &settings, trig.clone());

        let msg_window = window.clone();
        glib::spawn_future_local(async move {
            while let Ok(msg) = rx.recv().await {
                msg_window.handle_msg(msg);
            }
        });

        let trig_settings = trig.clone();
        settings.connect_changed(None, move |_, key| {
            if scheduler::is_source_key(key) {
                let _ = trig_settings.send(scheduler::Trigger::SettingsChange);
            }
        });

        // GSettings theme watch as a fallback when the portal is unavailable.
        let iface = gio::Settings::new("org.gnome.desktop.interface");
        let trig_theme = trig.clone();
        iface.connect_changed(Some("color-scheme"), move |_, _| {
            let _ = trig_theme.send(scheduler::Trigger::ThemeChange);
        });

        window.present();
    });

    let quit = gio::SimpleAction::new("quit", None);
    {
        let app = app.clone();
        quit.connect_activate(move |_, _| app.quit());
    }
    app.add_action(&quit);

    app.run();
}

fn register_icon_theme() {
    if let Some(display) = gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_resource_path("/io/github/tunix/valhalla/icons");
    }
}

fn load_css() {
    let css = gtk::CssProvider::new();
    css.load_from_string(".preview { border-radius: 16px; }\n");
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
