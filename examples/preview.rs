//! Print every block `sa` can draw, without touching the execution state.
//!
//! Useful for tweaking the layout from a machine that is not Windows:
//!
//! ```text
//! cargo run --example preview
//! ```

use std::io::stderr;
use std::time::Duration;

use sa::Stop;
use sa::platform::{ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
use sa::ui::{self, Controls, Live};

fn main() {
    colored::control::set_override(true);
    let controls = Controls { enter: true };
    let limit = Duration::from_secs(5_400);

    ui::banner(
        &mut stderr(),
        true,
        ES_CONTINUOUS,
        ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED,
        Some(limit),
        controls,
    )
    .unwrap();

    // Two seconds of the timed status line, sped up to cover the whole bar.
    let live = Live::new(false, Some(limit), controls);
    for step in 0..40 {
        live.tick(limit.mul_f64(f64::from(step) / 40.0));
        std::thread::sleep(Duration::from_millis(50));
    }
    live.clear();

    ui::farewell(&mut stderr(), Stop::Elapsed, ES_CONTINUOUS, limit).unwrap();

    ui::banner(
        &mut stderr(),
        false,
        ES_CONTINUOUS,
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
        None,
        controls,
    )
    .unwrap();

    let live = Live::new(false, None, controls);
    for step in 0..20 {
        live.tick(Duration::from_secs(step));
        std::thread::sleep(Duration::from_millis(50));
    }
    live.clear();

    ui::farewell(
        &mut stderr(),
        Stop::Interrupt,
        ES_CONTINUOUS,
        Duration::from_secs(19),
    )
    .unwrap();
}
