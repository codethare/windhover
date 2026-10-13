//! Entry point: argument parsing, wiring, and exit codes.

mod config;
mod hint;
mod keys;
mod mode;
mod render;
mod session;
mod wl;

use std::path::PathBuf;
use std::process::ExitCode;

use session::Mode;

const HELP: &str = "\
windhover — keyboard-driven mouse cursor for wlroots-based Wayland compositors

USAGE:
    windhover [OPTIONS]

OPTIONS:
    -n, --normal        start in normal mode (default)
    -g, --grid          start in grid mode
    -x, --hint          start in hint mode
    -o, --oneshot       exit when the mode hands over, instead of dropping into
                        normal mode (grid and hint only)
    -c, --config PATH   read this configuration file instead of the default
    -h, --help          print this help and exit
    -V, --version       print the version and exit

Bind one of the mode flags to a compositor hotkey; see the README for sway,
niri and river snippets. Wayland does not allow clients to bind global hotkeys,
so there is no daemon and no configuration option for one.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("windhover: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    match Command::parse(std::env::args().skip(1))? {
        Command::Help => {
            print!("{HELP}");
            Ok(())
        }
        Command::Version => {
            println!("windhover {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Start {
            mode,
            config,
            oneshot,
        } => session::run(mode, config.as_deref(), oneshot),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Start {
        mode: Mode,
        config: Option<PathBuf>,
        oneshot: bool,
    },
}

impl Command {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut start = None;
        let mut config = None;
        let mut oneshot = false;
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(Self::Help),
                "-V" | "--version" => return Ok(Self::Version),
                "-n" | "--normal" => start = Some(Mode::Normal),
                "-g" | "--grid" => start = Some(Mode::Grid),
                "-x" | "--hint" => start = Some(Mode::Hint),
                "-o" | "--oneshot" => oneshot = true,
                "-c" | "--config" => {
                    let path = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("`{arg}` needs a path"))?;
                    config = Some(PathBuf::from(path));
                }
                other => anyhow::bail!("unrecognised argument `{other}` (try --help)"),
            }
        }
        // Last mode flag wins, so `windhover --grid` and `windhover --normal --grid`
        // both mean grid.
        let mode = start.unwrap_or(Mode::Normal);
        // Normal mode never hands over, so the flag would do nothing at all.
        if oneshot && mode == Mode::Normal {
            anyhow::bail!("`--oneshot` needs a mode that hands over: use it with --grid or --hint");
        }
        Ok(Self::Start {
            mode,
            config,
            oneshot,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> anyhow::Result<Command> {
        Command::parse(args.iter().map(|arg| arg.to_string()))
    }

    fn start(mode: Mode) -> Command {
        Command::Start {
            mode,
            config: None,
            oneshot: false,
        }
    }

    #[test]
    fn defaults_to_normal() {
        assert_eq!(parse(&[]).unwrap(), start(Mode::Normal));
    }

    #[test]
    fn reads_every_mode_flag() {
        assert_eq!(parse(&["--grid"]).unwrap(), start(Mode::Grid));
        assert_eq!(parse(&["-x"]).unwrap(), start(Mode::Hint));
        assert_eq!(parse(&["--normal"]).unwrap(), start(Mode::Normal));
    }

    #[test]
    fn later_mode_flag_wins() {
        assert_eq!(parse(&["--normal", "--grid"]).unwrap(), start(Mode::Grid));
    }

    #[test]
    fn help_and_version_win_over_mode() {
        assert_eq!(parse(&["--grid", "--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
    }

    #[test]
    fn a_config_flag_is_not_a_mode() {
        assert_eq!(
            parse(&["--config", "/dev/null"]).unwrap(),
            Command::Start {
                mode: Mode::Normal,
                config: Some(PathBuf::from("/dev/null")),
                oneshot: false,
            }
        );
    }

    #[test]
    fn reads_the_config_flag() {
        assert_eq!(
            parse(&["--config", "/tmp/x"]).unwrap(),
            Command::Start {
                mode: Mode::Normal,
                config: Some(PathBuf::from("/tmp/x")),
                oneshot: false,
            }
        );
        assert_eq!(
            parse(&["-c", "/tmp/x", "--hint"]).unwrap(),
            Command::Start {
                mode: Mode::Hint,
                config: Some(PathBuf::from("/tmp/x")),
                oneshot: false,
            }
        );
    }

    #[test]
    fn a_config_flag_without_a_path_is_rejected() {
        assert!(parse(&["--config"]).is_err());
    }

    #[test]
    fn rejects_unknown_arguments() {
        let err = parse(&["--nope"]).unwrap_err().to_string();
        assert!(err.contains("--nope"), "unexpected message: {err}");
    }

    #[test]
    fn oneshot_is_read_and_kept_with_the_mode() {
        assert_eq!(
            parse(&["--hint", "--oneshot"]).unwrap(),
            Command::Start {
                mode: Mode::Hint,
                config: None,
                oneshot: true,
            }
        );
    }

    #[test]
    fn oneshot_with_normal_mode_is_refused() {
        // Normal mode never hands over, so there is nothing to end the session.
        let err = parse(&["--normal", "--oneshot"]).unwrap_err().to_string();
        assert!(err.contains("--oneshot"), "unexpected message: {err}");
        assert!(parse(&["--oneshot"]).is_err(), "the default mode is normal");
        assert!(parse(&["--grid", "--oneshot"]).is_ok());
    }
}
