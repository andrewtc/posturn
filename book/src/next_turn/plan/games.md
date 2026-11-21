<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Setting up a game

**Decided:** `Play` is replaced by a much smaller trait, `Setup`. A type implementing `Setup` holds everything needed
to start one game (board size, starting state, available choices, players...). Its one method registers the game's
modules and listeners, and then turns the setup into the game itself: an `async` function that `posturn` will run.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
pub trait Setup {
   /// What the game produces when it finishes.
   type Outcome;

   /// Registers modules and listeners, then returns the game as an async function that will receive the game's
   /// `CtxMut`.
   fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Self::Outcome + 'static;
}
```
<!-- SPDX-SnippetEnd -->

The intended usage is one setup type per game:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
pub mod tic_tac_toe {
   pub struct Setup {
      pub size: u16,
      pub first_player: Player,
   }

   impl posturn::Setup for Setup {
      type Outcome = Outcome; // This game's own `Outcome` type.

      fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Outcome + 'static {
         registry.insert(Board::new(self.size));
         registry.insert(Turn(self.first_player));
         registry.on_event(on_piece_placed);

         // The implementor decides how to express the game: an async closure, or a separate async fn.
         async move |ctx: CtxMut| play(self, ctx).await
      }
   }

   async fn play(setup: Setup, mut ctx: CtxMut) -> Outcome {
      // Run turns by submitting requests and emitting events...
   }
}

let mut host = Host::new(tic_tac_toe::Setup { size: 3, first_player: Player::X }); // Host<Outcome>
```
<!-- SPDX-SnippetEnd -->

A game that never ends can use `!` (or `std::convert::Infallible` on stable) as its `Outcome`.

## What `Host::new` does

`Host::new<S: Setup<Outcome = Outcome>>(setup: S) -> Host<Outcome>`:

1. Creates an empty `Registry` by value.
2. Calls `setup.set_up(&mut registry)`. The setup inserts every module the game will ever have and registers every
   game listener, editing the `Registry` directly.
3. **Freezes** it by moving it into an `Rc<Registry>`. Nobody can get a `&mut Registry` back out of a shared `Rc`, so
   from here on no module or game listener can be added.
4. Creates the game's one and only `CtxMut` (wrapping a clone of that `Rc`) and calls the returned async function with
   it, producing the game future. The `Host` keeps the other `Rc`.
5. Pins and boxes that future inside the `Host`. Nothing is polled until the first [`step()`](./driving.md).

## `Registry`

The `Registry` holds all shared state (its fields are listed under
[Inside the `Registry`](./host.md#inside-the-registry)). The user only ever sees it as `&mut Registry` during
`set_up`, where its job is registering modules and listeners. It also implements `module::Borrow` and
`module::BorrowMut`, so a setup can read or adjust modules it has already inserted, and it can hand out a read-only
`Ctx` with `registry.ctx()` (see [Looking up modules](./host.md#looking-up-modules)).

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
impl Registry {
   /// Registers a module. Panics if a module of the same type was already inserted.
   pub fn insert<M: Module>(&mut self, module: M);

   /// Registers a game listener for an event type. See "Events, requests, and listeners".
   pub fn on_event<E: Event, F>(&mut self, listener: F) where F: AsyncFn(&mut CtxMut, E, event::Next<E>) + 'static;

   /// Registers a game listener for a request type.
   pub fn on_request<R: Request, F>(&mut self, listener: F) where
      F: AsyncFn(&mut CtxMut, R, request::Next<R>) -> R::Response + 'static;
}
```
<!-- SPDX-SnippetEnd -->

**Modules and listeners can only be registered during setup, and the compiler enforces it,** twice over:

- `set_up` only _borrows_ the `Registry`, so the `'static` async function it returns can't hold on to it. Trying to
  capture it (for example, to insert a module or reorder listeners later in the game) fails to compile with "lifetime
  may not live long enough".
- After setup the `Registry` lives in an `Rc`, which never hands out `&mut`. Neither `Host` nor `CtxMut` has an
  `insert` method, and `CtxMut` has no way to register listeners. (The `Host` registers only the UI's callbacks, which
  always sit at the bottom of each chain; see [Handing control to the UI](./ui_handlers.md#always-at-the-bottom).)

A consequence: a module that only matters part of the time (say, a `BattleModule`) is inserted up front in an empty
state, e.g. with `current_battle: None`.

Because the module map never changes after setup, borrowing a module returns a `Ref` straight out of a fixed
`BTreeMap`. **No `unsafe` is needed.**

## Why return an async function rather than a `Future`?

If `set_up` returned the game's `Future` directly, that future would need a `CtxMut` before the module map exists. That
means either handing out a `CtxMut` during setup that can't be used yet (checked only at runtime), or letting the future
hold on to the `Registry` (which would let it insert modules mid-game). Returning `AsyncFnOnce(CtxMut)` lets `Host` hand
over the game's handle only once the modules are fixed, and guarantees there is exactly one `CtxMut`.

## Consequences

- **There is no `Game` type parameter.** `Play`'s `Event` and `Input` associated types are gone, replaced by
  individual [event and request types](./events.md). Its `Outcome` lives on as `Setup::Outcome`, which becomes the
  type parameter of `Host<Outcome>`.
- **The game owns its private state.** The setup struct and local variables in the game's `async` function are the
  game's private state. Only state meant to be **inspected by the UI** is published as a [module](./host.md).
- **`handle_event` is gone.** Reacting to events is done by [listeners](./events.md), of which there can be many.
- **Helpers that can suspend take `&mut CtxMut`.** Sub-games compose naturally: a battle minigame is just another
  `async fn` that the overworld awaits with `battle::play(&mut ctx).await`. The `&mut CtxMut` in its signature tells
  the reader that it may suspend.
- **"New Game" is `Host::new(setup)` again**, with a fresh setup value.
