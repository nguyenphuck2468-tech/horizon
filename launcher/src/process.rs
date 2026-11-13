//! The programs the launcher runs in the background: on Windows they open no console window
//! (the launcher has none to share), and keep the streams their caller gives them.
use std::ffi::OsStr;
use std::process::Command;

/// Variables only the launcher itself may read. A child process (the game, a mod it loads, a
/// pre-launch or post-exit command, a wrapper) inherits the launcher's environment unless told
/// otherwise, so each is removed from every program the launcher starts.
pub const LAUNCHER_ONLY_ENV: &[&str] = &[
    "HORIZON_ELY_CLIENT_SECRET",
    "HORIZON_GITHUB_TOKEN",
    "HORIZON_CURSEFORGE_API_KEY",
];

/// Keeps [`LAUNCHER_ONLY_ENV`] out of `command`'s environment.
pub fn scrub_secrets(command: &mut Command) -> &mut Command {
    for key in LAUNCHER_ONLY_ENV {
        command.env_remove(key);
    }
    command
}

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    hide_console(&mut command);
    command
}

/// Windows' flag for a console program started without a console window. `creation_flags` sets
/// the whole value, so a caller adding its own flags includes this one
/// ([`crate::rust_toolchain::low_priority`]).
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// [`command`] for a `Command` made elsewhere.
pub fn hide_console(command: &mut Command) -> &mut Command {
    scrub_secrets(command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn removed(command: &Command, key: &str) -> bool {
        command
            .get_envs()
            .any(|(k, v)| k == OsStr::new(key) && v.is_none())
    }

    #[test]
    fn the_ely_client_secret_never_reaches_a_child_process() {
        for command in [command("true"), {
            let mut plain = Command::new("true");
            hide_console(&mut plain);
            plain
        }] {
            assert!(removed(&command, "HORIZON_ELY_CLIENT_SECRET"));
            assert!(removed(&command, "HORIZON_GITHUB_TOKEN"));
            assert!(removed(&command, "HORIZON_CURSEFORGE_API_KEY"));
        }
        let mut direct = Command::new("true");
        scrub_secrets(&mut direct);
        assert!(removed(&direct, "HORIZON_ELY_CLIENT_SECRET"));
        assert!(removed(&direct, "HORIZON_GITHUB_TOKEN"));
        assert!(removed(&direct, "HORIZON_CURSEFORGE_API_KEY"));
    }
}
