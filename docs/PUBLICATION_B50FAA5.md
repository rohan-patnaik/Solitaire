# Publication record for b50faa5

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the optional TriPeaks Ace-King-wrap rule. It does not
claim installed interaction with the rule selector or final move.

## Immutable source and review

- Commit: `b50faa54c520f49ea27a478786b640b91c8ca9f1`
- Parent: `a72f3ce093fbabfd02a64bdc6680b0fff30652c1`
- Tree: `9209f7b73fac79529ec680aa1d3abe74fd03dd0a`
- An independent read-only GPT-5.6-Sol Extra High review inspected that exact
  commit and signed off with no actionable findings. It verified strict rule
  parsing, atomic pending-deal behavior, save/reopen and history preservation,
  hostile-input evidence, the UI mapping contract, and compatibility limits.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33130626248`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33130626248/job/98719044292`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33130626248/job/98719044070`
- Both jobs succeeded for the full commit. Rust passed formatting, package
  checksums, generated-catalog drift, Clippy with warnings denied, 191 tests,
  and release build. The clean Arch container built, tested, installed, and
  queried `solitaire-omarchy 0.1.0.r0.gb50faa5-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the full commit above. Validation passed, the
installed tree was `9209f7b73fac79529ec680aa1d3abe74fd03dd0a`, the checkout
was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed.
No plugin layer or native window was summoned, focused, cycled, or displayed.

The exact-package rule selection, Ace/King input, and final-transition gates
remain open as recorded in `TRIPEAKS_WRAPAROUND_ACCEPTANCE.md`.
