//! N03 — the compatibility facade the next-generation shell calls.
//!
//! The new shell should not depend on the current module layout. These use cases name the work
//! ("finish an Ely.by sign-in", "a Nightly's label", "the share a tailnet policy grants") and
//! delegate to the existing service. None of them changes behaviour; the N01 preservation tests
//! still call the same entry points underneath.
//!
//! The accounts boundary is the important one: the shell receives a [`accounts::AccountCard`] or
//! a [`accounts::SignIn`] and never a [`crate::account::Secrets`]. A sign-in can be persisted
//! through [`accounts::SignIn::add_to`], so no screen needs to read or write a token.

use crate::account::{Account, Accounts, Endpoints, MsTokens, SecretStore};
use crate::backend as launcher_backend;
use crate::config as launcher_config;
use crate::ely;
use crate::instances as launcher_instances;
use crate::launch as launch_engine;
use crate::native_status as native_engine;
use crate::nightly as nightly_engine;
use crate::servers as server_engine;
use crate::tailscale as tailscale_engine;

/// Accounts, providers and safe sign-in state.
pub mod accounts {
    use super::*;
    use std::fmt;

    /// Who signed in. `Unavailable` means the stored sign-in could not be read (a locked
    /// keychain, for example), not that the account is offline.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Provider {
        Microsoft,
        ElyBy,
        Unavailable,
    }

    pub use crate::account::session::{
        Phase as SignInPhase, State as SessionState, Status as SessionStatus,
    };
    pub use crate::account::Player;

    /// What a screen may show about an account: its identity, provider and safe status. It has
    /// no secrets, so it can be cloned, serialized or logged.
    #[derive(Debug, Clone)]
    pub struct AccountCard {
        pub uuid: String,
        pub name: String,
        pub provider: Provider,
        pub status: SessionStatus,
    }

    /// The card for `account`, reading `store` (use [`Self::card`] for the real store).
    pub fn card_in(store: &dyn SecretStore, account: &Account) -> AccountCard {
        let provider = match crate::account::secrets_in(store, &account.uuid) {
            Some(secrets) if crate::account::is_ely(&secrets) => Provider::ElyBy,
            Some(_) => Provider::Microsoft,
            None => Provider::Unavailable,
        };
        AccountCard {
            uuid: account.uuid.clone(),
            name: account.name.clone(),
            provider,
            status: crate::account::session::status(store, &account.uuid),
        }
    }

    /// The card for `account`, from the launcher's keychain or secrets file.
    pub fn card(account: &Account) -> AccountCard {
        card_in(crate::account::store(), account)
    }

    /// Renew an account's session, as the Account screen does; the result is safe to show.
    pub fn refresh(account: &Account) -> Result<SessionStatus, String> {
        crate::account::session::refresh(&account.uuid)
    }

    /// Sign out: the secrets leave the store, the account the list.
    pub fn sign_out(
        accounts: &mut Accounts,
        store: &dyn SecretStore,
        uuid: &str,
    ) -> Result<(), String> {
        accounts.remove(store, uuid)
    }

    /// Whether any account was confirmed to own the game. Offline names and Ely.by do not.
    pub fn has_owner(accounts: &Accounts, store: &dyn SecretStore) -> bool {
        accounts.has_owner(store)
    }

    /// Choose who plays.
    pub fn play_as(
        accounts: &mut Accounts,
        store: &dyn SecretStore,
        who: Player,
    ) -> Result<(), String> {
        accounts.play_as(store, who)
    }

    /// A finished sign-in, ready to keep. The secrets stay private; `add_to` writes them through
    /// `Accounts`, exactly as `account::finish_sign_in` and `account::finish_ely_for_cli` do.
    pub struct SignIn {
        pub account: Account,
        pub provider: Provider,
        /// Whether Horizon lets this account play (owns the game, or Ely.by's `plays`).
        pub can_play: bool,
        /// Whether the Minecraft services confirmed ownership. Always false for Ely.by.
        pub owns_minecraft: bool,
        secrets: crate::account::Secrets,
    }

    impl SignIn {
        /// A finished Ely.by sign-in.
        pub fn finish_ely(tokens: ely::Tokens) -> Self {
            let (account, secrets) = crate::account::finish_ely_for_cli(tokens);
            Self::new(account, secrets, Provider::ElyBy)
        }

        /// A finished Microsoft sign-in. Live Microsoft/Xbox/Minecraft calls happen here, as in
        /// `account::finish_sign_in`; this adds no retry or timeout of its own.
        pub fn finish_microsoft(ep: &Endpoints, ms: MsTokens) -> Result<Self, String> {
            let (account, secrets) = crate::account::finish_sign_in(ep, ms)?;
            Ok(Self::new(account, secrets, Provider::Microsoft))
        }

        fn new(account: Account, secrets: crate::account::Secrets, provider: Provider) -> Self {
            Self {
                can_play: secrets.plays(),
                owns_minecraft: secrets.owns_minecraft(),
                account,
                provider,
                secrets,
            }
        }

        /// Keep the account and its secrets, and select it, as the current sign-in flows do.
        pub fn add_to(
            self,
            accounts: &mut Accounts,
            store: &dyn SecretStore,
        ) -> Result<(), String> {
            accounts.add(store, self.account, &self.secrets)
        }
    }

    impl fmt::Debug for SignIn {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("SignIn")
                .field("account", &self.account)
                .field("provider", &self.provider)
                .field("can_play", &self.can_play)
                .field("owns_minecraft", &self.owns_minecraft)
                .field("secrets", &"<hidden>")
                .finish()
        }
    }
}

