//! FFmpeg PATH probes and the "install ffmpeg" banner for inline video posters.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::prompt_images::InlineMediaInfo;

/// How long a "not on PATH" answer is reused. A hit is kept for the process.
/// Re-probing every frame walked slow PATH entries (WSL Windows drives) and stalled typing.
const FFMPEG_PROBE_RETRY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct TimedProbe<T> {
    value: T,
    checked_at: Option<Instant>,
}

static FFMPEG_PROBE: Mutex<TimedProbe<bool>> = Mutex::new(TimedProbe {
    value: false,
    checked_at: None,
});

static INSTALL_PROBE: Mutex<TimedProbe<Option<&'static str>>> = Mutex::new(TimedProbe {
    value: None,
    checked_at: None,
});

#[cfg(test)]
static PATH_LOOKUPS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn lock_probe<T>(slot: &Mutex<TimedProbe<T>>) -> std::sync::MutexGuard<'_, TimedProbe<T>> {
    slot.lock().unwrap_or_else(|err| err.into_inner())
}

fn read_cached<T: Copy>(slot: &Mutex<TimedProbe<T>>, settled: impl Fn(T) -> bool) -> Option<T> {
    let guard = lock_probe(slot);
    if settled(guard.value) {
        return Some(guard.value);
    }
    let fresh = guard.checked_at.is_some_and(|at| {
        Instant::now().saturating_duration_since(at) < FFMPEG_PROBE_RETRY
    });
    fresh.then_some(guard.value)
}

fn store_probe<T: Copy>(slot: &Mutex<TimedProbe<T>>, value: T, settled: impl Fn(T) -> bool) {
    let mut guard = lock_probe(slot);
    if settled(guard.value) && !settled(value) {
        return;
    }
    guard.value = value;
    guard.checked_at = Some(Instant::now());
}

fn command_on_path(name: &str) -> bool {
    #[cfg(test)]
    PATH_LOOKUPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    xai_grok_config::shell::is_command_on_local_path(name)
}

pub const FFMPEG_HINT_TEXT: &str = "Install ffmpeg to view inline";

/// Inline playback needs both a graphics protocol and ffmpeg.
/// Enter on a video card must not start a decode when either is missing.
pub fn inline_video_playback_available() -> bool {
    crate::terminal::image::detect_graphics_protocol().supports_images() && ffmpeg_available()
}

/// Whether ffmpeg is on a local PATH entry.
///
/// A hit is cached for the process. A miss is reused for [`FFMPEG_PROBE_RETRY`] so typing does not stat PATH on every frame, then probed again so a mid-session install still replaces the banner.
pub fn ffmpeg_available() -> bool {
    #[cfg(test)]
    if let Some(v) = TEST_FFMPEG_OVERRIDE.with(|c| c.get()) {
        return v;
    }

    if let Some(cached) = read_cached(&FFMPEG_PROBE, |found| found) {
        return cached;
    }
    let available = command_on_path("ffmpeg");
    store_probe(&FFMPEG_PROBE, available, |found| found);
    read_cached(&FFMPEG_PROBE, |found| found).unwrap_or(available)
}

#[cfg(test)]
thread_local! {
    static TEST_FFMPEG_OVERRIDE: std::cell::Cell<Option<bool>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_ffmpeg_available_for_test(available: bool) -> FfmpegTestGuard {
    TEST_FFMPEG_OVERRIDE.with(|c| c.set(Some(available)));
    FfmpegTestGuard
}

#[cfg(test)]
pub(crate) struct FfmpegTestGuard;

#[cfg(test)]
impl Drop for FfmpegTestGuard {
    fn drop(&mut self) {
        TEST_FFMPEG_OVERRIDE.with(|c| c.set(None));
    }
}

/// The first package manager found on PATH wins; when none is found the banner shows only the hint, no install line.
fn ffmpeg_install_candidates() -> &'static [(&'static str, &'static str)] {
    if cfg!(target_os = "macos") {
        &[("brew", "! brew install ffmpeg")]
    } else if cfg!(target_os = "windows") {
        &[
            ("winget", "! winget install ffmpeg"),
            ("choco", "! choco install ffmpeg"),
            ("scoop", "! scoop install ffmpeg"),
        ]
    } else {
        &[
            ("apt", "! sudo apt install ffmpeg"),
            ("apt-get", "! sudo apt-get install ffmpeg"),
            ("dnf", "! sudo dnf install ffmpeg"),
            ("pacman", "! sudo pacman -S ffmpeg"),
            ("zypper", "! sudo zypper install ffmpeg"),
            ("apk", "! sudo apk add ffmpeg"),
        ]
    }
}

