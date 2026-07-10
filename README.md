# sa

Keep a Windows machine awake.

[![CI](https://github.com/curtisalexander/sa/actions/workflows/ci.yml/badge.svg)](https://github.com/curtisalexander/sa/actions/workflows/ci.yml)

`sa` holds a [`SetThreadExecutionState`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate) request open for as long as it runs, and hands it back when it exits. A Rust binary, shipped as a Python wheel so it installs with a single `uv tool install`.

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ☕  SA — KEEPING THIS MACHINE AWAKE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mode      Display  sleep is blocked and the display stays on
  Runs for  1h 30m   then resets automatically
  Was       ES_CONTINUOUS (0x80000000)
  Now       ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED (0x80000003)

  Enter to stop  ·  Ctrl-C also works
  Verify with `powercfg /requests` in an elevated prompt.

  ⠹ [━━━━━━━━━━━╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌] awake 00:31:30  ·  58m 30s left  ·  Enter to stop
```

## Install

Windows x64. Grab the wheel from the [Releases](https://github.com/curtisalexander/sa/releases) page and let [uv](https://docs.astral.sh/uv/) put `sa` on your PATH:

```pwsh
uv tool install https://github.com/curtisalexander/sa/releases/latest/download/sa-0.1.0-py3-none-win_amd64.whl
```

To upgrade, point at the newer wheel and add `--force`. To remove it:

```pwsh
uv tool uninstall sa
```

Building from source instead needs a [Rust toolchain](https://rustup.rs/):

```pwsh
uv tool install git+https://github.com/curtisalexander/sa
```

## Usage

There are two modes:

- **System** (default) — the machine will not go to sleep, but the display may still turn off
- **Display** (`--display`) — the machine will not go to sleep and the display stays on

```pwsh
sa                 # awake until you press Enter
sa --display       # awake, display on
sa --for 90m       # awake for 90 minutes, then reset
sa -d -f 1h30m     # display on for an hour and a half
```

`sa` keeps running in the foreground, drawing a live status line, and resets the execution state on the way out — whether you press <kbd>Enter</kbd>, press <kbd>Ctrl</kbd>+<kbd>C</kbd>, or `--for` runs out.

### Options

```
Usage: sa [OPTIONS]

Options:
  -d, --display          Keep the display on as well as the machine
  -f, --for <DURATION>   Stay awake for a fixed duration, then reset (30s, 45m, 2h, 1h30m)
  -q, --quiet            Print nothing but errors
  -h, --help             Print help
  -V, --version          Print version
```

A `--for` duration is a number and a unit — `s`, `m`, `h`, or `d` — and units can be chained: `45s`, `90m`, `2h`, `1h30m`, `1d 12h`. A bare number is rejected, because guessing wrong about `--for 30` means either a machine that sleeps mid-job or one that stays awake all night.

Colored output follows [`NO_COLOR`](https://no-color.org/). All output goes to stderr, so stdout stays free for redirection.

### From a script

`sa` reads <kbd>Enter</kbd> only when stdin is a terminal, so it is safe to launch from a scheduled task or a pipeline. Pair `--for` with `--quiet` to keep a long job awake and get out of the way:

```pwsh
Start-Process sa -ArgumentList '--display','--for','8h','--quiet'
```

## Does it work?

Run `sa`, then in an **elevated** PowerShell:

```pwsh
powercfg /requests
```

You should see `sa.exe` listed under `SYSTEM:` — and, in display mode, under `DISPLAY:` as well.

> [!NOTE]
> As the [Win32 documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate) says, `SetThreadExecutionState` does **not** stop you from putting the computer to sleep by closing the laptop lid or pressing the power button. The screen saver may still run.

## Develop

The crate compiles on every platform — the Win32 call is behind `cfg(windows)`, and off Windows `sa` exits with an error rather than pretending. That means the logic, the CLI, and the layout can all be tested from macOS or Linux:

```sh
cargo test
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings   # type-check the Win32 path
cargo run --example preview                                                 # draw every screen
```

## How it differs from stay-awake2

`sa` is a rewrite of [`stay-awake2`](https://github.com/curtisalexander/stay-awake2), with:

- a shorter name, and a wheel you can `uv tool install`
- `--for <DURATION>`, so a session ends on its own
- <kbd>Ctrl</kbd>+<kbd>C</kbd> handled, not just <kbd>Enter</kbd>
- a live status line instead of a static message
- a fix: `stay-awake2` exits immediately when stdin is not a terminal, because `read_line` returns EOF — which makes it unusable from a script
- execution states decoded bit by bit, rather than a fixed list of combinations that falls back to `???`

## Prior implementations

- [`stay-awake2`](https://github.com/curtisalexander/stay-awake2) — `Rust`, via the `windows` crate
- [`stay-awake-rs`](https://github.com/curtisalexander/stay-awake-rs) — `Rust`, loading `kernel32.dll` and transmuting
- [`stay-awake-cs`](https://github.com/curtisalexander/stay-awake-cs) — `C#`

## Alternatives

[Microsoft PowerToys](https://learn.microsoft.com/en-us/windows/powertoys/) ships [Awake](https://learn.microsoft.com/en-us/windows/powertoys/awake), which [also calls](https://github.com/microsoft/PowerToys/blob/main/src/modules/awake/Awake/Core/Manager.cs) `SetThreadExecutionState`.

## License

[MIT](LICENSE)
