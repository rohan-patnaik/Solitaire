# Solitaire

An offline, ad-free collection of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

The canonical evidence/status inventory is `docs/offline-capabilities.json`; its generated human-readable view is [docs/OFFLINE_CAPABILITIES.md](docs/OFFLINE_CAPABILITIES.md). It intentionally makes no precise parity score claim.

## Status

M0 foundation and playable Slint surfaces for Klondike, Spider, FreeCell, standard Pyramid, and standard TriPeaks are implemented. Spider exposes one-, two-, and four-suit deals; FreeCell, Pyramid, and TriPeaks expose deterministic numbered deals. The surfaces route all pile interactions through renderer-independent engines and include undo, redo, hints, replay-backed save/resume, keyboard activation, accessible labels, live status text, and bounded device-local played/won counters.

Broader hostile/property coverage, the collection layer, and final real-Omarchy acceptance remain tracked in [ROADMAP.md](ROADMAP.md).

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
- Card and empty-slot controls declare assistive-technology names/default actions, accept Tab focus plus Space/Enter activation, and declare status changes as a polite live region. These are implementation contracts, not real-platform acceptance evidence; screen-reader acceptance and full gamepad navigation remain M5 work.
- Choose draw one or draw three before starting a new deal.

Progress is saved atomically under `$XDG_DATA_HOME/solitaire`, falling back to `~/.local/share/solitaire`, and restored at startup.

The active game's local statistics count a deal as played on its first successful mutation and count a win once for that numbered deal. The fixed-size versioned profile uses the same bounded atomic save, stale-writer detection, retry/close guard, and corrupt-file quarantine behavior as game saves. It is one anonymous device-local profile: named profiles, achievements, streaks, import/export, cross-device sync, and cloud identity remain unimplemented. Game and profile files are each atomic but are not one cross-file transaction, so power-loss acceptance remains open.

## Spider, FreeCell, Pyramid, and TriPeaks controls

- Choose a game from the picker and select the Spider suit count before starting a new deal.
- In Spider, select a face-up card or same-suit run, then choose another column. Use the stock control to deal a row; every column must be occupied.
- In FreeCell, select a card or alternating run, then choose a cascade, free cell, or suit foundation. Movable run size is derived from the available free cells and empty cascades.
- Undo, redo, and deterministic hints operate on the active game. Each game uses a separate versioned local save reconstructed from its replay actions.
- Cards and empty destinations declare pointer, assistive-technology default-action, and Tab plus Space/Enter activation semantics. Real assistive-technology acceptance remains pending.
- In standard Pyramid, activate an exposed king to remove it, or activate two exposed tableau/waste cards whose ranks total 13. The original seven-row layout exposes the exact deal number, stock, waste, score, move count, redeals, hints, and win status. Covered-card identities stay hidden visually and from accessible names.
- In standard TriPeaks, activate an exposed tableau card one rank above or below the waste, without King/Ace wraparound, or activate the stock/waste control to draw. The original four-row layout exposes deal, stock, waste, streak score, move count, hints, and win status without copying vendor presentation.

## Omarchy plugin

After installing the native binary on `PATH` as `solitaire`:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

## License

MIT.
