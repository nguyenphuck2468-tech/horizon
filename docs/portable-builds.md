# Portable launcher builds

The release includes opt-in portable archives alongside the normal installer and self-update assets:

| Platform | Portable archive | Data folder |
| --- | --- | --- |
| Windows x64 | `horizon-launcher-windows-x64-portable.zip` | `Horizon-Data` beside `horizon-launcher.exe` |
| Linux x86_64 | `horizon-launcher-linux-x86_64-portable.tar.gz` | `Horizon-Data` beside `horizon-launcher` |
| macOS Apple Silicon | `horizon-launcher-macos-arm64-portable.zip` | `Horizon-Data` beside `Horizon.app` |
| macOS Intel | `horizon-launcher-macos-x64-portable.zip` | `Horizon-Data` beside `Horizon.app` |

Extract the archive to a folder you can write to and launch Horizon from there. The portable marker opts the
launcher into storing settings, accounts, downloaded game versions, Java runtimes, worlds, screenshots, options and other game/launcher data in
the adjacent `Horizon-Data` folder. Keep the launcher and that folder together when moving or backing it up.
The data folder is created on first launch. The marker stays outside the replaceable executable/app bundle, so a
portable installation keeps using its adjacent data folder after a self-update.

The ordinary Windows ZIP, Linux self-update tarball and macOS app ZIP do not contain the portable marker:
they keep using the normal per-user data location and remain compatible with the launcher's self-updater.
The Windows setup executable, macOS DMG and Linux AppImage remain available as convenience formats.

## What "portable" means for accounts

Portable mode moves the launcher's **files** and the game's **home folder** next to the launcher. It does not make
provider credentials transferable between computers:

- `accounts.json` contains profile metadata and account selection, not access or refresh tokens.
- Windows and macOS keep sign-in secrets in the operating system's credential store. Copying `Horizon-Data`
  does not copy those credentials.
- On Linux and FreeBSD, when the Secret Service is unavailable, Horizon can keep an encrypted
  `account-secrets.json` fallback in `Horizon-Data`. Its encryption key is derived from that machine's
  identity. The file can be used on the same machine, but a copy on another machine cannot be decrypted.

Consequently, a portable folder copied to a different computer may still show saved account profiles but require
Microsoft or Ely.by sign-in again. This is intentional: Horizon does not export plaintext tokens or weaken the
operating system's credential protection to make a folder transferable. Keep a separate backup of the folder;
never treat a copied account card as proof that its sign-in credentials moved too.

## Moving an existing installation into portable mode

For a same-computer migration, close every Horizon launcher and game process first, extract the portable archive to
a writable folder, and make a backup before copying anything. The usual source folders are:

| Platform | Launcher data folder | Game home |
| --- | --- | --- |
| Windows | `%APPDATA%\horizon-launcher` | `%APPDATA%\horizon` |
| Linux | `$XDG_DATA_HOME/horizon-launcher` (normally `~/.local/share/horizon-launcher`) | `$XDG_DATA_HOME/horizon` (normally `~/.local/share/horizon`) |
| macOS | `~/Library/Application Support/horizon-launcher` | `~/Library/Application Support/horizon` |

If you customized `XDG_DATA_HOME`, `HORIZON_LAUNCHER_HOME` or `HORIZON_HOME`, use the actual paths those
settings selected instead. Copy the contents of the old launcher's data folder into the new `Horizon-Data`
folder, then copy the contents of the old game's home into `Horizon-Data/horizon`. Preserve the directory
structure and existing files; do not replace a non-empty destination without comparing it first. On the same
machine, the OS keychain remains available and a Linux fallback secrets file retains the machine identity it needs.
The portable launcher's Settings → About section includes a preview-first migration assistant. It scans these
default per-user folders in the background, shows file counts, sizes and conflicts, and copies only missing
files after the player confirms. Existing destination files are never overwritten; symbolic links are skipped.
The operation is additive and can be run again safely to pick up files added later. Back up both source folders
before starting, and close all Horizon/game processes first. Custom path overrides are intentionally not guessed:
use the actual folders manually when `HORIZON_LAUNCHER_HOME` or `HORIZON_HOME` is set.

On another computer, migrate the game and launcher files the same way if desired, but **do not copy
`account-secrets.json`** from the old machine. Keep the original backup, then sign in again on the destination.
Credentials held by Windows Credential Manager, macOS Keychain, or a Linux Secret Service are not contained in
the portable folder and must be made available through that computer's own credential store.

Portable data paths are deliberately separate: launcher metadata and settings live directly in `Horizon-Data`;
the game's versions, Java runtimes, instances, worlds, options and screenshots live in
`Horizon-Data/horizon`. The portable marker must remain beside the launcher executable (or beside
`Horizon.app` on macOS), not inside `Horizon-Data`.
