//! Single-instance guard and cross-process state locks: at most one launcher process holds the
//! data directory, and shared state writes serialize through per-file lock files.
//!
//! The launcher-level lock (`launcher.lock`) makes a second launcher refuse to start while the
//! first is alive; a stale lock left by a crashed process is detected through the owning process
//! id and reused, so there is no permanent deadlock.
//!
//! Shared-state writers use [`StateLock`], a cross-process mutual exclusion on a per-file lock
//! name, for read-modify-write cycles that must not interleave.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The launcher-level lock file's name inside the launcher's data directory.
const LAUNCHER_LOCK: &str = "launcher.lock";

/// A cross-process mutual exclusion lock on a file inside a directory.
///
/// The lock is a file named `name` inside `dir` holding the owning process id. Creating the file
/// with `create_new` is atomic, so exactly one live process can hold it; when the file already
/// exists the owner is checked through its process id and a stale lock is recycled, which keeps
/// the lock crash-safe without permanent deadlocks.
pub struct StateLock {
    lock_path: PathBuf,
}

impl StateLock {
    /// Try to acquire the cross-process lock `name` inside `dir`.
    ///
    /// Fails with a clear message when another live process already holds the lock.
    pub fn acquire(dir: &Path, name: &str) -> Result<Self, String> {
        let lock_path = dir.join(name);
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
                            "Another process (PID {shown}) holds the lock {}.",
                            lock_path.display()
                        ));
                    }
                    let _ = fs::remove_file(&lock_path);
                    if attempts > 10 {
                        return Err(format!(
                            "Could not acquire the lock {} after retries: {e}",
                            lock_path.display()
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => return Err(format!("{lock_path:?}: {e}")),
            }
        }
    }

    /// Release the lock by removing its file. A crash leaves the file, but the next holder
    /// detects the dead owner and reuses it.
    pub(crate) fn release(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        self.release();
    }
}

/// The application-level single-instance guard: at most one launcher process owns the data
/// directory. Backed by a [`StateLock`] on `launcher.lock`.
pub struct SingleInstance {
    inner: StateLock,
}

impl SingleInstance {
    /// Try to acquire the single-instance lock for the launcher's data directory.
    ///
    /// Fails with a clear message when another live launcher instance already holds the lock.
    pub fn acquire(data_dir: &Path) -> Result<Self, String> {
        // A fresh install (and a clean CI runner) may not have created the data directory yet.
        // Create it before opening launcher.lock; otherwise the first launch fails with NotFound.
        fs::create_dir_all(data_dir)
            .map_err(|e| format!("Could not create the launcher's data directory {}: {e}", data_dir.display()))?;
        Ok(Self {
            inner: StateLock::acquire(data_dir, LAUNCHER_LOCK)?,
        })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // Release the single-instance lock.
        self.inner.release();
    }
}

/// Whether a process with `pid` is still alive.
#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    } else {
        // macOS and the BSDs have no /proc: ask the kernel through kill(pid, 0).
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
}

#[cfg(not(unix))]
fn process_alive(_pid: u32) -> bool {
    // Windows has no portable liveness probe without platform APIs; treat an existing lock as
    // held so a second instance is refused, and clear the lock file after a crash manually.
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
    fn single_instance_creates_a_missing_data_directory() {
        let dir = std::env::temp_dir().join(format!(
            "horizon-single-instance-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        assert!(!dir.exists());
        let guard = SingleInstance::acquire(&dir).unwrap();
        assert!(dir.join(LAUNCHER_LOCK).exists());
        drop(guard);
        let _ = fs::remove_dir_all(dir);
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
        let lock = dir.join(LAUNCHER_LOCK);
        fs::write(&lock, "999999999\n").unwrap(); // a PID that does not exist
        assert!(SingleInstance::acquire(&dir).is_ok());
    }

    #[test]
    fn state_lock_excludes_a_second_holder_and_releases() {
        let dir = scratch("statelock");
        let first = StateLock::acquire(&dir, "shared.lock").unwrap();
        assert!(StateLock::acquire(&dir, "shared.lock").is_err());
        drop(first);
        assert!(StateLock::acquire(&dir, "shared.lock").is_ok());
    }
}
