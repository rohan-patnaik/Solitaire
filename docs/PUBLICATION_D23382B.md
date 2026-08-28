# Publication record for d23382b

This record closes the review, remediation, exact-SHA CI, package, and supported
plugin publication gates for the bounded Klondike complete-deal Controller
restart lifecycle. It does not claim installed final-action or process/window
acceptance.

## Immutable source and review

- Candidate commit: `2bf6aa7d4529370051f83552646422a56d020498`
- Published remediation commit: `d23382b9ec62c7e18dcec9b84f13bb16072338b4`
- Parent before the slice: `9dca631ad3ae5b3f6ca3fb1b35c355a259539c3b`
- Published tree: `1ecf716d36752ff96415032d2f08c0747d023fa0`
- An independent read-only GPT-5.6-Sol Extra High review found one actionable P2
  in the candidate: failure paths could leave task-owned restart roots and
  unexpected quarantine residue. The remediation introduced an ownership-
  checked scoped guard, complete recursive cleanup on success or unwind, a
  visible successful-path cleanup failure, and focused unexpected-residue
  coverage.
- The same independent reviewer inspected the exact remediation commit,
  independently exercised positive, unwind, hostile ambient-environment, and
  full-suite paths, and signed off with no actionable findings or edits.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33137835181`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33137835181/job/98741750892`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33137835181/job/98741750767`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 203 tests, and release
  build. The clean Arch container built, tested, installed, and queried
  `solitaire-omarchy 0.1.0.r0.gd23382b-1` at the exact source marker.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the published commit. Validation passed, the
installed tree was `1ecf716d36752ff96415032d2f08c0747d023fa0`, the checkout
was clean, and the origin remained
`https://github.com/rohan-patnaik/Solitaire.git`. The plugin remained
`enabled:false` and `active:false`, and no Solitaire process existed before or
after the update. No plugin layer or native window was summoned, focused,
cycled, or displayed.

The exact-package Klondike final-action and process/window identity gates remain
open as recorded in `KLONDIKE_COMPLETE_DEAL_ACCEPTANCE.md`.
