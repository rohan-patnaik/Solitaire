# Publication record for d9d0498

This record closes the build, review, exact-SHA CI, and supported plugin-update
gates for explicit FreeCell numbered deals. It does not claim installed input
acceptance, which remains open under the desktop-safe boundary.

## Immutable source and independent review

- Commit: `d9d0498d8854fb9268b6105c2a767710a63e40e6`
- Parent: `c22c1737d04cfd7cb8fa09b8c65caebec2206eec`
- Tree: `3637242543b9791bc5e81114dc38ddde11db9e51`
- An independent read-only GPT-5.6-Sol Extra High review first found and then
  verified remediation of sequence corruption after restart, truncated `u64`
  display, missing deal-zero coverage, stale CI metadata, and cross-cutting
  evidence gaps. It signed off the exact amended commit with no actionable
  findings and did not modify the checkout.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33116183706`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33116183706/job/98671301926`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33116183706/job/98671302227`
- Both jobs succeeded for the full commit. The Rust job passed formatting,
  checksum and generated-catalog drift, Clippy with warnings denied, 168 tests,
  and a release build. The clean Arch container built, tested, installed, and
  verified `solitaire-omarchy 0.1.0.r0.gd9d0498-1` at the exact source marker.

## Supported Omarchy update

After CI was green, the documented command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `c22c1737d04cfd7cb8fa09b8c65caebec2206eec`
to the full commit above. Validation passed, the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`, and the plugin list still
reported `enabled:false` and `active:false`. No plugin layer or Solitaire
process was summoned, focused, or cycled.

The source-only background/nested-compositor boundary and the skipped input
interaction are recorded in `FREECELL_NUMBERED_DEAL_ACCEPTANCE.md`. Exact
installed numbered-entry input, keyboard traversal, and AT-SPI action
acceptance therefore remain open.
