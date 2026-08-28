# Publication record for 2ebbe7e

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for Spider suit-selector synchronization. It does not claim
live interaction with the selector or complete-deal workflow.

## Immutable source and review

- Commit: `2ebbe7edaa0beb04588ead7897e38ecd35a70648`
- Parent: `fa15999d04876160337bd13c0126b20e78873132`
- Tree: `a67f4a0a7b0c5d3bc149558b8ea00322c22971f5`
- An independent read-only GPT-5.6-Sol Extra High review inspected the exact
  commit, reran 198 tests plus checksum and generated-catalog checks, and
  signed off with no actionable findings or edits.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33134886017`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33134886017/job/98732525776`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33134886017/job/98732525616`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 198 tests, and release
  build. The clean Arch container built, tested, installed, and queried
  `solitaire-omarchy 0.1.0.r0.g2ebbe7e-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the full commit above. Validation passed, the
installed tree was `a67f4a0a7b0c5d3bc149558b8ea00322c22971f5`, the checkout
was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed before or
after the update. No plugin layer or native window was summoned, focused,
cycled, or displayed.

The exact-package selector and complete-deal gates remain open as recorded in
`SPIDER_SUIT_SELECTOR_ACCEPTANCE.md` and `SPIDER_COMPLETE_DEAL_ACCEPTANCE.md`.
