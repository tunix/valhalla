use adw::prelude::*;
use gtk::{gio, glib};

use crate::credentials;

const INTERVAL_MINUTES: [u32; 5] = [0, 15, 60, 360, 1440];
const INTERVAL_LABELS: [&str; 5] = [
    "Off",
    "Every 15 minutes",
    "Hourly",
    "Every 6 hours",
    "Daily",
];

const STRICTNESS_VALUES: [&str; 2] = ["prefer", "require"];
const STRICTNESS_LABELS: [&str; 2] = ["Prefer match", "Require match"];

const WALLHAVEN_SORTING: [&str; 6] = [
    "random",
    "date_added",
    "views",
    "favorites",
    "toplist",
    "relevance",
];
const WALLHAVEN_SORTING_LABELS: [&str; 6] = [
    "Random",
    "Newest",
    "Most viewed",
    "Most favorited",
    "Toplist",
    "Relevance",
];

const CONTENT_FILTER_VALUES: [&str; 2] = ["low", "high"];
const CONTENT_FILTER_LABELS: [&str; 2] = ["Low (moderate)", "High (strict)"];

/// Build the preferences dialog for the given settings.
pub fn build(settings: &gio::Settings) -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::new();
    // 8 pages: HIG recommends enabling search once the sidebar grows.
    dialog.set_search_enabled(true);

    dialog.add(&general_page(settings));

    let (wh_page, wh_key_row, nsfw_row, _purity_rows) = wallhaven_page(settings);
    dialog.add(&wh_page);

    let (un_page, un_key_row) = unsplash_page(settings);
    dialog.add(&un_page);

    let (px_page, px_key_row) = pexels_page(settings);
    dialog.add(&px_page);

    let (pb_page, pb_key_row) = pixabay_page(settings);
    dialog.add(&pb_page);

    dialog.add(&bing_page());
    let nasa_key_row = adw::EntryRow::new();
    nasa_key_row.set_title("API key (optional)");
    nasa_key_row.set_show_apply_button(true);
    let nasa_row_weak = nasa_key_row.downgrade();
    nasa_key_row.connect_apply(move |row| {
        let text = row.text().to_string();
        let h = crate::tokio_handle().clone();
        h.spawn_blocking(move || {
            if text.is_empty() {
                credentials::clear("nasa");
            } else if let Err(e) = credentials::set("nasa", &text) {
                tracing::warn!("failed to store key: {e}");
            }
        });
    });
    dialog.add(&nasa_page(nasa_key_row.clone()));

    let manual_page = manual_page(settings);
    dialog.add(&manual_page);

    // Load stored keys from the keyring off the main thread.
    let (tx, rx) = async_channel::unbounded::<(Option<String>, Option<String>)>();
    for source in ["wallhaven", "unsplash", "pexels", "pixabay", "nasa"] {
        let tx = tx.clone();
        crate::tokio_handle().spawn(async move {
            let key = crate::tokio_handle()
                .spawn_blocking(move || credentials::get(source))
                .await
                .ok()
                .flatten();
            let _ = tx.send((Some(source.to_string()), key)).await;
        });
    }
    drop(tx);
    let wh_weak = wh_key_row.downgrade();
    let un_weak = un_key_row.downgrade();
    let px_weak = px_key_row.downgrade();
    let pb_weak = pb_key_row.downgrade();
    let nasa_weak = nasa_row_weak.clone();
    let nsfw_weak = nsfw_row.downgrade();
    glib::spawn_future_local(async move {
        while let Ok((source, key)) = rx.recv().await {
            let Some(key) = key else { continue };
            let row = match source.as_deref().unwrap_or("") {
                "wallhaven" => wh_weak.upgrade(),
                "unsplash" => un_weak.upgrade(),
                "pexels" => px_weak.upgrade(),
                "pixabay" => pb_weak.upgrade(),
                "nasa" => nasa_weak.upgrade(),
                _ => None,
            };
            if let Some(row) = row {
                row.set_text(&key);
            }
            if source.as_deref() == Some("wallhaven") {
                if let Some(row) = nsfw_weak.upgrade() {
                    row.set_sensitive(true);
                }
            }
        }
    });

    dialog
}