/// Release and Nightly classification.
pub mod versions {
    use super::*;
    use std::cmp::Ordering;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TagKind {
        /// A game build (`v2026.10.13`).
        Game,
        /// A launcher release (`launcher@0.2.54`, or the old `launcher-v0.2.54`).
        Launcher,
        /// A Nightly (`nightly-2026-10-28-0c02067`).
        Nightly,
        /// Anything else: not offered as an update.
        Other,
    }

    pub fn kind(tag: &str) -> TagKind {
        if nightly_engine::date_of(tag).is_some() {
            TagKind::Nightly
        } else if launcher_backend::is_game_tag(tag) {
            TagKind::Game
        } else if launcher_backend::launcher_version(tag).is_some() {
            TagKind::Launcher
        } else {
            TagKind::Other
        }
    }

    /// The order the updater uses: newer is greater.
    pub fn order(a: &str, b: &str) -> Ordering {
        launcher_backend::compare_versions(a, b)
    }

    /// `Nightly · 4baa571` for a Nightly version, `None` for anything else.
    pub fn nightly_label(version: &str) -> Option<String> {
        nightly_engine::label(version)
    }

    /// The date of a Nightly version, `None` for anything else.
    pub fn nightly_date(version: &str) -> Option<&str> {
        nightly_engine::date_of(version)
    }
}

/// Launch options the player edits.
pub mod launch {
    use super::*;
    use std::collections::BTreeMap;

    pub use crate::launch::NotMemory;

    /// A typed memory limit: `None` means no limit, `Err` means the text is not one.
    pub fn memory(typed: &str) -> Result<Option<u32>, NotMemory> {
        launch_engine::parse_memory(typed)
    }

    /// The memory a slider fraction of `total_mb` picks.
    pub fn memory_at(fraction: f32, total_mb: u32) -> Option<u32> {
        launch_engine::memory_at(fraction, total_mb)
    }

    /// `NAME=value` words as the game receives them.
    pub fn env(text: &str) -> Result<BTreeMap<String, String>, String> {
        launch_engine::parse_env(text)
    }

    /// The variables as the editor shows them.
    pub fn env_text(env: &BTreeMap<String, String>) -> String {
        launch_engine::env_line(env)
    }
}

/// Instance facts a list or detail screen shows.
pub mod instances {
    use super::*;
    use std::time::Duration;

    pub use crate::instances::Meta;

    /// `12 min`, `3 h 05 min`, `140 h`.
    pub fn play_time(seconds: u64) -> String {
        launcher_instances::play_time(Duration::from_secs(seconds))
    }

    /// `meta.json` from an older launcher: missing fields take their defaults.
    pub fn meta(json: &str) -> Result<Meta, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

/// Installed content.
pub mod content {
    use std::path::Path;

    pub use crate::browser::{Project, Query, Version};
    pub use crate::packs::Kind as PackKind;

