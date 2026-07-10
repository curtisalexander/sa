//! The Win32 thread execution state, and an RAII guard that restores it.

use std::fmt;
use std::marker::PhantomData;
use std::ops::BitOr;

use anyhow::{Result, bail};

/// A bitmask of `ES_*` flags, mirroring the Win32 `EXECUTION_STATE` type.
///
/// Defined here rather than pulled from `windows-sys` so the crate compiles and
/// tests on every platform; `windows_constants_match` pins them to the real ones.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExecutionState(pub u32);

pub const ES_SYSTEM_REQUIRED: ExecutionState = ExecutionState(0x0000_0001);
pub const ES_DISPLAY_REQUIRED: ExecutionState = ExecutionState(0x0000_0002);
pub const ES_USER_PRESENT: ExecutionState = ExecutionState(0x0000_0004);
pub const ES_AWAYMODE_REQUIRED: ExecutionState = ExecutionState(0x0000_0040);
pub const ES_CONTINUOUS: ExecutionState = ExecutionState(0x8000_0000);

/// Flags in the order Windows documents them, used to render a state as text.
const NAMED_FLAGS: [(ExecutionState, &str); 5] = [
    (ES_CONTINUOUS, "ES_CONTINUOUS"),
    (ES_DISPLAY_REQUIRED, "ES_DISPLAY_REQUIRED"),
    (ES_SYSTEM_REQUIRED, "ES_SYSTEM_REQUIRED"),
    (ES_AWAYMODE_REQUIRED, "ES_AWAYMODE_REQUIRED"),
    (ES_USER_PRESENT, "ES_USER_PRESENT"),
];

impl ExecutionState {
    /// `ES_CONTINUOUS | ES_SYSTEM_REQUIRED`, and so on. Any bits Windows has
    /// grown since this was written are appended as a bare hex value.
    pub fn label(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut remaining = self.0;

        for (flag, name) in NAMED_FLAGS {
            if remaining & flag.0 == flag.0 {
                remaining &= !flag.0;
                parts.push(name.to_string());
            }
        }
        if remaining != 0 {
            parts.push(format!("{remaining:#X}"));
        }
        if parts.is_empty() {
            return String::from("ES_NONE");
        }
        parts.join(" | ")
    }

    /// `ES_CONTINUOUS | ES_SYSTEM_REQUIRED (0x80000001)`
    pub fn describe(self) -> String {
        format!("{} ({:#010X})", self.label(), self.0)
    }
}

impl BitOr for ExecutionState {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        ExecutionState(self.0 | rhs.0)
    }
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

#[cfg(windows)]
mod sys {
    pub const SUPPORTED: bool = true;

    pub fn set_thread_execution_state(flags: u32) -> u32 {
        // SAFETY: a plain FFI call taking a bitmask by value and returning one.
        unsafe { windows_sys::Win32::System::Power::SetThreadExecutionState(flags) }
    }
}

#[cfg(not(windows))]
mod sys {
    pub const SUPPORTED: bool = false;

    pub fn set_thread_execution_state(_flags: u32) -> u32 {
        0
    }
}

/// Holds the requested execution state for as long as it is alive.
///
/// `SetThreadExecutionState` applies to the *calling thread*, and Windows drops
/// the request when that thread exits. So the guard must be acquired and released
/// on the same thread, and it is deliberately `!Send` to enforce that: a Ctrl-C
/// handler (which Windows runs on its own thread) can only set a flag, never
/// release the guard.
#[derive(Debug)]
pub struct StayAwake {
    released: bool,
    _not_send: PhantomData<*const ()>,
}

impl StayAwake {
    /// Requests `state`, returning the guard and the state that was previously in
    /// effect for this thread.
    pub fn acquire(state: ExecutionState) -> Result<(Self, ExecutionState)> {
        if !sys::SUPPORTED {
            bail!(
                "sa keeps Windows machines awake, and this is not Windows.\n       \
                 On macOS use `caffeinate`; on Linux use `systemd-inhibit`."
            );
        }
        if state.0 & ES_CONTINUOUS.0 == 0 {
            bail!("{} must be combined with ES_CONTINUOUS", state.label());
        }

        let previous = set(state)?;
        let guard = StayAwake {
            released: false,
            _not_send: PhantomData,
        };
        Ok((guard, previous))
    }

    /// Clears the request, returning the state Windows is left in.
    ///
    /// Only marks itself released once the call succeeds, so a failure here still
    /// gets one more best-effort attempt from `Drop`.
    pub fn release(&mut self) -> Result<ExecutionState> {
        set(ES_CONTINUOUS)?;
        self.released = true;
        Ok(ES_CONTINUOUS)
    }
}

impl Drop for StayAwake {
    fn drop(&mut self) {
        if !self.released {
            let _ = set(ES_CONTINUOUS);
        }
    }
}

fn set(state: ExecutionState) -> Result<ExecutionState> {
    match sys::set_thread_execution_state(state.0) {
        0 => bail!(
            "Windows refused to set the thread execution state to {}",
            state.label()
        ),
        previous => Ok(ExecutionState(previous)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_decomposes_a_combined_state() {
        let state = ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED;
        assert_eq!(
            state.label(),
            "ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED"
        );
        assert_eq!(state.0, 0x8000_0003);
    }

    #[test]
    fn label_names_a_lone_flag() {
        assert_eq!(ES_CONTINUOUS.label(), "ES_CONTINUOUS");
        assert_eq!(ES_SYSTEM_REQUIRED.label(), "ES_SYSTEM_REQUIRED");
    }

    #[test]
    fn label_keeps_unknown_bits_visible() {
        assert_eq!(
            ExecutionState(ES_CONTINUOUS.0 | 0x10).label(),
            "ES_CONTINUOUS | 0x10"
        );
        assert_eq!(ExecutionState(0).label(), "ES_NONE");
    }

    #[test]
    fn describe_appends_the_hex_value() {
        assert_eq!(ES_CONTINUOUS.describe(), "ES_CONTINUOUS (0x80000000)");
    }

    #[test]
    fn acquire_rejects_a_state_without_es_continuous() {
        let err = StayAwake::acquire(ES_SYSTEM_REQUIRED).unwrap_err();
        // The Windows check comes first off-Windows, so only assert on Windows.
        if cfg!(windows) {
            assert!(err.to_string().contains("ES_CONTINUOUS"));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn acquire_explains_itself_off_windows() {
        let err = StayAwake::acquire(ES_CONTINUOUS | ES_SYSTEM_REQUIRED).unwrap_err();
        assert!(err.to_string().contains("not Windows"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_constants_match() {
        use windows_sys::Win32::System::Power as p;
        assert_eq!(ES_CONTINUOUS.0, p::ES_CONTINUOUS);
        assert_eq!(ES_SYSTEM_REQUIRED.0, p::ES_SYSTEM_REQUIRED);
        assert_eq!(ES_DISPLAY_REQUIRED.0, p::ES_DISPLAY_REQUIRED);
        assert_eq!(ES_AWAYMODE_REQUIRED.0, p::ES_AWAYMODE_REQUIRED);
        assert_eq!(ES_USER_PRESENT.0, p::ES_USER_PRESENT);
    }

    #[cfg(windows)]
    #[test]
    fn acquire_then_release_round_trips() {
        let (mut guard, _previous) =
            StayAwake::acquire(ES_CONTINUOUS | ES_SYSTEM_REQUIRED).unwrap();
        assert_eq!(guard.release().unwrap(), ES_CONTINUOUS);
    }
}
