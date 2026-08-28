# Publication record for 4e80b44

This record closes the review, exact-SHA CI, package, and supported plugin
publication gates for the session-scoped left-handed Klondike layout. It does
not claim installed selector input, visual mirroring, or foreground acceptance.

## Immutable source and review

- Published commit: `4e80b44b4ea3d4820fa5e38f1e8e71aa4e33386a`
- Parent before the slice: `d3c384a188bd1281291bc02acf16ebbef8077849`
- Published tree: `cb4756907d2791a1975f550c7dcd438aa10e9a6f`
- Two independent read-only GPT-5.6-Sol Standard-mode workers reviewed the
  exact candidate in fresh isolated no-local clones. The Extra High
  code/security/evidence and High/Extra High acceptance/package/accessibility
  workers both returned PASS with zero actionable findings.
- Neither worker edited, published, installed on the host, invoked the
  production binary, or launched a GUI.

## Exact-SHA CI and package

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33163743373`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33163743373/job/98824121006`
- Arch package job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33163743373/job/98824120717`
- Both jobs succeeded for the full commit. Rust passed formatting, hostile
  checksum and generated-catalog checks, Clippy with warnings denied, all 225
  tests, and the release build. The clean Arch container built, ran the same
  tests, installed, verified the source marker, and queried
  `solitaire-omarchy 0.1.0.r0.g4e80b44-1`.
- The read-only acceptance worker independently built that exact package. Its
  source archive SHA-256 was
  `a344226178f9a33a89fb492883253cb7328cee3b151cbf2291936b28dd7762a7`,
  package SHA-256 was
  `5467caed810b1e903eed151ec57655f4bc56204cfaf2a40f889da13c8ef34f77`,
  and binary SHA-256 was
  `93bad167489d12b46200fe4a24b0be6371749a7db9e1275df9008ec177e56fea`.
  A fake-root install/query verified the full revision marker, nine package
  files, no missing files, executable mode 0755, and marker mode 0644.

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
and deferred numbered-deal work remained untouched. Exact-package layout input,
visual mirroring, AT-SPI/spoken output, and foreground-preservation acceptance
remain open as recorded in `KLONDIKE_LEFT_HANDED_ACCEPTANCE.md`.
