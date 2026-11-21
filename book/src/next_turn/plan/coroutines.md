# Under the hood

`posturn` builds its coroutine protocol directly on Rust's `async fn`. `.await` is the yield point and the game's
future is the state machine. No dependencies are needed.

**Decided:** pausing for an **event** and pausing for a **request** are implemented separately, each with its own
slot, handle, and hand-written `Future`. They differ enough that sharing one generic mechanism would only blur them:

| | Event | Request |
| --- | --- | --- |
| What the UI gets | `event::Handle` | `request::Handle<T>` (`T = R::Response`) |
| How the game resumes | the handle is **dropped** | `handle.resume(value)` |
| What flows back | nothing | a `T` |
| Can it go wrong? | no: every drop resumes | yes: dropping without resuming abandons the request |
| Slot | `event::Slot` | `request::Slot<T>` |
| Future the game awaits | `event::Future` (`Output = ()`) | `request::Future<T>` (`Output = T`) |

Both handles are named `Handle` and both futures `Future`; each lives in its own module, so the plan always writes
them module-qualified: `event::Handle`, `event::Future`, `request::Handle<T>`, `request::Future<T>`. Inside those
modules, a struct named `Future` shadows the standard trait, so the sketches below implement it by its full path,
`std::future::Future`.

## Events: `event::Handle` and `event::Future`

An event suspension carries no value; the only question is whether the UI is done with it.

```rust,ignore
/// `posturn::event::Slot` (private). Shared between a `Handle` and its `Future`; `true` once the `Handle` has been
/// dropped.
type Slot = Rc<Cell<bool>>;

/// `posturn::event::Handle`. Handed to a `Host::on_event` callback. Has no methods: dropping it lets the game
/// continue.
pub struct Handle { done: Slot }

impl Drop for Handle {
   fn drop(&mut self) { self.done.set(true); }
}

/// `posturn::event::Future` (private). What the UI callback's listener awaits after calling the callback.
struct Future { done: Slot }

impl std::future::Future for Future {
   type Output = ();
   fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
      if self.done.get() { Poll::Ready(()) } else { Poll::Pending }
   }
}
```

"Play an animation, then continue" is therefore just: whatever owns the animation owns the `event::Handle`, and drops it
when the animation finishes. A UI that doesn't need to pause for an event drops the handle inside the callback.

## Requests: `request::Handle` and `request::Future`

A request suspension must deliver exactly one value, and it can be abandoned.

```rust,ignore
/// `posturn::request::State` (private).
enum State<T> {
   /// The UI holds the handle and hasn't answered yet.
   Waiting,
   /// The UI answered; the game takes the value on its next poll.
   Filled(T),
   /// The game has taken the value.
   Taken,
   /// The handle was dropped without an answer.
   Abandoned,
}

/// `posturn::request::Slot` (private).
type Slot<T> = Rc<RefCell<State<T>>>;

/// `posturn::request::Handle`. Handed to a `Host::on_request` callback.
pub struct Handle<T> {
   slot: Slot<T>,
   /// Dies with the `Host` (which owns one `Rc<Registry>` and, through the game future, the other).
   registry: Weak<Registry>,
}

impl<T> Handle<T> {
   /// Supplies the response. Consumes the handle, so a request is answered at most once.
   ///
   /// Returns `Err(value)`, handing the value back, if the `Host` that made the request has been dropped (for
   /// example, after "New Game"), since no game is left to receive it.
   pub fn resume(self, value: T) -> Result<(), T> {
      if self.registry.strong_count() == 0 {
         return Err(value);
      }
      *self.slot.borrow_mut() = State::Filled(value);
      Ok(())
   }
}

impl<T> Drop for Handle<T> {
   fn drop(&mut self) {
      // `resume` has already filled the slot; anything else is an abandoned request. If the `Host` is gone, nobody
      // will ever check, so dropping a stale handle is harmless.
      let mut state = self.slot.borrow_mut();
      if matches!(*state, State::Waiting) { *state = State::Abandoned; }
   }
}

/// `posturn::request::Future` (private). What the UI callback's listener awaits after calling the callback.
struct Future<T> { slot: Slot<T> }

impl<T> std::future::Future for Future<T> {
   type Output = T;
   fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
      let mut state = self.slot.borrow_mut();
      match std::mem::replace(&mut *state, State::Taken) {
         State::Filled(value) => Poll::Ready(value),
         State::Waiting => { *state = State::Waiting; Poll::Pending }
         // `Host::step()` panics on an abandoned request before polling, so these can't be reached.
         State::Abandoned | State::Taken => unreachable!(),
      }
   }
}
```

Neither handle is `Clone` or `Send`: there is exactly one continuation per suspension.

### Handles that outlive their `Host`

**Decided:** a handle can outlive its game, for example when the UI starts a new game while still holding the old
turn's prompt. Each handle shares only its own slot, so nothing breaks:

| Leftover handle | What happens |
| --- | --- |
| `event::Handle` dropped | sets `done` on a slot nobody reads; harmless |
| `request::Handle` dropped | marks the slot `Abandoned`; nobody checks it, so this is **not** an error |
| `handle.resume(value)` | returns `Err(value)`: the value is handed back instead of being silently discarded |

`request::Handle` keeps a `Weak<Registry>` to detect this. The `Result` is `#[must_use]`, so a UI that doesn't care
writes `let _ = handle.resume(value);` (or `.ok()`); one that does can, say, discard a stale prompt.

## Current suspension

So that `step()` can avoid polling when nothing would change, the registry ([`Registry`](./host.md#inside-the-registry),
shared by `Host` and `CtxMut`) records what the game is currently parked on. The two cases stay separate here too:

```rust,ignore
enum Suspension {
   /// Parked on an event; resumes once the `event::Handle` is dropped.
   Event {
      /// `std::any::type_name` of the event, for diagnostics.
      event_type: &'static str,
      done: event::Slot,
   },
   /// Parked on a request; resumes once the `request::Handle` is resumed, and panics if it was abandoned.
   Request {
      /// `std::any::type_name` of the request, e.g. for "request::Handle<PartyInput> for PartyInputRequest was
      /// dropped without being resumed".
      request_type: &'static str,
      /// The slot's state with `T` erased: is it waiting, filled, or abandoned?
      status: Rc<dyn Fn() -> RequestStatus>,
   },
}

enum RequestStatus { Waiting, Filled, Abandoned }
```

Before polling, `step()` checks it:

| Current suspension | `step()` |
| --- | --- |
| none | polls the game |
| `Event`, `done` is `false` | returns `Poll::Pending` without polling |
| `Event`, `done` is `true` | polls the game |
| `Request`, `Waiting` | returns `Poll::Pending` without polling |
| `Request`, `Filled` | polls the game |
| `Request`, `Abandoned` | panics, naming the request type |

The UI callback's listener sets `current` just before awaiting its `event::Future` or `request::Future`, and clears it
once that future completes.

## The no-op waker

`Host::step()` polls with `Waker::noop()` (stable since Rust 1.85, well within the 1.86 MSRV).
Wakeups are meaningless here: the caller always knows when to poll again, namely after dropping an `event::Handle` or
resuming a `request::Handle`.

## Why not `genawaiter`?

`genawaiter` gave `v0.3` one `Yield` type and one `Resume` type per coroutine. That restriction is the root of
[input bloat](../input_bloat.md). With a fresh, typed slot for every suspension, every request has its own response
type, events carry no response at all, and the public API no longer needs `Coroutine`, `GeneratorState`, or
`pin_mut!`.
