# Publication record for 8604137

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the normal TriPeaks complete-deal candidate. It does not
claim that the installed final action was exercised.

## Immutable source and review

- Commit: `860413721e62d8967a938a04562491919b219ab5`
- Parent: `d9d0498d8854fb9268b6105c2a767710a63e40e6`
- Tree: `36a188bec20487dc914e610eb46f87bb409f25e8`
- An independent read-only GPT-5.6-Sol Extra High review verified the legal
  replay and controller lifecycle, required removal of an unreproducible search
  provenance claim, and corrected the installed gate from engine index zero to
  accessible tableau position one. It then signed off the amended exact commit
  with no actionable findings and made no checkout changes.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33118432350`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33118432350/job/98678969667`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33118432350/job/98678969895`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog drift, Clippy with warnings denied, 172 tests, and release
  build. The clean Arch container built, tested, installed, and verified
  `solitaire-omarchy 0.1.0.r0.g8604137-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `d9d0498d8854fb9268b6105c2a767710a63e40e6`
to the full commit above. Validation passed, the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`, and the plugin remained
`enabled:false` and `active:false`. No plugin layer or Solitaire process was
summoned, focused, cycled, or displayed.

The installed keyboard/AT-SPI final-transition gate remains open as recorded in
`TRIPEAKS_COMPLETE_DEAL_ACCEPTANCE.md`.
