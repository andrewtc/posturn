# Handing control to the UI

The UI cannot `await` inside the game coroutine: it lives _outside_ the coroutine and is the thing that drives it. So
the UI's link in the listener chain is not an `async` function. It is a plain callback that receives the event and a
**handle**. The UI stores the handle, carries on rendering and reading input across as many frames as it needs, and
eventually uses the handle to let the game continue.

## Two kinds of handle

**Decided:** there are two handle types, and two registration methods, so that the compiler knows which one a callback
receives:

```rust,ignore
// Notification events (Output = ()): the callback gets a `Suspend`.
host.on_suspend(|ev: PartyMoved, suspend: Suspend| {
   ui.start_walk_animation(ev.direction, suspend); // Resumes when the animation drops it.
});

// Every other event: the callback gets a `WaitForInput<E::Output>`.
host.on_input(|ev: PartyInputRequested, wait: WaitForInput<PartyInput>| {
   ui.prompt_party_action(wait); // Later: wait.resume(PartyInput::Rest { hours })
});
```

- `on_suspend` requires `E: Event<Output = ()>`. The callback is `FnMut(E, Suspend) + 'static`.
- `on_input` takes any `E: Event`. The callback is `FnMut(E, WaitForInput<E::Output>) + 'static`.

Stable Rust cannot select the handle type automatically from `E::Output`: a trait impl for `()` and a blanket impl
for every other `T` would overlap without specialization. Hence the two methods.

### `Suspend`

- `suspend.resume()` lets the game continue.
- **Dropping a `Suspend` also resumes the game.** This makes "play an animation, then continue" trivial: whatever owns
  the animation owns the `Suspend` and drops it when finished.

### `WaitForInput<R>`

- `wait.resume(value: R)` supplies the output and lets the game continue. It consumes the handle, so a request is
  answered at most once.
- **Dropping a `WaitForInput<R>` without resuming is an error.** The drop marks the slot as _abandoned_; the next
  `step()` panics with a message naming the event type, e.g.
  `WaitForInput<PartyInput> for PartyInputRequested was dropped without being resumed`. (`Drop` itself does not
  panic, because panicking during an unwind aborts the process.)

## Resuming early

A callback may resume its handle immediately, inside the callback. In that case the game never actually suspends:
the UI handler's future is ready on its first poll, and the game carries on within the same `step()`. This is useful
for UIs (or headless test harnesses, or AI players) that can answer synchronously.

## Under the hood

Each UI callback is wrapped into an ordinary listener. When that listener runs, it creates a fresh slot, calls the
callback with the event and a handle pointing at the slot, and then awaits a future on the slot. That future is
`Pending` until the handle fills the slot. The host also records the slot as the session's _current suspension_, so
the [driver](./driving.md) can tell whether stepping would make progress. See
[Under the hood](./coroutines.md) for the slot itself.
