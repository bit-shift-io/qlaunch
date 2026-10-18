use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::platform::{PlatformError, WatchHandle};

/// How long the directories must stay quiet before the callback fires.
const DEBOUNCE: Duration = Duration::from_millis(250);

/// Calls `on_change` once per burst of changes to `dirs`.
///
/// A directory that does not exist is skipped, and if none exists the handle
/// is inert. If some exist but none can be watched, that is an error: the
/// launcher would otherwise never hear about a new application.
///
/// `on_change` runs on the debounce thread and is not guarded: if it panics
/// the thread ends and no further notifications are sent.
pub(super) fn watch(
    dirs: &[PathBuf],
    on_change: Box<dyn Fn() + Send + Sync>,
) -> Result<WatchHandle, PlatformError> {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();
    for dir in dirs {
        match std::fs::metadata(dir) {
            Ok(meta) if meta.is_dir() => candidates.push(dir),
            Ok(_) => failures.push(format!("{}: not a directory", dir.display())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => failures.push(format!("{}: {e}", dir.display())),
        }
    }
    if candidates.is_empty() {
        return if failures.is_empty() {
            Ok(WatchHandle::new(|| {}))
        } else {
            Err(unwatchable(&failures))
        };
    }

    let (tx, rx) = mpsc::channel();
    // Opening and reading a file is an `Access` event, and re-scanning does
    // exactly that, so forwarding them would make every callback cause the next.
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        if !matches!(&event, Ok(e) if matches!(e.kind, EventKind::Access(_))) {
            let _ = tx.send(());
        }
    })
    .map_err(|e| PlatformError::Io(format!("could not start watching: {e}")))?;

    let mut watched = 0;
    for dir in candidates {
        match watcher.watch(dir, RecursiveMode::NonRecursive) {
            Ok(()) => watched += 1,
            Err(e) => failures.push(format!("{}: {e}", dir.display())),
        }
    }
    if watched == 0 {
        return Err(unwatchable(&failures));
    }

    let thread = thread::Builder::new()
        .name("qlaunch-watch-debounce".into())
        .spawn(move || debounce(rx, on_change))
        .map_err(|e| PlatformError::Io(format!("could not start the watch thread: {e}")))?;

    Ok(stop_on_drop(watcher, thread))
}

fn unwatchable(failures: &[String]) -> PlatformError {
    PlatformError::Io(format!(
        "could not watch the applications directories: {}",
        failures.join("; ")
    ))
}

/// Blocks for a first event, then fires `on_change` once the channel has been
/// quiet for [`DEBOUNCE`]. Returns when the channel disconnects, dropping a
/// pending change rather than reporting it after the watcher is gone.
fn debounce(events: Receiver<()>, on_change: Box<dyn Fn() + Send + Sync>) {
    while events.recv().is_ok() {
        loop {
            match events.recv_timeout(DEBOUNCE) {
                Ok(()) => {}
                Err(RecvTimeoutError::Timeout) => {
                    on_change();
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

fn stop_on_drop(watcher: RecommendedWatcher, thread: JoinHandle<()>) -> WatchHandle {
    let running = Mutex::new(Some((watcher, thread)));
    WatchHandle::new(move || {
        let taken = running.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some((watcher, thread)) = taken else {
            return;
        };
        // The watcher goes first. Dropping it makes notify's own thread exit,
        // and that drops the event handler and with it the only `Sender`, so
        // the debounce thread's `recv`/`recv_timeout` returns `Disconnected`.
        // That disconnect is what wakes the thread and tells it to stop; there
        // is no separate flag. Notify's `Drop` can panic if its thread already
        // died, which must not escape this closure.
        let _ = panic::catch_unwind(AssertUnwindSafe(|| drop(watcher)));
        // This waits for that disconnect, and for a callback already running,
        // so it ends within a debounce window of the watcher going. Joining
        // from the debounce thread itself (a callback dropping the handle)
        // would wait forever, and a panicked thread is not our panic.
        if thread.thread().id() != thread::current().id() {
            let _ = thread.join();
        }
    })
}
