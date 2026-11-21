<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# One big game state

`v0.3` asks for the whole game to be a single type implementing `Play`. All state lives in that one value, behind a
single `Rc<RefCell<Game>>` inside the `Host`, and both the game coroutine and the UI reach into it through
`borrow_game`, `borrow_game_mut`, `with_game`, and `with_game_mut`.

This causes two problems.

## Everything is one lock

There is no way to borrow _part_ of the game. While the game logic holds `borrow_game_mut()` to update the board, the
UI cannot even read the score. As a game grows (an overworld, a battle system, an inventory, a quest log), all of it
has to live inside the one `Play` type, and every subsystem competes for the same `RefCell`.

It also makes composition awkward. There is no natural seam for splitting a game into reusable pieces, because
`Play::play` and `Play::handle_event` are each a single function on a single type.

## Borrows can leak across a turn

Nothing stops a game from holding a borrow across a yield:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
let mut game = ctx.host.borrow_game_mut();
game.take_turn(pos)?;

// The RefMut is still alive while the game is suspended...
let pos = ctx.yield_event(next_event).await;
```
<!-- SPDX-SnippetEnd -->

This compiles. But while the game is suspended, the UI calls `host.with_game(...)` to render the board, and that
panics at runtime because the game is still mutably borrowed. The same hazard exists in reverse: if the UI holds a
`Ref` from `borrow_game` while it resumes the coroutine, the game panics the moment it tries to update its state.

The documentation warns about this, but the API does nothing to prevent it. Ideally the shape of the API would make it
hard (or impossible) to hold onto state across the boundary between the game and the UI.

## Leaning on `genawaiter`

Finally, `v0.3` depends on [`genawaiter`](https://docs.rs/genawaiter) to provide coroutines on stable Rust, and its
types leak into the public API: callers drive the game through `genawaiter::Coroutine`, must `pin_mut!` it themselves,
and match on `genawaiter::GeneratorState`. `posturn` even re-exports the whole crate.

This is unnecessary. A Rust `async fn` already compiles to a resumable state machine. All `posturn` needs is a thin,
typed protocol for handing control back and forth, which it can provide itself with no dependencies.
