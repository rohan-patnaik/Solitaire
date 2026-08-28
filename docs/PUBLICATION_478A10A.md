# Publication record for 478a10a

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the bounded Pyramid complete-deal Controller restart
lifecycle. It does not claim installed final-action or process/window
acceptance.

## Immutable source and review

- Published commit: `478a10a9aed6751c1cd9b90b0122d85faad021dd`
- Parent before the slice: `720fab04ab3528d1e8e66768ebf47a85dc2f94b1`
- Published tree: `a08ac5814c3a352a536883019267af2fff3fc040`
- An independent read-only GPT-5.6-Sol Extra High reviewer inspected the exact
  commit and returned PASS with no actionable findings. Focused lifecycle,
  catalog, and deployment-contract checks passed and the checkout stayed clean.
- Procedural disclosure: while locating the hashed test harness, that reviewer
  briefly invoked a non-test Solitaire artifact despite the no-production-launch
  instruction. It was terminated immediately; the reviewer confirmed no process
  or residue remained and completed the review through exact test commands. No
  GUI observation was used to inflate capability status.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33144862140`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33144862140/job/98763550798`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33144862140/job/98763550953`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 209 tests, and release
  build. The clean Arch container built, ran the same tests, installed, verified
  the source marker, and queried
  `solitaire-omarchy 0.1.0.r0.g478a10a-1`.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the published commit. Manifest validation
passed, the installed tree was `a08ac5814c3a352a536883019267af2fff3fc040`,
and the checkout was clean. The plugin remained `enabled:false` and
`active:false`, and no Solitaire process existed before or after the update.
Nothing was enabled, summoned, focused, cycled, or restarted.

The authoritative `Stuff` mount was unavailable at the final gate. It was not
mounted, repaired, or modified, and the deferred numbered-deal work remained
untouched. Exact-package Pyramid final-action and process/window identity remain
open as recorded in `PYRAMID_COMPLETE_DEAL_ACCEPTANCE.md`.
