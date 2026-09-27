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

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

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

        pub settings: OnceLock<gio::Settings>,
        pub trig: OnceLock<mpsc::UnboundedSender<Trigger>>,
        pub dropdown: RefCell<Option<gtk::DropDown>>,
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
        let window: Window = glib::Object::builder().property("application", app).build();
        let imp = window.imp();
        let _ = imp.settings.set(settings.clone());
        let _ = imp.trig.set(trig);

        window.setup_dropdown();
        window.setup_actions();
        window.setup_close();
        window.watch_settings();
        window.update_banner();
        window.restore_preview();
        window
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
                dialog.set_developer_name("Tunix");
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
                path,
                source,
                title,
                author,
                scheme,
            } => {
                imp.refresh_stack.set_visible_child_name("button");
                imp.wallpaper_picture.set_filename(Some(&path));
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
            }
            Msg::Failed(err) => {
                imp.refresh_stack.set_visible_child_name("button");
                imp.toast_overlay.add_toast(adw::Toast::new(&err));
                self.restore_preview();
            }
            Msg::Info(text) => imp.info_label.set_text(&text),
        }
    }
}
