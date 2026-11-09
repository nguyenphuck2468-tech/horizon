# Home audit (Phase 2)

Scope: `launcher/src/glass_home.rs` (988 lines), `glass_home_tabs.rs`, `glass_home_lists.rs`,
`glass_home_mods.rs`, `glass_home_packs.rs`, the Home bits of `main.rs:3290-3560`
(`glass_home` page assembly, `crash_notice`), and `glass_palette.rs` (quick actions).
Read on `horizon-next/phase-1-design-system` at `5a74dcf`; every verdict is from the source.

## The charter's Home checklist vs. the glass Home

| # | Item from the charter | What the code has | Verified |
|---|----------------------|-------------------|----------|
| 1 | Play button | `glass_home_head` -> `play_group(tag, prepared \|\| java.is_some(), cx)`; the mockup's `h-14` primary | reading; CLI contract newly pinned by `glass_home::tests::the_play_command_is_the_documented_cli_call` |
| 2 | Selected instance | edition tabs (Rust/Java Edition) -> `switch_edition` -> `edition_pick` (that edition's, played last; New instance for none) | `glass_home::tests::an_edition_tab_goes_to_that_editions_instance_played_last` |
| 3 | Account | top bar sign-in/offline in the glass look; the night look also has the account index rows (`main.rs`) | reading; the `account` tests (110) cover the flows behind the UI |
| 4 | Update state | Play's menu Prepare, preparation `steps` right under the head, `prepared` flag; Horizon's own update = `SelfUpdate` (footer/Settings) | reading |
| 5 | Running state | `played_text` "Playing for N min now" (`played_for`, `client_since`) | `glass_home::tests::played_time_reads_as_the_mockup_writes_it` |
| 6 | Recent activity | worlds/multiplayer rows show `last_played` ("25 min ago", "yesterday", "Oct 31") | `glass_home::tests::last_played_counts_hours_within_a_day_then_days` |
| 7 | Quick actions | worlds hover-Play (`quick_play`), folder menu (drop data packs into a world), Mods `big_icon`, Play's menu (Prepare/Play/Shortcut/Command), the palette (play a named instance, open pages, open the game folder, install/remove mods) | reading + `glass_palette::tests` (new) |
| 8 | Notifications | `crash_notice` (report/copy/logs/Discord/dismiss; look-agnostic), `notice` plates for `prepare_error` and `client_error`, `glass_toast` confirmations, Worlds/Multiplayer `danger_line` | reading |

## What was new this phase (and what was not)

- **No missing feature was found.** The glass Home already implements the checklist in the
  mockup's own language (its `HomeView`, per the module docs). The tabbed panel, its seven
  tabs and the lists reuse `glass_ui` rows, so the Phase A1 audit covers them.
- `5a74dcf` -- palette: the query now matches labels and hints **case-insensitively** (typing
  "SETT" used to miss "Open Settings"); the filter is extracted into the pure
  `palette_filter`, unit-tested (cap at `SHOWN`, order, label/hint match). The CLI play
  command `horizon-launcher play --instance <id>` (docs/launcher-commands.md:526) is pinned by
  a test. No command removed; no behaviour reimplemented.
- The launcher's own tests live in the *bin* target (the lib holds the core), so the CI fence
  `cargo test --lib` never runs them; they were run here (`cargo test --locked --bin
  horizon-launcher`): 95 passed / 0 failed. CI's `clippy --all-targets` still compiles them,
  so a broken test binary fails `checks`.

## Known notes (logged, no change)

- The night-look-only account rows (`main.rs` Microsoft account / Playing offline) are absent
  from the glass look by design: its top bar carries Sign in with Microsoft and says Offline
  (comments at `main.rs:3318-3345`). Covered by the A1 audit's look rules.
- The `vanilla-parity`/`jfr-analysis` CI jobs queue behind self-hosted runners and may stay
  cold in a branch run; the launcher and verify jobs are the authoritative ones here.
