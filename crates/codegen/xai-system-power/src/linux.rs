//! Linux system sleep/wake via systemd-logind's `PrepareForSleep` D-Bus
//! signal, with a `delay` inhibitor lock so we get a short window to react
//! before the system actually sleeps.
//!
//! Uses the `zbus` blocking API on a dedicated thread so we don't require the
//! caller to run any particular async runtime. If the system bus socket is
//! absent, `start` returns `None` and the caller degrades gracefully. When the
//! socket is present, the Hello handshake runs on the worker: that call has no
//! timeout, and doing it on the startup thread stalls the process for as long
//! as a slow or wedged bus takes to answer.

use std::io::ErrorKind;
use std::path::Path;
use std::thread;

use super::{PowerCallback, PowerEvent};

const DEST: &str = "org.freedesktop.login1";
const PATH: &str = "/org/freedesktop/login1";
const IFACE: &str = "org.freedesktop.login1.Manager";

/// There is intentionally no clean stop: the worker thread parks on a blocking logind signal iterator, which cannot be
/// interrupted without a signal arriving, so dropping this neither joins nor cancels it. (macOS can `CFRunLoopStop` from
/// `Drop` and so joins; Linux cannot — hence the asymmetry, and why there is no `Drop` impl here.)
pub(crate) struct Listener;

impl Listener {
    pub(crate) fn start(callback: PowerCallback) -> Option<Self> {
        // A missing socket is a stat, not a handshake, so startup still hears
        // "no logind" immediately. A present socket can still wedge inside
        // Hello; that wait belongs on the worker.
        open_listener(system_bus_might_answer(), move || {
            if let Some((proxy, signals)) = subscribe() {
                run_thread(proxy, signals, callback);
            }
        })
    }
}

/// Spawn the logind worker without waiting for it to reach the system bus.
/// `bus_present` is the cheap pre-check from [`system_bus_might_answer`].
fn open_listener<F>(bus_present: bool, work: F) -> Option<Listener>
where
    F: FnOnce() + Send + 'static,
{
    if !bus_present {
        return None;
    }
    thread::Builder::new()
        .name("xai-power-listener".into())
        .spawn(work)
        .ok()
        .map(|_| Listener)
}

fn subscribe() -> Option<(
    zbus::blocking::Proxy<'static>,
    zbus::blocking::proxy::SignalIterator<'static>,
)> {
    let conn = zbus::blocking::Connection::system().ok()?;
    let proxy = zbus::blocking::Proxy::new(&conn, DEST, PATH, IFACE).ok()?;
    let signals = proxy.receive_signal("PrepareForSleep").ok()?;
    Some((proxy, signals))
}

/// Whether a system bus might exist. This is a filesystem check (or "not a
/// plain missing `unix:path`"), never a D-Bus round trip.
fn system_bus_might_answer() -> bool {
    system_bus_might_answer_from(
        std::env::var("DBUS_SYSTEM_BUS_ADDRESS").ok().as_deref(),
        default_system_bus_socket_exists(),
    )
}

fn system_bus_might_answer_from(env_addr: Option<&str>, default_socket_exists: bool) -> bool {
    match env_addr {
        Some(addr) => bus_address_might_answer(addr),
        None => default_socket_exists,
    }
}

fn default_system_bus_socket_exists() -> bool {
    // Same path `zbus` uses when `DBUS_SYSTEM_BUS_ADDRESS` is unset.
    path_might_be_a_socket(Path::new("/var/run/dbus/system_bus_socket"))
}

/// `unix:path=` that is not on disk cannot answer. Any other address (abstract
/// socket, tcp, an escaped path) is left to the worker so this check cannot
/// stall startup or skip a bus it failed to parse.
fn bus_address_might_answer(address: &str) -> bool {
    let Some(path) = unix_path_address(address) else {
        return !address.is_empty();
    };
    if path.is_empty() || path.chars().any(|c| c == '%' || c == '\\') {
        return !path.is_empty();
    }
    path_might_be_a_socket(Path::new(path))
}

fn unix_path_address(address: &str) -> Option<&str> {
    let rest = address.strip_prefix("unix:")?;
    rest.split(',').find_map(|part| part.strip_prefix("path="))
}

fn path_might_be_a_socket(path: &Path) -> bool {
    match path.metadata() {
        Ok(_) => true,
        Err(err) if err.kind() == ErrorKind::NotFound => false,
        // Permission or a transient stat error: let the worker try the bus.
        Err(_) => true,
    }
}

