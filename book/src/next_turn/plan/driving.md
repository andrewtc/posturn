# Driving the game

The UI drives the game by hand through a driver object, provisionally called `Session`. It owns the pinned, boxed game
future and polls it with a no-op waker.

```rust,ignore
let host = Host::new();
register_ui_handlers(&host, &ui);

let mut session = Session::new(&host, play(setup, &host));

loop {
   match session.step() {
      Step::Waiting => { /* render, read input, eventually resume a stored handle */ }
      Step::Done(outcome) => break,
   }
}
```

## `step()`

- **If there is a current suspension and its handle has not been resumed, `step()` does nothing** and returns
  `Step::Waiting`. The UI can call `step()` every frame without tracking the handle's state itself.
- **If the current suspension was abandoned** (a `WaitForInput` was dropped without resuming), `step()` panics with a
  message naming the event type.
- **Otherwise it polls the game future once.** Within that single poll the game runs as far as it can: through any
  number of emits whose UI handlers resume immediately, until it either:
  - reaches a UI handler that keeps its handle, so the future returns `Pending` and `step()` returns
    `Step::Waiting`, or
  - finishes, and `step()` returns `Step::Done(output)`.
- **If the future returns `Pending` without a posturn suspension**, the game awaited something posturn did not create
  (a timer, a channel, another executor's future). `step()` panics: with a no-op waker, nothing could ever wake it.
- Calling `step()` again after `Done` panics.

This also removes the [null input](../null_input.md) problem: the first `step()` takes no input, because the game
always _asks_ for input through an event.

## The `Step` type

```rust,ignore
pub enum Step<Out> {
   /// The game is suspended on a UI handle that has not been resumed yet.
   Waiting,

   /// The game finished.
   Done(Out),
}
```

## One suspension at a time

In `v0.4` a session has at most one current suspension. Awaiting two emits at once inside the game (e.g. with a
`join!`) is not supported. Simultaneous input from several players is modeled as a single event whose output carries
everyone's choices, or as sequential requests. See [Future work](./future_work.md).
