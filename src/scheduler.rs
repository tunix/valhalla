use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ashpd::desktop::settings::Settings;
use gio::prelude::*;
use tokio::sync::mpsc;

use crate::{
    applier, brightness, config, credentials, http,
    source::Source,
    state::{HistoryEntry, LockedState, StateStore},
    theme::{self, ColorScheme},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Manual,
    Interval,
    ThemeChange,
    SettingsChange,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Busy,
    WallpaperSet {
        /// Candidate id — the heart toggle's like key.
        id: String,
        path: std::path::PathBuf,
        source: &'static str,
        title: Option<String>,
        author: Option<String>,
        scheme: ColorScheme,
    },
    Failed(String),
    Info(String),
    /// Not a failure, but nothing changed either: the only surviving
    /// candidate is the wallpaper that is already applied. The GUI shows a
    /// transient toast and resets the spinner; the daemon logs it.
    Notice(String),
}

/// Settings keys that should trigger a refresh when edited.
const REFRESH_KEYS: &[&str] = &[
    "active-source",
    "wallhaven-query",
    "wallhaven-categories",
    "wallhaven-purity",
    "wallhaven-sorting",
    "wallhaven-atleast",
    "unsplash-query",
    "unsplash-orientation",
    "unsplash-content-filter",
    "pexels-query",
    "pixabay-query",
    "pixabay-category",
];

