//! The banner, the live status line, and the closing summary.
//!
//! Everything here writes to stderr so that `sa` stays quiet on stdout, and every
//! block is a pure function over a writer so the layout can be asserted on and
//! previewed (`cargo run --example preview`) from any platform.

use std::io::{self, Write};
use std::time::Duration;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::Stop;
use crate::duration::{clock, humanize};
use crate::platform::ExecutionState;

const RULE_WIDTH: usize = 66;
const LABEL_WIDTH: usize = 8;
/// Keeps the dimmed note after each value starting in the same column.
const VALUE_WIDTH: usize = 7;

fn rule() -> String {
    "━".repeat(RULE_WIDTH)
}

fn row(w: &mut impl Write, label: &str, value: &str) -> io::Result<()> {
    // Pad before coloring: ANSI escapes would otherwise count toward the width.
    let label = format!("{label:<LABEL_WIDTH$}");
    writeln!(w, "  {}  {}", label.dimmed(), value)
}

/// How the session is meant to end, which decides what we tell the user to press.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Controls {
    /// Whether stdin is a terminal we can read an Enter keypress from.
    pub enter: bool,
}

impl Controls {
    fn hint(self) -> String {
        if self.enter {
            format!(
                "{} to stop  ·  {} also works",
                "Enter".bright_yellow().bold(),
                "Ctrl-C".bright_yellow().bold()
            )
        } else {
            format!("{} to stop", "Ctrl-C".bright_yellow().bold())
        }
    }

    fn short_hint(self) -> String {
        if self.enter {
            format!("{} to stop", "Enter".yellow())
        } else {
            format!("{} to stop", "Ctrl-C".yellow())
        }
    }
}

/// The opening block: what mode we are in, and exactly which bits we set.
pub fn banner(
    w: &mut impl Write,
    display: bool,
    previous: ExecutionState,
    next: ExecutionState,
    limit: Option<Duration>,
    controls: Controls,
) -> io::Result<()> {
    let (mode, effect) = if display {
        ("Display", "sleep is blocked and the display stays on")
    } else {
        ("System", "sleep is blocked, but the display may turn off")
    };
    let mode = format!("{mode:<VALUE_WIDTH$}");

    writeln!(w)?;
    writeln!(w, "{}", rule().bright_cyan())?;
    writeln!(
        w,
        "  ☕  {}",
        "SA — KEEPING THIS MACHINE AWAKE".bright_cyan().bold()
    )?;
    writeln!(w, "{}", rule().bright_cyan())?;
    writeln!(w)?;

    row(
        w,
        "Mode",
        &format!("{}  {}", mode.bright_green().bold(), effect.dimmed()),
    )?;

    let (span, note) = match limit {
        Some(limit) => (humanize(limit), "then resets automatically"),
        None => (String::from("Manual"), "runs until you stop it"),
    };
    let span = format!("{span:<VALUE_WIDTH$}");
    row(
        w,
        "Runs for",
        &format!("{}  {}", span.bright_green().bold(), note.dimmed()),
    )?;

    row(w, "Was", &previous.describe().dimmed().to_string())?;
    row(w, "Now", &next.describe().bright_white().bold().to_string())?;

    writeln!(w)?;
    writeln!(w, "  {}", controls.hint())?;
    writeln!(
        w,
        "  {}",
        "Verify with `powercfg /requests` in an elevated prompt.".dimmed()
    )?;
    writeln!(w)
}

/// The closing block, printed once the execution state has been handed back.
pub fn farewell(
    w: &mut impl Write,
    stop: Stop,
    restored: ExecutionState,
    elapsed: Duration,
) -> io::Result<()> {
    let reason = match stop {
        Stop::Enter => "Stopped",
        Stop::Interrupt => "Interrupted",
        Stop::Elapsed => "Time is up",
    };
    let paint = |text: String| match stop {
        Stop::Interrupt => text.bright_yellow(),
        _ => text.bright_green(),
    };

    writeln!(w)?;
    writeln!(w, "{}", paint(rule()))?;
    writeln!(
        w,
        "  {}  {}  {}  execution state reset",
        paint(String::from("✓")).bold(),
        paint(reason.to_string()).bold(),
        "—".dimmed()
    )?;
    writeln!(
        w,
        "     {}  {}  stayed awake for {}",
        restored.describe().dimmed(),
        "·".dimmed(),
        humanize(elapsed).bright_white().bold()
    )?;
    writeln!(w, "{}", paint(rule()))
}

/// The status line that ticks while the request is held.
///
/// Hidden when `--quiet` is passed or stderr is not a terminal, in which case
/// every method is a no-op.
pub struct Live {
    bar: ProgressBar,
    limit: Option<Duration>,
    controls: Controls,
}

impl Live {
    pub fn new(quiet: bool, limit: Option<Duration>, controls: Controls) -> Self {
        let bar = if quiet {
            ProgressBar::hidden()
        } else {
            build_bar(limit)
        };
        Live {
            bar,
            limit,
            controls,
        }
    }

