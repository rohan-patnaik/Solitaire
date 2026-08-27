# FreeCell numbered-deal acceptance boundary

This record distinguishes automated workflow coverage and desktop-safe source
rendering from the exact-package input gate that remains open.

## Automated candidate evidence

- `exact_freecell_deal_is_strict_atomic_reopenable_and_does_not_consume_next_deal`
  covers the minimum and maximum `u64` range, malformed and 4,096-byte hostile
  input, source/save/counter preservation, dirty pending confirmation, atomic
  save/reopen, and independence from the durable next-deal sequence.
- `exact_freecell_deal_restart_preserves_the_durable_next_sequence` starts a
  fresh Controller in an isolated subprocess after saving the maximum exact
  deal, then proves that **Next deal** still opens and advances the separately
  persisted sequence.
- `freecell_deal_number_preserves_full_u64_range` pins the displayed string at
  both `0` and `18446744073709551615` without an integer-width truncation.
- `freecell_deal_entry_is_scoped_to_freecell` rejects the entry route outside
  FreeCell without changing the active game, counters, or pending request.
- `freecell_numbered_deal_entry_declares_keyboard_and_accessibility_contracts`
  pins the text field's numeric input type, accessible name, Enter callback,
  buttons, pending-state disabling, and dedicated row ordering.
- The full local gate passed Clippy with warnings denied, 168 tests, a release
  build, catalog/checksum drift checks, and Omarchy plugin validation.

The resulting source release binary was 22,827,296 bytes with SHA-256
`c90dd474c0d3cbbabe03fe105031d1713e316041f2e58db37bc191cb7d682e8b`.

## Desktop-safe runtime observation

The release binary was launched only through
`/home/rohan/.local/bin/codex-background-launch`. Hyprland reported one native
Wayland client (`xwayland=false`) at 1180x820 on the hidden
`codex-background` workspace. The active foreground-window record was
byte-identical before launch, after launch, and after cleanup; each copy had
SHA-256 `9d358022af547259a531ba5083b4fe1e1fd8300abfbec1c46f4512ce882ac9b4`.
The process was terminated without focusing or cycling it.

A separate Weston 15.0.1 headless compositor used its Pixman renderer, fake
seat, isolated runtime/data roots, and an 1180x820 output. The source release
started successfully there and exposed an AT-SPI application/frame. Weston's
headless compositor did not expose the virtual-keyboard protocol required by
`wtype`, so changing the game picker and entering a deal could not be exercised
without a different input-capable nested compositor. That interaction was
skipped rather than sending input to the user's active desktop. The Weston log
had SHA-256 `beac885a1d0fca2d911e2411f1e2450da5a4d39fa649a36aac8bc7366ce2f2fb`.
The temporary toolkit-accessibility setting was restored to its original
`false` value, both test processes exited, and the foreground window remained
unchanged.

## Remaining exact-package gate

After a reviewed commit is published and both exact-SHA CI jobs are green, an
input-capable offscreen compositor or an explicitly authorized non-foreground
method must exercise the installed package. Enter `0`, `42`, and
`18446744073709551615`; require the exact visible deal, strict invalid-input
status, dirty/pending cancel and retry behavior, 0600 save/reopen, unchanged
next-deal counter for explicit openings, keyboard focus/Enter activation, and
the expected AT-SPI names/actions. Record package/source markers, binary and
save hashes, clean shutdown, and foreground preservation. Until then, the
FreeCell and keyboard/real-platform rows remain Partial.
