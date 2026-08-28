# TriPeaks hostile-action and action-space candidate acceptance

This record covers dependency-free, renderer-independent TriPeaks engine
hardening. It is automated source evidence, not exact-package input, graphical,
or assistive-technology acceptance.

## Hostile-action atomicity

`hostile_actions_are_exact_and_fully_atomic` exercises covered, non-adjacent,
empty-stock, empty-waste, out-of-range, counter-overflow, replay-capacity, and
completed-game requests. Every request must return its exact typed error while
preserving both structural equality and serialized bytes for the complete game,
including replay and undo/redo history.

The out-of-range sweep includes the first index beyond the 28-card tableau and
the maximum value representable by the public `Remove(u8)` action. Counter
coverage separates draw-move, removal-move, streak, and score overflow paths.

## Fixed action-space sweep

`fixed_seed_rule_action_space_preserves_tripeaks_invariants` checks Standard
and Ace-King-wrap rules at seeds 0, 7, 41, and `u64::MAX`. At each reached state
it probes `Draw` plus every possible `Remove(u8)` value, including all hostile
indices, then advances only through the production deterministic hint.

Every accepted action must preserve exactly 52 unique cards, the dependency
graph's exposure result, move/replay agreement, legal replay reconstruction,
and exact undo/redo round trips. Every rejected action must preserve the full
game. The sweep requires observed legal draws, legal removals, and rejected
actions across the matrix.

## Remaining acceptance boundary

This evidence narrows the previously open hostile/property gap but does not
change the TriPeaks capability from Partial. Exact-package rule selection,
final-transition/process identity, drag/touch, full keyboard traversal, and
spoken assistive-technology acceptance remain open.
