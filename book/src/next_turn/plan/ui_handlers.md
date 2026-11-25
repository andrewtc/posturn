<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Handing control to the UI

The UI cannot `await` inside the game coroutine: it lives _outside_ the coroutine and is the thing that drives it. So
the UI's links in a listener chain are not `async` functions. Each one is a plain callback that receives the message
and a **resume handle**. The UI stores the resume handle, carries on rendering and reading input across as many frames
as it needs, and eventually uses it to let the game continue.

These plain callbacks are **host listeners**: they run outside the game, in the host application that owns the
`Host`. The UI is the most common example, and the one this page uses, but a host listener can just as well play a
sound, send a message over the network, or write a log.

## `on_event` and `on_request`

**Decided:** the UI registers its listeners on the `Host`, with the same two method names the game uses on `Registry`
and `CtxMut`. Like the game's, each one is registered with a [priority](./events.md#priorities) and returns a
[handle](./events.md#handles) that keeps it registered. Events and requests each get their own kind of resume handle:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
let mut host = Host::new(rpg::Setup::from_save(&save));

// Events: the callback gets the event by reference, and an `event::Resume`.
let walk = host.on_event(Priority::Ui, |ctx: Ctx<'_>, ev: &PartyMovedEvent, resume: event::Resume| {
   ui.borrow_mut().start_walk_animation(ev.direction, resume); // Continues when the animation drops it.
});

