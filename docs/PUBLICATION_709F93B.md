# Publication record for 709f93b

This record closes the review, forward CI remediation, exact-tip CI, package,
and supported plugin publication gates for exposed Klondike card double-click.
It does not claim installed pointer timing, touch, AT-SPI, spoken output, or
foreground acceptance.

## Immutable source and review

- Published feature commit: `7feccc63f8ab05eca1fda948fd72061c9b2fca71`
- Published forward remediation and final tip:
  `709f93ba55680ecbafb332ee95c345d6aa8ad016`
- Parent before the slice: `4e80b44b4ea3d4820fa5e38f1e8e71aa4e33386a`
- Final published tree: `2d92490fa78526a7f8631e6536d83775febb9460`
- The initial feature-tip CI exposed a fontless-container test-platform
  initialization failure. The published remediation removed that unnecessary
  test initialization and retained the real Slint `SingleShot` timer behind a
  zero-delay test seam; no history was rewritten.
- Two independent read-only GPT-5.6-Sol Standard-mode workers reviewed the
  exact final candidate in fresh isolated clones. The Extra High
  code/security/evidence and High/Extra High acceptance/package/accessibility
  workers both returned PASS with zero actionable findings.
- Neither worker edited, published, installed on the host, invoked the
  production binary, or launched a GUI.

## Exact-tip CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33179782126`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33179782126/job/98877740992`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33179782126/job/98877741273`
- Both jobs succeeded for the full final commit. The exact-tip suite contained
  232 tests, and the clean Arch job built, tested, installed, verified the
  source marker, and queried `solitaire-omarchy 0.1.0.r0.g709f93b-1`.
- The read-only acceptance worker independently built that exact package. Its
  source archive SHA-256 was
  `5ca7f38fa00068f8664916eb259d0f5c7e410da5e07e1dba9e7f0ee28dfab420`,
  package SHA-256 was
  `a80f3c39aa2b4c7ca9915f75291b4e35d78b24eb768e3acb543ee7bcb1a39863`,
  and binary SHA-256 was
  `7f9c5538b6c9c317ec7b90e3638267512069d1154d4dd40b541d29ae17078882`.
  A fake-root install/query verified nine package files, no missing files,
  executable mode 0755, and license/source-marker mode 0644.

## Supported Omarchy publication

After exact-tip CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the final commit and tree above. Manifest
validation passed and the installed checkout was clean. Shell configuration
hash `ef50cdc9e7619de4f1722c87bfec7b4ab1c4f83984eb0f28c4520ba9d0874a40`
was unchanged, so the plugin remained `enabled:false`; `omarchy-shell` remained
stopped and the plugin remained `active:false`. No Solitaire process existed.
Nothing was enabled, summoned, focused, cycled, or restarted.

The authoritative `Stuff` mount was unavailable and untouched. The clean
preserved baseline remained at `478a10a9aed6751c1cd9b90b0122d85faad021dd`,
and deferred numbered-deal work remained untouched.
