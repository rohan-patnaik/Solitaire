# Klondike double-click candidate acceptance

This record defines the dependency-free Klondike pointer workflow for moving an
exposed waste or top-tableau card directly to its suit foundation. It is source
and exact-package candidate evidence, not installed pointer, visual, or spoken
assistive-technology acceptance.

## Workflow contract

- A single pointer click on the waste or an exposed top-tableau card retains the
  existing select/move behavior. A double-click on the same card requests one
  direct move to that card's suit foundation through the existing deterministic
  Klondike action path.
- Slint's platform double-click interval is 500 ms. One application-scoped Rust
  `SingleShot` timer defers only eligible pointer single-clicks for that interval;
  it is stopped on the second pointer-down before the second release and is
  cancelled/replaced when another eligible card is pressed. There are no
  per-card timers or idle wakeups. Keyboard Enter/Space and the accessibility default action remain immediate.
- Each eligible click captures the card's bounded rank/suit label, the ephemeral
  Klondike deal-instance token, and a session-only interaction generation. Every
  intervening controller input advances that generation, so a pending single
  click cannot overtake stock, toolbar, pointer, keyboard, or accessibility
  input. The delayed single-click and double-click callbacks must still match the
  current interaction, deal, and exposed card at the exact
  waste or top-tableau source. Game changes, new or restarted deals, reloads,
  blocked or confirmed close requests, stale indices, face-down cards, covered
  cards, and changed source cards fail
  closed with an actionable retry status and no game, selection, replay, or file
  change. The instance token is not persisted and wraps only after `u64::MAX`
  successful Klondike replacements.
- The arbiter retains the first click's complete source/card/deal/generation
  identity independently of timer cancellation. A double action is armed only
  when the second pointer-down matches that identity, and the release must still
  match before the identity is consumed. A different live card rebound into the
  same visual item and a direct double callback without a first click both fail
  closed without game, selection, profile, or file mutation.
- A legal direct move uses the existing scoring, move counter, replay, bounded
  history, save, stale-writer, local-profile, win, undo, and redo paths. An
  illegal foundation move remains atomic and reports the existing friendly
  foundation error.
- Foundations are not double-click sources, and tableau runs do not move by
  double-click. Drag/drop remains explicitly deferred.
- No save, profile, replay, fixture, package dependency, asset, license, plugin,
  settings-storage, or collection taxonomy schema changes are introduced.

## Automated evidence

- `pointer_click_timer_is_idle_single_shot_and_cancelable` runs the real Slint
  timer headlessly and proves zero idle callbacks, exactly one callback for a
  single schedule, no repetition, and pointer-down cancellation past the full
  interval.
- `deferred_pointer_click_cannot_overtake_keyboard_or_stock_input` proves that
  intervening immediate card activation and stock mutation win deterministically
  while the older delayed callback remains inert.
- `double_click_requires_matching_first_click_identity` covers a direct double
  without a first click, first-card to intervening-stock to second-card rebasing,
  unchanged game/selection/profile/file bytes, and the exact matching gesture.
- `blocked_close_invalidates_a_pending_pointer_click` schedules a valid card
  single, forces the dirty/profile-dirty close guard to keep the window shown,
  expires the real timer, and proves unchanged game, selection, profile, and
  owner file bytes. Its guarded stale branch includes explicit stock, profile,
  and file sentinels, so every unchanged assertion would fail if the invalidated
  callback were allowed to run.
- `klondike_double_activation_is_exact_atomic_and_undoable` reconstructs the
  pinned normal near-win, rejects negative, overflowing, wrong-game,
  stale-instance, non-top, wrong-card, and 4 KiB tokens without mutation, proves
  the checked delayed single-click selection route, completes exact legal
  tableau and waste foundation moves while replacing a prior selection, and
  proves undo/redo.
- `klondike_double_click_declares_pointer_keyboard_and_recovery_contracts` pins
  the 500 ms Rust `SingleShot` arbiter, second-pointer-down cancellation,
  interaction generation, captured-card validation, waste/tableau callback
  routing, top-card boundary, immediate keyboard/default actions, documentation,
  and truthful catalog limits.
- The full controller, domain, persistence, recovery, complete-deal, profile,
  and deployment suites remain the regression boundary.

## Remaining installed gate

The exact packaged workflow has not been exercised through an input-capable
offscreen or nested compositor. Pointer timing at the boundary, selection and
direct movement, stale-card recovery during a pending click, visible focus,
AT-SPI state/action updates, touch double-tap behavior, spoken output, and
foreground preservation remain open. No live Omarchy summon or production GUI
is permitted, so the Klondike, accessibility, packaging, and real-platform rows
remain Partial.
