# Publication record for 5f40bdc

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the normal Klondike complete-deal candidate. It does not
claim that the installed final foundation move was exercised.

## Immutable source and review

- Commit: `5f40bdcabe87db420f15ab34d71aa26ff5f9e3bb`
- Parent: `5f81b4b1a76ddc99636a895ca5900059bd523299`
- Tree: `ddab109bc00743eeb5c60f4fc4339a39e1f1c452`
- An independent read-only GPT-5.6-Sol Extra High review verified the fixture,
  production replay reconstruction, final controller route, persistence and
  profile lifecycle, documentation/catalog limits, and prior publication
  facts. It signed off the exact commit with no actionable findings and made
  no checkout changes.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33125817274`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33125817274/job/98703592996`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33125817274/job/98703593128`
- Both jobs succeeded for the full commit. Rust passed formatting, package
  checksums, generated-catalog drift, Clippy with warnings denied, 184 tests,
  and release build. The clean Arch container built, tested, installed, and
  queried `solitaire-omarchy 0.1.0.r0.g5f40bdc-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `5f81b4b1a76ddc99636a895ca5900059bd523299`
to the full commit above. Validation passed, the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`, and the plugin remained
`enabled:false` and `active:false`. No plugin layer or Solitaire process was
summoned, focused, cycled, or displayed.

The installed keyboard/AT-SPI final-transition gate remains open as recorded in
`KLONDIKE_COMPLETE_DEAL_ACCEPTANCE.md`.
