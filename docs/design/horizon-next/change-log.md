# Horizon Next — change log

## 2026-11-09

- feat(ely): the Ely.by button is always offered (Settings → Account, the top bar, the classic
  account card), and its device-code, token, refresh and profile requests ask again after a
  transient 502/503/504 (250 ms, then 500 ms) instead of ending the sign-in.
- The profile read follows ElyHorizon's model: the Mojang-shaped
  `.../api/mojang/services/minecraft/profile` with the Bearer token, `id`/`name` or
  `uuid`/`username`, and a 404 that says a Minecraft profile is needed.
- The only published release, `launcher@0.2.1`, predates the glass Ely.by commit by an hour;
  a build from any phase branch has the button.
- Fence: fmt/clippy clean; lib 561 + bin 113 green. Evidence: `ely-sign-in.md`.
- refactor(glass): one tested `danger_fill()` helper replaces the three copies of the 12 % red
  (delete-row hover, refused field, second destructive click), and pins its alpha and hue.

## 2026-11-08

- `334fbf746` fix(game): read the game, its builds and the mixin registry from
  `horizon-rust/horizon`; the launcher's own releases stay on the fork (`LAUNCHER_REPO`).
- `b10dca70a` fix(net): retry 502/503/504 twice (250 ms, 500 ms) before failing a download,
  with a loopback test that serves 503, 504 then 200.
- `15e42bd10` fix(nightly): require `SHA256SUMS.v2.sig` signed for the Nightly's own version
  (`nightly-<date>-<commit>`); the channel signature is refused. Same rule in `horizon-server`.
- Same three commits cherry-picked to `horizon-next/phase-2-main-experience` (`0f1c2daf4`,
  `64e4a097f`, `29d6591a4`) and `horizon-next/phase-3-content-experience` (`d50f5705e`,
  `d67677e1a`, `a7d3d7bbd`).
- `cb3f7fd19` refactor(palette): match mod queries in any case and unit-test the extracted
  filter (partial title, nothing matching, list order).
- test(server): the Nightly pick test now serves `SHA256SUMS.v2.sig`, and covers an older
  build under tonight's title, a pre-v2 Nightly, and a title that names no build.
- Fence on every tip: fmt/clippy clean; lib tests green (551 phase 1, 557 phase 2, 554 phase 3;
  the environment-dependent binary-replacement test skipped and failing identically at
  baseline).
- Evidence and file map: `download-fixes.md`.

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
- `412b199fe` test(servers): pin the server-state dot tone the glass mockup writes
  (running+clean → Live, running+crashed → Warn, stopped → Off, else → Busy).
- `5a74dcf22` refactor(palette): extract the query filter into the pure `palette_filter`,
  match queries case-insensitively, unit-test the cap/order/matching, and pin the documented
  CLI play command (`--instance`). Phase 2 (Home) audited: no missing feature
  (`glass-home-audit.md`).
- `e1807894e` test(versions): pin the release-day formatting (`Nov 1`, `Mar 9, 14:30 UTC`)
  shared by Home and Versions.
- `23f57c00e` refactor(mods): extract the search placeholder and unit-test its every branch.
- Phase 0 closed with the outcome of run 42375384481 (p0-summary.md 8.3): `verify` green at
  baseline, `launcher` failing in the fence step exactly as forecast; fixes already on the
  horizon-next branch.
- `c78ee124b` docs(horizon-next): phase A1 glass-ui audit + change log + baseline forecast.
- `613843fae` fix(elyby): drop the unused `symbol()` wrapper (dead_code the CI fence refuses).
- `c05e82203` refactor(glass): route the sign-in ink through the design token and unit-test
  the appearance helpers.
- `e09ef7c32` fix(clippy): drop the needless borrow in the release-loading test binding.
- Environment: the codespace moved from basicLinux32gb (2 cores / 8 GB) to standardLinux32gb
  (4 cores / 16 GB) with the user's go-ahead; bin-target tests became practical
  (95+ passed / 0 failed).
- `6907b3759` test(toast): pin the tilde home-folder shortening (`/home/player/a/b` -> `~/a/b`).
- `11cad3af5` refactor(java): extract the loader note and unit-test its five branches
  (fabric/quilt/neoforge/forge/other).
- `e32535bfe` refactor(account): extract the player status tone/line and unit-test its branches
  (Microsoft -> Live/online servers; Offline/GameName -> Offline/single player).
- `b4412c2de` refactor(instance): extract the instance dialog's primary button label and unit-test it (Save changes / Copying… / Create instance).
- `a432bbd04` refactor(mods): extract the project support tone/reason mapping and unit-test it (Works/Partial/No).
- `8af6be187` refactor(home): extract the multiplayer server reach status and unit-test its rows (Online/Offline/Asking).
- `b4412c2de` is the 13th independent commit on the branch; every commit carries its own fence run.
- `afd8e54d9` refactor(home): extract the packs count note and unit-test it (1 pack / N packs).
- `068105e03` refactor(home): extract the mods count note and unit-test it (1 mod / N mods).
