//! Subtle prompt band for the terminal theme.
//!
//! The band is a short step from the background the terminal reported toward its foreground.
//! OSC 11 supplies the background. `COLORFGBG` supplies ANSI indexes when that probe did not.
//! A missing report uses a black or white canvas from the known polarity.

use std::sync::Mutex;

use ratatui::style::Color;

use super::system_appearance::SystemAppearance;

const MIX_TOWARD_FG: f32 = 0.12;
const MIN_STEP: i16 = 18;

const ANSI16: [[u8; 3]; 16] = [
    [0, 0, 0],
    [205, 0, 0],
    [0, 205, 0],
    [205, 205, 0],
    [0, 0, 238],
    [205, 0, 205],
    [0, 205, 205],
    [229, 229, 229],
    [127, 127, 127],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [92, 92, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

/// Foreground and background a terminal reported, as 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReportedTerminalColors {
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
}

static OVERRIDE: Mutex<Option<ReportedTerminalColors>> = Mutex::new(None);
static OSC11_BG: Mutex<Option<(u8, u8, u8)>> = Mutex::new(None);

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Remember an OSC 11 background. Later probes replace it.
pub(crate) fn record_reported_background(rgb: (u8, u8, u8)) {
    *lock(&OSC11_BG) = Some(rgb);
}

/// The colors the band is mixed from. A test override wins over the live probe and `COLORFGBG`.
pub fn reported_terminal_colors() -> ReportedTerminalColors {
    if let Some(forced) = *lock(&OVERRIDE) {
        return forced;
    }
    let mut colors = ReportedTerminalColors {
        fg: None,
        bg: *lock(&OSC11_BG),
    };
    if let Some((fg, bg)) = colorfgbg_rgb() {
        if colors.fg.is_none() {
            colors.fg = fg;
        }
        if colors.bg.is_none() {
            colors.bg = bg;
        }
    }
    colors
}

/// Install reported colors for the rest of this guard. Dropping it returns to the live probe.
#[cfg(any(test, feature = "test-support"))]
pub struct ReportedColorsGuard;

#[cfg(any(test, feature = "test-support"))]
impl ReportedColorsGuard {
    pub fn force(fg: Option<(u8, u8, u8)>, bg: Option<(u8, u8, u8)>) -> Self {
        *lock(&OVERRIDE) = Some(ReportedTerminalColors { fg, bg });
        Self
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Drop for ReportedColorsGuard {
    fn drop(&mut self) {
        *lock(&OVERRIDE) = None;
    }
}

/// One step of `bg` toward `fg`. Equal channels still move by [`MIN_STEP`].
pub fn subtle_band_rgb(bg: (u8, u8, u8), fg: (u8, u8, u8)) -> (u8, u8, u8) {
    let channel = |base: u8, toward: u8| -> u8 {
        let delta = toward as i16 - base as i16;
        let mut step = (delta as f32 * MIX_TOWARD_FG).round() as i16;
        if step == 0 {
            step = if delta > 0 {
                MIN_STEP
            } else if delta < 0 {
                -MIN_STEP
            } else if base < 128 {
                MIN_STEP
            } else {
                -MIN_STEP
            };
        } else if step.abs() < MIN_STEP {
            step = MIN_STEP * step.signum();
        }
        (base as i16 + step).clamp(0, 255) as u8
    };
    (
        channel(bg.0, fg.0),
        channel(bg.1, fg.1),
        channel(bg.2, fg.2),
    )
}

/// Fill for a terminal-theme prompt row. Quantized to the terminal's color level.
pub fn terminal_prompt_band() -> Color {
    let colors = reported_terminal_colors();
    let (bg, fg) = band_endpoints(&colors);
    let (r, g, b) = subtle_band_rgb(bg, fg);
    super::color_support::quantize(Color::Rgb(r, g, b))
}

fn band_endpoints(colors: &ReportedTerminalColors) -> ((u8, u8, u8), (u8, u8, u8)) {
    let dark = match colors.bg {
        Some(bg) => is_dark(bg),
        None => {
            super::system_appearance::detect().unwrap_or(SystemAppearance::Dark)
                == SystemAppearance::Dark
        }
    };
    let bg = colors
        .bg
        .unwrap_or(if dark { (0, 0, 0) } else { (255, 255, 255) });
    let fg = colors
        .fg
        .unwrap_or(if dark { (255, 255, 255) } else { (0, 0, 0) });
    (bg, fg)
}

fn is_dark(rgb: (u8, u8, u8)) -> bool {
    super::osc11::classify_luminance(rgb.0, rgb.1, rgb.2) == SystemAppearance::Dark
}

fn colorfgbg_rgb() -> Option<(Option<(u8, u8, u8)>, Option<(u8, u8, u8)>)> {
    let raw = std::env::var("COLORFGBG").ok()?;
    let mut fields = raw.split(';').map(str::trim).filter(|field| !field.is_empty());
    let fg = fields.next().and_then(ansi_index).and_then(ansi16_rgb);
    let bg = raw
        .split(';')
        .next_back()
        .map(str::trim)
        .and_then(ansi_index)
        .and_then(ansi16_rgb);
    if fg.is_none() && bg.is_none() {
        None
    } else {
        Some((fg, bg))
    }
}

fn ansi_index(field: &str) -> Option<u8> {
    let index = field.parse::<u8>().ok()?;
    (index < 16).then_some(index)
}

fn ansi16_rgb(index: u8) -> Option<(u8, u8, u8)> {
    ANSI16
        .get(index as usize)
        .map(|rgb| (rgb[0], rgb[1], rgb[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
        let d = |x: u8, y: u8| (x as i32 - y as i32).unsigned_abs();
        d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
    }

    #[test]
    fn subtle_band_stays_closer_to_the_reported_background() {
        let bg = (0x1a, 0x1b, 0x26);
        let fg = (0xee, 0xee, 0xee);
        let band = subtle_band_rgb(bg, fg);
        assert_ne!(band, bg);
        assert!(dist(band, bg) < dist(band, fg));

        let light_bg = (240, 240, 240);
        let light_fg = (20, 20, 20);
        let light = subtle_band_rgb(light_bg, light_fg);
        assert!(dist(light, light_bg) < dist(light, light_fg));
        assert!(light.0 < light_bg.0);
    }

    #[test]
    fn terminal_prompt_band_uses_the_forced_report() {
        let _theme = crate::theme::cache::pin_theme();
        let _guard = ReportedColorsGuard::force(Some((238, 238, 238)), Some((26, 27, 38)));
        let Color::Rgb(r, g, b) = terminal_prompt_band() else {
            panic!("truecolor keeps the mixed rgb");
        };
        assert_eq!((r, g, b), subtle_band_rgb((26, 27, 38), (238, 238, 238)));
    }
}
