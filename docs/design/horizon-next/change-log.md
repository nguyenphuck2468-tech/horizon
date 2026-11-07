# Horizon Next — change log

## 2026-11-07

- Phase 0 close-out docs added (p0-summary, preservation-matrix, roadmap). `ccec427`
- `e09ef7c` fix(clippy): needless borrow in release-loading test — independent fix, CI-fence-compliant.
- `c05e822` refactor(glass): sign-in ink via the token ledger + first `glass_ui` appearance tests.
- `613843f` fix(elyby): drop the dead `symbol()` wrapper (dead_code with `-D warnings`).
- Phase 1 audit published: docs/design/horizon-next/glass-ui-audit.md.
- Local fence on `horizon-next/phase-1-design-system`: fmt/clean, clippy `-D warnings` clean
  (both baseline warnings gone), `cargo test --lib` 549 passed / 0 failed / 4 ignored
  (1 skip: the documented environment-sensitive test; authoritative check = GitHub CI).
- `5a74dcf22` refactor(palette): extract the query filter into the pure `palette_filter`,
  match queries case-insensitively, unit-test the cap/order/matching, and pin the documented
  CLI play command (`--instance`). Phase 2 (Home) audited: no missing feature
  (`glass-home-audit.md`).
- Phase 0 closed with the outcome of run 42375384481 (p0-summary.md 8.3): `verify` green at
  baseline, `launcher` failing in the fence step exactly as forecast; fixes already on the
  horizon-next branch.
- Environment: the codespace moved from basicLinux32gb (2 cores / 8 GB) to standardLinux32gb
  (4 cores / 16 GB) with the user's go-ahead; bin-target tests became practical
  (95 passed / 0 failed).