    /// Search Horizon's registry for content-only .hmod mods.
    ///
    /// Native .hnmod entries (and future native .hmod entries) stay in the native-mod
    /// workflow; the registry's shared browser kind is not enough to classify them.
    pub fn registry_mods(query: &Query) -> Result<Vec<Project>, String> {
        if query.kind != crate::browser::Kind::Mod {
            return Err("A Horizon content-mod search must use the Mods kind.".into());
        }
        let source = crate::browser::source(crate::browser::SourceId::Horizon)
            .ok_or_else(|| "Horizon registry is unavailable.".to_owned())?;
        let projects = crate::browser::Source::search(&*source, query)?;
        Ok(projects
            .into_iter()
            .filter(|project| !crate::browser::horizon::is_native_mod(&project.id))
            .collect())
    }

    /// Look up a content-only registry mod by its stable project ID.
    pub fn registry_mod(id: &str) -> Result<Project, String> {
        let source = crate::browser::source(crate::browser::SourceId::Horizon)
            .ok_or_else(|| "Horizon registry is unavailable.".to_owned())?;
        let project = crate::browser::Source::project(&*source, id)?;
        if crate::browser::horizon::is_native_mod(id) {
            return Err(
                "This project is a native mod; open it in the Native Mods workspace.".into(),
            );
        }
        Ok(project)
    }

    /// The registry versions for a previously looked-up content mod.
    pub fn registry_mod_versions(project: &Project) -> Result<Vec<Version>, String> {
        if project.kind != crate::browser::Kind::Mod {
            return Err("This project is not a Horizon mod.".into());
        }
        let source = crate::browser::source(crate::browser::SourceId::Horizon)
            .ok_or_else(|| "Horizon registry is unavailable.".to_owned())?;
        // Resolve through the live registry before classifying: is_native_mod deliberately
        // consults only the already-loaded index and must never guess from a stale UI object.
        let current = crate::browser::Source::project(&*source, &project.id)?;
        if crate::browser::horizon::is_native_mod(&current.id) {
            return Err("Native mod versions belong in the Native Mods workspace.".into());
        }
        crate::browser::Source::versions(&*source, &current)
    }

    /// The trust and capability summary shown before installing a registry content mod.
    pub fn registry_mod_trust(id: &str) -> Vec<String> {
        crate::browser::horizon::trust_and_capabilities(id)
    }

    /// Whether a file is a resource pack or a shader pack; a mod jar is neither.
    pub fn pack_kind(path: &Path) -> Result<PackKind, String> {
        crate::packs::kind_of(path)
    }
}

/// Registry .hpack modpacks. Kept separate from .hmod content mods and .hnmod native mods.
pub mod modpacks {
    use std::path::Path;

    pub use crate::browser::horizon::pack::Installed as RegistryModpack;

    /// Installed registry modpacks, each with its own profile and game folder.
    pub fn registry_modpacks() -> Vec<RegistryModpack> {
        crate::browser::horizon::pack::list()
    }

    /// Install or update a registry .hpack through the signed/hash-checked pipeline.
    pub fn install_registry_modpack(
        id: &str,
        progress: &dyn Fn(&str, f32),
    ) -> Result<RegistryModpack, String> {
        crate::browser::horizon::pack::install(id, progress)
    }

    /// Remove an installed .hpack profile through the existing service.
    pub fn remove_registry_modpack(work: &Path) -> Result<(), String> {
        crate::browser::horizon::pack::remove(work)
    }
}

/// Native mod discovery and build operations. Progress is emitted by the existing pipeline;
/// callers should run these functions on a background executor and stream steps to the UI.
pub mod native_mods {
    use std::path::{Path, PathBuf};

    pub use crate::native_mods::{Bundle, Plan, Profile, Step};

    /// Inspect .hnmod or unpacked native bundles with the same parser used by the builder.
    pub fn discover(folder: &Path) -> Result<Vec<Bundle>, String> {
        crate::native_mods::bundles(folder)
    }

    /// Build the selected instance with the trusted native-mod pipeline.
    pub fn build(
        base: &str,
        folder: &Path,
        instance: &crate::instances::Instance,
        allow_revision_mismatch: bool,
        profile: Profile,
        progress: impl Fn(Step),
    ) -> Result<String, String> {
        crate::native_mods::build(
            base,
            folder,
            instance,
            allow_revision_mismatch,
            profile,
            progress,
        )
    }

    /// Export a standalone headless server runtime without mutating a game instance.
    pub fn build_server(
        base: &str,
        folder: &Path,
        output: &Path,
        allow_revision_mismatch: bool,
        profile: Profile,
        progress: impl Fn(Step),
    ) -> Result<PathBuf, String> {
        crate::native_mods::build_server(
            base,
            folder,
            output,
            allow_revision_mismatch,
            profile,
            progress,
        )
    }
}

/// Hosted servers and their sharing.
pub mod servers {
    use super::*;
    use serde_json::Value;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Share {
        /// Shared friends reach every port (Tailscale's default).
        Everything,
        /// Shared friends reach only the server's port.
        ServerOnly,
        /// Access controls someone wrote; the launcher leaves them alone.
        Custom,
    }

