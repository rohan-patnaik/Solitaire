# Publication record for d3c384a

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the resilient timed-Klondike workflow. It does not claim
installed timing input, visual progression, or foreground acceptance.

## Immutable source and review

- Published commit: `d3c384a188bd1281291bc02acf16ebbef8077849`
- Parent before the slice: `c0f61e85126072f74a10ffea1fcd831c8b3e3d34`
- Published tree: `dcbb1877f18b4f39880afbfb121991c9ef0be96c`
- Two independent read-only GPT-5.6-Sol Standard-mode workers reviewed the
  exact candidate in fresh isolated no-local clones. The Extra High
  code/security/evidence and High/Extra High acceptance/package/accessibility
  workers both returned PASS with zero actionable findings.
- Neither worker edited, published, installed on the host, invoked the
  production binary, or launched a GUI.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33155179947`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33155179947/job/98796148931`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33155179947/job/98796149278`
- Both jobs succeeded for the full commit. Rust passed formatting, hostile
  checksum and generated-catalog checks, Clippy with warnings denied, all 222
  tests, and the release build. The clean Arch container built, ran the same
  tests, installed, verified the source marker, and queried
  `solitaire-omarchy 0.1.0.r0.gd3c384a-1`.
- The read-only acceptance worker independently built that exact package. Its
  source archive SHA-256 was
  `07b5a5d1ff268e2cf67ac660b94eb642cc37bb123e1c0f437e4f3d3e71ff176c`,
  package SHA-256 was
  `38dd27d4b5aee221c7adbd40843fb08d13300a2b197667a9cc1e163412b58b3e`,
  and binary SHA-256 was
  `5402122009b5ee92700f3dd95fe9351b1802c818d9ca09b14d489da669c3cb40`.

## Supported Omarchy publication

After exact-tip CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the full commit and tree above. Manifest
validation passed and the installed checkout was clean. Shell configuration
hash `a63610dae3cefcf90090639d0f22e8dab49330f020092b6df4cedea74a648ec9`
was unchanged, so the plugin remained `enabled:false`; `omarchy-shell` remained
stopped and the plugin remained `active:false`. No Solitaire process existed.
Nothing was enabled, summoned, focused, cycled, or restarted.

The authoritative `Stuff` mount was unavailable and untouched. The clean
preserved baseline remained at `478a10a9aed6751c1cd9b90b0122d85faad021dd`,
and deferred numbered-deal work remained untouched. Exact-package timing input,
AT-SPI/spoken output, and foreground-preservation acceptance remain open as
recorded in `KLONDIKE_TIMED_PLAY_ACCEPTANCE.md`.
