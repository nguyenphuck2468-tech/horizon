# Workflow map — current launcher → services → persistence (N02)

Status: N02 complete. Describes `launcher/` at `launcher` `cadddef84d` (the base of
`horizon-next/rebrand-foundation`).
Date: 2026-11-09

This map records, for every preserved area, which current surface calls which application module,
where the data lives, how failures reach the player, and which test pins the behaviour. It is the
bridge between the N01 preservation contract (`launcher/src/next_preservation.rs`) and the N03
compatibility facade: the new shell may replace the surfaces, but it must keep these calls, files
and error states.

It changes no code. Where this map and the code disagree, the code wins and the map is corrected.

## 1. How to read it

- **Surface** — a GPUI module in `launcher/src`. `main.rs` owns the `Page` enum and the sheets;
  the `glass_*` modules are the current visual implementation.
- **Service** — the non-UI module the surface calls. These are the entry points N01 pins.
- **Persistence** — files and folders the service reads or writes, relative to the roots in §3.
- **Errors** — what the player sees today, not a new taxonomy.
- **Test / N01** — the suite that pins the behaviour and the matching `Workflow` entry.

The new shell must add no second path to these services. A screen that needs a workflow the map
does not list is a gap (§8), not a licence to call HTTP or the filesystem from the UI.

## 2. Current surfaces

Top-level pages (`Page` in `main.rs`):

| Page | Module | Contains |
|---|---|---|
| Play / Home | `home.rs`, `home_tabs.rs`, `glass_home*.rs` | Overview, Mods, Worlds, Multiplayer, Packs, Screenshots, Console tabs |
| Mods | `mods_page.rs`, `packs_page.rs`, `modpacks_page.rs`, `datapacks_page.rs`, `github_mods_page.rs` | catalogue browsing, installed mods/packs, pack import |
| Servers | `servers_page.rs`, `glass_servers.rs`, `glass_server_dialog.rs` | owned servers, create/start/stop, Tailscale sharing |
| Wardrobe | `wardrobe_page.rs` | cosmetics, only when the cosmetics flag is on |
| Versions | `glass_versions.rs` | installed builds, Nightly, update channel |
| Notes | `glass_notes.rs`, `markdown.rs` | patch notes |
| Settings | `glass_settings.rs`, `themes_page.rs`, `language_page.rs`, `skin_page.rs`, `agents_page.rs` | appearance, language, account/skin, Java, proxy, telemetry, agents |

Sheets and panels (not pages): owner sign-in gate (`owner_page.rs`), new instance
(`new_instance.rs`), edit instance (`glass_instance.rs`), quick account switch (`switch_page.rs`),
shortcut/desktop (`shortcut_page.rs`), command palette (`glass_palette.rs`), build log
(`native_log_page.rs`), toasts (`glass_toast.rs`).

Everything below runs in the same process as the game launcher; there is no separate backend
service. Long work (downloads, prepare, server start) is spawned on threads and reported through
channels, which is why the facade in N03 has to keep progress and cancellation, not just results.

## 3. Data roots

| Root | Function | Default | Override |
|---|---|---|---|
| Launcher data | `backend::data_dir()` | `dirs::data_dir()/horizon-launcher` | `HORIZON_LAUNCHER_HOME` |
| Game home | `backend::game_home()` | `dirs::data_dir()/horizon` | `HORIZON_HOME` |
| Build folder | `backend::build_dir()` | the data folder, canonicalised; on Windows an ASCII-safe folder when the user path is not | — |
| Version install | `backend::version_dir(tag)` | `data_dir()/versions/<tag>` | — |
| Operator config | `config` | `horizon-launcher.toml` beside the executable | — |

The two roots are independent on purpose: deleting the launcher's data must not delete worlds,
screenshots or the game, and vice versa. The new shell must keep both names and both overrides —
`HORIZON_LAUNCHER_HOME` and `HORIZON_HOME` are used by tests, scripts and support.

