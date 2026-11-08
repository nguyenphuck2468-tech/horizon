# Nightly download fixes (2026-11-08)

Reported after comparing the phase branches against `main`: the three phase branches still read
the game from the fork, did not retry a transient HTTP 504, and did not read
`SHA256SUMS.v2.sig`. Each gap was fixed in its own commit, then cherry-picked unchanged to the
other two phase branches.

## 1. The game comes from upstream

`launcher/src/backend.rs`

- `REPO` is `horizon-rust/horizon` (was `nguyenphuck2468-tech/horizon`). The game's releases,
  the mixin registry, `links.json`, the download URLs and the mirror check all follow it.
- `LAUNCHER_REPO` stays `nguyenphuck2468-tech/horizon`: the launcher's own releases are still
  published there, and the mirror only serves Horizon's (now upstream's) repository reads.

## 2. 502/503/504 are retried

`launcher/src/backend.rs`

- `get()` now asks up to three times (`GET_ATTEMPTS`): a 502, 503 or 504 is retried after
  250 ms, then 500 ms (`retry_delay`). Other statuses and transport errors fail immediately,
  and GitHub's rate-limit message still wins over the generic status text.
- `get_once()` is the single ask, boxed to keep the error small.
- Tests: `a_transient_status_is_asked_again` runs a loopback server that answers 503, 504,
  then 200 and asserts the body arrives; `only_a_briefly_unavailable_status_is_asked_again`
  pins the status set and the backoff.

## 3. Nightlies are verified against their own version

`crates/horizon-release-sign/src/lib.rs`, `launcher/src/signed.rs`,
`launcher/src/backend.rs`, `crates/horizon-server/src/update.rs`

- New `SIGNATURE_V2 = "SHA256SUMS.v2.sig"`; new `nightly_title_version` (strict title parse)
  and `is_nightly_build` (`nightly-<date>-<commit>`).
- `horizon_release_sign::nightly_digest` now takes the version and requires the v2 signature
  over `horizon release <version>`, so a channel signature, another Nightly's signature or a
  release's signature is refused, and an older Nightly cannot be served as tonight's.
- `signed::nightly_digest` fetches `SHA256SUMS` and `SHA256SUMS.v2.sig` from the Nightly's
  GitHub tag and verifies against the version it installs as. `backend::install` passes both.
- `horizon-server` does the same: the title must parse strictly, and the v2 signature is
  required.
- Tests: the crate's `nightly_and_release_signatures_never_pass_for_each_other`,
  `an_older_nightly_is_not_tonights`, `a_nightlys_title_is_parsed_strictly`; the launcher's
  rewritten `the_release_channel_refuses_nightly_signatures` (v2 served, channel-only
  refused, other version refused).

## Verification (codespace)

- `cargo fmt --check` clean; `cargo clippy --all-targets --locked -- -D warnings` clean.
- `cargo test --locked --lib`: 551 passed / 0 failed / 4 ignored on phase 1, with the
  environment-dependent `a_replaced_binary_is_replaced_again_at_its_path` skipped. That test
  fails identically at the unmodified baseline in this codespace because its filesystem
  creates files with mode 0646 (umask handling), so it is recorded, not changed here.
- `cargo test -p horizon-release-sign`: 11 unit + 4 integration tests pass.
- `cargo test -p horizon-server --all-targets`: 134 pass, clippy clean.
- Bin-target palette tests: 7 pass (`cargo test --bin horizon-launcher glass_palette`).

GitHub CI (`checks`) on the three pull requests, after the server test was updated to serve
`SHA256SUMS.v2.sig`:

| Pull request | Run | `launcher` | `verify` |
| --- | --- | --- | --- |
| #5 phase 1 | 42496887060 | pass | pass |
| #3 phase 2 | 42496930892 | pass | pass |
| #4 phase 3 | 42496931408 | pass | pass |

Branch tips with the three fixes:

| Branch | Tip |
| --- | --- |
| `horizon-next/phase-1-design-system` | `cb3f7fd19` (plus the palette test) |
| `horizon-next/phase-2-main-experience` | `29d6591a4` |
| `horizon-next/phase-3-content-experience` | `a7d3d7bbd` |

`launcher` itself was not touched: it stays frozen at its audit baseline, and these commits
reach it when the phase branches merge.
