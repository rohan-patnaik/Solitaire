# Publication record for 720fab0

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the bounded TriPeaks complete-deal Controller restart
lifecycle. It does not claim installed final-action or process/window
acceptance.

## Immutable source and review

- Published commit: `720fab04ab3528d1e8e66768ebf47a85dc2f94b1`
- Parent before the slice: `0c806cbe8d26ed71bbef888620a5a77cbeaa12e1`
- Published tree: `96322866fe5d8c33126f594f0dd7f9590463adea`
- An independent read-only GPT-5.6-Sol Extra High reviewer inspected the exact
  commit and the focused lifecycle, hostile ambient-environment rejection,
  cleanup, full suite, catalog, and package evidence. It signed off with no
  actionable findings or edits.
- The exact checkout was clean before and after review. No restart root or
  Solitaire process remained after the gates.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33141845041`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33141845041/job/98754184672`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33141845041/job/98754184768`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 207 tests, and release
  build. The clean Arch container built, ran the same 207 tests, installed, and
  queried `solitaire-omarchy 0.1.0.r0.g720fab0-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the published commit. Manifest validation
passed, the installed tree was `96322866fe5d8c33126f594f0dd7f9590463adea`,
the checkout was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed before or
after the update. No plugin layer or native window was summoned, focused,
cycled, or displayed.

The exact-package TriPeaks final-action and process/window identity gates remain
open as recorded in `TRIPEAKS_COMPLETE_DEAL_ACCEPTANCE.md`.
