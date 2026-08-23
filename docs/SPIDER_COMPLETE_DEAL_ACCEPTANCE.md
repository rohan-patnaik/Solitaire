# Spider complete-deal candidate acceptance

This is the acceptance procedure for the remaining installed Spider complete-win
workflow. It is not a record of a passed package run. The accepted revision,
source archive, package archive, installed source marker, binary hash, and live
Wayland observations must be recorded separately before the catalog can close
that installed-evidence gap.

## Legal replay fixture

- Source boundary: published revision
  `e45da16bbd19e0c04f7a76696d309eac7681f4db` plus the reviewed candidate diff.
- Fixture: `tests/fixtures/spider-one-suit-near-win.json`.
- Fixture SHA-256:
  `caf35da8ed4a60d55f87ad2967e80a89f804b993b40619fd85a3b0af341f394e`.
- Exact shape: 4,340 bytes; save envelope version 1; game `spider`;
  replay version 2; seed 3; setup `One`; 118 ordinary actions (113 moves and
  five stock rows). It contains no serialized `State`, debug flag, arbitrary
  state injection, or profile data.
- Provenance: a task-local deterministic search explored only the ordinary
  one-suit `Move` and `DealRow` action space. The checked-in fixture is the
  one-action-short prefix of the resulting 119-action solution. Production
  `Game::from_replay` is the authority: it reconstructs the fixture to 7/8
  completed runs, 104 cards, score 1,082, move 118, empty stock, ten cards in
  zero-based column 0, and three cards in zero-based column 2. Every remaining
  card is a face-up spade. Column 0 is exactly Ten, Nine, Eight, Seven, Six,
  Five, Four, Three, Two, Ace; column 2 is exactly King, Queen, Jack.
- Final action: `Move { from: 0, to: 2, count: 10 }`, meaning the visible first
  column's Ten-through-Ace run moves onto the King-through-Jack run in the
  visible third column. The production engine reaches 8/8, 104 cards, score
  1,181, move 119, and an empty tableau. Full replay reconstruction, undo, and
  redo are exact.

The fixture is intentionally a normal replay save, not a claim that a person
manually entered all 118 setup actions through the UI. Acceptance covers the
ordinary installed final transition from a complete legal transcript.

The focused controller test reopens the saved game and profile through the
production persistence loaders. It does not exercise installed `Controller`
startup. The clean installed-process close and startup/reopen below remains a
mandatory runtime gate.

## Exact-package Wayland gate

Do not start runtime acceptance until `HEAD == origin/main ==` the recorded full
published revision and terminal success is recorded for both the exact-head
Rust and exact-revision Arch package CI jobs. Record the full revision, tree,
CI run and job URLs, package version, source archive SHA-256, package archive
SHA-256, full installed source marker and its SHA-256, and installed binary
SHA-256.

Use the exact candidate package on the normal host with an isolated
`XDG_DATA_HOME`. Then:

1. Record the original AT-SPI enabled state and confirm the normal user
   Solitaire data path is absent or byte-identical before and after the run.
   Confirm there is no isolated local profile. Install the fixture as
   `$XDG_DATA_HOME/solitaire/spider-save.json` with mode 0600. Do not create or
   seed `local-profile.json`.
2. On the normal host, require `pacman -Q solitaire-omarchy` to report the
   recorded package version and `pacman -Qkk solitaire-omarchy` to report
   exactly `9 total files, 0 altered files`.
3. Launch the installed `/usr/bin/solitaire` on real Wayland and select Spider.
   Require exactly one Solitaire process and one native Wayland window with
   client size 1180x820 and `xwayland=false`. Resolve `/proc/$PID/exe` to
   `/usr/bin/solitaire` and require its SHA-256 to equal the recorded installed
   binary hash.
   Loading alone must leave device-local Spider statistics at `0 played · 0 won`.
   The saved replay must retain seed 3. The visible toolbar must read
   `Score  1082     Moves  118     Runs  7/8`; the board must show stock empty
   and `Completed suited runs  7 of 8`.
4. Through real Tab focus plus Space or Enter only, activate the Ten of spades
   at the start of the first column's ten-card run, then the King of spades in
   the third column. Record visible focus and both focused AT-SPI button nodes.
   Pointer clicks and AT-SPI default-action invocation are not substitutes for this keyboard-only gate.
5. Require the visible toolbar to read
   `Score  1181     Moves  119     Runs  8/8`, with an empty tableau and
   `Completed suited runs  8 of 8`. The complete status must be exactly
   `Spider complete — all eight runs are home`; the polite live-region name
   must be exactly `Game status: Spider complete — all eight runs are home`.
   Device-local Spider statistics must become exactly `1 played · 1 won`.
6. Preserve original-resolution 1180x820 screenshots before and after the final
   transition, with hashes, alongside the focused AT-SPI tree evidence.
7. Require both `spider-save.json` and `local-profile.json` to be regular 0600
   files. Record their full SHA-256 values. The save must remain a version-1
   envelope containing a version-2 one-suit replay with 119 actions and the
   final move above.
8. Activate Undo and Redo through the installed controls. Undo must restore the
   exact 7/8 prefix; Redo must restore the exact 8/8 game. The local-profile
   bytes and SHA-256 must not change. The focused controller test separately
   exercises an additional repeated observation and requires the same bytes.
9. Close cleanly and require the process and window to disappear. Reopen the
   installed application through a new normal `Controller` startup, repeat the
   process/executable/window identity checks, select Spider, and require the 8/8
   game and `1 played · 1 won` profile to reopen exactly. Both recorded file
   hashes must remain unchanged after the final Redo and reopen.
10. Inspect the live tree with AT-SPI. Require named, enabled card buttons for the
    final action, the named `Status details. Use arrow keys to scroll status
    messages` group, the exact polite live-region status, and no face-down card
    identity exposure.
11. Close cleanly again and require no Solitaire process or window. Confirm no
    normal user Solitaire data was created or changed, restore the original
    AT-SPI enabled state, and checksum the complete evidence bundle.

## Boundary

Passing this gate proves one installed one-suit final transition from a normal,
legally reconstructed complete-deal transcript. It does not prove manual UI
entry of the setup transcript, installed complete wins in two- or four-suit
mode, drag/drop, touch input, full keyboard-only traversal, Orca speech, package
reproducibility, signing, marketplace publication, or Microsoft parity. Spider
and the overall product remain Partial while the named gaps remain open.