    pub fn slug(name: &str) -> String {
        server_engine::slug(name)
    }

    /// What a tailnet policy grants shared friends for this computer and port.
    pub fn share(policy: &Value, ip: &str, port: u16) -> Share {
        match tailscale_engine::reach(policy, ip, port) {
            tailscale_engine::Reach::Everything => Share::Everything,
            tailscale_engine::Reach::ServerOnly => Share::ServerOnly,
            tailscale_engine::Reach::Custom => Share::Custom,
        }
    }

    /// The two rules to paste into access controls.
    pub fn share_snippet(ip: &str, port: u16) -> String {
        tailscale_engine::snippet(ip, port)
    }
}

/// Configuration and client IDs.
pub mod settings {
    use super::*;

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct ClientIds {
        pub microsoft: Option<String>,
        pub elyby: Option<String>,
    }

    /// `horizon-launcher.toml` as the launcher reads it; a bad file is all `None`.
    pub fn client_ids(text: &str) -> ClientIds {
        let parsed = launcher_config::parse_config_text(text);
        ClientIds {
            microsoft: parsed.horizon_msa_client_id,
            elyby: parsed.horizon_ely_client_id,
        }
    }

    /// Config first, then environment, then the built-in default.
    pub fn resolve_client_id(
        config: Option<&str>,
        env: Option<&str>,
        default: &str,
    ) -> Option<String> {
        launcher_config::resolve_client_id(config, env, default)
    }
}

/// Native-mod diagnostics.
pub mod diagnostics {
    use super::*;

    pub type Mod = crate::native_status::Mod;
    pub use crate::native_status::State as BuildState;

    /// The build state for the mods turned on and the version's built mods.
    pub fn native_state(enabled: &[Mod], built: Option<&[Mod]>) -> BuildState {
        native_engine::state(enabled, built)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{Accounts, MemoryStore, SecretStore};

    fn ely_tokens() -> ely::Tokens {
        ely::Tokens {
            access_token: "access-token-for-test".into(),
            refresh_token: Some("refresh-token-for-test".into()),
            expires_at: 1_900_000_000,
            profile: ely::Profile {
                uuid: "0123456789abcdef0123456789abcdef".into(),
                name: "ElyTester".into(),
            },
        }
    }

    fn ely_json() -> String {
        r#"{"refresh_token":"elyby-refresh:r","minecraft_token":"elyby:a","minecraft_expires_at":1900000000,"owns":false,"plays":true}"#.into()
    }

    fn microsoft_json() -> String {
        r#"{"refresh_token":"r","minecraft_token":"a","minecraft_expires_at":1900000000,"owns":true}"#.into()
    }

    fn account(uuid: &str, name: &str) -> Account {
        Account {
            uuid: uuid.into(),
            name: name.into(),
            skin: None,
            cape: None,
        }
    }

    #[test]
    fn a_finished_ely_sign_in_is_ely_and_can_play() {
        let sign_in = accounts::SignIn::finish_ely(ely_tokens());
        assert_eq!(sign_in.provider, accounts::Provider::ElyBy);
        assert!(sign_in.can_play);
        assert!(!sign_in.owns_minecraft);
        let shown = format!("{sign_in:?}");
        assert!(shown.contains("ElyTester"), "{shown}");
        assert!(!shown.contains("access-token-for-test"), "{shown}");
        assert!(!shown.contains("refresh-token-for-test"), "{shown}");
    }

    #[test]
    fn keeping_a_sign_in_leaves_tokens_out_of_the_account_file() {
        let store = MemoryStore::default();
        let mut accounts = Accounts::default();
        let sign_in = accounts::SignIn::finish_ely(ely_tokens());
        let uuid = sign_in.account.uuid.clone();
        sign_in.add_to(&mut accounts, &store).unwrap();

        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.selected.as_deref(), Some(uuid.as_str()));
        assert!(store
            .get(&uuid)
            .unwrap()
            .unwrap()
            .contains("access-token-for-test"));
        let file = serde_json::to_string(&accounts).unwrap();
        assert!(!file.contains("access-token-for-test"), "{file}");
        assert!(!file.contains("refresh-token-for-test"), "{file}");
        assert!(!format!("{accounts:?}").contains("access-token-for-test"));
    }

