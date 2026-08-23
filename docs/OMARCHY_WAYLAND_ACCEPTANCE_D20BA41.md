# Spider variant and long-status acceptance at `d20ba41`

Date: 2026-08-23

This record covers a focused installed-package acceptance pass on real
Omarchy/Wayland. The Spider one-, two-, and four-suit workflow described below
passed in full. It does not make the wider Spider or product capability
Complete; every applicable catalog row remains Partial.

## Exact subject

- Published revision: `d20ba4111deb2e948e593fbeec4ca2c45b597bef`
- Git tree: `d01c6f635bfb73e0e47838ac3f0287c6889c1069`
- Exact-tip CI: <https://github.com/rohan-patnaik/Solitaire/actions/runs/32638817768>
- Successful CI jobs: Rust and exact-revision Arch package.
- Package: `solitaire-omarchy 0.1.0.r0.gd20ba41-1`
- Source archive SHA-256:
  `13c046bd855a7d03ef3652361a59983aa98d20ad0306a3d24181ee79219361c6`
- Package archive SHA-256:
  `a9978c21b0c303ecdf741455066617b6def6b467a328dde27b9b07600668cd05`
- `pacman -Qkk solitaire-omarchy`: `9 total files, 0 altered files`
- Installed source marker: the full published revision above; SHA-256
  `3a964a66fdc02dbb8b91dcf22b6b4467f1f10fe7354cc3f772892768c290c986`
- Installed `/usr/bin/solitaire`: mode 0755, 22,045,464 bytes; SHA-256
  `c23d413fd52598590c9809c2844ee447679dbc396feef9c9f60a9b5d22babe08`

The checkout was clean with `HEAD == origin/main`, and exact-tip CI completed
successfully. Each runtime exercise used an isolated `XDG_DATA_HOME`, a native
Wayland window (`xwayland=false`), and the default 1180x820 client size. Every
created save and local-profile file was mode 0600.

## Installed Spider workflow

For each of one, two, and four suits, keyboard-only input selected Spider,
selected the named next-deal suit control, and started a deal. AT-SPI exposed
the selector as `Suit count for a new Spider deal`. Each fresh replay stored
the corresponding `One`, `Two`, or `Four` setup enum.

In every mode:

1. AT-SPI exposed the stock as the named button `50 cards\nDeal row`, with one
   default `click` action and focused state after keyboard traversal.
2. Space dealt one card to every column. Stock, score, and moves changed from
   `50/500/0` to `40/499/1`, and the exact live status became
   `Game status: Move accepted`.
3. Keyboard Undo restored `50/500/0` with `Move undone`; keyboard Redo restored
   `40/499/1` with `Move restored`. The anonymous profile remained one deal
   played and zero won.
4. A clean close and installed-binary relaunch reopened the exact seed, setup,
   `DealRow` replay, stock, moves, visible cards, save/profile bytes, and played
   count. The suit combo was not touched during reopen, so the persisted replay
   rather than the next-deal selector proved the restored mode.
5. Screenshots kept stock, all tableau bottoms, the board, and status area in
   bounds at 1180x820.

AT-SPI exposed stock and tableau cards as named action controls. Face-up cards
had rank-and-suit names. All 44 hidden cards in every fresh and reopened
exercise were named only `Face-down card`; no hidden rank or suit was exposed.
The one-suit deal exposed only spades, the two-suit deal exposed spades and
hearts, and the four-suit deal exposed all four suits.

### Two-suit rule distinction

The two-suit pass additionally used keyboard card actions to move `4 hearts`
from the visible fourth column onto `5 spades` in the visible fifth column. The
legal cross-suit single-card build persisted with zero-based replay indexes as
`Move { from: 3, to: 4, count: 1 }`.

Selecting the resulting mixed `5 spades` plus `4 hearts` run and targeting a
distinct column produced the exact status
`Only a descending same-suit run can move together`. Save bytes, modification
time, score, moves, and profile remained unchanged, proving the rejected move
was atomic. The successful rule move was then explicitly undone before the
common stock-deal, undo/redo, and reopen workflow.

## Long-status correction

The same exact package retained the focused long-status acceptance evidence
from the immediately preceding pass. A path-rich recovery message was
character-wrapped in the dedicated status surface, Page Down reached its final
line, and Tab continued to the first game control. AT-SPI exposed one named
`Status details. Use arrow keys to scroll status messages` groupbox and a
polite live-region name equal to the complete visible status. After scrolling
to the bottom, a legal keyboard mutation reset the viewport and visibly exposed
the shorter `Move accepted` status. Recovery actions remained named and
enabled, and no face-down identity was disclosed.

The default 1180x820 package screenshots also kept Klondike, Spider, FreeCell,
Pyramid, and TriPeaks cards and fixed stock/waste controls in bounds. This
closes the previously recorded visual and AT-SPI acceptance gap for the
long-status surface.

## Evidence integrity

The durable source bundle is retained outside the repository at
`/home/rohan/Documents/Codex/2026-08-23/solitaire-spider-variant-acceptance-d20ba41.KMKDST/`.
Its `SHA256SUMS` manifest verified all 92 listed files, including the acceptance
record, screenshots, AT-SPI caches, and final isolated save/profile data.

## Remaining gaps

- No complete Spider deal was won through the installed UI. Complete-win
  presentation and the live `deals_won` transition remain unverified.
- Broader hostile/property coverage and drag/touch behavior remain pending.
- Full keyboard-only traversal and Orca spoken-output acceptance remain
  pending; Orca was not installed for this pass.
- Live missing-binary and immediate-startup-failure launcher notifications
  remain unverified.
- The accepted package was unsigned. Reproducibility, signed checksums, SBOM,
  release-upgrade evidence, and marketplace submission remain open.

Accordingly, Spider, accessibility, Arch packaging, and real-Omarchy
capabilities remain Partial.
