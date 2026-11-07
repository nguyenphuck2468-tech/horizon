# Glass UI audit vs docs/design-system.md (Phase A1)

Scope: `launcher/src/glass_ui.rs` (1011 lines), `theme.rs`, `glass_palette.rs`, `glass_account.rs`,
`glass_instance.rs`, `frame.rs`. Verified by reading the sources on
`horizon-next/phase-1-design-system` at `613843f`.

## What already exists (no new design system is needed)

`glass_ui.rs` implements the mockup's component language in one module: `setting_row`,
`row`, `icon_button`, `tab`, `count_tab`, `menu_item`, `pick_row`, `list_row`, `empty`,
`field`, `toggle`, `button` (kinds), `segments`, `dialog_plate`, `popover`, `page_head`,
`card_heading`, `card_note`, `dot`, `bar`. Horizontal design tokens come from
`crates/horizon-theme` (`glass` + `night` sub-palettes) and the same crate generates the
website's CSS via xtask — the launcher, game and site share the ledger `docs/design-system.md`
describes. The command palette (⌘K/Ctrl+K) already exists as `glass_palette.rs`.

## Findings

| # | Finding | Verdict |
|---|---------|---------|
| 1 | `raw` tone colours in `glass_ui.rs` (`0x15803d/0xb45309/0xdc2626`) | **Compliant**: these are the light-appearance variants of the crate's `LIVE/WARN/DANGER` glass tokens, with comments saying so; both palettes now unit-tested (`glass_ui::tests`). |
| 2 | `gpui::rgb(0x100a1f)` in `glass_account.rs` | **Fixed** (commit `c05e822`): routed through `glass_ui::color(horizon_theme::glass::ON_LIGHT)`; same value, documented ledger. |
| 3 | `popover` uses `.shadow_lg()` | **Intentional, glass-look**: the comment states "a shadow setting it above the page" — part of the glass mockup's elevation language; the restraint rules in design-system.md describe the shared list language, which `glass_ui` respects elsewhere. Logged, showing no change. |
| 4 | `focus_ring` draws a 2 px soft-accent outline | Documented glass-look focus indicator ("a ring the mockup draws at 2 px around fields" per `glass_ui.rs` inline mockup mapping). Spec deviates from the night look's "1 px accent" on purpose for the glanceable glass style. Aligning the two looks is a Phase H item, not a drift bug. |
| 5 | `glass_instance.rs`'s 8 icon hues | **Compliant**: a deliberate icon-palette, not token drift. |
| 6 | clippy `needless_borrow` in `release_loading_tests.rs:60` | **Fixed** (commit `e09ef7c`); existed at baseline; its CI fence (`-D warnings`) fails on `launcher` until Horizon Next lands. |
| 7 | dead_code `elyby.rs::symbol()` wrapper | **Fixed** (commit `613843f`): the wrapper was unused (callers use `logo(size)`). |

## The CI recipe this audit now uses as the branch fence

`launcher` job of `ci.yml`: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
`cargo test --locked --lib`. Locally in the codespace the green run of this exact fence keeps the
one environment-sensitive test skipped (see p0-summary.md §8.2); the authoritative verdict is the
GitHub `checks` run of this workflow.