## 4. Persistence ownership

| Data | Under | Written by | Read by | Compatibility rule |
|---|---|---|---|---|
| `accounts.json` | data root | `Accounts::save` | `Accounts::load`, account pickers | Never a token; a missing/corrupt file is an empty account list |
| `account-secrets.json` | data root | `account::FileStore` | `secrets_in` | Only when the OS keychain is unavailable; file is owner-only |
| OS keychain | OS | `account::SystemStore` | `secrets_in` | Preferred store; tokens and the provider marker live here |
| `settings.json` | data root | `Settings::save` (temp file + rename) | `Settings::load` | Missing/corrupt → defaults; an unknown appearance/language degrades instead of wiping the file |
| `instances.json` | data root | `instances::Store::save` | `instances::Store::load`, `instances::list` | Per-instance fields are optional; older files load and defaults are not written back |
| `instances/<slug>/` | game home | `instances::{create, import}`, the game | `instances::list`, launch | An instance folder is a game folder; a folder deleted outside the launcher is forgotten |
| `servers.json` | data root | `servers::save_list` (temp file + sync + rename) | `servers::list` | One entry per owned server, name + folder; folders that are gone are left out of the list |
| `servers/<slug>/server.toml` | data root | `servers` setup / the server | `servers::config`, `can_host` | Missing file means "not a server folder"; game mode and world settings are read from it |
| `servers/<slug>/logs/supervisor.log` | data root | server supervisor | `servers` log view | Append-only; length used to follow the log |
| `server-runtime/` | data root | `servers::set_up` | `servers::{program, run}` | Holds `releases/<tag>`, `current` and shared `game-files/`; refreshable |
| `versions/<tag>/` | data root | `backend::install`, `backend::prepare_step` | launch, `servers::can_host` | `.installed` marks a complete install; `resources` manifest marks prepared |
| `minecraft/runtime/` | data root | `java::install` | `java::{installed, pick}` | Mojang runtimes, separate from the player's own Java |
| `skins/<name>.png`, `<name>.cape.png` | data root | `skins::keep` | `skins::env` | Lower-case offline name; only kept files are exported to the game |
| `wardrobe/<uuid>.json` | data root | `wardrobe::Saved::save` (temp + rename) | `wardrobe::Saved::load` | Missing file shows the default outfit offline |
| `horizon-launcher.toml` | executable folder | the operator | `config::load` | Invalid/partial TOML degrades field by field |
| `saves/<world>/` | game home | the game | `worlds::list` | Launcher only reads; world deletion/backup act on the folder |
| `backups/` | game home | `worlds::backup` | player | One archive per backup, never inside `saves/` |
| `screenshots/` | game home | the game | `screenshots::{list, find, delete}` | Only image files are listed |
| `logs/`, `crash-reports/` | game home | the game | `game_log`, `console`, crash sheet | Launcher reads and copies; never rewrites the game's files |
| `resourcepacks/`, `shaderpacks/`, `mods/` | game home or instance | player, catalogue installs | `packs::list`, `instance_mods` | Pack kind comes from the file, not the folder |
| `<world>/datapacks/`, `<world>/datapacks.json` | game home | manager | `datapacks` | Missing settings file means defaults |

Two writes are deliberately not atomic today: `accounts.json` and `instances.json` write in place.
The facade must not make that worse, and N03 should decide whether to adopt the temp+rename used
by `settings.json`, `servers.json` and the wardrobe.

## 5. Area-by-area mapping

Each row is one N01 workflow. `Manual` means N01 records it as manual-only.