    #[test]
    fn a_card_names_the_provider_and_carries_only_safe_status() {
        let store = MemoryStore::default();
        let ely = account("0123456789abcdef0123456789abcdef", "ElyTester");
        store.set(&ely.uuid, &ely_json()).unwrap();
        let card = accounts::card_in(&store, &ely);
        assert_eq!(card.provider, accounts::Provider::ElyBy);
        assert_eq!(card.status.state, accounts::SessionState::Ready);
        assert!(!card.status.owns_minecraft);

        let ms = account("fedcba9876543210fedcba9876543210", "MicrosoftTester");
        store.set(&ms.uuid, &microsoft_json()).unwrap();
        let card = accounts::card_in(&store, &ms);
        assert_eq!(card.provider, accounts::Provider::Microsoft);
        assert_eq!(card.status.state, accounts::SessionState::Ready);
        assert!(card.status.owns_minecraft);

        let missing = account("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "NoSecrets");
        let card = accounts::card_in(&store, &missing);
        assert_eq!(card.provider, accounts::Provider::Unavailable);
        assert_eq!(card.status.state, accounts::SessionState::SignInRequired);
    }

    #[test]
    fn signing_out_removes_the_account_and_its_secret() {
        let store = MemoryStore::default();
        let mut accounts = Accounts::default();
        let sign_in = accounts::SignIn::finish_ely(ely_tokens());
        let uuid = sign_in.account.uuid.clone();
        sign_in.add_to(&mut accounts, &store).unwrap();
        assert!(!accounts::has_owner(&accounts, &store), "Ely.by never owns");

        accounts::sign_out(&mut accounts, &store, &uuid).unwrap();
        assert!(accounts.accounts.is_empty());
        assert!(store.get(&uuid).unwrap().is_none());
    }

    #[test]
    fn the_quick_switch_keeps_its_owner_rule() {
        let store = MemoryStore::default();
        let mut accounts = Accounts::default();
        let ms = account("fedcba9876543210fedcba9876543210", "MicrosoftTester");
        store.set(&ms.uuid, &microsoft_json()).unwrap();
        accounts.accounts.push(ms.clone());
        assert!(accounts::has_owner(&accounts, &store));
        accounts::play_as(
            &mut accounts,
            &store,
            accounts::Player::Microsoft(ms.uuid.clone()),
        )
        .unwrap();
        assert_eq!(accounts.selected.as_deref(), Some(ms.uuid.as_str()));

        let mut no_owner = Accounts::default();
        let offline = accounts::play_as(
            &mut no_owner,
            &store,
            accounts::Player::Offline("Alex".into()),
        );
        assert!(offline.is_err(), "offline needs an owner");
    }

    #[test]
    fn tags_keep_their_kinds_and_order() {
        assert_eq!(versions::kind("v2026.10.13"), versions::TagKind::Game);
        assert_eq!(
            versions::kind("launcher@0.2.54"),
            versions::TagKind::Launcher
        );
        assert_eq!(
            versions::kind("nightly-2026-10-28-0c02067"),
            versions::TagKind::Nightly
        );
        assert_eq!(versions::kind("release"), versions::TagKind::Other);
        assert_eq!(
            versions::nightly_label("nightly-2026-10-28-0c02067").as_deref(),
            Some("Nightly · 0c02067")
        );
        assert_eq!(
            versions::nightly_date("nightly-2026-10-28-0c02067"),
            Some("2026-10-28")
        );
        assert_eq!(
            versions::order("launcher@0.2.1", "launcher@0.2.0"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn launch_memory_and_env_keep_their_round_trip() {
        assert_eq!(launch::memory("4G").unwrap(), Some(4096));
        assert_eq!(launch::memory("off").unwrap(), None);
        assert!(launch::memory("plenty").is_err());
        assert_eq!(launch::memory_at(0.5, 8192), Some(4096));

        let env = launch::env("MANGOHUD=1 NAME=\"two words\"").unwrap();
        assert_eq!(env["NAME"], "two words");
        assert_eq!(launch::env(&launch::env_text(&env)).unwrap(), env);
    }

    #[test]
    fn native_build_profiles_are_explicit_and_stable() {
        use crate::native_mods::Profile;
        assert_eq!(Profile::parse("performance"), Some(Profile::Performance));
        assert_eq!(Profile::parse("fast"), Some(Profile::Performance));
        assert_eq!(Profile::parse("dev"), Some(Profile::Dev));
        assert_eq!(Profile::parse("release"), Some(Profile::Release));
        assert_eq!(Profile::parse("unknown"), None);
        assert_eq!(
            Profile::ALL,
            [Profile::Performance, Profile::Dev, Profile::Release]
        );
    }

    #[test]
    fn a_pack_file_keeps_its_kind() {
        let root = std::env::temp_dir().join(format!("horizon-facade-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let shader = root.join("water.hshader");
        std::fs::write(&shader, b"hshader").unwrap();
        assert_eq!(
            content::pack_kind(&shader).unwrap(),
            content::PackKind::Shader
        );

        let jar = root.join("mod.jar");
        std::fs::write(&jar, b"PK").unwrap();
        assert!(content::pack_kind(&jar).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_names_and_sharing_keep_their_contract() {
        assert_eq!(servers::slug("My Server!"), "my-server");
        let policy = serde_json::json!({
            "acls": [{ "action": "accept", "src": ["*"], "dst": ["*:*"] }]
        });
        assert_eq!(
            servers::share(&policy, "100.64.0.1", 25565),
            servers::Share::Everything
        );
        let limited = crate::tailscale::limit_shared(&policy, "100.64.0.1", 25565).unwrap();
        assert_eq!(
            servers::share(&limited, "100.64.0.1", 25565),
            servers::Share::ServerOnly
        );
        assert!(servers::share_snippet("100.64.0.1", 25565).contains("\"100.64.0.1:25565\""));

        let custom = serde_json::json!({
            "acls": [{ "action": "accept", "src": ["group:friends"], "dst": ["100.64.0.1:25565"] }]
        });
        assert_eq!(
            servers::share(&custom, "100.64.0.1", 25565),
            servers::Share::Custom
        );
    }

    #[test]
    fn client_ids_keep_their_precedence() {
        let ids = settings::client_ids(
            "HORIZON_MSA_CLIENT_ID = \"msa\"\nHORIZON_ELY_CLIENT_ID = \"ely\"\n",
        );
        assert_eq!(ids.microsoft.as_deref(), Some("msa"));
        assert_eq!(ids.elyby.as_deref(), Some("ely"));
        assert_eq!(
            settings::resolve_client_id(Some("config"), Some("env"), "default").as_deref(),
            Some("config")
        );
        assert_eq!(
            settings::resolve_client_id(Some("  "), Some("env"), "default").as_deref(),
            Some("env")
        );
        assert_eq!(
            settings::resolve_client_id(None, None, "default").as_deref(),
            Some("default")
        );
    }

    #[test]
    fn native_mod_state_is_explicit() {
        let mods: Vec<diagnostics::Mod> = vec![("water".into(), "1.0".into(), Some("hash".into()))];
        assert_eq!(
            diagnostics::native_state(&[], None),
            diagnostics::BuildState::NoMods
        );
        assert_eq!(
            diagnostics::native_state(&mods, None),
            diagnostics::BuildState::NotBuilt
        );
        assert_eq!(
            diagnostics::native_state(&mods, Some(&mods)),
            diagnostics::BuildState::UpToDate
        );
        let other: Vec<diagnostics::Mod> = vec![("water".into(), "2.0".into(), None)];
        assert_eq!(
            diagnostics::native_state(&mods, Some(&other)),
            diagnostics::BuildState::Outdated
        );
    }

    #[test]
    fn content_mods_and_registry_modpacks_have_separate_facades() {
        let _: fn(&content::Query) -> Result<Vec<content::Project>, String> =
            content::registry_mods;
        let _: fn() -> Vec<modpacks::RegistryModpack> = modpacks::registry_modpacks;
        assert_eq!(crate::browser::Kind::Mod.label(), "Mods");
        assert_eq!(crate::browser::Kind::Modpack.label(), "Modpacks");
    }

    #[test]
    fn instance_play_time_and_old_metadata_are_stable() {
        assert_eq!(instances::play_time(12 * 60), "12 min");
        assert_eq!(instances::play_time(3 * 3600 + 5 * 60), "3 h 05 min");
        assert_eq!(instances::play_time(140 * 3600), "140 h");
        assert_eq!(instances::meta("{}").unwrap(), instances::Meta::default());
    }
}
