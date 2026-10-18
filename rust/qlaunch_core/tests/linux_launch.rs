use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use qlaunch_core::model::AppRef;
use qlaunch_core::platform::linux::{
    LinuxSource, build_command, is_on_path_in, terminal_emulator_from,
};
use qlaunch_core::platform::{AppSource, LaunchError};

const WAIT: Duration = Duration::from_secs(3);

fn app(id: &str) -> AppRef {
    AppRef {
        package: id.to_owned(),
        activity: String::new(),
    }
}

fn strings(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|s| (*s).to_owned()).collect()
}

/// A source over a tempdir laid out as a data root, holding `desktop` as
/// `applications/<id>`.
fn source_with(root: &Path, id: &str, desktop: &str, emulator: Option<&str>) -> LinuxSource {
    let applications = root.join("applications");
    fs::create_dir_all(&applications).unwrap();
    fs::write(applications.join(id), desktop).unwrap();
    LinuxSource::from_parts(
        vec![root.to_path_buf()],
        None,
        None,
        None,
        emulator.map(str::to_owned),
    )
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !done() {
        assert!(
            Instant::now() < deadline,
            "timed out after {WAIT:?} waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn builds_argv_with_no_shell() {
    assert_eq!(
        build_command("/usr/bin/foo --a \"b c\" %U", false, None).unwrap(),
        ["/usr/bin/foo", "--a", "b c"]
    );
}

#[test]
fn shell_metacharacters_stay_inside_their_argument() {
    assert_eq!(
        build_command("/usr/bin/foo ; rm -rf $HOME `id` | cat", false, None).unwrap(),
        strings(&[
            "/usr/bin/foo",
            ";",
            "rm",
            "-rf",
            "$HOME",
            "`id`",
            "|",
            "cat"
        ])
    );
}

#[test]
fn terminal_entries_are_wrapped_in_the_configured_terminal() {
    assert_eq!(
        build_command("/usr/bin/top", true, Some("foot")).unwrap(),
        ["foot", "-e", "/usr/bin/top"]
    );
}

#[test]
fn a_terminal_entry_keeps_its_arguments_after_the_separator() {
    assert_eq!(
        build_command("/usr/bin/top -d 1 %f", true, Some("foot")).unwrap(),
        ["foot", "-e", "/usr/bin/top", "-d", "1"]
    );
}

#[test]
fn a_terminal_entry_without_an_emulator_is_unsupported_not_silent() {
    assert!(matches!(
        build_command("/usr/bin/top", true, None),
        Err(LaunchError::Unsupported(_))
    ));
}

#[test]
fn an_empty_or_blank_emulator_is_no_emulator() {
    for emulator in ["", " ", "\t  "] {
        assert!(
            matches!(
                build_command("/usr/bin/top", true, Some(emulator)),
                Err(LaunchError::Unsupported(_))
            ),
            "emulator {emulator:?}"
        );
    }
}

#[test]
fn the_emulator_is_ignored_for_a_graphical_entry() {
    assert_eq!(
        build_command("/usr/bin/foo", false, Some("foot")).unwrap(),
        ["/usr/bin/foo"]
    );
}

#[test]
fn an_exec_with_no_program_is_an_error_not_a_spawn_attempt() {
    for exec in ["%U", "", "   ", "\"\"", "%f %F"] {
        for terminal in [false, true] {
            let result = build_command(exec, terminal, Some("foot"));
            assert!(
                matches!(result, Err(LaunchError::Failed(_))),
                "exec {exec:?}, terminal {terminal}: {result:?}"
            );
        }
    }
}

#[test]
fn an_empty_exec_is_reported_before_a_missing_emulator() {
    assert!(matches!(
        build_command("%U", true, None),
        Err(LaunchError::Failed(_))
    ));
}

#[test]
fn the_emulator_comes_from_terminal_then_x_terminal_emulator_then_nothing() {
    assert_eq!(
        terminal_emulator_from(Some("foot"), true).as_deref(),
        Some("foot")
    );
    assert_eq!(
        terminal_emulator_from(Some("foot"), false).as_deref(),
        Some("foot")
    );
    assert_eq!(
        terminal_emulator_from(None, true).as_deref(),
        Some("x-terminal-emulator")
    );
    assert_eq!(terminal_emulator_from(None, false), None);
}

#[test]
fn a_blank_terminal_variable_falls_through_to_x_terminal_emulator() {
    for blank in ["", "  ", "\t"] {
        assert_eq!(
            terminal_emulator_from(Some(blank), true).as_deref(),
            Some("x-terminal-emulator"),
            "{blank:?}"
        );
        assert_eq!(
            terminal_emulator_from(Some(blank), false),
            None,
            "{blank:?}"
        );
    }
}

#[test]
fn a_program_is_on_the_path_when_a_listed_directory_holds_it_as_a_file() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    fs::write(second.path().join("tool"), "").unwrap();
    let path = std::env::join_paths([first.path(), second.path()]).unwrap();

    assert!(is_on_path_in("tool", Some(&path)));
    assert!(!is_on_path_in("other", Some(&path)));
}

#[test]
fn a_directory_of_the_right_name_is_not_a_program() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("tool")).unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();

    assert!(!is_on_path_in("tool", Some(&path)));
}

#[test]
fn nothing_is_on_an_unset_or_empty_path() {
    assert!(!is_on_path_in("tool", None));
    assert!(!is_on_path_in("tool", Some(OsStr::new(""))));
}

