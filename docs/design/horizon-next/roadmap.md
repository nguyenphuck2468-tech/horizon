# Horizon Next — working plan (charter: feature preservation first)

Owning branch series: `horizon-next/*` off `0206f289` (`launcher`). `launcher` stays frozen.

## Phases (from the user-approved charter, reordered by evidence)

- P0 — repository & feature audit: DONE (see p0-summary.md, preservation-matrix.md).
- P1 — design system: the repo already defines one (docs/design-system.md + `crates/horizon-theme`
  tokens powering launcher, game UI and website through generated CSS). Therefore P1 here =
  (a) audit `launcher/src/glass_ui.rs` (+ glass_palette/theme wrappers) against the documented
  component language, (b) close the gaps as small, tested changes, (c) document findings.
- P2 — main experience (Home, navigation, instance library, accounts, versions, play):
  after P1 lands, on `horizon-next/phase-2-*`.
- P3+ — content experience, system tools, server platform, polish, release verification, each
  with its own branch, its own run of `cargo test -p horizon-launcher`, and eventually CI on PRs.

## Working rules (standing, user-approved)

1. Freeze feature/UI changes on `launcher`; everything lands on `horizon-next/*` branches.
2. No reimplementation of working behaviour; UI calls the existing workflows/services.
3. Every change compiles, clippy-clean, fmt-clean, and keeps the 659-test fence green.
4. Warnings logged, not in-flight-fixed (dead_code elyby.rs:20; needless_borrow will be checked
   by CI `checks` on the baseline before any separate cleanup commit).
5. No files outside the repo (no local machine artifacts); codespace `scaling-giggle-q7wrxvr7qg6qc49wv`
   is the build/test environment while the GitHub runners are the authoritative CI.
6. Journal: log each landed change in this file under "Log".
