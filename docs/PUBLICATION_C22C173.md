# Publication record for c22c173

This record closes the build, review, CI, and repository-defined plugin
publication gates for the Klondike draw/scoring selector slice. It does not
claim exact-package Wayland acceptance for the Vegas workflow.

## Immutable source and review

- Commit: `c22c1737d04cfd7cb8fa09b8c65caebec2206eec`
- Parent: `0b6325f847cb72cf8ed9c0e9d2641525380c6464`
- Tree: `bc8b1e621104d23b9fb3fc2a646cfda74882edf5`
- An independent read-only GPT-5.6-Sol Extra High review of that exact commit
  signed off with no actionable findings after remediation of strict option
  parsing, seed atomicity, recovery-row layout, and evidence provenance.

## Exact-SHA CI and package gate

- GitHub Actions run:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33112983537`
- Rust job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33112983537/job/98660344058`
- Exact-revision Arch job:
  `https://github.com/rohan-patnaik/Solitaire/actions/runs/33112983537/job/98660344289`
- Both jobs completed successfully for the full commit above. The Rust job
  passed formatting, package checksum drift, capability-catalog drift, Clippy
  with warnings denied, 161 tests, and a release build.
- The clean Arch container built, tested, and installed the exact source as
  `solitaire-omarchy 0.1.0.r0.gc22c173-1`; its installed source marker was
  required by the workflow to equal the full GitHub SHA.

This CI pass does not replace the separately recorded live package-file and
binary hashes at `d20ba41`, and it does not establish package reproducibility,
signing, or an SBOM.

## Repository-defined Omarchy publication

After CI was green, the supported command

```sh
omarchy plugin add https://github.com/rohan-patnaik/Solitaire.git --yes
```

installed `io.github.rohan-patnaik.solitaire` at the exact full commit. A
subsequent `omarchy plugin validate` passed, the installed origin remained the
HTTPS repository URL, and `omarchy plugin list --json` reported
`enabled:false` and `active:false`. No plugin layer or Solitaire process was
summoned for this publication check.

## Source-build Wayland boundary

A single exact-tree release process rendered natively on Wayland at 1180x820.
The dedicated clean Klondike option-row capture had SHA-256
`c29c1cfc0c7e4a2664450524c146e1c346c4c216bd1080bcd6257787326a229b`.
The dirty/pending recovery capture, taken after removing a stale unrelated test
process, had SHA-256
`b5f0a0916ac9a6627ae81eb36b9ce65e926fa70c5e87faf0e28f3e677f5e6eac`.
The latter showed the complete recovery row, the separate disabled option row,
and the board fitting at 1180x820. AT-SPI exposed names and default actions for
the two selectors and new-deal control. A forced save failure kept progress in
memory; retry saved and committed the pending deal, producing regular 0600
JSON files. The temporary toolkit-accessibility setting was restored.

These observations used the exact source-built release binary, not the
installed Arch package. Keyboard focus evidence affected by a compositor
overlay was discarded and is not cited. Exact-package Vegas selection,
keyboard traversal, save/reopen, and accessibility acceptance therefore remain
open.
