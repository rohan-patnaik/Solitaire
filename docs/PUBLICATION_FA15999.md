# Publication record for fa15999

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for bounded Pyramid redeal selection. It does not claim live
interaction with the selector.

## Immutable source and review

- Commit: `fa15999d04876160337bd13c0126b20e78873132`
- Parent: `e1b3b5fbd4ef10ab92b5756522b34717c0d4d3d8`
- Tree: `e13313983abac6b03e8972502bbab2ec4241cf80`
- An independent read-only GPT-5.6-Sol Extra High review inspected the complete
  two-commit slice. After its catalog and API-wording findings were remediated,
  it signed off this exact tip with no actionable findings and made no edits.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33133432510`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33133432510/job/98728013394`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33133432510/job/98728013604`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 195 tests, and release
  build. The clean Arch container built, tested, installed, and queried
  `solitaire-omarchy 0.1.0.r0.gfa15999-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the full commit above. Validation passed, the
installed tree was `e13313983abac6b03e8972502bbab2ec4241cf80`, the checkout
was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed.
No plugin layer or native window was summoned, focused, cycled, or displayed.

The exact-package selector, used/maximum display, final transition, and AT-SPI
gates remain open as recorded in `PYRAMID_REDEAL_LIMIT_ACCEPTANCE.md`.