#[test]
fn an_empty_path_entry_is_not_the_current_directory() {
    // Cargo runs tests from the package root, where Cargo.toml is a file: it
    // would be found if an empty entry meant ".".
    assert!(Path::new("Cargo.toml").is_file());
    for path in [":", "::", ":/nonexistent", "/nonexistent:"] {
        assert!(
            !is_on_path_in("Cargo.toml", Some(OsStr::new(path))),
            "PATH {path:?}"
        );
    }
}

#[test]
fn a_missing_directory_on_the_path_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tool"), "").unwrap();
    let path = std::env::join_paths([Path::new("/nonexistent/dir"), dir.path()]).unwrap();

    assert!(is_on_path_in("tool", Some(&path)));
}

#[test]
fn launching_starts_the_process_and_an_unknown_id_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let source = source_with(
        dir.path(),
        "marker.desktop",
        &format!(
            "[Desktop Entry]\nType=Application\nName=Marker\nExec=/bin/sh -c \"echo hi > {}\"\n",
            marker.display()
        ),
        None,
    );

    source.launch(&app("marker.desktop")).unwrap();

    wait_for("the launched process to write its marker file", || {
        fs::read_to_string(&marker).is_ok_and(|text| text == "hi\n")
    });

    let missing = app("missing.desktop");
    assert_eq!(
        source.launch(&missing),
        Err(LaunchError::NotFound(missing.clone()))
    );
}

#[test]
fn an_id_that_is_not_a_plain_file_name_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let source = source_with(
        dir.path(),
        "real.desktop",
        "[Desktop Entry]\nType=Application\nName=Real\nExec=/bin/true\n",
        None,
    );
    let escaping = app("../applications/real.desktop");

    assert_eq!(
        source.launch(&escaping),
        Err(LaunchError::NotFound(escaping.clone()))
    );
}

#[test]
fn an_entry_without_exec_cannot_be_launched() {
    let dir = tempfile::tempdir().unwrap();
    let source = source_with(
        dir.path(),
        "noexec.desktop",
        "[Desktop Entry]\nType=Application\nName=No Exec\n",
        None,
    );

    match source.launch(&app("noexec.desktop")) {
        Err(LaunchError::Failed(message)) => assert!(
            message.contains("noexec.desktop"),
            "message should name the id: {message}"
        ),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn an_entry_whose_exec_is_only_field_codes_cannot_be_launched() {
    let dir = tempfile::tempdir().unwrap();
    let source = source_with(
        dir.path(),
        "codes.desktop",
        "[Desktop Entry]\nType=Application\nName=Codes\nExec=%U\n",
        None,
    );

    assert!(matches!(
        source.launch(&app("codes.desktop")),
        Err(LaunchError::Failed(_))
    ));
}

#[test]
fn a_program_that_cannot_be_spawned_is_a_failure_carrying_the_os_message() {
    let dir = tempfile::tempdir().unwrap();
    let source = source_with(
        dir.path(),
        "gone.desktop",
        "[Desktop Entry]\nType=Application\nName=Gone\nExec=/nonexistent/qlaunch-test-binary\n",
        None,
    );

    match source.launch(&app("gone.desktop")) {
        Err(LaunchError::Failed(message)) => assert!(
            message.contains("No such file or directory"),
            "message should carry the OS error: {message}"
        ),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn a_terminal_entry_with_no_emulator_available_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let source = source_with(
        dir.path(),
        "top.desktop",
        "[Desktop Entry]\nType=Application\nName=Top\nExec=/usr/bin/top\nTerminal=true\n",
        None,
    );

    assert!(matches!(
        source.launch(&app("top.desktop")),
        Err(LaunchError::Unsupported(_))
    ));
}

#[test]
fn a_terminal_entry_is_run_through_the_source_emulator() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    // `/bin/sh` stands in for the emulator, so the entry runs as
    // `sh -e <script> x y` and the script records the arguments it received.
    // The script is read, never executed, so a file just written cannot be busy.
    let script = dir.path().join("record.sh");
    fs::write(
        &script,
        format!("printf '%s\\n' \"$*\" > {}\n", marker.display()),
    )
    .unwrap();
    let source = source_with(
        dir.path(),
        "term.desktop",
        &format!(
            "[Desktop Entry]\nType=Application\nName=Term\nExec={} x y\nTerminal=true\n",
            script.display()
        ),
        Some("/bin/sh"),
    );

    source.launch(&app("term.desktop")).unwrap();

    wait_for("the emulator to run the application", || {
        fs::read_to_string(&marker).is_ok_and(|text| text == "x y\n")
    });
}

#[cfg(target_os = "linux")]
#[test]
fn a_launched_process_that_exits_is_reaped_not_left_a_zombie() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let source = source_with(
        dir.path(),
        "pid.desktop",
        &format!(
            "[Desktop Entry]\nType=Application\nName=Pid\nExec=/bin/sh -c \"echo \\$\\$ > {}\"\n",
            pid_file.display()
        ),
        None,
    );

    source.launch(&app("pid.desktop")).unwrap();

    wait_for("the process to record its pid", || {
        fs::read_to_string(&pid_file).is_ok_and(|text| text.ends_with('\n'))
    });
    let pid: u32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .expect("the process wrote its own pid");

    // An exited but unreaped child stays in /proc as a zombie until its
    // parent waits for it; a reaped one is gone.
    wait_for("the exited process to be reaped", || {
        !Path::new(&format!("/proc/{pid}")).exists()
    });
}
