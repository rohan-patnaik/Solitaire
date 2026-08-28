# Solitaire — five-game alpha

An offline, ad-free alpha of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

The canonical evidence/status inventory is `docs/offline-capabilities.json`; its generated human-readable view is [docs/OFFLINE_CAPABILITIES.md](docs/OFFLINE_CAPABILITIES.md). It intentionally makes no precise parity score claim.

## Status

This is an alpha, not a Complete/Verified Microsoft Solitaire Collection replacement. M0 foundation and playable Slint surfaces for Klondike, Spider, FreeCell, Pyramid, and Standard plus optional Ace-King-wrap TriPeaks are implemented. Spider exposes one-, two-, and four-suit deals; FreeCell can open an explicit repository-defined decimal deal number or advance to the next one; Pyramid exposes zero-, one-, and two-redeal choices; and Pyramid and TriPeaks use sequential deterministic deals. These numbers reproduce this repository's generator and are not a claim of compatibility with another product's deal numbering. The surfaces route all pile interactions through renderer-independent engines and include undo, redo, hints, replay-backed save/resume, keyboard activation, accessible labels, live status text, and bounded device-local played/won counters.

See the [alpha install, demonstrated workflows, privacy boundary, and known limits](docs/ALPHA_RELEASE.md). Broader hostile/property coverage, the collection layer, and remaining release acceptance are tracked in [ROADMAP.md](ROADMAP.md).

## Build

Install the Rust toolchain and native Slint prerequisites, then:

```sh
cargo run
```

On Arch Linux the native prerequisites are provided by `fontconfig`, `wayland`, and `libxkbcommon`.

## Klondike controls

- Select a face-up card or complete run, then select a tableau column or foundation.
- Select the stock to draw or recycle the waste.
- Use the focusable toolbar controls for undo, redo, a deterministic hint, or safe foundation moves.
- Card and empty-slot controls declare assistive-technology names/default actions, accept Tab focus plus Space/Enter activation, and declare status changes as a polite live region. Focused installed passes exercised these semantics through keyboard input and AT-SPI; full keyboard-only traversal, spoken screen-reader output, and gamepad navigation remain M5 work.
- Choose draw one or draw three, Standard or Vegas scoring, and unlimited, one,
  or three stock redeals plus Untimed or Timed play before starting a new deal.
  Vegas starts at -52 and awards five points per foundation card. The current
  redeal count, bounded remainder, and timed `HH:MM:SS` clock stay visible.
  Timed progress pauses outside active Klondike, during pending deal changes,
  and after a win; atomic checkpoints run every 15 seconds and before leaving
  the game or closing. Time is informational and does not alter scoring.
- Choose Right-handed or Left-handed table layout from the keyboard-focusable
  session selector. Left-handed mode mirrors stock, waste, and foundations while
  preserving pile identities and keyboard order. Tableau columns are unchanged,
  and the choice intentionally resets to Right-handed after process exit.
- Single-click exposed waste and top-tableau cards for the existing select/move
  workflow, or double-click one to request a direct move to its suit foundation.
  Keyboard and accessibility-default activation remain immediate; stale card
  identity or position fails closed and asks for another click.
- Use the keyboard-focusable **Finish safe moves** action to move every
  currently safe exposed Klondike card to a foundation. It clears stale card
  selection, reports an exact move count, persists through the normal checked
  save/profile path, and leaves each moved card as one ordinary undoable replay
  action. A zero-move request is a write-free no-op.
- A checked-in normal seed-zero draw-one/Standard replay reconstructs to one
  foundation move before victory. A bounded display-independent lifecycle gate
  starts a fresh Controller, exercises the final tableau/foundation route,
  status, atomic persistence and undo/redo, exits, then proves a second fresh
  Controller reopens byte-identical won-game and one-time profile files.
  Exact-package final-action and process/window acceptance remain open.

The display-independent safe-finish lifecycle, including no-op, undo/redo,
checked save/reopen, one-time profile observation, and stale-writer reload, is
recorded in
[`KLONDIKE_SAFE_FINISH_ACCEPTANCE.md`](docs/KLONDIKE_SAFE_FINISH_ACCEPTANCE.md).
Exact-package keyboard, AT-SPI, spoken-output, and visible multi-move acceptance
remain open.

Progress is saved atomically under `$XDG_DATA_HOME/solitaire`, falling back to `~/.local/share/solitaire`, and restored at startup.