### 5.1 Accounts and identity — `FeatureArea::AccountsAndIdentity`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Owner sign-in gate (`owner_page.rs`), Settings > Account (`glass_account.rs`) | `account::{device_code, poll, wait_for_sign_in, finish_sign_in}`, `microsoft` | `accounts.json`, keychain | `NOT_CONFIGURED` when the build has no Microsoft client ID; `NEEDS_OWNER`; XSTS codes 2148916233/35/36/37/38 each have a player-facing sentence | `account::tests` fixtures; N01 *Microsoft device-code sign-in* (Manual) |
| Settings > Account (Ely.by) | `ely::{device_code, poll}`, `account::{finish_ely_for_cli, is_ely_account}`, `elyby` | `accounts.json`, keychain (prefix `elyby:` / `elyby-refresh:`) | Same gate; provider is shown from the stored marker, never inferred from the name | N01 *Ely.by sign-in* (Smoke) |
| Account switch (`switch_page.rs`), CLI `account` | `Accounts::{load, save, play_as, remove}`, `account::session` | `accounts.json`, keychain | Refresh failure marks `sign_in_required`; the account stays listed and can be re-authenticated | `account::tests` |

Provider rules: Microsoft confirms ownership (`owns`), Ely.by never does (`owns: false`,
`plays: Some(true)`), offline names have neither. The new shell may present these differently but
must not change the rules or put a secret in an `Account`.

### 5.2 Versions and builds — `FeatureArea::VersionsAndBuilds`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Versions page (`glass_versions.rs`), footer update | `backend::{installed, own_tag, release_order, launcher_version, is_game_tag, compare_versions}`, `github`, `signed` | `versions/<tag>/` | A launcher/game tag that cannot be ordered is ignored, not installed over a newer build | `backend::release_loading_tests`; N01 *Launcher and game tags keep their order* (Smoke) |
| Nightly badge / channel | `nightly::{version_of, date_of, label, after_update}` | `versions/<tag>/` | A tag that is not a Nightly yields `None` and is shown as an ordinary version; it never panics | `nightly` tests; N01 *Nightly naming* (Smoke) |
| Launcher self-update | `backend::install`, `signed::{digest, launcher_digest}`, `mac_update` | running executable, `versions/` | A missing/invalid signature refuses the install; a failed replace says to close and reopen | `signed::tests`; N01 *Self-update* (Manual) |

### 5.3 Install, prepare and launch — `FeatureArea::InstallPrepareLaunch`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Play / Home, setup sheet (`setup_page.rs`) | `backend::{install, prepare_step, need_prepared, is_prepared, has_sounds, resources_summary}`, `installer` | `versions/<tag>/`, game home | `need_prepared` names the version and the exact `prepare` command; missing sounds are reported as a state, not a crash | `backend` tests; N01 *Prepared resources are reported* (Smoke), *Real download/prepare/launch* (Manual) |
| Play button (`launch_page.rs`) | `launch::{plan, parse_memory, memory_at, memory_fraction, env_line, parse_env}`, `process`, `while_playing` | settings (`memory`, `env`), version dir | A memory value that cannot parse is rejected before the game starts; missing Java is a setup error | `launch/tests`; N01 *Memory/env round-trip* (Smoke) |

Transient network failures on downloads follow the shared policy in §6.2.

### 5.4 Instances — `FeatureArea::Instances`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Home overview, instance strip, Edit dialog (`glass_instance.rs`) | `instances::{list, Store, Meta, play_time, selected}` | `instances.json`, `instances/<slug>/` | A missing/corrupt `instances.json` is an empty store; `Meta` missing fields default; play time is display-only | `instances/tests`; N01 *Older metadata still loads* (Smoke) |
| New instance, import/export, delete | `instances::{create, select, edit, delete_with_selection}`, `import`, `export`, `build_import` | instance folder, `instances.json` | Duplicate name is refused with its name; a missing instance lists the ids; export refuses a destination it cannot write | `instances/tests`, import/export tests; N01 *Create/clone/import/export* (Manual) |

