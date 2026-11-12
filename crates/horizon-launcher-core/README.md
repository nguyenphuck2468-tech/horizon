# Horizon launcher core

This package is the backend boundary for the standalone Horizon desktop application.

## Consumer contract

The standalone UI should depend on the documented compatibility facade:

- `horizon_launcher_core::next_facade` for account-safe display/sign-in flows, version classification, launch options, instance facts and other UI-facing use cases.
- `horizon_launcher_core::next_navigation` for stable route/workspace identifiers and feature inventory.
- `horizon_launcher_core::next_preservation` for the executable feature-preservation contract used while replacing the old shell.

Do not build new UI code against internal implementation modules such as `account::Secrets`, backend storage internals, or the old page/view modules. In particular, secrets must remain inside the core and must never be serialized into UI state, logs, diagnostics or IPC payloads.

## Transitional source layout

The library source currently lives at `launcher/src/lib.rs`. The `[lib]` path deliberately points there during extraction, allowing the legacy GPUI app and a future standalone frontend to share the same backend implementation.

This is a bridge, not the final source layout. The facade manifest intentionally excludes GPUI and GPUI platform crates. The old desktop binary keeps its own GPUI dependency, CLI dependencies and build resources.

## Version pinning

The future `nguyenphuck2468-tech/Horizon-Launcher` repository must pin this package to an exact Horizon Git revision (not a branch or tag that can move). Record that revision in build provenance and keep the generated lockfile committed.

Example dependency shape (replace the placeholder with the reviewed commit SHA):

```toml
horizon-launcher-core = { git = "https://github.com/nguyenphuck2468-tech/horizon", rev = "<exact-40-character-commit-sha>" }
```

## Compatibility and migration

The existing `launcher/` frontend remains supported until the standalone app passes account, launch, data-path, feature-parity, upgrade and platform checks. The core package does not migrate or delete user data.

Any API breaking change must be accompanied by:
1. a compatibility note and updated version/pin;
2. tests for Microsoft, Ely.by and offline account semantics where relevant;
3. preservation tests for the affected launch/instance workflows;
4. a migration note if any on-disk format or data path changes.
