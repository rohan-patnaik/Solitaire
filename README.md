# Solitaire

An offline, ad-free collection of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

## Status

M0 foundation and the playable Klondike vertical slice are implemented. Renderer-independent, deterministic, serializable rules engines are also complete for Spider (one/two/four suit), FreeCell, Pyramid, and TriPeaks. Their Slint play surfaces and the collection layer remain tracked in [ROADMAP.md](ROADMAP.md).

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

## Omarchy plugin

After installing the native binary on `PATH` as `solitaire`:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

## License

MIT.

