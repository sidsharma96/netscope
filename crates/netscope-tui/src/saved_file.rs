//! Bounded file reads and replacement only after a complete write.
use netscope_core::{SAVED_FILE_LIMIT, SavedDraft};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

/// Read one local draft without contacting its destination.
/// # Errors
/// Returns bounded, content-free explanations for I/O or validation failures.
pub fn load(path: &Path) -> Result<SavedDraft, &'static str> {
    if !fs::metadata(path)
        .map_err(|_| "Could not inspect saved draft")?
        .is_file()
    {
        return Err("Saved draft must be a regular file");
    }
    let file = File::open(path).map_err(|_| "Could not open saved draft")?;
    if !file
        .metadata()
        .map_err(|_| "Could not inspect saved draft")?
        .is_file()
    {
        return Err("Saved draft must be a regular file");
    }
    let mut bytes = Vec::new();
    file.take(SAVED_FILE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read saved draft")?;
    SavedDraft::decode(&bytes)
}
/// Replace the chosen file with a fully written, synchronized temporary file.
/// # Errors
/// Leaves the old destination intact if writing or replacement fails.
pub fn save(path: &Path, saved: &SavedDraft) -> Result<(), &'static str> {
    let bytes = saved.encode()?;
    replace(path, |file| {
        file.write_all(bytes.as_bytes())?;
        file.sync_all()
    })
    .map_err(|_| "Save failed; check the path, permissions, or a leftover .netscope-tmp file")
}
fn replace(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(io::Error::other("Destination must be a regular file"));
        }
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("Missing filename"))?;
    let mut temporary_name = name.to_os_string();
    temporary_name.push(".netscope-tmp");
    let temporary = path.with_file_name(temporary_name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    let result = write(&mut file);
    drop(file);
    let result = result.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_files_round_trip_and_reads_reject_invalid_input()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("netscope-load-{}", std::process::id()));
        fs::create_dir(&root)?;
        let path = root.join("book.json");
        let saved = SavedDraft::decode(include_bytes!("../../../tests/fixtures/drafts/book.json"))?;
        save(&path, &saved)?;
        assert_eq!(load(&path)?.encode()?, saved.encode()?);
        fs::write(&path, b"malformed private content")?;
        let error = load(&path).err().ok_or("accepted invalid JSON")?;
        assert!(!error.contains("private"));
        fs::write(&path, vec![b' '; SAVED_FILE_LIMIT + 1])?;
        assert!(load(&path).is_err());
        assert!(load(&root).is_err());
        assert!(save(&root, &saved).is_err());
        #[cfg(unix)]
        {
            let link = root.join("link.json");
            std::os::unix::fs::symlink(&path, &link)?;
            assert!(save(&link, &saved).is_err());
            assert_eq!(fs::metadata(&path)?.len(), (SAVED_FILE_LIMIT + 1) as u64);
        }
        fs::remove_dir_all(root)?;
        Ok(())
    }
    #[test]
    fn interrupted_write_preserves_previous_file() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("netscope-save-{}", std::process::id()));
        fs::create_dir(&root)?;
        let path = root.join("draft.json");
        fs::write(&path, b"old")?;
        let result = replace(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path)?, b"old");
        assert!(!root.join("draft.json.netscope-tmp").exists());
        replace(&path, |file| file.write_all(b"complete"))?;
        assert_eq!(fs::read(&path)?, b"complete");
        fs::write(root.join("draft.json.netscope-tmp"), b"another writer")?;
        assert!(replace(&path, |file| file.write_all(b"wrong")).is_err());
        assert_eq!(fs::read(&path)?, b"complete");
        assert_eq!(
            fs::read(root.join("draft.json.netscope-tmp"))?,
            b"another writer"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);
        }
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
