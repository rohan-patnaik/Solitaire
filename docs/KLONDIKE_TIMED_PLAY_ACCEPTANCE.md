# Klondike timed-play candidate acceptance

This record defines the dependency-free local timed/untimed Klondike workflow.
It is source and package candidate evidence, not installed keyboard, visual, or
spoken assistive-technology acceptance.

## Workflow contract

- A keyboard-focusable **Timing mode for a new Klondike deal** selector offers
  `Untimed` and `Timed`. Legacy three-field new-deal requests remain untimed;
  malformed, extra, case-changed, and 4 KiB timing fields fail atomically.
- A timed deal advances only while Klondike is active, no deal change is
  pending, and the deal is not won. Untimed, inactive, pending, won, zero-length,
  and saturated timer updates do not change state.
- The visible elapsed clock uses a bounded `HH:MM:SS` formatter. Hours may grow
  beyond two digits; even `u64::MAX` remains a short fixed-work conversion.
- Elapsed time is informational and does not change Standard or Vegas scoring.
  No time bonus, time penalty, pause button, or background timing is claimed.
- Undo and redo change cards and moves but never rewind the elapsed clock.
  Restart intentionally begins the same deal at zero elapsed seconds.
- The controller uses the existing checked mode-0600 save and stale-writer
  ownership protocol. It checkpoints at most once per 15 elapsed seconds and
  before leaving Klondike or closing. Successful moves also save the current
  clock through the existing mutation path. A crash can lose elapsed seconds
  since the last checkpoint; power-loss durability remains an open catalog gap.
- Missing save locations and stale writers preserve the in-memory timed game,
  expose the existing retry/reload/close guard, and prevent switching away when
  the final checkpoint cannot be owned safely. Timer checkpoints never observe
  a played deal and never write deal counters or the local profile.

## Automated evidence

- `timed_elapsed_seconds_do_not_rewind_through_undo_or_redo` pins monotonic
  elapsed time across replay history and reconstruction.
- `klondike_new_deal_choices_are_saved_and_reopen_with_exact_options` covers
  timed Standard and timed Vegas creation and exact loader reopen.
- `timed_klondike_checkpoints_are_bounded_atomic_and_profile_independent`
  proves no game write before the 15-second boundary, exact atomic reopen at
  the boundary, mode-0600 files, and byte-identical counters/profile.
- `timed_klondike_pauses_and_checkpoints_before_switching_games` covers pending
  and inactive pause behavior plus the final switch checkpoint.
- `timed_klondike_checkpoint_failure_is_recoverable_and_fail_closed` covers a
  missing save, retained memory, blocked switch, retry, and exact reopen.
- `timed_klondike_stale_checkpoint_preserves_both_owners_until_reload` creates
  a real stale-writer conflict and proves both owners survive until reload.
- `elapsed_time_format_is_bounded_and_exact` covers zero, hour rollover, and
  the maximum representable elapsed value.
- `klondike_timed_play_declares_keyboard_accessibility_and_checkpoint_contracts`
  pins the focusable selector, active clock name, timer route, pause conditions,
  checkpoint bound, and documented limits without launching a window.

## Remaining installed gate

The exact packaged selector and clock have not been exercised through an
input-capable offscreen or nested compositor. Keyboard selection, visible
one-second progression, AT-SPI name/value/state updates, pause behavior,
checkpoint/reopen, and foreground preservation therefore remain open. No live
Omarchy summon or production GUI is permitted for this candidate, and the
Klondike, accessibility, persistence, packaging, and real-platform rows remain
Partial.
