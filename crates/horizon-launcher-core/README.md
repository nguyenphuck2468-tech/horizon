# Horizon launcher core

This package exposes the existing launcher service layer to independent desktop frontends.

## Extraction strategy

The library source currently lives at `launcher/src/lib.rs`. The `[lib]` path deliberately points there for this transition, so the old GPUI application and the new standalone application use the same backend implementation while it is being separated.

This is a bridge, not the final source layout. Do not add GPUI, window/view types, page state, or presentation components to this crate. Keep the public surface UI-independent: account/session data, instance summaries, commands, progress events, errors, Java discovery, preparation, and process launching.

## Consumers

The future `nguyenphuck2468-tech/Horizon-Launcher` repository should pin this package to an exact Horizon Git revision. Release builds must not depend on a moving branch.

## Compatibility

The existing `launcher/` package remains the supported desktop application until the standalone frontend has passed account, launch, data-path, feature-parity, upgrade and platform tests. This package does not migrate or delete any user data.