fn general_page(settings: &gio::Settings) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("General");
    page.set_icon_name(Some("applications-system-symbolic"));

    let refresh_group = adw::PreferencesGroup::new();
    refresh_group.set_title("Automatic refresh");
    page.add(&refresh_group);

    let interval = adw::ComboRow::new();
    interval.set_title("Interval");
    interval.set_model(Some(&gtk::StringList::new(&INTERVAL_LABELS)));
    let current = settings.uint("refresh-interval");
    let index = INTERVAL_MINUTES
        .iter()
        .position(|v| *v == current)
        .unwrap_or(2);
    interval.set_selected(index as u32);
    {
        let s = settings.clone();
        interval.connect_selected_notify(move |row| {
            let _ = s.set_uint(
                "refresh-interval",
                INTERVAL_MINUTES[row.selected() as usize],
            );
        });
    }
    refresh_group.add(&interval);

    let theme_match = adw::SwitchRow::new();
    theme_match.set_title("Match desktop theme");
    theme_match.set_subtitle("Pick wallpapers that suit the current light or dark appearance");
    settings.bind("match-theme", &theme_match, "active").build();
    refresh_group.add(&theme_match);

    let strictness = adw::ComboRow::new();
    strictness.set_title("Theme match strictness");
    strictness.set_subtitle("Require a matching tone, or prefer it and fall back gracefully");
    bind_combo(
        &strictness,
        settings,
        "match-strictness",
        &STRICTNESS_VALUES,
        &STRICTNESS_LABELS,
    );
    refresh_group.add(&strictness);

    let threshold = adw::SpinRow::with_range(0.0, 1.0, 0.05);
    threshold.set_title("Dark threshold");
    threshold.set_subtitle("Average brightness below this counts as dark");
    threshold.set_digits(2);
    threshold.set_value(settings.double("brightness-threshold"));
    {
        let s = settings.clone();
        threshold.connect_value_notify(move |row| {
            let _ = s.set_double("brightness-threshold", row.value());
        });
    }
    refresh_group.add(&threshold);

    let behavior_group = adw::PreferencesGroup::new();
    behavior_group.set_title("Behavior");
    page.add(&behavior_group);

    let on_theme_change = adw::SwitchRow::new();
    on_theme_change.set_title("Refresh on theme change");
    on_theme_change.set_subtitle("Switch the wallpaper immediately when light/dark mode changes");
    settings
        .bind("refresh-on-theme-change", &on_theme_change, "active")
        .build();
    behavior_group.add(&on_theme_change);

    let lockscreen = adw::SwitchRow::new();
    lockscreen.set_title("Also change the lock screen");
    settings
        .bind("apply-lockscreen", &lockscreen, "active")
        .build();
    behavior_group.add(&lockscreen);

    let background = adw::SwitchRow::new();
    background.set_title("Run in the background");
    background.set_subtitle("Keep the window running in the background when closed");
    settings
        .bind("run-in-background", &background, "active")
        .build();
    behavior_group.add(&background);

    let startup = adw::SwitchRow::new();
    startup.set_title("Refresh on startup");
    startup.set_subtitle("Download a fresh wallpaper when Valhalla starts");
    settings
        .bind("refresh-on-startup", &startup, "active")
        .build();
    behavior_group.add(&startup);

    let login = adw::SwitchRow::new();
    login.set_title("Start at login");
    login.set_subtitle("Launch the background daemon automatically when you log in");
    // Reflect current state; toggle drives the real action on a blocking
    // thread (file IO must stay off the main loop).
    login.set_active(crate::autostart::is_enabled());
    {
        let row_weak = login.downgrade();
        login.connect_active_notify(move |row| {
            let want = row.is_active();
            // Widgets are main-thread-only: the blocking job does file IO and
            // reports back over a channel; the receiver (main loop) re-syncs
            // the switch with reality.
            let (done_tx, done_rx) = async_channel::unbounded::<Result<(), String>>();
            crate::tokio_handle().spawn_blocking(move || {
                let result = if want {
                    crate::autostart::enable()
                } else {
                    crate::autostart::disable()
                };
                let _ = done_tx.send_blocking(result);
            });
            let row_weak = row_weak.clone();
            glib::spawn_future_local(async move {
                if let Ok(res) = done_rx.recv().await {
                    if let Err(e) = res {
                        tracing::warn!("autostart toggle failed: {e}");
                    }
                    if let Some(row) = row_weak.upgrade() {
                        let actual = crate::autostart::is_enabled();
                        if row.is_active() != actual {
                            row.set_active(actual);
                        }
                    }
                }
            });
        });
    }
    behavior_group.add(&login);

    page
}

