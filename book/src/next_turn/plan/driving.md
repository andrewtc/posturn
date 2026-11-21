<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Driving the game

The UI drives the game by hand through its `Host<Outcome>`, which owns the pinned, boxed game future and polls it with a
no-op waker.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
// Creating the host runs `set_up`, but doesn't poll the game yet.
let mut host = Host::new(rpg::Setup::from_save(&save));
register_ui_callbacks(&mut host, &ui);

let outcome = loop {
   if let Poll::Ready(outcome) = host.step() {
      break outcome;
   }

   // Between steps the UI may borrow modules freely...
   let overworld = host.borrow::<OverworldModule>();
   ui.borrow_mut().update_and_render(&overworld); // ...and may resume a stored handle.
   // ...but `overworld` must be dropped before the next `step()`, or this won't compile.
};
```
<!-- SPDX-SnippetEnd -->

"New Game" means dropping the `Host` (which drops the game future, its `CtxMut`, and every module and listener) and
creating a new one from a fresh `Setup`.

## `step()`

`step(&mut self) -> Poll<Outcome>`:

- **If the `Host` is [poisoned](#when-something-panics)**, `step()` panics immediately with a clear message.
- **If there is a current suspension and its handle has not been resumed, `step()` does nothing** and returns
  `Poll::Pending`. The UI can call `step()` every frame without tracking the handle's state itself.
- **If the current suspension was abandoned** (a `request::Handle` was dropped without resuming), `step()` panics with a
  message naming the event type.
- **Otherwise it polls the game future once.** Within that single poll the game runs as far as it can: through any
  number of emits and requests whose UI callbacks resume immediately, until it either:
  - reaches a UI callback that keeps its handle, so the future returns `Pending` and `step()` returns
    `Poll::Pending`, or
  - finishes, and `step()` returns `Poll::Ready(output)`.
- **If the future returns `Pending` without a posturn suspension**, the game awaited something posturn did not create
  (a timer, a channel, another executor's future). `step()` panics: with a no-op waker, nothing could ever wake it.
- Calling `step()` again after it has returned `Poll::Ready` panics.

This also removes the [null input](../null_input.md) problem: the first `step()` takes no input, because the game
always _asks_ for input through an event.

## When something panics

**Decided:** a panic anywhere in the game (the game's async function, a listener, or a UI callback, all of which run
inside `step()`) **propagates** out of `step()` to the UI loop, and **poisons** the `Host`.

- While polling, `step()` holds a small guard. If the guard is dropped during an unwind
  (`std::thread::panicking()`), it marks the `Host` as poisoned. No `catch_unwind` and no `UnwindSafe` bounds are
  involved.
- Every later `step()` panics immediately with a message like "the game panicked during an earlier `step()`; create a
  new `Host`", instead of Rust's own "`async fn` resumed after panicking".
- `host.is_poisoned() -> bool` lets the UI check up front, e.g. to switch to an error screen.
- Reading modules (`host.borrow`, `host.module`, ...) is still allowed after poisoning, so a crash screen can show the
  last known state. The state may be partway through an update, which the docs will point out. Any `Ref`/`RefMut` guards
  held during the panic were released by the unwind, so no `RefCell` is left locked.
- A UI that wants to recover (say, return to the main menu) can wrap `step()` in `std::panic::catch_unwind` itself,
  drop the poisoned `Host`, and create a new one.
- Listeners are not isolated from each other: catching a panic in one listener and carrying on with the next would
  run the chain on half-updated modules, after the panicking listener had already consumed its `next`.

With `panic = "abort"`, none of this applies: the process ends at the first panic.

## Why `Poll`?

**Decided:** `step()` returns the standard `std::task::Poll<Outcome>` rather than a dedicated enum. It has exactly the
right shape: `Poll::Pending` while the UI still holds a handle, and `Poll::Ready(out)` once the game has finished.
It's also familiar to anyone who knows Rust's `Future`s, which is fitting, since `step()` is a thin wrapper around
polling the game future.

## One suspension at a time

A game has at most one current suspension. This falls out of the borrowing rule: every suspension point takes `&mut
CtxMut`, so two emits or requests can't be in flight at once (e.g. inside a `join!`). Simultaneous input from several
players is modeled as a single request whose response carries everyone's choices, or as sequential requests. See [Future
work](./future_work.md).
