# FreeCell complete-deal candidate acceptance

This record pins a normal deterministic FreeCell deal-zero replay one legal
move before victory. It is automated candidate evidence, not installed input
or assistive-technology acceptance.

## Legal replay fixture

- `tests/fixtures/freecell-seed-zero-near-win.json` is a revisioned normal save
  envelope with no synthetic state or nonstandard setup.
- Its SHA-256 is
  `3824f1f98ec7e5f0a0c4038198765f02428606f0f1e7133649a146d8d2afdc82`.
- Production replay reconstruction applies all 105 recorded actions under the
  standard FreeCell rules and preserves all 52 cards.
- The reconstructed state has empty cascades; Clubs, Diamonds, and Hearts at
  13 cards; Spades at 12 cards; and only the King of Spades in free cell 2
  (internal index 1).
- Moving that card to the Spades foundation is action 106 and wins. Replay
  reconstruction, undo, and redo reproduce the exact near-win and won states.

The fixture records a legal path found for this repository's seed-zero shuffle.
It does not claim compatibility with another product's deal numbering or
solver.

## Controller lifecycle candidate

`controller_completes_legal_freecell_replay_once_and_reopens` loads the normal
save through revisioned persistence and routes the last move through the same
free-cell and foundation activation methods used by keyboard, pointer, and
assistive-technology default actions. It proves:

- selection of user-visible free cell 2 and activation of the Spades
  foundation;
- exact status `FreeCell complete — every suit is home`;
- a 106-action revisioned save and local profile with `0600` permissions;
- exactly one played and one won observation for FreeCell deal zero;
- no duplicate profile write across undo, redo, or repeated observation; and
- loader-level exact reopen of the won game and profile bytes.

`freecell_complete_deal_survives_normal_controller_restart` extends that
coverage across two fresh source processes. The parent installs the pinned
normal save in an ownership-checked temporary root. The first child starts a
fresh `Controller`, selects FreeCell, completes the keyboard-routable final
move, verifies undo/redo and the exact one-time profile, and exits. The second
child starts another fresh `Controller` and proves the won game and profile
reopen byte-for-byte. Each phase has the shared ten-second kill-and-reap
deadline, bounded diagnostics, hostile ambient-environment guard, and complete
task-root cleanup on success or unwind.

This is display-independent source-process lifecycle evidence. It is not the
installed process/window identity or final input gate below.

## Remaining installed gate

After publication and exact-SHA package CI, an input-capable offscreen
compositor or another method that continuously preserves the active desktop
may install the exact package, stage this isolated save, start the package,
activate free cell 2 and the Spades foundation, and verify the visible and
AT-SPI status transition plus save/profile bytes. Record exact source/package
markers, clean shutdown, and foreground preservation.

If input cannot be delivered without touching the active desktop, skip the
interaction and keep the FreeCell, accessibility, real-platform, and profile
rows Partial.
