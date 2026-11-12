# Standalone Horizon Launcher — extraction boundary

Status: proposed implementation plan. No source files are moved or deleted by this document.

## Goal

Move the desktop launcher product out of the Horizon game-engine repository and build a genuinely new frontend in its own repository. The Horizon repository remains the source of truth for the game, engine, protocol, mod APIs and shared launcher services. The new launcher repository owns the desktop application, visual design, packaging, releases and UI CI.

Proposed destination repository: `nguyenphuck2468-tech/Horizon-Launcher`.

The existing `nguyenphuck2468-tech/Launcher` repository is a PineconeMC/Prism Launcher fork. It is not the Horizon Rust launcher and must not be repurposed or overwritten.

## What exists today

- `launcher/` is already an independent Cargo package and has its own Cargo workspace/lockfile; it is excluded from the root workspace.
- `launcher/src/lib.rs` exposes the launcher service/library surface (accounts, backend jobs, instances, Java, launch, modpacks, import/export, telemetry, etc.).
- `launcher/src/main.rs` is the current GPUI desktop application and owns the old page/theme/view composition.
- The package is not independently relocatable yet: `launcher/Cargo.toml` points at many `../crates/*` packages, and `launcher/build.rs` reads the icon from `../packaging/icons`. Documentation and other assets also refer to files outside `launcher/`.

Therefore copying `launcher/` verbatim into a new repository would break Cargo path dependencies and build assets. A clean boundary must be introduced first.

## Target architecture

```text
nguyenphuck2468-tech/horizon                 nguyenphuck2468-tech/Horizon-Launcher
┌───────────────────────────────────┐       ┌──────────────────────────────────────┐
│ Game / engine / protocol / mods    │       │ Desktop app + new UI                 │
│                                   │       │                                      │
│ crates/horizon-launcher-core  ◄───┼───────┤ apps/desktop                         │
│   stable service API              │ pinned│   GPUI frontend (new view tree)       │
│                                   │ git rev│                                      │
│ crates/horizon-theme (optional) ◄─┼───────┤ crates/launcher-design                │
│                                   │       │   launcher-specific tokens/components│
│ No desktop UI or release pipeline  │       │ packaging / installers / auto-update │
└───────────────────────────────────┘       └──────────────────────────────────────┘
```

### Boundary

1. Extract a new `horizon-launcher-core` library package in the Horizon repository. Initially it may point at the existing backend source in `launcher/src/lib.rs` so that the extraction is small and reviewable; it must not depend on GPUI or any view module.
2. Give the core crate a deliberate, documented API for app state, commands, events, account state, instance summaries, install/launch progress and errors. Avoid exposing the old `Launcher` view model or GPUI types.
3. The new repository consumes that package at an exact Horizon Git revision. No floating branch dependency in release builds.
4. The new repository owns the new desktop shell, design system, app navigation, packaging and release workflows.
5. Keep the current launcher in Horizon as a compatibility frontend while the new app reaches feature parity. Do not delete the old frontend or alter existing users' data paths until migration and upgrade tests pass.

### Dependency and asset handling

- The new core facade belongs in the root Horizon Cargo workspace so it can depend on the existing local Horizon crates without copying the engine.
- The facade must use a minimal backend-only dependency list. GPUI, GPUI platform integration, window/view modules, and binary-only CLI dependencies stay in the desktop application.
- Move the Windows icon and any launcher-owned build assets into the new repository. Keep game-owned assets and shared engine crates in Horizon.
- Any required shared data (release metadata, mod registry formats, preparation protocol) must be versioned and tested as an API, not copied as an undocumented implementation detail.
- Pin the core Git revision in the new repository's lockfile and record it in build provenance.

## New UI direction

The new interface is a redesign, not a reskin of Glass/Night and not a WebView wrapper.

- Native Rust desktop app; retain GPUI for the first implementation to avoid a heavy runtime, a second rendering stack, and a risky backend rewrite. Re-evaluate the renderer only after measurements on the low-end target.
- Calm dark-first visual language with an optional light theme, clear typography, consistent spacing, restrained surfaces, crisp icons and purposeful motion.
- Navigation rail for Home, Instances, Mods, Servers, Screenshots and Settings; contextual sub-navigation belongs inside each page rather than in a crowded global bar.
- Home is a useful dashboard: selected instance, version/loader, account, Java/runtime health, install/update state, recent activity and one unmistakable Play action.
- Instances is the management center: search, sort, filters, create/import/duplicate/edit, and clear per-instance status.
- Long-running operations are observable and cancellable where safe; progress, retries and errors are explicit.
- Use progressive disclosure for advanced JVM flags, environment overrides, developer tools and diagnostics.
- Preserve accessibility, keyboard navigation, localization (all supported locales), reduced-motion support and high-DPI scaling.

## Performance policy

Do not claim the new UI is lighter based on appearance alone.

Before replacing the old frontend, record comparable release-build measurements on the same machine and dataset:
- cold start to interactive;
- idle and active resident memory;
- idle CPU and CPU during scrolling/search;
- frame-time distribution during navigation and list scrolling;
- GPU use and animation cost where available;
- binary size and clean-install size.

The new UI must not regress launch reliability or background task behavior. Prefer virtualized lists, cached immutable view data, bounded image decoding, no continuously-running decorative animation, and no full-window blur by default.

## Migration phases

1. **Boundary:** add and test `horizon-launcher-core`; keep the current launcher working unchanged.
2. **New repository:** bootstrap independent Cargo workspace, CI matrix, pinned core dependency, icons, localization pipeline and release metadata.
3. **Design system:** implement app shell, navigation, tokens, typography, buttons, fields, cards, dialogs, toasts and loading/error states.
4. **Vertical slice:** Home + account display + instance selection + Play, wired to the existing backend. Test Microsoft, Ely.by and offline accounts before adding more pages.
5. **Feature parity:** migrate Versions, Mods/Modpacks, Servers, Worlds, Screenshots, Console, Settings and diagnostics one vertical slice at a time.
6. **Performance and release:** compare measurements, exercise upgrade/data migration, then publish opt-in test builds. Keep the legacy launcher available until the new app is proven.

## Non-goals and safety rails

- Do not rewrite Microsoft/Ely.by OAuth, token storage, Java discovery, game preparation or process launching as part of the UI rewrite.
- Do not remove the current launcher, change its default data directory, or silently migrate user data.
- Do not merge PRs or switch the default downloadable build until the new app passes feature, account, launch, upgrade and platform checks.
- Keep Horizon Native Instance Builder and SOUL AI independent; they are not part of this UI extraction.