/// Install hint for the first package manager on the local PATH.
///
/// A hit is cached for the process. A miss is reused for [`FFMPEG_PROBE_RETRY`], same as [`ffmpeg_available`], because the banner is measured on the frame path.
pub fn ffmpeg_install_cmd() -> Option<&'static str> {
    #[cfg(test)]
    if let Some(v) = TEST_FFMPEG_INSTALL_CMD_OVERRIDE.with(|c| c.get()) {
        return v;
    }

    if let Some(cached) = read_cached(&INSTALL_PROBE, |cmd| cmd.is_some()) {
        return cached;
    }
    let mut found = None;
    for (manager, cmd) in ffmpeg_install_candidates() {
        if command_on_path(manager) {
            found = Some(*cmd);
            break;
        }
    }
    store_probe(&INSTALL_PROBE, found, |cmd| cmd.is_some());
    read_cached(&INSTALL_PROBE, |cmd| cmd.is_some()).unwrap_or(found)
}

#[cfg(test)]
fn reset_ffmpeg_path_probe_for_test() {
    PATH_LOOKUPS.store(0, std::sync::atomic::Ordering::Relaxed);
    *lock_probe(&FFMPEG_PROBE) = TimedProbe {
        value: false,
        checked_at: None,
    };
    *lock_probe(&INSTALL_PROBE) = TimedProbe {
        value: None,
        checked_at: None,
    };
}

#[cfg(test)]
fn expire_ffmpeg_path_probe_for_test() {
    let stale = Instant::now().checked_sub(FFMPEG_PROBE_RETRY + Duration::from_secs(1));
    lock_probe(&FFMPEG_PROBE).checked_at = stale;
    lock_probe(&INSTALL_PROBE).checked_at = stale;
}

