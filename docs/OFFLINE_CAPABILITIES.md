# Offline capability catalog

Generated from `docs/offline-capabilities.json`; do not edit by hand.
No parity score is claimed. Statuses describe evidence, not UI presence.
Baseline evidence revision: `df9f4f3cf4b49482f031ea5b890a117a31b93408`.
Baseline CI: https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489.
Pinned application-source CI: `4b31024426b73fafe93597e4cd42312eef2b26b0` at https://github.com/rohan-patnaik/Solitaire/actions/runs/32645102863 (success).

| Status | Count |
| --- | ---: |
| Complete | 1 |
| Partial | 10 |
| Planned | 0 |
| Excluded | 3 |

| ID | Capability | Status | Known limit |
| --- | --- | --- | --- |
| `foundation.omarchy-launcher` | Detached Omarchy launcher | Complete | The supported live summon kept Quickshell responsive and the detached native process survived an Omarchy shell restart at exact published SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. At exact published SHA 4b31024426b73fafe93597e4cd42312eef2b26b0, the normal native Wayland launch and isolated live missing-binary and immediate-exit-42 branches delivered their exact critical notifications, diagnostics, and statuses without altering the installed package or leaving a process/window behind. |
| `foundation.arch-package` | Exact-revision Arch package | Partial | Rust and exact-revision Arch package CI is green at published application SHA d20ba4111deb2e948e593fbeec4ca2c45b597bef; the installed source marker, nine package files, and binary hash were verified with no altered files. The evidence package is unsigned; reproducibility, signed checksums, SBOM, and release-upgrade evidence remain open. |
| `foundation.persistence-safety` | Bounded atomic local persistence | Partial | Real Wayland save/reopen in all five games, corrupt-profile quarantine, dirty close guarding, and two-process stale-writer recovery succeeded at published alpha SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Crash injection, cross-file transactions, and power-loss acceptance remain pending. |
| `game.klondike` | Playable Klondike | Partial | An installed keyboard stock draw, undo/redo, and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Drag/drop, full options, a complete-deal win presentation, and broader platform evidence remain open. |
| `game.spider` | Playable Spider | Partial | At exact installed SHA d20ba4111deb2e948e593fbeec4ca2c45b597bef, keyboard selection, stock deal, undo/redo, save/reopen, and AT-SPI identity/action semantics passed in one-, two-, and four-suit modes. The two-suit pass also accepted a legal cross-suit build and atomic rejection of a mixed-suit run. A complete-deal win, broader hostile/property coverage, and drag/touch remain open. |
| `game.freecell` | Playable FreeCell | Partial | An installed deterministic hint, illegal move, legal keyboard move, and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Solver-grade hints, a complete-deal win, drag/touch, and broader platform evidence remain open. |
| `game.pyramid` | Playable Pyramid | Partial | Standard Pyramid is playable with deterministic numbered deals, pair-to-13 and king removal, two redeals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics. An installed keyboard stock draw and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041; a complete-deal win, solvability metadata, optional variants, drag/touch, and spoken assistive-technology acceptance remain pending. |
| `game.tripeaks` | Playable TriPeaks | Partial | Standard mode is playable with deterministic numbered deals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics. An installed keyboard stock draw and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041; a complete-deal win, optional modes, drag/touch, and spoken assistive-technology acceptance remain pending. |
| `collection.local-profile` | Local profiles, statistics, and achievements | Partial | One anonymous device-local profile exposes per-game deals played and won using idempotent numbered-deal observations, fixed-size u64 counters, bounded atomic persistence, CAS conflict handling, retry/close guards, and corrupt-source quarantine. Real Wayland played/reopen in all five games plus quarantine/conflict workflows succeeded; an installed won-deal UI transition in each game, named profiles, achievements, streaks, import/export, sync, multi-process merge, cross-file transactions, power-loss acceptance, and broader statistics remain pending. |
| `quality.accessibility` | Keyboard and assistive-technology acceptance | Partial | Real Wayland keyboard navigation/activation, visible focus, AT-SPI names/actions/states across all five games, and changing live-region names were observed at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. At exact installed SHA d20ba4111deb2e948e593fbeec4ca2c45b597bef, all three Spider modes and the complete wrapped long-status keyboard/AT-SPI workflow passed; full keyboard-only traversal and Orca spoken output remain pending. |
| `quality.real-omarchy` | Real Omarchy and Wayland acceptance | Partial | At exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041, installed legal mutation/save/reopen in all five games, plugin summon, shell responsiveness, native-process survival across shell restart, quarantine, conflict recovery, and clean shutdown were observed on real Omarchy/Wayland. At exact installed SHA d20ba4111deb2e948e593fbeec4ca2c45b597bef, one-, two-, and four-suit Spider workflows and the wrapped long-status surface passed focused Wayland acceptance. At exact SHA 4b31024426b73fafe93597e4cd42312eef2b26b0, normal and both launcher-failure paths passed live acceptance. Orca, complete-deal UI wins, and drag/touch remain pending. |
| `excluded.accounts` | Microsoft and Xbox accounts | Excluded | Hosted identity is outside the offline product boundary. |
| `excluded.ads` | Advertising services | Excluded | The product is intentionally offline and ad-free. |
| `excluded.vendor-content` | Microsoft-branded events and copied daily deals | Excluded | Vendor content and expressive presentation will not be copied. |

## Status definitions

- **Complete:** Workflow and required evidence are complete at a green exact revision.
- **Partial:** Some implementation exists, but workflow or acceptance evidence remains open.
- **Planned:** In scope and not implemented.
- **Excluded:** Outside the offline product boundary, with a documented reason.
