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
