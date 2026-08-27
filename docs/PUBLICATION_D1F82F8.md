# Publication record for d1f82f8

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the normal Pyramid complete-deal candidate. It does not
claim that the installed final pair was exercised.

## Immutable source and review

- Commit: `d1f82f8eb90c29ce25c80963083634dd6e1a105e`
- Parent: `860413721e62d8967a938a04562491919b219ab5`
- Tree: `6fb553cced72ce63f1cfd4ff3b28e638d88b4d89`
- An independent read-only GPT-5.6-Sol Extra High review verified the normal
  legal replay, final controller selection/pair route, atomic persistence,
  profile idempotence, documentation, catalog, and package contracts at that
  exact clean commit. It reported no actionable findings and made no checkout
  changes.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33121031503`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33121031503/job/98687625682`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33121031503/job/98687625950`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog drift, Clippy with warnings denied, 176 tests, and release
  build. The clean Arch container built, tested, installed, and verified
  `solitaire-omarchy 0.1.0.r0.gd1f82f8-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `860413721e62d8967a938a04562491919b219ab5`
to the full commit above. Validation passed, the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`, and the plugin remained
`enabled:false` and `active:false`. No plugin layer or Solitaire process was
summoned, focused, cycled, or displayed.

The installed keyboard/AT-SPI final-transition gate remains open as recorded in
`PYRAMID_COMPLETE_DEAL_ACCEPTANCE.md`.
