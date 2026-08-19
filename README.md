# Solitaire

An offline, ad-free collection of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

## Status

M0 foundation and playable Slint surfaces for Klondike, Spider, and FreeCell are implemented. Spider exposes one-, two-, and four-suit deals; FreeCell exposes deterministic numbered deals. Both surfaces route all pile interactions through their renderer-independent engines and include undo, redo, hints, replay-backed save/resume, adaptive columns, keyboard activation, accessible labels, and live status text.

Pyramid and TriPeaks currently remain engine-only. Their Slint play surfaces, the broader collection layer, and final real-Omarchy acceptance remain tracked in [ROADMAP.md](ROADMAP.md).

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
- Card and empty-slot controls expose assistive-technology default actions, accept Tab focus plus Space/Enter activation, and announce status changes as a polite live region. Full gamepad navigation remains M5 work.
- Choose draw one or draw three before starting a new deal.

Progress is saved atomically under `$XDG_DATA_HOME/solitaire`, falling back to `~/.local/share/solitaire`, and restored at startup.

## Spider and FreeCell controls

- Choose a game from the picker and select the Spider suit count before starting a new deal.
- In Spider, select a face-up card or same-suit run, then choose another column. Use the stock control to deal a row; every column must be occupied.
- In FreeCell, select a card or alternating run, then choose a cascade, free cell, or suit foundation. Movable run size is derived from the available free cells and empty cascades.
- Undo, redo, and deterministic hints operate on the active game. Each game uses a separate versioned local save reconstructed from its replay actions.
- Cards and empty destinations accept pointer activation, assistive-technology default actions, and Tab plus Space/Enter keyboard activation.

## Omarchy plugin

After installing the native binary on `PATH` as `solitaire`:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

## License

MIT.

