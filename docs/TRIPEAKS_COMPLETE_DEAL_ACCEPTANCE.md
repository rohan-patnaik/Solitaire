# TriPeaks complete-deal candidate acceptance

This record pins a normal deterministic replay to one keyboard-routable move
before a complete standard TriPeaks win. It is automated candidate evidence,
not a claim that the installed final transition has been exercised.

## Legal replay fixture

- Fixture: `tests/fixtures/tripeaks-seed-zero-near-win.json`
- SHA-256: `c7063d5b9a9c99c1c2034c7f17a8c4a5322807b9242de3f5bdb73d2e832e3f9c`
- Deal: repository-defined seed `0`, standard mode with wraparound disabled.
- The fixture contains only replay metadata and 48 ordinary `Draw`/`Remove`
  actions. It contains no synthetic state or profile.
- Production `Game::from_replay` reconstructs 52 conserved cards, one exposed
  tableau card at position zero, two stock cards, 49 waste cards, score 5,700,
  and move 48. Removing tableau position zero is legal and reaches score 5,800,
  move 49, and an empty tableau.

The checked-in transcript's authority is production replay reconstruction:
every recorded action must pass the normal engine rules in order, and the
resulting state must satisfy the pinned conservation and near-win assertions.

## Controller lifecycle candidate

`controller_completes_legal_tripeaks_replay_once_and_reopens` loads the normal
0600 replay through the revisioned persistence path, routes the final action
through `activate_tripeaks_card(0)`, and requires:

- visible status `TriPeaks complete — all three peaks are clear`;
- 52-card conservation, score 5,800, move 49, and 49 replay actions;
- atomic 0600 game and profile saves;
- exactly one played and one won observation for TriPeaks deal zero;
- no duplicate profile mutation across undo, redo, repeated observation, or
  loader-level reopen; and
- exact reconstruction of the won game and profile from disk.

`tripeaks_complete_deal_survives_normal_controller_restart` extends that
coverage across two fresh source processes. The parent installs the pinned
normal save in an ownership-checked temporary root. The first child starts a
fresh `Controller`, selects TriPeaks, completes the keyboard-routable final
removal, verifies score, undo/redo, and the exact one-time profile, and exits.
The second child starts another fresh `Controller` and proves the won game and
profile reopen byte-for-byte. Each phase has the shared ten-second kill-and-
reap deadline, bounded diagnostics, hostile ambient-environment guard, and
complete task-root cleanup on success or unwind.

This is display-independent source-process lifecycle evidence. It is not the
installed process/window identity or final input gate below.

## Remaining installed gate

After publication and exact-SHA package CI, an input-capable offscreen
compositor or another explicitly authorized non-foreground method must install
the fixture as the isolated TriPeaks save, start the exact package without
changing the user's active app, and use keyboard input to activate tableau
position 1, the sole remaining top card. This is engine/callback index `0` but
the visible and AT-facing position is one-based. Record the visible
win/status/statistics transition, AT-SPI name/action and polite live-region
update, 0600 save/profile hashes, undo/redo, save/reopen, package/source
markers, clean shutdown, and foreground preservation. If those inputs cannot
be delivered safely, skip the interaction and keep the TriPeaks,
accessibility, real-platform, and profile rows Partial.
