use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::platform::LaunchError;

use super::exec_argv;

/// The argv that starts an `Exec` line: its arguments as [`exec_argv`] splits
/// them, preceded by `<emulator> -e` for a terminal application. Nothing here
/// involves a shell.
///
/// An `Exec` that leaves no program (empty, or nothing but field codes) is
/// [`LaunchError::Failed`]. A terminal application with no emulator, or a
/// blank one, is [`LaunchError::Unsupported`].
pub fn build_command(
    exec: &str,
    terminal: bool,
    terminal_emulator: Option<&str>,
) -> Result<Vec<String>, LaunchError> {
    let argv = exec_argv(exec);
    if argv.first().is_none_or(|program| program.is_empty()) {
        return Err(LaunchError::Failed(format!(
            "the Exec line {exec:?} names no program to run"
        )));
    }
    if !terminal {
        return Ok(argv);
    }
    let emulator = terminal_emulator
        .map(str::trim)
        .filter(|emulator| !emulator.is_empty())
        .ok_or_else(|| {
            LaunchError::Unsupported(
                "no terminal emulator is available for this application".into(),
            )
        })?;
    let mut wrapped = vec![emulator.to_owned(), "-e".to_owned()];
    wrapped.extend(argv);
    Ok(wrapped)
}

/// The terminal emulator to use: `$TERMINAL` if it is not blank, else
/// `x-terminal-emulator` if that is on `PATH`, else none.
pub fn terminal_emulator_from(
    env_terminal: Option<&str>,
    x_terminal_emulator_on_path: bool,
) -> Option<String> {
    env_terminal
        .map(str::trim)
        .filter(|terminal| !terminal.is_empty())
        .map(str::to_owned)
        .or_else(|| x_terminal_emulator_on_path.then(|| "x-terminal-emulator".to_owned()))
}

/// Whether `name` is a file in one of the directories of `path`, a
/// `:`-separated list. Empty entries are ignored: they conventionally mean the
/// current directory, which must not decide what the launcher runs.
pub fn is_on_path_in(name: &str, path: Option<&OsStr>) -> bool {
    path.into_iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| !dir.as_os_str().is_empty())
        .any(|dir| dir.join(name).is_file())
}

/// [`is_on_path_in`] over the environment's `PATH`.
pub fn is_on_path(name: &str) -> bool {
    is_on_path_in(name, std::env::var_os("PATH").as_deref())
}

/// The terminal emulator the environment provides, as
/// [`terminal_emulator_from`] decides it.
pub fn terminal_emulator() -> Option<String> {
    terminal_emulator_from(
        std::env::var("TERMINAL").ok().as_deref(),
        is_on_path("x-terminal-emulator"),
    )
}

/// Starts `argv` without waiting for it, with no standard streams.
///
/// The child is handed to a thread that waits for it and ignores how it ended:
/// dropping a `Child` does not reap it, so without this every application the
/// launcher ever started would stay a zombie until the launcher exits.
pub(super) fn spawn(argv: &[String]) -> Result<(), LaunchError> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| LaunchError::Failed("there is no program to run".into()))?;
    let mut child = Command::new(Path::new(program))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| LaunchError::Failed(format!("could not start {program}: {e}")))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
