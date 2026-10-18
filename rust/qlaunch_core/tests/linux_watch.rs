use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qlaunch_core::model::AppEntry;
use qlaunch_core::platform::linux::LinuxSource;
use qlaunch_core::platform::{AppSource, FakeSource, PlatformError, WatchHandle};
use tempfile::tempdir;

const DEBOUNCE: Duration = Duration::from_millis(250);
const QUIET: Duration = Duration::from_secs(1);
/// Far above the debounce, so a loaded machine does not trip it, but short
/// enough that a `Drop` that hangs fails the suite instead of stalling it.
const DROP_BOUND: Duration = Duration::from_secs(5);

const NEW_DESKTOP: &str = "[Desktop Entry]\nType=Application\nName=New\nExec=/bin/true\n";

fn watch(src: &LinuxSource) -> (WatchHandle, Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    let handle = src
        .watch(Box::new(move || {
            let _ = tx.send(());
        }))
        .unwrap();
    (handle, rx)
}

/// Drops the handle on another thread so that a `Drop` that deadlocks fails
/// here instead of hanging the test run.
fn drop_promptly(handle: WatchHandle) {
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        drop(handle);
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(DROP_BOUND).is_ok(),
        "dropping the handle took longer than {DROP_BOUND:?}"
    );
}

fn assert_quiet(rx: &Receiver<()>) {
    let started = Instant::now();
    assert_eq!(rx.recv_timeout(QUIET), Err(RecvTimeoutError::Timeout));
    assert!(started.elapsed() >= DEBOUNCE);
}

/// After the handle is gone, the callback (and the sender it owns) may have been
/// released too, which disconnects the channel: that is also "no callback".
fn assert_no_callback(rx: &Receiver<()>) {
    match rx.recv_timeout(QUIET) {
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
        Ok(()) => panic!("callback after the handle was dropped"),
    }
}

fn labels(src: &LinuxSource) -> Vec<String> {
    let apps: Vec<AppEntry> = src.list_apps().unwrap();
    apps.into_iter().map(|a| a.label).collect()
}

#[test]
fn creating_a_desktop_file_triggers_exactly_one_coalesced_callback() {
    let dir = tempdir().unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let (handle, rx) = watch(&src);
    assert!(labels(&src).is_empty());

    std::fs::write(dir.path().join("new.desktop"), NEW_DESKTOP).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    assert_quiet(&rx);
    assert_eq!(labels(&src), ["New"]);
    // Re-scanning reads the directory and its files; that must not itself
    // count as a change, or the launcher would re-scan forever.
    assert_quiet(&rx);

    drop_promptly(handle);
}

#[test]
fn a_burst_of_writes_is_one_callback() {
    let dir = tempdir().unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let (handle, rx) = watch(&src);

    for i in 0..5 {
        std::fs::write(dir.path().join(format!("app{i}.desktop")), NEW_DESKTOP).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    assert_quiet(&rx);
    assert_eq!(labels(&src).len(), 5);

    drop_promptly(handle);
}

#[test]
fn removing_a_desktop_file_triggers_a_callback() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("gone.desktop");
    std::fs::write(&path, NEW_DESKTOP).unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let (handle, rx) = watch(&src);

    std::fs::remove_file(&path).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    assert!(labels(&src).is_empty());

    drop_promptly(handle);
}

#[test]
fn dropping_the_handle_stops_the_callbacks() {
    let dir = tempdir().unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let (handle, rx) = watch(&src);

    drop_promptly(handle);

    std::fs::write(dir.path().join("late.desktop"), NEW_DESKTOP).unwrap();

    assert_no_callback(&rx);
}

#[test]
fn dropping_the_handle_with_a_change_pending_does_not_call_back() {
    let dir = tempdir().unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let (handle, rx) = watch(&src);

    std::fs::write(dir.path().join("new.desktop"), NEW_DESKTOP).unwrap();
    // Inside the debounce window: the event is in flight, the callback is not.
    std::thread::sleep(Duration::from_millis(50));
    drop_promptly(handle);

    // A callback already sent by the time drop returned may be waiting; any
    // callback after that would be the bug.
    while rx.try_recv().is_ok() {}
    assert_no_callback(&rx);
}

#[test]
fn a_directory_that_does_not_exist_is_not_an_error() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("no-such-applications");
    let src = LinuxSource::new_with_dirs(vec![missing.clone()]);

    let (handle, rx) = watch(&src);

    drop_promptly(handle);
    // Creating it afterwards is not noticed: nothing was being watched.
    std::fs::create_dir(&missing).unwrap();
    std::fs::write(missing.join("new.desktop"), NEW_DESKTOP).unwrap();
    assert_no_callback(&rx);
}

#[test]
fn no_directories_at_all_is_not_an_error() {
    let src = LinuxSource::new_with_dirs(Vec::new());

    let (handle, _rx) = watch(&src);

    drop_promptly(handle);
}

#[test]
fn a_missing_directory_is_skipped_when_another_can_be_watched() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("no-such-applications");
    let real = dir.path().join("applications");
    std::fs::create_dir(&real).unwrap();
    let src = LinuxSource::new_with_dirs(vec![missing, real.clone()]);
    let (handle, rx) = watch(&src);

    std::fs::write(real.join("new.desktop"), NEW_DESKTOP).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());

    drop_promptly(handle);
}

#[test]
fn a_path_that_is_a_file_cannot_be_watched() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("applications");
    std::fs::write(&file, "not a directory").unwrap();
    let src = LinuxSource::new_with_dirs(vec![file]);

    let result = src.watch(Box::new(|| {}));

    assert!(matches!(result, Err(PlatformError::Io(_))));
}

#[test]
fn a_file_is_skipped_when_another_directory_can_be_watched() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    std::fs::write(&file, "not a directory").unwrap();
    let real = dir.path().join("applications");
    std::fs::create_dir(&real).unwrap();
    let src = LinuxSource::new_with_dirs(vec![file, real.clone()]);
    let (handle, rx) = watch(&src);

    std::fs::write(real.join("new.desktop"), NEW_DESKTOP).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());

    drop_promptly(handle);
}

#[test]
fn a_source_without_watching_reports_unsupported() {
    let src = FakeSource::new(Vec::new());

    let result = AppSource::watch(&src, Box::new(|| {}));

    assert!(matches!(result, Err(PlatformError::Unsupported(_))));
}

#[test]
fn watching_works_through_a_boxed_source() {
    let dir = tempdir().unwrap();
    let src: Box<dyn AppSource> =
        Box::new(LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]));
    let (tx, rx) = mpsc::channel();
    let handle = src
        .watch(Box::new(move || {
            let _ = tx.send(());
        }))
        .unwrap();

    std::fs::write(dir.path().join("new.desktop"), NEW_DESKTOP).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());

    drop_promptly(handle);
}

#[test]
fn a_callback_may_drop_its_own_handle() {
    let dir = tempdir().unwrap();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let slot: Arc<Mutex<Option<WatchHandle>>> = Arc::new(Mutex::new(None));
    let (tx, rx) = mpsc::channel();
    let in_callback = Arc::clone(&slot);
    let handle = src
        .watch(Box::new(move || {
            drop(in_callback.lock().unwrap().take());
            let _ = tx.send(());
        }))
        .unwrap();
    *slot.lock().unwrap() = Some(handle);

    std::fs::write(dir.path().join("new.desktop"), NEW_DESKTOP).unwrap();

    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    assert!(slot.lock().unwrap().is_none());
}