The active game's local statistics count a deal as played on its first successful mutation and count a win once for that numbered deal. The fixed-size versioned profile uses the same bounded atomic save, stale-writer detection, retry/close guard, and corrupt-file quarantine behavior as game saves. It is one anonymous device-local profile: named profiles, achievements, streaks, import/export, cross-device sync, and cloud identity remain unimplemented. Game and profile files are each atomic but are not one cross-file transaction, so power-loss acceptance remains open.

All five games expose **Restart deal** as a keyboard-focusable action. It
recreates the active deal from the same repository-defined number and exact
rules, atomically replaces the save, preserves next-deal counters and local
statistics, and fails closed with explicit recovery controls. Restart begins a
fresh replay history and is intentionally not itself undoable. Exact-package
keyboard and AT-SPI acceptance for this control remains open.

## Spider, FreeCell, Pyramid, and TriPeaks controls

- Choose a game from the picker and select the Spider suit count before starting
  a new deal. Reopened one-, two-, and four-suit games synchronize the selector
  value/index and report the active mode separately from a pending future choice.
- In Spider, select a face-up card or same-suit run, then choose another column. Use the stock control to deal a row; every column must be occupied.
- A checked-in normal one-suit Spider replay reaches one move before victory.
  A display-independent process lifecycle test starts from that save through a
  fresh Controller, completes it, exercises undo/redo, exits, and proves a
  second fresh Controller reopens byte-identical 8/8 game and one-time profile
  files. Exact-package final-action and process/window acceptance remain open.
- In FreeCell, select a card or alternating run, then choose a cascade, free cell, or suit foundation. Movable run size is derived from the available free cells and empty cascades.
- Enter any decimal `u64` deal number and press Enter or choose **Open deal**
  to reproduce that repository-defined FreeCell layout. **Next deal** uses the
  independent durable sequence and explicit openings do not consume it.
- A checked-in normal seed-zero FreeCell replay reconstructs to one foundation
  move before victory. A bounded display-independent lifecycle gate starts a
  fresh Controller, exercises the final free-cell/foundation route, status,
  atomic persistence and undo/redo, exits, then proves a second fresh Controller
  reopens byte-identical won-game and one-time profile files. Exact-package
  final-action and process/window acceptance remain open.
- Undo, redo, and deterministic hints operate on the active game. Each game uses a separate versioned local save reconstructed from its replay actions.
- Cards and empty destinations declare pointer, assistive-technology default-action, and Tab plus Space/Enter activation semantics. Installed AT-SPI acceptance covers the five game surfaces and all three Spider suit modes; full keyboard-only traversal and spoken screen-reader output remain pending.
- In Pyramid, choose zero, one, or two stock redeals for the next sequential
  deal. Activate an exposed king to remove it, or activate two exposed
  tableau/waste cards whose ranks total 13. The original seven-row layout
  exposes the exact deal number, stock, waste, score, move count, redeals used
  against the active maximum, hints, and win status. Covered-card identities
  stay hidden visually and from accessible names.
- A checked-in normal seed-zero Pyramid replay reconstructs to one legal pair
  before victory. A bounded display-independent lifecycle gate starts a fresh
  Controller, exercises the final selection/pair route, status, score, atomic
  persistence and undo/redo, exits, then proves a second fresh Controller
  reopens byte-identical won-game and one-time profile files. Exact-package
  final-action and process/window acceptance remain open.
- In TriPeaks, choose Standard adjacency or the optional Ace-King wrap rule for
  a new deal. Activate an exposed tableau card one rank above or below the
  waste, or activate the stock/waste control to draw. The original four-row
  layout exposes the active rule, deal, stock, waste, streak score, move count,
  hints, and win status without copying vendor presentation.
- A checked-in normal seed-zero TriPeaks replay reconstructs to one legal move
  before victory. A bounded display-independent lifecycle gate starts a fresh
  Controller, exercises the final removal, score, atomic persistence and
  undo/redo, exits, then proves a second fresh Controller reopens byte-identical
  won-game and one-time profile files. Exact-package final-action and
  process/window acceptance remain open.

## Omarchy plugin

Follow the [exact accepted Arch package instructions](docs/ALPHA_RELEASE.md#install-the-exact-accepted-arch-package), then install and summon the thin launcher:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable --yes
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

Remove the launcher and native package with:

```sh
omarchy plugin remove io.github.rohan-patnaik.solitaire
sudo pacman -Rns solitaire-omarchy
```

The root `preview.png` is the original monochrome marketplace mark for this
plugin listing.

## License

MIT.
