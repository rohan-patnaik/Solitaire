# Omarchy launcher acceptance at `4b31024`

Date: 2026-08-23

This record closes the `foundation.omarchy-launcher` workflow on real
Omarchy/Wayland. It combines the previously accepted supported launch and
shell-restart behavior with focused live acceptance of both launcher failure
branches. It does not make any gameplay, package, accessibility, or broader
real-Omarchy capability Complete.

## Exact subject

- Published revision: `4b31024426b73fafe93597e4cd42312eef2b26b0`
- Git tree: `54d9c6ee27c9e0839e21579aac8a918549a4dbc1`
- Exact-tip CI: <https://github.com/rohan-patnaik/Solitaire/actions/runs/32645102863>
- `Plugin.qml` SHA-256:
  `7f3fc9770c4af2d44024a5a67151a1df8ba4414dd80bcf05e737ba72808e2914`
- `manifest.json` SHA-256:
  `fa61e0f95cc992dd783bfa701bb8787b3442d215d407132c0749e5abf3850c1e`
- Installed package: `solitaire-omarchy 0.1.0.r0.g4b31024-1`
- Installed `/usr/bin/solitaire` SHA-256:
  `b7b2d8f7befe8ca2be10bbf92d784bde0ade18aa4bf3367fb6c0cfc0cb52bfd1`
- `pacman -Qkk solitaire-omarchy`: `9 total files, 0 altered files`
- Installed source marker: the full published revision above.

The isolated evidence lane and authoritative checkout were clean before and
after acceptance. The exact plugin and manifest required no product change.

## Supported launch

The supported command returned `ok` while the existing main Quickshell process
remained responsive:

```sh
omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
```

The exact installed binary mapped as a native Wayland 1180x820 window. Keyboard
Tab retained native-app focus, and AT-SPI exposed application `solitaire`, frame
`Solitaire — Klondike`, and the focused, focusable `Game picker` combo box. The
earlier exact-SHA pass at `f6b0cb7e55d296bdf77714efc48a1775b858c041`
separately proved that the detached native process survived a supported
`omarchy restart shell`.

## Missing binary

A read-only Bubblewrap namespace overlaid only `/usr/bin/solitaire` with a
non-executable task-local fixture. Inside that namespace,
`command -v solitaire` returned status 1 without moving, replacing, or changing
the package-owned binary.

The real isolated Quickshell loaded the exact repository plugin. Its launcher
delivered a critical D-Bus notification with exact title
`Solitaire is not installed` and exact body
`Install the native solitaire binary and try again.` The correlated notification
call/reply returned an ID, the exact stderr diagnostic was captured, and the
launcher exited 127. No Solitaire process or window existed.

## Immediate startup failure

A second read-only namespace overlaid only `/usr/bin/solitaire` with an
executable task-local fixture that emitted its marker and exited 42 immediately.
The launcher delivered a critical D-Bus notification with exact title
`Solitaire could not start` and exact body
`Solitaire failed during startup (exit 42). Run solitaire in a terminal for details.`
The correlated notification call/reply, exact stderr, fixture marker, and
final status 42 were captured. No Solitaire process or window existed.

Original-resolution screenshots visibly contained both accepted notification
title/body pairs. Repeated setup attempts left identical earlier missing-binary
toasts in that branch's screenshot; the contemporaneous D-Bus serial/reply,
screenshot hash, and detection timing identify the accepted delivery.

## Isolation and cleanup

The evidence fixture lived outside both repositories. Bubblewrap mounted the
host filesystem read-only and made only task-local evidence, XDG, and runtime
paths writable. A byte-identical copy of the host login profile instrumented
the launcher shell without editing `Plugin.qml` or any host file. D-Bus capture
was restricted to notification calls and corresponding service replies.

Final verification found only the original main Quickshell, no Solitaire or
fixture process/window, AT-SPI restored to false, no normal user Solitaire data,
the exact installed marker and binary hashes above, and both worktrees clean.

## Evidence integrity and scope

The source bundle is retained outside the repository at
`/home/rohan/Documents/Codex/2026-08-23/solitaire-launcher-row-acceptance-4b31024.fWs2sm/`.
Its `SHA256SUMS` manifest verified all 29 listed acceptance, screenshot, D-Bus,
launcher, fixture, and cleanup files.

This evidence makes only `foundation.omarchy-launcher` Complete. Arch package
signing/reproducibility, persistence fault tolerance, gameplay completion,
full keyboard traversal, Orca output, drag/touch, and broader real-Omarchy
acceptance retain their recorded Partial status.
