# Pyramid redeal-limit candidate acceptance

This record covers the local Pyramid stock-limit workflow. It is automated
source, controller, and persistence evidence, not exact-package keyboard or
assistive-technology acceptance.

## User workflow and compatibility

The Pyramid new-deal control offers `No redeals`, `1 redeal`, and `2 redeals`.
The active deal reports redeals used against its persisted maximum, separately
from a future pending choice. The existing serialized `Options::max_redeals`
`u8` field is used without a schema-shape change. Valid persisted values outside
the three UI presets remain loadable and are displayed without falsely selecting
a preset.

This is not full downgrade compatibility. Published builds through `b50faa5`
accept only the default value of two. Their recovery path quarantines a save
with another limit, preserves its source bytes, and opens a fresh deal. Do not
open a zero- or one-redeal save with those older builds; two-redeal saves remain
compatible.

The ComboBox has an explicit accessible label and two-way value/index bindings.
While dirty progress awaits confirmation, rendering does not replace the future
selection with the current deal's value.

## Atomic and lifecycle evidence

`pyramid_redeal_limits_are_strict_atomic_reopenable_and_enforced` proves:

- every preset starts and atomically reopens with its exact persisted bound;
- one allowed recycle succeeds, exhaustion rejects another recycle with
  `No Pyramid redeals remain`, and memory, save bytes, and counter bytes remain
  unchanged on rejection;
- undo/redo reconstructs the exhausted state and the revisioned loader reopens
  it equivalently;
- malformed, case- or whitespace-altered, trailing, and 4 KiB hostile fields
  preserve game, save, counter, and pending request;
- dirty confirmation commits the staged zero-redeal deal exactly; and
- game and deal-counter files retain mode `0600`.

`bounded_pyramid_redeal_setups_are_accepted_and_preserved` covers normal load,
replay load, and recovery load for zero, one, two, and `u8::MAX` without source
rewriting. `reopened_pyramid_options_map_values_and_indices_without_a_display`
tests the production render mapping for all presets and a custom valid value
without creating a display backend.

## Remaining installed gate

After publication and exact-SHA package CI, a safe offscreen input environment
may select every preset, verify the active used/maximum header, exhaust each
bound, and save/reopen through keyboard and AT-SPI. The no-focus policy forbids
using the user's live shell for this check. If an isolated environment cannot
continuously preserve the user's active application, the interaction is
skipped and the Pyramid, accessibility, and real-platform rows remain Partial.
