use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use uuid::Uuid;

/// Replaces `path` with `bytes` so that a reader, or a crash, sees either the
/// old contents or the new ones and never a mixture: write a temp file next to
/// the target (the same directory, so the rename stays on one filesystem),
/// flush it to disk, rename it over the target, then flush the directory so the
/// rename itself survives a power cut.
///
/// The temp name is unique to each call (`.<name>.<uuid>.tmp`) and is opened
/// with `create_new`, so two overlapping writers, in this process or another,
/// never share or truncate each other's temp file; a name collision is an
/// error, not a silent truncation. The document is therefore always one
/// writer's complete output. Which writer wins is not defined: the last rename
/// stands and the other writer's changes are lost. That is accepted. There is
/// no lock; none could serialise separate processes, and a lock inside a
/// `Clone`able `Store` would not serialise the clones.
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
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));

    // Not cleaned up if this fails: a failed `create_new` means the file is
    // someone else's.
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let written = write_and_sync(&mut file, bytes).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written?;
    sync_dir(dir)
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> io::Result<()> {
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