// Requests: the callback gets the request by reference, and a `request::Resume<R::Response>`.
let prompt = host.on_request(Priority::Ui, |ctx: Ctx<'_>, req: &PartyInputRequest, resume| {
   ui.borrow_mut().prompt_party_action(resume); // Later: let _ = resume.with(PartyInput::Rest { hours });
});
```
<!-- SPDX-SnippetEnd -->

- `on_event<E: Event>(&self, priority: E::Priority, F) -> event::Handle`, where the callback is
  `F: FnMut(Ctx<'_>, &E, event::Resume) + 'static`.
- `on_request<R: Request>(&self, priority: R::Priority, F) -> request::Handle`, where the callback is
  `F: FnMut(Ctx<'_>, &R, request::Resume<R::Response>) + 'static`.

**Decided: a host listener receives the message by reference.** The UI only looks at a message; it never changes it.
That way the same message can carry on [down the chain](#sorted-into-the-chain) once the UI is done with it. A UI that
needs part of the message later (say, the direction of a walk animation) copies or clones that part.

**Decided:** every host listener also receives a read-only [`Ctx`](./host.md#ctxa-the-read-only-view), so it can look up
modules (for example, to find out what an animation should show) without going through shared UI state. This is
safe: when a host listener runs, every frame of the game is parked at an `emit`, `submit`, or `next.forward`, all of
which hold `&mut CtxMut`, so the game holds no module borrows. The `Ctx` can't escape the callback, because of its
lifetime.

Callbacks are `'static` and can't capture the `Host`, so the UI state they need is typically shared with the UI loop
through an `Rc<RefCell<..>>` (the `ui` above).

Event and request suspensions are implemented separately, each with its own slot and `Future`; see
[Under the hood](./coroutines.md).

### `event::Resume`

- **`event::Resume` has no methods. Dropping it lets the event continue** down the chain, and the game with it. This
  makes "play an animation, then continue" trivial: whatever owns the animation owns the `event::Resume` and drops it
  when finished. A UI that doesn't need to pause for an event simply drops it in its callback.
- Dropping an `event::Resume` can never be an error, since there is no response to forget.

### `request::Resume<T>`

A `request::Resume<T>` (where `T = R::Response`) must be used in one of two ways. Both consume it, so a request is
handled at most once, and both let the game continue:

- **`resume.with(value) -> Result<(), T>` answers the request.** The chain ends here: no listener below this one sees
  the request. The response travels back up through the listeners above, which may
  [post-process](./events.md#listeners-form-a-chain) it, and on to the game. If the `Host` has since been dropped (e.g.
  after "New Game"), it returns `Err(value)` instead; see [Handles that outlive their
  `Host`](./coroutines.md#handles-that-outlive-their-host).
- **`resume.resume()` passes the request on** to the listeners below this one, as if this host listener hadn't been
  there, and their response goes back to the game. This lets a host listener answer only some of the time. For example,
  a debug console can answer input requests while it's open, and pass them on to the normal prompt while it's closed.

**Dropping a `request::Resume<T>` without using it is an error.** The drop marks the slot as _abandoned_; the next
`step()` panics with a message naming the request type, e.g.
`request::Resume<PartyInput> for PartyInputRequest was dropped without being used`. (`Drop` itself does not panic,
because panicking during an unwind aborts the process.) Once the `Host` is gone there is no next `step()`, so dropping
a leftover `request::Resume` is harmless.

## Sorted into the chain

**Decided:** host listeners are sorted into each message's chain **together with the game's listeners**, by the same
[ordering rule](./events.md#ordering): greatest priority first, and the most recently registered first among equal
priorities. The UI has no dedicated slot, and there can be **any number of host listeners** per message type.

- **The game's priority type decides where the UI goes.** A game typically gives its UI the lowest priority (like
  `Priority::Ui` in [Priorities](./events.md#priorities)), so the UI sees each message after the game's rules have
  handled it, and the rules can still override or post-process what the UI answers.
- **Several host listeners can present one event, one after another.** Each one's `event::Resume` has to drop before the
  next listener down the chain sees the event. For example, a message log at a higher priority drops its `Resume`
  straight away, and the map view below it then plays the walk animation.
- **For requests, the first host listener to answer wins.** `with(value)` ends the chain; `resume()` passes it on to the
  next one.

## Registering at any time

**Decided:** the UI can register and unregister listeners **at any time**, including between any two `step()`s. A
listener stays registered for as long as the UI keeps its [handle](./events.md#handles), so the natural owner of a
handle is the part of the UI that presents the message. For example, each screen owns the handles for the messages it
presents, and switching screens drops the old one, unregistering its listeners:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// Presents the battle for as long as it's open.
struct BattleScreen {
   events: Vec<event::Handle>,
   requests: Vec<request::Handle>,
   // ...
}

impl BattleScreen {
   fn open(host: &Host<Infallible>, ui: &Rc<RefCell<Ui>>) -> Self;
}
```
<!-- SPDX-SnippetEnd -->

A host listener meant to last as long as the `Host`, such as a message log, can give up its handle instead:
`host.on_event(..).forever();`.

**Registering happens between steps.** A host listener can't register or unregister listeners itself: it can't reach
the `Host`, only a `Ctx`. When something it sees calls for new listeners (say, a `BattleStartedEvent`, after which
the game will make battle requests), it **keeps its `event::Resume`**, which holds the game at that event. The UI loop
then registers the new listeners between steps, and only afterwards drops the `Resume`, so the game can't make a
battle request before something is listening for it. The [worked example](./example.md#the-ui) does exactly this.

A message the game sends while no listener is registered for it is handled as usual: an event
[completes](./events.md#reaching-the-end-of-the-chain) without the UI ever seeing it, and a request panics as
unhandled. So a UI registers its listeners for the requests the game can make before the first `step()` that could
make them, usually right after `Host::new`. A *missing* request listener still can't be detected up front, because
`posturn` can't know which requests the game will make; it remains a clear panic naming the request type when the game
first makes it.

A UI doesn't need a listener for every event, only for the ones it presents. Every request the game can make, on the
other hand, needs a listener that answers it: a host listener, or a game listener.

## Resuming early

A callback may use its resume handle immediately, inside the callback: drop the `event::Resume`, or call `with` or
`resume()` on the `request::Resume`. In that case the game never actually suspends at that listener: its future is
ready on the first poll, and the game carries on within the same `step()`. This is useful for UIs (or headless test
harnesses, or AI players) that can answer synchronously.

## Under the hood

Each callback registered on the `Host` is stored as a `Callback::Host` (the user's `FnMut` in an `Rc<RefCell<..>>`) in
a [listener](./events.md#definitions-and-dispatch) of its message type's chain, sorted in among the game's
`Callback::Game`s. There is no separate wrapper type: when `forward` reaches a `Callback::Host`, it:

1. creates a fresh slot;
2. calls the callback with a `Ctx`, the message by reference, and a resume handle pointing at the slot;
3. records the slot as the registry's _current suspension_, and awaits a future on the slot, which is `Pending`
   until the resume handle is used;
4. carries on according to how it was used.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
// In `request::Next::forward`, inside the loop over the copied chain:
match callback {
   None => {} // Unregistered since the copy was taken.
   Some(Callback::Game(f)) => return f(ctx, request, self).await,
   Some(Callback::Host(f)) => {
      let slot = Slot::default();
      (f.borrow_mut())(ctx.ctx(), &request, Resume::new(&slot, ctx));

      // Records the current suspension while it waits; see "Under the hood".
      match (Future { slot }).await {
         // `resume.with(value)`: the chain ends here.
         Some(response) => return response,
         // `resume.resume()`: the loop carries on with the same request.
         None => {}
      }
   }
}
```
<!-- SPDX-SnippetEnd -->

The event version is the same, except that its future yields `()` once the `event::Resume` drops, after which the loop
always carries on with the same event. Because the loop simply continues, a host listener that lets a message through
needs no `Next` of its own. The `Host` checks the current suspension in [`step()`](./driving.md) to tell whether
polling would make progress. See [Under the hood](./coroutines.md) for the slots themselves.
