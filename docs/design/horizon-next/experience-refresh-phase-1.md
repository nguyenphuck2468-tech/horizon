# Horizon Launcher — experience refresh, Phase 1

## Intent

Refresh the launcher so it feels calm, current and immediate without sacrificing the feature depth already present. This is an incremental redesign of the existing GPUI application, not a second frontend and not a replacement of working account, instance, download or launch services.

## Existing foundations to preserve

- The launcher already has two visual systems (Night and Glass), shared design tokens in `crates/horizon-theme`, a command palette, localized UI, and canonical route metadata.
- Night already follows reduced-motion preferences and limits background animation while unfocused.
- Glass already caches rendered backdrop pieces and screenshot privacy copies.
- The existing feature inventory and route catalogue are compatibility contracts. Do not remove or silently hide capabilities to make the UI look simpler.

## Direction

1. **Make the primary task obvious.** Home should answer: which instance is selected, is it ready, and what happens when Play is pressed? Keep one dominant action and move secondary actions into predictable contextual locations.
2. **Make navigation legible.** Use a compact workspace rail with clear labels/tooltips, a searchable instance list, and a single canonical destination per feature. Keep Ctrl/⌘K as the fast path.
3. **Use progressive disclosure.** Show the common controls first; expose advanced Java, memory, JVM arguments, native build and diagnostic settings only when requested.
4. **Improve information density without crowding.** Consistent rows, readable status labels, clear empty/loading/error states, and responsive layouts at narrow widths. Never communicate state with colour alone.
5. **Keep the visual language restrained.** Geist typography, neutral surfaces, one accent, subtle separators, deliberate spacing. Glass is an appearance, not a reason to blur every surface.
6. **Respect accessibility.** Keyboard navigation, visible focus, WCAG AA text contrast, reduced motion on every supported OS, scalable text and useful accessible labels.
7. **Keep it fast on modest hardware.** Prefer static surfaces and cached assets. Avoid continuous animation when nothing meaningful changes, repeated full-page rebuilds, expensive blur work on interaction, and unnecessary network requests.

## Performance work before visual expansion

- Profile startup, first interactive frame, idle CPU, memory and navigation latency on a low-end Windows machine and one reference machine.
- Add an explicit user-facing motion preference (System / Reduced / Full) and persist it. System must honour platform accessibility settings where available; Reduced must stop decorative scene motion and non-essential transitions.
- Make the animated scene stop scheduling frames when static, hidden, minimized or unfocused; preserve the existing bounded unfocused grace period only if measurements justify it.
- Audit Glass backdrop invalidation and cache churn. Keep the Night appearance as the low-cost fallback; do not remove Glass.
- Virtualize or paginate long instance/content lists only where measurements show a real cost. Keep keyboard and screen-reader semantics intact.
- Add reproducible before/after measurements; do not label a build “lighter” without measurements.

## Delivery phases

### Phase 1 — navigation and home
- Refine Home hierarchy, instance switcher, Play/Prepare/Update states and the first-run empty state.
- Consolidate duplicate entry points using the existing route catalogue.
- Add responsive breakpoints and keyboard-focus regression tests.

### Phase 2 — instance and content management
- Unify instance details, versions, mods and modpacks around consistent list/detail patterns.
- Make install, download, import and update progress visible without blocking the UI.
- Add safe cancellation/retry and explicit failure recovery.

### Phase 3 — settings and diagnostics
- Group settings by user intent; keep advanced controls discoverable but out of the way.
- Add a useful download queue, crash summary and copyable diagnostics.
- Preserve all account types, including Ely.by, and keep sensitive tokens out of UI logs.

### Phase 4 — performance and polish
- Compare measured startup/idle/interaction performance against Phase 0.
- Tune image decoding, list rendering, blur and animation budgets.
- Run localization layout checks across all shipped locales.

## Acceptance criteria

- No feature-area regression against `PRESERVED_FEATURE_AREAS`.
- Existing account, launch, import, update, download and native-build tests remain green.
- All primary actions work by mouse and keyboard; focus is visible.
- Reduced motion is effective and persistent, not an environment-variable-only workaround.
- No decorative animation causes continuous work while the launcher is idle.
- Before/after performance numbers include machine, OS, build, measurement method and sample count.
- All locale keys/placeholders remain valid and layouts do not clip at common window sizes.

## Non-goals

- No webview rewrite, heavy UI framework, remote analytics, ad/donation surfaces, or new mandatory network dependency.
- No deletion of the existing Night/Glass foundations before a measured replacement is ready.
- No claims of parity or performance improvement based on screenshots alone.
