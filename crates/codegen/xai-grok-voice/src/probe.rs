//! Voice diagnostics: input-device lookup, silent-mic fix text, and an end-to-end probe.
//! The probe captures mic audio, streams it to STT, and reports the transcript.

#[cfg(feature = "audio")]
use std::sync::Arc;
#[cfg(feature = "audio")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "audio")]
use std::time::Duration;

#[cfg(feature = "audio")]
use tokio::time::timeout;

use crate::auth::SharedVoiceAuth;
use crate::config::VoiceConfig;
use crate::error::VoiceError;
#[cfg(feature = "audio")]
use crate::stt::{StreamingSttEvent, StreamingSttSession};

/// Options for [`run_streaming_probe`].
#[derive(Debug, Clone)]
pub struct VoiceProbeOptions {
    pub config: VoiceConfig,
    pub auth: SharedVoiceAuth,
    /// How long to capture microphone audio before `audio.done`.
    pub capture_secs: u32,
}

/// Collected probe output.
#[derive(Debug)]
pub struct VoiceProbeReport {
    pub pcm_bytes: usize,
    pub stt_log: Vec<String>,
    pub transcript: Option<String>,
}

/// Capture mic audio and stream it to xAI STT, reporting the transcript.
#[cfg(feature = "audio")]
pub async fn run_streaming_probe(opts: VoiceProbeOptions) -> Result<VoiceProbeReport, VoiceError> {
    let bearer = crate::auth::require_bearer(&opts.auth).await?;
    let mut stt = StreamingSttSession::connect(&opts.config, &bearer).await?;
    let stt_tx = stt
        .audio_sender()
        .ok_or_else(|| VoiceError::Stt("STT audio sender unavailable".into()))?;

    let byte_count = Arc::new(AtomicUsize::new(0));
    let byte_count_cb = Arc::clone(&byte_count);
    let (pcm_tx, pcm_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let forward = tokio::spawn(async move {
        let mut pcm_rx = pcm_rx;
        while let Some(chunk) = pcm_rx.recv().await {
            byte_count_cb.fetch_add(chunk.len(), Ordering::Relaxed);
            if stt_tx.send(chunk).await.is_err() {
                break;
            }
        }
    });

    let sample_rate = opts.config.sample_rate;
    let secs = opts.capture_secs.max(1);
    let capture = crate::audio::spawn_pcm_capture(sample_rate, pcm_tx)?;
    tracing::info!(secs, "speak now — probe is listening");
    tokio::time::sleep(Duration::from_secs(secs as u64)).await;
    capture.stop();
    let _ = forward.await;
    stt.finish_audio();

    let mut stt_log = Vec::new();
    let mut transcript = None;
    let deadline = Duration::from_secs(30);

    loop {
        let ev = match timeout(deadline, stt.recv()).await {
            Ok(Some(ev)) => ev,
            Ok(None) => {
                stt_log.push("STT channel closed".into());
                break;
            }
            Err(_) => {
                stt_log.push("STT recv timed out (30s)".into());
                break;
            }
        };

        match &ev {
            StreamingSttEvent::Ready => stt_log.push("STT: ready (transcript.created)".into()),
            StreamingSttEvent::Partial(p) => {
                stt_log.push(format!(
                    "STT: partial is_final={} speech_final={} text={:?}",
                    p.is_final, p.speech_final, p.text
                ));
                if !p.text.trim().is_empty() && (p.speech_final || p.is_final) {
                    transcript = Some(p.text.clone());
                }
            }
            StreamingSttEvent::Done { text } => {
                stt_log.push(format!("STT: done text={:?}", text));
                if !text.trim().is_empty() {
                    transcript = Some(text.clone());
                }
                break;
            }
            StreamingSttEvent::Error { message } => {
                stt_log.push(format!("STT: error {message}"));
                break;
            }
        }
    }

    let pcm_bytes = byte_count.load(Ordering::Relaxed);

    Ok(VoiceProbeReport {
        pcm_bytes,
        stt_log,
        transcript,
    })
}

/// Record mic only (no STT): a quick hardware check.
#[cfg(feature = "audio")]
pub fn run_mic_only_probe(sample_rate: u32, seconds: u32) -> Result<(usize, u32), VoiceError> {
    let (pcm, chunks) = crate::audio::capture_pcm_for_duration(sample_rate, seconds)?;
    Ok((pcm.len(), chunks))
}

#[cfg(not(feature = "audio"))]
pub async fn run_streaming_probe(_opts: VoiceProbeOptions) -> Result<VoiceProbeReport, VoiceError> {
    Err(VoiceError::Config(
        "voice probe requires the `audio` feature (cpal)".into(),
    ))
}

/// The input device capture would use (cpal default, or the Linux recorder name).
/// It stays available without `audio` so `/terminal-setup` compiles in no-audio builds.
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    pub name: String,
    pub detail: String,
}

