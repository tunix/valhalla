use std::sync::OnceLock;

use gtk::{gdk, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenInfo {
    pub width: i32,
    pub height: i32,
}

impl ScreenInfo {
    pub fn aspect(self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }

    /// Halved geometry used as a fallback when no wallpaper matches the
    /// native resolution.
    pub fn relaxed(self) -> ScreenInfo {
        ScreenInfo {
            width: (self.width / 2).max(1280),
            height: (self.height / 2).max(720),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Landscape,
    Portrait,
    Square,
}

pub fn orientation_of(aspect: f32) -> Orientation {
    if aspect > 1.1 {
        Orientation::Landscape
    } else if aspect < 0.909 {
        Orientation::Portrait
    } else {
        Orientation::Square
    }
}

pub fn unsplash_orientation(screen: ScreenInfo) -> &'static str {
    match orientation_of(screen.aspect()) {
        Orientation::Landscape => "landscape",
        Orientation::Portrait => "portrait",
        Orientation::Square => "squarish",
    }
}

/// Wallhaven's supported aspect-ratio families.
const WALLHAVEN_RATIOS: &[(&str, f32)] = &[
    ("48x9", 48.0 / 9.0),
    ("32x9", 32.0 / 9.0),
    ("21x9", 21.0 / 9.0),
    ("16x9", 16.0 / 9.0),
    ("16x10", 16.0 / 10.0),
    ("3x2", 3.0 / 2.0),
    ("4x3", 4.0 / 3.0),
    ("5x4", 5.0 / 4.0),
    ("1x1", 1.0),
    ("10x16", 10.0 / 16.0),
    ("9x16", 9.0 / 16.0),
    ("9x18", 9.0 / 18.0),
];

/// The two Wallhaven ratio families closest to the screen aspect.
pub fn wallhaven_ratios(screen: ScreenInfo) -> Vec<&'static str> {
    let target = screen.aspect();
    let mut scored: Vec<(f32, &'static str)> = WALLHAVEN_RATIOS
        .iter()
        .map(|(name, ratio)| ((target / ratio).ln().abs(), *name))
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(2);
    scored.into_iter().map(|(_, name)| name).collect()
}

static SCREEN: OnceLock<ScreenInfo> = OnceLock::new();

/// Query the largest connected monitor. Must be called from the main thread;
/// initializes GDK when no GTK stack is up yet (daemon mode).
pub fn probe() -> Option<ScreenInfo> {
    if let Some(s) = SCREEN.get() {
        return Some(*s);
    }
    if !gtk::is_initialized() {
        gtk::init().ok()?;
    }
    let display = gdk::Display::default()?;
    let monitors = display.monitors();
    let mut best: Option<(i64, i32, i32)> = None;
    for i in 0..monitors.n_items() {
        let Some(monitor) = monitors
            .item(i)
            .and_then(|obj| obj.downcast::<gdk::Monitor>().ok())
        else {
            continue;
        };
        let geometry = monitor.geometry();
        let area = geometry.width() as i64 * geometry.height() as i64;
        if best.is_none_or(|(a, _, _)| area > a) {
            best = Some((area, geometry.width(), geometry.height()));
        }
    }
    let (_, width, height) = best?;
    if width <= 0 || height <= 0 {
        return None;
    }
    let info = ScreenInfo { width, height };
    let _ = SCREEN.set(info);
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(w: i32, h: i32) -> ScreenInfo {
        ScreenInfo {
            width: w,
            height: h,
        }
    }

    #[test]
    fn orientations() {
        assert_eq!(orientation_of(16.0 / 9.0), Orientation::Landscape);
        assert_eq!(orientation_of(9.0 / 16.0), Orientation::Portrait);
        assert_eq!(orientation_of(1.0), Orientation::Square);
    }

    #[test]
    fn landscape_ratios() {
        let ratios = wallhaven_ratios(screen(2560, 1440));
        assert_eq!(ratios[0], "16x9");
        assert!(!ratios[1].starts_with('9'));
    }

    #[test]
    fn ultrawide_ratios() {
        let ratios = wallhaven_ratios(screen(3440, 1440));
        assert_eq!(ratios[0], "21x9");
    }

    #[test]
    fn portrait_ratios() {
        let ratios = wallhaven_ratios(screen(1080, 1920));
        assert_eq!(ratios[0], "9x16");
        assert_eq!(
            orientation_of(screen(1080, 1920).aspect()),
            Orientation::Portrait
        );
        assert_eq!(unsplash_orientation(screen(1080, 1920)), "portrait");
    }

    #[test]
    fn unsplash_orientation_map() {
        assert_eq!(unsplash_orientation(screen(3840, 2160)), "landscape");
        assert_eq!(unsplash_orientation(screen(1000, 1000)), "squarish");
    }

    #[test]
    fn relaxed_geometry() {
        assert_eq!(screen(3840, 2160).relaxed(), screen(1920, 1080));
        assert_eq!(screen(1000, 800).relaxed(), screen(1280, 720));
    }
}
