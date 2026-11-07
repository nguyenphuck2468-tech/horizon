# Horizon Next — Phase 0 summary (repository & feature audit)

Baseline branch: `launcher` — HEAD `0206f2891d0451a0765d54b55c95ffd3eb10271d`.

## Branch topology (verified via compare API, 2026-11-07)

All feature branches share merge-base `d8edfbc` (= current `main`):

```
main
 └─ elyby-on-upstream (+398)
     └─ feat/elyby-sign-in-ui (+167)
         └─ fix/windows-ci-secrets-file-tests
             └─ feat/launcher-repo-split (+3)
                 ├─ horizon-repo-review-plan-abfc6 (+4)
                 │    └─ launcher (+2 ← 5a9f214, 0206f28)
                 └─ feat/game-from-fork (+6, same HEAD as launcher)
```

`launcher` is a strict superset of every other branch (behind = 0 vs all). Its two unique
commits point the updater at this fork's own Windows nightlies and reuse the Ely.by icon/skin.

## Inventory (verified)

- `launcher/src`: 133 .rs files, 28 subdirectories; workspace-included crate `horizon-launcher` 0.2.0
  (launcher/ is excluded from the game workspace; it shares crates with the game: horizon-prepare,
  horizon-mcp, horizon-world, horizon-cosmetics, horizon-mods, horizon-protocol, horizon-friends).
- UI stack: gpui-fast 0.1.13 (Linux wgpu), glass_* modules.
- Security: chacha20poly1305 secrets file when no keychain; keyring/oo7 for Windows/macOS/Linux;
  Modrinth SHA-512 checks; signed updates via horizon-release-sign.
- 659 `#[test]` in launcher/src (account 110, backend 50, native_mods 49, sources 18, config 15,
  github_mods 14, ely 14, instances 13, browser 13, launch 12, mojang 11, servers 10, ...).
- 91 docs at docs/ (launcher-preparation, launcher-commands, mod-browser, modpacks, mods,
  microsoft-account, elyby-account, telemetry, security, release-flow, design-system, ...).
- 23 CI workflows on the launcher branch.

## Verification state at baseline

- `windows-exe` run 42221961716 at `0206f28`: **success**.
- `checks` (ci.yml, job `verify`): never run on the launcher branch at this HEAD; restarted via
  workflow_dispatch run 42375384481 (**result: see §8.3 below**).
- Codespace `cargo test -p horizon-launcher` (runs 2–5): **549 passed / 1 failed / 4 ignored**.
  The single failure (backend::tests::a_replaced_binary_is_replaced_again_at_its_path) was
  classified **environment-sensitive, not a code regression**, on a five-step evidence chain:
  /tmp has a default ACL (mode 0o646 files regardless of umask); umask 0022 cannot override it;
  TMPDIR on /workspaces reports 0o666 files; TMPDIR on /dev/shm additionally breaks three other
  tempdir tests. `replace_binary` follows POSIX semantics; the tests assume GitHub-runner /tmp.
  **Scope: this verdict covers the investigated failure only, not all filesystem behaviour.**

## Decisions (user-approved charter)

- Baseline stays exactly at `0206f28`. No rebasing/rewriting of historical branches; none are deleted.
- App UI work proceeds on a new branch per phase; `launcher` stays frozen.
- Warnings are logged, not in-flight-fixed (dead_code at src/elyby.rs:20 `pub(super) fn symbol()`
  — kept until the `checks` run settles what the canonical CI sees).
- 659 tests are a regression fence, not a proof of completeness: the preservation matrix below
  still links feature → docs → module → test → verification state, including untested features.
