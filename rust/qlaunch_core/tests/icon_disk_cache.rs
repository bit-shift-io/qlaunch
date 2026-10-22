use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use qlaunch_core::icons::IconDiskCache;
use qlaunch_core::model::{AppRef, IconFormat, IconImage};

fn app(package: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: "Main".into(),
    }
}

fn rgba(width: u32, height: u32, fill: u8) -> IconImage {
    IconImage {
        format: IconFormat::Rgba,
        width,
        height,
        bytes: vec![fill; (width * height * 4) as usize],
    }
}

fn bin_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .collect();
    files.sort();
    files
}

#[test]
fn put_then_get_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    let image = rgba(2, 2, 7);
    cache.put(&app("a"), 48, 0, &image);
    assert_eq!(cache.get(&app("a"), 48, 0), Some(image));
}

#[test]
fn an_update_token_is_a_different_key() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    cache.put(&app("a"), 48, 1, &rgba(2, 2, 7));
    assert!(cache.get(&app("a"), 48, 1).is_some());
    assert_eq!(cache.get(&app("a"), 48, 2), None, "an update is a miss");
}

#[test]
fn a_corrupt_file_reads_as_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    let file = bin_files(dir.path()).pop().unwrap();
    std::fs::write(&file, b"QLIC but not really").unwrap();
    assert_eq!(cache.get(&app("a"), 48, 0), None);
}

#[test]
fn a_truncated_rgba_payload_reads_as_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    let file = bin_files(dir.path()).pop().unwrap();
    let mut data = std::fs::read(&file).unwrap();
    data.truncate(data.len() - 4); // a valid header, one pixel short
    std::fs::write(&file, &data).unwrap();
    assert_eq!(cache.get(&app("a"), 48, 0), None);
}

#[test]
fn a_different_key_or_size_reads_as_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    assert_eq!(cache.get(&app("b"), 48, 0), None);
    assert_eq!(cache.get(&app("a"), 96, 0), None);
}

#[test]
fn clear_removes_everything() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    cache.put(&app("b"), 48, 0, &rgba(2, 2, 7));
    cache.clear();
    assert_eq!(cache.get(&app("a"), 48, 0), None);
    assert_eq!(cache.get(&app("b"), 48, 0), None);
}

#[test]
fn a_write_from_before_a_clear_is_discarded() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 1 << 20);
    let generation = cache.generation();
    cache.clear();
    cache.put_if(generation, &app("a"), 48, 0, &rgba(2, 2, 7));
    assert_eq!(cache.get(&app("a"), 48, 0), None);
}

#[test]
fn over_the_cap_evicts_oldest_first() {
    let dir = tempfile::tempdir().unwrap();
    // Each entry is ~43 bytes; a cap of 100 fits two, so the third evicts the
    // oldest. The small sleeps make the modification times strictly ordered.
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 100);
    cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    sleep(Duration::from_millis(10));
    cache.put(&app("b"), 48, 0, &rgba(2, 2, 8));
    sleep(Duration::from_millis(10));
    cache.put(&app("c"), 48, 0, &rgba(2, 2, 9));

    assert_eq!(cache.get(&app("a"), 48, 0), None, "oldest was evicted");
    assert!(cache.get(&app("b"), 48, 0).is_some());
    assert!(cache.get(&app("c"), 48, 0).is_some());
}

#[test]
fn replacement_and_restart_account_for_existing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 100);
    for _ in 0..10 {
        cache.put(&app("a"), 48, 0, &rgba(2, 2, 7));
    }
    cache.put(&app("b"), 48, 0, &rgba(2, 2, 8));
    assert_eq!(bin_files(dir.path()).len(), 2);
    let restarted = IconDiskCache::new(dir.path().to_path_buf(), 100);
    restarted.put(&app("c"), 48, 0, &rgba(2, 2, 9));
    let total: u64 = bin_files(dir.path())
        .iter()
        .map(|p| std::fs::metadata(p).unwrap().len())
        .sum();
    assert!(total <= 100);
    assert!(restarted.get(&app("c"), 48, 0).is_some());
}

#[test]
fn cloned_caches_serialize_writes_and_invalidation() {
    let dir = tempfile::tempdir().unwrap();
    let cache = IconDiskCache::new(dir.path().to_path_buf(), 4096);
    let threads: Vec<_> = (0..12)
        .map(|i| {
            let cache = cache.clone();
            std::thread::spawn(move || {
                for n in 0..30 {
                    cache.put(&app("shared"), 48, 0, &rgba(2, 2, i));
                    cache.put(&app(&format!("app{i}")), 48, 0, &rgba(2, 2, n));
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert!(cache.get(&app("shared"), 48, 0).is_some());
    let old = cache.generation();
    cache.clear();
    cache.put_if(old, &app("shared"), 48, 0, &rgba(2, 2, 0));
    assert!(bin_files(dir.path()).is_empty());
}
