use qlaunch_core::platform::AppSource;
use qlaunch_core::platform::linux::LinuxSource;
use tempfile::tempdir;

fn source_with_downloads(dir: &tempfile::TempDir) -> LinuxSource {
    LinuxSource::new_with_dirs(Vec::new()).with_downloads(Some(dir.path().to_path_buf()))
}

#[test]
fn backups_round_trip_through_a_downloads_folder() {
    let dir = tempdir().unwrap();
    let source = source_with_downloads(&dir);

    assert!(source.list_backups().unwrap().is_empty());

    source
        .write_backup("qlaunch-backup-2026-10-20-090000.json", br#"{"old": true}"#)
        .unwrap();
    source
        .write_backup("qlaunch-backup-2026-10-21-140530.json", br#"{"new": true}"#)
        .unwrap();
    // A Downloads folder holds everything else too; the listing must ignore it.
    std::fs::write(dir.path().join("holiday.jpg"), b"not json").unwrap();

    assert_eq!(
        source.list_backups().unwrap(),
        [
            "qlaunch-backup-2026-10-21-140530.json",
            "qlaunch-backup-2026-10-20-090000.json",
        ]
    );
    assert_eq!(
        source
            .read_backup("qlaunch-backup-2026-10-21-140530.json")
            .unwrap(),
        br#"{"new": true}"#
    );
}

#[test]
fn a_name_that_is_not_a_backup_is_refused() {
    let dir = tempdir().unwrap();
    let source = source_with_downloads(&dir);

    assert!(source.write_backup("../escape.json", b"x").is_err());
    assert!(
        source
            .read_backup("qlaunch-backup-2026-10-21-140530.txt")
            .is_err()
    );
    assert!(!dir.path().join("escape.json").exists());
}

#[test]
fn a_missing_downloads_folder_is_an_empty_listing_not_an_error() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("never-created");
    let source = LinuxSource::new_with_dirs(Vec::new()).with_downloads(Some(missing));

    assert!(source.list_backups().unwrap().is_empty());
}
