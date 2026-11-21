<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Handing control to the UI

The UI cannot `await` inside the game coroutine: it lives _outside_ the coroutine and is the thing that drives it. So
the UI's link in a listener chain is not an `async` function. It is a plain callback that receives the message and a
**handle**. The UI stores the handle, carries on rendering and reading input across as many frames as it needs, and
eventually uses the handle to let the game continue.

## `on_event` and `on_request`

**Decided:** the UI registers its callbacks on the `Host`, with the same two method names the game uses on `Registry`.
Events and requests each get their own kind of handle:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
let mut host = Host::new(rpg::Setup::from_save(&save));

// Events: the callback gets an `event::Handle`.
host.on_event(|ctx: Ctx<'_>, ev: PartyMovedEvent, handle: event::Handle| {
   ui.borrow_mut().start_walk_animation(ev.direction, handle); // Resumes when the animation drops it.
});

// Requests: the callback gets a `request::Handle<R::Response>`.
host.on_request(|ctx: Ctx<'_>, req: PartyInputRequest, handle: request::Handle<PartyInput>| {
   ui.borrow_mut().prompt_party_action(handle); // Later: let _ = handle.resume(PartyInput::Rest { hours });
});
```
<!-- SPDX-SnippetEnd -->

- `on_event<E: Event>(&mut self, ..)`: the callback is `FnMut(Ctx<'_>, E, event::Handle) + 'static`.
- `on_request<R: Request>(&mut self, ..)`: the callback is `FnMut(Ctx<'_>, R, request::Handle<R::Response>) + 'static`.

**Decided:** every UI callback receives a read-only [`Ctx`](./host.md#ctxa-the-read-only-view), so it can look up
modules (for example, to find out what an animation should show) without going through shared UI state. This is
safe: when a UI callback runs, every frame of the game is parked at an `emit`, `submit`, or `next.forward`, all of which
hold `&mut CtxMut`, so the game holds no module borrows. The `Ctx` can't escape the callback, because of its
lifetime.

Callbacks are `'static` and can't capture the `Host`, so the UI state they need is typically shared with the UI loop
through an `Rc<RefCell<..>>` (the `ui` above).

Event and request suspensions are implemented separately, each with its own slot and `Future`; see
[Under the hood](./coroutines.md).

### `event::Handle`

- **`event::Handle` has no methods. Dropping it is what lets the game continue.** This makes "play an animation, then
  continue" trivial: whatever owns the animation owns the `event::Handle` and drops it when finished. A UI that doesn't
  need to pause for an event simply drops the handle in its callback.
- Dropping an `event::Handle` can never be an error, since there is no response to forget.

### `request::Handle<R>`

- `handle.resume(value: R) -> Result<(), R>` supplies the response and lets the game continue. It consumes the handle,
  so a request is answered at most once. If the `Host` has since been dropped (e.g. after "New Game"), it returns
  `Err(value)` instead; see [Handles that outlive their `Host`](./coroutines.md#handles-that-outlive-their-host).
- **Dropping a `request::Handle<R>` without resuming is an error.** The drop marks the slot as _abandoned_; the next
  `step()` panics with a message naming the request type, e.g.
  `request::Handle<PartyInput> for PartyInputRequest was dropped without being resumed`. (`Drop` itself does not
  panic, because panicking during an unwind aborts the process.) Once the `Host` is gone there is no next `step()`,
  so dropping a leftover handle is harmless.

## Always at the bottom

**Decided:** the UI callback for a message type sits in a dedicated slot at the **bottom** of that type's chain, below
every game listener: the `ui` field of the message's [definition](./events.md#definitions-and-dispatch). Game
listeners are registered during setup, inside `Host::new`, so they exist before the UI can register anything. A plain
most-recent-first order would put the UI callback on top, which is why it gets its own slot.

There is at most one UI callback per message type. The slot is a `OnceCell`, so registering a second one for the same
type fails to `set` it, and `posturn` panics. If the game registered no listeners for a message type, `Host::on_event` /
`Host::on_request` creates its definition, holding just the UI callback.

**Decided: UI callbacks must be registered before the first `step()`, and this is checked at runtime.** Calling
`on_event` or `on_request` after the first `step()` panics with a message like "UI callbacks must be registered before
the first `step()`". This catches a late registration at the call site, instead of letting it surface far away: an
event the game emitted earlier would have [completed](./events.md#reaching-the-end-of-the-chain) without the UI ever
seeing it, and a request it made earlier would already have panicked as unhandled. (A UI that wants to present the
same message differently over time, say on different screens, registers one callback that dispatches to whatever is
current.)

A *missing* request callback still can't be detected up front, because `posturn` can't know which requests the game
will make; it remains a clear panic naming the request type when the game first makes it.

A UI doesn't need a callback for every event, only for the ones it presents. Every request the game can make, on the
other hand, needs either a UI callback or a game listener that answers it.

## Resuming early

A callback may resume its handle immediately, inside the callback. In that case the game never actually suspends:
the UI callback's future is ready on its first poll, and the game carries on within the same `step()`. This is useful
for UIs (or headless test harnesses, or AI players) that can answer synchronously.

## Under the hood

Each UI callback is wrapped into an ordinary listener, an `event::Listener` or `request::Listener` (holding the user's
`FnMut` in a `RefCell`, since listeners are called through `Fn`), and stored in the `ui` slot of its message type's
[definition](./events.md#definitions-and-dispatch). When that listener runs, it creates a fresh
slot, calls the callback with the message and a handle pointing at the slot, records the slot as the registry's _current
suspension_, and then awaits a future on the slot. That future is `Pending` until the handle fills the slot. The `Host`
checks the current suspension in [`step()`](./driving.md) to tell whether polling would make progress. See [Under the
hood](./coroutines.md) for the slot itself.
