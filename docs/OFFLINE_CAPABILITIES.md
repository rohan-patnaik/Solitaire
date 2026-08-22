# Offline capability catalog

Generated from `docs/offline-capabilities.json`; do not edit by hand.
No parity score is claimed. Statuses describe evidence, not UI presence.
Baseline evidence revision: `df9f4f3cf4b49482f031ea5b890a117a31b93408`.
Baseline CI: https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489.
Pinned application-source CI: `1595fb02488525629c3f7d4aa1962e12c99198c0` at https://github.com/rohan-patnaik/Solitaire/actions/runs/32586839845 (success).

| Status | Count |
| --- | ---: |
| Complete | 0 |
| Partial | 11 |
| Planned | 0 |
| Excluded | 3 |

| ID | Capability | Status | Known limit |
| --- | --- | --- | --- |
| `foundation.omarchy-launcher` | Detached Omarchy launcher | Partial | At exact published SHA 1595fb02488525629c3f7d4aa1962e12c99198c0, the supported live Omarchy install/summon path returned in 70 ms, kept Quickshell responsive, and the detached native process survived an Omarchy shell restart. Live missing-binary and immediate-startup-failure notification evidence remains pending. |
| `foundation.arch-package` | Exact-revision Arch package | Partial | Rust and exact-revision Arch package CI is green at published baseline 1595fb02488525629c3f7d4aa1962e12c99198c0; its installed package identity and binary hash were verified in the real Omarchy acceptance pass. Reproducibility and broader release evidence remain partial. |
| `foundation.persistence-safety` | Bounded atomic local persistence | Partial | Real Wayland save/reopen, corrupt-profile quarantine, and two-process stale-writer recovery succeeded at published SHA 1595fb02488525629c3f7d4aa1962e12c99198c0. Crash injection, cross-file transactions, and power-loss acceptance remain pending. |
| `game.klondike` | Playable Klondike | Partial | Drag/drop, full options, win presentation, and platform acceptance remain open. |
| `game.spider` | Playable Spider | Partial | Broader hostile/property and platform evidence remain open. |
| `game.freecell` | Playable FreeCell | Partial | Solver-grade hints and platform acceptance remain open. |
| `game.pyramid` | Playable Pyramid | Partial | Standard Pyramid is playable with deterministic numbered deals, pair-to-13 and king removal, two redeals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics; solvability metadata, optional variants, real Omarchy, keyboard-only, and assistive-technology acceptance remain pending. |
| `game.tripeaks` | Playable TriPeaks | Partial | Standard mode is playable with deterministic numbered deals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics; real Omarchy, keyboard-only, and assistive-technology acceptance remain pending. |
| `collection.local-profile` | Local profiles, statistics, and achievements | Partial | One anonymous device-local profile exposes per-game deals played and won using idempotent numbered-deal observations, fixed-size u64 counters, bounded atomic persistence, CAS conflict handling, retry/close guards, and corrupt-source quarantine. Real Wayland played/reopen/quarantine/conflict workflows succeeded; a won-deal UI transition, named profiles, achievements, streaks, import/export, sync, multi-process merge, cross-file transactions, power-loss acceptance, and broader statistics remain pending. |
| `quality.accessibility` | Keyboard and assistive-technology acceptance | Partial | Real Wayland Tab plus Space/Enter operation, visible focus, AT-SPI names/actions/states, and changing live-region names were observed. Orca spoken-output acceptance and an unclipped visual surface for long recovery errors remain pending. |
| `quality.real-omarchy` | Real Omarchy and Wayland acceptance | Partial | Exact-SHA installed binary and plugin summon, shell responsiveness, native-process survival across shell restart, keyboard interaction, save/reopen, quarantine, conflict recovery, and clean shutdown were observed on real Omarchy/Wayland. Orca, unclipped long errors, a won-deal UI transition, and live missing-binary/startup-failure notifications remain pending. |
| `excluded.accounts` | Microsoft and Xbox accounts | Excluded | Hosted identity is outside the offline product boundary. |
| `excluded.ads` | Advertising services | Excluded | The product is intentionally offline and ad-free. |
| `excluded.vendor-content` | Microsoft-branded events and copied daily deals | Excluded | Vendor content and expressive presentation will not be copied. |

## Status definitions

- **Complete:** Workflow and required evidence are complete at a green exact revision.
- **Partial:** Some implementation exists, but workflow or acceptance evidence remains open.
- **Planned:** In scope and not implemented.
- **Excluded:** Outside the offline product boundary, with a documented reason.
