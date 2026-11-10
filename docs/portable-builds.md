# Portable launcher builds

The release includes opt-in portable archives alongside the normal installer and self-update assets:

| Platform | Portable archive | Data folder |
| --- | --- | --- |
| Windows x64 | `horizon-launcher-windows-x64-portable.zip` | `Horizon-Data` beside `horizon-launcher.exe` |
| Linux x86_64 | `horizon-launcher-linux-x86_64-portable.tar.gz` | `Horizon-Data` beside `horizon-launcher` |
| macOS Apple Silicon | `horizon-launcher-macos-arm64-portable.zip` | `Horizon-Data` beside `Horizon.app` |
| macOS Intel | `horizon-launcher-macos-x64-portable.zip` | `Horizon-Data` beside `Horizon.app` |

Extract the archive to a folder you can write to and launch Horizon from there. The portable marker opts the
launcher into storing settings, accounts, downloaded game versions, Java runtimes and other launcher data in
the adjacent `Horizon-Data` folder. Keep the launcher and that folder together when moving or backing it up.
The data folder is created on first launch. The marker stays outside the replaceable executable/app bundle, so a
portable installation keeps using its adjacent data folder after a self-update.

The ordinary Windows ZIP, Linux self-update tarball and macOS app ZIP do not contain the portable marker:
they keep using the normal per-user data location and remain compatible with the launcher's self-updater.
The Windows setup executable, macOS DMG and Linux AppImage remain available as convenience formats.