/// Take a `delay` sleep inhibitor: logind holds off sleep until the returned fd
/// drops. `None` if unavailable — we then react without a pre-sleep window.
fn take_inhibitor(proxy: &zbus::blocking::Proxy<'_>) -> Option<zbus::zvariant::OwnedFd> {
    proxy
        .call(
            "Inhibit",
            &("sleep", "grok", "Pause token refresh across sleep", "delay"),
        )
        .ok()
}

fn run_thread(
    proxy: zbus::blocking::Proxy<'static>,
    signals: zbus::blocking::proxy::SignalIterator<'static>,
    callback: PowerCallback,
) {
    // Hold the delay lock so the first PrepareForSleep(true) gives us a window.
    let mut inhibitor = take_inhibitor(&proxy);

    for msg in signals {
        let Ok(about_to_sleep) = msg.body().deserialize::<bool>() else {
            continue;
        };
        if about_to_sleep {
            // The callback may block (bounded) waiting for an in-flight token refresh to finish; the `delay` inhibitor is still held
            // across it, so that wait holds off the suspend (up to logind's `InhibitDelayMaxSec`, default 5 s). Release it only once
            // the callback returns so the system can then proceed to sleep.
            callback(PowerEvent::WillSleep);
            inhibitor = None;
        } else {
            callback(PowerEvent::DidWake);
            // Re-acquire the delay lock for the next sleep cycle.
            inhibitor = take_inhibitor(&proxy);
        }
    }

    drop(inhibitor);
}

pub(crate) fn current_power_state() -> crate::PowerState {
    // Linux has no "dark wake" equivalent to query (the system is either
    // suspended or fully awake); report Unknown so callers fall back to the
    // logind `PrepareForSleep` path.
    crate::PowerState::Unknown
}

/// No power-assertion support on this platform: callers carry on unprotected, which is the same behaviour as before
/// assertions existed. Never constructed here (`hold_awake` always returns `None`); it exists so the cross-platform
/// `SleepAssertion` has a field type on every target.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct Assertion;

pub(crate) fn hold_awake(_reason: &str) -> Option<Assertion> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn missing_unix_path_is_not_a_bus() {
        assert!(!bus_address_might_answer(
            "unix:path=/no/such/grok-system-bus"
        ));
        assert!(!bus_address_might_answer("unix:path="));
        assert!(!bus_address_might_answer(""));
    }

    #[test]
    fn present_unix_path_might_answer() {
        let path = std::env::temp_dir().join(format!("grok-bus-probe-{}", std::process::id()));
        std::fs::write(&path, b"").unwrap();
        let addr = format!("unix:path={},guid=abc", path.display());
        assert!(bus_address_might_answer(&addr));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn non_path_address_is_left_to_the_worker() {
        assert!(bus_address_might_answer(
            "unix:abstract=/org/freedesktop/DBus"
        ));
        assert!(bus_address_might_answer("tcp:host=127.0.0.1,port=1"));
        // An escaped path is not stat'd here; a failed unescape must not skip the bus.
        assert!(bus_address_might_answer("unix:path=/tmp/bus%20sock"));
    }

    #[test]
    fn unset_address_uses_the_default_socket_check() {
        assert!(!system_bus_might_answer_from(None, false));
        assert!(system_bus_might_answer_from(None, true));
        assert!(!system_bus_might_answer_from(
            Some("unix:path=/no/such/grok-system-bus"),
            true
        ));
    }

    #[test]
    fn absent_bus_does_not_spawn_the_worker() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&called);
        let listener = open_listener(false, move || {
            flag.store(true, Ordering::SeqCst);
        });
        assert!(listener.is_none());
        assert!(!called.load(Ordering::SeqCst));
    }

    #[test]
    fn open_listener_returns_while_the_bus_handshake_is_blocked() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let began = Instant::now();
        let listener = open_listener(true, move || {
            let _ = entered_tx.send(());
            // Bound the wait so a regression that joins the worker fails the
            // timing assert instead of hanging the suite.
            let _ = release_rx.recv_timeout(Duration::from_secs(2));
        });
        assert!(listener.is_some());
        assert!(
            began.elapsed() < Duration::from_millis(500),
            "startup waited {:?} for the bus handshake",
            began.elapsed()
        );
        assert!(entered_rx.recv_timeout(Duration::from_secs(2)).is_ok());
        let _ = release_tx.send(());
    }
}
