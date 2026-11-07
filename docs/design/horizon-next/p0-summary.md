## Verification forecast (added during Horizon Next Phase 1)

The `checks` job recipe is `fmt --check` + `clippy --all-targets --locked -D warnings` +
`test --locked --lib`. On baseline `-launcher` HEAD `0206f28` this forecast predicts a
`checks` failure with exactly two clippy warnings (needless_borrow, dead_code elyby.rs:20),
both documented and independently fixed on `horizon-next/phase-1-design-system` (`e09ef7c`,
`613843f`). If run 42375384481 fails on anything else, that becomes the next independent fix.

## 8.3 Outcome: run 42375384481 (baseline `checks` at `0206f28`) -- recorded 2026-11-07

Jobs (Actions API): `verify` **success**; `launcher` **failure** -- exactly in the
"Check the launcher's formatting, lint and tests" step; `parity-scope`/`parity` skipped;
`vanilla-parity`/`jfr-analysis` left queued (no self-hosted runner picked them up; no
conclusion). The launcher failure is the forecast above: the fence's
`clippy --all-targets --locked -- -D warnings` fails on the two warnings present at baseline
(`needless_borrow` release_loading_tests.rs:60, `dead_code` elyby.rs:20 `symbol()`). Both are
fixed on `horizon-next/phase-1-design-system` (`e09ef7c`, `613843f`), where the same fence is
green (fmt/clippy/test in the codespace). **Phase 0 closes: baseline verified superset;
the two clippy warnings recorded as baseline known-issues; no further changes to `launcher`.**