pub fn is_source_key(key: &str) -> bool {
    REFRESH_KEYS.contains(&key)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// GSettings is documented thread-safe, but glib-rs objects are not `Send` by
/// default. This wrapper lets the scheduler read settings from tokio workers.
struct SendSettings(gio::Settings);

unsafe impl Send for SendSettings {}
unsafe impl Sync for SendSettings {}

impl std::ops::Deref for SendSettings {
    type Target = gio::Settings;
    fn deref(&self) -> &gio::Settings {
        &self.0
    }
}

/// Upper bound for one refresh attempt: candidate fetch, thumbnail analysis,
/// full download and apply. On expiry the refresh returns gracefully and the
/// current wallpaper is kept.
const REFRESH_BUDGET: Duration = Duration::from_secs(300);

pub struct Core {
    settings: SendSettings,
    store: StateStore,
    tx: async_channel::Sender<Msg>,
    last_settings_trigger: AtomicU64,
}

impl Core {
    pub fn new(settings: gio::Settings, tx: async_channel::Sender<Msg>) -> Self {
        Self {
            settings: SendSettings(settings),
            store: StateStore::new(),
            tx,
            last_settings_trigger: AtomicU64::new(0),
        }
    }

    fn send(&self, msg: Msg) {
        let _ = self.tx.send_blocking(msg);
    }

    /// Spawn the long-running supervision loop. Returns a sender used to feed
    /// triggers from GTK callbacks or settings watchers.
    pub fn spawn(self: Arc<Self>) -> mpsc::UnboundedSender<Trigger> {
        let (trig_tx, trig_rx) = mpsc::unbounded_channel::<Trigger>();
        crate::tokio_handle().spawn(self.clone().run(trig_rx));
        self.watch_portal_theme(trig_tx.clone());
        trig_tx
    }

    fn watch_portal_theme(&self, trig_tx: mpsc::UnboundedSender<Trigger>) {
        crate::tokio_handle().spawn(async move {
            use futures_util::StreamExt;
            let Ok(proxy) = Settings::new().await else {
                tracing::warn!("settings portal unavailable; relying on GSettings theme watch");
                return;
            };
            let Ok(stream) = proxy.receive_color_scheme_changed().await else {
                tracing::warn!("could not subscribe to portal color-scheme changes");
                return;
            };
            let mut stream = Box::pin(stream);
            while stream.next().await.is_some() {
                let _ = trig_tx.send(Trigger::ThemeChange);
            }
        });
    }

    async fn run(self: Arc<Self>, mut trig_rx: mpsc::UnboundedReceiver<Trigger>) {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = ticker.tick() => self.process(Trigger::Interval).await,
                trigger = trig_rx.recv() => match trigger {
                    Some(Trigger::SettingsChange) => {
                        let core = self.clone();
                        core.settings_changed();
                    }
                    Some(trigger) => self.process(trigger).await,
                    None => break,
                },
            }
        }
        tracing::info!("scheduler stopped");
    }

    /// Settings changed while the user types: debounce, then refresh once.
    fn settings_changed(self: Arc<Self>) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_settings_trigger.store(stamp, Ordering::SeqCst);
        crate::tokio_handle().spawn(async move {
            tokio::time::sleep(Duration::from_millis(2500)).await;
            if self.last_settings_trigger.load(Ordering::SeqCst) == stamp {
                self.process(Trigger::SettingsChange).await;
            }
        });
    }

    async fn process(&self, trigger: Trigger) {
        // Downloading a fresh wallpaper at launch is opt-in.
        if trigger == Trigger::Startup && !self.settings.boolean("refresh-on-startup") {
            tracing::debug!("startup refresh disabled; skipping");
            return;
        }

        let now = unix_now();
        let Some(mut locked) = self.store.try_lock() else {
            tracing::debug!("refresh lock held elsewhere; skipping {trigger:?}");
            return;
        };

        // Interval refreshes only run when due.
        if trigger == Trigger::Interval {
            let interval_min = self.settings.uint("refresh-interval");
            if interval_min == 0
                || now.saturating_sub(locked.get().last_refresh_at) < u64::from(interval_min) * 60
            {
                return;
            }
        }

        let scheme = theme::current().await;

        // Deduplicate theme-change storms (portal + GSettings both fire).
        if trigger == Trigger::ThemeChange {
            if !self.settings.boolean("refresh-on-theme-change") {
                return;
            }
            if let Some(prev) = &locked.get().scheme {
                if prev.as_str() == scheme.label() {
                    tracing::debug!("scheme unchanged; skipping");
                    return;
                }
            }
        }

        // Past all skip-paths: real work starts, so the UI may show the spinner.
        self.send(Msg::Busy);
        tracing::info!(?trigger, scheme = scheme.label(), "refreshing wallpaper");

        // The whole refresh is bounded; on timeout we return gracefully and
        // keep the current wallpaper.
        match tokio::time::timeout(REFRESH_BUDGET, self.refresh(&mut locked, now, scheme)).await {
            Ok(()) => {}
            Err(_) => {
                tracing::warn!(
                    "refresh timed out after {REFRESH_BUDGET:?}; keeping the current wallpaper"
                );
                self.send(Msg::Failed(
                    "Refresh timed out — keeping the current wallpaper".into(),
                ));
            }
        }
    }

    async fn refresh(&self, locked: &mut LockedState<'_>, now: u64, scheme: ColorScheme) {
        let source = Source::from_id(self.settings.string("active-source").as_str())
            .unwrap_or(Source::Wallhaven);
        let (wh_key, un_key, px_key, pb_key, nasa_key) = {
            // Keyring lookups are independent DBus round-trips: run them
            // concurrently instead of paying five serial latencies on every
            // refresh. Plain spawn_blocking (no global handle needed —
            // refresh always runs inside the tokio runtime).
            let (wh, un, px, pb, ns) = tokio::join!(
                tokio::task::spawn_blocking(|| credentials::get("wallhaven")),
                tokio::task::spawn_blocking(|| credentials::get("unsplash")),
                tokio::task::spawn_blocking(|| credentials::get("pexels")),
                tokio::task::spawn_blocking(|| credentials::get("pixabay")),
                tokio::task::spawn_blocking(|| credentials::get("nasa")),
            );
            (
                wh.ok().flatten(),
                un.ok().flatten(),
                px.ok().flatten(),
                pb.ok().flatten(),
                ns.ok().flatten(),
            )
        };
        let screen = config::screen_geometry(&self.settings);
        let wh_cfg = config::wallhaven(&self.settings, wh_key.clone(), screen);
        let un_cfg = config::unsplash(&self.settings, un_key.clone(), screen);
        let px_cfg = config::pexels(&self.settings, px_key.clone(), screen);
        let pb_cfg = config::pixabay(&self.settings, pb_key.clone(), screen);

        let mut candidates = match source
            .candidates(&wh_cfg, &un_cfg, &px_cfg, &pb_cfg, nasa_key.clone())
            .await
        {
            Ok(c) => c,
            Err(e) => {
                self.send(Msg::Failed(format!("{}: {e}", source.title())));
                return;
            }
        };
        if candidates.is_empty() {
            // Nothing at native resolution: retry once with relaxed geometry,
            // keeping the screen aspect so orientation still matches.
            if let Some(s) = screen {
                tracing::info!(
                    "no results at {}x{}; retrying with relaxed limits",
                    s.width,
                    s.height
                );
                let relaxed = s.relaxed();
                let wh_relaxed = config::wallhaven(&self.settings, wh_key, Some(relaxed));
                let un_relaxed = config::unsplash(&self.settings, un_key, Some(relaxed));
                let px_relaxed = config::pexels(&self.settings, px_key, Some(relaxed));
                let pb_relaxed = config::pixabay(&self.settings, pb_key, Some(relaxed));
                match source
                    .candidates(&wh_relaxed, &un_relaxed, &px_relaxed, &pb_relaxed, nasa_key)
                    .await
                {
                    Ok(c) if !c.is_empty() => candidates = c,
                    Ok(_) => {}
                    Err(e) => tracing::warn!("relaxed retry failed: {e}"),
                }
            }
        }
        if candidates.is_empty() {
            self.send(Msg::Failed(format!(
                "{} returned no results",
                source.title()
            )));
            return;
        }

        // Analyze thumbnails and bucket candidates by tone.
        let threshold = self.settings.double("brightness-threshold") as f32;
        let match_theme = self.settings.boolean("match-theme");
        let strict_require = self.settings.string("match-strictness").as_str() == "require";

        let mut matching: Vec<_> = Vec::new();
        let mut others: Vec<_> = Vec::new();
        for c in &candidates {
            match self.analyze(c, threshold).await {
                Ok(tone) => {
                    if !match_theme || tone == scheme {
                        matching.push(c.clone());
                    } else {
                        others.push(c.clone());
                    }
                }
                Err(e) => tracing::debug!("skipping {}: {e}", c.id),
            }
        }

        if matching.is_empty() {
            if strict_require {
                self.send(Msg::Failed(format!(
                    "No {} wallpaper matched the current filters",
                    scheme.label()
                )));
                return;
            }
            tracing::info!("no matching tone found; falling back to best effort");
            matching = others;
        }
        if matching.is_empty() {
            self.send(Msg::Failed("No usable wallpaper candidates".into()));
            return;
        }

        // A refresh must produce a visible change whenever an alternative
        // exists: the currently applied wallpaper (state history's newest
        // entry) is excluded from the theme-appropriate pool. When it is the
        // ONLY survivor — a single favorite, a single manual file, Bing's
        // single archive image — re-applying it would look like a no-op, so
        // keep it and say so instead. Theme matching stays law: the
        // exclusion only ever shrinks an already theme-matched pool.
        let current_id = locked.get().history.first().map(|e| e.id.clone());
        if let Some(current) = current_id {
            let alternatives: Vec<_> = matching
                .iter()
                .filter(|c| c.id != current)
                .cloned()
                .collect();
            if alternatives.is_empty() {
                tracing::info!("only the applied wallpaper is available; keeping it");
                // Record the refresh so interval/dedup gating does not
                // re-run this pipeline every tick for the same outcome.
                locked.get_mut().last_refresh_at = now;
                locked.save();
                self.send(Msg::Notice(
                    "Only one wallpaper is available; nothing was changed.".into(),
                ));
                return;
            }
            matching = alternatives;
        }

        // Avoid immediate repeats until the pool is exhausted.
        fastrand::shuffle(&mut matching);
        let used: Vec<String> = locked
            .get()
            .used
            .get(source.id())
            .cloned()
            .unwrap_or_default();
        let fresh: Vec<_> = matching.iter().filter(|c| !used.contains(&c.id)).collect();
        let pool: Vec<_> = if fresh.is_empty() {
            matching
        } else {
            fresh.into_iter().cloned().collect()
        };

        let mut chosen: Option<(crate::source::Candidate, std::path::PathBuf)> = None;
        for c in pool.iter().take(8) {
            // Favorites candidates point at local copies (file:// URLs) —
            // no download; the copy is applied directly.
            let path: Result<std::path::PathBuf, String> =
                if let Some(local) = c.image_url.strip_prefix("file://") {
                    Ok(std::path::PathBuf::from(local))
                } else {
                    http::download_image(&c.image_url, &http::wallpapers_dir()).await
                };
            match path {
                Ok(p) => {
                    chosen = Some((c.clone(), p));
                    break;
                }
                Err(e) => tracing::warn!("download failed for {}: {e}", c.id),
            }
        }
        let Some((cand, path)) = chosen else {
            self.send(Msg::Failed("Failed to download a wallpaper".into()));
            return;
        };

        // Images from sources without a resolution filter (or from the
        // relaxed retry) may be smaller than the screen: scale them up to
        // cover it. The pre-check reads only the image header — fully
        // decoding a wallpaper just to learn it already covers the screen
        // was the dominant cost of a local (Favorites) refresh.
        if let Some(s) = screen {
            if needs_upscale(&path, s.width, s.height) {
                if let Err(e) = crate::fit::upscale_to_cover(&path, s.width, s.height) {
                    tracing::warn!("fit check failed: {e}");
                }
            }
        }

        // Persist state before applying.
        let (light_path, dark_path) = {
            let state = locked.get_mut();
            state.last_refresh_at = now;
            state.scheme = Some(scheme.label().to_string());
            match scheme {
                ColorScheme::Light => {
                    state.current_light = Some(path.to_string_lossy().into_owned());
                }
                ColorScheme::Dark => {
                    state.current_dark = Some(path.to_string_lossy().into_owned());
                }
            }
            state.history.insert(
                0,
                HistoryEntry {
                    id: cand.id.clone(),
                    source: source.id().to_string(),
                    path: path.to_string_lossy().into_owned(),
                    title: cand.title.clone(),
                    author: cand.author.clone(),
                },
            );
            state.history.truncate(50);
            let entry = state.used.entry(source.id().to_string()).or_default();
            entry.push(cand.id.clone());
            if entry.len() > 100 {
                let excess = entry.len() - 100;
                entry.drain(0..excess);
            }
            let light = state.current_light.clone();
            let dark = state.current_dark.clone();
            (light, dark)
        };
        locked.save();

        let kind = applier::detect();
        match applier::apply(
            kind,
            light_path.as_deref().map(Path::new),
            dark_path.as_deref().map(Path::new),
            scheme,
            self.settings.boolean("apply-lockscreen"),
        )
        .await
        {
            Ok(()) => {
                tracing::info!(?kind, "wallpaper applied");
                self.send(Msg::WallpaperSet {
                    id: cand.id.clone(),
                    path,
                    source: source.id(),
                    title: cand.title.clone(),
                    author: cand.author.clone(),
                    scheme,
                });
            }
            Err(e) => self.send(Msg::Failed(format!("Applying wallpaper failed: {e}"))),
        }
    }

    async fn analyze(
        &self,
        c: &crate::source::Candidate,
        threshold: f32,
    ) -> Result<ColorScheme, String> {
        // Favorites point at local copies (file://); read directly instead
        // of routing through the HTTP cache.
        let bytes = match c.thumb_url.strip_prefix("file://") {
            Some(local) => std::fs::read(local).map_err(|e| e.to_string())?,
            None => {
                let thumb = http::download_cached(&c.thumb_url, &http::thumbs_dir()).await?;
                std::fs::read(&thumb).map_err(|e| e.to_string())?
            }
        };
        let analysis = brightness::analyze(&bytes)?;
        Ok(brightness::classify(&analysis, threshold))
    }
}

