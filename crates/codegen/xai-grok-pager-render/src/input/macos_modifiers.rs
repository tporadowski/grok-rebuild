//! Native macOS modifier key detection via CoreGraphics.
//!
//! Reads modifier state straight from the OS rather than from the PTY's key events.
//! `CGEventSourceFlagsState` blocks forever when this process has no GUI session
//! (SSH, or no WindowServer). That call runs only after a session id is present
//! and CoreGraphics reports a window-server session.

use super::ModifierState;

// CGEventFlags bitmask constants from <CoreGraphics/CGEventTypes.h>
const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x0002_0000;
const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x0004_0000;
const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x0008_0000; // Option key
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x0010_0000;

/// `loginwindow` sets `SECURITYSESSIONID` for a GUI login. SSH does not.
pub(crate) fn gui_session_id_present(security_session_id: Option<&str>) -> bool {
    security_session_id.is_some_and(|id| !id.trim().is_empty())
}

/// Read modifier flags only when a GUI session exists. `read_flags` is the
/// CoreGraphics call, and it is not run otherwise.
pub(crate) fn modifiers_if_gui_session(
    has_gui_session: bool,
    read_flags: impl FnOnce() -> u64,
) -> ModifierState {
    if !has_gui_session {
        return ModifierState::default();
    }
    decode(read_flags())
}

fn decode(flags: u64) -> ModifierState {
    ModifierState {
        command: flags & K_CG_EVENT_FLAG_MASK_COMMAND != 0,
        option: flags & K_CG_EVENT_FLAG_MASK_ALTERNATE != 0,
        shift: flags & K_CG_EVENT_FLAG_MASK_SHIFT != 0,
        control: flags & K_CG_EVENT_FLAG_MASK_CONTROL != 0,
    }
}

#[cfg(target_os = "macos")]
mod core_graphics {
    use super::ModifierState;

    const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;

    // CoreGraphics CGEventSourceFlagsState returns the current global modifier flags
    // without requiring any special permissions. It waits on the window server.
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceFlagsState(stateID: i32) -> u64;
        fn CGSessionCopyCurrentDictionary() -> *mut std::ffi::c_void;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const std::ffi::c_void);
    }

    fn flags() -> u64 {
        // SAFETY: CGEventSourceFlagsState is a stable, public CoreGraphics API
        // available since macOS 10.4. Integer in, integer out, no pointers
        // cross the boundary. Callers run it only inside a GUI session.
        unsafe { CGEventSourceFlagsState(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE) }
    }

    /// `NULL` when this process is outside a Quartz GUI session or the window server is off.
    fn core_graphics_session_present() -> bool {
        // SAFETY: returns a +1 CFDictionary or null. A non-null dictionary is released.
        let dict = unsafe { CGSessionCopyCurrentDictionary() };
        if dict.is_null() {
            return false;
        }
        unsafe { CFRelease(dict.cast()) };
        true
    }

    pub fn snapshot() -> ModifierState {
        let session_id = std::env::var("SECURITYSESSIONID").ok();
        let has_gui = super::gui_session_id_present(session_id.as_deref())
            && core_graphics_session_present();
        super::modifiers_if_gui_session(has_gui, flags)
    }
}

#[cfg(target_os = "macos")]
pub use core_graphics::snapshot;

#[cfg(not(target_os = "macos"))]
pub fn snapshot() -> ModifierState {
    ModifierState::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_check_without_a_gui_session_does_not_read_flags() {
        assert!(!gui_session_id_present(None));
        assert!(!gui_session_id_present(Some("  ")));
        let state = modifiers_if_gui_session(false, || {
            panic!("CoreGraphics must not be called without a GUI session")
        });
        assert_eq!(state, ModifierState::default());
    }

    #[test]
    fn modifier_check_with_a_gui_session_reads_flags() {
        assert!(gui_session_id_present(Some("aqua-1")));
        let state = modifiers_if_gui_session(true, || {
            K_CG_EVENT_FLAG_MASK_COMMAND | K_CG_EVENT_FLAG_MASK_SHIFT
        });
        assert!(state.command);
        assert!(state.shift);
        assert!(!state.option);
        assert!(!state.control);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn smoke_test_modifier_detection() {
        let s = snapshot();
        let _ = s.command;
        let _ = s.option;
        let _ = s.shift;
        let _ = s.control;
    }
}
