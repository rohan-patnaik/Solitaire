# Solitaire — five-game alpha

An offline, ad-free alpha of five classic patience games for Omarchy Quattro: Klondike, Spider, FreeCell, Pyramid, and TriPeaks.

This project implements public-domain game mechanics with its own code and original visual/audio presentation. It does not copy Microsoft card faces, card backs, backgrounds, animations, sounds, wording, screenshots, or layout.

The repository contains:

- A native Rust + Slint desktop game.
- A small Omarchy Quattro `menu` plugin that launches the native game.

The canonical evidence/status inventory is `docs/offline-capabilities.json`; its generated human-readable view is [docs/OFFLINE_CAPABILITIES.md](docs/OFFLINE_CAPABILITIES.md). It intentionally makes no precise parity score claim.

## Status

This is an alpha, not a Complete/Verified Microsoft Solitaire Collection replacement. M0 foundation and playable Slint surfaces for Klondike, Spider, FreeCell, standard Pyramid, and standard TriPeaks are implemented. Spider exposes one-, two-, and four-suit deals; FreeCell can open an explicit repository-defined decimal deal number or advance to the next one, while Pyramid and TriPeaks expose sequential deterministic deals. These numbers reproduce this repository's generator and are not a claim of compatibility with another product's deal numbering. The surfaces route all pile interactions through renderer-independent engines and include undo, redo, hints, replay-backed save/resume, keyboard activation, accessible labels, live status text, and bounded device-local played/won counters.

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
- Choose draw one or draw three and Standard or Vegas scoring before starting a
  new deal. Vegas starts at -52 and awards five points per foundation card;
  both scoring modes are currently untimed and allow unlimited stock passes.

Progress is saved atomically under `$XDG_DATA_HOME/solitaire`, falling back to `~/.local/share/solitaire`, and restored at startup.

The active game's local statistics count a deal as played on its first successful mutation and count a win once for that numbered deal. The fixed-size versioned profile uses the same bounded atomic save, stale-writer detection, retry/close guard, and corrupt-file quarantine behavior as game saves. It is one anonymous device-local profile: named profiles, achievements, streaks, import/export, cross-device sync, and cloud identity remain unimplemented. Game and profile files are each atomic but are not one cross-file transaction, so power-loss acceptance remains open.

## Spider, FreeCell, Pyramid, and TriPeaks controls

- Choose a game from the picker and select the Spider suit count before starting a new deal.
- In Spider, select a face-up card or same-suit run, then choose another column. Use the stock control to deal a row; every column must be occupied.
- In FreeCell, select a card or alternating run, then choose a cascade, free cell, or suit foundation. Movable run size is derived from the available free cells and empty cascades.
- Enter any decimal `u64` deal number and press Enter or choose **Open deal**
  to reproduce that repository-defined FreeCell layout. **Next deal** uses the
  independent durable sequence and explicit openings do not consume it.
- Undo, redo, and deterministic hints operate on the active game. Each game uses a separate versioned local save reconstructed from its replay actions.
- Cards and empty destinations declare pointer, assistive-technology default-action, and Tab plus Space/Enter activation semantics. Installed AT-SPI acceptance covers the five game surfaces and all three Spider suit modes; full keyboard-only traversal and spoken screen-reader output remain pending.
- In standard Pyramid, activate an exposed king to remove it, or activate two exposed tableau/waste cards whose ranks total 13. The original seven-row layout exposes the exact deal number, stock, waste, score, move count, redeals, hints, and win status. Covered-card identities stay hidden visually and from accessible names.
- In standard TriPeaks, activate an exposed tableau card one rank above or below the waste, without King/Ace wraparound, or activate the stock/waste control to draw. The original four-row layout exposes deal, stock, waste, streak score, move count, hints, and win status without copying vendor presentation.

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
