## Verification forecast (added during Horizon Next Phase 1)

The `checks` job recipe is `fmt --check` + `clippy --all-targets --locked -D warnings` +
`test --locked --lib`. On baseline `-launcher` HEAD `0206f28` this forecast predicts a
`checks` failure with exactly two clippy warnings (needless_borrow, dead_code elyby.rs:20),
both documented and independently fixed on `horizon-next/phase-1-design-system` (`e09ef7c`,
`613843f`). If run 42375384481 fails on anything else, that becomes the next independent fix.