/// Look up the input device without opening a stream (does not trigger the macOS mic-permission prompt).
#[cfg(feature = "audio")]
pub fn input_device_info() -> Result<InputDeviceInfo, VoiceError> {
    crate::audio::input_device_info()
}

#[cfg(not(feature = "audio"))]
pub fn input_device_info() -> Result<InputDeviceInfo, VoiceError> {
    Err(VoiceError::Config(
        "voice audio capture disabled (build without `audio` feature)".into(),
    ))
}

/// Platform-specific fix text for a mic that isn't being picked up.
/// On macOS the grant belongs to the terminal app and only applies after that app restarts.
pub fn mic_fix_help() -> String {
    mic_permission_help(std::env::consts::OS, current_terminal_host_app())
}

/// Hint for a capture error that means the OS or the device refused the microphone.
/// Other voice failures stay hint-free. `os` is `std::env::consts::OS`.
pub fn mic_block_hint(os: &str, error: &str) -> Option<String> {
    if !is_microphone_block(error) {
        return None;
    }
    Some(mic_permission_help(os, None))
}

fn is_microphone_block(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("microphone")
        || lower.contains("mic permission")
        || lower.contains("input config")
        || lower.contains("input device")
        || lower.contains("input audio")
        || lower.contains("no default input")
}

/// The `/voice` permission sentence for `os` (`"macos"`, `"windows"`, or anything else).
/// `app` is the terminal macOS lists under Microphone, when this process can tell.
pub fn mic_permission_help(os: &str, app: Option<&str>) -> String {
    if os == "macos" {
        macos_microphone_permission_sentence(app)
    } else if os == "windows" {
        "Windows is blocking the microphone. Turn access on in Settings → \
         Privacy & security → Microphone. If access is already on, check the \
         input device and level in Settings → System → Sound."
            .to_string()
    } else {
        "check the default input device and its volume in your sound settings \
         (e.g. `pavucontrol`, or `wpctl status` on PipeWire)."
            .to_string()
    }
}

/// macOS `/voice` text: which app is missing Microphone permission, and how to allow it.
fn macos_microphone_permission_sentence(app: Option<&str>) -> String {
    match app.map(str::trim).filter(|app| !app.is_empty()) {
        Some(app) => format!(
            "{app} is missing Microphone permission. Open System Settings → Privacy & Security → \
             Microphone, turn on {app}, then restart {app}. If access is already on, check the \
             input device and level in System Settings → Sound → Input."
        ),
        None => "Your terminal is missing Microphone permission. Open System Settings → Privacy & \
                 Security → Microphone, turn the terminal on, then restart it. If access is \
                 already on, check the input device and level in System Settings → Sound → Input."
            .to_string(),
    }
}

