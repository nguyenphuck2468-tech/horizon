# Horizon Launcher — complete frontend replacement

## Product decision

The existing launcher frontend is being replaced, not merely reskinned. The replacement ships from this same Horizon repository and continues to use the existing launcher backend. The old frontend is a migration reference only; it must not remain the default UI or a runtime dependency of the finished product.

This does not mean removing launcher capabilities or rewriting their trusted implementations. Preserve the services and workflows listed below and give them a new UI.

## What stays

Keep the existing Rust backend/services for:
- Minecraft/Horizon version discovery, game build/download, installation, preparation and launch.
- Instance creation, selection, editing, import/export, game directories and Java discovery.
- Microsoft, Ely.by and offline accounts, including secure secret storage and ownership rules.
- .homd package discovery/download, validation, installation, update and removal (explicit product requirement).
- .hmod registry content packs already supported by Horizon, preserving their signed/hash-checked install path.
- .hnmod discovery/download, validation, dependency resolution, installation and removal.
- Native mod build pipeline, toolchain/dependency checks, build logs, artifact validation and packaging.
- Modrinth/other content sources, modpacks, resource packs, shaders and datapacks.
- Servers/multiplayer, worlds, screenshots, skins/cosmetics, settings, updates, diagnostics and CLI/MCP.
- Existing persistence formats, paths, signed-update verification and security boundaries.

The UI calls typed application services/facades. It must not read account secrets, implement archive extraction itself, bypass package validation, or reimplement build/launch logic.

## What is replaced

- The current desktop screen tree, page composition, navigation, visual components and interaction model are not the target architecture.
- Do not preserve the old UI as a second selectable product after cutover.
- Do not create another repository or move the launcher out of this repository.
- Do not replace native GPUI with a WebView or a heavyweight UI runtime.

## New experience

Build a new native GPUI application shell with:
- Home: selected instance, account, version/loader, Java/readiness, activity and one primary Play action.
- Library: instances, versions and profiles.
- Content: .homd packages, existing .hmod registry packs, .hnmod native mods, modpacks, resource packs and shaders.
- Mod development: build configuration, dependency checks, build progress, logs and artifacts.
- Accounts: provider-aware account management and sign-in.
- Servers, worlds, cosmetics and diagnostics.
- Settings: appearance, language, paths, Java, performance and update preferences.

Use a coherent design system: compact typography, consistent spacing, accessible contrast/focus, clear loading/empty/error states, keyboard navigation, responsive layouts and light/dark themes. Avoid decorative motion that consumes CPU/GPU; respect reduced-motion preferences.

## Migration and cutover

1. Map each retained workflow to its current backend entry point and add parity coverage.
2. Create the replacement UI module and app shell inside launcher/; keep it separate from the old screen modules during development.
3. Implement Home, instance library, accounts and launch end-to-end.
4. Implement .homd package management, existing .hmod registry-pack management, .hnmod management and native mod building end-to-end.
5. Migrate remaining retained workflows, with parity tests for each.
6. Switch the app entry point to the new frontend and remove the old UI modules only after feature parity and Windows/Linux/macOS smoke tests pass.

The old frontend may be used as a temporary reference during migration, but the final executable launches the new frontend by default. Do not ship a dual-UI toggle as the final outcome.

## Compatibility and performance

- Preserve user data, account storage, instance layout and existing config migration behavior.
- Preserve CLI/MCP independently from desktop UI.
- Never weaken authentication, ownership, archive validation, package signatures or build checks.
- Keep background work off the UI thread and expose cancellable progress where safe.
- Lazy-load expensive content; bound image/log caches; avoid continuous scene animation by default.
- Record before/after startup time, idle CPU/RAM, navigation latency and scrolling responsiveness.

## Acceptance criteria

- The old UI is no longer the app default and is removed after migration.
- All retained workflows above are reachable in the new UI.
- .homd packages have a documented, validated import/install pipeline; existing .hmod registry packs and .hnmod native mods continue using their current verified pipelines.
- Native mod building exposes its real build stages, logs and verified artifacts.
- No account, instance, world, mod or settings data is lost or relocated.
- All existing security and backend tests pass; UI parity tests cover migrated workflows.
- Windows, Linux and macOS builds and smoke tests pass before old UI removal.