<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Under the hood

`posturn` builds its coroutine protocol directly on Rust's `async fn`. `.await` is the yield point and the game's
future is the state machine. No dependencies are needed.

**Decided:** pausing for an **event** and pausing for a **request** are implemented separately, each with its own
slot, resume handle, and hand-written `Future`. They differ enough that sharing one generic mechanism would only blur
them:

| | Event | Request |
| --- | --- | --- |
| What the UI gets | `event::Resume` | `request::Resume<T>` (`T = R::Response`) |
| How the UI lets the game continue | the resume handle is **dropped** | `resume.with(value)` answers; `resume.resume()` passes the request on |
| What flows back | nothing | a `T`, or nothing if the request was passed on |
| What happens next | the event continues down the chain | the chain ends with the answer, or continues with the request |
| Can it go wrong? | no: every drop continues | yes: dropping it unused abandons the request |
| Slot | `event::Slot` | `request::Slot<T>` |
| Future awaited at a host listener | `event::Future` (`Output = ()`) | `request::Future<T>` (`Output = Option<T>`) |

Both resume handles are named `Resume` and both futures `Future`; each lives in its own module, so the plan always
writes them module-qualified: `event::Resume`, `event::Future`, `request::Resume<T>`, `request::Future<T>`. Inside
those modules, a struct named `Future` shadows the standard trait, so the sketches below implement it by its full path,
`std::future::Future`.

(The resume handles are not the [listener handles](./events.md#handles), `event::Handle` and `request::Handle`, which
keep a listener registered. A resume handle keeps the _game_ waiting.)

## Events: `event::Resume` and `event::Future`

An event suspension carries no value; the only question is whether the UI is done with it.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// `posturn::event::Slot` (private). Shared between a `Resume` and its `Future`; `true` once the `Resume` has been
/// dropped.
type Slot = Rc<Cell<bool>>;

/// `posturn::event::Resume`. Handed to a host listener registered with `Host::on_event`. Has no methods: dropping it
/// lets the event continue down the chain.
pub struct Resume { done: Slot }

impl Drop for Resume {
   fn drop(&mut self) { self.done.set(true); }
}

/// `posturn::event::Future` (private). What `forward` awaits at a host listener, after calling its callback.
struct Future { done: Slot }

impl std::future::Future for Future {
   type Output = ();
   fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
      if self.done.get() { Poll::Ready(()) } else { Poll::Pending }
   }
}
```
<!-- SPDX-SnippetEnd -->

"Play an animation, then continue" is therefore just: whatever owns the animation owns the `event::Resume`, and drops
it when the animation finishes. A UI that doesn't need to pause for an event drops it inside the callback.

## Requests: `request::Resume` and `request::Future`

A request suspension ends in exactly one of two ways (an answer, or passing the request on), and it can be abandoned.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// `posturn::request::State` (private).
enum State<T> {
   /// The UI holds the resume handle and hasn't used it yet.
   Waiting,
   /// The host answered; `forward` takes the value on its next poll.
   Filled(T),
   /// The UI passed the request on to the listeners below.
   Passed,
   /// `forward` has taken the outcome.
   Taken,
   /// The resume handle was dropped without being used.
   Abandoned,
}

/// `posturn::request::Slot` (private).
type Slot<T> = Rc<RefCell<State<T>>>;

/// `posturn::request::Resume`. Handed to a host listener registered with `Host::on_request`.
pub struct Resume<T> {
   slot: Slot<T>,
   /// Dies with the `Host` (which owns one `Rc<Registry>` and, through the game future, the other).
   registry: Weak<Registry>,
}

impl<T> Resume<T> {
   /// Answers the request, ending the chain here. Consumes the resume handle, so a request is handled at most once.
   ///
   /// Returns `Err(value)`, handing the value back, if the `Host` that made the request has been dropped (for
   /// example, after "New Game"), since no game is left to receive it.
   pub fn with(self, value: T) -> Result<(), T> {
      if self.registry.strong_count() == 0 {
         return Err(value);
      }
      *self.slot.borrow_mut() = State::Filled(value);
      Ok(())
   }

   /// Passes the request on to the listeners below this one, whose response goes back to the game. Harmless if the
   /// `Host` has been dropped, since nothing will read the slot.
   pub fn resume(self) {
      *self.slot.borrow_mut() = State::Passed;
   }
}

impl<T> Drop for Resume<T> {
   fn drop(&mut self) {
      // `with` or `resume` has already filled the slot; anything else is an abandoned request. If the `Host` is gone,
      // nobody will ever check, so dropping a stale resume handle is harmless.
      let mut state = self.slot.borrow_mut();
      if matches!(*state, State::Waiting) { *state = State::Abandoned; }
   }
}

/// `posturn::request::Future` (private). What `forward` awaits at a host listener: `Some(value)` if the
/// host answered, `None` if it passed the request on.
struct Future<T> { slot: Slot<T> }

impl<T> std::future::Future for Future<T> {
   type Output = Option<T>;
   fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<T>> {
      let mut state = self.slot.borrow_mut();
      match std::mem::replace(&mut *state, State::Taken) {
         State::Filled(value) => Poll::Ready(Some(value)),
         State::Passed => Poll::Ready(None),
         State::Waiting => { *state = State::Waiting; Poll::Pending }
         // `Host::step()` panics on an abandoned request before polling, so these can't be reached.
         State::Abandoned | State::Taken => unreachable!(),
      }
   }
}
```
<!-- SPDX-SnippetEnd -->

