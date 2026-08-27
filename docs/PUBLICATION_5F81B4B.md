# Publication record for 5f81b4b

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the normal FreeCell complete-deal candidate. It does not
claim that the installed final foundation move was exercised.

## Immutable source and review

- Commit: `5f81b4b1a76ddc99636a895ca5900059bd523299`
- Parent: `d1f82f8eb90c29ce25c80963083634dd6e1a105e`
- Tree: `6315af1ad8762c8861ce5c7915ed11464ef47df3`
- An independent read-only GPT-5.6-Sol Extra High review verified the legal
  replay, controller lifecycle, persistence/profile behavior, docs/catalog,
  and prior publication facts. It identified one fixture-integrity gap; the
  deployment contract was amended to hash and structurally pin the actual
  fixture and reject state/profile injection at both envelope levels. Repeat
  review signed off the amended exact commit with no actionable findings and
  made no checkout changes.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33123378335`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33123378335/job/98695558469`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33123378335/job/98695558244`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog drift, Clippy with warnings denied, 180 tests, and release
  build. The clean Arch container built, tested, installed, and verified
  `solitaire-omarchy 0.1.0.r0.g5f81b4b-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `d1f82f8eb90c29ce25c80963083634dd6e1a105e`
to the full commit above. Validation passed, the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`, and the plugin remained
`enabled:false` and `active:false`. No plugin layer or Solitaire process was
summoned, focused, cycled, or displayed.

The installed keyboard/AT-SPI final-transition gate remains open as recorded in
`FREECELL_COMPLETE_DEAL_ACCEPTANCE.md`.