### 5.5 Mods, packs and content — `FeatureArea::ModsAndPacks`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Home > Mods, Mods page (`mods_page.rs`, `mods_project.rs`) | `instance_mods`, `browser`, `browser::horizon`, `github_mods`, `loaders` | `mods/`, `instances.json` (loader choice) | Provider failure is shown per source; an incompatible mod version is filtered, not silently installed | `browser/tests`, `github_mods/tests`; N01 *Catalogue downloads* (Manual) |
| Home > Packs, Packs page (`packs_page.rs`) | `packs::{list, kind_of, Kind}`, `modpacks`, `datapacks` | `resourcepacks/`, `shaderpacks/`, `horizon-packs/`, world `datapacks/` + `datapacks.json` | `kind_of` rejects a non-pack (for example a mod jar) and names `.zip` / `.hshader`; unreadable pack metadata falls back to the file name | `packs` tests; N01 *Pack kinds* (Smoke) |
| Datapacks page (`datapacks_page.rs`) | `datapacks::{list, add, find, remove, coverage}` | world folder | A settings file that cannot be read means defaults; a name collision gets `(2)` | `datapacks/tests` |

### 5.6 Worlds and screenshots — `FeatureArea::WorldsAndScreenshots`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Home > Worlds (`worlds_page.rs`) | `worlds::{list, find, copy, backup, delete}` | `saves/`, `backups/` | A missing world lists the query and the `saves` folder; a backup failure leaves the world untouched | `worlds/tests`; N01 *Backup + screenshot listing* (Smoke) |
| Home > Screenshots (`screenshots_page.rs`, `glass_shot_blur.rs`) | `screenshots::{folder, list, find, delete}` | `screenshots/` | Non-image files are never listed; deleting a missing file reports it | `screenshots` tests |

### 5.7 Multiplayer — `FeatureArea::Multiplayer`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Home > Multiplayer (`multiplayer_page.rs`) | `server_list::{List, load, add_official, find}`, `telemetry::server_players` | game home `servers.dat`, official marker | An unreadable list is left alone, not overwritten; the official server is offered once and not re-added after removal | `server_list::tests`; N01 *Official server offer* (Smoke), *Join + status* (Manual) |

### 5.8 Servers and network — `FeatureArea::ServersAndNetwork`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Servers page (`servers_page.rs`, `glass_servers.rs`, `glass_server_dialog.rs`) | `servers::{list, add, forget, delete, create, set_up, can_host, run, run_streamed, command, config, set_config, status, backups}`, `tailscale::{State, reach, limit_shared, snippet}`, `proxy`, `agents` | `servers.json`, `servers/<slug>/`, `server-runtime/` | `can_host` distinguishes a missing server program from an unprepared version and names the fix; "This is not a server folder" protects deletion; an unrecognised ACL is `Reach::Custom` | `servers/tests`, `tailscale::tests`; N01 *Server names + Tailscale rules* (Smoke), *Start server / live tailnet* (Manual) |

### 5.9 Java and launch settings — `FeatureArea::JavaAndLaunchSettings`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Settings > Java (`glass_instance_java.rs`), launch sheet | `java::{release_version, runtimes_dir, installed, pick, install}`, `java_launch`, `rust_toolchain` | `minecraft/runtime/`, `Meta.java` per instance | A home without `bin/java` is not offered; a required version that is missing reports what the game needs | `backend/java_tests`, `java_launch/tests`; N01 *Pick/install Java* (Manual) |
| Settings > Launch / instance launch options | `launch::{parse_memory, memory_at, env_line}`, `instances::Meta` (`launch`) | `settings.json`, `instances.json` | Invalid memory/env text is rejected with the offending word | N01 *Memory/env round-trip* (Smoke) |

### 5.10 Console, logs and diagnostics — `FeatureArea::ConsoleAndDiagnostics`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Home > Console (`console_page.rs`), build log (`native_log_page.rs`) | `game_log`, `console`, `native_status::{state, Mod, State}`, `dev_options`, `perf` | game home `logs/`, `crash-reports/` | A missing log is an empty view, not an error; native-mod state is explicit (`NoMods`, `NotBuilt`, `Outdated`, `UpToDate`) | `game_log/tests`, `native_status` tests; N01 *Native mod state* (Smoke), *Capture logs/crash* (Manual) |