    /// Advances the spinner and rewrites the message. Called on every wait tick.
    pub fn tick(&self, elapsed: Duration) {
        if let Some(limit) = self.limit {
            let left = limit.saturating_sub(elapsed);
            self.bar.set_position(millis(elapsed));
            self.bar.set_message(format!(
                "awake {}  {}  {} left  {}  {}",
                clock(elapsed).bright_white(),
                "·".dimmed(),
                humanize(left).bright_cyan(),
                "·".dimmed(),
                self.controls.short_hint()
            ));
        } else {
            self.bar.set_message(format!(
                "awake {}  {}  {}",
                clock(elapsed).bright_white(),
                "·".dimmed(),
                self.controls.short_hint()
            ));
        }
        self.bar.tick();
    }

    /// Removes the status line so the closing block lands on a clean row.
    pub fn clear(&self) {
        self.bar.finish_and_clear();
    }
}

fn build_bar(limit: Option<Duration>) -> ProgressBar {
    // Milliseconds rather than seconds so the bar creeps rather than jumps.
    let template = match limit {
        Some(_) => "  {spinner:.cyan} [{bar:28.cyan/blue}] {msg}",
        None => "  {spinner:.cyan} {msg}",
    };
    let style = ProgressStyle::with_template(template)
        .expect("progress template is a compile-time constant")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
        .progress_chars("━━╌");

    let bar = match limit {
        Some(limit) => ProgressBar::new(millis(limit)),
        None => ProgressBar::new_spinner(),
    };
    bar.set_style(style);
    bar
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};

    fn render(f: impl Fn(&mut Vec<u8>) -> io::Result<()>) -> String {
        colored::control::set_override(false);
        let mut buffer = Vec::new();
        f(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    #[test]
    fn banner_reports_the_mode_and_both_states() {
        let previous = ES_CONTINUOUS;
        let next = ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED;
        let text = render(|w| {
            banner(
                w,
                true,
                previous,
                next,
                Some(Duration::from_secs(5_400)),
                Controls { enter: true },
            )
        });

        assert!(text.contains("Display"));
        assert!(text.contains("the display stays on"));
        assert!(text.contains("1h 30m"));
        assert!(text.contains("then resets automatically"));
        assert!(text.contains("ES_CONTINUOUS (0x80000000)"));
        assert!(
            text.contains("ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED (0x80000003)")
        );
        assert!(text.contains("Enter to stop"));
        assert!(text.contains("powercfg /requests"));
    }

    #[test]
    fn banner_without_a_duration_says_it_runs_until_stopped() {
        let text = render(|w| {
            banner(
                w,
                false,
                ES_CONTINUOUS,
                ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
                None,
                Controls { enter: true },
            )
        });
        assert!(text.contains("System"));
        assert!(text.contains("the display may turn off"));
        assert!(text.contains("runs until you stop it"));
        assert!(!text.contains("resets automatically"));
    }

    #[test]
    fn banner_only_offers_ctrl_c_when_stdin_is_not_a_terminal() {
        let text = render(|w| {
            banner(
                w,
                false,
                ES_CONTINUOUS,
                ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
                None,
                Controls { enter: false },
            )
        });
        assert!(text.contains("Ctrl-C to stop"));
        assert!(!text.contains("Enter"));
    }

    #[test]
    fn farewell_names_the_reason_and_the_restored_state() {
        for (stop, expected) in [
            (Stop::Enter, "Stopped"),
            (Stop::Interrupt, "Interrupted"),
            (Stop::Elapsed, "Time is up"),
        ] {
            let text = render(|w| farewell(w, stop, ES_CONTINUOUS, Duration::from_secs(83)));
            assert!(text.contains(expected), "{text}");
            assert!(text.contains("ES_CONTINUOUS (0x80000000)"));
            assert!(text.contains("stayed awake for 1m 23s"));
        }
    }

    #[test]
    fn a_quiet_live_line_draws_nothing() {
        let live = Live::new(true, None, Controls { enter: true });
        live.tick(Duration::from_secs(1));
        live.clear();
        assert!(live.bar.is_hidden());
    }

    #[test]
    fn a_timed_live_line_tracks_elapsed_milliseconds() {
        let live = Live::new(
            false,
            Some(Duration::from_secs(60)),
            Controls { enter: true },
        );
        live.tick(Duration::from_secs(15));
        assert_eq!(live.bar.position(), 15_000);
        assert_eq!(live.bar.length(), Some(60_000));
        live.clear();
    }

    #[test]
    fn a_timed_live_line_caps_unrepresentable_milliseconds() {
        let live = Live::new(false, Some(Duration::MAX), Controls { enter: true });
        assert_eq!(live.bar.length(), Some(u64::MAX));
        live.clear();
    }
}
