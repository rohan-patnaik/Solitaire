# Solitaire product and implementation plan

## Product boundary

Functional parity is achievable for the five core games and common collection features. The project intentionally uses an original name treatment, navigation, card art, backgrounds, sounds, animation language, copy, progression, challenges, and statistics presentation. Microsoft accounts, Xbox services, ads, branded events, copyrighted level layouts, and copied daily-deal data are out of scope.

## Architecture

- Rust domain crates for cards, deals, move validation, scoring, hints, solver hooks, replay, persistence, and seeded challenges.
- Slint native UI with a lightweight renderer and accessibility semantics.
- Renderer-independent game state so every rule is unit-testable without a window.
- Event-sourced move log for undo, replay, deterministic bug reports, and resumable games.
- SQLite or a compact versioned local store for profiles, settings, statistics, and challenges.
- Original vector card system assembled from geometric suit/rank primitives; no traced reference art.
- Optional sound pack composed specifically for the project and licensed in-repository.

## Milestones

### Current implementation evidence

- `docs/offline-capabilities.json` is the canonical machine-readable status catalog; `docs/OFFLINE_CAPABILITIES.md` is generated and drift-checked. The catalog deliberately reports counts rather than a precise parity score.
- M0 foundation is implemented and covered by CI, exact-revision Arch packaging checks, provenance records, and launcher contract tests. The detached Omarchy launcher's normal, missing-binary, immediate-startup-failure, and shell-restart workflows have exact live evidence and are Complete; packaging and persistence safety retain their separate Partial gaps.
- Klondike has a playable vertical slice. Its new-deal controls expose draw-one,
  draw-three, Standard, and Vegas scoring; both scoring modes currently remain
  untimed with unlimited stock passes. The remaining M1 product workflows below
  are not yet complete.
- Spider and FreeCell now have playable Slint surfaces backed exclusively by their deterministic engines. FreeCell can open any strict decimal `u64` deal number without consuming its durable next-deal sequence; these repository-defined numbers do not claim interoperability with another product's numbering algorithm. Automated tests cover variants, legal move routing, scoring rules, undo/redo, hints, replay save/resume, corrupt replay rejection, adaptive long-column sizing, keyboard activation, and accessibility contracts. Spider's dependency-free hostile-action and fixed seed/mode action-space tests exercise rejection atomicity, conservation, exposure, replay, and history invariants. A pinned normal one-suit replay now reaches a production-reconstructed 7/8 state, and controller coverage proves its final keyboard-routable move, exact 8/8 status, 0600 save, and idempotent 1/1 profile. Installed Spider one-, two-, and four-suit keyboard mutation, undo/redo, save/reopen, and AT-SPI acceptance passed at exact revision `d20ba41`; the exact-package final-transition gate, drag/touch, full keyboard traversal, and spoken screen-reader acceptance remain open.
- TriPeaks standard mode now has a playable Slint surface backed by its deterministic engine, with numbered deals, undo/redo, hints, replay save/resume, bounded recovery, and declared keyboard/accessibility semantics. Real-platform acceptance and broader hostile/property evidence remain open, so the capability stays Partial.
- Pyramid standard mode now has a playable original Slint surface backed by its deterministic engine, with numbered deals, pair-to-13 and king removal, two redeals, undo/redo, hints, replay save/resume, bounded recovery, identity-hidden covered cards, and declared keyboard/accessibility semantics. Real-platform acceptance and broader hostile/property evidence remain open, so the capability stays Partial.
- A bounded anonymous device-local profile now records per-game deals played and won from proven controller lifecycle transitions. Named profiles, achievements, streaks, import/export, sync, broader collection workflows, and release-quality work remain open.
- Real Omarchy/Wayland acceptance at exact published SHA `f6b0cb7e55d296bdf77714efc48a1775b858c041` verified an installed legal mutation and save/reopen in all five games, local-profile recovery and conflict handling, live plugin summon, shell responsiveness, and detached native-process survival across a supported shell restart. A focused exact-package pass at `d20ba41` subsequently accepted all three Spider suit modes and the keyboard/AT-SPI long-status workflow. At `4b31024`, the exact plugin's normal launch plus isolated live missing-binary and immediate-exit-42 notification paths passed, closing only the launcher row. The scopes and open alpha limits are recorded in `docs/ALPHA_RELEASE.md`.

