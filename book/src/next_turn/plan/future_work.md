<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Future work

Out of scope for `v0.4`, but the design should not rule these out.

- **Listener dependency language.** Instead of plain most-recent-first ordering, modules declare how their listeners
  relate to other modules' listeners (e.g. "run before `BattleModule`'s handler for `TurnStartedEvent`"), through a
  builder API or a procedural macro.
- **Simultaneous input.** Several outstanding suspensions at once (e.g. both players choosing in `RoShamBo`), each
  resumed independently and in any order.
- **Networking, replay, and saves.** An `async` state machine can't be serialized mid-turn, so these would work by
  logging event outputs and replaying them deterministically. This may require events and outputs to be
  `serde`-friendly.
- **`std` coroutines.** If the `Coroutine` trait is stabilized, consider whether it offers anything over `async fn`.
