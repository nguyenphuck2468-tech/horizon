# Ely.by sign-in (2026-11-09)

Requested after `launcher@0.2.1`: the published build has no Ely.by button, and a sign-in
could end on a proxy's 504. The button was in the source; the release predates it. This note
records where the button lives, why the release lacked it, and the redo on the ElyHorizon model
(`ElyPrismLauncher/Launcher`, `launcher/minecraft/auth/steps/Ely*` and
`docs/ely-by-account.md` in `sambukuper/elyhorizon-free`).

## Where the button is

- Settings → Account, under **Sign in with Microsoft**: `glass-sign-in-elyby`,
  `launcher/src/glass_account.rs`.
- The top bar while no Microsoft account plays: `topbar-sign-in-elyby`, `launcher/src/frame.rs`.
- The classic account card: `sign-in-elyby`, `launcher/src/main.rs`.

All three now offer the button whether or not `account::ely_configured()` answers true. The
built-in public client ID (`horizon-launcher2`) already makes it true in every build; removing the
gate means a missing button can never be what keeps a player out. A build with no client ID says
why when the button is pressed instead of hiding the way in.

## Why the published build had none

`launcher@0.2.1` was tagged 2026-11-06 16:52 +0700. The glass Ely.by button landed in
`338dc123b5` at 17:54 +0700 that day. The tag is an ancestor of `launcher` but does not contain
the Ely commit; `launcher` and all three phase branches do. A launcher cut from a phase branch
carries the button.

## The redo, following ElyHorizon

`launcher/src/ely.rs`

- **Profile endpoint**: `GET https://account.ely.by/api/mojang/services/minecraft/profile`
  with `Authorization: Bearer <access token>`, the Mojang-shaped endpoint ElyHorizon reads.
  The parser accepts its `id`/`name` and the account record's `uuid`/`username`, so both shapes
  give the same UUID and name.
- **404**: said plainly (`This Ely.by account has no Minecraft profile yet. Create one at
  minecraft.net, then sign in again.`) instead of a generic profile error.
- **Fallback verification page**: `https://account.ely.by/device` (the server's
  `verification_uri` still wins when it sends one).
- **Transient retry**: every Ely.by request now asks up to three times when the answer is 502,
  503 or 504, waiting 250 ms then 500 ms, the same policy as the downloader. This covers the
  device-code request, each token poll, refresh, the authorization-code exchange and the profile
  read. Other statuses and transport errors fail at once, as before; the polling loop still
  treats a persistent 5xx/429/408 as "still waiting" rather than ending the sign-in.

## Verification

- `cargo fmt --check`: clean.
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo test --locked --lib` (skipping the documented environment-only binary-replacement
  test): 561 passed.
- `cargo test --locked --bin horizon-launcher`: 113 passed.
- `ely::tests` (18): the retry predicate and backoff, a loopback server that answers 504, 503,
  then 200 (three requests), a 400 that is not retried, both profile shapes, and the endpoints
  pinned to ElyHorizon's.
