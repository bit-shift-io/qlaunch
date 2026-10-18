// A separate file so this test is the only one in its binary: it mutates the process environment, which would race any test running beside it.
use std::path::PathBuf;

use qlaunch_core::store::config_root;

#[test]
fn config_root_reads_the_environment() {
    // SAFETY: this is the only test in this binary, so nothing else reads or
    // writes the environment while it runs.
    unsafe {
        std::env::set_var("HOME", "/tmp/home-test");

        std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test");
        assert_eq!(
            config_root().unwrap(),
            PathBuf::from("/tmp/xdg-test/bitshift/qlaunch")
        );

        std::env::set_var("XDG_CONFIG_HOME", "");
        assert_eq!(
            config_root().unwrap(),
            PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch")
        );

        std::env::set_var("XDG_CONFIG_HOME", "relative");
        assert_eq!(
            config_root().unwrap(),
            PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch")
        );

        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            config_root().unwrap(),
            PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch")
        );
    }
}
