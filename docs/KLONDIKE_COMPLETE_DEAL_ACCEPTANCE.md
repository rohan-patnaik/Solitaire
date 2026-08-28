# Klondike complete-deal candidate acceptance

This record pins a normal deterministic default Klondike deal-zero replay one
legal move before victory. It is automated candidate evidence, not installed
input or assistive-technology acceptance.

## Legal replay fixture

- `tests/fixtures/klondike-seed-zero-near-win.json` is a revisioned replay
  envelope with draw-one, Standard scoring, unlimited redeals, untimed play,
  and no synthetic state or profile.
- Its SHA-256 is
  `64c2c0ac7f7900ae019bb406d5cdb4b33cb1a27f6b6de2e1273373a7840c86ef`.
- Production replay reconstruction accepts all 155 actions: 72 draws, three
  recycles, 51 foundation moves, eight waste-to-tableau moves, and 21
  tableau-to-tableau moves, with no foundation rollback.
- The reconstructed state preserves all 52 cards, has empty stock and waste,
  complete Clubs/Hearts/Spades foundations, Diamonds through Queen, and only
  the exposed King of Diamonds in tableau column 1 (internal index 0).
- Moving that card to the Diamonds foundation is action 156 and wins at score
  365. Replay reconstruction, undo, and redo reproduce the exact near-win and
  won states.

The fixture records a legal path for this repository's seed-zero shuffle. It
does not claim compatibility with another product's deal numbering or solver.

## Controller lifecycle candidate

`controller_completes_legal_klondike_replay_once_and_reopens` reconstructs the
fixture, stages it through validated revisioned Klondike persistence, and
routes the final move through the same tableau/foundation activation methods
used by keyboard, pointer, and assistive-technology default actions. It proves:

- selection of user-visible tableau column 1 and activation of Diamonds;
- exact status `Deal complete — beautifully played`;
- a 156-action full Klondike save and local profile with `0600` permissions;
- exactly one played and one won observation for Klondike deal zero;
- no duplicate profile write across undo, redo, or repeated observation; and
- loader-level exact reopen of the won game and profile bytes.

`klondike_complete_deal_survives_normal_controller_restart` extends that
coverage across two fresh source processes. The parent reconstructs the fixture
through production code and atomically writes the normal Klondike save format.
The first child starts a fresh `Controller`, completes the keyboard-routable
final move, verifies undo/redo and the exact one-time profile, and exits. The
second child starts another fresh `Controller` and proves the won game and
profile reopen byte-for-byte. Each child first verifies that `XDG_DATA_HOME`
matches a parent-created canonical temporary root beneath the system temporary
directory and that its mode-0600 PID/nanosecond nonce marker matches. Every
phase has a ten-second monotonic deadline; a stalled child is killed and reaped,
and captured diagnostics are bounded to 8 KiB per stream.

This is display-independent source-process lifecycle evidence. It is not the
installed process/window identity or final input gate below.

## Remaining installed gate

After publication and exact-SHA package CI, an input-capable offscreen
compositor or another method that continuously preserves the active desktop
may stage this isolated game, start the exact package, activate tableau column
1 and the Diamonds foundation, and verify the visible/AT-SPI completion status
plus save/profile bytes. Record exact source/package markers, clean shutdown,
and foreground preservation.

If input cannot be delivered without touching the active desktop, skip the
interaction and keep the Klondike, accessibility, real-platform, and profile
rows Partial.