### M0 — foundation

- Native app shell, theme tokens, settings model, CI, packaging skeleton.
- Omarchy manifest/launcher and clear missing-binary diagnostics.
- Card/deck/deal primitives, seeded RNG, replay format, license provenance ledger.

### M1 — Klondike

- Draw-one/draw-three, standard/vegas scoring, timed/untimed, left-handed layout.
- Click, double-click, drag/drop, keyboard play, undo/redo, hints, autocomplete.
- Winning detection, tasteful original completion animation, save/resume.

### M2 — Spider and FreeCell

- Spider one/two/four-suit variants, legal run rules, stock constraints, scoring.
- FreeCell legal supermoves derived from empty cells/cascades, strict
  repository-defined deal-number entry, solver-assisted hints.
- Extensive property tests for move legality and state conservation.
- Playable Slint surfaces with pointer and keyboard pile interaction, variants, scoring display, undo/redo, hints, win/no-move status, and replay-backed save/resume are implemented. Spider one-, two-, and four-suit installed keyboard/save/reopen/AT-SPI acceptance passed; its legal one-suit complete-deal candidate is pinned and locally reconstructed through the final controller lifecycle. Exact installed final-transition acceptance and drag/touch remain open, while broader FreeCell platform evidence is unchanged.

### M3 — Pyramid and TriPeaks

- Pyramid pair-to-13 rules, stock/waste variants, clears, scoring, solvability metadata.
- TriPeaks adjacency/run rules, streak scoring, wild-card variant as an original optional mode.
- Touch-friendly targets without imitating Microsoft layouts.
- Standard-mode TriPeaks is playable through the original Slint layout. Optional wild-card work, touch/platform acceptance, and broader evidence remain open.
- Standard Pyramid is playable through an original seven-row Slint layout. Solvability metadata, optional rule variants, touch/platform acceptance, and broader evidence remain open.

### M4 — collection layer

- Offline daily seeded challenges with a published deterministic generator.
- Curated journeys using original deals and names; difficulty ratings backed by solvers.
- Local achievements, statistics, streaks, profiles, import/export, reduced-motion mode.
- Themes assembled only from repository-owned or audited open assets.
- Foundation implemented: one versioned device-local profile records idempotent per-game played/won counters with bounded atomic persistence, stale-writer detection, corrupt-file quarantine, retry, and source preservation. It does not yet implement named profiles, achievements, streaks, import/export, sync, or cross-file transactional recovery.

### M5 — release quality

- Gamepad and complete keyboard operation, screen-reader labels, high contrast, color-blind modes.
- Responsive animations, 60 Hz frame pacing, startup/memory budgets, battery profiling.
- Reproducible Arch package/AppImage, signed checksums, SBOM, fuzzing of save/replay files.
- Omarchy validation on a real Quattro machine and marketplace submission.
- Exact-SHA five-game alpha acceptance is summarized in `docs/ALPHA_RELEASE.md`; the dedicated wrapped long-status surface passed exact-package keyboard, visual, and AT-SPI acceptance at `d20ba41`, and the detached launcher row passed its remaining live failure acceptance at `4b31024`. Full keyboard traversal, Orca output, complete-deal UI wins, drag/touch, package signing/reproducibility, and marketplace submission remain open.

## Copyright and provenance gate

Every visual/audio asset must have a source, author, license, and creation record in `assets/PROVENANCE.md`. Reference screenshots may guide feature inventory only; implementation must not trace or reproduce expressive presentation. Game rules may be reimplemented, but wording and art are independently authored.

## Definition of done per feature

Each rule variant requires deterministic tests, invalid-move tests, save/load and undo coverage, keyboard/touch behavior, accessibility labels, documentation, and a focused commit pushed after CI passes.
