# Klondike safe-finish acceptance

This gate covers the existing **Finish safe moves** control as a narrow local
Klondike workflow. It does not claim a solver, forced completion, or a change
to the rules that decide whether a foundation move is safe.

## Product contract

- The toolbar button is keyboard-focusable and its accessible label is
  `Move every currently safe Klondike card to a foundation`.
- Activation clears any stale card selection before asking the existing
  deterministic Klondike engine to move currently safe exposed cards.
- The loop is bounded by the 52-card deck: each successful iteration moves one
  card to a foundation and no iteration allocates an unbounded collection.
- Zero available moves is a successful write-free no-op. It reports
  `Moved 0 safe cards to foundations`, clears selection, and does not write the
  game or local profile.
- One or more moves use the normal replay, checked mode-0600 save, local-profile
  observation, dirty close guard, and explicit retry/reload recovery paths.
- Each moved card remains one ordinary replay action. Undo and redo therefore
  remain per card; this slice does not introduce grouped history or a schema
  change.

## Display-independent evidence

The pinned seed-zero draw-one/Standard near-win fixture reaches victory through
one safe-finish move. Focused controller coverage proves exact move count,
selection clearing, card conservation, win state, checked game/profile saves,
one-time statistics, undo/redo, byte-identical no-op behavior, and loader-level
reopen. A stale-writer test proves that the completed in-memory game remains
dirty and the external disk owner remains byte-identical until explicit reload
chooses the newer disk state.

The deployment contract pins the visible label, accessible label, controller
route, bounded domain loop, recovery language, and this evidence boundary.
Malformed-input tests are not applicable because the control accepts no text,
number, path, or variable-sized payload.

## Open exact-package gate

No production GUI or live Omarchy shell is invoked for this candidate. Exact-
package keyboard traversal, default-action invocation, live-region output,
AT-SPI state, spoken output, visible multi-move progression, foreground
preservation, and pointer/touch activation remain open. `game.klondike`,
`quality.accessibility`, and `quality.real-omarchy` therefore remain Partial.
