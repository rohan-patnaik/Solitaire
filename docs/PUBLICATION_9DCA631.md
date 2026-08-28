# Publication record for 9dca631

This record closes the remediation review, exact-SHA CI, package, and supported
plugin publication gates for the bounded Spider complete-deal Controller restart
lifecycle. It does not claim installed final-action or process/window acceptance.

## Immutable source and review

- Commit: `9dca631ad3ae5b3f6ca3fb1b35c355a259539c3b`
- Parent: `b4224998eb94b5b81edf65bfdc9fd29c89becaa5`
- Tree: `ba97d08268923f037a1c32e9cc48d81b3a2442e7`
- An independent read-only GPT-5.6-Sol Extra High review of the parent found two
  actionable P2 issues: an unbounded child wait and ambient phase variables that
  could reach user data. The remediation added a ten-second monotonic deadline
  with kill-and-reap behavior, bounded 8 KiB diagnostics, and a canonical
  temporary root plus PID/nanosecond nonce marker that each child verifies before
  constructing `Controller`.
- The same independent reviewer inspected this exact remediation commit, reran
  focused lifecycle and negative ambient-phase tests, and signed off with no
  actionable findings or edits.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33136360301`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33136360301/job/98737098834`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33136360301/job/98737098996`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 200 tests, and release
  build. The clean Arch container built, tested, installed, and queried
  `solitaire-omarchy 0.1.0.r0.g9dca631-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the full commit above. Validation passed, the
installed tree was `ba97d08268923f037a1c32e9cc48d81b3a2442e7`, the checkout
was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed before or
after the update. No plugin layer or native window was summoned, focused,
cycled, or displayed.

The exact-package Spider final-action and process/window identity gates remain
open as recorded in `SPIDER_COMPLETE_DEAL_ACCEPTANCE.md`.
