# Klondike left-handed layout candidate acceptance

This record defines the dependency-free, session-scoped left-handed Klondike
table workflow. It is source and exact-package candidate evidence, not installed
keyboard, visual, or spoken assistive-technology acceptance.

## Workflow contract

- A keyboard-focusable **Klondike table layout for this session** selector
  offers `Right-handed` and `Left-handed`. Right-handed remains the startup
  default.
- Left-handed mode mirrors the top-row stock, waste, and four foundation
  positions. Pile identities, suit indices, action routes, accessible names,
  and keyboard traversal order do not change.
- The seven tableau columns are intentionally unchanged. The workflow moves the
  draw controls to the right-hand side without reversing cards, rules, deals,
  replay actions, scoring, timers, or saved game state.
- The layout choice survives game changes, new deals, restart deal, undo, redo,
  and rendering during the current process because it is native UI state. It
  intentionally returns to Right-handed after process exit; persistence requires
  an owner-approved settings-storage design and is not claimed here.
- The bounded position function clamps every normal finite width inside the
  available surface, produces an exact mirror for all six top-row piles, and
  returns a safe origin for invalid slots and non-finite widths. It performs
  fixed work and allocates nothing.
- No save, profile, counter, replay, fixture schema, package dependency, asset,
  license, plugin, or collection taxonomy changes are introduced.

## Automated evidence

- `klondike_handed_layout_is_an_exact_bounded_mirror` proves exact positions,
  mirror symmetry, small and negative width bounds, invalid-slot handling, and
  non-finite input recovery without a display.
- `klondike_left_handed_layout_declares_keyboard_accessibility_and_scope`
  pins the selector values, default, accessible session scope, shared position
  route for stock/waste/foundations, unchanged activation indices, documentation,
  and catalog limits.
- The existing controller, domain, persistence, recovery, undo/redo, timed-play,
  complete-deal, and deployment suites remain the regression boundary because
  this slice changes no game or stored state.

## Remaining installed gate

The exact packaged selector has not been exercised through an input-capable
offscreen or nested compositor. Keyboard selection, visible mirroring at narrow
and default window sizes, AT-SPI value/state updates, interaction after switching
games, process-reset behavior, and foreground preservation remain open. No live
Omarchy summon or production GUI is permitted for this candidate, so the
Klondike, accessibility, packaging, and real-platform rows remain Partial.
