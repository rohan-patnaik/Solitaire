# Solitaire

An offline, ad-free collection of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

## Status

Foundation build. The native shell currently opens the original home screen and game selector. See [ROADMAP.md](ROADMAP.md).

## Build

Install the Rust toolchain and native Slint prerequisites, then:

```sh
cargo run
```

## Omarchy plugin

After installing the native binary on `PATH` as `solitaire`:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

## License

MIT.

