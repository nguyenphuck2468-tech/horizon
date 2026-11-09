# Next-generation launcher — technical architecture

Status: approved product direction; implementation charter
Date: 2026-11-09
Base: `launcher` at `cadddef84deecd4aba33af1b2af50c694dc7f538` (Phase 1-3 merged)
Working branch: `horizon-next/rebrand-foundation`

## 1. Product decision

Build a genuinely new premium desktop launcher experience with a new brand identity. This is a fresh product/UI architecture, not a rewrite of working game, account, download, server, or content engines.

The public brand name, logo, domain, and package identifiers are deliberately **TBD**. Until the owner approves a name, use the internal codename "Next" in code and documentation. Do not rename published package IDs, executable names, OAuth redirect identifiers, update channels, data directories, or signing identities as part of visual rebranding.

## 2. Non-negotiable compatibility contract

The launcher must retain all existing user-visible capabilities and stored data. No feature may silently disappear because it is not represented in the new navigation.

Preserve and regression-test at minimum:

1. Microsoft, Ely.by, and offline identities; provider identity, ownership semantics, token refresh, reauthentication, logout, and secret storage.
2. Horizon release channels, stable/nightly selection, metadata resolution, signature verification, checksum verification, launcher self-update, and rollback safety.
3. Minecraft version discovery, installation, preparation, repair, launch, process tracking, logs, crash reporting, and Java runtime selection.
4. Java instances, Horizon instances, instance isolation, create/clone/import/export, backups, restore, folder access, and migration.
5. Modrinth, CurseForge, Horizon registry, mods, modpacks, resource packs, shader packs, datapacks, loaders, native mods, dependencies, compatibility checks, and installed-content management.
6. Worlds, screenshots, skins, capes, wardrobe, heads, multiplayer, server lists, hosted servers, console, Tailscale, agents, and diagnostics.
7. Global and per-instance settings, themes, accent, language/i18n, shortcuts, accessibility, CLI, MCP, telemetry consent, and platform-specific installer/updater behavior.

Compatibility rules:
- Existing data directories and file formats remain readable and writable.
- Existing CLI commands and documented automation interfaces remain stable unless a separately reviewed migration is provided.
- Existing OAuth clients, scopes, callback/device flows, release keys, signing public keys, update URLs, and channel identifiers are not changed by rebranding.
- Ely.by is never represented as Microsoft; its ownership flag remains separate from its ability to launch. Offline accounts are not promoted to authenticated identities.
- UI view models contain safe account summaries only; access/refresh tokens and secrets never enter UI state, logs, analytics, or crash reports.
- A new UI must call existing application/domain services. It must not reimplement HTTP, OAuth, filesystem mutation, signing, or launch logic inside view code.
- A feature is not considered migrated until its entry point, primary workflow, error states, and regression tests are accounted for in the preservation matrix.

## 3. Architecture

Keep Rust and GPUI Fast for this generation. The existing `launcher/` package remains the desktop executable and CLI compatibility boundary. The new presentation layer is introduced incrementally beside the existing one, then becomes the default only after parity gates pass.

Target layers:

- **Presentation** — pages, reusable visual components, navigation, animations, accessibility, and responsive layouts. No network or filesystem business logic.
- **Application** — explicit use cases and orchestration: sign in, create instance, install content, prepare, launch, backup, update, host server, etc.
- **Domain** — account, instance, version, content, Java, server, update, and diagnostics models plus invariants.
- **Infrastructure** — HTTP clients, provider adapters, filesystem, keychain, process management, archive handling, and platform integration.
- **Compatibility facade** — stable calls into the existing `horizon_launcher` library and CLI contracts while domains are extracted behind tests.

Suggested source organization (introduce only when each module has real callers):

```
launcher/src/
  app/                 # application bootstrap, app state, lifecycle
  navigation/          # route IDs, metadata, transitions, deep links
  design/              # tokens, typography, icons, surfaces, motion
  components/           # buttons, fields, menus, dialogs, tables, status
  pages/
    home/
    instances/
    accounts/
    content/
    servers/
    multiplayer/
    worlds/
    appearance/
    diagnostics/
    settings/
  view_models/          # presentation-safe state; no secrets
  application/          # use cases and task orchestration
  compatibility/        # stable facade over existing launcher services
```

Do not move every existing file in one mass refactor. Establish boundaries with small commits and tests. Existing domain modules remain the source of truth until a separately tested extraction is justified.

## 4. Premium visual and interaction direction

The visual identity must be new rather than a cosmetic reskin of the existing Glass or Blocks look.

Design goals:
- Editorial premium gaming feel: cinematic but restrained, strong typography, deliberate hierarchy, rich artwork used as context rather than decoration.
- Dark appearance as the initial default, with light/system options retained.
- One clear primary action per page; clear focus, selection, loading, success, warning, and error states.
- Motion is short, purposeful, interruptible, and respects reduced-motion settings.
- Responsive layouts for narrow windows and high-DPI displays; no clipped essential controls.
- Keyboard-first navigation, visible focus, screen-reader labels where supported, adequate contrast (WCAG AA target), and no color-only status encoding.
- No fake metrics, fake online status, or placeholder content presented as live data.
- Keep the interface efficient on low-memory systems: virtualize long lists, lazy-load artwork, bound image caches, and avoid continuous background animation.

