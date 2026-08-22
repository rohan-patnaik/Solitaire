# Offline capability catalog

Generated from `docs/offline-capabilities.json`; do not edit by hand.
No parity score is claimed. Statuses describe evidence, not UI presence.
Baseline evidence revision: `df9f4f3cf4b49482f031ea5b890a117a31b93408`.
Baseline CI: https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489.
Pinned application-source CI: `447fa319cc05bcb509735d9d124a4daa5f35ffcb` at https://github.com/rohan-patnaik/Solitaire/actions/runs/32579498934 (success).

| Status | Count |
| --- | ---: |
| Complete | 0 |
| Partial | 9 |
| Planned | 2 |
| Excluded | 3 |

| ID | Capability | Status | Known limit |
| --- | --- | --- | --- |
| `foundation.omarchy-launcher` | Detached Omarchy launcher | Partial | Real Omarchy acceptance is pending. |
| `foundation.arch-package` | Exact-revision Arch package | Partial | Rust and exact-revision Arch package CI is green at published baseline 447fa319cc05bcb509735d9d124a4daa5f35ffcb; the current Pyramid slice requires its own exact-tip run after publication. |
| `foundation.persistence-safety` | Bounded atomic local persistence | Partial | Real-filesystem and power-loss acceptance remain pending. |
| `game.klondike` | Playable Klondike | Partial | Drag/drop, full options, win presentation, and platform acceptance remain open. |
| `game.spider` | Playable Spider | Partial | Broader hostile/property and platform evidence remain open. |
| `game.freecell` | Playable FreeCell | Partial | Solver-grade hints and platform acceptance remain open. |
| `game.pyramid` | Playable Pyramid | Partial | Standard Pyramid is playable with deterministic numbered deals, pair-to-13 and king removal, two redeals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics; solvability metadata, optional variants, real Omarchy, keyboard-only, and assistive-technology acceptance remain pending. |
| `game.tripeaks` | Playable TriPeaks | Partial | Standard mode is playable with deterministic numbered deals, bounded replay/history, atomic local save/recovery, and declared keyboard/accessibility semantics; real Omarchy, keyboard-only, and assistive-technology acceptance remain pending. |
| `collection.local-profile` | Local profiles, statistics, and achievements | Planned | Not implemented. |
| `quality.accessibility` | Keyboard and assistive-technology acceptance | Partial | Declared semantics exist; real screen-reader and keyboard-only acceptance is pending. |
| `quality.real-omarchy` | Real Omarchy and Wayland acceptance | Planned | No real-platform evidence has been recorded. |
| `excluded.accounts` | Microsoft and Xbox accounts | Excluded | Hosted identity is outside the offline product boundary. |
| `excluded.ads` | Advertising services | Excluded | The product is intentionally offline and ad-free. |
| `excluded.vendor-content` | Microsoft-branded events and copied daily deals | Excluded | Vendor content and expressive presentation will not be copied. |

## Status definitions

- **Complete:** Workflow and required evidence are complete at a green exact revision.
- **Partial:** Some implementation exists, but workflow or acceptance evidence remains open.
- **Planned:** In scope and not implemented.
- **Excluded:** Outside the offline product boundary, with a documented reason.