Neither resume handle is `Clone` or `Send`: there is exactly one continuation per suspension.

### Handles that outlive their `Host`

**Decided:** any handle can outlive its game, for example when the UI starts a new game while still holding the old
turn's prompt, or a screen that still holds its listener handles. Each handle shares only its own slot or entry, so
nothing breaks:

| Leftover handle | What happens |
| --- | --- |
| `event::Resume` dropped | sets `done` on a slot nobody reads; harmless |
| `request::Resume` dropped | marks the slot `Abandoned`; nobody checks it, so this is **not** an error |
| `resume.with(value)` | returns `Err(value)`: the value is handed back instead of being silently discarded |
| `resume.resume()` | marks the slot `Passed`; nobody reads it; harmless |
| `event::Handle` / `request::Handle` dropped | its `Weak` entry is already gone, so there's nothing to unregister |

`request::Resume` keeps a `Weak<Registry>` to detect this. The `Result` from `with` is `#[must_use]`, so a UI that
doesn't care writes `let _ = resume.with(value);` (or `.ok()`); one that does can, say, discard a stale prompt.

## Current suspension

So that `step()` can avoid polling when nothing would change, the registry ([`Registry`](./host.md#inside-the-registry),
shared by `Host` and `CtxMut`) records what the game is currently parked on. The two cases stay separate here too:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
enum Suspension {
   /// Parked on an event; continues once the `event::Resume` is dropped.
   Event {
      /// `std::any::type_name` of the event, for diagnostics.
      event_type: &'static str,
      done: event::Slot,
   },
   /// Parked on a request; continues once the `request::Resume` is used, and panics if it was abandoned.
   Request {
      /// `std::any::type_name` of the request, e.g. for "request::Resume<PartyInput> for PartyInputRequest was
      /// dropped without being used".
      request_type: &'static str,
      /// The slot's state with `T` erased: is it waiting, filled, passed on, or abandoned?
      status: Rc<dyn Fn() -> RequestStatus>,
   },
}

enum RequestStatus { Waiting, Filled, Passed, Abandoned }
```
<!-- SPDX-SnippetEnd -->

Before polling, `step()` checks it:

| Current suspension | `step()` |
| --- | --- |
| none | polls the game |
| `Event`, `done` is `false` | returns `Poll::Pending` without polling |
| `Event`, `done` is `true` | polls the game |
| `Request`, `Waiting` | returns `Poll::Pending` without polling |
| `Request`, `Filled` or `Passed` | polls the game |
| `Request`, `Abandoned` | panics, naming the request type |

At a host listener, `forward` sets `current` just before awaiting the `event::Future` or `request::Future`, and clears
it once that future completes. If the loop then carries on and a later host listener suspends again, `current` is set
anew.

## The no-op waker

`Host::step()` polls with `Waker::noop()` (stable since Rust 1.85, well within the 1.86 MSRV).
Wakeups are meaningless here: the caller always knows when to poll again, namely after dropping an `event::Resume` or
using a `request::Resume`.

## Why not `genawaiter`?

`genawaiter` gave `v0.3` one `Yield` type and one `Resume` type per coroutine. That restriction is the root of
[input bloat](../input_bloat.md). With a fresh, typed slot for every suspension, every request has its own response
type, events carry no response at all, and the public API no longer needs `Coroutine`, `GeneratorState`, or
`pin_mut!`.
