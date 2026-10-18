// A separate file so this test is the only one in its binary: it mutates the process environment, which would race any test running beside it.
use std::path::PathBuf;

use qlaunch_core::platform::linux::dirs_to_scan;

#[test]
fn dirs_to_scan_reads_the_environment() {
    // SAFETY: this is the only test in this binary, so nothing else reads or
    // writes the environment while it runs.
    unsafe {
        std::env::set_var("HOME", "/tmp/home-test");

        std::env::set_var("XDG_DATA_HOME", "/tmp/data-home");
        std::env::set_var("XDG_DATA_DIRS", "/tmp/a:/tmp/b");
        assert_eq!(
            dirs_to_scan(),
            [
                PathBuf::from("/tmp/data-home/applications"),
                PathBuf::from("/tmp/a/applications"),
                PathBuf::from("/tmp/b/applications"),
            ]
        );

        std::env::set_var("XDG_DATA_HOME", "");
        std::env::set_var("XDG_DATA_DIRS", "");
        let defaults = [
            PathBuf::from("/tmp/home-test/.local/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/usr/share/applications"),
        ];
        assert_eq!(dirs_to_scan(), defaults);

        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_DATA_DIRS");
        assert_eq!(dirs_to_scan(), defaults);
    }
}