/// The terminal app that holds the macOS Microphone grant, from the same env markers the pager uses.
/// A multiplexer is not that app: the grant is on the terminal that hosts it.
pub fn terminal_host_app(get: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    fn nonempty(get: &impl Fn(&str) -> Option<String>, key: &str) -> Option<String> {
        get(key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
    fn label(raw: &str) -> Option<&'static str> {
        let normalized: String = raw
            .chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_' | '.'))
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match normalized.as_str() {
            "appleterminal" => Some("Terminal"),
            "ghostty" => Some("Ghostty"),
            "iterm" | "iterm2" | "itermapp" => Some("iTerm2"),
            "warp" | "warpterminal" => Some("Warp"),
            "vscode" => Some("Visual Studio Code"),
            "wezterm" => Some("WezTerm"),
            "kitty" => Some("kitty"),
            "alacritty" => Some("Alacritty"),
            "zed" => Some("Zed"),
            "rio" => Some("Rio"),
            "hyper" => Some("Hyper"),
            "tabby" => Some("Tabby"),
            "grokdesktop" => Some("Grok"),
            "rex" => Some("Rex"),
            "windowsterminal" => Some("Windows Terminal"),
            "tmux" | "screen" | "byobu" => None,
            _ => None,
        }
    }

    if nonempty(&get, "CURSOR_TRACE_ID").is_some() {
        return Some("Cursor");
    }
    if let Some(askpass) = nonempty(&get, "VSCODE_GIT_ASKPASS_MAIN") {
        let lower = askpass.to_ascii_lowercase();
        if lower.contains("cursor") {
            return Some("Cursor");
        }
        if lower.contains("windsurf") {
            return Some("Windsurf");
        }
        return Some("Visual Studio Code");
    }
    if let Some(name) = nonempty(&get, "LC_TERMINAL").as_deref().and_then(label) {
        return Some(name);
    }
    if let Some(name) = nonempty(&get, "TERM_PROGRAM").as_deref().and_then(label) {
        return Some(name);
    }
    if nonempty(&get, "WEZTERM_VERSION").is_some() {
        return Some("WezTerm");
    }
    if nonempty(&get, "ITERM_SESSION_ID").is_some() || nonempty(&get, "ITERM_PROFILE").is_some() {
        return Some("iTerm2");
    }
    if nonempty(&get, "KITTY_WINDOW_ID").is_some() {
        return Some("kitty");
    }
    if let Some(emulator) = nonempty(&get, "TERMINAL_EMULATOR") {
        let lower = emulator.to_ascii_lowercase();
        if lower.contains("jetbrains") || lower.contains("jediterm") {
            return Some("JetBrains");
        }
    }
    if nonempty(&get, "REX_SESSION").is_some()
        || nonempty(&get, "REX_BLOCK").is_some()
        || nonempty(&get, "TERM").is_some_and(|term| term == "xterm-rex")
    {
        return Some("Rex");
    }
    if nonempty(&get, "TERM_SESSION_ID").is_some() {
        return Some("Terminal");
    }
    None
}

fn current_terminal_host_app() -> Option<&'static str> {
    terminal_host_app(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

/// Human-readable multi-line report for terminal output.
pub fn format_probe_report(report: &VoiceProbeReport) -> String {
    let mut out = String::from("=== xai-grok-voice probe ===\n\n");

    out.push_str(&format!(
        "Mic capture (streamed)\n  pcm_bytes: {}\n",
        report.pcm_bytes
    ));
    if report.pcm_bytes == 0 {
        out.push_str("  WARNING: no PCM captured — check mic permission / default input device\n");
    } else {
        let secs_approx = report.pcm_bytes as f64 / (16000.0 * 2.0);
        out.push_str(&format!(
            "  approx duration: {secs_approx:.2}s @ 16kHz mono PCM16\n"
        ));
    }

    out.push_str("\nSTT events\n");
    if report.stt_log.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for line in &report.stt_log {
            out.push_str(&format!("  {line}\n"));
        }
    }

    out.push_str("\nTranscript\n");
    match &report.transcript {
        Some(t) if !t.trim().is_empty() => out.push_str(&format!("  {t}\n")),
        Some(_) => out.push_str("  (empty string)\n"),
        None => out.push_str("  (none — STT returned no text)\n"),
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn macos_voice_names_the_terminal_missing_microphone_permission() {
        let ghostty = mic_permission_help("macos", Some("Ghostty"));
        assert!(
            ghostty.contains("Ghostty") && ghostty.contains("Microphone"),
            "the sentence has to name the terminal and Microphone permission: {ghostty}"
        );
        assert!(
            ghostty.contains("System Settings") && ghostty.contains("Privacy & Security"),
            "the sentence has to say where to turn the grant on: {ghostty}"
        );
        assert!(
            ghostty.to_lowercase().contains("restart"),
            "the grant applies only after the terminal restarts: {ghostty}"
        );
        assert_eq!(
            terminal_host_app(env(&[("TERM_PROGRAM", "Apple_Terminal")])),
            Some("Terminal")
        );
        assert_eq!(
            terminal_host_app(env(&[("TERM_PROGRAM", "iTerm.app")])),
            Some("iTerm2")
        );
        assert_eq!(
            terminal_host_app(env(&[
                ("TERM_PROGRAM", "tmux"),
                ("LC_TERMINAL", "iTerm2"),
            ])),
            Some("iTerm2"),
            "tmux is not the app listed under Microphone"
        );
        let unnamed = mic_permission_help("macos", None);
        assert!(
            unnamed.contains("terminal") && unnamed.contains("Microphone"),
            "an unknown host still names the terminal and the permission: {unnamed}"
        );
        assert!(
            unnamed.contains("System Settings"),
            "an unknown host still says how to fix it: {unnamed}"
        );
    }

    #[test]
    fn windows_blocked_microphone_names_the_settings_page() {
        let help = mic_permission_help("windows", Some("Windows Terminal"));
        assert!(
            help.contains("Settings → Privacy & security → Microphone"),
            "windows has to name the page where microphone access is turned on: {help}"
        );
        assert!(
            help.to_lowercase().contains("turn access on"),
            "the sentence has to say to turn access on: {help}"
        );
        assert!(
            !help.contains("System Settings"),
            "windows must not send the user to the macOS settings app: {help}"
        );
        assert!(
            !help.contains("Windows Terminal"),
            "the windows sentence does not depend on the macOS host app: {help}"
        );

        let blocked = mic_block_hint(
            "windows",
            "default input config for Microphone: Access is denied",
        );
        let blocked = blocked.expect("a refused input device is a microphone block");
        assert!(
            blocked.contains("Privacy & security → Microphone"),
            "the block hint uses the windows page: {blocked}"
        );
        assert!(
            mic_block_hint("windows", "configuration: bad STT URL").is_none(),
            "an unrelated voice error does not grow a microphone walkthrough"
        );
    }
}