/// Header-only pre-check for `fit::upscale_to_cover`: true when the image
/// is smaller than the screen in either dimension. An unreadable header
/// keeps the old always-decode behavior. Reading the dimensions instead of
/// fully decoding the image first keeps a refresh of large local
/// wallpapers (Favorites) near-instant.
fn needs_upscale(path: &Path, screen_w: i32, screen_h: i32) -> bool {
    let dims = image::ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| e.to_string())
        .and_then(|r| r.into_dimensions().map_err(|e| e.to_string()));
    match dims {
        Ok((w, h)) => (w as i32) < screen_w || (h as i32) < screen_h,
        Err(_) => true,
    }
}

/// Headless mode: refreshes wallpapers without any UI.
pub fn run_daemon() {
    let settings = config::settings();

    // Persist the screen geometry so wallpaper selection matches the display
    // even when the GUI never ran.
    if let Some(s) = crate::screen::probe() {
        config::store_screen_geometry(&settings, s);
    }

    let (msg_tx, msg_rx) = async_channel::unbounded::<Msg>();
    crate::tokio_handle().spawn(async move {
        while let Ok(msg) = msg_rx.recv().await {
            match msg {
                Msg::WallpaperSet { path, scheme, .. } => {
                    tracing::info!("wallpaper set: {} ({})", path.display(), scheme.label());
                }
                Msg::Failed(e) => tracing::warn!("{e}"),
                Msg::Notice(text) => tracing::info!("{text}"),
                Msg::Busy | Msg::Info(_) => {}
            }
        }
    });

    let core = Arc::new(Core::new(settings.clone(), msg_tx));
    let trig = core.clone().spawn();
    let _ = trig.send(Trigger::Startup);

    // GSettings watch: filters + theme fallback (portal watch runs on tokio).
    let t1 = trig.clone();
    settings.connect_changed(None, move |_, key| {
        if is_source_key(key) {
            let _ = t1.send(Trigger::SettingsChange);
        }
    });
    let iface = gio::Settings::new("org.gnome.desktop.interface");
    let t2 = trig.clone();
    iface.connect_changed(Some("color-scheme"), move |_, _| {
        let _ = t2.send(Trigger::ThemeChange);
    });

    tracing::info!("valhalla daemon started");
    let mainloop = glib::MainLoop::new(None, false);
    mainloop.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_keys_trigger_refresh() {
        for key in REFRESH_KEYS {
            assert!(is_source_key(key), "{key} must trigger");
        }
    }

    #[test]
    fn unrelated_keys_do_not_trigger() {
        assert!(!is_source_key("brightness-threshold"));
        assert!(!is_source_key("refresh-on-startup"));
        assert!(!is_source_key(""));
        assert!(!is_source_key("wallhaven-query-x"));
    }

    #[test]
    fn triggers_are_distinct_and_debuggable() {
        // The enum is used as a tracing field and compared for equality.
        let all = [
            Trigger::Startup,
            Trigger::Manual,
            Trigger::Interval,
            Trigger::ThemeChange,
            Trigger::SettingsChange,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
            assert!(!format!("{a:?}").is_empty());
        }
    }

    #[test]
    fn msg_variants_debug() {
        let msgs = [
            Msg::Busy,
            Msg::WallpaperSet {
                id: "w1".into(),
                path: "/w.jpg".into(),
                source: "wallhaven",
                title: Some("T".into()),
                author: None,
                scheme: ColorScheme::Dark,
            },
            Msg::Failed("boom".into()),
            Msg::Info("info".into()),
            Msg::Notice("notice".into()),
        ];
        for m in &msgs {
            assert!(!format!("{m:?}").is_empty());
        }
        // WallpaperSet carries the full metadata for the window caption.
        if let Msg::WallpaperSet {
            id,
            path,
            source,
            title,
            author,
            scheme,
        } = &msgs[1]
        {
            assert_eq!(id, "w1");
            assert_eq!(path, &std::path::PathBuf::from("/w.jpg"));
            assert_eq!(*source, "wallhaven");
            assert_eq!(title.as_deref(), Some("T"));
            assert!(author.is_none());
            assert_eq!(*scheme, ColorScheme::Dark);
        }
    }

    /// Interval gating math: an interval refresh is due only when the stored
    /// last_refresh_at is older than interval_min minutes. Extracted here as
    /// the exact comparison process() performs.
    #[test]
    fn interval_gate_math() {
        let now: u64 = 10 * 3600;
        let last: u64 = 9 * 3600; // 1h ago

        let due = |interval_min: u64| -> bool {
            interval_min != 0 && now.saturating_sub(last) >= interval_min * 60
        };
        assert!(due(60), "1h elapsed, 60m interval -> due");
        assert!(!due(120), "1h elapsed, 120m interval -> not due");
        assert!(!due(0), "interval disabled -> never due");
        // last in the future (clock skew) must not panic.
        let future_last: u64 = 11 * 3600;
        assert!((now.saturating_sub(future_last)) == 0);
    }

    /// Theme-change dedup: a refresh runs only when the scheme actually
    /// changed since the last refresh.
    #[test]
    fn theme_change_dedup_math() {
        let should_run = |refresh_enabled: bool, prev: Option<&str>, cur: &str| {
            refresh_enabled && prev.map(|p| p != cur).unwrap_or(true)
        };
        assert!(!should_run(false, None, "dark"), "feature off -> skip");
        assert!(!should_run(true, Some("dark"), "dark"), "unchanged -> skip");
        assert!(should_run(true, Some("light"), "dark"), "changed -> run");
        assert!(should_run(true, None, "dark"), "first ever -> run");
    }

    /// The refresh budget is a compile-time constant; pin it so an accidental
    /// change is a reviewable diff.
    #[test]
    fn refresh_budget_is_five_minutes() {
        assert_eq!(REFRESH_BUDGET, Duration::from_secs(300));
    }

    /// Used-pool trimming: at most 100 ids per source are remembered.
    #[test]
    fn used_pool_trim_math() {
        let mut entry: Vec<String> = (0..103).map(|i| format!("id{i}")).collect();
        if entry.len() > 100 {
            let excess = entry.len() - 100;
            entry.drain(0..excess);
        }
        assert_eq!(entry.len(), 100);
        assert_eq!(entry[0], "id3"); // oldest trimmed
        assert_eq!(entry[99], "id102");
    }

    /// History cap: at most 50 entries, newest first.
    #[test]
    fn history_cap_math() {
        let mut history: Vec<HistoryEntry> = (0..60)
            .map(|i| HistoryEntry {
                id: format!("h{i}"),
                source: "s".into(),
                path: "/p".into(),
                title: None,
                author: None,
            })
            .collect();
        history.truncate(50);
        assert_eq!(history.len(), 50);
    }

    /// file:// candidates (Favorites, Manual) must resolve to local reads:
    /// the analysis step reads the file directly and never touches the
    /// HTTP client, and a missing file degrades to a skippable error.
    #[test]
    fn analyze_reads_file_candidates_locally() {
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        let (tx, rx) = async_channel::unbounded();
        let core = Core::new(gio::Settings::new("io.github.tunix.valhalla"), tx);

        let dir = std::env::temp_dir().join(format!("valhalla-sched-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let img = dir.join("dark.jpg");
        let mut px = image::RgbImage::new(32, 32);
        for p in px.pixels_mut() {
            *p = image::Rgb([10, 10, 14]);
        }
        image::DynamicImage::ImageRgb8(px)
            .write_to(
                &mut std::io::BufWriter::new(std::fs::File::create(&img).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();

        let candidate = |thumb: String| crate::source::Candidate {
            id: "f1".into(),
            source: "favorites",
            image_url: thumb.clone(),
            thumb_url: thumb,
            web_url: String::new(),
            title: None,
            author: None,
            dominant_colors: Vec::new(),
        };

        // analyze() is async by signature only (no awaits inside); drive it
        // on a small runtime like applier's portal guard test does, so the
        // env lock is never held across an await.
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let scheme = rt
            .block_on(core.analyze(&candidate(format!("file://{}", img.display())), 0.45))
            .expect("local analysis");
        assert_eq!(scheme, ColorScheme::Dark);

        // Missing file: graceful error the caller skips the candidate for.
        assert!(rt
            .block_on(core.analyze(
                &candidate("file:///nonexistent/valhalla-missing.jpg".into()),
                0.45
            ))
            .is_err());

        // Local analysis must not emit any scheduler messages and must not
        // leave anything in the HTTP cache dirs.
        assert!(rx.try_recv().is_err(), "no messages expected");
        std::fs::remove_dir_all(&dir).ok();
        std::env::remove_var("GSETTINGS_BACKEND");
    }

    /// Build a small solid-color JPEG for the favorites pipeline tests.
    fn solid_jpeg(dir: &std::path::Path, name: &str, color: [u8; 3]) -> std::path::PathBuf {
        let mut px = image::RgbImage::new(32, 32);
        for p in px.pixels_mut() {
            *p = image::Rgb(color);
        }
        let path = dir.join(name);
        image::DynamicImage::ImageRgb8(px)
            .write_to(
                &mut std::io::BufWriter::new(std::fs::File::create(&path).unwrap()),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        path
    }

    /// Full-pipeline environment for refresh() against the Favorites source:
    /// temp XDG_DATA_HOME (favorites list), temp XDG_STATE_HOME (state
    /// store) and the memory GSettings backend — all process-global, so the
    /// caller must hold ENV_LOCK for the whole test. Every knob the refresh
    /// pipeline reads is pinned explicitly; the memory backend is shared
    /// between tests, so nothing may rely on defaults.
    fn favorites_core(tag: &str) -> (Core, async_channel::Receiver<Msg>, std::path::PathBuf) {
        std::env::set_var("GSETTINGS_BACKEND", "memory");
        let tmp = std::env::temp_dir().join(format!("valhalla-sched-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_DATA_HOME", tmp.join("data"));
        std::env::set_var("XDG_STATE_HOME", tmp.join("state"));

        let (tx, rx) = async_channel::unbounded();
        let core = Core::new(gio::Settings::new("io.github.tunix.valhalla"), tx);
        let s = gio::Settings::new("io.github.tunix.valhalla");
        let _ = s.set_string("active-source", "favorites");
        // Tiny screen: the fit check exercises without big decodes.
        let _ = s.set_string("screen-geometry", "128x128");
        let _ = s.set_boolean("match-theme", true);
        let _ = s.set_string("match-strictness", "require");
        let _ = s.set_double("brightness-threshold", 0.45);
        (core, rx, tmp)
    }

    fn favorites_teardown(core: &Core, tmp: &std::path::Path) {
        // Restore the shared memory-backend keys before dropping the backend.
        let _ = core.settings.0.set_string("active-source", "wallhaven");
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_STATE_HOME");
        std::env::remove_var("GSETTINGS_BACKEND");
        std::fs::remove_dir_all(tmp).ok();
    }

    /// Mark `id` as the currently applied wallpaper (state history's newest
    /// entry — what the exclusion logic reads).
    fn seed_current(core: &Core, id: &str, path: &std::path::Path) {
        let mut locked = core.store.try_lock().unwrap();
        locked.get_mut().history.insert(
            0,
            HistoryEntry {
                id: id.into(),
                source: "favorites".into(),
                path: path.to_string_lossy().into_owned(),
                title: None,
                author: None,
            },
        );
        locked.save();
    }

    fn drain(rx: &async_channel::Receiver<Msg>) -> Vec<Msg> {
        let mut msgs = Vec::new();
        while let Ok(m) = rx.try_recv() {
            msgs.push(m);
        }
        msgs
    }

    /// Drive one full refresh() on a current-thread runtime. refresh() is
    /// awaited inside block_on, so ENV_LOCK is never held across an await.
    fn run_refresh(core: &Core, now: u64, scheme: ColorScheme) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let mut locked = core.store.try_lock().unwrap();
        rt.block_on(core.refresh(&mut locked, now, scheme));
    }

    /// (a) The currently applied wallpaper is excluded from selection
    /// whenever another candidate survives theme matching: a refresh must
    /// change something visible, not silently re-apply the same file.
    #[test]
    fn refresh_excludes_applied_wallpaper_when_alternatives_exist() {
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (core, rx, tmp) = favorites_core("excl");
        let dark = solid_jpeg(&tmp, "dark.jpg", [10, 10, 14]);
        crate::favorites::add("fA", "wallhaven", &dark, None, None, None).unwrap();
        crate::favorites::add("fB", "wallhaven", &dark, None, None, None).unwrap();
        seed_current(&core, "fA", &dark);

        run_refresh(&core, 1_000, ColorScheme::Dark);

        let msgs = drain(&rx);
        assert_eq!(msgs.len(), 1, "exactly one WallpaperSet expected: {msgs:?}");
        match &msgs[0] {
            Msg::WallpaperSet {
                id, source, scheme, ..
            } => {
                assert_eq!(id, "fB", "the applied wallpaper must be excluded");
                assert_eq!(*source, "favorites");
                assert_eq!(*scheme, ColorScheme::Dark);
            }
            other => panic!("expected WallpaperSet, got {other:?}"),
        }
        assert_eq!(core.store.read().history[0].id, "fB");
        favorites_teardown(&core, &tmp);
    }

    /// (b) When the only theme-matching candidate IS the applied wallpaper
    /// (a single favorite here), the refresh must not re-apply it: a Notice
    /// is emitted, the history is untouched, and the refresh counts as done
    /// for interval gating.
    #[test]
    fn refresh_single_survivor_notices_without_reapplying() {
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (core, rx, tmp) = favorites_core("single");
        let dark = solid_jpeg(&tmp, "dark.jpg", [10, 10, 14]);
        crate::favorites::add("fA", "wallhaven", &dark, None, None, None).unwrap();
        seed_current(&core, "fA", &dark);

        run_refresh(&core, 2_000, ColorScheme::Dark);

        let msgs = drain(&rx);
        assert_eq!(msgs.len(), 1, "only a Notice expected: {msgs:?}");
        match &msgs[0] {
            Msg::Notice(text) => assert!(
                text.contains("Only one wallpaper"),
                "notice must say nothing changed: {text}"
            ),
            other => panic!("expected Notice, got {other:?}"),
        }
        let state = core.store.read();
        assert_eq!(state.history[0].id, "fA", "wallpaper unchanged");
        assert_eq!(state.last_refresh_at, 2_000, "refresh recorded");
        favorites_teardown(&core, &tmp);
    }

    /// (c) No applied wallpaper yet (empty history): the single candidate is
    /// applied normally — the exclusion only guards repeats.
    #[test]
    fn refresh_without_current_applies_single_candidate() {
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (core, rx, tmp) = favorites_core("first");
        let dark = solid_jpeg(&tmp, "dark.jpg", [10, 10, 14]);
        crate::favorites::add("fA", "wallhaven", &dark, None, None, None).unwrap();

        run_refresh(&core, 3_000, ColorScheme::Dark);

        let msgs = drain(&rx);
        assert_eq!(msgs.len(), 1, "exactly one WallpaperSet expected: {msgs:?}");
        match &msgs[0] {
            Msg::WallpaperSet { id, .. } => assert_eq!(id, "fA"),
            other => panic!("expected WallpaperSet, got {other:?}"),
        }
        assert_eq!(core.store.read().history[0].id, "fA");
        favorites_teardown(&core, &tmp);
    }

    /// (d) Theme matching stays law: a light favorite is NOT used to dodge
    /// the exclusion in dark mode. If excluding the current id would empty
    /// the theme-matched pool, the no-change + notice path wins.
    #[test]
    fn refresh_exclusion_never_violates_theme_matching() {
        let _env = crate::http::test_env::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (core, rx, tmp) = favorites_core("themelaw");
        let dark = solid_jpeg(&tmp, "dark.jpg", [10, 10, 14]);
        let light = solid_jpeg(&tmp, "light.jpg", [240, 240, 240]);
        crate::favorites::add("fA", "wallhaven", &dark, None, None, None).unwrap();
        crate::favorites::add("fB", "wallhaven", &light, None, None, None).unwrap();
        seed_current(&core, "fA", &dark);

        run_refresh(&core, 4_000, ColorScheme::Dark);

        let msgs = drain(&rx);
        assert_eq!(msgs.len(), 1, "only a Notice expected: {msgs:?}");
        match &msgs[0] {
            Msg::Notice(text) => assert!(
                text.contains("Only one wallpaper"),
                "theme pool had only the applied one: {text}"
            ),
            other => panic!("expected Notice, got {other:?}"),
        }
        assert_eq!(core.store.read().history[0].id, "fA", "wallpaper unchanged");
        favorites_teardown(&core, &tmp);
    }
}