fn wallhaven_page(
    settings: &gio::Settings,
) -> (
    adw::PreferencesPage,
    adw::EntryRow,
    adw::SwitchRow,
    Vec<adw::SwitchRow>,
) {
    let page = adw::PreferencesPage::new();
    page.set_title("Wallhaven");
    page.set_icon_name(Some("source-wallhaven"));

    let api_group = adw::PreferencesGroup::new();
    api_group.set_title("API");
    page.add(&api_group);

    let api_key = adw::EntryRow::new();
    api_key.set_title("API key");
    api_key.set_show_apply_button(true);
    api_group.add(&api_key);

    let nsfw = adw::SwitchRow::new();
    nsfw.set_title("Allow NSFW");
    nsfw.set_subtitle("Requires an API key");
    nsfw.set_sensitive(false);
    api_group.add(&nsfw);

    let search_group = adw::PreferencesGroup::new();
    search_group.set_title("Search");
    page.add(&search_group);

    let query = adw::EntryRow::new();
    query.set_title("Tags");
    query.set_tooltip_text(Some("Wallhaven search syntax: +required -excluded"));
    settings.bind("wallhaven-query", &query, "text").build();
    search_group.add(&query);

    let sorting = adw::ComboRow::new();
    sorting.set_title("Sorting");
    bind_combo(
        &sorting,
        settings,
        "wallhaven-sorting",
        &WALLHAVEN_SORTING,
        &WALLHAVEN_SORTING_LABELS,
    );
    search_group.add(&sorting);

    let categories = build_string_triple(
        settings,
        "wallhaven-categories",
        &["General", "Anime", "People"],
    );
    for row in &categories {
        search_group.add(row);
    }

    let purity = build_string_triple(settings, "wallhaven-purity", &["SFW", "Sketchy", "NSFW"]);
    for row in &purity {
        search_group.add(row);
    }

    // Saving the API key also unlocks the NSFW switch.
    {
        let purity_rows = purity.clone();
        let nsfw_row = nsfw.clone();
        api_key.connect_apply(move |row| {
            let text = row.text().to_string();
            nsfw_row.set_sensitive(!text.is_empty());
            let h = crate::tokio_handle().clone();
            h.spawn_blocking(move || {
                if text.is_empty() {
                    credentials::clear("wallhaven");
                } else if let Err(e) = credentials::set("wallhaven", &text) {
                    tracing::warn!("failed to store key: {e}");
                }
            });
            let _ = purity_rows;
        });
    }

    (page, api_key, nsfw, purity)
}

fn pexels_page(settings: &gio::Settings) -> (adw::PreferencesPage, adw::EntryRow) {
    let page = adw::PreferencesPage::new();
    page.set_title("Pexels");
    page.set_icon_name(Some("source-pexels"));

    let api_group = adw::PreferencesGroup::new();
    api_group.set_title("API");
    page.add(&api_group);

    let api_key = adw::EntryRow::new();
    api_key.set_title("API key");
    api_key.set_show_apply_button(true);
    api_group.add(&api_key);

    let search_group = adw::PreferencesGroup::new();
    search_group.set_title("Search");
    page.add(&search_group);

    let query = adw::EntryRow::new();
    query.set_title("Query");
    settings.bind("pexels-query", &query, "text").build();
    search_group.add(&query);

    api_key.connect_apply(move |row| {
        let text = row.text().to_string();
        let h = crate::tokio_handle().clone();
        h.spawn_blocking(move || {
            if text.is_empty() {
                credentials::clear("pexels");
            } else if let Err(e) = credentials::set("pexels", &text) {
                tracing::warn!("failed to store key: {e}");
            }
        });
    });

    (page, api_key)
}

fn pixabay_page(settings: &gio::Settings) -> (adw::PreferencesPage, adw::EntryRow) {
    let page = adw::PreferencesPage::new();
    page.set_title("Pixabay");
    page.set_icon_name(Some("source-pixabay"));

    let api_group = adw::PreferencesGroup::new();
    api_group.set_title("API");
    page.add(&api_group);

    let api_key = adw::EntryRow::new();
    api_key.set_title("API key");
    api_key.set_show_apply_button(true);
    api_group.add(&api_key);

    let search_group = adw::PreferencesGroup::new();
    search_group.set_title("Search");
    page.add(&search_group);

    let query = adw::EntryRow::new();
    query.set_title("Query");
    settings.bind("pixabay-query", &query, "text").build();
    search_group.add(&query);

    let category = adw::EntryRow::new();
    category.set_title("Category");
    category.set_tooltip_text(Some("Optional Pixabay category, e.g. nature, backgrounds"));
    settings.bind("pixabay-category", &category, "text").build();
    search_group.add(&category);

    api_key.connect_apply(move |row| {
        let text = row.text().to_string();
        let h = crate::tokio_handle().clone();
        h.spawn_blocking(move || {
            if text.is_empty() {
                credentials::clear("pixabay");
            } else if let Err(e) = credentials::set("pixabay", &text) {
                tracing::warn!("failed to store key: {e}");
            }
        });
    });

    (page, api_key)
}

