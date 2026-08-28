# Publication record for a72f3ce

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for bounded Klondike redeals and their display-independent
reopen mapping. It does not claim installed interaction with those controls.

## Immutable source and review

- Commit: `a72f3ce093fbabfd02a64bdc6680b0fff30652c1`
- Parent: `57b8dc6e223f0fdf9590ae7893c84253bfa168dc`
- Tree: `fc49a07d0795f8ec52c58feb84cb26e55504e675`
- An independent read-only GPT-5.6-Sol Extra High review inspected that exact
  commit and signed off with no actionable findings. It verified the shared
  production/test option mapping, pending-selection no-write behavior, explicit
  Slint value/index bindings, documentation limits, and clean checkout.

The parent run `33128045066` failed only when a component test tried to create a
Winit backend without Wayland or X11. The remediation tests the same production
mapping without a display server; the failed parent run is not publication
evidence.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33128758547`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33128758547/job/98713067551`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33128758547/job/98713067274`
- Both jobs succeeded for the full commit. Rust passed formatting, package
  checksums, generated-catalog drift, Clippy with warnings denied, 188 tests,
  and release build. The clean Arch container built, tested, installed, and
  queried `solitaire-omarchy 0.1.0.r0.ga72f3ce-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin from `5f40bdcabe87db420f15ab34d71aa26ff5f9e3bb`
to the full commit above. Validation passed, the installed tree was
`fc49a07d0795f8ec52c58feb84cb26e55504e675`, the checkout was clean, and the
origin remained `https://github.com/rohan-patnaik/Solitaire.git`. The plugin
remained `enabled:false` and `active:false`, and no Solitaire process existed.
No plugin layer or native window was summoned, focused, cycled, or displayed.

The exact-package redeal selection, visible remaining-count, keyboard, and
AT-SPI gates remain open as recorded in `KLONDIKE_REDEAL_LIMIT_ACCEPTANCE.md`.