Information architecture:
- **Home** — selected launch profile, Play, readiness/update state, recent activity, and contextual quick actions.
- **Instances** — searchable library, filters, create/import/clone, per-instance settings, repair, backup, and launch.
- **Accounts** — provider-separated account center, status, add/switch/remove, reauthentication, and security explanation.
- **Content** — one discover/manage surface for mods, modpacks, resource packs, shaders, datapacks, worlds, and supported native content; source and compatibility always visible.
- **Servers & Multiplayer** — favorites, recent servers, hosted server controls, console, network tools, and player status.
- **Worlds & Media** — worlds, screenshots, skins, capes, and wardrobe.
- **Diagnostics** — downloads/tasks, logs, crash reports, repair tools, and support bundles.
- **Settings** — global settings with explicit per-instance overrides; appearance, accounts, Java/memory, downloads, network, updates, privacy, language, shortcuts, accessibility, advanced.

Navigation must be metadata-driven and permission/feature aware. A capability may have one canonical management location and contextual shortcuts elsewhere; do not duplicate competing settings.

## 5. Task and download model

Use one observable task model for downloads, installs, imports, exports, backups, updates, and server operations. Tasks need stable IDs, typed stages, progress where measurable, cancellation semantics, retry policy, and user-readable failure reasons.

Download invariants:
- Retry only transient failures with bounded exponential backoff and jitter.
- Resume only when server and local metadata make it safe; otherwise restart cleanly.
- Verify expected hashes and signed metadata before atomic installation.
- Never weaken signature or version-binding checks to improve availability.
- Keep rate-limit responses distinguishable from proxy/server 5xx failures.
- Quarantine invalid artifacts; never overwrite a known-good install with an unverified file.
- Report indeterminate progress honestly rather than inventing percentages.

## 6. State, persistence, and migration

- Keep the current on-disk schema as the compatibility baseline.
- Introduce versioned migrations only for genuinely new state; migrations must be idempotent, transactional where possible, and backed up before destructive transformations.
- Persist preferences separately from secrets. Use OS keychain where supported and retain the currently reviewed encrypted fallback behavior.
- Do not change app data directory or package/bundle identifiers until a migration and rollback plan exists for Windows, Linux, and macOS.
- New UI state is ephemeral unless the user explicitly expects it to persist.

## 7. Verification gates

Every implementation PR must:
1. pass `cargo fmt --check`;
2. pass `cargo clippy --all-targets --locked -- -D warnings`;
3. pass the applicable launcher library and binary tests;
4. add tests for changed state transitions and failure paths;
5. update the feature preservation matrix;
6. verify that no secrets appear in logs or UI models;
7. include screenshots or a reproducible UI test for visible changes;
8. run platform CI for affected targets.

The preservation matrix is a living release gate, not a one-time checklist. Feature parity is measured by workflows and edge cases, not merely by the presence of a navigation item.

## 8. Commit-by-commit plan

Each item is a small independently reviewable commit. Reorder only when a dependency requires it.

### Foundation
- N00 — `docs: define next-generation launcher architecture` (this document).
- N01 — `test: establish launcher feature preservation baseline`: convert the existing feature inventory into executable smoke/regression coverage and mark manual-only paths.
- N02 — `docs: map current launcher workflows to application domains`: record current entry points, services, persistence, and tests.
- N03 — `refactor: add compatibility facade for launcher use cases`: no behavior change; tests pin current contracts.

### New product shell and design
- N04 — `feat(ui): add next-generation app shell behind feature flag`.
- N05 — `feat(ui): introduce rebrand design tokens and typography`.
- N06 — `feat(ui): add accessible premium component primitives`.
- N07 — `feat(ui): add metadata-driven navigation and command routing`.
- N08 — `test(ui): add navigation and visual regression fixtures`.

### Core experience
- N09 — `feat(ui): build new home and launch readiness experience`.
- N10 — `feat(ui): build instance library and instance detail workspace`.
- N11 — `feat(ui): build account center with provider-safe summaries`.
- N12 — `feat(ui): build unified task and download center`.
- N13 — `test: pin launch and account compatibility across new shell`.

### Content and platform
- N14 — `feat(ui): build content center over existing providers`.
- N15 — `feat(ui): add dependency and compatibility presentation`.
- N16 — `feat(ui): build servers and multiplayer workspace`.
- N17 — `feat(ui): build worlds media and cosmetics workspace`.
- N18 — `feat(ui): add diagnostics repair and support workspace`.
- N19 — `feat(ui): reorganize global and instance settings`.

### Hardening and release
- N20 — `perf(ui): bound caches and virtualize large collections`.
- N21 — `test: add end-to-end feature parity scenarios`.
- N22 — `build: verify Windows Linux and macOS distribution`.
- N23 — `feat(update): add staged rollout and safe rollback for launcher updates` (only if current updater audit proves a gap).
- N24 — `release: enable next-generation shell after parity sign-off`.

## 9. Git and rollout policy

- This work is isolated on `horizon-next/rebrand-foundation`; `launcher` is the frozen compatibility base until the owner explicitly approves integration.
- The three Phase 1-3 PRs are merged into `launcher`; this branch is rebased onto that tip (`cadddef84d`) instead of remaining frozen at `5106b8902`. Rebase onto current `launcher` before each review checkpoint rather than cherry-picking individual commits, so the Ely.by, config, backend, signed and release-sign work stays intact.
- Do not reuse the existing `horizon-next/glass-experience` branch name for the new implementation. It is an earlier design experiment and remains available for selective reference.
- Keep every commit narrowly scoped. Push and open a draft PR at a meaningful review checkpoint, not after every trivial edit.
- Keep the old shell available behind a rollback/diagnostic switch until the new shell passes the preservation gate on all supported platforms.
- Brand name, logo, domain, package identifiers, and public release naming require a separate explicit brand decision; no irreversible rename is implied by this charter.
