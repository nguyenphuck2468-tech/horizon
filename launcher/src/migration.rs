//! Safe, preview-first migration from a regular per-user install into a portable bundle.
//!
//! Migration is deliberately additive: existing destination files are never overwritten. Symbolic
//! links are skipped rather than followed, and source/destination trees must not overlap.
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreePreview {
    pub files: u64,
    pub bytes: u64,
    pub conflicts: u64,
    pub skipped_symlinks: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preview {
    pub launcher: TreePreview,
    pub game: TreePreview,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeResult {
    pub copied: u64,
    pub skipped_existing: u64,
    pub skipped_symlinks: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResultSummary {
    pub launcher: TreeResult,
    pub game: TreeResult,
}

#[derive(Clone, Debug)]
pub struct Migration {
    pub launcher_source: PathBuf,
    pub launcher_destination: PathBuf,
    pub game_source: PathBuf,
    pub game_destination: PathBuf,
}

impl Migration {
    /// The normal install's data folders, if the platform has a per-user data directory.
    /// Callers should only offer this migration while running from a portable bundle.
    pub fn from_user_data(portable_data: &Path) -> Option<Self> {
        let user_data = dirs::data_dir()?;
        Some(Self {
            launcher_source: user_data.join("horizon-launcher"),
            launcher_destination: portable_data.to_path_buf(),
            game_source: user_data.join("horizon"),
            game_destination: portable_data.join("horizon"),
        })
    }

    /// Inspect both trees without writing anything. Missing sources are treated as empty.
    pub fn preview(&self) -> Result<Preview, String> {
        validate_pair(&self.launcher_source, &self.launcher_destination)?;
        validate_pair(&self.game_source, &self.game_destination)?;
        let mut preview = Preview::default();
        inspect_tree(
            &self.launcher_source,
            &self.launcher_destination,
            &mut preview.launcher,
        )?;
        inspect_tree(&self.game_source, &self.game_destination, &mut preview.game)?;
        Ok(preview)
    }

    /// Copy only files that are still absent at the destination. Existing files are never replaced.
    /// The preview is recalculated at execution time to avoid relying on stale UI state.
    pub fn execute(&self) -> Result<ResultSummary, String> {
        validate_pair(&self.launcher_source, &self.launcher_destination)?;
        validate_pair(&self.game_source, &self.game_destination)?;
        let mut result = ResultSummary::default();
        copy_tree(
            &self.launcher_source,
            &self.launcher_destination,
            &mut result.launcher,
        )?;
        copy_tree(&self.game_source, &self.game_destination, &mut result.game)?;
        Ok(result)
    }
}

fn validate_pair(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    let source_metadata =
        fs::symlink_metadata(source).map_err(|e| format!("{}: {e}", source.display()))?;
    if !source_metadata.file_type().is_dir() {
        return Err(format!("{} is not a regular folder", source.display()));
    }
    let source = fs::canonicalize(source).map_err(|e| format!("{}: {e}", source.display()))?;
    let destination = canonicalize_with_missing_tail(destination)?;
    if source == destination || source.starts_with(&destination) || destination.starts_with(&source)
    {
        return Err(format!(
            "Source and destination must be separate folders ({} → {})",
            source.display(),
            destination.display()
        ));
    }
    Ok(())
}

/// Canonicalize the nearest existing ancestor, then append the not-yet-created path components.
fn canonicalize_with_missing_tail(path: &Path) -> Result<PathBuf, String> {
    if lstat_exists(path) && !is_real_directory(path) {
        return Err(format!(
            "{} exists but is not a regular folder",
            path.display()
        ));
    }
    let mut ancestor = path;
    let mut tail = Vec::new();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| format!("{} has no existing parent folder", path.display()))?;
        tail.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| format!("{} has no existing parent folder", path.display()))?;
    }
    let mut result =
        fs::canonicalize(ancestor).map_err(|e| format!("{}: {e}", ancestor.display()))?;
    for component in tail.iter().rev() {
        result.push(component);
    }
    Ok(result)
}

