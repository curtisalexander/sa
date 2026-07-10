//! Command-line surface for `sa`.

use std::time::Duration;

use clap::Parser;
use clap::builder::styling::{AnsiColor, Effects, Styles};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

const EXAMPLES: &str = "\x1b[1;96mExamples:\x1b[0m
  \x1b[1;92msa\x1b[0m                 keep the machine awake until you press Enter
  \x1b[1;92msa --display\x1b[0m       also keep the display on
  \x1b[1;92msa --for 90m\x1b[0m       stay awake for 90 minutes, then reset
  \x1b[1;92msa -d -f 1h30m\x1b[0m     display on for an hour and a half

Verify what is holding the machine awake with `powercfg /requests` in an
elevated prompt. Set NO_COLOR=1 to turn off colored output.";

#[derive(Parser, Debug, Default)]
#[command(
    name = "sa",
    version,
    about = "Keep a Windows machine awake",
    long_about = "Keep a Windows machine awake.\n\n\
                  Holds a Win32 SetThreadExecutionState request for as long as sa runs, \
                  and releases it on exit. Closing the laptop lid or pressing the power \
                  button still sleeps the machine.",
    styles = STYLES,
    after_help = EXAMPLES
)]
pub struct Args {
    /// Keep the display on as well as the machine
    #[arg(short, long)]
    pub display: bool,

    /// Stay awake for a fixed duration, then reset (30s, 45m, 2h, 1h30m)
    #[arg(
        short = 'f',
        long = "for",
        value_name = "DURATION",
        value_parser = crate::duration::parse,
    )]
    pub duration: Option<Duration>,

    /// Print nothing but errors
    #[arg(short, long)]
    pub quiet: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definition_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn defaults_to_system_mode_until_stopped() {
        let args = Args::try_parse_from(["sa"]).unwrap();
        assert!(!args.display);
        assert!(!args.quiet);
        assert_eq!(args.duration, None);
    }

    #[test]
    fn accepts_short_and_long_flags() {
        let long = Args::try_parse_from(["sa", "--display", "--for", "1h30m", "--quiet"]).unwrap();
        let short = Args::try_parse_from(["sa", "-d", "-f", "1h30m", "-q"]).unwrap();
        assert!(long.display && long.quiet);
        assert_eq!(long.duration, Some(Duration::from_secs(5_400)));
        assert_eq!(short.duration, long.duration);
    }

    #[test]
    fn rejects_an_unparseable_duration() {
        let err = Args::try_parse_from(["sa", "--for", "later"]).unwrap_err();
        assert!(err.to_string().contains("unknown unit"), "{err}");

        let err = Args::try_parse_from(["sa", "--for", "30"]).unwrap_err();
        assert!(err.to_string().contains("needs a unit"), "{err}");
    }
}
