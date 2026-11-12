# Horizon Launcher UI rewrite

## Scope

Rewrite the desktop launcher's user interface **in this repository**, under `launcher/`.
Do not create a second repository, do not extract a separately published launcher core,
and do not move the launcher out of the Horizon workspace.

The existing launcher remains the product entry point and the source of truth for
accounts, instances, Java discovery, downloads, modpacks, servers, updates and launch.

## Migration strategy

Build the replacement UI alongside the existing GPUI frontend in the same
`launcher/` package, then switch the application entry point only after the replacement
covers the existing workflows. Keep the current frontend available during migration so
feature parity and rollback remain possible.

The rewrite is a frontend replacement, not a backend rewrite. Reuse the established
launcher services and persistence APIs; introduce typed view models and UI state adapters
where the current UI/backend boundary is too coupled.

## Product direction

- A clear, compact desktop shell with persistent navigation and a strong primary Play action.
- Home centered on the selected instance, version/loader, account, Java readiness and launch state.
- Dedicated, consistent pages for Instances, Versions, Mods, Modpacks, Servers, Skins and Settings.
- A unified component system for typography, spacing, surfaces, controls, empty states, loading,
  errors and confirmation dialogs.
- Light and dark themes, localization across all shipped locales, keyboard navigation and
  accessible focus states.
- No sponsor/donation/ad surfaces in the personal fork.

## Performance constraints

- Keep the native GPUI rendering path; do not add a WebView or a heavyweight UI runtime.
- Avoid continuously animated backgrounds, repeated full-page rebuilds and eager loading of
  off-screen content.
- Prefer stable view state, incremental updates, bounded image caches and lazy page data.
- Respect reduced-motion preferences and lower-end machines.
- Measure startup, idle CPU/RAM, navigation responsiveness and scrolling before/after.

## Non-negotiable compatibility

- Preserve existing account storage and Microsoft/Ely.by/offline login behavior.
- Preserve instance/game directory layouts, imports, Java selection, mod installation,
  launch arguments, update/signature checks and user settings.
- Do not migrate or delete user data as part of the UI rewrite.
- Keep CLI/MCP behavior independent from desktop view code.
- Do not change authentication ownership semantics or security boundaries for presentation reasons.

## Phases

1. Inventory current screens, actions, state ownership and backend calls.
2. Introduce the replacement shell, navigation and design tokens in `launcher/src/ui/`.
3. Implement Home and account/instance selection end to end.
4. Migrate remaining pages one by one with parity tests.
5. Add visual/accessibility tests and performance baselines.
6. Make the replacement the default only after Windows, Linux and macOS validation.

## Acceptance criteria

- All existing launcher workflows remain available.
- No account, instance, world or settings data is lost or relocated.
- No new mandatory network requests for decorative UI.
- Keyboard-only navigation and reduced motion work.
- No regression in launch, account, download or persistence tests.
- Build and smoke tests pass on Windows, Linux and macOS before cutover.
