# Launcher product and engineering overhaul

Status: baseline and execution plan
Owner: Horizon fork
Scope: the desktop launcher product, not a translation-only pass

This plan covers the launcher shipped from this repository. It does not change the Horizon game client's vanilla-parity contract, and it does not treat localization cleanup as a substitute for product work.

## Current product surface confirmed from source

The launcher is a GPUI Fast desktop application with a separate core shared by the GUI, CLI and MCP entry points. The current source tree includes:

- Home / Play, selected instances, preparation and launch state
- Versions, release selection, install/remove and imported game builds
- Microsoft and Ely.by device-code sign-in, account switching and ownership checks
- Java runtime discovery and Java-instance launch support
- Mojang-pinned game, library and asset downloads with size/SHA-1 verification
- Mods, modpacks, resource packs, shader packs and data packs
- Servers, console, backups, updates and Tailscale-based friend access
- Wardrobe/cosmetics, notes, settings, themes, language and proxy configuration
- Crash summaries, logs, screenshots, agent control, CLI and MCP
- Portable data layout, migration, self-update and desktop integration

Relevant implementation entry points include `launcher/src/main.rs`, `launcher/src/home.rs`, `launcher/src/backend.rs`, and the shared library modules in `launcher/src/lib.rs` (`account`, `browser`, `java`, `java_launch`, `mojang`, `persistence`, `proxy`, `launch`, `instances`, `single_instance`, and others).

## Baseline findings

These are architecture observations, not claims that the features are broken:

1. `launcher/src/main.rs` owns a very large UI state object spanning navigation, accounts, installs, native builds, servers, cosmetics, settings, crashes and background work. This creates a high regression surface when changing unrelated screens.
2. The shared core is a strong foundation: GUI, CLI and MCP use the same backend. Improvements should preserve that single behavior path rather than implementing GUI-only shortcuts.
3. Download paths already include Mojang size/hash validation, cancellation and reuse of complete files. The overhaul should strengthen the common transfer contract (timeouts, retries, resume semantics, progress and disk-space failures) without weakening integrity checks.
4. Account handling already separates metadata from secrets and uses the OS keychain where available. UX improvements must preserve token secrecy, ownership gating and the distinction between Microsoft ownership and Ely.by authentication.
5. The design system defines typography, color contrast, spacing, motion and component rules. A visual redesign should improve hierarchy, responsiveness, discoverability and accessibility while keeping the established Horizon visual identity coherent.
6. The repository already has portable packaging and platform-specific flows. The user-facing installation, upgrade, recovery and uninstall paths need end-to-end verification on each supported OS, not just successful compilation.

## Workstreams and acceptance gates

### A. Product UX and information architecture
- Review every page and modal at narrow, normal and wide window sizes.
- Make the primary path obvious: first run → account / import → choose instance → prepare → Play.
- Reduce duplicate controls and hidden prerequisites; explain blockers next to the action that can resolve them.
- Ensure empty, loading, stale, offline, error and success states are designed for every major page.
- Verify keyboard-only navigation, focus order, shortcuts, readable scaling and reduced-motion behavior.
- Keep destructive actions explicit, reversible where practical, and consistent.

Acceptance: a page-by-page interaction checklist; screenshots at representative sizes; keyboard/focus regression tests for critical flows; no clipped or inaccessible primary actions.

### B. Launcher core and state architecture
- Map UI state transitions for install, prepare, launch, update, import, account refresh and cancellation.
- Identify duplicated state, stale async completions, race windows and work that can outlive the selected instance/page.
- Move cohesive workflows behind typed core services/state machines where this reduces the root view's coordination burden.
- Preserve GUI/CLI/MCP parity and make operations cancellable and observable.
- Add regression tests for concurrent operations, app restart recovery and interrupted workflows.

Acceptance: explicit state-transition tests; no stale task mutating a newly selected instance; cancellation and retry tests; shared behavior across entry points.

### C. Downloads, preparation and storage
- Audit every remote transfer for bounded memory use, connect/read/overall timeouts, retry policy, cancellation, partial-file handling and atomic finalization.
- Verify checksum and expected size before promoting a downloaded file into the usable cache.
- Add free-space checks and actionable diagnostics for permissions, locked files, full disks and corrupt caches.
- Make progress reflect bytes where the source exposes a trustworthy content length; otherwise label file-count progress honestly.
- Test proxy and no-proxy behavior, transient 5xx, rate limiting, offline startup and recovery after forced termination.

Acceptance: deterministic mock-server tests for interruption/retry/corruption; no unverified file is treated as installed; bounded memory for large downloads; actionable error messages.

### D. Accounts and security
- Keep access/refresh tokens out of logs, URLs, command-line arguments, crash reports and exported diagnostics.
- Test Microsoft device-code refresh, expiry, revocation, account switching and ownership changes.
- Test Ely.by as a separate authentication provider; never treat it as Microsoft ownership or Mojang download authorization.
- Audit keychain fallback, file permissions, session-file lifetime, cleanup and multi-process access.
- Review deep links, local callback handling, proxy behavior and untrusted import boundaries.

Acceptance: secret-redaction tests; provider-separation tests; refresh/expiry tests; least-privilege storage; security review of every new import/update path.

### E. Performance and reliability
- Establish repeatable cold-start, warm-start, page-switch, instance-list, release-list, memory and idle-CPU baselines.
- Profile the real launcher build before optimizing; do not infer performance from compile time.
- Move blocking IO and expensive filesystem scans off the UI thread; debounce and cache only with explicit invalidation.
- Add structured diagnostics with redaction and a support bundle that excludes credentials and personal world contents by default.
- Test crash recovery, single-instance behavior, sleep/resume and long-running sessions.

Acceptance: benchmark report on a documented reference machine; no UI-thread network or large filesystem work; crash/recovery tests; no unexplained growth in idle CPU or memory.

### F. Distribution and lifecycle
- Verify clean install, first launch, self-update, rollback/failure recovery, portable mode, migration, desktop integration and uninstall.
- Check signing/notarization status and clearly communicate platform security prompts.
- Test upgrades from the previous published launcher and preservation of instances, worlds, settings and accounts.
- Keep game build artifacts and launcher artifacts distinct and verifiable.

Acceptance: clean-machine E2E matrix for Windows, Linux and macOS; upgrade/migration tests; artifact provenance and checksum verification.

## Execution order

1. Finish the source audit and capture measurable baselines before changing architecture.
2. Fix high-severity correctness/security/reliability defects found by the audit.
3. Improve the first-run and Home/Play journey, then responsive navigation and page-level empty/error states.
4. Harden downloads and preparation as one shared pipeline.
5. Reduce root UI coordination complexity without changing behavior.
6. Add performance benchmarks and optimize measured bottlenecks.
7. Complete install/update/rollback and cross-platform E2E verification.

## Rules for this overhaul

- Work on a dedicated branch; do not push directly to `main`.
- Prefer small, independently reviewable PRs with focused tests.
- Do not merge while required checks are failing or while a platform build is unverified.
- Do not weaken hash verification, ownership checks, keychain handling or secret redaction to make a flow appear easier.
- Do not describe a feature as complete based only on compilation or a mocked UI.
- Translation work is separate maintenance; it is not counted as a launcher product improvement.