/// Bing's daily wallpaper needs no settings; the page explains that.
fn bing_page() -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("Bing");
    page.set_icon_name(Some("source-bing"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Daily wallpaper");
    group.set_description(Some(
        "Bing's featured daily image needs no configuration. The archive keeps the last 8 days, so refreshes stay varied.",
    ));
    page.add(&group);

    page
}

/// NASA imagery: optional key (demo access works without one).
fn nasa_page(key_row: adw::EntryRow) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("NASA");
    page.set_icon_name(Some("source-nasa"));

    let api_group = adw::PreferencesGroup::new();
    api_group.set_title("API");
    api_group.set_description(Some(
        "Optional: a free key from api.nasa.gov lifts the rate limit. Leave empty for limited demo access.",
    ));
    page.add(&api_group);
    api_group.add(&key_row);

    page
}

/// Manual source page: pick individual files and/or a directory (all images
/// inside are added). The selected set is listed and removable.
fn manual_page(settings: &gio::Settings) -> adw::PreferencesPage {
    use adw::prelude::*;

    let page = adw::PreferencesPage::new();
    page.set_title("Manual");
    page.set_icon_name(Some("folder-pictures-symbolic"));

    let group = adw::PreferencesGroup::new();
    group.set_title("Your wallpapers");
    group.set_description(Some(
        "Files and folders cycle with the theme-matching engine; stored locally, used offline.",
    ));
    page.add(&group);

    // Pick buttons row
    let pick_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let pick_files = gtk::Button::with_label("Add files…");
    pick_files.add_css_class("suggested-action");
    let pick_dir = gtk::Button::with_label("Add folder…");
    pick_dir.add_css_class("flat");
    pick_box.append(&pick_files);
    pick_box.append(&pick_dir);
    group.add(&pick_box);

    // List of selected paths (refreshed on change)
    let list_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    group.add(&list_box);

    let rebuild_list = {
        let list_box = list_box.clone();
        move || {
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            for p in crate::manual::paths() {
                let row = adw::ActionRow::new();
                let name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<path>")
                    .to_string();
                let is_dir = p.is_dir();
                row.set_title(&name);
                row.set_subtitle(&p.to_string_lossy());
                let icon = if is_dir {
                    "folder-symbolic"
                } else {
                    "image-x-generic-symbolic"
                };
                row.add_prefix(&gtk::Image::from_icon_name(icon));
                let remove_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                remove_btn.add_css_class("flat");
                remove_btn.set_tooltip_text(Some("Remove"));
                let p_c = p.clone();
                remove_btn.connect_clicked(move |_| {
                    if let Err(e) = crate::manual::remove(&p_c) {
                        tracing::warn!("manual remove failed: {e}");
                    }
                    // Rebuild happens via the paths watcher below.
                });
                row.add_suffix(&remove_btn);
                list_box.append(&row);
            }
            if crate::manual::paths().is_empty() {
                let empty = gtk::Label::new(Some("Nothing selected yet"));
                empty.add_css_class("dimmed");
                list_box.append(&empty);
            }
        }
    };

    // File chooser: files (multi-select)
    {
        let rebuild = rebuild_list.clone();
        pick_files.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            dialog.set_title("Pick wallpapers");
            let rebuild = rebuild.clone();
            dialog.open_multiple(
                None::<&gtk::Window>,
                gio::Cancellable::NONE,
                move |result| {
                    let Ok(files) = result else { return };
                    let paths: Vec<std::path::PathBuf> = (0..files.n_items())
                        .filter_map(|i| files.item(i))
                        .filter_map(|obj| obj.downcast::<gio::File>().ok())
                        .filter_map(|f| f.path())
                        .collect();
                    if paths.is_empty() {
                        return;
                    }
                    let rebuild = rebuild.clone();
                    crate::tokio_handle().spawn_blocking(move || {
                        if let Err(e) = crate::manual::add(&paths) {
                            tracing::warn!("manual add failed: {e}");
                        }
                    });
                    // Rebuild after the blocking write completes.
                    glib::spawn_future_local(async move {
                        rebuild();
                    });
                },
            );
        });
    }

    // Folder chooser: expands to all images inside (one level)
    {
        let rebuild = rebuild_list.clone();
        pick_dir.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            dialog.set_title("Pick a folder of wallpapers");
            let rebuild = rebuild.clone();
            dialog.select_folder(
                None::<&gtk::Window>,
                gio::Cancellable::NONE,
                move |result| {
                    let Ok(file) = result else { return };
                    let Some(path) = file.path() else { return };
                    let rebuild = rebuild.clone();
                    crate::tokio_handle().spawn_blocking(move || {
                        if let Err(e) = crate::manual::add(&[path]) {
                            tracing::warn!("manual folder add failed: {e}");
                        }
                    });
                    glib::spawn_future_local(async move {
                        rebuild();
                    });
                },
            );
        });
    }

    // Initial fill + refresh after any change (paths watcher)
    rebuild_list();
    {
        let rebuild = rebuild_list.clone();
        settings.connect_changed(Some("manual-paths"), move |_, _| {
            rebuild();
        });
    }

    page
}