#[cfg(test)]
fn path_lookups_for_test() -> u32 {
    PATH_LOOKUPS.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
thread_local! {
    static TEST_FFMPEG_INSTALL_CMD_OVERRIDE: std::cell::Cell<Option<Option<&'static str>>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_ffmpeg_install_cmd_for_test(
    cmd: Option<&'static str>,
) -> FfmpegInstallCmdTestGuard {
    TEST_FFMPEG_INSTALL_CMD_OVERRIDE.with(|c| c.set(Some(cmd)));
    FfmpegInstallCmdTestGuard
}

#[cfg(test)]
pub(crate) struct FfmpegInstallCmdTestGuard;

#[cfg(test)]
impl Drop for FfmpegInstallCmdTestGuard {
    fn drop(&mut self) {
        TEST_FFMPEG_INSTALL_CMD_OVERRIDE.with(|c| c.set(None));
    }
}

fn ffmpeg_hint_banner_rows() -> u16 {
    if ffmpeg_install_cmd().is_some() { 2 } else { 1 }
}

/// `(image_area, total)` rows for an inline-media preview.
/// Shared by the code that reserves entry height and the code that places the block, so the two cannot drift.
pub fn inline_media_reserved_rows(info: &InlineMediaInfo, content_width: u16) -> (u16, u16) {
    use crate::terminal::image::fit_image_to_cells;

    if info.is_video && !ffmpeg_available() {
        let banner_rows = ffmpeg_hint_banner_rows();
        return (banner_rows, banner_rows + 1);
    }
    let max_cols = content_width.saturating_sub(2);
    let max_rows = (content_width / 2).clamp(4, 20);
    let (_cols, rows) = fit_image_to_cells(info.width, info.height, max_cols, max_rows);
    (rows, rows + 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_install_candidates_are_runnable_prompt_hints() {
        for (manager, cmd) in ffmpeg_install_candidates() {
            assert!(!manager.is_empty(), "empty manager name");
            assert!(cmd.starts_with("! "), "not prompt-runnable: {cmd:?}");
            assert!(cmd.contains("ffmpeg"), "missing package: {cmd:?}");
        }
    }

    #[test]
    fn ffmpeg_install_cmd_override_none_omits_command() {
        let _no_pm = set_ffmpeg_install_cmd_for_test(None);
        assert_eq!(ffmpeg_install_cmd(), None);
    }

    #[test]
    fn ffmpeg_install_cmd_override_some_returns_command() {
        let _pm = set_ffmpeg_install_cmd_for_test(Some("! brew install ffmpeg"));
        assert_eq!(ffmpeg_install_cmd(), Some("! brew install ffmpeg"));
    }

    fn media(is_video: bool) -> InlineMediaInfo {
        InlineMediaInfo {
            path: std::path::PathBuf::from("/tmp/x"),
            width: 640,
            height: 480,
            is_video,
            alt_text: String::new(),
        }
    }

    #[test]
    fn reserved_rows_image_is_full_poster_plus_button() {
        let _no_ffmpeg = set_ffmpeg_available_for_test(false);
        let (image_rows, total_rows) = inline_media_reserved_rows(&media(false), 80);
        assert!(
            (4..=20).contains(&image_rows),
            "poster budget: {image_rows}"
        );
        assert_eq!(total_rows, image_rows + 3);
    }

    #[test]
    fn reserved_rows_video_shrinks_to_two_line_banner_with_install_cmd() {
        let _no_ffmpeg = set_ffmpeg_available_for_test(false);
        let _cmd = set_ffmpeg_install_cmd_for_test(Some("! brew install ffmpeg"));
        assert_eq!(inline_media_reserved_rows(&media(true), 80), (2, 3));
    }

    #[test]
    fn reserved_rows_video_shrinks_to_one_line_banner_without_install_cmd() {
        let _no_ffmpeg = set_ffmpeg_available_for_test(false);
        let _no_pm = set_ffmpeg_install_cmd_for_test(None);
        assert_eq!(inline_media_reserved_rows(&media(true), 80), (1, 2));
    }

    #[test]
    fn inline_playback_needs_graphics_and_ffmpeg() {
        use crate::terminal::image::{GraphicsProtocol, set_protocol_for_test};

        {
            let _none = set_protocol_for_test(GraphicsProtocol::None);
            let _ffmpeg = set_ffmpeg_available_for_test(true);
            assert!(!inline_video_playback_available());
        }
        {
            let _kitty = set_protocol_for_test(GraphicsProtocol::Kitty);
            let _no_ffmpeg = set_ffmpeg_available_for_test(false);
            assert!(!inline_video_playback_available());
        }
        {
            let _kitty = set_protocol_for_test(GraphicsProtocol::Kitty);
            let _ffmpeg = set_ffmpeg_available_for_test(true);
            assert!(inline_video_playback_available());
        }
    }

    #[test]
    fn reserved_rows_video_is_full_poster_with_ffmpeg() {
        let _ffmpeg = set_ffmpeg_available_for_test(true);
        let (image_rows, total_rows) = inline_media_reserved_rows(&media(true), 80);
        assert!(
            (4..=20).contains(&image_rows),
            "poster budget: {image_rows}"
        );
        assert_eq!(total_rows, image_rows + 3);
    }

    /// Each frame lays out the scrollback. With ffmpeg missing, that used to stat every PATH entry, including WSL
    /// Windows drives. The second frame must not walk PATH. A later frame, after the retry interval, looks again.
    #[test]
    #[serial_test::serial]
    fn missing_ffmpeg_layout_does_not_walk_path_every_frame() {
        let fast = tempfile::tempdir().expect("temp dir");
        let path = format!("/mnt/c/Windows/System32:{}", fast.path().display());
        let _env = xai_grok_test_support::EnvGuard::set("PATH", &path);
        reset_ffmpeg_path_probe_for_test();

        let searched: Vec<_> = std::env::split_paths(
            &xai_grok_config::shell::local_command_search_path(),
        )
        .collect();
        assert!(
            searched.iter().all(|dir| !dir.starts_with("/mnt/c")),
            "Windows drive mounts stay out of the frame probe, got {searched:?}"
        );
        assert!(
            searched.iter().any(|dir| dir == fast.path()),
            "local PATH entries stay searchable, got {searched:?}"
        );

        let mut state = crate::scrollback::ScrollbackState::new();
        state.prepare_layout(80, 10);
        let after_frame = path_lookups_for_test();
        assert_eq!(
            after_frame, 1,
            "the first layout probes ffmpeg once, got {after_frame}"
        );
        state.prepare_layout(80, 10);
        assert_eq!(
            path_lookups_for_test(),
            after_frame,
            "the next frame must not walk PATH"
        );
        assert!(!ffmpeg_available());

        let video = media(true);
        assert_eq!(inline_media_reserved_rows(&video, 80), (1, 2));
        let after_banner = path_lookups_for_test();
        let managers = ffmpeg_install_candidates().len() as u32;
        assert_eq!(
            after_banner,
            after_frame + managers,
            "the install banner probes each package manager once, got {after_banner}"
        );
        assert_eq!(inline_media_reserved_rows(&video, 80), (1, 2));
        assert_eq!(
            path_lookups_for_test(),
            after_banner,
            "measuring the banner again must not walk PATH"
        );

        #[cfg(unix)]
        {
            let exe = fast.path().join("ffmpeg");
            std::fs::write(&exe, b"#!/bin/sh\nexit 0\n").expect("write ffmpeg");
            std::fs::set_permissions(
                &exe,
                std::os::unix::fs::PermissionsExt::from_mode(0o755),
            )
            .expect("chmod ffmpeg");
        }
        assert!(
            !ffmpeg_available(),
            "a cached miss stays until the retry interval"
        );
        assert_eq!(path_lookups_for_test(), after_banner);

        expire_ffmpeg_path_probe_for_test();
        state.prepare_layout(80, 10);
        assert!(
            ffmpeg_available(),
            "after the retry interval a layout notices ffmpeg on the local PATH"
        );
        let after_install = path_lookups_for_test();
        assert_eq!(
            after_install,
            after_banner + 1,
            "the refresh probes ffmpeg once, got {after_install}"
        );
        state.prepare_layout(80, 10);
        assert_eq!(
            path_lookups_for_test(),
            after_install,
            "a hit stays cached and later frames do not walk PATH"
        );
        let (image_rows, total_rows) = inline_media_reserved_rows(&video, 80);
        assert!(
            (4..=20).contains(&image_rows),
            "ffmpeg present reserves a poster, not the banner: {image_rows}"
        );
        assert_eq!(total_rows, image_rows + 3);
        assert_eq!(path_lookups_for_test(), after_install);

        reset_ffmpeg_path_probe_for_test();
    }
}
