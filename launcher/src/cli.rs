//! `horizon-launcher <subcommand>`: the same steps as the window, from a terminal. Every
//! command goes through `backend`; this file only parses arguments and prints.
use clap::{Parser, Subcommand};
use horizon_launcher::account::{self, Accounts};
use horizon_launcher::backend::{self, Release};
use horizon_launcher::build_import;
use horizon_launcher::github_mods;
use horizon_launcher::i18n::{tr, trf};
use horizon_launcher::instances;
use horizon_launcher::loaders;
use horizon_launcher::native_mods;
use horizon_launcher::native_status;
use horizon_launcher::other_launchers;
use horizon_launcher::server_list;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "horizon-launcher",
    version = backend::VERSION,
    about = "Installs, prepares and starts Horizon. Without a subcommand, opens the window."
)]
struct Cli {
    /// Print JSON instead of text, where a command has something to report
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum JavaCmd {
    /// The Java runtimes Horizon downloaded from Mojang and the ones installed on this computer
    List,
    /// Download Mojang's Java runtime: a major (8, 16, 17, 21, 25) or a component name from a
    /// version's JSON (java-runtime-delta)
    Install { java: String },
}

#[derive(Subcommand)]
enum RustCmd {
    /// Whether it is installed, its version and whether this computer can link with it
    Status,
    /// Download the pinned toolchain from static.rust-lang.org (once), checked by SHA-256
    Install,
    /// Delete the toolchain and the native builds' Cargo home
    Remove,
}

#[derive(Subcommand)]
enum ServersCmd {
    /// The servers, their folder and whether they run
    List,
    /// Create a server; with no options it is a new world, survival, normal, 10 players, friends only
    Create {
        #[arg(long, default_value = "My server")]
        name: String,
        /// The new world's seed (default: random)
        #[arg(long, conflicts_with = "copy_world")]
        seed: Option<String>,
        /// Start from a copy of this world folder (one of your saves)
        #[arg(long)]
        copy_world: Option<PathBuf>,
        #[arg(long, default_value = "survival", value_parser = ["survival", "creative", "adventure", "spectator"])]
        mode: String,
        #[arg(long, default_value = "normal", value_parser = ["peaceful", "easy", "normal", "hard"])]
        difficulty: String,
        #[arg(long, default_value_t = 10)]
        max_players: u32,
        /// Let anyone in (no whitelist)
        #[arg(long)]
        open: bool,
        /// Do not check players' Minecraft accounts (offline mode: anyone can use any name)
        #[arg(long)]
        offline: bool,
        /// Kept for old scripts: account checking is now the default
        #[arg(long, hide = true, conflicts_with = "offline")]
        check_accounts: bool,
        /// Who may join (repeatable); the first becomes an operator
        #[arg(long = "player")]
        players: Vec<String>,
        /// The installed version whose programs and game files servers use, if none are set up yet
        #[arg(long)]
        version: Option<String>,
        /// Agree to the Minecraft EULA (https://aka.ms/MinecraftEULA)
        #[arg(long)]
        accept_eula: bool,
    },
    /// Copy an installed, prepared version's programs and game files for the servers (again)
    SetUp {
        #[arg(long)]
        version: Option<String>,
    },
    /// Run a horizon-server command on a server, for example: servers run "My server" -- status
    Run {
        /// The server's name or folder
        server: String,
        #[arg(last = true, required = true)]
        args: Vec<String>,
    },
    /// Take a server off the list (its folder stays)
    Forget { server: String },
}

#[derive(Subcommand)]
enum AccountCmd {
    /// The signed-in accounts and which one plays
    List,
    /// Sign in with a Microsoft account in your browser (device code). Play needs one that owns
    /// Minecraft: Java Edition
    SignIn {
        /// Sign this account (name or UUID) in again: another Microsoft account is refused and
        /// who plays stays the same
        #[arg(long = "for")]
        target: Option<String>,
        /// Authentication provider
        #[arg(long, default_value = "microsoft", value_parser = ["microsoft", "elyby"])]
        provider: String,
    },
    /// Renew an account's session now, even when its access token is still fresh
    Refresh { who: String },
    /// Renew an account's session for a game already running: prints the read-once session file
    /// it hands over (`--json`: {"session_file"} or {"authentication"} when it can't)
    Session { who: String },
    /// Sign out: the account's tokens leave the keychain (and the secrets file, where it kept them)
    SignOut {
        /// Its name or UUID
        who: String,
    },
    /// Play as this account, or as one of your offline names
    Select { who: String },
    /// Play offline (LAN, offline-mode servers) beside a signed-in account that owns the game:
    /// as NAME (kept to switch back to; 3 to 16 letters, digits or _), or as the name the game
    /// made up
    Offline { name: Option<String> },
    /// Forget one of your offline names; its skin and cape go with it
    ForgetOffline { name: String },
    /// Show or set an offline name's skin and cape: a 64x64 (or 64x32) skin PNG, a 64x32 cape
    /// PNG, the model. Kept by the launcher and given to the game when that name plays
    Skin {
        name: String,
        /// The skin's PNG
        file: Option<PathBuf>,
        /// classic or slim arms
        #[arg(long)]
        model: Option<String>,
        /// The cape's PNG
        #[arg(long)]
        cape: Option<PathBuf>,
        /// Back to the default skin
        #[arg(long)]
        no_skin: bool,
        /// Take the cape off
        #[arg(long)]
        no_cape: bool,
    },
}

#[derive(Subcommand)]
enum DataCmd {
    /// What is allowed, the install ID and how many items wait (the default)
    Status,
    /// Turn kinds on: performance, crashes, bugs, benchmarks, or all
    Allow {
        #[arg(required = true)]
        kinds: Vec<String>,
    },
    /// Turn kinds off: performance, crashes, bugs, benchmarks, or all
    Deny {
        #[arg(required = true)]
        kinds: Vec<String>,
    },
    /// Print exactly what would be sent
    Show,
    /// Delete everything sent from here, and what waits, and take a new install ID
    Delete,
    /// Send what is waiting now
    Send,
    /// Send a bug report you write; it is printed first, as it would leave
    Bug {
        text: String,
        /// Add the end of the game's log (scrubbed, without chat)
        #[arg(long)]
        log: bool,
        /// Send without asking
        #[arg(long)]
        yes: bool,
    },
    /// Upload a benchmark report (docs/benchmark-report.md); it is printed first
    Benchmark {
        file: PathBuf,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ModpackCmd {
    /// The imported packs and their coverage (the default)
    List,
    /// Import a .mrpack, a CurseForge zip or an instance folder, and write its coverage report
    Import { path: PathBuf },
    /// Get the jars of what this computer can port and carry them over
    Port {
        /// The pack's folder name, as list shows it
        pack: String,
        /// A folder with your own jars (an installed instance, a downloads folder)
        #[arg(long)]
        from: Option<PathBuf>,
        /// The installed version the ports are for (default: the selected one)
        #[arg(long)]
        version: Option<String>,
    },
    /// Play a ported pack as its own profile
    Play {
        pack: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Remove an imported pack: its jars, ports and the profile's saves
    Remove { pack: String },
}

#[derive(Subcommand)]
enum InstanceCmd {
    /// The instances, each with its game folder (* = the one Play starts)
    List,
    /// Make an instance: empty but for your options and name, or a copy of another one's
    /// worlds, options and packs
    Create {
        name: String,
        /// The instance to copy (its name or id)
        #[arg(long)]
        copy: Option<String>,
        /// Make a Minecraft: Java Edition instance of this version (1.21.4, latest or
        /// latest-snapshot) instead of a Horizon one
        #[arg(long, conflicts_with = "copy")]
        minecraft: Option<String>,
        /// With this mod loader: fabric, quilt, forge or neoforge
        #[arg(long, requires = "minecraft")]
        loader: Option<String>,
        /// The loader's version (default: its newest stable one)
        #[arg(long, requires = "loader")]
        loader_version: Option<String>,
    },
    /// The versions of Minecraft: Java Edition a Java instance can play, the newest first
    Versions {
        /// Snapshots too
        #[arg(long)]
        snapshots: bool,
        /// Old betas and alphas too
        #[arg(long)]
        old: bool,
    },
    /// A Java instance's Java: its own Java home or the automatic pick, memory and JVM arguments
    Java {
        instance: String,
        /// A Java home to run it on
        #[arg(long, conflicts_with = "auto")]
        home: Option<PathBuf>,
        /// Go back to the Java the version needs, picked or downloaded by the launcher
        #[arg(long)]
        auto: bool,
        /// The most memory the game may use, in MiB
        #[arg(long)]
        memory: Option<u32>,
        /// Extra JVM arguments (one string; "" clears them)
        #[arg(long, allow_hyphen_values = true)]
        args: Option<String>,
        /// The game window's size, as 1280x720; "default" goes back to the game's own
        #[arg(long)]
        window: Option<String>,
    },
    /// A Java instance's mod loader: fabric, quilt, forge or neoforge, or none. --list shows its versions
    Loader {
        instance: String,
        loader: String,
        /// The loader's version (default: its newest stable one)
        #[arg(long)]
        version: Option<String>,
        /// List the loader's versions for the instance's Minecraft instead
        #[arg(long, conflicts_with = "version")]
        list: bool,
    },
    /// Rename an instance (its folder stays)
    Rename { instance: String, name: String },
    /// Delete an instance: its folder (worlds, options, packs) goes to the Trash, where you can
    /// get it back. Horizon's own instance stays
    Delete { instance: String },
    /// Set an instance's icon: a block or item name from your prepared files (grass_block),
    /// a colour (#3fa7d6), or none
    Icon { instance: String, icon: String },
    /// Put an instance in a sidebar group, or take it out with no group
    Group {
        instance: String,
        group: Option<String>,
    },
    /// Make Play start this instance
    Select { instance: String },
    /// Keep an instance on one installed Horizon version, or "latest" to follow updates again
    Version { instance: String, version: String },
    /// Export an instance (or a port) to share: its settings, packs, Horizon mods and the worlds
    /// named, as a .mrpack (resource packs from Modrinth listed by download) or a zip. Never
    /// logs, screenshots or account data; worlds go without their per-player files
    Export {
        /// The instance (default: the selected one)
        instance: Option<String>,
        #[arg(long, value_enum, default_value_t = ExportFormat::Mrpack)]
        format: ExportFormat,
        /// A world to put in it (its name or folder); repeat for more
        #[arg(long)]
        world: Vec<String>,
        /// Where to write it (default: "<name>.mrpack" in the current folder)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Show or set what runs around an instance's game: a shell command before it (the game
    /// starts only if it succeeds), a wrapper it runs through (`nice -n 5`), environment
    /// variables ("NAME=value NAME2=value") and a shell command after it exits. An empty value
    /// clears one. The game and the commands also get INST_NAME, INST_ID, INST_DIR,
    /// INST_MC_DIR and, for a Java instance, INST_JAVA. On Linux, also GameMode, MangoHud and
    /// the dedicated graphics card, when the computer has them
    Launch {
        instance: String,
        #[arg(long)]
        pre_launch: Option<String>,
        #[arg(long)]
        wrapper: Option<String>,
        #[arg(long)]
        post_exit: Option<String>,
        #[arg(long)]
        env: Option<String>,
        /// Linux: run it through GameMode (gamemoderun), on or off
        #[arg(long)]
        gamemode: Option<String>,
        /// Linux: MangoHud's overlay, on or off
        #[arg(long)]
        mangohud: Option<String>,
        /// Linux: the dedicated graphics card (DRI_PRIME, or NVIDIA's offload), on or off
        #[arg(long)]
        dedicated_gpu: Option<String>,
    },
    /// Put a shortcut on the desktop that plays an instance (`play --instance`): a .desktop
    /// file (also in the applications menu) on Linux, an app on macOS, a .lnk on Windows
    Shortcut {
        /// The instance (default: the selected one)
        instance: Option<String>,
        /// Move the instance's shortcuts to the trash instead
        #[arg(long)]
        remove: bool,
    },
    /// Import what a file is, as dropping it on the window does: a shared instance (a .mrpack
    /// without mod loaders, such as `instance export` writes, or a game folder zip) becomes a
    /// new instance; a Java modpack goes to the Modpacks import; a resource or shader pack into
    /// the instance (default: the selected one)
    Import {
        file: PathBuf,
        /// The instance a pack goes into
        #[arg(long)]
        instance: Option<String>,
    },
    /// List the instances other launchers keep (Prism Launcher, MultiMC, the official
    /// launcher, the CurseForge app, the Modrinth App), read without changing anything there
    Others {
        /// Look in this folder instead: a launcher's folder, its instances/ or one instance
        #[arg(long)]
        folder: Option<PathBuf>,
    },
    /// Copy another launcher's instance into a new Java instance: its Minecraft version and its
    /// game folder (worlds, mods, packs, options); the other launcher's copy stays as it is
    CopyFrom {
        /// Its name, as `instance others` lists it
        name: String,
        /// Look in this folder instead: a launcher's folder, its instances/ or one instance
        #[arg(long)]
        folder: Option<PathBuf>,
        /// The new instance's name (default: the same)
        #[arg(long = "as")]
        new_name: Option<String>,
        /// The Minecraft version it plays, when its launcher doesn't say (the CurseForge app,
        /// the Modrinth App)
        #[arg(long)]
        minecraft: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    /// Modrinth's modpack format
    Mrpack,
    /// The game folder's layout, zipped
    Zip,
}

#[derive(Subcommand)]
enum WorldsCmd {
    /// The worlds: name, mode, version, last played and size (the default)
    List {
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Zip a world into the instance's backups/ folder, as the game's Make Backup does
    Backup {
        /// Its folder or name
        world: String,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Copy a world beside it, as "<name> (1)"
    Copy {
        world: String,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Move a world to the system's trash
    Delete {
        world: String,
        #[arg(long)]
        instance: Option<String>,
    },
    /// A world's data packs, and which are on
    DatapackList {
        world: String,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Copy a data pack zip into a world (the game turns it on when the world opens)
    DatapackAdd {
        world: String,
        file: PathBuf,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Move a world's data pack to the system's trash
    DatapackRemove {
        world: String,
        /// Its file name, with or without .zip
        pack: String,
        #[arg(long)]
        instance: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum PackKindArg {
    Resource,
    Shader,
}

impl From<PackKindArg> for horizon_launcher::packs::Kind {
    fn from(k: PackKindArg) -> Self {
        match k {
            PackKindArg::Resource => Self::Resource,
            PackKindArg::Shader => Self::Shader,
        }
    }
}

#[derive(Subcommand)]
enum PacksCmd {
    /// The resource and shader packs, and which are on (the default)
    List {
        /// Only this kind
        #[arg(long, value_enum)]
        kind: Option<PackKindArg>,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Copy a pack file in: a resource pack .zip, a shader pack .zip or a .hshader
    Add {
        file: PathBuf,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Turn a pack on (a resource pack goes on top; a shader pack replaces the one that was on)
    On {
        /// Its file name or title
        pack: String,
        #[arg(long, value_enum, default_value_t = PackKindArg::Resource)]
        kind: PackKindArg,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Turn a pack off
    Off {
        pack: String,
        #[arg(long, value_enum, default_value_t = PackKindArg::Resource)]
        kind: PackKindArg,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Turn a pack off and move it to the system's trash
    Remove {
        pack: String,
        #[arg(long, value_enum, default_value_t = PackKindArg::Resource)]
        kind: PackKindArg,
        #[arg(long)]
        instance: Option<String>,
    },
}

#[derive(Subcommand)]
enum ScreenshotsCmd {
    /// The screenshots, the newest first, with their files (the default)
    List {
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Move a screenshot to the system's trash
    Delete {
        /// Its file name, with or without .png
        screenshot: String,
        #[arg(long)]
        instance: Option<String>,
    },
}

#[derive(Subcommand)]
enum MultiplayerCmd {
    /// The servers of the game's Multiplayer screen (the default)
    List {
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Add a server to the list
    Add {
        /// Its address, as in the game's Add Server (host or host:port)
        address: String,
        /// Its name in the list (default: Minecraft Server)
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        instance: Option<String>,
    },
    /// Remove a server from the list: its position from 1, name or address
    Remove {
        server: String,
        #[arg(long)]
        instance: Option<String>,
    },
}

#[derive(Subcommand)]
enum ModsCmd {
    /// What the browser installed in an instance (the default)
    List {
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Search for packs Horizon loads, or for what a Java instance runs (made for its
    /// Minecraft version and loader)
    Search {
        text: Vec<String>,
        #[arg(long, value_enum, default_value_t = KindArg::Resource)]
        kind: KindArg,
        /// Also what was made for other versions or runs only partly, each marked
        #[arg(long)]
        everything: bool,
        #[arg(long, value_enum, default_value_t = SourceArg::Modrinth)]
        source: SourceArg,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Say how well a Java mod runs in Horizon: its jar is downloaded (checksum checked),
    /// carried over and read; the answer is kept
    Check {
        project: String,
        /// Allow a version made for another Minecraft version (1.21 and later)
        #[arg(long)]
        everything: bool,
        #[arg(long, value_enum, default_value_t = SourceArg::Modrinth)]
        source: SourceArg,
    },
    /// Install a pack and what it needs into an instance, by its id or slug (checksums
    /// checked); a Java mod is ported into horizon-mods and turned on (a Java instance gets the
    /// jar made for its version and loader in mods/); a modpack is imported as an instance of
    /// its own; a data pack goes into the world `--world` names
    Install {
        project: String,
        /// Install the project's data pack into this world of the instance (its name or folder)
        #[arg(long)]
        world: Option<String>,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
        /// Allow a version made for another Minecraft version
        #[arg(long)]
        everything: bool,
        #[arg(long, value_enum, default_value_t = SourceArg::Modrinth)]
        source: SourceArg,
    },
    /// Update what the browser installed (or one pack)
    Update {
        project: Option<String>,
        /// Only say what is newer
        #[arg(long)]
        check: bool,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Install a native mod from a GitHub link (docs/native-mods.md, "Install from GitHub"):
    /// https://github.com/OWNER/REPO (its latest release, else its default branch),
    /// .../releases/tag/TAG, .../tree/REF or .../commit/SHA. Resolved to an exact release file or
    /// commit, checked like a registry mod. NOT reviewed by the Horizon registry
    AddGithub {
        url: String,
        /// The mod to install when the link holds several
        #[arg(long = "mod")]
        mod_id: Option<String>,
        /// Install every mod the link holds
        #[arg(long)]
        all: bool,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
        /// You read the mod's source (the link printed) and trust it with full access to this computer
        #[arg(long)]
        trust_native_code: bool,
    },
    /// Look for newer versions of the native mods installed from GitHub (the latest release, or the
    /// default branch's head for a mod installed from a commit), and install them
    UpdateGithub {
        /// One mod (default: all of them)
        id: Option<String>,
        /// Only say what is newer
        #[arg(long)]
        check: bool,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
        /// You read the new source and trust it with full access to this computer
        #[arg(long)]
        trust_native_code: bool,
    },
    /// Remove a pack the browser installed
    Remove {
        project: String,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Turn on a mod the browser installed (horizon-mods.txt, as the game's Options > Mods does;
    /// in a Java instance, its file loses `.disabled`)
    Enable {
        project: String,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Turn off a mod the browser installed; it stays installed (in a Java instance, as
    /// `<file>.disabled`)
    Disable {
        project: String,
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum KindArg {
    Mod,
    Resource,
    Modpack,
    /// Horizon's own shader packs (Horizon's registry only)
    Shader,
    /// Data packs (Modrinth): install one into a world with `mods install <pack> --world <name>`
    Datapack,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum SourceArg {
    Modrinth,
    /// CurseForge support is coming soon
    Curseforge,
    /// Horizon's own registry: mods, modpacks and resource packs made for Horizon, reviewed
    Horizon,
}

impl SourceArg {
    fn get(self) -> Result<Box<dyn horizon_launcher::browser::Source>, String> {
        use horizon_launcher::browser::{self, SourceId};
        let id = match self {
            SourceArg::Modrinth => SourceId::Modrinth,
            SourceArg::Curseforge => SourceId::CurseForge,
            SourceArg::Horizon => SourceId::Horizon,
        };
        browser::source(id).ok_or_else(|| browser::CURSEFORGE_SOON.into())
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Write a shell completion script to stdout
    Completions { shell: clap_complete::Shell },
    /// List Horizon releases, or the installed versions
    Versions {
        /// Only the versions installed here (no network)
        #[arg(long)]
        installed: bool,
    },
    /// Download and install a release (a tag such as v2026.10.13, or "latest")
    Install { tag: String },
    /// Remove an installed version
    Remove { tag: String },
    /// Install a game program you have, such as a horizon-client built on your fork's Actions,
    /// as its own version on an installed release or Nightly's game files, and select it.
    /// Nothing signed it: it runs with your rights, and updates never replace it
    ImportBuild {
        /// The program (horizon-client, horizon-client.exe on Windows), or a zip holding it
        path: PathBuf,
        /// The installed release or Nightly whose game files it plays with (default: the
        /// selected one, else the newest)
        #[arg(long)]
        version: Option<String>,
        /// You picked this build yourself and trust it with full access to this computer
        #[arg(long)]
        trust_unsigned: bool,
    },
    /// Prepare a version's resources from your own Minecraft install, or from Mojang
    Prepare {
        /// The game folder of the official launcher, Prism Launcher, MultiMC, the Modrinth App,
        /// ATLauncher or GDLauncher (default: the launcher's, then every launcher's default)
        #[arg(long, conflicts_with = "download")]
        minecraft: Option<PathBuf>,
        /// Download Minecraft from Mojang for the signed-in account that owns it, instead of
        /// using an install
        #[arg(long)]
        download: bool,
        /// The installed version (default: the selected one)
        #[arg(long)]
        version: Option<String>,
    },
    /// Start the client (needs a signed-in Microsoft account that owns Minecraft: Java Edition)
    Play {
        #[arg(long)]
        version: Option<String>,
        /// The instance to play (its name or id; default: the selected one). A Java instance
        /// starts its Minecraft: Java Edition, downloaded with its Java the first time
        #[arg(long)]
        instance: Option<String>,
        /// Open or create this world (a folder under saves) at once
        #[arg(long)]
        world: Option<String>,
        /// Creative mode for a new world
        #[arg(long)]
        creative: bool,
        /// Join a friend's world with its invite code
        #[arg(long)]
        join: Option<String>,
        /// Join this server at once: an address, or a server of the list by name or position
        #[arg(long, conflicts_with_all = ["world", "join"])]
        server: Option<String>,
        /// An extra argument for the game, passed through as-is (repeatable), for example
        /// --server-thread
        #[arg(long = "arg", value_name = "ARG", allow_hyphen_values = true)]
        arg: Vec<String>,
        /// Play this launch as this signed-in account (its name or UUID), without changing
        /// the one `account select` picked: several games can play as several accounts at once
        #[arg(long, value_name = "WHO")]
        account: Option<String>,
    },
    /// Run a dedicated server here; type commands into its console
    Server {
        /// The server's folder (default: the one the launcher uses)
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        version: Option<String>,
        /// Agree to the Minecraft EULA (https://aka.ms/MinecraftEULA)
        #[arg(long)]
        accept_eula: bool,
    },
    /// Native mods (docs/native-mods.md): compile mixin bundles on the Horizon revision they
    /// pin, else the one the instance plays, with the launcher's own Rust, as the version the
    /// instance then plays. Not sandboxed: the mods' code runs with your rights
    NativeBuild {
        /// A mod folder (horizon-mixin.toml, or an unpacked .hnmod), or a folder of them
        /// (default: the native mods installed and turned on in the instance)
        folder: Option<PathBuf>,
        /// Export mode: build a standalone dedicated-server engine into this folder instead of
        /// an instance's build. Needs a mod FOLDER; no instance is read or changed, and no
        /// launcher config is written
        #[arg(
            long = "server-output",
            value_name = "DIR",
            conflicts_with = "instance",
            requires = "folder"
        )]
        server_output: Option<PathBuf>,
        /// The instance whose installed native mods to build, and which then plays the build
        /// (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
        /// The installed version whose game files the build reuses, and whose commit it builds on
        /// when no mod pins one (default: the one the instance plays, so a rebuild stays on its
        /// Horizon)
        #[arg(long)]
        version: Option<String>,
        /// Move to the newest installed release or Nightly instead of the Horizon the instance
        /// plays (a full rebuild)
        #[arg(long, conflicts_with = "version")]
        update: bool,
        /// You read the mods' source and trust it with full access to this computer
        #[arg(long)]
        trust_native_code: bool,
        /// Build mods made for different Horizon versions together anyway, on the newest of them
        #[arg(long)]
        allow_version_mismatch: bool,
        /// The build profile: performance (the default: incremental, quick to rebuild), dev (quickest
        /// to rebuild a mod, slow game) or release (thin LTO: slowest to build, slightly faster
        /// game). Default: the native-profile setting
        #[arg(long, value_parser = ["performance", "dev", "release"])]
        profile: Option<String>,
    },
    /// Show a release's patch notes (default: the newest)
    Notes { tag: Option<String> },
    /// Show the Discord, sponsor and website links
    Links,
    /// Microsoft accounts: list them (the default), sign in, sign out, or pick the one that plays
    Account {
        #[command(subcommand)]
        action: Option<AccountCmd>,
    },
    /// Show the launcher's settings, or set one (the Settings page): selected-version TAG,
    /// minecraft-dir PATH, server-dir PATH ("default" resets a folder), install-new-versions
    /// on|off, prune-old-versions on|off (delete old versions after an install, keeping the
    /// selected one, the ones instances use and the newest few), keep-versions N (how many of the
    /// newest to keep; "default" is 2), opening-animation on|off, proxy http://host:port
    /// ("default" removes it), while-playing stay|minimize|hide (what the window does while the game runs), language
    /// system|en|fr|ru|de|zh|es|uk|pl (the window's language; system follows the computer's), accent
    /// violet|blue|teal|green|amber|red|pink|#rrggbb|DEGREES (a named accent, any colour, or any
    /// hue), background PATH (a picture behind Home; "default"
    /// shows Horizon's landscape), native-profile performance|dev|release (the profile native
    /// mods build with: performance is incremental, dev rebuilds a mod quickest for a slow game,
    /// release is slowest to build and slightly faster to play)
    Settings {
        #[arg(value_parser = SETTING_KEYS)]
        key: Option<String>,
        /// The new value (with KEY)
        #[arg(requires = "key")]
        value: Option<String>,
    },
    /// Show or set the extra arguments the game starts with (for example --server-thread);
    /// quotes group a word with spaces in it
    GameArgs {
        /// Omit to show the current arguments
        #[arg(allow_hyphen_values = true)]
        args: Option<String>,
    },
    /// Upload an instance's latest.log to mclo.gs (a paste service for Minecraft logs) and print
    /// its link, for a bug report: only when the player asks. The home folder, the user name and
    /// the account names are taken out first; the link expires after 90 days unread
    ShareLog {
        /// The instance (default: the selected one)
        #[arg(long)]
        instance: Option<String>,
    },
    /// Delete a log shared with share-log: the "<id>:<token>" it printed
    UnshareLog { shared: String },
    /// Print the end of the game's log (logs/latest.log in the game folder), or its newest crash
    /// report
    Logs {
        /// How many lines of latest.log
        #[arg(long, default_value_t = 100)]
        lines: usize,
        /// The newest crash report instead
        #[arg(long)]
        crash: bool,
    },
    /// Look for a newer Horizon release and launcher, and install them
    Update {
        /// Only report what is available
        #[arg(long)]
        check: bool,
    },
    /// Linux: put Horizon in the applications menu (an entry and the icon, under
    /// $XDG_DATA_HOME), or take it out with --remove
    DesktopEntry {
        /// Remove the entry and the icon, and stop adding them back
        #[arg(long)]
        remove: bool,
    },
    /// Opt-in data collection (docs/telemetry.md): what is allowed, what would be sent, and
    /// deleting it; all off until you allow a kind
    Data {
        #[command(subcommand)]
        action: Option<DataCmd>,
    },
    /// Instances: separate game folders, each with its own worlds, options and packs
    Instance {
        #[command(subcommand)]
        action: Option<InstanceCmd>,
    },
    /// An instance's single-player worlds: list them, back one up, copy or delete it
    Worlds {
        #[command(subcommand)]
        action: Option<WorldsCmd>,
    },
    /// An instance's resource and shader packs: list, add, turn on or off, remove
    Packs {
        #[command(subcommand)]
        action: Option<PacksCmd>,
    },
    /// Java for Minecraft: Java Edition instances: list the runtimes, download Mojang's
    Java {
        #[command(subcommand)]
        action: Option<JavaCmd>,
    },
    /// The screenshots F2 took in an instance: list, delete
    Screenshots {
        #[command(subcommand)]
        action: Option<ScreenshotsCmd>,
    },
    /// The servers in an instance's Multiplayer list (servers.dat): list, add, remove
    Multiplayer {
        #[command(subcommand)]
        action: Option<MultiplayerCmd>,
    },
    /// Modpacks: import a Java pack, see its coverage report, port what this computer can and
    /// play it as its own profile (docs/modpacks.md)
    Modpack {
        #[command(subcommand)]
        action: Option<ModpackCmd>,
    },
    /// The mod browser: Java mods, resource packs and modpacks from Modrinth
    /// (CurseForge support is coming soon), each file checked against its checksum
    /// (docs/mod-browser.md)
    Mods {
        #[command(subcommand)]
        action: Option<ModsCmd>,
    },
    /// The portable Rust toolchain native mods compile with (docs/native-mods.md): the launcher's
    /// own, in its data folder, never installed on the system
    Rust {
        #[command(subcommand)]
        action: Option<RustCmd>,
    },
    /// Servers hosted with horizon-server (the Servers page): list them, create one, or run any
    /// horizon-server command on one
    Servers {
        #[command(subcommand)]
        action: Option<ServersCmd>,
    },
    /// Serve the Model Context Protocol on stdin and stdout, for AI agents (refused while No AI
    /// is on)
    Mcp,
    /// How to connect an AI agent (Claude Code, Codex, Cursor, any MCP client) to `mcp`: the
    /// config to paste and the command that adds it (refused while No AI is on)
    AgentConfig {
        /// Only this client
        #[arg(long, value_parser = ["claude-code", "codex", "cursor", "generic"])]
        client: Option<String>,
        /// Start the MCP server as a client would and say whether it answered
        #[arg(long)]
        test: bool,
    },
    /// Show or set No AI: on hides and turns off every AI feature (the companion, agent control,
    /// horizon-ctl and both MCP servers); off, the default, makes them available
    NoAi {
        /// Omit to show the current state
        #[arg(value_parser = ["on", "off"])]
        state: Option<String>,
    },
    /// Measure the launcher: starts (the first is the cold one), page switching, idle CPU and
    /// memory, the release list fetch; writes a benchmark report to benchmarks/ in its data folder
    Benchmark {
        /// How many times to start it
        #[arg(long, default_value_t = 5)]
        repeats: usize,
    },
}

/// The CLI's definition, for the MCP tools generated from it (cli_tools.rs).
pub fn command() -> clap::Command {
    <Cli as clap::CommandFactory>::command()
}

/// The commands that exist for AI agents: hidden and refused while No AI is on. Every other
/// command stays: the launcher, packagers and scripts need them, and none drives the game.
pub const AI_COMMANDS: [&str; 2] = ["mcp", "agent-config"];

/// The keys `settings` sets.
const SETTING_KEYS: [&str; 13] = [
    "selected-version",
    "minecraft-dir",
    "server-dir",
    "install-new-versions",
    "prune-old-versions",
    "keep-versions",
    "opening-animation",
    "proxy",
    "while-playing",
    "language",
    "accent",
    "background",
    "native-profile",
];

fn on_off(value: &str) -> Result<bool, String> {
    match value {
        "on" | "true" => Ok(true),
        "off" | "false" => Ok(false),
        _ => Err(format!("{value}: say on or off")),
    }
}

fn set_setting(s: &mut backend::Settings, key: &str, value: &str) -> Result<(), String> {
    let folder = |value: &str| (value != "default").then(|| PathBuf::from(value));
    match key {
        "selected-version" => s.selected = Some(backend::resolve_version(Some(value))?),
        "minecraft-dir" => s.minecraft_dir = folder(value),
        "server-dir" => s.server_dir = folder(value),
        "install-new-versions" => s.install_new_versions = Some(on_off(value)?),
        "prune-old-versions" => s.prune_old_versions = Some(on_off(value)?),
        "keep-versions" => {
            s.keep_versions = match value {
                "default" => None,
                n => Some(
                    n.parse::<usize>()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| format!("{value}: say a number from 1, or default"))?,
                ),
            }
        }
        "opening-animation" => s.skip_opening = !on_off(value)?,
        "while-playing" => {
            s.while_playing = backend::WhilePlaying::parse(value)
                .ok_or_else(|| format!("{value}: say stay, minimize or hide"))?
        }
        "accent" => {
            use horizon_launcher::themes::Accent;
            s.accent = Accent::parse(value).ok_or_else(|| {
                let names: Vec<_> = Accent::ALL.iter().map(|a| a.name()).collect();
                format!(
                    "{value}: say {}, a colour like #2563eb or a hue in degrees (0 to 360)",
                    names.join(", ")
                )
            })?
        }
        "background" => {
            s.background = match value.trim() {
                "default" | "" => None,
                file => Some(horizon_launcher::themes::keep_background(
                    &backend::data_dir(),
                    std::path::Path::new(file),
                )?),
            }
        }
        "language" => {
            use horizon_launcher::i18n::Language;
            s.language = match value.trim() {
                "system" | "default" => None,
                code => Some(Language::parse(code).ok_or_else(|| {
                    format!("{value}: say system, en, fr, ru, de, zh, es, uk, pl or pt-br")
                })?),
            }
        }
        "native-profile" => {
            // "default", as for the other settings, goes back to `performance`.
            s.native_profile = match value {
                "default" => None,
                _ => Some(
                    native_mods::Profile::parse(value)
                        .ok_or_else(|| format!("{value}: say performance, dev or release"))?,
                ),
            };
            s.native_release = false;
        }
        "proxy" => {
            if value == "default" || value.is_empty() {
                s.proxy = None;
            } else if let Some(why) = horizon_launcher::proxy::refuse(value) {
                return Err(why);
            } else {
                s.proxy = Some(value.trim().to_string());
            }
        }
        _ => return Err(format!("No setting {key}.")),
    }
    Ok(())
}

/// Every launcher setting, as `settings` shows them.
fn settings_json(s: &backend::Settings) -> serde_json::Value {
    serde_json::json!({
        "selected_version": backend::resolve_version(None).ok(),
        "minecraft_dir": s.minecraft_dir,
        "server_dir": backend::server_dir(None),
        "install_new_versions": s.install_new_versions.unwrap_or(true),
        "prune_old_versions": s.auto_prune(),
        "keep_versions": s.keep_newest(),
        "opening_animation": !s.skip_opening,
        "game_args": s.game_args,
        "proxy": s.proxy,
        "desktop_entry": s.desktop_entry.unwrap_or(true),
        "while_playing": s.while_playing.name(),
        "language": s.language.map_or("system", |l| l.code()),
        "accent": s.accent.name(),
        "background": horizon_launcher::themes::background(&backend::data_dir(), s.background.as_deref()),
        "native_profile": native_mods::Profile::chosen(s).name(),
        "no_ai": backend::no_ai(),
    })
}

/// Runs a subcommand and returns the exit code, or None when the window should open.
fn parse_cli_from(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Cli, clap::Error> {
    use clap::FromArgMatches;
    let no_ai = backend::no_ai();
    let mut definition = command();
    if no_ai {
        // Hidden from the help too; a call still gets the refusal below.
        for name in AI_COMMANDS {
            definition = definition.mut_subcommand(name, |c| c.hide(true));
        }
    }
    let matches = definition.try_get_matches_from(args)?;
    Cli::from_arg_matches(&matches)
}

/// Runs a subcommand and returns the exit code, or None when the window should open.
pub fn run() -> Option<i32> {
    let cli = match parse_cli_from(std::env::args_os()) {
        Ok(cli) => cli,
        Err(error) => {
            // Do not let clap call process::exit: main must drop the single-instance guard first.
            let code = error.exit_code();
            let _ = error.print();
            return Some(code);
        }
    };
    let command = cli.command?;
    let no_ai = backend::no_ai();
    let result = match command {
        Cmd::Mcp | Cmd::AgentConfig { .. } if no_ai => Err(horizon_core::no_ai::REFUSAL.into()),
        Cmd::Mcp => crate::mcp::serve(),
        command => {
            let result = execute(command, cli.json);
            // A game it started has ended (attached) or none did (an error): a session file it
            // didn't read goes now.
            account::discard_unread_sessions();
            result
        }
    };
    Some(match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    })
}

/// Download progress on one terminal line, only when stderr is a terminal and the percentage moves.
fn progress(label: String) -> impl Fn(f32) {
    let last = std::cell::Cell::new(-1);
    let tty = std::io::stderr().is_terminal();
    move |p| {
        let pct = (p * 100.) as i32;
        if !tty || pct == last.replace(pct) {
            return;
        }
        eprint!("\r{label} {pct:>3}%");
        let _ = std::io::stderr().flush();
        if p >= 1.0 {
            eprintln!();
        }
    }
}

fn json(value: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}

/// Runs a game process in this terminal and reports how it ended.
fn attached(mut cmd: Command) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("Could not start {:?}: {e}", cmd.get_program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Exited with {status}"))
    }
}

fn release_json(r: &Release) -> serde_json::Value {
    serde_json::json!({
        "tag": r.tag, "name": r.name, "date": r.date, "prerelease": r.prerelease,
        "available_here": r.asset.is_some(), "installed": backend::installed().contains(&r.tag),
    })
}

fn execute(command: Cmd, as_json: bool) -> Result<(), String> {
    if matches!(command, Cmd::Versions { .. } | Cmd::Update { .. }) {
        for (tag, from) in backend::reuse_prepared_everywhere() {
            if !as_json {
                eprintln!("Prepared {tag} with the resources of {from}.");
            }
        }
    }
    match command {
        Cmd::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut self::command(),
                "horizon-launcher",
                &mut std::io::stdout(),
            );
        }
        Cmd::Versions { installed: true } => {
            let installed = backend::installed();
            let selected = backend::resolve_version(None).ok();
            if as_json {
                json(serde_json::json!({ "installed": installed, "selected": selected }));
            } else if installed.is_empty() {
                println!("No version is installed.");
            }
            for tag in installed.iter().filter(|_| !as_json) {
                let mark = if selected.as_ref() == Some(tag) {
                    "*"
                } else {
                    " "
                };
                let state = if !backend::is_prepared(tag) {
                    "not prepared"
                } else if backend::has_sounds(tag) {
                    "prepared"
                } else {
                    "prepared, no sounds (prepare again)"
                };
                println!("{mark} {tag}  {state}");
            }
        }
        Cmd::Versions { installed: false } => {
            let releases = backend::releases()?;
            if as_json {
                json(releases.iter().map(release_json).collect());
                return Ok(());
            }
            let installed = backend::installed();
            for r in &releases {
                let state = match (installed.contains(&r.tag), r.asset.is_some()) {
                    (true, _) => "installed",
                    (false, true) => "",
                    (false, false) => "no build for this platform",
                };
                println!("{:<15} {}  {}", r.tag, r.date, state);
            }
        }
        Cmd::Install { tag } => {
            let label = format!("Installing {tag}");
            let tag = backend::install_tag(&tag, false, progress(label))?;
            if as_json {
                json(
                    serde_json::json!({ "installed": tag, "prepared": backend::is_prepared(&tag) }),
                );
            } else if backend::is_prepared(&tag) {
                println!(
                    "Installed {tag}. It is prepared already, from another installed version."
                );
            } else {
                println!(
                    "Installed {tag}. Prepare it next: horizon-launcher prepare --version {tag}"
                );
            }
        }
        Cmd::NativeBuild {
            folder,
            server_output,
            instance,
            version,
            update,
            trust_native_code,
            allow_version_mismatch,
            profile,
        } => {
            if !trust_native_code {
                return Err("Native mods run with full access to this computer. Read their source, then pass --trust-native-code.".into());
            }
            let started = std::time::Instant::now();
            let profile = match profile.as_deref().and_then(native_mods::Profile::parse) {
                Some(profile) => profile,
                None => native_mods::Profile::chosen(&backend::Settings::load()),
            };
            let steps = |step: native_mods::Step| {
                let secs = started.elapsed().as_secs();
                eprintln!("[{:>2}:{:02}] {}", secs / 60, secs % 60, step.label())
            };
            if let Some(output) = server_output {
                // Export mode: a standalone server engine in `output`. No instance is read
                // or changed, and no launcher config is written; `folder` is required (clap).
                let folder = folder.unwrap();
                let base = match &version {
                    Some(v) => backend::resolve_version(Some(v))?,
                    // The instance's version is never looked at here: the newest installed
                    // release or Nightly, or that one with `--update` (never a native build).
                    None => native_mods::base_for(None, &backend::installed(), update)
                        .ok_or_else(|| tr("No version is installed yet."))?,
                };
                let built = native_mods::build_server(
                    &base,
                    &folder,
                    &output,
                    allow_version_mismatch,
                    profile,
                    steps,
                )
                .map_err(|e| match e.starts_with(native_mods::MISMATCH) {
                    true => trf(
                        "{error} They often still build: pass --allow-version-mismatch to try.",
                        &[("error", e.as_str())],
                    ),
                    false => e,
                })?;
                let built = std::path::absolute(&built).unwrap_or(built);
                let path = built.display().to_string();
                println!(
                    "{}",
                    trf("Built the server engine: {path}", &[("path", &path)])
                );
                println!(
                    "{}",
                    tr(
                        "Point the server at it: set engine.path to this file in the server's \
                        server.toml (docs/hosting.md); the launcher writes nothing itself."
                    )
                );
                return Ok(());
            }
            let target = match &instance {
                Some(name) => instances::find(name)?,
                None => instances::selected(),
            };
            let base = match version {
                Some(v) => backend::resolve_version(Some(&v))?,
                None => {
                    let plays = target.resolve_version(None).ok();
                    native_mods::base_for(plays.as_deref(), &backend::installed(), update)
                        .ok_or("No version is installed yet.")?
                }
            };
            let folder = match folder {
                Some(folder) => folder,
                None => native_mods::installed_mods(&target.home)?,
            };
            let built = native_mods::build(
                &base,
                &folder,
                &target,
                allow_version_mismatch,
                profile,
                steps,
            )
            .map_err(|e| match e.starts_with(native_mods::MISMATCH) {
                true => {
                    format!("{e} They often still build: pass --allow-version-mismatch to try.")
                }
                false => e,
            })?;
            println!(
                "{built} built and selected for its instance ({:.0} s).",
                started.elapsed().as_secs_f64()
            );
        }
        Cmd::Remove { tag } => {
            backend::remove_version(&tag)?;
            println!("Removed {tag}.");
        }
        Cmd::ImportBuild {
            path,
            version,
            trust_unsigned,
        } => {
            if !trust_unsigned {
                return Err("An imported build is unsigned: Horizon can't check where it came from. If you trust it, pass --trust-unsigned.".into());
            }
            let tag = build_import::import(&path, version.as_deref())?;
            let prepared = backend::is_prepared(&tag);
            if as_json {
                json(serde_json::json!({ "imported": tag, "prepared": prepared }));
            } else if prepared {
                println!("Imported {tag} and selected it. It is prepared already, from another installed version.");
            } else {
                println!("Imported {tag} and selected it. Prepare it next: horizon-launcher prepare --version {tag}");
            }
        }
        Cmd::Prepare {
            minecraft,
            download,
            version,
        } => {
            let tag = backend::resolve_version(version.as_deref())?;
            let minecraft = backend::minecraft_dir(minecraft)?;
            let from = if download {
                backend::MinecraftFrom::Mojang
            } else {
                backend::MinecraftFrom::Install
            };
            let (preparer, minecraft) =
                backend::preparer(&tag, &minecraft, from, &|line| eprintln!("{line}"))?;
            backend::sounds(&tag, &minecraft, &|line| println!("{line}"));
            match preparer {
                backend::Preparer::Process(cmd) => attached(cmd)?,
                backend::Preparer::InProcess(options) => {
                    backend::prepare_in_process(&options, &|line| println!("{line}"))?
                }
            }
            println!("{tag} is prepared.");
        }
        Cmd::Play {
            version,
            instance,
            world,
            creative,
            join,
            server,
            arg,
            account,
        } => {
            let instance = match instance {
                Some(name) => instances::find(&name)?,
                None => instances::selected(),
            };
            if instance.java.is_some() {
                if version.is_some()
                    || world.is_some()
                    || join.is_some()
                    || creative
                    || account.is_some()
                {
                    return Err(format!(
                        "{} plays Minecraft: Java Edition: --version, --world, --creative, \
                         --join and --account are for Horizon instances.",
                        instance.name
                    ));
                }
                let server = server.map(|server| {
                    let list = server_list::List::load(&instance.home);
                    server_list::find(&list, &server).map_or(server, |(_, s)| s.ip)
                });
                let mut cmd =
                    backend::java_game(&instance, server.as_deref(), &|line| eprintln!("{line}"))?;
                if let Some(e) = account::take_unsaved() {
                    eprintln!("{e}");
                }
                cmd.args(arg);
                let vars = instance.vars(cmd.get_program());
                cmd.envs(vars.iter().map(|(k, v)| (k, v)));
                let started = std::time::Instant::now();
                let played = attached(horizon_launcher::launch::wrap(cmd, &instance.launch));
                if let Err(e) = instances::record_play(&instance.id, started.elapsed()) {
                    eprintln!("Play time not saved: {e}");
                }
                after_exit(&instance, &vars);
                return played;
            }
            // Who plays this launch; the accounts below choose it in memory only, for its
            // `--name` and look.
            let mut accounts = Accounts::load();
            let who = match account {
                Some(who) => Some(account::Player::Microsoft(
                    accounts
                        .find(&who)
                        .ok_or_else(|| {
                            format!("No signed-in account {who}. See `horizon-launcher account`.")
                        })?
                        .uuid
                        .clone(),
                )),
                None => None,
            };
            if let Some(who) = &who {
                accounts.play_as(account::store(), who.clone())?;
            }
            let mut cmd =
                backend::client_as(&instance.resolve_version(version.as_deref())?, who.as_ref())?;
            instances::apply(&mut cmd, &instance)?;
            if let Some(code) = join {
                backend::join_friend(&mut cmd, &code)?;
            }
            if let Some(world) = world {
                cmd.arg("--world").arg(world);
            }
            if let Some(server) = server {
                let list = server_list::List::load(&instance.home);
                let ip = server_list::find(&list, &server).map_or(server, |(_, s)| s.ip);
                cmd.arg("--server").arg(ip);
            }
            if creative {
                cmd.arg("--creative");
            }
            cmd.args(arg);
            let started = std::time::Instant::now();
            if let Some(name) = accounts.name_arg() {
                cmd.arg("--name").arg(name);
            }
            cmd.envs(accounts.look_env());
            let vars = instance.vars(cmd.get_program());
            cmd.envs(vars.iter().map(|(k, v)| (k, v)));
            horizon_launcher::launch::limit_memory(&mut cmd, &instance.launch);
            let played = attached(horizon_launcher::launch::wrap(cmd, &instance.launch));
            if let Err(e) = instances::record_play(&instance.id, started.elapsed()) {
                eprintln!("Play time not saved: {e}");
            }
            after_exit(&instance, &vars);
            played?
        }
        Cmd::Server {
            dir,
            version,
            accept_eula,
        } => {
            let tag = backend::resolve_version(version.as_deref())?;
            let dir = backend::server_dir(dir);
            // The server inherits this terminal, so typed lines reach its console.
            attached(backend::server(&tag, &dir, accept_eula)?)?
        }
        Cmd::Notes { tag } => {
            let r = backend::release_notes(tag.as_deref())?;
            if as_json {
                json(
                    serde_json::json!({ "tag": r.tag, "name": r.name, "date": r.date, "notes": r.body }),
                );
            } else {
                let title = if r.name.is_empty() { &r.tag } else { &r.name };
                println!(
                    "{title}\n{}  {}\n\n{}",
                    r.tag,
                    r.date,
                    backend::markdown_to_text(&r.body)
                );
            }
        }
        Cmd::Account { action } => account_command(action.unwrap_or(AccountCmd::List), as_json)?,
        Cmd::Benchmark { repeats } => {
            let path = crate::perf::benchmark(repeats.max(1))?;
            if as_json {
                json(serde_json::json!({ "report": path }));
            } else {
                println!("{}", path.display());
            }
        }
        Cmd::Links => {
            let links = backend::links();
            if as_json {
                json(serde_json::to_value(&links).unwrap());
            } else {
                for (name, url) in [
                    ("Discord", &links.discord),
                    ("Sponsor", &links.sponsor),
                    ("Website", &links.website),
                ] {
                    println!("{name:<8} {}", if url.is_empty() { "(none)" } else { url });
                }
            }
        }
        Cmd::NoAi { state } => {
            if let Some(state) = state {
                backend::set_no_ai(state == "on")?;
            }
            let on = backend::no_ai();
            if as_json {
                json(serde_json::json!({ "no_ai": on }));
            } else if on {
                println!("No AI is on: every AI feature is hidden and off.");
            } else {
                println!("No AI is off: AI features are available.");
            }
        }
        Cmd::AgentConfig { test: true, .. } => {
            let tools = horizon_launcher::agents::self_test(&backend::update_target())?;
            if as_json {
                json(serde_json::json!({ "answered": true, "tools": tools }));
            } else {
                println!("The MCP server answered with {tools} tools.");
            }
        }
        Cmd::AgentConfig { client, .. } => {
            use horizon_launcher::agents;
            let exe = &backend::update_target();
            if as_json {
                let mut all = agents::json(exe);
                if let Some(id) = &client {
                    all["clients"] = all["clients"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|c| c["id"] == id.as_str())
                        .cloned()
                        .collect();
                }
                json(all);
                return Ok(());
            }
            for c in agents::clients(exe)
                .iter()
                .filter(|c| client.as_ref().is_none_or(|id| id == c.id))
            {
                println!("{}", c.name);
                if let Some(line) = c.add_line() {
                    println!("  Add it with:  {line}");
                }
                println!("  Or put this in {}:", c.file);
                for line in c.config.lines() {
                    println!("    {line}");
                }
                println!();
            }
            println!("Then ask your agent to say hello: it can call the links tool, or play.");
        }
        Cmd::Settings { key, value } => {
            let mut settings = backend::Settings::load();
            if let Some(key) = key {
                let value = value.ok_or("settings KEY VALUE: the value is missing")?;
                set_setting(&mut settings, &key, &value)?;
                settings.save()?;
                horizon_launcher::proxy::announce(settings.proxy.clone());
            }
            let all = settings_json(&settings);
            if as_json {
                json(all);
            } else {
                for (key, value) in all.as_object().into_iter().flatten() {
                    println!("{key}: {value}");
                }
            }
        }
        Cmd::GameArgs { args } => {
            let mut settings = backend::Settings::load();
            if let Some(line) = args {
                settings.game_args = line;
                settings.save()?;
            }
            let line = settings.game_args;
            if as_json {
                json(serde_json::json!({
                    "game_args": line,
                    "parsed": backend::split_args(&line),
                }));
            } else if line.is_empty() {
                println!("No extra game arguments.");
            } else {
                println!("Game arguments: {line}");
            }
        }
        Cmd::ShareLog { instance } => {
            use horizon_launcher::game_log;
            let i = match instance {
                Some(name) => instances::find(&name)?,
                None => instances::selected(),
            };
            let path = i.home.join("logs/latest.log");
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let names = game_log::names(&[&i.home, &backend::game_home()]);
            let text = game_log::private(&text, dirs::home_dir().as_deref(), &names);
            let shared = game_log::share(&text)?;
            if as_json {
                json(serde_json::json!({
                    "url": shared.url, "delete": format!("{}:{}", shared.id, shared.token),
                }));
            } else {
                println!("{}", shared.url);
                println!(
                    "Delete it: horizon-launcher unshare-log {}:{}",
                    shared.id, shared.token
                );
            }
        }
        Cmd::UnshareLog { shared } => {
            let (id, token) = shared
                .split_once(':')
                .ok_or("Give the \"<id>:<token>\" share-log printed.")?;
            horizon_launcher::game_log::unshare(&horizon_launcher::game_log::Shared {
                url: String::new(),
                id: id.into(),
                token: token.into(),
            })?;
            println!("Deleted the shared log {id}.");
        }
        Cmd::Logs { lines, crash } => {
            let (path, text) = backend::read_logs(lines, crash)?;
            if as_json {
                json(serde_json::json!({ "path": path, "text": text }));
            } else {
                print!("{text}");
            }
        }
        Cmd::Update { check } => update(check, as_json)?,
        Cmd::Data { action } => data(action.unwrap_or(DataCmd::Status), as_json)?,
        Cmd::DesktopEntry { remove } => {
            let mut settings = backend::Settings::load();
            settings.desktop_entry = Some(!remove);
            settings.save()?;
            if remove {
                horizon_launcher::desktop::remove()?;
                println!("Removed Horizon from the applications menu.");
            } else {
                horizon_launcher::desktop::install(backend::exe_path())?;
                println!("Horizon is in the applications menu.");
            }
        }
        Cmd::Worlds { action } => worlds(
            action.unwrap_or(WorldsCmd::List { instance: None }),
            as_json,
        )?,
        Cmd::Packs { action } => game_packs(
            action.unwrap_or(PacksCmd::List {
                kind: None,
                instance: None,
            }),
            as_json,
        )?,
        Cmd::Screenshots { action } => screenshots(
            action.unwrap_or(ScreenshotsCmd::List { instance: None }),
            as_json,
        )?,
        Cmd::Multiplayer { action } => multiplayer(
            action.unwrap_or(MultiplayerCmd::List { instance: None }),
            as_json,
        )?,
        Cmd::Instance { action } => instance(action.unwrap_or(InstanceCmd::List), as_json)?,
        Cmd::Modpack { action } => modpack(action.unwrap_or(ModpackCmd::List), as_json)?,
        Cmd::Mods { action } => mods(action.unwrap_or(ModsCmd::List { instance: None }), as_json)?,
        Cmd::Servers { action } => servers(action.unwrap_or(ServersCmd::List), as_json)?,
        Cmd::Rust { action } => rust(action.unwrap_or(RustCmd::Status), as_json)?,
        Cmd::Java { action } => java(action.unwrap_or(JavaCmd::List), as_json)?,
        Cmd::Mcp => unreachable!(),
    }
    Ok(())
}

fn java(action: JavaCmd, as_json: bool) -> Result<(), String> {
    use horizon_launcher::java;
    match action {
        JavaCmd::List => {
            let mut all = java::mojang_runtimes(&java::runtimes_dir());
            all.extend(java::installed());
            if as_json {
                json(serde_json::json!(all));
            } else if all.is_empty() {
                println!("No Java yet. Horizon downloads Mojang's when an instance needs it.");
            } else {
                for r in &all {
                    let from = match r.source {
                        java::Source::Mojang => "Mojang's, downloaded by Horizon",
                        java::Source::Installed => "installed",
                    };
                    println!("Java {}  ({from})\n  {}", r.version, r.home.display());
                }
            }
        }
        JavaCmd::Install { java: which } => {
            let need = java::Need::named(&which)?;
            let runtime = java::install(&need, &|line| eprintln!("{line}"))?;
            if as_json {
                json(serde_json::json!(runtime));
            } else {
                println!(
                    "Java {} is ready: {}",
                    runtime.version,
                    runtime.home.display()
                );
            }
        }
    }
    Ok(())
}

fn rust(action: RustCmd, as_json: bool) -> Result<(), String> {
    use horizon_launcher::rust_toolchain as rust;
    match action {
        RustCmd::Status => {
            let installed = rust::installed();
            let version = installed.as_ref().map(|t| t.version()).transpose()?;
            let linker = rust::linker().err();
            if as_json {
                json(serde_json::json!({
                    "pinned": rust::VERSION,
                    "host": rust::host().map(|h| h.target),
                    "installed": installed.as_ref().map(|t| t.dir.display().to_string()),
                    "version": version,
                    "linker": linker,
                }));
            } else {
                match (&installed, &version) {
                    (Some(t), Some(v)) => println!("{v} in {}", t.dir.display()),
                    _ => println!(
                        "Rust {} is not installed (`horizon-launcher rust install`).",
                        rust::VERSION
                    ),
                }
                println!(
                    "{}",
                    linker.unwrap_or_else(|| "This computer can link the game.".into())
                );
            }
        }
        RustCmd::Install => {
            let shown = std::cell::Cell::new(0);
            let toolchain = rust::install(|f| {
                let percent = (f * 100.0) as u32;
                if percent >= shown.get() + 10 {
                    shown.set(percent);
                    eprintln!("{percent}%");
                }
            })?;
            println!("{} in {}", toolchain.version()?, toolchain.dir.display());
        }
        RustCmd::Remove => {
            rust::remove()?;
            println!("Removed the launcher's Rust toolchain and Cargo home.");
        }
    }
    Ok(())
}

fn update(check: bool, as_json: bool) -> Result<(), String> {
    let updates = backend::check_updates()?;
    if as_json && check {
        json(serde_json::json!({
            "horizon": updates.horizon.as_ref().map(|r| &r.tag),
            "launcher": updates.launcher.as_ref().and_then(|r| backend::launcher_version(&r.tag)),
            "launcher_current": backend::VERSION,
        }));
        return Ok(());
    }
    match &updates.horizon {
        None => println!("Horizon: up to date."),
        Some(r) if check => println!("Horizon: {} is available.", r.tag),
        Some(r) => {
            let tag =
                backend::install_tag(&r.tag, true, progress(format!("Installing {}", r.tag)))?;
            println!("Horizon: installed {tag}.");
        }
    }
    match &updates.launcher {
        None => println!("Launcher: {} is up to date.", backend::VERSION),
        Some(r) => {
            let version = backend::launcher_version(&r.tag).unwrap_or_default();
            if check {
                println!(
                    "Launcher: {version} is available (this is {}).",
                    backend::VERSION
                );
            } else if !backend::self_update_allowed() {
                println!("Launcher: {version} is available; this build does not replace itself.");
            } else {
                backend::self_update(r, progress(format!("Updating the launcher to {version}")))?;
                println!("Launcher: updated to {version}. Restart the launcher to use it.");
            }
        }
    }
    Ok(())
}

/// `account …`: names and UUIDs only; tokens are never printed.
fn account_command(action: AccountCmd, as_json: bool) -> Result<(), String> {
    let mut accounts = Accounts::load();
    let find = |accounts: &Accounts, who: &str| {
        accounts
            .find(who)
            .cloned()
            .ok_or_else(|| format!("No signed-in account {who}. See `horizon-launcher account`."))
    };
    match action {
        AccountCmd::List => {
            if as_json {
                json(serde_json::json!({
                    "accounts": accounts.accounts.iter().map(|a| serde_json::json!({
                        "name": a.name, "uuid": a.uuid,
                        "selected": accounts.selected.as_ref() == Some(&a.uuid),
                        "authentication": account::session::status(account::store(), &a.uuid),
                    })).collect::<Vec<_>>(),
                    "offline": accounts.offline.iter().map(|n| serde_json::json!({
                        "name": n,
                        "selected": accounts.player() == account::Player::Offline(n.clone()),
                    })).collect::<Vec<_>>(),
                    "sign_in_configured": account::client_id().is_some(),
                    "owner": accounts.has_owner(account::store()),
                }));
            } else if accounts.accounts.is_empty() && accounts.offline.is_empty() {
                println!(
                    "No Microsoft account signed in. Play needs one that owns Minecraft: Java \
                     Edition: horizon-launcher account sign-in"
                );
            } else {
                for a in &accounts.accounts {
                    let mark = if accounts.selected.as_ref() == Some(&a.uuid) {
                        "*"
                    } else {
                        " "
                    };
                    let status = account::session::status(account::store(), &a.uuid);
                    println!("{mark} {}  {}  {}", a.name, a.uuid, status.label());
                }
                for n in &accounts.offline {
                    let on = accounts.player() == account::Player::Offline(n.clone());
                    println!("{} {n}  offline", if on { "*" } else { " " });
                }
                if accounts.player() == account::Player::GameName {
                    println!(
                        "Playing offline as the game's own name (`account select NAME` to switch)."
                    );
                }
                if !accounts.has_owner(account::store()) {
                    println!("{}.", account::NEEDS_OWNER);
                }
            }
        }
        AccountCmd::Refresh { who } => {
            let status = account::session::refresh(&who)?;
            if as_json {
                json(serde_json::to_value(&status).map_err(|e| e.to_string())?);
            } else if status.state != account::session::State::Ready {
                return Err(format!("{}: {}", status.label(), status.message));
            } else {
                println!("{}: {}", status.label(), status.message);
            }
        }
        AccountCmd::Session { who } => {
            let dir = backend::game_home().join("sessions");
            match (account::session::renew_for_game(&dir, &who), as_json) {
                (Ok(file), true) => json(serde_json::json!({ "session_file": file })),
                (Ok(file), false) => println!("{}", file.display()),
                (Err(status), true) => json(serde_json::json!({ "authentication": status })),
                (Err(status), false) => return Err(status.message),
            }
        }
        AccountCmd::SignIn {
            target: Some(who),
            provider,
        } => {
            let a = find(&accounts, &who)?;
            let sign_in = if provider == "elyby" {
                account::SignIn::start_ely_for(a.clone())
            } else {
                account::SignIn::start_for(a.clone())
            };
            let mut shown = false;
            let signed_in = loop {
                match sign_in.state() {
                    account::SignInState::Code(code) if !shown => {
                        shown = true;
                        if as_json {
                            json(serde_json::json!({
                                "link": code.verification_uri,
                                "code": code.user_code,
                            }));
                        } else if code.user_code.is_empty() {
                            eprintln!(
                                "To sign {} in again, open {} in a browser",
                                a.name, code.verification_uri
                            );
                        } else {
                            eprintln!(
                                "To sign {} in again, open {} in a browser and enter the code {}",
                                a.name, code.verification_uri, code.user_code
                            );
                        }
                    }
                    account::SignInState::Done(result) => break result?,
                    _ => {}
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            };
            if as_json {
                json(serde_json::json!({ "signed_in": signed_in.uuid }));
            } else {
                println!("Signed in as {} again.", signed_in.name);
            }
        }
        AccountCmd::SignIn {
            target: None,
            provider,
        } => {
            if provider == "elyby" {
                let sign_in = account::SignIn::start_ely();
                let mut shown = false;
                let signed_in = loop {
                    match sign_in.state() {
                        account::SignInState::Code(code) if !shown => {
                            shown = true;
                            if as_json {
                                json(serde_json::json!({
                                    "link": code.verification_uri,
                                    "code": code.user_code,
                                }));
                            } else if code.user_code.is_empty() {
                                eprintln!(
                                    "To sign in with Ely.by, open {} in a browser",
                                    code.verification_uri
                                );
                            } else {
                                eprintln!(
                                    "To sign in with Ely.by, open {} in a browser and enter the code {}",
                                    code.verification_uri, code.user_code
                                );
                            }
                        }
                        account::SignInState::Done(result) => break result?,
                        _ => {}
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                };
                if as_json {
                    json(serde_json::json!({ "signed_in": signed_in.uuid }));
                } else {
                    println!("Signed in as {}.", signed_in.name);
                }
                return Ok(());
            }
            let client_id = account::client_id().ok_or(account::NOT_CONFIGURED)?;
            let ep = account::Endpoints::live();
            let code = account::device_code(&ep, &client_id)?;
            eprintln!(
                "To sign in, open {} in a browser and enter the code {}",
                code.verification_uri, code.user_code
            );
            let ms = account::wait_for_sign_in(&ep, &client_id, &code, &|| false)?;
            let (signed_in, secrets) = account::finish_sign_in(&ep, ms)?;
            accounts.add(account::store(), signed_in.clone(), &secrets)?;
            accounts.save()?;
            println!("Signed in as {}.", signed_in.name);
            if let Some(said) = account::no_keychain() {
                eprintln!("{said}");
            }
        }
        AccountCmd::SignOut { who } => {
            let a = find(&accounts, &who)?;
            accounts.remove(account::store(), &a.uuid)?;
            accounts.save()?;
            println!("Signed out {}.", a.name);
        }
        AccountCmd::Select { who } => {
            let offline = accounts
                .offline
                .iter()
                .find(|n| n.eq_ignore_ascii_case(&who))
                .cloned();
            let (player, name) = match (accounts.find(&who).cloned(), offline) {
                (Some(a), _) => (account::Player::Microsoft(a.uuid), a.name),
                (None, Some(n)) => (
                    account::Player::Offline(n.clone()),
                    format!("{n} (offline)"),
                ),
                (None, None) => return Err(find(&accounts, &who).unwrap_err()),
            };
            accounts.play_as(account::store(), player)?;
            accounts.save()?;
            println!("The game plays as {name}.");
        }
        AccountCmd::Offline { name } => {
            let player = match name {
                Some(n) => account::Player::Offline(n),
                None => account::Player::GameName,
            };
            accounts.play_as(account::store(), player)?;
            accounts.save()?;
            match accounts.name_arg() {
                Some(n) => println!("The game plays offline as {n}."),
                None => println!("The game plays offline, as the name it made up."),
            }
        }
        AccountCmd::Skin {
            name,
            file,
            model,
            cape,
            no_skin,
            no_cape,
        } => {
            use horizon_launcher::skins::{self, Model};
            let name = accounts
                .offline
                .iter()
                .find(|n| n.eq_ignore_ascii_case(&name))
                .cloned()
                .ok_or_else(|| {
                    format!("No offline name {name}: add it with `account offline {name}`.")
                })?;
            let model = match model.as_deref() {
                None => None,
                Some("classic") => Some(Model::Classic),
                Some("slim") => Some(Model::Slim),
                Some(m) => return Err(format!("{m}: say classic or slim")),
            };
            let mut look = accounts.look(&name);
            if let Some(file) = &file {
                skins::keep(&skins::dir(), &name, file, false)?;
                look.skin = true;
            }
            if let Some(file) = &cape {
                skins::keep(&skins::dir(), &name, file, true)?;
                look.cape = true;
            }
            if let Some(m) = model {
                look.model = m;
            }
            look.skin &= !no_skin;
            look.cape &= !no_cape;
            accounts
                .looks
                .insert(name.to_ascii_lowercase(), look.clone());
            accounts.save()?;
            if as_json {
                json(serde_json::json!({
                    "name": name, "skin": look.skin, "cape": look.cape, "model": look.model.name(),
                }));
            } else {
                let say = |on: bool, what: &str| if on { what.to_string() } else { "none".into() };
                println!(
                    "{name}: skin {} ({} arms), cape {}",
                    say(look.skin, "kept"),
                    look.model.name(),
                    say(look.cape, "kept")
                );
            }
        }
        AccountCmd::ForgetOffline { name } => {
            accounts.remove_offline(&name, &horizon_launcher::skins::dir())?;
            accounts.save()?;
            println!("Forgot the offline name {name}.");
        }
    }
    Ok(())
}

/// `data`: the same consent as Settings → Data collection.
fn data(action: DataCmd, as_json: bool) -> Result<(), String> {
    use horizon_launcher::telemetry;
    let review = |item: &horizon_telemetry::Item, yes: bool| -> Result<bool, String> {
        println!(
            "{}",
            serde_json::to_string_pretty(item).map_err(|e| e.to_string())?
        );
        if yes {
            return Ok(true);
        }
        if !std::io::stdin().is_terminal() {
            return Err("Add --yes to send it without asking.".into());
        }
        eprint!("Send this? [y/N] ");
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        Ok(line.trim().eq_ignore_ascii_case("y"))
    };
    match action {
        DataCmd::Status => {
            let status = telemetry::status_json();
            if as_json {
                json(status);
                return Ok(());
            }
            let consent = telemetry::consent();
            if consent.needs_asking() {
                println!("Not answered yet: nothing is collected.");
            }
            for (kind, name, _) in telemetry::SWITCHES {
                let on = consent.allows(kind);
                println!("{name}: {}", if on { "on" } else { "off" });
            }
            println!("Waiting to be sent: {}", status["waiting"]);
            match horizon_telemetry::endpoint() {
                Some(url) => println!("Server: {url}"),
                None => println!("Server: turned off (HORIZON_TELEMETRY_URL=off), nothing is sent"),
            }
        }
        DataCmd::Allow { kinds } => {
            telemetry::set(&telemetry::parse_kinds(&kinds)?, true)?;
            return data(DataCmd::Status, as_json);
        }
        DataCmd::Deny { kinds } => {
            telemetry::set(&telemetry::parse_kinds(&kinds)?, false)?;
            return data(DataCmd::Status, as_json);
        }
        DataCmd::Show => {
            let (what, text) = telemetry::preview();
            eprintln!("{what}");
            println!("{text}");
        }
        DataCmd::Delete => println!("{}", telemetry::delete_my_data()?),
        DataCmd::Send => println!("{:?}", telemetry::send()),
        DataCmd::Bug { text, log, yes } => {
            telemetry::ensure_allowed(horizon_telemetry::Kind::Bug)?;
            let item = telemetry::bug_report(&text, log)?;
            if review(&item, yes)? {
                println!("{}", telemetry::submit(&item)?);