fn lstat_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn inspect_tree(source: &Path, destination: &Path, stats: &mut TreePreview) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", source.display()))?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let metadata =
            fs::symlink_metadata(&from).map_err(|e| format!("{}: {e}", from.display()))?;
        let kind = metadata.file_type();
        if kind.is_symlink() {
            stats.skipped_symlinks += 1;
        } else if kind.is_dir() {
            if lstat_exists(&to) && !is_real_directory(&to) {
                stats.conflicts += 1;
                continue;
            }
            inspect_tree(&from, &to, stats)?;
        } else if kind.is_file() {
            stats.files += 1;
            stats.bytes = stats.bytes.saturating_add(metadata.len());
            if lstat_exists(&to) {
                stats.conflicts += 1;
            }
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path, result: &mut TreeResult) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|e| format!("{}: {e}", destination.display()))?;
    let entries = fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", source.display()))?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let metadata =
            fs::symlink_metadata(&from).map_err(|e| format!("{}: {e}", from.display()))?;
        let kind = metadata.file_type();
        if kind.is_symlink() {
            result.skipped_symlinks += 1;
        } else if kind.is_dir() {
            if lstat_exists(&to) && !is_real_directory(&to) {
                result.skipped_existing += 1;
                continue;
            }
            copy_tree(&from, &to, result)?;
        } else if kind.is_file() {
            if lstat_exists(&to) {
                result.skipped_existing += 1;
                continue;
            }
            fs::create_dir_all(destination)
                .map_err(|e| format!("{}: {e}", destination.display()))?;
            let mut input =
                fs::File::open(&from).map_err(|e| format!("{}: {e}", from.display()))?;
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mut temporary = None;
            for attempt in 0..16u8 {
                let part = destination.join(format!(
                    ".horizon-migrate-{}-{nonce}-{attempt}.part",
                    std::process::id()
                ));
                match OpenOptions::new().write(true).create_new(true).open(&part) {
                    Ok(file) => {
                        temporary = Some((part, file));
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(format!("{}: {error}", part.display())),
                }
            }
            let Some((part, mut output)) = temporary else {
                return Err(format!(
                    "Could not create a temporary file beside {}",
                    to.display()
                ));
            };
            if let Err(error) = io::copy(&mut input, &mut output)
                .and_then(|_| output.set_permissions(metadata.permissions()))
                .and_then(|_| output.sync_all())
            {
                drop(output);
                let _ = fs::remove_file(&part);
                return Err(format!("{}: {error}", to.display()));
            }
            drop(output);
            // A hard link publishes the complete file atomically without replacing a file that
            // appeared after the earlier existence check. Some portable drives (notably FAT/exFAT)
            // do not support hard links, so fall back to an exclusive create-and-copy there.
            let published = match fs::hard_link(&part, &to) {
                Ok(()) => true,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied
                    ) =>
                {
                    let mut input = match fs::File::open(&part) {
                        Ok(file) => file,
                        Err(error) => {
                            let _ = fs::remove_file(&part);
                            return Err(format!("{}: {error}", part.display()));
                        }
                    };
                    let mut output = match OpenOptions::new().write(true).create_new(true).open(&to)
                    {
                        Ok(file) => file,
                        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                            let _ = fs::remove_file(&part);
                            result.skipped_existing += 1;
                            continue;
                        }
                        Err(error) => {
                            let _ = fs::remove_file(&part);
                            return Err(format!("{}: {error}", to.display()));
                        }
                    };
                    if let Err(error) = io::copy(&mut input, &mut output)
                        .and_then(|_| output.set_permissions(metadata.permissions()))
                        .and_then(|_| output.sync_all())
                    {
                        drop(output);
                        let _ = fs::remove_file(&to);
                        let _ = fs::remove_file(&part);
                        return Err(format!("{}: {error}", to.display()));
                    }
                    true
                }
                Err(error) => {
                    let _ = fs::remove_file(&part);
                    return Err(format!("{}: {error}", to.display()));
                }
            };
            let _ = fs::remove_file(&part);
            if published {
                result.copied += 1;
            } else {
                result.skipped_existing += 1;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("horizon-migration-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn migration(root: &Path) -> Migration {
        Migration {
            launcher_source: root.join("old-launcher"),
            launcher_destination: root.join("portable/Horizon-Data"),
            game_source: root.join("old-game"),
            game_destination: root.join("portable/Horizon-Data/horizon"),
        }
    }

    #[test]
    fn preview_is_read_only_and_counts_files_bytes_and_conflicts() {
        let temp = TempTree::new();
        let migration = migration(&temp.0);
        fs::create_dir_all(migration.launcher_source.join("nested")).unwrap();
        fs::write(
            migration.launcher_source.join("nested/settings.toml"),
            "hello",
        )
        .unwrap();
        fs::create_dir_all(migration.launcher_destination.join("nested")).unwrap();
        fs::write(
            migration.launcher_destination.join("nested/settings.toml"),
            "keep",
        )
        .unwrap();
        fs::create_dir_all(&migration.game_source).unwrap();
        fs::write(migration.game_source.join("options.txt"), "abc").unwrap();

        let preview = migration.preview().unwrap();
        assert_eq!(preview.launcher.files, 1);
        assert_eq!(preview.launcher.bytes, 5);
        assert_eq!(preview.launcher.conflicts, 1);
        assert_eq!(preview.game.files, 1);
        assert_eq!(
            fs::read_to_string(migration.launcher_destination.join("nested/settings.toml"))
                .unwrap(),
            "keep"
        );
    }

    #[test]
    fn execute_copies_missing_files_and_never_overwrites_existing_files() {
        let temp = TempTree::new();
        let migration = migration(&temp.0);
        fs::create_dir_all(&migration.launcher_source).unwrap();
        fs::write(migration.launcher_source.join("settings.toml"), "new").unwrap();
        fs::create_dir_all(&migration.launcher_destination).unwrap();
        fs::write(migration.launcher_destination.join("settings.toml"), "keep").unwrap();
        fs::create_dir_all(&migration.game_source).unwrap();
        fs::write(migration.game_source.join("options.txt"), "game").unwrap();

        let result = migration.execute().unwrap();
        assert_eq!(result.launcher.copied, 0);
        assert_eq!(result.launcher.skipped_existing, 1);
        assert_eq!(result.game.copied, 1);
        assert_eq!(
            fs::read_to_string(migration.launcher_destination.join("settings.toml")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(migration.game_destination.join("options.txt")).unwrap(),
            "game"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copied_files_preserve_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempTree::new();
        let migration = migration(&temp.0);
        fs::create_dir_all(&migration.launcher_source).unwrap();
        let source = migration.launcher_source.join("account-secrets.json");
        fs::write(&source, "encrypted").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();

        migration.execute().unwrap();

        let destination = migration.launcher_destination.join("account-secrets.json");
        let mode = fs::metadata(destination).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_skipped_instead_of_followed() {
        use std::os::unix::fs::symlink;

        let temp = TempTree::new();
        let migration = migration(&temp.0);
        fs::create_dir_all(&migration.launcher_source).unwrap();
        fs::write(temp.0.join("outside.txt"), "outside").unwrap();
        symlink(
            temp.0.join("outside.txt"),
            migration.launcher_source.join("linked.txt"),
        )
        .unwrap();

        let preview = migration.preview().unwrap();
        assert_eq!(preview.launcher.files, 0);
        assert_eq!(preview.launcher.skipped_symlinks, 1);
        let result = migration.execute().unwrap();
        assert_eq!(result.launcher.copied, 0);
        assert_eq!(result.launcher.skipped_symlinks, 1);
        assert!(!migration.launcher_destination.join("linked.txt").exists());
    }

    #[test]
    fn overlapping_source_and_destination_are_rejected() {
        let temp = TempTree::new();
        let source = temp.0.join("source");
        fs::create_dir_all(&source).unwrap();
        let destination = source.join("portable");
        let error = validate_pair(&source, &destination).unwrap_err();
        assert!(error.contains("separate folders"));
    }

    #[test]
    fn missing_sources_are_a_valid_empty_migration() {
        let temp = TempTree::new();
        let preview = migration(&temp.0).preview().unwrap();
        assert_eq!(preview, Preview::default());
        let result = migration(&temp.0).execute().unwrap();
        assert_eq!(result, ResultSummary::default());
    }
}
