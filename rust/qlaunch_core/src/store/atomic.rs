use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Replaces `path` with `bytes` so that a reader, or a crash, sees either the
/// old contents or the new ones and never a mixture: write a temp file next to
/// the target (the same directory, so the rename stays on one filesystem),
/// flush it to disk, rename it over the target, then flush the directory so the
/// rename itself survives a power cut.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let (dir, name) = match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => (dir, name),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a file path",
            ));
        }
    };
    let tmp = dir.join(format!(".{}.tmp", name.to_string_lossy()));

    let written = write_and_sync(&tmp, bytes).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written?;
    sync_dir(dir)
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(unix)]
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
pub(crate) fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}
