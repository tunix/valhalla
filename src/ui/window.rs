use adw::prelude::*;
use gtk::{gio, glib};
use tokio::sync::mpsc;

use crate::{
    scheduler::{Msg, Trigger},
    source::Source,
    state::StateStore,
    theme::ColorScheme,
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

/// The wallpaper currently previewed in the window — the heart toggle's
/// subject. `scheme` is captured at apply-time.
#[derive(Debug, Clone)]
pub struct CurrentWallpaper {
    pub id: String,
    pub source: &'static str,
    pub path: std::path::PathBuf,
    pub title: Option<String>,
    pub author: Option<String>,
    pub scheme: ColorScheme,
}

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use super::CurrentWallpaper;

    use adw::subclass::prelude::*;
    use gtk::{gio, glib, TemplateChild};
    use tokio::sync::mpsc;

    use crate::scheduler::Trigger;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/tunix/valhalla/window.ui")]
    pub struct Window {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub dropdown_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub refresh_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub wallpaper_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub info_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub like_button: TemplateChild<gtk::Button>,

        pub settings: OnceLock<gio::Settings>,
        pub trig: OnceLock<mpsc::UnboundedSender<Trigger>>,
        pub dropdown: RefCell<Option<gtk::DropDown>>,
        /// The wallpaper currently previewed (path + id + source), so the heart
        /// toggle knows what to like/unlike. None = nothing liked-able yet.
        pub current_wallpaper: RefCell<Option<CurrentWallpaper>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "ValhallaWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl adw::subclass::application_window::AdwApplicationWindowImpl for Window {}
}

impl Window {
    pub fn new(
        app: &adw::Application,
        settings: &gio::Settings,
        trig: mpsc::UnboundedSender<Trigger>,
    ) -> Self {
        let window: Window = glib::Object::builder()
            .property("application", app)
            .property("icon-name", "io.github.tunix.valhalla")
            .build();
        let imp = window.imp();
        let _ = imp.settings.set(settings.clone());
        let _ = imp.trig.set(trig);

        window.setup_geometry_persistence();
        window.setup_dropdown();
        window.setup_actions();
        window.setup_close();
        window.watch_settings();
        window.update_banner();
        window.restore_preview();
        window.update_heart();
        window
    }

    /// Reflect favorite state on the heart button: filled red when liked,
    /// outline otherwise. No-op when nothing is previewed.
    fn update_heart(&self) {
        let imp = self.imp();
        let liked = imp
            .current_wallpaper
            .borrow()
            .as_ref()
            .map(|w| crate::favorites::contains(&w.id))
            .unwrap_or(false);
        let button = imp.like_button.get();
        let was_liked = button.has_css_class("liked");
        if liked {
            button.set_icon_name(crate::app::heart_icon_name());
            button.add_css_class("liked");
            button.set_tooltip_text(Some("Remove from Favorites"));
            // Small pop animation when the heart is newly liked: a "pop"
            // CSS animation on the button (GTK 4.14 supports CSS keyframes
            // on widget transforms; the class removal restarts it cleanly).
            if !was_liked {
                const POP_CLASS: &str = "heart-pop";
                let target = button.clone();
                button.remove_css_class(POP_CLASS);
                button.add_css_class(POP_CLASS);
                // Drop the class after the animation so it can re-run.
                glib::timeout_add_local(std::time::Duration::from_millis(260), move || {
                    target.remove_css_class(POP_CLASS);
                    glib::ControlFlow::Break
                });
            }
        } else {
            button.set_icon_name(crate::app::heart_icon_name());
            button.remove_css_class("liked");
            button.set_tooltip_text(Some("Add to Favorites"));
            // Symmetric shrink on un-like: same pop class, reversed feel via
            // the heart-unpop keyframes (scale down with a tiny undershoot).
            if was_liked {
                const UNPOP_CLASS: &str = "heart-unpop";
                let target = button.clone();
                button.remove_css_class(UNPOP_CLASS);
                button.add_css_class(UNPOP_CLASS);
                glib::timeout_add_local(std::time::Duration::from_millis(260), move || {
                    target.remove_css_class(UNPOP_CLASS);
                    glib::ControlFlow::Break
                });
            }
        }
    }

    fn imp(&self) -> &imp::Window {
        glib::subclass::types::ObjectSubclassIsExt::imp(self)
    }

    fn setup_dropdown(&self) {
        let imp = self.imp();
        let titles: Vec<&str> = Source::ALL.iter().map(|s| s.title()).collect();
        let model = gtk::StringList::new(&titles);
        let dropdown = gtk::DropDown::new(Some(model), None::<gtk::Expression>);

        let settings = imp.settings.get().unwrap().clone();
        let current = settings.string("active-source").to_string();
        let index = Source::ALL
            .iter()
            .position(|s| s.id() == current)
            .unwrap_or(0);
        dropdown.set_selected(index as u32);

        let s2 = settings.clone();
        dropdown.connect_selected_notify(move |dd| {
            if let Some(source) = Source::ALL.get(dd.selected() as usize) {
                let _ = s2.set_string("active-source", source.id());
            }
        });

        imp.dropdown_box.append(&dropdown);
        imp.dropdown.replace(Some(dropdown));
    }

    fn setup_actions(&self) {
        let imp = self.imp();

        let toggle_favorite = gio::SimpleAction::new("toggle-favorite", None);
        let win_fav = self.downgrade();
        toggle_favorite.connect_activate(move |_, _| {
            let Some(window) = win_fav.upgrade() else {
                return;
            };
            let imp = window.imp();
            let Some(current) = imp.current_wallpaper.borrow().clone() else {
                return; // nothing to like yet
            };
            let was_liked = crate::favorites::contains(&current.id);
            let result = if was_liked {
                crate::favorites::remove(&current.id)
            } else {
                crate::favorites::add(
                    &current.id,
                    current.source,
                    &current.path,
                    current.title.clone(),
                    current.author.clone(),
                    Some(current.scheme),
                )
            };
            if let Err(e) = &result {
                imp.toast_overlay.add_toast(adw::Toast::new(e));
            }
            window.update_heart();
            // A fresh like pre-warms the thumbnail and preview caches so
            // the Favorites page and the preview window are instant later.
            if !was_liked && result.is_ok() {
                if let Some(fav) = crate::favorites::list()
                    .into_iter()
                    .find(|f| f.id == current.id)
                {
                    crate::ui::thumbnails::prewarm(&fav.path);
                    crate::ui::thumbnails::prewarm_preview(&fav.path);
                }
            }
        });
        self.add_action(&toggle_favorite);

        let refresh = gio::SimpleAction::new("refresh", None);
        let trig = imp.trig.get().unwrap().clone();
        refresh.connect_activate(move |_, _| {
            let _ = trig.send(Trigger::Manual);
        });
        self.add_action(&refresh);

        let settings = imp.settings.get().unwrap().clone();
        let win = self.downgrade();
        let preferences = gio::SimpleAction::new("preferences", None);
        preferences.connect_activate(move |_, _| {
            if let Some(win) = win.upgrade() {
                let dialog = crate::ui::preferences::build(&settings);
                dialog.present(Some(&win));
            }
        });
        self.add_action(&preferences);

        let win2 = self.downgrade();
        let about = gio::SimpleAction::new("about", None);
        about.connect_activate(move |_, _| {
            if let Some(win) = win2.upgrade() {
                let dialog = adw::AboutDialog::new();
                dialog.set_application_name("Valhalla");
                dialog.set_application_icon("io.github.tunix.valhalla");
                dialog.set_version(env!("CARGO_PKG_VERSION"));
                dialog.set_website("https://github.com/tunix/valhalla");
                dialog.set_developer_name("Alper Kanat");
                dialog.set_comments("Theme-aware wallpaper switcher for Linux desktops");
                dialog.set_license_type(gtk::License::Gpl30Only);
                dialog.present(Some(&win));
            }
        });
        self.add_action(&about);

        let banner_settings = imp.settings.get().unwrap().clone();
        let win3 = self.downgrade();
        imp.status_banner.connect_button_clicked(move |_| {
            let _ = banner_settings.set_uint("refresh-interval", 60);
            if let Some(w) = win3.upgrade() {
                w.update_banner();
            }
        });
    }

    fn setup_close(&self) {
        let settings = self.imp().settings.get().unwrap().clone();
        let win = self.downgrade();
        self.connect_close_request(move |_| {
            let Some(window) = win.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if settings.boolean("run-in-background") {
                // Keep the process alive for scheduled refreshes; the menu's
                // Quit action exits fully.
                window.set_visible(false);
                tracing::info!("window hidden; continuing in the background");
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }

    /// Persist the window geometry with GTK's own mechanism: GTK 4 tracks
    /// interactive resizes in `default-width`/`default-height`
    /// (surface_size_changed → set_default_size_internal) and the window
    /// state in `maximized`, so plain GSettings binds both restore at
    /// construction and save continuously. A hard kill (Ctrl+C on
    /// `cargo run`) keeps the last size, unlike close-time saves which
    /// never run in that case. GTK itself skips remembering while the
    /// window is maximized/tiled/fullscreened.
    fn setup_geometry_persistence(&self) {
        let settings = self.imp().settings.get().unwrap();
        settings.bind("window-width", self, "default-width").build();
        settings
            .bind("window-height", self, "default-height")
            .build();
        settings.bind("window-maximized", self, "maximized").build();
    }

    fn watch_settings(&self) {
        let imp = self.imp();
        let settings = imp.settings.get().unwrap().clone();
        let win = self.downgrade();
        settings.connect_changed(None, move |_, key| {
            if key == "refresh-interval" {
                if let Some(w) = win.upgrade() {
                    w.update_banner();
                }
            }
        });
    }

    fn update_banner(&self) {
        let imp = self.imp();
        let settings = imp.settings.get().unwrap();
        let off = settings.uint("refresh-interval") == 0;
        imp.status_banner.set_title("Automatic refresh is off");
        imp.status_banner.set_button_label(Some("Enable"));
        imp.status_banner.set_revealed(off);
    }

    fn restore_preview(&self) {
        let imp = self.imp();
        let state = StateStore::new().read();
        let Some(entry) = state.history.first() else {
            return;
        };
        let path = std::path::PathBuf::from(&entry.path);
        if !path.exists() {
            return;
        }
        imp.wallpaper_picture.set_filename(Some(&path));
        // Heart toggle subject on restored preview too (scheme label comes
        // from state; source is the entry's source).
        let scheme = state.scheme.as_deref().map(|s| {
            if s == "dark" {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            }
        });
        *imp.current_wallpaper.borrow_mut() = Some(CurrentWallpaper {
            id: entry.id.clone(),
            source: "wallhaven", // best-effort label; favorites lookups are by id
            path: path.clone(),
            title: entry.title.clone(),
            author: entry.author.clone(),
            scheme: scheme.unwrap_or(ColorScheme::Light),
        });
        self.update_heart();
        let mut text = entry.source.clone();
        if let Some(author) = &entry.author {
            text.push_str(&format!(" · by {author}"));
        }
        if let Some(scheme) = &state.scheme {
            text.push_str(&format!(" · {scheme}"));
        }
        imp.info_label.set_text(&text);
    }

    pub fn handle_msg(&self, msg: Msg) {
        let imp = self.imp();
        match msg {
            Msg::Busy => {
                imp.refresh_stack.set_visible_child_name("spinner");
                imp.info_label.set_text("Fetching a new wallpaper…");
            }
            Msg::WallpaperSet {
                id,
                path,
                source,
                title,
                author,
                scheme,
            } => {
                imp.refresh_stack.set_visible_child_name("button");
                imp.wallpaper_picture.set_filename(Some(&path));
                // Heart toggle subject: the freshly applied wallpaper.
                *imp.current_wallpaper.borrow_mut() = Some(CurrentWallpaper {
                    id,
                    source,
                    path: path.clone(),
                    title: title.clone(),
                    author: author.clone(),
                    scheme,
                });
                let mut parts: Vec<String> = vec![source.to_string()];
                if let Some(author) = author.as_deref().filter(|a| !a.is_empty()) {
                    parts.push(format!("by {author}"));
                }
                if let Some(title) = title.as_deref().filter(|t| !t.is_empty()) {
                    parts.push(title.to_string());
                }
                parts.push(
                    match scheme {
                        ColorScheme::Dark => "dark",
                        ColorScheme::Light => "light",
                    }
                    .to_string(),
                );
                imp.info_label.set_text(&parts.join(" · "));
                self.update_heart();
            }
            Msg::Failed(err) => {
                imp.refresh_stack.set_visible_child_name("button");
                imp.toast_overlay.add_toast(adw::Toast::new(&err));
                self.restore_preview();
            }
            Msg::Info(text) => imp.info_label.set_text(&text),
            Msg::Notice(text) => {
                // Nothing changed: leave the spinner immediately and surface
                // the "why" as a transient toast instead.
                imp.refresh_stack.set_visible_child_name("button");
                imp.info_label.set_text(&text);
                imp.toast_overlay.add_toast(adw::Toast::new(&text));
            }
        }
    }
}