fn unsplash_page(settings: &gio::Settings) -> (adw::PreferencesPage, adw::EntryRow) {
    let page = adw::PreferencesPage::new();
    page.set_title("Unsplash");
    page.set_icon_name(Some("source-unsplash"));

    let api_group = adw::PreferencesGroup::new();
    api_group.set_title("API");
    page.add(&api_group);

    let access_key = adw::EntryRow::new();
    access_key.set_title("Access key");
    access_key.set_show_apply_button(true);
    api_group.add(&access_key);

    let search_group = adw::PreferencesGroup::new();
    search_group.set_title("Search");
    page.add(&search_group);

    let query = adw::EntryRow::new();
    query.set_title("Query");
    settings.bind("unsplash-query", &query, "text").build();
    search_group.add(&query);

    let content_filter = adw::ComboRow::new();
    content_filter.set_title("Content filter");
    bind_combo(
        &content_filter,
        settings,
        "unsplash-content-filter",
        &CONTENT_FILTER_VALUES,
        &CONTENT_FILTER_LABELS,
    );
    search_group.add(&content_filter);

    access_key.connect_apply(move |row| {
        let text = row.text().to_string();
        let h = crate::tokio_handle().clone();
        h.spawn_blocking(move || {
            if text.is_empty() {
                credentials::clear("unsplash");
            } else if let Err(e) = credentials::set("unsplash", &text) {
                tracing::warn!("failed to store key: {e}");
            }
        });
    });

    (page, access_key)
}

/// Three switches persisted as a "101"-style bit string, e.g. categories/purity.
fn build_string_triple(
    settings: &gio::Settings,
    key: &'static str,
    labels: &[&str; 3],
) -> Vec<adw::SwitchRow> {
    let rows: Vec<adw::SwitchRow> = labels
        .iter()
        .map(|label| {
            let row = adw::SwitchRow::new();
            row.set_title(label);
            row
        })
        .collect();

    let current = settings.string(key).to_string();
    for (i, row) in rows.iter().enumerate() {
        row.set_active(
            current
                .as_str()
                .chars()
                .nth(i)
                .map(|c| c == '1')
                .unwrap_or(true),
        );
    }

    let s = settings.clone();
    for row in &rows {
        let s = s.clone();
        let row_refs = rows.clone();
        row.connect_active_notify(move |_| {
            let mut value = String::new();
            for r in &row_refs {
                value.push(if r.is_active() { '1' } else { '0' });
            }
            let _ = s.set_string(key, &value);
        });
    }

    rows
}

fn bind_combo(
    row: &adw::ComboRow,
    settings: &gio::Settings,
    key: &'static str,
    values: &[&'static str],
    labels: &[&str],
) {
    row.set_model(Some(&gtk::StringList::new(labels)));
    let current = settings.string(key).to_string();
    let index = values.iter().position(|v| *v == current).unwrap_or(0);
    row.set_selected(index as u32);

    let s = settings.clone();
    let values = values.to_vec();
    row.connect_selected_notify(move |r| {
        let _ = s.set_string(key, values[r.selected() as usize]);
    });
}
