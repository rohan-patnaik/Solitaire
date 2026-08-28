# Spider suit-selector synchronization candidate acceptance

This record covers accurate one-, two-, and four-suit selection across a new
deal, dirty confirmation, and save/reopen. It is automated controller and
display-independent mapping evidence, not new exact-package input acceptance.

## User workflow and compatibility

The Spider selector exposes the existing `1 suit`, `2 suits`, and `4 suits`
rules. The active mode is now reported separately in the header and gameplay
description. On startup or game selection, rendering maps the persisted
`SuitMode` to both the ComboBox value and selection index. While a dirty future
deal is pending, rendering does not overwrite that future selection with the
current game's mode.

No replay or save schema changes. All three enum values were already supported
by published builds, so saves remain compatible with published `fa15999` and
earlier builds that implemented the three Spider modes.

## Atomic and lifecycle evidence

`spider_suit_options_are_strict_atomic_reopenable_and_mapped` proves:

- every preset starts and atomically reopens with the exact persisted mode;
- the production mapping returns the matching value and index without creating
  a display backend;
- a dirty four-suit choice remains staged while empty, malformed, pluralization,
  whitespace, unavailable three-suit, trailing, and 4 KiB hostile values are
  rejected;
- each rejection preserves the pending request, in-memory two-suit game, game
  save bytes, and durable counter bytes;
- game and counter files remain mode `0600`; and
- explicit dirty confirmation commits and reopens the staged four-suit deal.

Existing Spider engine and controller coverage supplies legal moves, undo/redo,
bounded replay/history, hints, completion, hostile actions, conservation,
save/reopen, corrupt recovery, and all three rules. The exact installed
`d20ba41` pass exercised new-deal selection, keyboard mutation, undo/redo,
save/reopen, and AT-SPI identity/action semantics in all three modes. It did not verify the corrected reopened ComboBox value/index synchronization.

## Remaining installed gate

A future safe offscreen exact-package pass may reopen each mode and inspect the
selector's value/index plus active header through keyboard and AT-SPI. Under the
no-focus policy, this is skipped unless the user's active application can be
preserved continuously. The Spider, accessibility, and real-platform rows therefore remain Partial.
