# Restart-current-deal candidate acceptance

This record defines the local restart workflow shared by Klondike, Spider,
FreeCell, TriPeaks, and Pyramid. It is display-independent candidate evidence,
not installed input or spoken assistive-technology acceptance.

## Workflow contract

- **Restart deal** recreates the active game's initial state from the same
  repository-defined seed or deal number and the exact active rule options.
- Restart never reserves, increments, lowers, or rewrites a next-deal sequence.
  The existing bounded deal-counter file is left untouched and preserved
  byte-for-byte.
- Restart is a deal boundary rather than a replay action. The fresh game has no
  undo or redo entry, while the device-local played/won profile remains intact
  and is not counted again merely because the deal was restarted.
- The prospective initial game must replace the active save atomically before
  it replaces memory. Missing save locations and stale-writer conflicts keep
  the current game in memory and expose retry, reload, discard, and cancel paths.
- A successful restart clears card selections and any prior replay history,
  writes mode-0600 state, and reopens through the existing versioned loaders.

## Automated evidence

`restart_current_deal_preserves_seed_rules_counters_and_profile_for_all_games`
progresses every game through a legal production action, invokes the shared
restart controller route, and requires the exact initial game, seed/deal number,
rule options (including a progressed timed Klondike deal returning to zero
elapsed seconds), mode-0600 save, loader equivalence, unchanged next-seed state
and counter bytes, unchanged local profile, empty history, clean dirty state,
and no pending request.

`restart_without_a_writable_save_fails_closed_and_can_be_cancelled` proves that
the in-memory game and counters remain unchanged when no writable save exists,
that the bounded pending restart remains visible, and that cancellation retains
the current game.

`restart_conflict_preserves_memory_then_reloads_and_retries_atomically` creates
a real stale-writer conflict, proves the external bytes and in-memory game are
both preserved, reloads the newer disk owner, then retries the same captured
restart to an exact atomic reopen.

`restart_current_deal_declares_keyboard_accessibility_and_recovery_contracts`
pins the focusable button, accessible name, pending restart state, and distinct
discard/cancel recovery copy.

## Remaining installed gate

The exact packaged control has not been invoked through an input-capable
offscreen or nested compositor. Exact-package keyboard focus/activation,
AT-SPI action/name/state, visible status transition, save/reopen, and foreground
preservation therefore remain open. No live Omarchy summon or production GUI is
permitted for this candidate, and all game, accessibility, packaging, profile,
and real-platform rows remain Partial.
