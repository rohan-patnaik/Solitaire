# Publication record for 0c806cb

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the bounded FreeCell complete-deal Controller restart
lifecycle. It does not claim installed final-action or process/window
acceptance.

## Immutable source and review

- Published commit: `0c806cbe8d26ed71bbef888620a5a77cbeaa12e1`
- Parent before the slice: `d23382b9ec62c7e18dcec9b84f13bb16072338b4`
- Published tree: `5d3bc88b77952ab7057e2311dc28434dd2ffe646`
- An independent read-only GPT-5.6-Sol Extra High reviewer inspected the exact
  commit, independently exercised the focused lifecycle, unwind cleanup,
  hostile ambient-environment rejection, and the full 205-test suite, and
  signed off with no actionable findings or edits.
- The reviewer verified the exact SHA, tree, parent, and clean checkout both
  before and after review. No restart roots or Solitaire processes remained.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33138972312`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33138972312/job/98745289144`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33138972312/job/98745289065`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 205 tests, and release
  build. The clean Arch container built, ran the same 205 tests, installed, and
  queried `solitaire-omarchy 0.1.0.r0.g0c806cb-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the published commit. Manifest validation
passed, the installed tree was `5d3bc88b77952ab7057e2311dc28434dd2ffe646`,
the checkout was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed before or
after the update. No plugin layer or native window was summoned, focused,
cycled, or displayed.

The exact-package FreeCell final-action and process/window identity gates remain
open as recorded in `FREECELL_COMPLETE_DEAL_ACCEPTANCE.md`.