### 5.11 Skins and cosmetics — `FeatureArea::SkinsAndCosmetics`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Settings > Account > Skin (`skin_page.rs`), offline names | `skins::{keep, env, forget, dir, Look, Model}`, `Accounts::{look, look_env, remove_offline}`, `heads` | `skins/<name>.png`, `skins/<name>.cape.png` | A missing or invalid picture is refused; the game gets no env vars unless the look is kept | `skins/tests`; N01 *Offline skin only while kept* (Smoke) |
| Wardrobe (`wardrobe_page.rs`) | `wardrobe::{Service, Saved, Catalogue, ClaimState, Error}` | `wardrobe/<uuid>.json` | Offline shows the saved outfit; a retryable claim says so; `retry_later` distinguishes network from terminal refusal | `wardrobe/tests`; N01 *Claim cosmetics* (Manual) |

### 5.12 Configuration and platform — `FeatureArea::ConfigurationAndPlatform`

| Surface | Service | Persistence | Errors | Test / N01 |
|---|---|---|---|---|
| Settings (`glass_settings.rs`), themes (`themes_page.rs`), language (`language_page.rs`) | `config::{parse_config_text, resolve_client_id}`, `themes`, `i18n::{set, tr, trf}` | `settings.json`, `horizon-launcher.toml` | Invalid TOML loses only the bad field; an unknown appearance/language follows the default instead of resetting every setting | `config` tests, `i18n/tests`; N01 *Config precedence* (Smoke) |
| Shortcuts, agents, installers, desktop | `shortcut_page`, `agents_page`, `installer`, `desktop`, `desktop_seams`, `cli`, `cli_tools`, `mcp`, `telemetry` | OS integration, outbox | Missing permissions or a refused keychain are reported; telemetry is opt-in and every switch names what it sends | `account::tests` in-memory store; N01 *Keychain/installers/desktop* (Manual) |

## 6. Cross-cutting contracts

### 6.1 Secrets never reach the UI

- `accounts.json` holds an `Account` (uuid, name, skin, cape) and the selected uuid. It never
  holds a token; `account::tests` and the N01 Ely.by smoke assert this.
- Tokens and the provider marker live in the OS keychain, or `account-secrets.json` where no
  keychain exists. `Secrets` has a hidden `Debug`; nothing formats it into a message.
- Ely.by is identified by the `elyby:` prefixes in `Secrets`, not by an account name. A new screen
  must ask `account::is_ely_account`/the stored marker, never guess.
- The N01 smoke also asserts that `serde_json::to_string(&Account)` contains no token and that
  an `Account` round-trips.

### 6.2 Network failures

- `backend::get` retries HTTP 502, 503 and 504 up to 3 attempts, waiting 250 ms then 500 ms
  (`GET_ATTEMPTS`, `retry_delay`). This covers a proxy or GitHub briefly unavailable during a
  deploy. Other statuses and transport errors surface immediately.
- A failed download is retried from the caller (installer or updater), not hidden; a version whose
  files could not be removed stays listed so the removal can be retried.
- Authentication errors are never retried blindly: Microsoft/XSTS answers become the sentences in
  `account::xsts_error`; sign-in can be restarted.
- Provider catalogues (Modrinth, CurseForge, Horizon, GitHub) report per-source failures; one dead
  source must not empty the others.

### 6.3 Old data stays readable

- Missing files mean defaults, not errors: accounts, settings, instances, servers, wardrobe,
  datapacks, world/screenshot listings.
- `instances::Meta` fields are optional and defaults are not written back, so a file from an older
  launcher stays valid.
- `Settings::parse` repairs an appearance or language written by a newer build instead of wiping
  the file; `owns`/`plays` keeps pre-Ely.by accounts and Microsoft accounts working.
