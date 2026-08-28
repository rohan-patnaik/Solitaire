# Klondike redeal-limit candidate acceptance

This record covers the repository's bounded Klondike stock-redeal workflow. It
is automated source-level evidence, not exact-package input or assistive-
technology acceptance.

## User workflow and compatibility

The Klondike new-deal row exposes keyboard-focusable, accessibility-labelled
choices for `Unlimited`, `1 redeal`, and `3 redeals`, alongside the existing
draw and scoring choices. Starting a deal carries the exact option through the
pending-deal transaction into the renderer-independent `Options::max_redeals`
field. Existing two-field new-deal requests remain compatible and mean
unlimited redeals; the persisted replay schema is unchanged.
While a dirty-state decision is pending, rendering leaves the selected future
options intact instead of replacing them with the current deal's rules.
`reopened_klondike_options_map_values_and_indices_without_a_display` pins the
single renderer mapping used for reopened draw/scoring/redeal values and
selection indices without requiring a display server. It covers the supported
presets and the intentional index `-1` custom-limit state. A deployment
contract pins the matching two-way `ComboBox.current-index` bindings. The
mapping test also pins the guard that retains a pending future selection
instead of syncing current-deal rules over it.

The active surface reports redeals used and either the bounded remainder or
that the deal is unlimited. A save with another valid `u8` limit is shown as a
custom rule rather than silently rewritten.

## Atomic and lifecycle evidence

`klondike_redeal_limits_are_atomic_reopenable_and_enforced` proves all three
choices survive revisioned atomic save/reopen. It then plays a draw-three deal
through one complete recycle, exhausts the second stock pass, and requires:

- exactly one recorded redeal and zero remaining;
- the next recycle to fail with `No redeals remain`;
- exact in-memory game and on-disk bytes to survive that rejection;
- undo/redo to reproduce the exhausted state and exact `0600` save bytes; and
- loader-level equality after the failed action and history cycle.

`malformed_new_deal_options_preserve_game_save_counter_and_pending_request`
also covers unsupported limits, trailing fields, and a 4 KiB hostile field.
Those inputs preserve the current game, save bytes, deal counters, and pending
request. Existing domain tests cover recycle order, counter overflow, replay,
and validation of a redeal count that exceeds its declared maximum.

The first exact-SHA CI attempt at `57b8dc6e223f0fdf9590ae7893c84253bfa168dc`
failed only because its component assertion initialized Slint's Winit backend
on a GitHub runner with no `WAYLAND_DISPLAY`, `WAYLAND_SOCKET`, or `DISPLAY`.
Run `33128045066` still passed formatting, checksum, generated-catalog, Clippy,
and all 118 domain tests; 40 of 41 controller tests passed. The mapping contract
above is the display-independent remediation. No virtual display dependency was
added and the failed run is not accepted as publication evidence.

## Remaining installed gate

After publication and exact-SHA package CI, a safe offscreen input environment
may select each limit, start and reopen a deal, exhaust the one-redeal case,
and verify the visible remaining count plus keyboard/AT-SPI names. If input
cannot continuously preserve the active desktop, skip that interaction and
keep the Klondike, accessibility, and real-platform rows Partial.
