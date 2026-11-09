//! Atomic, crash-safe persistence for launcher state files.
//!
//! State files (`accounts.json`, `instances.json`, `settings.json`, ...) are written to a
//! temporary file in the same directory, flushed to disk with `fsync`, then renamed over the
//! target. A `.bak` copy of the previous valid state is kept so a crash or corrupt write can be
//! recovered.
//!
//! Loading never silently replaces valid user data with an empty default: a missing file is
//! treated as empty, but a corrupt file returns [`LoadError::Corrupt`] so the caller can offer a
//! recovery path instead of destroying data.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Why a state file could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The file does not exist: an empty state is expected.
    Missing,
    /// The file exists but could not be read or parsed: user data may still be there.
    Corrupt(String),
    /// A filesystem error prevented reading the file.
    Io(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Missing => write!(f, "state file is missing"),
            LoadError::Corrupt(e) => write!(f, "state file is corrupt: {e}"),
            LoadError::Io(e) => write!(f, "state file cannot be read: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Atomically write `bytes` to `path`, keeping a `.bak` of the previous file.
///
/// The bytes are written to a temporary file in the same directory, synced to disk, then renamed
/// over `path`. On Windows, rename over an existing file fails, so the target is removed first
/// (a small unavoidable window; the `.bak` limits the damage).
///
/// The temporary file name carries a per-process counter and a timestamp component, so concurrent
/// writers in the same or in different processes never collide.
pub fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().unwrap_or(Path::new(""));
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    // Back up the previous valid state, if any, so a crash or corrupt write can be recovered.
    if path.exists() {
        let bak = path.with_extension("bak");
        let _ = fs::copy(path, &bak);
    }

    let tmp = random_temp_name(dir, path);

    {
        let mut file = fs::File::create(&tmp).map_err(|e| format!("{tmp:?}: {e}"))?;
        if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("{tmp:?}: {e}"));
        }
    }

    // Rename temp over target.
    if cfg!(windows) {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("{path:?}: {e}")
    })?;

    Ok(())
}

/// Atomically write a serialized value to `path`.
pub fn atomic_write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    atomic_write_bytes(path, (text + "\n").as_bytes())
}

/// Load and validate a JSON state file.
///
/// Returns `LoadError::Missing` when the file does not exist, `LoadError::Corrupt` when it
/// exists but cannot be parsed or fails validation, and `LoadError::Io` on filesystem errors.
/// A corrupt file is never silently replaced with an empty default.
pub fn load_json_validated<T, F>(path: &Path, validate: F) -> Result<T, LoadError>
where
    T: DeserializeOwned,
    F: FnOnce(&T) -> Result<(), String>,
{
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::Missing),
        Err(e) => return Err(LoadError::Io(e.to_string())),
    };
    let value: T = serde_json::from_str(&text).map_err(|e| LoadError::Corrupt(e.to_string()))?;
    validate(&value).map_err(LoadError::Corrupt)?;
    Ok(value)
}

/// Load a JSON state file with no extra validation.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T, LoadError> {
    load_json_validated(path, |_| Ok(()))
}

/// Check whether the file at `path` is present and parseable as `T`.
///
/// Used before overwriting a state file so that a corrupt original is never silently replaced
/// by an in-memory default.
pub fn is_readable<T: DeserializeOwned>(path: &Path) -> bool {
    load_json::<T>(path).is_ok()
}

/// A monotonically increasing counter for unique temporary file names inside one process.
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// A unique temporary file name in `dir`, next to `path`, for atomic writes.
///
/// Combines the process id, a timestamp and a per-process counter, so concurrent writers in the
/// same or in different processes do not collide.
fn random_temp_name(dir: &Path, path: &Path) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("state"),
        std::process::id(),
        nanos ^ seq as u128
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Thing {
        name: String,
        value: u32,
    }

    fn scratch(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("horizon-persistence-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn atomic_write_bytes_writes_and_reads_back() {
        let dir = scratch("write");
        let path = dir.join("state.json");
        atomic_write_bytes(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn atomic_write_bytes_keeps_bak_of_previous() {
        let dir = scratch("bak");
        let path = dir.join("state.json");
        fs::write(&path, "old").unwrap();
        atomic_write_bytes(&path, b"new").unwrap();
        assert!(path.with_extension("bak").exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn load_json_returns_missing_for_nonexistent_file() {
        let dir = scratch("missing");
        let path = dir.join("nope.json");
        assert!(matches!(
            load_json::<String>(&path),
            Err(LoadError::Missing)
        ));
    }

    #[test]
    fn load_json_validated_returns_corrupt_for_bad_json() {
        let dir = scratch("badjson");
        let path = dir.join("state.json");
        fs::write(&path, "{not valid json").unwrap();
        assert!(matches!(
            load_json::<Thing>(&path),
            Err(LoadError::Corrupt(_))
        ));
    }

    #[test]
    fn load_json_validated_returns_ok_for_valid_file() {
        let dir = scratch("valid");
        let path = dir.join("state.json");
        atomic_write_json(
            &path,
            &Thing {
                name: "x".into(),
                value: 7,
            },
        )
        .unwrap();
        let thing = load_json::<Thing>(&path).unwrap();
        assert_eq!(thing.name, "x");
        assert_eq!(thing.value, 7);
    }

    #[test]
    fn load_json_validated_rejects_validation_failure() {
        let dir = scratch("validation");
        let path = dir.join("state.json");
        atomic_write_json(
            &path,
            &Thing {
                name: "".into(),
                value: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            load_json_validated(&path, |t: &Thing| {
                if t.name.is_empty() {
                    Err("name must not be empty".into())
                } else {
                    Ok(())
                }
            }),
            Err(LoadError::Corrupt(_))
        ));
    }

    #[test]
    fn atomic_write_bytes_does_not_collide_between_concurrent_writers() {
        let dir = scratch("concurrent");
        let path = dir.join("state.json");
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let bytes = format!("writer-{i}");
                    atomic_write_bytes(&path, bytes.as_bytes()).unwrap()
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.starts_with("writer-"),
            "unexpected content: {content}"
        );
    }

    #[test]
    fn is_readable_is_true_for_valid_file_and_false_for_corrupt() {
        let dir = scratch("readable");
        let path = dir.join("state.json");
        assert!(!is_readable::<Thing>(&path));
        atomic_write_json(
            &path,
            &Thing {
                name: "x".into(),
                value: 1,
            },
        )
        .unwrap();
        assert!(is_readable::<Thing>(&path));
        fs::write(&path, "{not valid").unwrap();
        assert!(!is_readable::<Thing>(&path));
    }
}
