# Pyramid complete-deal candidate acceptance

This record pins a normal deterministic standard Pyramid replay to one legal
pair before victory. It is automated candidate evidence, not installed input
acceptance.

## Legal replay fixture

- Fixture: `tests/fixtures/pyramid-seed-zero-near-win.json`
- SHA-256: `c2ed2aba92d2af5ea4beff5a15d94c0892a6bf54b3b05f3eec7b6cb1a49777aa`
- Deal: repository-defined seed `0`, standard two-redeal mode.
- The fixture contains only replay metadata and 62 ordinary actions: 38 draws,
  two recycles, 18 pair removals, and four king removals. It contains no
  synthetic state or profile.
- Production `Game::from_replay` accepts every action and reconstructs one
  exposed pyramid card at engine index zero, ten stock cards, one waste card,
  two used redeals, score 400, and move 62. Forty removed cards plus the 12
  represented cards account for the original 52-card deck.
- The remaining pyramid and waste ranks total 13. Their legal pair removal
  reaches score 420, move 63, an empty pyramid, 63 replay actions, and 42
  removed plus ten represented cards.

The checked-in transcript's authority is production replay reconstruction and
the pinned action, accounting, exposure, scoring, and near-win assertions.

## Controller lifecycle candidate

`controller_completes_legal_pyramid_replay_once_and_reopens` loads the normal
0600 replay through revisioned persistence, activates engine index `0`, then
the waste through the same routes used by keyboard/default actions, and
requires:

- selection of the final pyramid card before the waste activation;
- visible status `Pyramid complete — every tableau card is clear`;
- exact score 420, move 63, and 63-action saved replay;
- atomic 0600 game and profile saves;
- exactly one played and one won observation for Pyramid deal zero;
- no duplicate profile mutation across undo, redo, repeated observation, or
  loader-level reopen; and
- exact reconstruction of the won game and profile from disk.

## Remaining installed gate

After publication and exact-SHA package CI, an input-capable offscreen
compositor or another explicitly authorized non-foreground method must install
the fixture as the isolated Pyramid save and start the exact package without
changing the user's active app. Use keyboard input to select the sole remaining
top card, exposed to users and AT as Pyramid tableau position 1, then activate
the waste card. The internal callback index is `0`; the visible and AT-facing
position is one-based. Record the selection announcement, visible
win/status/statistics transition, AT-SPI names/actions and polite live-region
update, 0600 save/profile hashes, undo/redo, save/reopen, package/source
markers, clean shutdown, and foreground preservation. If safe input delivery
is unavailable, skip the interaction and keep the Pyramid, accessibility,
real-platform, and profile rows Partial.
