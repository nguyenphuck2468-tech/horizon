# Branch consolidation — 2026-11-09

Status: complete for the safe actions described below. Two items are deliberately left for
human review (PR #1 and `horizon-next/glass-experience`); no branch was deleted for either.

## 1. Baseline

| Item | Value |
|---|---|
| Repository | `nguyenphuck2468-tech/horizon` |
| Default branch | `main` @ `d8edfbcd0f` (unchanged) |
| Horizon Next base branch | `launcher` |
| `launcher` before | `cadddef84d` (`merge: Horizon Next phase 3 (content experience) into launcher`) |
| `launcher` after | `da11574439` (`Merge pull request #6 from nguyenphuck2468-tech/horizon-next/rebrand-foundation`) |
| Branches before (origin) | 15, excluding `upstream/*` |
| Branches after (origin) | 7, excluding `upstream/*` |
| Local working tree | clean (`git status --short` empty) |
| Stashes | none |

`main` is an ancestor of `launcher` (launcher is 630 commits ahead); no release or `main` move was
performed. The `upstream` remote (`horizon-rust/horizon`) is a different repository and was not
touched.

## 2. Pull requests

| PR | Head → base | State | Merge commit / note |
|---|---|---|---|
| [#2](https://github.com/nguyenphuck2468-tech/horizon/pull/2) | `horizon-next/phase-1-design-system` → `launcher` | MERGED | `5106b89029` |
| [#3](https://github.com/nguyenphuck2468-tech/horizon/pull/3) | `horizon-next/phase-2-main-experience` → `launcher` | MERGED | `00d2953a8c` |
| [#4](https://github.com/nguyenphuck2468-tech/horizon/pull/4) | `horizon-next/phase-3-content-experience` → `launcher` | MERGED | `cadddef84d` |
| [#5](https://github.com/nguyenphuck2468-tech/horizon/pull/5) | `horizon-next/phase-1-design-system` → `launcher` | MERGED | `ccaac6b6d2` |
| [#6](https://github.com/nguyenphuck2468-tech/horizon/pull/6) | `horizon-next/rebrand-foundation` → `launcher` | MERGED in this pass | `da11574439` |
| [#1](https://github.com/nguyenphuck2468-tech/horizon/pull/1) | `horizon-repo-review-plan-abfc6` → `elyby-on-upstream` | OPEN, `UNSTABLE` | kept for review |

## 3. PR #6 — merge evidence

- Head before merge: `d4b118e32b` (`refactor(facade): give the next shell a stable use-case boundary`).
- 8 commits ahead, 0 behind `launcher` `cadddef84d`.
- 9 files, +2072 / −4:
  - `docs/design/horizon-next/{rebrand-architecture,workflow-map,preservation-matrix,change-log}.md`
  - `launcher/src/{next_navigation,next_preservation,next_facade}.rs`
  - `launcher/src/{lib,account}.rs`
- CI: [run 42574758937](https://github.com/nguyenphuck2468-tech/horizon/actions/runs/42574758937) —
  `parity-scope`, `launcher` and `verify` all succeeded on `d4b118e32b`.
- Mergeability: `MERGEABLE` / `CLEAN`; no reviews required and the branch is not protected.
- Merged with a merge commit, matching the existing `launcher` history:
  `da11574439ad804b58b5280609ba8881ba32fb26` at 2026-11-09T06:56:57Z.
- Post-merge fence on `launcher` `da11574439`:
  - `cargo fmt --check` — clean
  - `cargo clippy --all-targets --locked -- -D warnings` — clean
  - `cargo test --locked --lib` (minus the known path-replacement test) — **581 passed**
  - `cargo test --locked --bin horizon-launcher` — **117 passed**
- N01–N03 files verified present on `origin/launcher`: `next_preservation.rs`, `next_navigation.rs`,
  `next_facade.rs`, `workflow-map.md`, `preservation-matrix.md`, `rebrand-architecture.md`.

The PR was marked ready before merging; it had been a draft checkpoint. Its body still describes the
first checkpoint, so §3 of this report is the authoritative record of what was integrated.

## 4. Branches deleted

Every branch below was proven to be an ancestor of `origin/launcher` **before** deletion
(`git merge-base --is-ancestor origin/<branch> origin/launcher`), so no unique commit was lost.
Deletion used one explicit `git push origin --delete <branch>` per branch; no wildcard.

| Branch | SHA | Proof |
|---|---|---|
| `horizon-next/phase-1-design-system` | `8069fca706` | contained in `launcher`; PR #2/#5 merged |
| `horizon-next/phase-2-main-experience` | `690146ab85` | contained in `launcher`; PR #3 merged |
| `horizon-next/phase-3-content-experience` | `9265eba615` | contained in `launcher`; PR #4 merged |
| `feat/elyby-sign-in-ui` | `80d3ad54e3` | contained in `launcher`; no open PR |
| `feat/game-from-fork` | `0206f2891d` | contained in `launcher`; no open PR |
| `feat/launcher-repo-split` | `9d7f7cf195` | contained in `launcher`; no open PR |
| `fix/windows-ci-secrets-file-tests` | `5c0bed2022` | contained in `launcher`; no open PR |
| `nguyenphuck2468-tech-add-elyby-auth` | `47c88bfe0a` | contained in `launcher`; no open PR |

Local branches deleted after the same check: `horizon-next/phase-1-design-system`,
`horizon-next/phase-2-main-experience`, `horizon-next/phase-3-content-experience`, and the
temporary `integration/merge-phases` (which pointed at the old `launcher` `cadddef84d`).

## 5. Branches kept

| Branch | SHA | Why |
|---|---|---|
| `main` | `d8edfbcd0f` | default branch; not moved, no release cut |
| `launcher` | `da11574439` | Horizon Next integration base |
| `horizon-next/rebrand-foundation` | `d4b118e32b` | merged via PR #6, but is the active Horizon Next working branch; kept until the next phase branch is cut |
| `horizon-next/glass-experience` | `390376aaac` | 8 commits not in `launcher`; needs investigation (§6) |
| `elyby-on-upstream` | `a65baf021d` | base branch of the still-open PR #1 |
| `horizon-repo-review-plan-abfc6` | `b540150684` | head branch of the still-open PR #1 |

Note: `horizon-next/glass-experience` also has one local-only commit, `5b079d048f`
(`test(launcher): pin all pages have name and icon`), which was never pushed. It was not deleted.

## 6. Needs investigation

### PR #1 — stale but open

- Head `horizon-repo-review-plan-abfc6` @ `b540150684` is **already contained in `launcher`**
  (it is the merge commit `b540150684` `merge: players can sign in to Ely.by from the glass interface`).
- Base `elyby-on-upstream` @ `a65baf021d` is also contained in `launcher`.
- The PR has 100 commits / 100 files against that old base, its body is the generic
  "created by qwen-chat coder" text, and CI is `UNSTABLE`.
- No merge was attempted because CI is not green. Closing it is a human decision; deleting its
  base branch would close it automatically, so both branches were kept.

### `horizon-next/glass-experience` — unintegrated work

Unique commits vs `launcher` (8):

```
390376aaac fix(launcher): REPO→upstream, retry 504, Nightly v2 signatures, accounts page stub
ac85610083 fix(launcher): point REPO at upstream; support Nightly v2 signatures
666d0f0a6e config(launcher): point REPO at upstream horizon-rust/horizon for game updates
560382f9f6 fix(launcher): retry fetch_small read_to_end on transient errors
3491df193f docs: define glass experience redesign (C00)
ce2426e86b fix(launcher): retry fetch_small on transient 504 errors
70c942c84c test(launcher): pin play_press and starts_now launch behavior
1ea7876d2a test(launcher): pin play_press and starts_now launch behavior
```

It is 39 commits behind `launcher` and its diff touches `backend.rs`, `signed.rs` and `main.rs`.
Some of that work may overlap fixes already merged through other branches; a rebase onto
`launcher` and a fresh PR are required before any of it can be judged redundant. Do not delete.

## 7. Verification after cleanup

- `git fetch origin --prune` run in `/workspaces/horizon`.
- Final `origin` branches: `main`, `launcher`, `horizon-next/rebrand-foundation`,
  `horizon-next/glass-experience`, `elyby-on-upstream`, `horizon-repo-review-plan-abfc6`
  (plus `origin/HEAD`).
- Open PRs: #1 only. PR #6 is merged.
- Local `launcher` tracks `origin/launcher` at `da11574439`; working tree clean.
- Post-merge fence: fmt/clippy clean, lib **581**, bin **117**.
- Push to `launcher` triggered the `launcher` workflow
  [run 42576202223](https://github.com/nguyenphuck2468-tech/horizon/actions/runs/42576202223)
  (dry run for a non-`main` branch).

## 8. Remaining work

1. Resolve PR #1 (close as superseded, or rebase onto `launcher` if any of its 100 files are still
   wanted). Its head and base are already in `launcher`.
2. Rebase `horizon-next/glass-experience` onto `launcher`, decide which unique commits are still
   needed, and open a normal PR for the survivors.
3. Continue Horizon Next from `launcher` (N04 onward). `rebrand-foundation` can be retired once the
   next phase branch exists.
4. No release or `main` move was made; that still needs an explicit go-ahead.
