# Offline capability catalog

Generated from `docs/offline-capabilities.json`; do not edit by hand.
No parity score is claimed. Statuses describe evidence, not UI presence.
Baseline evidence revision: `df9f4f3cf4b49482f031ea5b890a117a31b93408`.
Baseline CI: https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489.
Pinned application-source CI: `f6b0cb7e55d296bdf77714efc48a1775b858c041` at https://github.com/rohan-patnaik/Solitaire/actions/runs/32591449130 (success).

| Status | Count |
| --- | ---: |
| Complete | 0 |
| Partial | 11 |
| Planned | 0 |
| Excluded | 3 |

| ID | Capability | Status | Known limit |
| --- | --- | --- | --- |
| `foundation.omarchy-launcher` | Detached Omarchy launcher | Partial | At exact published SHA f6b0cb7e55d296bdf77714efc48a1775b858c041, the supported live Omarchy install/summon path kept Quickshell responsive and the detached native process survived an Omarchy shell restart. Live missing-binary and immediate-startup-failure notification evidence remains pending. |
| `foundation.arch-package` | Exact-revision Arch package | Partial | Rust and exact-revision Arch package CI is green at published alpha SHA f6b0cb7e55d296bdf77714efc48a1775b858c041; the installed source marker, package files, binary hash, and runtime linkage were verified. The evidence package is unsigned; reproducibility, signed checksums, SBOM, and release-upgrade evidence remain open. |
| `foundation.persistence-safety` | Bounded atomic local persistence | Partial | Real Wayland save/reopen in all five games, corrupt-profile quarantine, dirty close guarding, and two-process stale-writer recovery succeeded at published alpha SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Crash injection, cross-file transactions, and power-loss acceptance remain pending. |
| `game.klondike` | Playable Klondike | Partial | An installed keyboard stock draw, undo/redo, and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Drag/drop, full options, a complete-deal win presentation, and broader platform evidence remain open. |
| `game.spider` | Playable Spider | Partial | An installed keyboard stock deal and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. A complete-deal win, broader hostile/property coverage, drag/touch, and variant-wide platform evidence remain open. |
| `game.freecell` | Playable FreeCell | Partial | An installed deterministic hint, illegal move, legal keyboard move, and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Solver-grade hints, a complete-deal win, drag/touch, and broader platform evidence remain open. |
| `game.pyramid` | Playable Pyramid | Partial | Standard Pyramid is playable with deterministic numbered deals, pair-to-13 and king removal, two redeals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics. An installed keyboard stock draw and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041; a complete-deal win, solvability metadata, optional variants, drag/touch, and spoken assistive-technology acceptance remain pending. |
| `game.tripeaks` | Playable TriPeaks | Partial | Standard mode is playable with deterministic numbered deals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics. An installed keyboard stock draw and save/reopen were accepted at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041; a complete-deal win, optional modes, drag/touch, and spoken assistive-technology acceptance remain pending. |
| `collection.local-profile` | Local profiles, statistics, and achievements | Partial | One anonymous device-local profile exposes per-game deals played and won using idempotent numbered-deal observations, fixed-size u64 counters, bounded atomic persistence, CAS conflict handling, retry/close guards, and corrupt-source quarantine. Real Wayland played/reopen in all five games plus quarantine/conflict workflows succeeded; an installed won-deal UI transition in each game, named profiles, achievements, streaks, import/export, sync, multi-process merge, cross-file transactions, power-loss acceptance, and broader statistics remain pending. |
| `quality.accessibility` | Keyboard and assistive-technology acceptance | Partial | Real Wayland keyboard navigation/activation, visible focus, AT-SPI names/actions/states across all five games, and changing live-region names were observed at exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041. Full keyboard-only traversal, Orca spoken-output acceptance, and an unclipped visual surface for long recovery errors remain pending. |
| `quality.real-omarchy` | Real Omarchy and Wayland acceptance | Partial | At exact SHA f6b0cb7e55d296bdf77714efc48a1775b858c041, installed legal mutation/save/reopen in all five games, plugin summon, shell responsiveness, native-process survival across shell restart, quarantine, conflict recovery, and clean shutdown were observed on real Omarchy/Wayland. Orca, unclipped long errors, complete-deal UI wins, drag/touch, and live missing-binary/startup-failure notifications remain pending. |
| `excluded.accounts` | Microsoft and Xbox accounts | Excluded | Hosted identity is outside the offline product boundary. |
| `excluded.ads` | Advertising services | Excluded | The product is intentionally offline and ad-free. |
| `excluded.vendor-content` | Microsoft-branded events and copied daily deals | Excluded | Vendor content and expressive presentation will not be copied. |

## Status definitions

- **Complete:** Workflow and required evidence are complete at a green exact revision.
- **Partial:** Some implementation exists, but workflow or acceptance evidence remains open.
- **Planned:** In scope and not implemented.
- **Excluded:** Outside the offline product boundary, with a documented reason.
