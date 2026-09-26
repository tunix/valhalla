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
    dialog.set_search_enabled(false);

    dialog.add(&general_page(settings));

    let (wh_page, wh_key_row, nsfw_row, _purity_rows) = wallhaven_page(settings);
    dialog.add(&wh_page);

    let (un_page, un_key_row) = unsplash_page(settings);
    dialog.add(&un_page);

    // Load stored keys from the keyring off the main thread.
    let (tx, rx) = async_channel::unbounded::<(Option<String>, Option<String>)>();
    crate::tokio_handle().spawn(async move {
        let wh = crate::tokio_handle()
            .spawn_blocking(|| credentials::get("wallhaven"))
            .await
            .ok()
            .flatten();
        let un = crate::tokio_handle()
            .spawn_blocking(|| credentials::get("unsplash"))
            .await
            .ok()
            .flatten();
        let _ = tx.send((wh, un)).await;
    });
    let wh_weak = wh_key_row.downgrade();
    let un_weak = un_key_row.downgrade();
    let nsfw_weak = nsfw_row.downgrade();
    glib::spawn_future_local(async move {
        while let Ok((wh, un)) = rx.recv().await {
            if let Some(key) = wh {
                if let Some(row) = wh_weak.upgrade() {
                    row.set_text(&key);
                }
                if let Some(row) = nsfw_weak.upgrade() {
                    row.set_sensitive(true);
                }
            }
            if let (Some(key), Some(row)) = (un, un_weak.upgrade()) {
                row.set_text(&key);
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

    behavior_group.add(&login_service_row());

    page
}

/// Switch backed by the systemd user unit: installs/enables the headless
/// daemon at login, stops/disables it when turned off.
fn login_service_row() -> adw::SwitchRow {
    let row = adw::SwitchRow::new();
    row.set_title("Start automatically at login");
    row.set_subtitle("Runs a headless service for scheduled wallpaper refreshes");
    row.set_active(crate::systemd::is_enabled());

    let handler: std::rc::Rc<std::cell::RefCell<Option<glib::SignalHandlerId>>> =
        Default::default();
    let id = {
        let handler = handler.clone();
        row.connect_active_notify(move |row| {
            let result = if row.is_active() {
                crate::systemd::enable_start()
            } else {
                crate::systemd::stop_disable()
            };
            if let Err(e) = result {
                tracing::warn!("systemd service toggle failed: {e}");
                // Revert the switch without re-triggering this handler.
                if let Some(id) = handler.borrow().as_ref() {
                    row.block_signal(id);
                    row.set_active(!row.is_active());
                    row.unblock_signal(id);
                }
            }
        })
    };
    *handler.borrow_mut() = Some(id);
    row
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
    page.set_icon_name(Some("applications-internet-symbolic"));

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

fn unsplash_page(settings: &gio::Settings) -> (adw::PreferencesPage, adw::EntryRow) {
    let page = adw::PreferencesPage::new();
    page.set_title("Unsplash");
    page.set_icon_name(Some("applications-internet-symbolic"));

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
