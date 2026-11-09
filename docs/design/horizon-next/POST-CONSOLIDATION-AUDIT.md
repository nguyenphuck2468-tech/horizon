# Horizon Next - Post-Consolidation Audit

**Status:** audit only; no production code, branches, PRs or history were changed.
**Date:** 2026-11-09
**Repository:** `https://github.com/nguyenphuck2468-tech/horizon`
**Baseline branch:** `launcher`
**Baseline commit:** `2d26cdbcfeba1765104182cfa0b1960f09e256f2`
**Author:** audit run for the Horizon Next programme

---

## 0. How to read this document

Every material statement carries one of three tags:

- **[V]** Verified in this audit against the checked-out repository, the GitHub API, or a command whose output is quoted.
- **[A]** Assumption or inference. It is plausible and consistent with the evidence, but was not proved. Treat it as a hypothesis to confirm before acting.
- **[R]** Recommendation. Not a statement about the current system.

No claim below should be treated as verified merely because a prior document, PR description or
commit message said so. Where a prior figure disagreed with the repository, the repository wins and
the disagreement is recorded in section 12.

---

## 1. Executive summary

The consolidation is real and the baseline is trustworthy. `launcher` is at
`2d26cdbcfe`, the working tree is clean, `origin/launcher` matches local HEAD, and `main` is an
ancestor 642 commits behind. The 8 merged topic branches are fully absorbed; the consolidation
report itself (PR #7) is merged. **[V]**

The engineering baseline is **not yet safe enough to build a new shell on without a short Phase 0**.
Three properties of the current launcher are load-bearing for the next phase and are currently
weaker than the architecture document implies:

1. **Durable user state is not crash-safe.** `accounts.json` and `instances.json` are rewritten in
   place, a corrupt or partially written file loads as an empty list without an error, and the next
   save overwrites the original. A crash at the wrong moment can silently discard every account or
   every instance's metadata. `settings.json`, `servers.json` and the wardrobe already use
   temp-file-plus-rename, so the fix is a pattern the codebase already knows. **[V]**
2. **The pull-request gate does not run the binary or integration suites.** CI runs
   `cargo test --locked --lib` for the launcher. The 117 binary tests and 11 integration tests only
   run in `windows-exe.yml`, which does not run on ordinary pull requests. A regression in process
   lifecycle, startup, or UI logic can merge green. **[V]**
3. **The facade is a first slice, not a boundary.** It exposes 26 functions for a 99,232-line,
   898-public-function crate (about 2.9 %). N04+ cannot yet be held to "UI calls the facade only"
   because most preserved workflows have no facade entry point and nothing enforces the rule. **[V]**

On top of those, the audit found a cluster of high-severity issues that are cheap to fix now and
expensive to fix after the old shell is removed: fragmented network retry policy, a process
lifecycle that can wait forever on inherited pipes / orphan grandchildren, silent loss of unknown
settings keys, and untyped English `String` errors that make retry and localization decisions
impossible.

The `horizon-next/glass-experience` branch is **mostly superseded, not lost work**: 7 of its 8
unique commits are either already implemented more strictly on `launcher` or duplicate existing
tests. Three small artefacts deserve recovery (a `fetch_small` retry refinement, the design note,
and one coverage test) plus one unpushed local test commit. Section 10 gives the recovery plan.
**[V]**

PR #1 is **stale but fully absorbed**: its head `b540150684` and base `a65baf021d` are both
ancestors of `launcher`, and `git rev-list b540150684 ^launcher` is 0. It should be closed as
superseded, not merged. Section 11 records the independent review and the one real defect its AI
review found (a TOCTOU that the current `launcher` has since solved more completely). **[V]**

### Verdict

| Question | Answer |
|---|---|
| Is `launcher` a trustworthy baseline? | **Yes** - state verified, tree clean, consolidation absorbed. |
| Can N04 start today? | **Only behind a flag**, and only after Phase 0 items P0-1..P0-3. |
| Is the preservation contract executable? | **Partially** - 24 workflows, 13 smoke-tested, 11 manual-only. |
| Is feature parity provable? | **No** - facade coverage 2.9 %, no E2E parity suite. |
| Is CI a reliable merge gate? | **No for the launcher** - lib only. |
| Is packaging releasable on all platforms? | **No** - macOS jobs are queued on an offline runner. |
| Is PR #1 needed? | **No** - fully contained in `launcher`; close as superseded. |

### Top ten actions, in order

1. Make `accounts.json` and `instances.json` atomic (temp + fsync + rename + backup) and stop
   silently resetting corrupt files. (P0-1)
2. Add a single-instance guard and a cross-process state-file lock. (P0-2)
3. Add `--bins --tests` to the `launcher` job in `ci.yml`, and fix or quarantine the
   `a_replaced_binary...` test. (P0-3)
4. Complete the facade for the twelve preserved areas before N04 UI work begins, with a
   compile-time or lint boundary. (P0-4 / N03b)
5. Unify network policy in one retry/backoff module and route every provider download through it.
   (P0-5)
6. Make process control tree-aware and time-bounded: process group / job object, drain pipes with
   a deadline, kill the tree. (P0-6)
7. Add a launcher-owned structured log with rotation and a crash record. (P0-7)
8. Restore a macOS runner for packaging, or explicitly mark macOS unsupported for this milestone.
   (P0-8)
9. Decide the fate of `glass-experience` per section 10 and record it. (P0-9)
10. Replace `String` errors at the facade boundary with a typed `FacadeError`. (P0-10)

---

## 2. Method and evidence

### 2.1 What was inspected

- Repository state: HEAD, branch, working tree, stashes, local and remote branches, ancestors and
  merge bases.
- All Horizon Next documents: `rebrand-architecture.md`, `preservation-matrix.md`,
  `workflow-map.md`, `change-log.md`, `roadmap.md`, plus `docs/maintenance/BRANCH-CONSOLIDATION.md`.
- N01-N03 implementation: `next_preservation.rs`, `next_navigation.rs`, `next_facade.rs`.
- Production source: accounts/session/secrets, backend/install/download/update, instances, servers,
  Tailscale, launch/process, import/export, worlds/screenshots, datapacks, Java, diagnostics,
  signed releases, config/settings, installer, desktop entry, macOS update, CLI.
- CI and packaging: 24 workflow files, Linux AppImage, macOS app/dmg, Windows setup/exe.
- `horizon-next/glass-experience`: commit list, divergence, diff against `launcher`, local
  unpushed commit.
- PR #1: metadata, 100 commits, 111 files, review comments, ancestor checks.

### 2.2 Commands that establish the baseline

Run inside the codespace at `/workspaces/horizon` with `cargo` from `$HOME/.cargo/bin`:

```bash
git status --porcelain=v1
git log -1 --format='%H %ci %s'
git rev-list --left-right --count launcher...origin/launcher
git rev-list --left-right --count launcher...origin/horizon-next/glass-experience
cargo +stable fmt --check
cargo +stable test --locked --lib
cargo +stable test --locked --bin horizon-launcher
```

GitHub state was read with `gh api` / `gh run list`; CI job-level state with
`/actions/runs/<id>/jobs`.

### 2.3 Limits of this audit

- The audit was executed in a Linux codespace. Windows-only and macOS-only code paths were read,
  not executed. Runtime behaviour on those platforms is **[A]** unless stated.
- The binary test suite could not be linked initially because the codespace disk was full. After
  removing `target/debug/incremental` and `launcher/target/debug/incremental` (about 4.9 GB), the
  suite built and ran. This is an environment event, not a repository defect. **[V]**
- The audit did not run a real game launch, a live Modrinth/CurseForge download, a Tailscale
  network, or a Microsoft/Ely.by sign-in. Those remain manual paths, as N01 already records. **[V]**
- Counts of `unwrap`/`expect` were produced with a raw grep and are **not** a reliable defect
  metric because many hits are inside inline `#[cfg(test)]` modules. They are reported only as a
  triage pointer. **[A]**

---

## 3. Verified current state

### 3.1 Repository

| Item | Value | Status |
|---|---|---|
| HEAD | `2d26cdbcfeba1765104182cfa0b1960f09e256f2` | [V] |
| Branch | `launcher` | [V] |
| Working tree | clean, no untracked files, no stashes | [V] |
| `origin/launcher` | identical to local HEAD | [V] |
| `main` | `d8edfbcd0f`, ancestor of `launcher` | [V] |
| `launcher` ahead of `main` | 642 commits | [V] |
| Last commit | merge of PR #7, `2026-11-09 14:04:22 +0700` | [V] |

### 3.2 Branches (local and remote)

| Branch | Tip | Disposition |
|---|---|---|
| `main` | `d8edfbcd0f` | release line, ancestor only |
| `launcher` | `2d26cdbcfe` | **baseline** |
| `horizon-next/rebrand-foundation` | `d4b118e32b` | merged via PR #6 |
| `horizon-next/glass-experience` | `390376aaac` (remote) / `5b079d048f` (local) | diverged experiment, section 10 |
| `elyby-on-upstream` | `a65baf021d` | absorbed, kept |
| `horizon-repo-review-plan-abfc6` | `b540150684` | PR #1 head, absorbed, kept |

The 8 fully integrated topic branches and 4 local branches were deleted during consolidation. The
branch count in `BRANCH-CONSOLIDATION.md` matches the repository after the correction commit. **[V]**

### 3.3 Pull requests

| PR | Title | State | Note |
|---|---|---|---|
| #1 | Update from task 7a1b1a65... | OPEN, UNSTABLE | fully absorbed, section 11 |
| #2-#5 | phase work | MERGED | contained in `launcher` |
| #6 | rebrand foundation | MERGED as `da11574439` | N00-N03 |
| #7 | branch consolidation | MERGED as `2d26cdbcfe` | maintenance report |

### 3.4 CI state at audit time

| Run | Workflow | Branch | State |
|---|---|---|---|
| `42576202223` | `launcher.yml` | `launcher` @ `da11574439` | `pending`, **0 jobs** |
| `42552620178` | `launcher.yml` | `launcher` @ `cadddef84d` (dispatch) | `queued`: `build linux` success, `build windows` success, `build macos` and `build macos-x64` queued indefinitely |
| `42576730530` | `ci.yml` | `maintenance/branch-consolidation` | success |

The macOS jobs have no runner assigned. The self-hosted Mac mini (`ludovics-mac-mini`, per
workflow comments) is unavailable. This is a **verified block on macOS packaging**, not a code
failure. **[V]** The `pending` push run with zero jobs is a scheduling anomaly worth watching; it
may start later. **[A]**

### 3.5 Local fence

| Check | Result |
|---|---|
| `cargo +stable fmt --check` | pass |
| `cargo +stable clippy --all-targets --locked -- -D warnings` | pass (per project fence record; re-run during audit did not contradict) |
| lib tests | 586 listed: **581 passed, 1 failed, 4 ignored** |
| failing lib test | `backend::tests::a_replaced_binary_is_replaced_again_at_its_path`, `assert_eq!` `495 == 493` |
| binary tests | 117 listed, **117 passed** |
| integration tests | 11 in `launcher/tests` (`desktop`, `hidden_processes`, `logs`, `owner_gate`, `startup`) |

The failing lib test is `#[cfg(unix)]` and compares a file length after `replace_binary`. The
observed mismatch is 2 bytes, consistent with a shell script / wrapper being written with a
different length than the test expects in this environment, or with a temp-directory collision
between concurrent test runs. The root cause is **[A]**; the failure is **[V]**. It is not
documented in the fence, yet the project record claims a green lib fence. That contradiction is a
finding (H2).

---

## 4. Baseline scale and structure

| Metric | Value | Status |
|---|---|---|
| Rust files under `launcher/src` | 186 | [V] |
| Lines under `launcher/src` | 99,232 | [V] |
| Public functions in `launcher/src` (rough grep) | 898 | [V] |
| `#[test]` attributes in `launcher/src` | 719 | [V] |
| Integration tests in `launcher/tests` | 11 | [V] |
| Preserved feature areas | 12 | [V] |
| Preserved workflows (N01) | 24 (13 smoke, 11 manual) | [V] |
| Routes (N02) | 20 | [V] |
| Workspaces (N02) | 10 | [V] |
| Facade contract tests (N03) | 12 | [V] |
| Facade public functions | 26 | [V] |

Largest production modules by line count: `main.rs` 5,721; `backend.rs` 5,427; `cli.rs` 3,958;
`native_mods.rs` 3,900; `servers_page.rs` 3,333; `account.rs` 2,807; `mods_page.rs` 1,913. **[V]**

The two 5k-line modules plus the page modules are the reason the facade is necessary. They also
mean that a "new shell" which calls pages directly would inherit the same coupling under a new
name.

---

## 5. Feature preservation matrix

Legend for **Facade**:
- `full` - the workflow can be driven through `next_facade` without touching internals.
- `partial` - some use cases are exposed; the rest require internals.
- `none` - no facade entry point.

Legend for **Automation**:
- `smoke (n)` - n N01 workflows call the production entry point in a test.
- `manual` - covered by unit suites but not executable end-to-end in CI without network/game.

| # | Area | Existing behaviour on `launcher` | Automation | Facade | Main gaps |
|---|---|---|---|---|---|
| 1 | Accounts and identity | Microsoft device-code, Ely.by device-code, offline names; keychain + encrypted fallback; ownership gate; session refresh; provider-safe cards | smoke (3) + manual | partial | `accounts.json` atomicity; typed auth errors; multi-account concurrency; no stale-session invariant test |
| 2 | Versions and builds | Release list, Nightly v2 signature, tag ordering, install, prune, keep-newest | smoke (2) | partial | no facade for install/prune/update; version schema has no migration marker |
| 3 | Install, prepare, launch | Prepare state, signed downloads, SHA-256 verify, retry 502-504, cancel between files, launch plan, env, memory | smoke (4) + manual | partial | no facade for prepare/launch/cancel; global cancel flag; pipe/process-tree lifecycle |
| 4 | Instances | Create, clone, import, export, delete, selection, metadata, other-launcher import | smoke (2) + manual | partial | `instances.json` non-atomic; no facade for mutations; no schema version |
| 5 | Mods and packs | Modrinth/CurseForge/Horizon catalogues, resource packs, shaders, datapacks, modpacks, dependency presentation | smoke (1) + manual | partial | only `pack_kind` exposed; no facade for search/install/remove; fragmented download retry |
| 6 | Worlds and screenshots | World list, active-world guard, backup, copy, delete; screenshot grid | smoke (1) | **none** | whole area absent from facade; backup has no verification/restore test |
| 7 | Multiplayer | Official + custom server list, reachability, MOTD, quick join | manual | partial | `slug/share/snippet` only; no ping/join facade |
| 8 | Servers and network | Tailscale install/status/up/down/ACL/port sharing, token in keychain; server dashboard and console | smoke (1) + manual | partial | no facade for start/stop/status; server process lifecycle |
| 9 | Java and launch settings | Java discovery/selection, Rust toolchain, memory, env, game args, JVM flags | smoke (2) | partial | no facade for Java selection/args/env persistence |
| 10 | Console and diagnostics | Game log tail, crash reports, downloads page, telemetry opt-out, repair | smoke (1) | **none** | no facade; no launcher-owned log; no bundle export |
| 11 | Skins and cosmetics | Skins, wardrobe, offline skins, theme equip | smoke (1) | **none** | no facade |
| 12 | Configuration and platform | Settings, i18n, desktop entry, shortcuts, installer, updater, signed releases | smoke (1) | partial | only client IDs; no settings/updater facade; unknown-key loss |

**Reading the matrix:** the *behaviour* is largely present and tested at unit level. The *contract*
for a new shell is not. Areas 6, 10 and 11 have no facade at all, and areas 3, 4 and 5 - the core
of the product - expose only display helpers, not the mutations a new UI must call.

### 5.1 Preservation quality by area

**Strong [V]:**
- Signed release verification is stricter than the glass branch: `SHA256SUMS.v2.sig` is required
  for Nightly and signed for the exact version, not just the channel.
- Secrets never enter `accounts.json`; the fallback file is ChaCha20-Poly1305 sealed, 0600 on Unix,
  and guarded by a lock file across processes.
- Path traversal in import is guarded with `Component` checks and canonicalization.
- Proxy support is centralized in `proxy::agent` for the main download path.
- Launcher renderer environment variables are stripped before launching the game, preventing the
  known software-rendering regression.
- No `TODO`/`FIXME`/`HACK` markers remain in `launcher/src`. **[V]**

**Weak [V]:**
- Three state files are written non-atomically (`accounts.json`, `instances.json`,
  `datapacks.json`), while `settings.json`, `servers.json` and the wardrobe are atomic. The
  inconsistency is the defect.
- Error type is `String` at nearly every public boundary.
- Network policy is duplicated: `backend::get` retries 502-504; `browser/download.rs` has its own
  agent; Ely.by has a separate retry; Tailscale has its own 20 s agent; `game_log` and `wardrobe`
  call `ureq` directly.
- The process API is `Running { child, stdin }` with `kill()` on the direct child only, and
  `spawn` waits for stdout/stderr EOF before reporting exit.

---

## 6. Facade coverage analysis

`next_facade.rs` exposes **26 functions** in 8 groups:

| Group | Exposed | Missing for the new shell |
|---|---|---|
| Accounts | `card_in`, `card`, `refresh`, `sign_out`, `has_owner`, `play_as`, `SignIn::{finish_ely, finish_microsoft, add_to}` | account list mutation, provider discovery, offline-name edit, ownership explanation |
| Tags/versions | `kind`, `order`, `nightly_label`, `nightly_date` | install, update, remove, prune, progress |
| Launch | `memory`, `memory_at`, `env`, `env_text` | prepare, start, cancel, wait, exit code, kill |
| Instances | `play_time`, `meta` | create, clone, import, export, delete, select, edit |
| Content | `pack_kind` | search, install, remove, update, dependency graph, download progress |
| Servers | `slug`, `share`, `share_snippet` | list, add, edit, ping, join, tailnet status, start/stop |
| Settings | `client_ids`, `resolve_client_id` | load, save, migrate, language, theme, telemetry |
| Diagnostics | `native_state` | log tail, crash report, repair, download history, support bundle |

**Coverage: 26 / 898 public functions = 2.9 %.** If the count is restricted to the functions the
12 areas actually need, coverage is still under 10 %, because the missing entries are the
mutating workflows. **[V]**

There is no enforcement that a new UI module imports only `next_facade`. A grep-based CI check or a
`pub(crate)` split would make the boundary real. Without it, N04 will call `backend` and
`instances` directly because that is the path of least resistance. **[R]**

---

## 7. Findings

Severity is assigned by user impact and by cost to fix after the old shell is removed, not by how
interesting the bug is.

### 7.1 Critical

#### C1 - `accounts.json` and `instances.json` are not crash-safe and silently reset on corruption

**Status: [V]**

`Accounts::save_to` does `fs::write(path, text + "\n")`; `Store::save` in `instances.rs` does the
same for `instances.json`. `Accounts::load_from` returns `Self::default()` on any read or parse
error; `Store::load` returns `Store::default()`. Neither reports the error. The next save writes the
empty/default state over the original file.

Impact:
- A crash, power loss, or full disk during save leaves a truncated file.
- The launcher starts normally with no accounts (or no instance metadata) and no warning.
- The first automatic save destroys the only recovery copy.
- Instance folders survive on disk, but names, memory, Java choice, loader and play time in
  `instances.json` are lost, which makes the library look empty.

Why critical: this is unbounded, silent, irreversible user-data loss in the two files that define
the product's identity and library. It is also cheap to fix with a pattern already used by
`Settings::save`.

Recommendation **[R]**: write to a temp file in the same directory, `sync_all`, rename, keep a
`.bak` of the previous version, and surface a distinct `Corrupt` result so the UI can offer
"restore backup" or "start fresh" instead of silently defaulting. Add tests for truncated JSON,
missing fields, and a failed rename.

#### C2 - No single-instance guard and no cross-process lock on shared state

**Status: [V]** (absence verified; race outcome is **[A]**)

`Settings::save` has a process-local `static Mutex`, which does nothing across processes. A second
launcher window, the CLI (`launcher/src/cli.rs`), or a background job can load, mutate and save the
same `accounts.json` / `instances.json` / `settings.json` concurrently. The secrets file has a lock
file; the state files do not. There is no single-instance guard, so two full launchers can run
against one data directory.

Impact: lost updates (last writer wins), duplicated downloads, two game processes, conflicting
`selected` account/instance state, and state files that disagree with the secrets store.

Recommendation **[R]**: create a lock file with the process id in the data directory at startup,
hold it for the process lifetime, and either focus/exit the second instance or support an explicit
`--allow-multiple` for tests. Wrap every read-modify-write in an OS-level file lock (the same
mechanism `FileStore` already uses). Add a test that runs two writers.

### 7.2 High

#### H1 - Pull-request CI does not run binary or integration tests for the launcher

**Status: [V]**

`ci.yml` `launcher` job runs:

```
cargo +stable fmt --check
cargo +stable clippy --all-targets --locked -- -D warnings
cargo +stable test --locked --lib
```

`--lib` excludes the 117 binary tests and the 11 integration tests. `windows-exe.yml` runs
`cargo test --release --locked`, but it triggers only on pushes to `main` / a hardcoded feature
branch, `launcher@*` tags, manual dispatch, and pull requests that modify the workflow file itself.
Therefore an ordinary PR can break `main.rs` test logic, process lifecycle, startup or owner-gate
integration and still merge green.

Recommendation **[R]**: add `--bins --tests` to the `launcher` job. The compile cost is already
paid by clippy `--all-targets`; the marginal run time is small compared with a broken baseline.

#### H2 - The documented fence is not reproducible: one lib test fails in a clean Linux environment

**Status: [V]** failure, **[A]** root cause

`backend::tests::a_replaced_binary_is_replaced_again_at_its_path` fails with `495 == 493`. The
project record says the fence is green, and N01-N03 notes say "lib 581 + bin 117". The audit
reproduced the failure on the same baseline. Either the test is environment-sensitive or the
record was taken on a machine where it passed. Both are unacceptable for an audit baseline.

Recommendation **[R]**: make the test isolate its temp directory per run (unique suffix, cleanup on
drop), assert the semantic property (the file is the new content and executable) instead of a byte
length, and record the platform in the fence. If the length assertion is intentional, document the
platform assumption.

#### H3 - Network resilience is fragmented and provider downloads bypass the retry policy

**Status: [V]**

- `backend::get` retries HTTP 502, 503 and 504 three times with 250 ms / 500 ms backoff.
- `browser/download.rs` builds its own agent and performs its own download; it does not inherit
  that policy.
- `ely.rs` has a separate retry for 502/503/504.
- `tailscale.rs` builds a 20 s agent with no retry.
- `game_log.rs` calls `ureq::post` / `ureq::delete` directly.
- `wardrobe.rs` builds its own agent and maps status/transport errors separately.

Impact: the same network condition produces "retry succeeds" for a game file and "download failed"
for a mod or a resource pack. Error messages and UX differ by provider. There is no single place to
add jitter, respect `Retry-After`, or cap total attempts.

Recommendation **[R]**: one `net` module with `get_bytes`, `get_json`, `download_to` and a shared
policy (connect timeout, read timeout, retryable status set, backoff with jitter, cancellation
token). Providers supply only URLs and validators.

#### H4 - Process lifecycle can hang on inherited pipes and leaks grandchildren

**Status: [V]** code shape, **[A]** frequency

`backend::spawn` pipes stdin/stdout/stderr, starts reader threads, and sends `Msg::Exit` only after
the readers reach EOF. `Running::kill` calls `kill()` on the direct child. If a launcher script,
wrapper, or Java child spawns a grandchild that inherits stdout/stderr, the pipes stay open after
the direct child exits, so the UI never sees `Exit`. Conversely, killing the direct child can
orphan the JVM.

Impact: the Play button remains "running", the next launch is blocked, and a zombie JVM holds the
game directory and memory.

Recommendation **[R]**: put the child in its own process group (Unix `setsid`) or a Job Object
(Windows), close inherited handles, add a drain deadline after `wait()` returns, and kill the tree
on cancel/exit. Add an integration test with a script that spawns a sleeping grandchild.

#### H5 - `settings.json` silently discards unknown keys on the next save

**Status: [V]**

`Settings::parse` deserializes into `serde_json::Value`, removes a few known-bad values, then
`serde_json::from_value`. Unknown fields are not preserved (the struct has no `#[serde(flatten)]
extra`), and `save` serializes only known fields. A player who runs a newer build, then an older
one, loses the newer keys the first time the older build saves.

The code comments show awareness of forward compatibility for `appearance`, `while_playing`,
`accent`, `glass`, `blur` and `language`, but only for invalid values, not for unknown keys.

Recommendation **[R]**: keep an `extra: BTreeMap<String, Value>` with `#[serde(flatten)]` and write
it back. Add a round-trip test with an unknown key and an unknown enum value.

#### H6 - Public errors are untyped English `String`s

**Status: [V]**

Nearly every boundary returns `Result<T, String>`. The UI cannot distinguish offline, 401, 403,
404, 429, rate limit, checksum mismatch, cancelled, corrupt state or ownership failure without
parsing prose. Retry policy cannot branch. Localization cannot translate. The N01 manual-only list
exists partly because these paths cannot be asserted.

Recommendation **[R]**: introduce `FacadeError` at the facade boundary with variants
`Offline`, `Timeout`, `Http { status, retry_after }`, `Auth`, `Ownership`, `Checksum`,
`Cancelled`, `CorruptState { path }`, `Io`, `Other`. Keep `String` internally if necessary, but
convert at the boundary. This is a prerequisite for N12 (task centre) and N18 (diagnostics).

#### H7 - macOS packaging and release verification are blocked

**Status: [V]**

The `launcher.yml` dispatch run `42552620178` has Linux and Windows success and both macOS jobs
queued with no runner since 02:45. The push run `42576202223` for `launcher` has 0 jobs. There is
no evidence in this audit of a successful macOS build for the consolidated baseline.

Impact: N22 ("verify Windows Linux and macOS distribution") cannot pass; a release cut from
`launcher` would ship Linux and Windows only.

Recommendation **[R]**: either bring the Mac mini runner online and re-run, or add a
GitHub-hosted `macos-14` fallback with the documented signing/notarization path. Record the
decision in the release runbook.

#### H8 - The facade is not yet a boundary; N04 would re-couple the UI

**Status: [V]**

2.9 % coverage, no enforcement, and three preserved areas with no entry points. The architecture
document says the new shell calls existing workflows; it does not say what happens when a workflow
has no facade. In practice N04 will import `backend`, `instances` and page modules directly.

Recommendation **[R]**: treat facade completion as part of N03 (call it N03b) and gate N04 on it
for the screens N04 actually opens (Home, library, accounts, versions). Add a CI grep that fails
when a module under `next_` imports a non-facade internal module.

#### H9 - `glass-experience` divergence is a silent-loss risk

**Status: [V]**

8 unique commits on the remote, 1 local unpushed commit, 51 commits behind. The branch is not
merged and not protected. Its unique behaviour is mostly superseded (section 10), but the design
note and the retry refinement are not on `launcher`. If the branch is deleted during a future
cleanup without the recovery step, those artefacts disappear.

Recommendation **[R]**: execute the recovery plan (section 10) and then archive the branch with a
tag, or delete it only after the recovered items are merged.

### 7.3 Medium

| ID | Finding | Evidence | Impact | Recommendation |
|---|---|---|---|---|
| M1 | No schema version in `accounts.json` or `instances.json` | struct definitions have only data fields [V] | cannot write a migration or detect old/new files reliably | add `version: u32` with default 1 and a migration function |
| M2 | Corrupt/partial state is indistinguishable from "empty" at the type level | `unwrap_or_default()` in both loaders [V] | UI cannot offer recovery; support cannot diagnose | return `Result<State, LoadError>` with `Missing` vs `Corrupt` |
| M3 | Download cancellation is global and coarse | `static CANCEL: AtomicBool` in `mojang.rs`; checked between files [V] | two concurrent prepares cancel each other; a single large file cannot be cancelled mid-read | per-job cancellation token checked in the read loop |
| M4 | No resume for launcher/game binary downloads | `download` opens `File::create` and hashes from zero [V] | a 200 MB update restarts after a network drop | support HTTP Range where the server allows it, with checksum verification at the end |
| M5 | No launcher-owned structured log or rotation | `eprintln!` only; `logs_dir()` is the game's log [V] | support cannot reconstruct a launcher failure; crash reports lack context | write `horizon-launcher.log` in data dir with size rotation and a panic hook |
| M6 | Panic handling is unspecified | no `panic::set_hook`, no `catch_unwind` in the launcher [V] | a UI panic can lose unsaved state or terminate silently | hook that records a crash file and shows a diagnostic dialog |
| M7 | `datapacks.json` is written in place | `fs::write` at `datapacks.rs:135` [V] | world datapack selection can be lost on crash | reuse the atomic helper |
| M8 | Import has path checks but no archive size/compression limits | `Component`/canonicalize guards found [V]; size limits not found [A] | zip bomb or huge archive can exhaust disk | enforce entry count, uncompressed size and ratio limits |
| M9 | `unsafe` blocks are undocumented in four modules | `launch.rs`, `shortcut.rs`, `rust_toolchain.rs`, `titlebar.rs` [V] | maintenance risk; platform behaviour opaque | add `// SAFETY:` comments and a short platform note |
| M10 | Windows code signing and notarization are not visible in the audited path | `windows-exe.yml` checks imports and hashes, no signing step seen [V] | SmartScreen warnings; trust gap | confirm the signing configuration or document its absence |
| M11 | i18n coverage for the Next shell is not enforced | 100+ lang files; no cross-language key test on `launcher` [V] | new keys can ship untranslated or crash the loader | add a key-parity test (the glass branch had one, see section 10) |
| M12 | `settings.json` save uses a PID-named temp file without a lock across processes | `settings.{pid}.tmp` + process-local mutex [V] | two processes with the same PID is impossible, but concurrent saves still last-writer-wins | reuse the state lock from C2 |
| M13 | Server lifecycle ownership is unclear | `servers_page` spawns many threads; `servers.rs` has a lock and `.spawn()` [V] | stop/start races, orphaned tasks | one server supervisor with explicit states and a stop timeout |
| M14 | Update rollback is not specified | `replace_binary` swaps the file; mac/AppImage paths exist [V] | a bad update can leave an unusable binary with no automatic fallback | keep `previous` binary, health-check after restart, roll back on immediate crash |

### 7.4 Low

| ID | Finding | Evidence | Recommendation |
|---|---|---|---|
| L1 | `roadmap.md` claims a "659-test fence" and `cargo test -p horizon-launcher` | actual: 586 lib + 117 bin, and the crate is its own workspace [V] | update the doc; use the real commands |
| L2 | `windows-exe.yml` triggers on a hardcoded feature branch `horizon-repo-review-plan-abfc6` | workflow `on.push.branches` [V] | remove before the branch is retired |
| L3 | Stale branches remain (`elyby-on-upstream`, `horizon-repo-review-plan-abfc6`) | branch list [V] | archive with tags after PR #1 is closed |
| L4 | Codespace disk pressure from duplicate `target` trees | 18 GB repo, 4.9 GB free before cleanup [V] | document `cargo clean` scope; remove `incremental` in CI-like runs |
| L5 | `REPO` and `LAUNCHER_REPO` point to different GitHub owners | `horizon-rust/horizon` vs `nguyenphuck2468-tech/horizon` [V] | document the split in the release runbook |
| L6 | Facade docs say 12 contract tests but do not name the boundary rule | `change-log.md` [V] | state "no new shell module imports internals" and enforce it |
| L7 | Raw `unwrap`/`expect` count is high but unclassified | 2,291 hits including tests [A] | triage only production sites; do not mass-rewrite |

### 7.5 Positive findings worth preserving

These are engineering assets; the new shell must not regress them. **[V]**

- Signed Nightly v2 verification is strictly better than the glass branch's version.
- Secrets are never serialized into `accounts.json`; the fallback store is encrypted and locked.
- The ownership gate passes a validated credential snapshot, closing the TOCTOU found in PR #1's
  review.
- Import path traversal is checked at every entry.
- The launcher strips its own renderer environment before launching the game.
- Proxy configuration is applied through one helper on the main path.
- The N01 preservation module fails if a preserved area loses its workflow.
- Tests do not reach the telemetry server (`HORIZON_TELEMETRY_URL=off`).
- No TODO/FIXME markers remain in `launcher/src`.

---

## 8. Technical debt inventory

Debt is listed with the trigger that makes it urgent, so it can be scheduled rather than felt.

| # | Debt | Cost today | Trigger to pay | Owner phase |
|---|---|---|---|---|
| D1 | Non-atomic state writes (C1) | silent data loss | any crash report | P0-1 |
| D2 | No instance/state lock (C2) | lost updates, double launch | CLI + GUI use in parallel | P0-2 |
| D3 | Lib-only PR gate (H1) | broken `main` | first UI PR | P0-3 |
| D4 | Facade at 2.9 % (H8) | re-coupling | N04 first screen | P0-4 |
| D5 | Fragmented network policy (H3) | inconsistent downloads | N12 task centre | P0-5 |
| D6 | Process lifecycle (H4) | stuck Play, orphans | first cancelled launch | P0-6 |
| D7 | `String` errors (H6) | no retry/localization | N12/N18 | P0-10 |
| D8 | No launcher log (M5) | blind support | first field report | P0-7 |
| D9 | No schema versions (M1) | migration ambiguity | first format change | N03b |
| D10 | Unknown-key loss (H5) | settings loss on downgrade | first forward-compatible key | P0-1 |
| D11 | Page modules as de facto API | 5k-line coupling | N04 | P0-4 |
| D12 | Manual-only paths (11) | parity not provable | N21 | N21 |
| D13 | macOS runner (H7) | no macOS artifact | N22 | P0-8 |
| D14 | Glass branch divergence (H9) | lost artefacts | next branch cleanup | P0-9 |
| D15 | Update rollback (M14) | bad release | first staged rollout | N23 |

---

## 9. Platform and packaging status

| Platform | Build | Package | Verification | Status |
|---|---|---|---|---|
| Linux x86_64 | `launcher.yml`, GitHub runner | `.tar.gz` + AppImage | Linux job success in `42552620178` | OK |
| Windows x64 | `launcher.yml` + `windows-exe.yml` | `.exe` / setup exe | Windows job success; `windows-exe` builds and hashes | OK, signing unclear (M10) |
| macOS arm64 | `launcher.yml`, self-hosted Mac mini | `.app` / `.dmg` | **queued, no runner** | BLOCKED |
| macOS x64 | cross-built on the same runner | `.app` / `.dmg` | **queued, no runner** | BLOCKED |

Linux desktop entry, icons, Windows per-user installer, and macOS update path all exist in source.
The macOS notarization scripts exist (`packaging/macos/notarize.sh`, `keychain.sh`) but were not
executed in this audit. **[V]** for existence, **[A]** for correctness.

---

## 10. `horizon-next/glass-experience` recovery plan

### 10.1 Divergence

- 51 commits behind `launcher`, 8 ahead on the remote, 1 local unpushed commit. **[V]**
- Merge base `5106b8902994ae882e58c78035c75957f4cc307e`. **[V]**
- Diff: 92 files, +430 / -123, dominated by 86 language files and a new design document. **[V]**
- Local-only commit `5b079d048f` adds a test that all pages have a name and icon. **[V]**

### 10.2 Commit-by-commit disposition

| Commit | Subject | Disposition | Reason |
|---|---|---|---|
| `1ea7876d2a` | test: pin play_press and starts_now launch behavior | **Drop** | duplicates coverage already on `launcher` |
| `70c942c84c` | test: pin play_press and starts_now launch behavior | **Drop** | duplicate of the above |
| `3491df193f` | docs: define glass experience redesign (C00) | **Recover** | unique design rationale not on `launcher` |
| `560382f9f6` | fix: retry `fetch_small` read_to_end on transient errors | **Evaluate** | `launcher` retries status codes; body-read retry may still be absent |
| `666d0f0a6e` | config: point REPO at upstream | **Already present** | `REPO = horizon-rust/horizon` on `launcher` |
| `ac85610083` | fix: REPO upstream + Nightly v2 signatures | **Superseded** | `launcher` implements v2 more strictly |
| `ce2426e86b` | fix: retry `fetch_small` on transient 504 | **Superseded** | `backend::get` retries 504 |
| `390376aaac` | fix: REPO, retry 504, Nightly v2, accounts page stub | **Mostly superseded** | Accounts stub replaced by Next Accounts workspace; REPO/v2/retry already present |
| `5b079d048f` (local) | test: pin all pages have name and icon | **Recover** | cheap guard not present on `launcher` |

Net recoverable value: one design document, one possible body-read retry refinement, one test, and
one local test commit. Everything else is already implemented more strictly on `launcher`.

### 10.3 Recovery procedure (recommended, not executed)

1. Create `horizon-next/glass-recovery` from `launcher`.
2. Cherry-pick `3491df193f` as a docs-only commit; retitle with the current date and link it from
   `change-log.md` as historical input, not as the current design of record.
3. Inspect the `fetch_small` body-read retry in `560382f9f6`; if it adds value over `backend::get`,
   implement it in the unified `net` module (D5) with a loopback test, not as a branch cherry-pick.
4. Port the `all_pages_have_name_and_icon` test from `5b079d048f` onto the current route metadata
   (it should iterate `ROUTES`, not page modules).
5. Do **not** port the Accounts page stub, REPO changes, or v2 signature code.
6. After the recovered items merge, tag the old branch tip `archive/glass-experience-2026-11` and
   delete the branch only with explicit owner approval.

**Constraint honored in this audit:** the branch was not modified, merged, deleted or pushed. **[V]**

---

## 11. PR #1 assessment (Ely.by)

### 11.1 Facts

| Field | Value |
|---|---|
| Number / title | #1, "Update from task 7a1b1a65-f8ed-447c-8521-bebe124abfc6" |
| Author | `nguyenphuck2468-tech` |
| Body | "This PR was created by qwen-chat coder for task ..." |
| State | OPEN, not draft |
| Merge state | `UNSTABLE`, `MERGEABLE` |
| Base | `elyby-on-upstream` @ `a65baf021d` |
| Head | `horizon-repo-review-plan-abfc6` @ `b540150684` |
| Size | 100 commits, 111 files, +3,760 / -868 |
| Created / updated | 2026-11-04 / 2026-11-06 |
| Reviews | 1 AI review (Qwen3.8), `COMMENTED` |
| Commits outside `launcher` | **0** |
| Head ancestor of `launcher` | **yes** |
| Base ancestor of `launcher` | **yes** |

All facts above are **[V]**.

### 11.2 Content assessment

The PR's stated purpose is correct and valuable: stop passing Ely.by tokens to vanilla Mojang Java
instances, add defensive `--userType` handling, and localize the resulting messages across 86
language files. The code it introduced is already in `launcher`, and the current implementation has
evolved beyond it:

- The review found a TOCTOU: `secrets_in` fetched a fresh secret, checked `is_ely`, then discarded
  it and used the cached `pass.session`. A stale Ely.by secret could still reach the game.
- `launcher` now passes a validated credential snapshot through the ownership gate and writes the
  launch session file with `write_launch_session_file_if_current`, which closes the window more
  completely than the PR's fix.
- Nightly v2 signature verification is stricter on `launcher`.

**The PR is therefore functionally absorbed and should not be merged.** Merging it would create a
merge commit with no unique content, and could reintroduce an older shape of the account code if
conflicts are resolved badly. **[V]** for absorption, **[R]** for close-as-superseded.

### 11.3 Recommendation

1. Comment on PR #1 with the ancestor evidence (`git rev-list b540150684 ^launcher` = 0) and the
   list of superseding commits.
2. Close it as "superseded; content absorbed into `launcher`".
3. Keep the `elyby-on-upstream` and `horizon-repo-review-plan-abfc6` refs as tags for traceability
   until the owner agrees to delete them.
4. Do not merge. Do not force-push. Do not rewrite the branch.

This audit did not close, merge, or comment on the PR. **[V]**

---

## 12. Documented figures vs verified figures

| Claim in docs | Verified value | Verdict |
|---|---|---|
| baseline `2d26cdbcfe` | `2d26cdbcfeba1765104182cfa0b1960f09e256f2` | matches [V] |
| `launcher` +642 over `main` | 642 | matches [V] |
| 24 preserved workflows | 24 (13 smoke, 11 manual) | matches [V] |
| 20 routes | 20 | matches [V] |
| 10 workspaces | enumerated workspaces in `next_navigation` | matches [V] |
| 12 facade contract tests | 12 | matches [V] |
| "lib 581 + bin 117 green" | 581 pass / 1 fail / 4 ignored; bin 117 pass | **does not match** (H2) [V] |
| "659-test fence" in `roadmap.md` | 586 lib + 117 bin listed | stale [V] |
| PR #1 open and unmerged | open, UNSTABLE | matches [V] |

---

## 13. Phased roadmap and acceptance criteria

The N00-N24 sequence remains sound. This audit inserts a **Phase 0** before N04 because three
baseline properties would otherwise be baked into the new shell, and it splits N03 into N03b for
facade completion. Phases after N04 keep the original numbering.

### Phase 0 - Baseline hardening (before any N04 UI work)

**P0-1. Durable state.**
- Atomic write helper used by `accounts.json`, `instances.json`, `settings.json`, `datapacks.json`,
  `servers.json`, wardrobe.
- `.bak` retained; corrupt load returns an error type, never a silent default.
- Unknown settings keys round-trip.
- **Acceptance:** fault-injection tests truncate each file mid-write and after a simulated crash;
  the next start either recovers the previous version or reports `CorruptState`; no test leaves a
  `.tmp` file behind. A test proves an unknown settings key survives load + save.

**P0-2. Single instance and cross-process lock.**
- Lock file in `data_dir()`; second instance focuses or exits with a clear message.
- All read-modify-write paths hold the lock.
- **Acceptance:** a test starts two writers and proves no lost update; a test starts a second
  instance and proves it does not open a second window; `--allow-multiple` is test-only.

**P0-3. CI gate.**
- `ci.yml` launcher job runs `--bins --tests` in addition to `--lib`.
- `a_replaced_binary...` is fixed or explicitly quarantined with a tracked issue.
- **Acceptance:** a PR that breaks a binary test fails `checks`; the launcher job is green on
  `launcher`; the fence command in docs matches CI exactly.

**P0-4 / N03b. Facade completion for the N04 surface.**
- Facade entries for accounts list/edit, instance create/clone/import/export/delete/select,
  version install/prune/update, prepare/launch/cancel/kill, content search/install/remove, server
  list/start/stop, settings load/save, diagnostics log/crash.
- **Acceptance:** every screen N04 opens can be driven without importing a non-facade module; a
  grep or visibility check enforces the rule; facade contract tests cover each new entry.

**P0-5. Unified network module.**
- One retry/backoff policy; provider downloads and Ely.by/Tailscale/game-log calls routed through
  it; cancellation token per job.
- **Acceptance:** a loopback test serves 503, 504, then 200 for a game file, a mod file and a
  profile request; each succeeds with the same policy; `Retry-After` is honored; cancel stops an
  in-flight read.

**P0-6. Process lifecycle.**
- Process group / Job Object; drain deadline; tree kill; explicit states.
- **Acceptance:** a testscript that spawns a grandchild exits and the UI receives `Exit`; cancel
  kills the tree; no orphan remains.

**P0-7. Launcher diagnostics.**
- Rotating log and crash record in `data_dir()`.
- **Acceptance:** a forced panic writes a crash file with version, OS, route and last 100 log
  lines; the support bundle includes it.

**P0-8. Platform decision.**
- Restore the macOS runner or declare macOS out of scope for this milestone.
- **Acceptance:** a `launcher.yml` run produces Linux, Windows and macOS artifacts, or the release
  checklist explicitly excludes macOS with an owner-signed note.

**P0-9. Glass recovery.**
- Execute section 10.
- **Acceptance:** design note and the two tests are on `launcher` or a next branch; the branch is
  tagged or deleted with approval; no unique commit is lost.

**P0-10. Typed facade errors.**
- `FacadeError` at the boundary; UI maps variants to localized messages.
- **Acceptance:** no new facade function returns `Result<_, String>`; tests assert variant mapping
  for offline, auth, ownership, checksum, cancelled and corrupt state.

### N04-N08 - Shell and design system

- N04 app shell behind a flag, old shell still reachable.
- N05 tokens and typography.
- N06 accessible primitives.
- N07 metadata-driven navigation and command routing.
- N08 navigation + visual fixtures.
- **Acceptance:** the flag defaults off; the old shell passes the P0 parity smoke; route metadata
  is the single source of truth; keyboard-only navigation passes; snapshots are committed for
  Home, library, accounts, versions at 3 window sizes.

### N09-N13 - Core experience

- N09 home and launch readiness; N10 instance library/detail; N11 account center; N12 task and
  download center; N13 launch/account compatibility.
- **Acceptance:** every N01 workflow in areas 1-4 runs through the new shell with the same
  outcome; download centre shows progress, cancel, retry, recovery; account cards never expose
  tokens; a parity test drives sign-in fixture -> play -> exit.

### N14-N19 - Content and platform

- N14 content centre; N15 dependency/compatibility; N16 servers/multiplayer; N17 worlds/media/
  cosmetics; N18 diagnostics/repair; N19 settings.
- **Acceptance:** areas 5-12 of the matrix pass through facade calls only; dependency conflicts are
  explained, not just blocked; server start/stop is observable and bounded; a support bundle can
  be produced from the UI.

### N20-N24 - Hardening and release

- N20 bounded caches and virtualization (target: low-end hardware, 4 GB RAM, integrated GPU).
- N21 end-to-end parity scenarios (replace the 11 manual-only paths).
- N22 Windows/Linux/macOS distribution verification.
- N23 staged rollout and rollback, only if the updater audit confirms the gap.
- N24 enable by default after sign-off.
- **Acceptance:** peak RSS bound and frame-time budget documented and met on the reference low-end
  machine; N21 covers all 24 workflows; N22 artifacts install and launch on each OS; N23 rollback
  is demonstrated; N24 requires explicit owner approval and a rollback switch.

---

## 14. Risks and open questions

| # | Risk / question | Type | Owner decision needed |
|---|---|---|---|
| R1 | Is macOS support required for the next release? | product | yes |
| R2 | Is `launcher` allowed to receive Phase 0 fixes before N04, or must everything land on `horizon-next/*`? | process | yes |
| R3 | Should the single-instance guard focus the existing window or exit the second process? | UX | yes |
| R4 | Is a `.bak` file acceptable in the data directory, or should backups live under a `backups/` subfolder? | UX | no |
| R5 | Which platforms are "supported" for N22 acceptance? | product | yes |
| R6 | Should the glass design note become the design of record, or remain historical input? | design | yes |
| R7 | Is Windows code signing in place outside the audited workflow? | release | yes |
| R8 | What is the maximum supported data size for import (zip bomb policy)? | security | yes |

---

## 15. Appendix - evidence index

### 15.1 Files read in full or in relevant part

```
docs/design/horizon-next/rebrand-architecture.md
docs/design/horizon-next/preservation-matrix.md
docs/design/horizon-next/workflow-map.md
docs/design/horizon-next/change-log.md
docs/design/horizon-next/roadmap.md
docs/maintenance/BRANCH-CONSOLIDATION.md
launcher/src/next_preservation.rs
launcher/src/next_navigation.rs
launcher/src/next_facade.rs
launcher/src/account.rs
launcher/src/account/session.rs
launcher/src/account/tests.rs (selected)
launcher/src/backend.rs
launcher/src/instances.rs
launcher/src/servers.rs
launcher/src/servers_page.rs
launcher/src/launch.rs
launcher/src/process.rs
launcher/src/import.rs
launcher/src/export.rs
launcher/src/worlds.rs
launcher/src/datapacks.rs
launcher/src/java.rs
launcher/src/java_launch.rs
launcher/src/mojang.rs
launcher/src/browser/download.rs
launcher/src/game_log.rs
launcher/src/tailscale.rs
launcher/src/signed.rs
launcher/src/settings (Settings in backend.rs)
launcher/src/desktop.rs
launcher/src/installer.rs
launcher/src/mac_update.rs
launcher/src/cli.rs
launcher/src/main.rs
launcher/Cargo.toml
.github/workflows/ci.yml
.github/workflows/launcher.yml
.github/workflows/windows-exe.yml
packaging/linux/appimage.sh
packaging/linux/horizon-launcher.desktop
packaging/macos/package.sh
packaging/macos/notarize.sh
```

### 15.2 Git facts

```
HEAD           2d26cdbcfeba1765104182cfa0b1960f09e256f2
origin/launcher 2d26cdbcfe
main           d8edfbcd0f  (ancestor)
launcher vs main: 642 ahead, 0 behind
glass-experience: 51 behind, 8 ahead
merge base:    5106b8902994ae882e58c78035c75957f4cc307e
PR #1 head     b540150684  (ancestor of launcher)
PR #1 base     a65baf021d  (ancestor of launcher)
git rev-list b540150684 ^launcher = 0
```

### 15.3 Test facts

```
lib:  586 listed, 581 passed, 1 failed, 4 ignored
bin:  117 listed, 117 passed
integration: 11 (desktop 1, hidden_processes 6, logs 1, owner_gate 2, startup 1)
failure: backend::tests::a_replaced_binary_is_replaced_again_at_its_path
         assertion 495 == 493 (unix-only)
```

### 15.4 CI facts

```
run 42576202223  launcher.yml  launcher@da11574439  pending, 0 jobs
run 42552620178  launcher.yml  launcher@cadddef84d  decide/linux/windows success,
                                                      macos + macos-x64 queued, no runner
run 42576730530  ci.yml        maintenance/branch-consolidation  success
```

### 15.5 Statements that are assumptions, not verified facts

- Runtime behaviour on Windows and macOS was read, not executed.
- The root cause of the `495 == 493` failure is inferred from the assertion and the test setup.
- The concurrency outcome of two launchers writing state is reasoned from the absence of a lock;
  it was not reproduced.
- Frequency of inherited-pipe hangs in real game launches is unknown.
- Archive size limits in import were not found by grep; they may exist in a helper not inspected.
- Windows code signing may exist outside the audited workflow.
- The "pending, 0 jobs" run may be a transient GitHub scheduling state.

---

## 16. Closing statement

The consolidation achieved what it claimed: one trustworthy `launcher` baseline, absorbed
branches, and an accurate maintenance record. The next phase should not begin with UI work. It
should begin with ten small, testable hardening changes that protect user state, make CI a real
gate, and complete the facade for the screens the new shell will open first. After that, the
N04-N24 plan is executable with measurable acceptance criteria rather than aspirational ones.

This document is the baseline of record until a later audit supersedes it. It is deliberately
docs-only: no production file, branch, PR or release was changed by this work.
