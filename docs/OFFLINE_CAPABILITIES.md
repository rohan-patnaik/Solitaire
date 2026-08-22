# Offline capability catalog

Generated from `docs/offline-capabilities.json`; do not edit by hand.
No parity score is claimed. Statuses describe evidence, not UI presence.
Baseline evidence revision: `df9f4f3cf4b49482f031ea5b890a117a31b93408`.
Baseline CI: https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489.
Current remediation exact-tip CI: `df9f4f3cf4b49482f031ea5b890a117a31b93408` at https://github.com/rohan-patnaik/Solitaire/actions/runs/32498442489 (success).

| Status | Count |
| --- | ---: |
| Complete | 0 |
| Partial | 7 |
| Planned | 4 |
| Excluded | 3 |

| ID | Capability | Status | Known limit |
| --- | --- | --- | --- |
| `foundation.omarchy-launcher` | Detached Omarchy launcher | Partial | Real Omarchy acceptance is pending. |
| `foundation.arch-package` | Exact-revision Arch package | Partial | Requires exact-tip CI evidence after remediation. |
| `foundation.persistence-safety` | Bounded atomic local persistence | Partial | Crash-injection and real-filesystem acceptance remain pending. |
| `game.klondike` | Playable Klondike | Partial | Drag/drop, full options, win presentation, and platform acceptance remain open. |
| `game.spider` | Playable Spider | Partial | Broader hostile/property and platform evidence remain open. |
| `game.freecell` | Playable FreeCell | Partial | Solver-grade hints and platform acceptance remain open. |
| `game.pyramid` | Playable Pyramid | Planned | Renderer-independent engine exists; no playable surface. |
| `game.tripeaks` | Playable TriPeaks | Planned | Renderer-independent engine exists; no playable surface. |
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
