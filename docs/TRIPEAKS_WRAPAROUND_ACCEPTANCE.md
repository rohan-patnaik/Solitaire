# TriPeaks Ace-King wrap candidate acceptance

This record covers the optional local TriPeaks rank-wrap workflow. It is
automated source and controller evidence, not exact-package keyboard or
assistive-technology acceptance.

## User workflow and compatibility

The TriPeaks new-deal control offers `Standard` and `Ace-King wrap`. Standard
keeps Ace and King non-adjacent. Ace-King wrap changes only that boundary pair;
all other adjacent-rank, exposure, scoring, stock, win, history, and resource
rules are shared. The choice is represented by the existing serialized
`Options::wraparound` Boolean, so the replay and save schema shapes are
unchanged. This is not downgrade compatibility: every published build through
`a72f3ce` rejected `wraparound: true`. Its recovery path quarantines that save,
preserves the source bytes, and opens a fresh deal. A wrap-enabled save should
therefore not be opened with those older builds; Standard saves remain
compatible.

The ComboBox has an explicit accessible label and two-way value/index bindings.
Rendering a reopened deal synchronizes both values. While a dirty-state choice
is pending, rendering does not replace the selected future rule with the
current deal's rule; the visible gameplay description and header continue to
report the active rule separately.

## Atomic and lifecycle evidence

`tripeaks_wraparound_is_strict_atomic_reopenable_and_history_safe` starts the
optional rule through the production pending-deal transaction and proves:

- exact rule and numbered deal persistence through the revisioned loader;
- a fixed seed-six exposed Ace/King boundary action routes through the
  controller, then undo/redo preserves the rule and reopens equivalently;
- the game save remains mode `0600`;
- the durable next-deal counter advances exactly once; and
- malformed, whitespace-altered, trailing, and 4 KiB hostile rule requests
  preserve the in-memory game, save bytes, counter bytes, and pending request.

`wraparound_tripeaks_checked_save_reopens_equivalent` covers checked
compare-and-replace persistence plus history reconstruction. A raw versioned
envelope with the wraparound option is accepted and preserved by
`wraparound_tripeaks_setup_is_accepted_and_preserved`. Existing
`adjacency_and_optional_wraparound_are_enforced` coverage proves the Ace/King
boundary and rejection under Standard rules.

## Remaining installed gate

After publication and exact-SHA package CI, a safe offscreen input environment
may select both rules, start and reopen deals, verify the active-rule header,
and exercise an Ace/King move through keyboard and AT-SPI. If input cannot
continuously preserve the user's active application, skip the interaction and
keep the TriPeaks, accessibility, and real-platform rows Partial.
