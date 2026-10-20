use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use crate::model::{AppRef, IconFormat, IconImage};

const MAGIC: &[u8; 4] = b"QLIC";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 1 + 4 + 4 + 4;

/// A cache of icons on disk, keyed by `(package, activity, size_px)`, so a
/// restart does not ask the platform for icons it already has.
///
/// An entry stores the key and a small header beside the bytes, so a hash
/// collision, a truncated write or a future format reads as a miss rather than
/// as the wrong image. Every I/O failure is ignored: the cache is an
/// optimisation, never a correctness requirement.
#[derive(Clone)]
pub struct IconDiskCache {
    dir: PathBuf,
    cap_bytes: u64,
    /// Bumped by [`clear`](Self::clear). A fetch captures it before asking the
    /// platform and only writes back if it is unchanged, so an icon that was
    /// already in flight when the apps changed cannot resurrect a stale entry.
    generation: Arc<AtomicU64>,
}

impl IconDiskCache {
    pub fn new(dir: PathBuf, cap_bytes: u64) -> Self {
        Self {
            dir,
            cap_bytes,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// The current generation, to pass to [`put_if`](Self::put_if) after a
    /// platform fetch.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// The cached image for `app` at `size_px` produced by an app whose
    /// `update_time` matches, or `None` on any miss or I/O failure. Never panics.
    pub fn get(&self, app: &AppRef, size_px: u32, update_time: i64) -> Option<IconImage> {
        let key = key_of(app, size_px, update_time);
        let data = fs::read(self.dir.join(file_name(&key))).ok()?;
        decode(&data, &key)
    }

    /// Stores `image` for `app` at `size_px`. A failed write is ignored.
    pub fn put(&self, app: &AppRef, size_px: u32, update_time: i64, image: &IconImage) {
        self.write(app, size_px, update_time, image);
    }

    /// [`put`](Self::put), but only if the cache has not been cleared since
    /// `generation` was taken.
    pub fn put_if(
        &self,
        generation: u64,
        app: &AppRef,
        size_px: u32,
        update_time: i64,
        image: &IconImage,
    ) {
        if self.generation() == generation {
            self.write(app, size_px, update_time, image);
        }
    }

    fn write(&self, app: &AppRef, size_px: u32, update_time: i64, image: &IconImage) {
        let key = key_of(app, size_px, update_time);
        let mut data = Vec::with_capacity(HEADER_LEN + key.len() + image.bytes.len());
        data.extend_from_slice(MAGIC);
        data.push(VERSION);
        data.push(format_byte(image.format));
        data.extend_from_slice(&image.width.to_le_bytes());
        data.extend_from_slice(&image.height.to_le_bytes());
        data.extend_from_slice(&(key.len() as u32).to_le_bytes());
        data.extend_from_slice(key.as_bytes());
        data.extend_from_slice(&image.bytes);

        if fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        // Write-then-rename: a crash or a full disk leaves either the old file
        // or the new one, never a header with a truncated payload.
        let name = file_name(&key);
        let temp = self.dir.join(format!("{name}.{}.tmp", std::process::id()));
        if fs::write(&temp, &data).is_err() {
            let _ = fs::remove_file(&temp);
            return;
        }
        if fs::rename(&temp, self.dir.join(name)).is_err() {
            let _ = fs::remove_file(&temp);
            return;
        }
        self.trim();
    }

    /// Removes every cached file and invalidates any fetch already in flight.
    /// Called when the installed apps change, since their icons may have too.
    pub fn clear(&self) {
        // Bump before removing: a `put_if` that already read the old generation
        // will then decline to write.
        self.generation.fetch_add(1, Ordering::SeqCst);
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            if is_bin(&entry.path()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    /// Deletes the oldest files until the directory is at or under the cap.
    fn trim(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
            .flatten()
            .filter(|entry| is_bin(&entry.path()))
            .filter_map(|entry| {
                let meta = entry.metadata().ok()?;
                Some((meta.modified().ok()?, meta.len(), entry.path()))
            })
            .collect();
        let mut total: u64 = files.iter().map(|(_, len, _)| len).sum();
        if total <= self.cap_bytes {
            return;
        }
        let oldest_first = |a: &(SystemTime, u64, PathBuf), b: &(SystemTime, u64, PathBuf)| {
            a.0.cmp(&b.0)
        };
        files.sort_by(oldest_first);
        for (_, len, path) in files {
            if total <= self.cap_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(len);
            }
        }
    }
}

fn key_of(app: &AppRef, size_px: u32, update_time: i64) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        app.package, app.activity, size_px, update_time
    )
}

fn file_name(key: &str) -> String {
    format!("{:016x}.bin", fnv1a(key.as_bytes()))
}

fn is_bin(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|extension| extension == "bin")
}

/// FNV-1a: a small, stable hash for the file name. Stability across Rust
/// versions is why this is not `DefaultHasher`.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn format_byte(format: IconFormat) -> u8 {
    match format {
        IconFormat::Png => 0,
        IconFormat::Rgba => 1,
    }
}

fn format_from_byte(byte: u8) -> Option<IconFormat> {
    match byte {
        0 => Some(IconFormat::Png),
        1 => Some(IconFormat::Rgba),
        _ => None,
    }
}

fn decode(data: &[u8], expected_key: &str) -> Option<IconImage> {
    if data.len() < HEADER_LEN || &data[0..4] != MAGIC.as_slice() || data[4] != VERSION {
        return None;
    }
    let format = format_from_byte(data[5])?;
    let width = u32::from_le_bytes(data[6..10].try_into().ok()?);
    let height = u32::from_le_bytes(data[10..14].try_into().ok()?);
    let key_len = u32::from_le_bytes(data[14..18].try_into().ok()?) as usize;
    let key_end = HEADER_LEN.checked_add(key_len)?;
    if data.len() < key_end || &data[HEADER_LEN..key_end] != expected_key.as_bytes() {
        return None;
    }
    let bytes = data[key_end..].to_vec();
    // A raw image's payload must match its dimensions exactly; a truncated
    // write must read as a miss, not reach the image decoder.
    if format == IconFormat::Rgba {
        let pixels = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?;
        if pixels == 0 || bytes.len() != pixels.checked_mul(4)? {
            return None;
        }
    }
    Some(IconImage {
        format,
        width,
        height,
        bytes,
    })
}
