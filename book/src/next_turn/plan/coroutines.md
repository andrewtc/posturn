# Under the hood

`posturn` builds its coroutine protocol directly on Rust's `async fn`. `.await` is the yield point and the game's
future is the state machine. No dependencies are needed.

## The slot

Every suspension is a one-shot, single-threaded channel from a UI handle back into the game:

```rust,ignore
enum SlotState<T> {
   /// The UI holds the handle and hasn't answered yet.
   Waiting,
   /// The UI answered; the game will pick this up on its next poll.
   Filled(T),
   /// The handle was dropped without an answer (only possible for `WaitForInput`).
   Abandoned,
}

type Slot<T> = Rc<RefCell<SlotState<T>>>;
```

- `WaitForInput<T>` holds a `Slot<T>`. `resume(self, v)` stores `Filled(v)`; its `Drop` stores `Abandoned` if the
  slot is still `Waiting`.
- `Suspend` holds a `Slot<()>`. Both `resume(self)` and `Drop` store `Filled(())`.
- The future the UI listener awaits takes the value out of the slot when it is `Filled`, returns `Pending` while it
  is `Waiting`, and is never polled while `Abandoned`, because the [driver](./driving.md) panics first.

Handles are neither `Clone` nor `Send`: there is exactly one continuation per suspension.

## Current suspension

So that `step()` can avoid polling when nothing would change, the `Host` keeps a type-erased reference to the current
slot plus the event's type name (for error messages):

```rust,ignore
struct CurrentSuspension {
   event_type: &'static str,
   state: Rc<dyn Fn() -> SuspensionState>, // Waiting | Ready | Abandoned
}
```

The UI listener sets it before returning `Pending` and clears it once it has taken the value.

## The no-op waker

The driver polls with a `Waker` whose vtable functions do nothing (built from `RawWaker` and `RawWakerVTable`, or
`Waker::noop()` once the MSRV allows it). Wakeups are meaningless here: the caller always knows when to poll again,
namely after resuming a handle.

## Why not `genawaiter`?

`genawaiter` gave `v0.3` one `Yield` type and one `Resume` type per coroutine. That restriction is the root of
[input bloat](../input_bloat.md). With typed slots created per suspension, every emit has its own output type, and
the public API no longer needs `Coroutine`, `GeneratorState`, or `pin_mut!`.
