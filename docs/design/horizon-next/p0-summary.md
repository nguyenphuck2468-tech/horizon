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

## 8.4 Phase 0 ? foundation hardening: steps 2-3 completion review (2026-11-09)

Follow-up to the Phase 0 foundation-hardening programme (data integrity, process safety, CI,
networking, lifecycle). This section records the completion review of **Step 2 (data integrity)**
and **Step 3 (process safety)**: both were implemented on dedicated branches, reviewed against the
Phase 0 acceptance criteria, and the incomplete criteria were closed and verified.

### 8.4.1 Step 2 ? data integrity (branch `phase-0/data-integrity-complete`, PR pending)

Final state after the completion review: branch `phase-0/data-integrity-complete` @
`0b1889e75103be984c0d08c61b21afb4e54d1a5b` (base `launcher` @ `2d26cdbcfe`, clean tree before the
final commit).

- `launcher/src/persistence.rs` (new): `atomic_write_bytes` (temp file in the same directory +
  `fsync` + rename over the target; unique temp name = pid + timestamp + per-process counter;
  `.bak` of the previous state kept; temp file cleaned up when the write/sync fails), `atomic_write_json`,
  `LoadError::{Missing, Corrupt, Io}`, `load_json_validated`, `load_json`, `is_readable::<T>`.
- `launcher/src/account.rs`: `Accounts::load_from` reports through `LoadError` (visible diagnostic on
  corrupt); `Accounts::load_validated` (kept for the next shell); `Accounts::save_to` refuses to
  overwrite a corrupt or unreadable file.
- `launcher/src/instances.rs`: `Store::load` via persistence + diagnostic; `Store::load_validated`;
  `Store::save` refuses to overwrite a corrupt file.

Acceptance (Step 2): (1) corrupt state cannot silently destroy user data -- **PASS** after the
completion fix (`save_to` refuses corrupt overwrite; corrupt load emits a visible diagnostic and
keeps the `.bak` recovery path); (2) interrupted writes cannot replace valid data with partial
content -- **PASS** (temp + fsync + rename + `.bak`); (3) concurrent writers are covered by the
unique temp file names plus regression tests (same-process concurrency).

### 8.4.2 Step 3 ? process safety (branch `phase-0/process-safety`, PR pending)

Final state: branch `phase-0/process-safety` @ `06f7529dfc` (base `launcher` @ `2d26cdbcfe`).

- `launcher/src/single_instance.rs`: `StateLock` (cross-process mutual exclusion via lock file +
  owner PID liveness + stale-lock recycle + release on drop) and `SingleInstance` (application-level
  guard on `launcher.lock`); `process_alive` implemented for Linux (`/proc`) and macOS/BSD
  (`kill(pid, 0)`, via `libc`); Windows uses a conservative "treat as held" probe (documented).
- `launcher/src/main.rs`: `SingleInstance::acquire(&backend::data_dir())` wired into the startup
  path before shared state (settings/accounts/instances) is read or written; exits with code 2 when
  the data directory is already owned by a live instance.
- `launcher/Cargo.toml`: added `libc = "0.2"`.

Regression tests: `single_instance::tests` 3/3 pass (acquire-once, stale-lock reuse, StateLock
exclusion/release).

### 8.4.3 Test evidence (Linux, codespace `scaling-giggle-q7wrxvr7qg6qc49wv`)

| Check | Step 2 final (`phase-0/data-integrity-complete` @ `0b1889e751`) | Step 3 final (`phase-0/process-safety` @ `06f7529dfc`) |
|---|---|---|
| `cargo fmt --check` | OK | OK |
| `cargo clippy --all-targets --locked -- -D warnings` | OK | OK |
| `cargo test --locked --lib` | 595 passed, 1 failed, 4 ignored | 584 passed, 1 failed, 4 ignored |

The single failure in both runs is the pre-existing environment-sensitive
`backend::tests::a_replaced_binary_is_replaced_again_at_its_path` (`495 == 493`), documented in
`POST-CONSOLIDATION-AUDIT.md`; it is unrelated to Steps 2-3.

### 8.4.4 Acceptance criteria status (explicit PASS / FAIL / BLOCKED)

- Data integrity, Step 2: corrupt-state protection **PASS**; interrupted-write protection **PASS**;
  concurrent-writer coverage **PASS** (after the completion fix).
- Process safety, Step 3: single-instance guard + cross-process `StateLock` **PASS** (wired into
  startup, Linux-verified); stale-lock recovery implemented for Linux/macOS (**PASS** design);
  Windows conservative (**open**); runtime evidence for macOS/Windows still **BLOCKED**.
- CI quality gates (Step 4), networking (Step 5), process lifecycle/diagnostics (Step 6): **NOT
  STARTED** in this session (deferred until Steps 2-3 land).

### 8.4.5 Remaining limitations and open items

1. macOS/Windows runtime test evidence (codespace environment is Linux only).
2. Windows stale-lock handling is conservative (documented; manual lock cleanup after a crash).
3. PID-reuse theoretical race in stale-lock detection (accepted risk, documented).
4. `Accounts::load_validated` / `Store::load_validated` are not yet called by production callers;
  kept for the next shell with backward-compatible `load_from` / `Store::load`.

### 8.4.6 Next steps

- Step 4 ? CI quality gates (add `--bins --tests` to the launcher PR fence; platform split).
- Step 5 ? networking retry/timeout unification (502/503/504, backoff+jitter, budgets, cancellation).
- Step 6 ? process lifecycle, typed errors, launcher log, diagnostics.
