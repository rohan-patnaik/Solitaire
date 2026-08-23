# Solitaire five-game alpha

Solitaire is an offline-first native Linux alpha for Omarchy Quattro. The
current scope is playable Klondike, Spider, FreeCell, standard Pyramid, and
standard TriPeaks. It is not a Complete/Verified Microsoft Solitaire
Collection replacement. Exactly one applicable capability row is Complete: the
detached Omarchy launcher. All other applicable rows remain Partial.

## Install the exact accepted Arch package

These commands build and install the exact source accepted on real
Omarchy/Wayland. They require Arch Linux, the base development tools, Cargo,
Git, `fontconfig`, `wayland`, and `libxkbcommon`.

```sh
git clone https://github.com/rohan-patnaik/Solitaire.git
cd Solitaire
git switch --detach 4b31024426b73fafe93597e4cd42312eef2b26b0

revision=$(git rev-parse HEAD)
source_root=$(mktemp -d)
mkdir "$source_root/Solitaire"
git archive "$revision" | tar -x -C "$source_root/Solitaire"
printf '%s\n' "$revision" > "$source_root/Solitaire/.solitaire-source-revision"
source_archive="solitaire-source-$revision.tar.gz"
tar -C "$source_root" -czf "packaging/arch/$source_archive" Solitaire
source_sha256=$(sha256sum "packaging/arch/$source_archive" | cut -d' ' -f1)

cd packaging/arch
SOLITAIRE_SOURCE_ARCHIVE="$source_archive" \
SOLITAIRE_SOURCE_SHA256="$source_sha256" \
SOLITAIRE_EXPECTED_REVISION="$revision" \
  makepkg --cleanbuild --clean --noconfirm
sudo pacman -U ./solitaire-omarchy-0.1.0.r0.g4b31024-1-x86_64.pkg.tar.zst
```

The exact-tip [CI run 32645102863](https://github.com/rohan-patnaik/Solitaire/actions/runs/32645102863)
built, checked, installed, and verified that application revision.
The local evidence package was unsigned; signed checksums and release-upgrade
evidence remain open.

## Install and launch the Omarchy plugin

The root plugin is only a launcher. Install the native package first so that
`solitaire` is on `PATH`, then use the supported Omarchy commands:

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable --yes
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

The accepted exact-SHA pass observed the summon returning successfully, the
installed native window appearing, Quickshell remaining responsive, and the
same native process surviving `omarchy restart shell`.

At exact published revision `4b31024426b73fafe93597e4cd42312eef2b26b0`,
isolated live acceptance also exercised both launcher failure branches without
altering the installed package. The missing-binary path visibly delivered
`Solitaire is not installed`; an immediate exit-42 path visibly delivered
`Solitaire could not start`. Both exact notification bodies, diagnostics, exit
statuses, absence of a native process/window, and final cleanup were captured
through the real notification service and an isolated Quickshell. See
[`OMARCHY_LAUNCHER_ACCEPTANCE_4B31024.md`](OMARCHY_LAUNCHER_ACCEPTANCE_4B31024.md).

## Demonstrated alpha workflows

At published revision `4b31024426b73fafe93597e4cd42312eef2b26b0`
(tree `54d9c6ee27c9e0839e21579aac8a918549a4dbc1`), the exact plugin's normal,
missing-binary, and immediate-startup-failure paths passed live Omarchy/Wayland
acceptance. Together with the earlier accepted supported summon and shell
restart, this makes only the detached launcher workflow Complete.

At published revision `d20ba4111deb2e948e593fbeec4ca2c45b597bef`
(tree `d01c6f635bfb73e0e47838ac3f0287c6889c1069`), a focused installed-package
pass accepted keyboard stock deal, undo/redo, save/reopen, and AT-SPI identity
and action semantics in Spider one-, two-, and four-suit modes. It also accepted
the dedicated long-status surface visually, by keyboard, and through AT-SPI at
1180x820. The exact package was `solitaire-omarchy 0.1.0.r0.gd20ba41-1`;
`pacman -Qkk` reported nine files and no altered files. Exact observations and
remaining boundaries are recorded in
[`OMARCHY_WAYLAND_ACCEPTANCE_D20BA41.md`](OMARCHY_WAYLAND_ACCEPTANCE_D20BA41.md).

At published revision `f6b0cb7e55d296bdf77714efc48a1775b858c041`
(tree `7d3118ed8a6ab81239ed15ffcfc9e3095d2d1097`), exact-tip CI
[run 32591449130](https://github.com/rohan-patnaik/Solitaire/actions/runs/32591449130)
and real Omarchy/Wayland acceptance demonstrated:

- Installed legal mutation and save/reopen for all five games.
- Real keyboard navigation/activation, visible focus, and AT-SPI names, states,
  actions, and changing live-region names across the exercised surfaces.
- Undo/redo in Klondike, a Spider stock deal, a legal and an illegal FreeCell
  move, and stock draws in Pyramid and TriPeaks.
- Per-game anonymous device-local `deals played` counters without duplicate
  counting after undo/redo or reopen.
- Mode-0600 game/profile saves, malformed-profile quarantine with byte
  preservation, stale-writer conflict reporting, a dirty close guard, and
  explicit reload recovery.
- Plugin installation and summon, shell-restart survival, and clean native
  shutdown without creating normal user data during the launcher-only pass.

The installed package was `solitaire-omarchy 0.1.0.r0.gf6b0cb7-1`; its source
revision marker contained the full SHA, and `pacman -Qkk` found no altered
package files.

## Privacy and offline behavior

- Runtime play requires no account, login, telemetry, advertising service,
  hosted service, or cloud connection.
- Game saves and the anonymous local played/won counters stay under
  `$XDG_DATA_HOME/solitaire`, or `~/.local/share/solitaire` when
  `XDG_DATA_HOME` is unset.
- Named profiles, cloud identity, synchronization, import/export, and
  cross-device semantics are not implemented.
- Network access is needed to clone/download installation inputs. It is not a
  runtime gameplay dependency.

## Known alpha limits

- No five complete wins were played through the installed UI. Win transitions,
  full rule variants, broader hostile/property tests, drag/touch behavior,
  solver-grade FreeCell hints, Pyramid solvability metadata, and optional
  TriPeaks modes remain incomplete or unverified.
- Orca was not installed. AT-SPI was inspected, but spoken screen-reader output
  was not accepted.
- Long malformed-save and conflict messages use a dedicated full-width,
  character-wrapped, vertically scrollable display-only live region. Exact
  package acceptance at `d20ba41` confirmed the complete text was visually and
  programmatically reachable at 1180x820, recovery controls remained in their
  keyboard order, and a subsequent short status returned to view. Full
  keyboard-only traversal and spoken screen-reader acceptance remain pending.
- Achievements are deferred. There is no achievement taxonomy, named profile,
  streak system, import/export, sync, or multi-process merge.
- Cross-file transactions, crash injection, power-loss recovery, gamepad
  acceptance, signed checksums, SBOM, and package reproducibility evidence
  remain open.

The earlier focused launcher/profile acceptance record remains available in
[`OMARCHY_WAYLAND_ACCEPTANCE_1595FB0.md`](OMARCHY_WAYLAND_ACCEPTANCE_1595FB0.md).
