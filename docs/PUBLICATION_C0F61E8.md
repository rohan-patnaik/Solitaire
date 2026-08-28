# Publication record for c0f61e8

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the shared restart-current-deal workflow. It does not
claim installed restart input or foreground acceptance.

## Immutable source and review

- Published commit: `c0f61e85126072f74a10ffea1fcd831c8b3e3d34`
- Parent before the slice: `478a10a9aed6751c1cd9b90b0122d85faad021dd`
- Published tree: `a72922d69d82c235cb8fe6712187b66dc49c0e49`
- Two independent read-only GPT-5.6-Sol Standard-mode workers reviewed the
  exact replacement candidate in isolated clones. The code/security/evidence
  and acceptance/package/accessibility workers both returned PASS with zero
  actionable findings.
- The first candidate was not published. Both workers identified that it
  dropped the valid persisted Klondike `timed` option during restart. The
  implementation was corrected, a progressed timed-deal regression was added,
  and both complete review gates were repeated against the replacement SHA.
- Neither worker edited, published, installed on the host, invoked the
  production binary, or launched a GUI.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33150141503`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33150141503/job/98779976691`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33150141503/job/98779976934`
- Both jobs succeeded for the full commit. Rust passed formatting, checksum and
  generated-catalog checks, Clippy with warnings denied, 214 tests, and release
  build. The clean Arch container built, ran the same 214 tests, installed,
  verified the source marker, and queried
  `solitaire-omarchy 0.1.0.r0.gc0f61e8-1`.

## Supported Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin update io.github.rohan-patnaik.solitaire --yes
```

advanced the installed plugin to the published commit. Manifest validation
passed, the installed tree was `a72922d69d82c235cb8fe6712187b66dc49c0e49`,
and the checkout was clean. The unchanged shell configuration kept the plugin
`enabled:false`; `omarchy-shell` was stopped, so it remained `active:false`.
No Solitaire process existed before or after the update. Nothing was enabled,
summoned, focused, cycled, or restarted.

The authoritative `Stuff` mount was unavailable at the final gate. It was not
mounted, repaired, or modified, and the deferred numbered-deal work remained
untouched. Exact-package restart input and foreground-preservation acceptance
remain open as recorded in `RESTART_CURRENT_DEAL_ACCEPTANCE.md`.
