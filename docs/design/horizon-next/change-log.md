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
