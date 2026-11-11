# Implementation steps

1. **Remove `genawaiter`.** Drop the dependency, the `pub use genawaiter`, `Play`, `Context`, and the generic
   `Host<Game>`. Bump the version to `0.4.0`.
2. **No-op waker** (`waker.rs`).
3. **Slots and handles** (`handle.rs`): `SlotState`, `Suspend`, `WaitForInput<R>`, and the future that awaits a slot.
4. **Module registry** (`module.rs`, `host.rs`): the `Module` trait and `TypeId`-keyed storage with `insert`,
   `borrow`, `borrow_mut`, `with`, `with_mut`, `module`, `clone_module`, building on the `module` patch.
5. **Events and listeners** (`event.rs`): the `Event` trait, `Next<E>`, `Host::listen`, `Host::emit`, and the
   per-type chains, including the unhandled-event panic.
6. **UI handlers:** `Host::on_suspend` and `Host::on_input`, wrapping callbacks into listeners and recording the
   current suspension.
7. **Driver** (`session.rs`): `Session` and `Step`, with the no-op/abandoned/foreign-`Pending` rules from
   [Driving the game](./driving.md).
8. **Tests:**
   - Notification-only game (rewrite `RoShamBo`) completes with `Suspend`s dropped immediately.
   - Typed request round-trip through `on_input`.
   - Listener ordering: most recent first; `next.run` reaches lower listeners.
   - Override: a listener returns without calling `next`, so the UI callback never runs.
   - Post-processing: a listener changes the UI's input before the game sees it.
   - `step()` is a no-op while a handle is held.
   - Synchronous resume inside the callback doesn't suspend.
   - Panics: unhandled event, abandoned `WaitForInput`, foreign `Pending`, `step()` after `Done`, missing module.
9. **Port `examples/tui`** (`TicTacToe`) to the new API.
10. **Docs:** README, crate docs, and update the [Where we are](../where_we_are.md) example in this book.

## Acceptance criteria

- No dependencies; no `Send`/`Sync` bounds or impls in the public API; no `unsafe` outside the waker.
- The [worked example](./example.md), reduced to something self-contained, compiles and runs as a doc test.
- All tests above pass; `examples/tui` plays a full game.
