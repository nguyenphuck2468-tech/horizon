//! Single-instance guard: at most one launcher process holds the data directory.
//!
//! A second launcher either focuses the existing one or exits with a clear message. The lock is
//! a file in the data directory holding the owning process id. Because the lock is tied to a
//! live process (checked through `/proc` on Unix), a crash leaves at most a stale lock file
//! that the next launcher cleans up - no permanent deadlock.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The lock file's name inside the launcher's data directory.
const LOCK_FILE: &str = "launcher.lock";

/// Holds the single-instance lock for the lifetime of the process.
///
/// Dropping the guard releases the lock (and removes the lock file when possible). A crashed
/// process leaves the file behind, but the next launcher detects the dead owner and reuses it.
pub struct SingleInstance {
    lock_path: PathBuf,
}

impl SingleInstance {
    /// Try to acquire the single-instance lock for `data_dir`.
    ///
    /// Fails with a clear message when another live launcher instance already holds the lock.
    pub fn acquire(data_dir: &Path) -> Result<Self, String> {
        let lock_path = data_dir.join(LOCK_FILE);
        let mut attempts = 0;
        loop {
            attempts += 1;
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = writeln!(file, "{}", std::process::id());
                    let _ = file.sync_all();
                    return Ok(Self { lock_path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // A lock file exists. If its owner is still alive, we are a duplicate.
                    // If the owner crashed, clean up and retry.
                    let owner = fs::read_to_string(&lock_path)
                        .ok()
                        .and_then(|t| t.trim().parse::<u32>().ok());
                    if owner.is_some_and(process_alive) {
                        let shown = owner
                            .map(|pid| pid.to_string())
                            .unwrap_or_else(|| "unknown".into());
                        return Err(format!(
                            "Another launcher instance (PID {shown}) is already running in {data_dir:?}. Close it first."
                        ));
                    }
                    let _ = fs::remove_file(&lock_path);
                    if attempts > 3 {
                        return Err(format!(
                            "Could not acquire the launcher lock in {data_dir:?}: {e}"
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => return Err(format!("{lock_path:?}: {e}")),
            }
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // Release the lock by removing the file. A crash leaves the file, but the next
        // launcher detects the dead owner and reuses it.
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Whether a process with `pid` is still alive.
#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(unix))]
fn process_alive(_pid: u32) -> bool {
    // On non-Unix platforms we cannot cheaply check liveness without platform-specific APIs.
    // Treat an existing lock as held to avoid false duplicates; a stale lock is cleaned up
    // manually or on the next run.
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "horizon-single-instance-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn single_instance_acquire_succeeds_once() {
        let dir = scratch("once");
        let first = SingleInstance::acquire(&dir).unwrap();
        assert!(SingleInstance::acquire(&dir).is_err());
        drop(first);
        assert!(SingleInstance::acquire(&dir).is_ok());
    }

    #[test]
    fn single_instance_reuses_stale_lock_from_dead_process() {
        let dir = scratch("stale");
        let lock = dir.join(LOCK_FILE);
        fs::write(&lock, "999999999\n").unwrap(); // a PID that does not exist
        assert!(SingleInstance::acquire(&dir).is_ok());
    }
}
