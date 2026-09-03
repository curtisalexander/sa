//! Keep a Windows machine awake for as long as `sa` is running.

pub mod cli;
pub mod duration;
pub mod platform;
pub mod ui;

use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use cli::Args;
use platform::{ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, StayAwake};
use ui::{Controls, Live};

/// How long the wait loop sleeps between redraws of the status line.
const TICK: Duration = Duration::from_millis(100);

/// Why a session ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stop {
    /// The user pressed Enter.
    Enter,
    /// The user pressed Ctrl-C.
    Interrupt,
    /// The `--for` duration ran out.
    Elapsed,
}

/// Holds the execution state until the user stops us or `--for` runs out, then
/// hands it back.
pub fn run(args: Args) -> Result<()> {
    let requested = if args.display {
        ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
    } else {
        ES_SYSTEM_REQUIRED
    };
    let next = ES_CONTINUOUS | requested;

    // Only offer Enter when there is a terminal to type it into: under a pipe or
    // a scheduled task, reading stdin returns EOF straight away.
    let controls = Controls {
        enter: io::stdin().is_terminal(),
    };

    // Installed before the request is taken so a Ctrl-C during startup does not
    // fall through to the default handler and kill us mid-acquire.
    let interrupted = interrupt_flag()?;

    let (mut guard, previous) = StayAwake::acquire(next)?;

    if !args.quiet {
        ui::banner(
            &mut io::stderr(),
            args.display,
            previous,
            next,
            args.duration,
            controls,
        )?;
    }

    let live = Live::new(args.quiet, args.duration, controls);
    let (stop, elapsed) = wait(args.duration, controls, &live, &interrupted);
    live.clear();

    let restored = guard.release()?;
    if !args.quiet {
        ui::farewell(&mut io::stderr(), stop, restored, elapsed)?;
    }
    Ok(())
}

/// Blocks until Enter, Ctrl-C, or the deadline, ticking the status line meanwhile.
fn wait(
    limit: Option<Duration>,
    controls: Controls,
    live: &Live,
    interrupted: &AtomicBool,
) -> (Stop, Duration) {
    let (tx, rx) = mpsc::channel::<()>();
    // A sender kept on this thread means `recv_timeout` always blocks for a full
    // tick rather than returning `Disconnected` instantly when nobody reads stdin.
    let _keepalive = tx.clone();
    if controls.enter {
        thread::spawn(move || {
            let mut line = String::new();
            // `Ok(0)` is EOF, which is not the user asking us to stop.
            if matches!(io::stdin().read_line(&mut line), Ok(n) if n > 0) {
                let _ = tx.send(());
            }
        });
    }

    let start = Instant::now();

    let stop = loop {
        if interrupted.load(Ordering::SeqCst) {
            break Stop::Interrupt;
        }
        if limit.is_some_and(|limit| start.elapsed() >= limit) {
            break Stop::Elapsed;
        }

        live.tick(start.elapsed());

        match rx.recv_timeout(TICK) {
            Ok(()) => break Stop::Enter,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => unreachable!("_keepalive holds the channel"),
        }
    };

    (stop, start.elapsed())
}

/// A flag the Ctrl-C handler raises.
///
/// The handler only stores; it never touches the execution state. Windows runs it
/// on a thread of its own, and `SetThreadExecutionState` applies to the calling
/// thread, so releasing the request from there would clear the wrong thread's.
fn interrupt_flag() -> Result<Arc<AtomicBool>> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&interrupted);
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst))
        .context("could not install a Ctrl-C handler")?;
    Ok(interrupted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live() -> Live {
        Live::new(true, None, Controls { enter: false })
    }

    #[test]
    fn wait_stops_when_the_duration_elapses() {
        let interrupted = AtomicBool::new(false);
        let limit = Duration::from_millis(150);
        let (stop, elapsed) = wait(
            Some(limit),
            Controls { enter: false },
            &live(),
            &interrupted,
        );

        assert_eq!(stop, Stop::Elapsed);
        assert!(
            elapsed >= limit,
            "{elapsed:?} should have reached {limit:?}"
        );
    }

    #[test]
    fn wait_stops_when_the_interrupt_flag_is_raised() {
        let interrupted = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&interrupted);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            flag.store(true, Ordering::SeqCst);
        });

        let (stop, _) = wait(None, Controls { enter: false }, &live(), &interrupted);
        assert_eq!(stop, Stop::Interrupt);
    }

    #[test]
    fn wait_accepts_the_largest_duration_without_overflowing() {
        let interrupted = AtomicBool::new(true);
        let (stop, _) = wait(
            Some(Duration::MAX),
            Controls { enter: false },
            &live(),
            &interrupted,
        );
        assert_eq!(stop, Stop::Interrupt);
    }

    #[test]
    fn wait_does_not_read_stdin_without_a_terminal() {
        // The regression this guards: reading a non-terminal stdin returns EOF at
        // once, which would end an `sa --for 8h` launched from a script instantly.
        let interrupted = AtomicBool::new(false);
        let (stop, _) = wait(
            Some(Duration::from_millis(120)),
            Controls { enter: false },
            &live(),
            &interrupted,
        );
        assert_eq!(stop, Stop::Elapsed);
    }
}
