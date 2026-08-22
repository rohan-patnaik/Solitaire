# Omarchy and Wayland acceptance at `1595fb0`

Date: 2026-08-22

This record covers a real Omarchy installation and Wayland session. It is
acceptance evidence for a partial capability, not a Complete/Verified parity
claim.

## Exact subject

- Published revision: `1595fb02488525629c3f7d4aa1962e12c99198c0`
- Git tree: `a840483b7131f26d23de55e5ca58ed43c73a3c78`
- Exact-tip CI: <https://github.com/rohan-patnaik/Solitaire/actions/runs/32586839845>
- Successful CI jobs: Rust `97064399460`; exact-revision Arch package
  `97064399522`
- Package: `solitaire-omarchy 0.1.0.r0.g1595fb0-1`
- Package SHA-256:
  `ed624fd4e938795317e48b8def1a215ffaf946fd2e724c5a1343b96e65a35637`
- Source archive SHA-256:
  `81de1c1408e344ac3b3cdef42d1048ffd629ed8637892647e71437da21bccd64`
- Installed `/usr/bin/solitaire` SHA-256:
  `93b766712e5cffb9f20a88da269265a0037d79a7691f97ae79d8df6673abdfcf`

`pacman -Q solitaire-omarchy` reported the package version above, and
`/usr/share/solitaire-omarchy/source-revision` contained the full published
revision. The session used Omarchy `4.0.0-1`, Hyprland `0.56.2-1`, a 1920x1080
scale-1 Wayland output, AT-SPI `2.60.6`, and `wtype 0.4-2`. Private
`XDG_DATA_HOME` directories isolated every game-data exercise.

## Reproducible acceptance and observations

1. An installed-binary launch mapped a native `Solitaire — Klondike` window.
   The process executable resolved to `/usr/bin/solitaire` with the hash above.
2. AT-SPI exposed the game and draw-mode pickers, named toolbar actions,
   device-local statistics, a polite status region, stock, waste, foundations,
   and visible cards. Face-down card identities were not disclosed.
3. Seven Tab presses focused the stock. Space showed its visible focus border,
   drew from 24 to 23 cards, changed status to `Move accepted`, and changed
   statistics from `Local: 0 played · 0 won` to
   `Local: 1 played · 0 won`. Undo with Space and redo with Enter restored the
   expected states without double-counting the deal.
4. The mutation created mode-0600 game and profile saves. A clean close and
   relaunch reopened the 23-card state and one-played profile. An empty
   foundation action remained non-mutating and exposed `That foundation is
   empty` visually and through AT-SPI.
5. A malformed profile whose wins exceeded played deals was quarantined. The
   recovery status remained fully available through AT-SPI, the app stayed
   usable, the quarantine bytes matched the input, and the next draw created a
   valid fresh profile.
6. Two installed processes sharing a private data root produced a real stale
   ownership conflict. The stale process retained its game/profile changes,
   exposed retry, reload, and discard controls, refused a close request while
   dirty, and closed after keyboard activation of Reload disk copy cleared the
   guard.
7. The plugin was installed and enabled with:

   ```sh
   omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --enable --yes
   ```

   Its clean checkout was the exact revision and tree above, and the live
   registry reported it enabled.
8. The running Quickshell host accepted:

   ```sh
   omarchy-shell shell summon io.github.rohan-patnaik.solitaire '{}'
   ```

   IPC returned `ok` in 70 ms, the exact installed binary mapped, and
   Quickshell remained responsive.
9. During `omarchy restart shell`, Quickshell changed from PID 1303 to 816413
   in 1,066 ms. The detached Solitaire process remained PID 815157, stayed
   mapped and usable, and the restarted registry still reported the plugin
   enabled. A compositor close then ended the app cleanly.
10. `~/.local/share/solitaire` was absent before and after the launcher-only
    summon/restart/close pass, so normal user game data was not created or
    changed.

The original acceptance bundle, including screenshots, command output, and
fixture hashes, is retained outside the repository at
`/home/rohan/Documents/Codex/2026-08-22/solitaire-profile-wayland-acceptance-1595fb0/`.

## Remaining gaps

- Orca was unavailable. Real keyboard focus and AT-SPI names, roles, states,
  actions, and changing live-region names were observed, but spoken screen
  reader output was not captured.
- Long malformed/conflict status text is visually clipped at the default
  1180-pixel width. AT-SPI exposes the full text and recovery controls remain
  visible, but an end-user visual error-details surface is still needed.
- No complete deal was won through the installed UI, so the live
  `deals_won` transition remains unverified.
- Named profiles, achievements, streaks, import/export, synchronization,
  multi-process merge, cross-file transactions, crash injection, and
  power-loss recovery remain absent or unverified.
- The installed-binary success path and shell-restart survival were exercised.
  Live missing-binary and immediate-startup-failure notifications were not
  induced because replacing the package-owned binary would be destructive.
  Automated deployment tests cover their contract, but live evidence remains
  missing.

Accordingly, the launcher, local profile, accessibility, persistence safety,
and real-Omarchy capabilities remain Partial.
