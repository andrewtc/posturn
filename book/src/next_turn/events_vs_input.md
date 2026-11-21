<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Events vs. input

In `v0.3`, a game has exactly one way to hand control back to the main program: `ctx.yield_event(event)`. Every
yield carries a `Game::Event` out and brings a `Game::Input` back. That conflates two very different things:

1. **Notifications.** "A piece moved", "the turn ended", "Ro!". The game is telling the UI that something happened,
   usually so that it can play an animation or a sound. No answer is needed.
2. **Requests.** "Which tile does player X want?", "What does the party do next?". The game cannot continue until it
   receives a meaningful answer of a _specific_ type.

Because both travel through the same channel, every notification still has to be "answered" with a `Game::Input`.
That is why `RoShamBo` declares `type Input = ()`, and why a game with real input (like `TicTacToe`) must accept a
`Pos` even after yielding something that was only a notification. The UI, in turn, has to inspect each `Event` to work
out whether the game is _actually_ waiting for the player or merely reporting progress, and then reduce whatever the
player did into the one `Input` type the game accepts.

There is also only a single place for game logic to react to events: `Play::handle_event`. Every event the game emits
goes through that one function before it reaches the UI. Nothing else can hook in. A second system (say, a quest
tracker that wants to know when the party moves) has to be wired into `handle_event` by hand, and there is no way for
one piece of logic to _override_ or _post-process_ what another piece does with the same event.

We want:

- Notifications that need no answer to be expressible as such, with no dummy input.
- Requests whose answer type is known at the point of the request, and checked by the compiler.
- Any number of independent pieces of logic to be able to hook the same event, override it, or adjust its result.