- `backend::launcher_version` accepts the current `launcher@` tags and the old `launcher-v` tags.
- The facade must keep reading these files in place. A migration may add fields; it may not
  require the player to re-add accounts, instances or settings.

### 6.4 Prepared and installed states

- `is_prepared(tag)` is the presence of the resources manifest; `has_sounds` is separate.
  `need_prepared` fails with the version and the command that fixes it.
- `can_host` requires the version's server program and a prepared version; it names which one is
  missing.
- The new shell must show these as states, not as generic errors, and must not start a game or
  server from a half-prepared version.

### 6.5 Release signatures

- `signed::digest`, `signed::launcher_digest` and `signed::nightly_digest` verify a signature and
  checksums before an install. A missing or invalid signature refuses the install; it is never
  treated as "unsigned but allowed".
- The same policy applies to the launcher's own update, not only the game build.

### 6.6 Writes and concurrency

- `settings.json`, `servers.json` and `wardrobe/<uuid>.json` write a temporary file and rename it.
- `accounts.json` and `instances.json` write in place; both load a corrupt file as empty.
- `settings.json` saves are serialised by a process-wide lock because the window and background
  jobs share the file.
- Server start/stop holds a folder lock in `servers/<slug>/.horizon` so a stale status cannot
  delete or start a live server.

## 7. What the new shell must keep

From N01, the contract is 24 workflows across 12 areas. This map adds the "where":

1. Every write above keeps its file name, folder and format.
2. Every service entry point in §5 stays callable without GPUI.
3. Every error in §5 either keeps its text or gets a replacement that names the same fix.
4. The two data roots and their environment overrides stay.
5. Secrets stay out of the UI and out of `accounts.json`.
6. 502/503/504 retry, signature refusal and prepared-state checks stay in the service layer, not
   in a screen.

## 8. N03 facade and the gaps it leaves

`launcher/src/next_facade.rs` adds the first shell-facing boundary. It names use cases for
accounts (safe card, finished sign-in, refresh, sign-out, owner check, play-as), release and
Nightly tags, launch memory/env, instance facts, pack kinds, server names and tailnet sharing,
client IDs and native-mod state. Every function delegates to the service in §5; 12 contract tests
pin the delegation.

The accounts boundary is deliberate: `AccountCard` carries identity, provider and the safe
`account::session::Status`, while `SignIn` keeps `Secrets` private and persists through
`add_to`. A screen can finish, refresh and sign out without seeing a token.

N03 does not close these, and they stay on the list for N04/N09:

- **Server start/stop and the palette** still orchestrate in `glass_servers.rs` and `main.rs`;
  the facade does not wrap them yet.
- **Progress and cancellation are channel-based.** Downloads, prepare and server start report
  progress through threads. The facade exposes results and options, not the progress stream; a
  later slice must wrap it without losing the current UX.
- **Errors keep their current shape.** Services return `Result<_, String>` (or `NotMemory`);
  the facade does not invent new player-facing text. Typed classification can come later.
- **`accounts.json` and `instances.json` are still not atomic.** This is a behaviour change, so
  N03 leaves it for a dedicated fix with a short-write regression test.
- **`settings.json` still drops unknown keys.** Preserving them is a separate compatibility
  decision.
- **Manual-only N01 paths still need parity hooks.** Microsoft sign-in, self-update, real launch,
  catalogue downloads, live multiplayer, Tailscale, Java install, log capture, cosmetics and OS
  integration cannot be unit-tested; the first shell behind a flag needs a manual parity script
  for each before it becomes the default.
- **Screenshot seams are environment variables** (`HORIZON_LAUNCHER_PAGE`). The new shell needs
  an equivalent so visual regression can open every workspace deterministically.

## 9. Verification

N03 changes Rust code and docs. On the N03 tip the fence is `cargo fmt --check` and clippy clean,
library tests **581**, bin tests **117**. The 12 facade tests are part of the library count; the
N01 smoke tests still call the same underlying entry points. The N03 change-log entry records the
same.
