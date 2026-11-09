# Feature Preservation Matrix (draft; verification column fills as phases run)

Scope: `launcher/` + shared crates. PJPL = the five-step charter in docs/design/horizon-next/roadmap.md.

| # | Feature group | Docs | Modules (launcher/src) | Tests | Verified |
|---|---------------|------|------------------------|-------|----------|
| 1 | Versions & builds | versioning, release-flow, version-ports, windows-allocators | nightly.rs, sources.rs, sources_tests.rs, build_import.rs, github.rs | sources_tests (18), github/tests (8), backend/release_loading_tests | CI checks (pending) |
| 2 | Prepare/install/launch | launcher-preparation, launcher-commands, prepare-performance | installer.rs, launch/, backend.rs, process.rs, rust_toolchain.rs | backend (50), backend/java_tests, launch/tests (12) | codespace run: backend+launch green |
| 3 | Accounts | microsoft-account, elyby-account, owners | account/, microsoft.rs, elyby.rs, ely.rs | account/tests (110), ely.rs (18) | codespace run: green |
| 4 | Instances | modpacks (cfg) | instances/, instances_page.rs, new_instance.rs, collection.rs, instance_icons.rs, instance_mods.rs, glass_instance.rs | instances/tests (13) | codespace run: green |
| 5 | Mods/packs/content | mods, mod-browser, modpacks, resource-packs, shaders, native-mods, native-hooks-neoforge, native-client | mods_page/, native_mods/, browser/, github_mods/, datapacks/, packs/, loaders/, import/, export/ | native_mods (73 across files), browser/tests (13), browser/horizon/tests (11), github_mods/tests (14) | codespace run: green |
| 6 | Worlds/screenshots | world-data | worlds/, worlds_page.rs, screenshots.rs, screenshots_page.rs | worlds/tests (7, in run) | codespace run: green |
| 7 | Multiplayer | play-with-friends, smp | multiplayer_page.rs, server_list/, heads.rs | server_list tests (in run) | codespace run: green |
| 8 | Servers & Tailscale | server, server-next, server-levels, hosting, velocity-forwarding, agent-control | servers/, glass_servers.rs, glass_server_dialog.rs, tailscale/, proxy.rs, agents.rs | servers/tests (10), tailscale tests | codespace run: green |
| 9 | Java & launch settings | launcher-commands, ci-runners | java/, java_launch/, glass_instance_java.rs | backend/java_tests, java_launch/tests (8) | codespace run: green |
| 10 | Console/logs/diagnostics | known-issues, narrator | console.rs, console_page.rs, game_log/, native_log_page.rs, native_status.rs, dev_options.rs, perf.rs | game_log/tests, native_status (7) | codespace run: green |
| 11 | Skin/cape/profile | cosmetics | skins/, skin_page.rs, wardrobe/, wardrobe_page.rs, heads.rs, profile_heads.rs | wardrobe/tests (8) | codespace run: green |
| 12 | Launcher config & system | telemetry (opt-in), security | config.rs, themes/, theme.rs, i18n/, telemetry.rs, desktop*, shortcut*, glass_settings.rs, cli.rs, cli_tools.rs, mcp.rs | config (15), i18n/tests, glass_settings | codespace run: green |

Untested-by-automation (candidates for added tests): parts of glass_* UI states (partially tested in
glass_home/glass_instance/glass_server_dialog/glass_settings/glass_shot_blur), desktop_seams,
titled window resize paths. `checks` CI is green on the current tips after the 2026-11-08
download fixes, on every phase branch (pull requests #5, #3 and #4) — the authoritative
regression fence for phases 1+. Nightly download path: `download-fixes.md`.

## N01 - executable baseline (2026-11-09)

`launcher/src/next_preservation.rs` turns this table into a contract:

- `WORKFLOWS` lists 24 workflows across the 12 areas, each with its production entry point.
- `cargo test --locked --lib next_preservation` runs all 13 `Automation::Smoke` entries against
  those entry points, so a renamed or broken workflow fails the fence before a new shell uses it.
- `every_preserved_area_has_a_workflow` fails if a preserved area loses its coverage, and
  `manual_workflows_say_what_a_test_needs` keeps the manual reasons honest.

Manual-only paths (no unit test can provide them) and the suite covering them meanwhile:

- Microsoft device-code sign-in: live Microsoft/Xbox/Minecraft services; `account::tests` fixtures.
- Launcher self-update install: a published signed release; `signed::tests`.
- Real game download, prepare and launch: a Minecraft install and a game window; `backend`/`launch` tests.
- Instance create/clone/import/export: the player's data directory; `instances::tests`, import/export tests.
- Provider catalogue downloads: live Modrinth/CurseForge/Horizon APIs; `browser/tests`, `github_mods/tests`.
- Joining a live multiplayer server: a running server and game client; `server_list::tests`.
- Starting a hosted server and changing the live tailnet: Tailscale CLI and admin rights; `tailscale::tests`.
- Picking and installing a Java runtime: Mojang downloads and Java homes; `java::tests`.
- Capturing logs and crash reports: a running game process and GPU; `game_log::tests`.
- Claiming wardrobe cosmetics: signed-in account and live API; `wardrobe::tests`.
- Keychain storage, installers and desktop integration: the OS; `account::tests` in-memory store.

N02 records where each of these workflows lives today: `docs/design/horizon-next/workflow-map.md` maps surface to service, persistence, error state and test, and lists the gaps the compatibility facade must close.

N03 adds `launcher/src/next_facade.rs` as the shell-facing name for these workflows: accounts (card/sign-in/refresh/sign-out/owner check/play-as), tags, launch options, instance facts, pack kinds, server sharing, client IDs and native-mod state. It delegates to the services above and keeps `Secrets` out of the shell; 12 contract tests pin it.
