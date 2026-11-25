<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Events, requests, and listeners

The game communicates through two kinds of message, which are deliberately kept apart.

By convention, the examples in this plan name event types with an `Event` suffix (`PartyMovedEvent`) and request types
with a `Request` suffix (`PartyInputRequest`), so it's obvious at every use which kind of message a type is.

## Events are broadcasts

**Decided:** an **event** is something that happened in the game that others may want to react to. Event types must
be **declared explicitly** by implementing the `Event` trait, so that only types designed to be events can be emitted
(and not, say, an arbitrary `Vec<HashSet<Widget>>`). Emitting one suspends the game until the event has been handled
(for example, until the UI has finished animating it), and returns nothing:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// Declares a type as an event that the game can emit.
pub trait Event: 'static {
   /// Orders this event's listeners: the greatest runs first. Use `()` if only registration order should matter.
   type Priority: Ord + 'static;
}

/// The party moved one step.
pub struct PartyMovedEvent { pub direction: Direction }
impl Event for PartyMovedEvent { type Priority = Priority; } // The game's own `Priority`, below.

ctx.emit(PartyMovedEvent { direction }).await; // -> ()
```
<!-- SPDX-SnippetEnd -->

## Requests expect a response

**Decided:** a **request** asks for something the game can't continue without, typically a player's decision. A
request type implements `Request`, whose associated `Response` is what the game receives:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
pub trait Request: 'static {
   /// What `ctx.submit(..)` resolves to.
   type Response: 'static;
   /// Orders this request's listeners: the greatest runs first. Use `()` if only registration order should matter.
   type Priority: Ord + 'static;
}

/// The game needs to know what the party does next.
pub struct PartyInputRequest;
impl Request for PartyInputRequest {
   type Response = PartyInput;
   type Priority = Priority;
}

match ctx.submit(PartyInputRequest).await {
   PartyInput::Move { direction } => { /* ... */ }
   PartyInput::Rest { hours } => { /* ... */ }
   PartyInput::UseItem { item_idx } => { /* ... */ }
}
```
<!-- SPDX-SnippetEnd -->

Together these solve [input bloat](../input_bloat.md), [null input](../null_input.md), and
[events vs. input](../events_vs_input.md): every request has its own response type, events need no response at all,
and the two can never be confused. Both `emit` and `submit` take `&mut CtxMut`, which is what makes holding a module
borrow across them a [compile error](./host.md#the-borrowing-rule).

### One type, one role

**Decided:** a type should be _either_ an `Event` _or_ a `Request`, never both. Nothing in the design breaks if a type
implements both traits (events and requests have entirely separate chains, listeners, handles, and futures, so
`ctx.emit(x)` and `ctx.submit(x)` would simply travel different chains), and stable Rust can't forbid it without
negative trait bounds. But the two would share a name while having unrelated listeners, which is confusing to read
and easy to get wrong. The documentation for both traits will discourage it, and recommend separate types instead
(e.g. `PartyMovedEvent` as the event and `MoveRequest` as the request).

## Priorities

**Decided:** every listener is registered with a **priority**, whose type is the message's associated `Priority`
(any `Ord` type). Priorities decide the order of a message's listeners (see [Ordering](#ordering)).

**`()` is a valid priority.** A message whose listeners should be ordered by registration alone uses
`type Priority = ();`, and registers every listener with `()`. All of its listeners then have equal priority, so the
most recently registered runs first, like a plain stack of overrides.

A game typically defines one priority type for all of its messages, naming the layers it's built from. Deriving `Ord`
on an enum orders its variants by declaration, so the variant declared last runs first:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// Who sees a message first: mods, then the game's own rules, then the UI.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
   Ui,
   Rules,
   Mods,
}
```
<!-- SPDX-SnippetEnd -->

**Why `Ord`, not `PartialOrd`?** Inserting a listener before the first one of equal or lower priority would still
produce a valid order for a `PartialOrd` priority. But two _incomparable_ priorities would be ordered by whichever was
registered first, the opposite of the newest-first rule for equal priorities, and harder to predict. Requiring `Ord`
keeps the rule simple: every pair of listeners is ordered by priority, then by registration. A game that wants
floating-point priorities can wrap them in a newtype whose `Ord` uses `f32::total_cmp`.

## Listeners form a chain

**Decided:** for each event type and each request type, the registry keeps a **definition**
([`event::Def<E>` / `request::Def<R>`](#definitions-and-dispatch)) holding one chain of listeners, sorted by
priority. There are two kinds of listener, and both live in the same chain:

- **Game listeners** are `async` functions or async closures. They are registered through `Registry` (during
  [setup](./games.md)) or `CtxMut` (at any time during the game).
- **Host listeners** are plain callbacks that run outside the game, in the host application: the UI, but just as well
  audio, networking, or logging. They are registered through the `Host`, at any time; see
  [Handing control to the UI](./ui_handlers.md).

A game listener is written in **middleware style**: it receives the context, the
message by value, and a handle for the rest of the chain. Events and requests follow the same pattern, but with
separate types throughout: an event listener gets a `event::Next<E>` and returns `()`; a request listener gets a
`request::Next<R>` and returns `R::Response`.

**Decided:** listeners can be registered **at any time**, and registering one returns a **handle** that keeps it
registered (see [Handles](#handles)):

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Outcome + 'static {
   // An event listener, as an async fn item. `forever()` gives up its handle, so it stays registered for good...
   registry.on_event(Priority::Rules, on_party_moved).forever();

   // ...and a request listener, as an async closure.
   registry.on_request(Priority::Rules, async |ctx: &mut CtxMut, req: PartyInputRequest, next| {
      let input = next.forward(ctx, req).await;
      // Code after the rest of the chain, which may inspect or replace the response...
      input
   }).forever();

   async move |ctx: CtxMut| play(ctx).await
}

async fn on_party_moved(ctx: &mut CtxMut, ev: PartyMovedEvent, next: event::Next<PartyMovedEvent>) {
   // Code before the rest of the chain (e.g. advance the clock, run triggers)...
   evaluate_triggers(ctx, &ev).await;

   // Defer to the next listener, like calling the base implementation of a `virtual` fn.
   next.forward(ctx, ev).await
}
```
<!-- SPDX-SnippetEnd -->

The same two methods exist on `Registry`, `CtxMut`, and `Host`, and all of them take `&self`, since registering never
suspends the game:

- `on_event<E: Event>(&self, priority: E::Priority, F) -> event::Handle` with
  `F: AsyncFn(&mut CtxMut, E, event::Next<E>) + 'static` (on `Registry` and `CtxMut`).
- `on_request<R: Request>(&self, priority: R::Priority, F) -> request::Handle` with
  `F: AsyncFn(&mut CtxMut, R, request::Next<R>) -> R::Response + 'static` (on `Registry` and `CtxMut`).
- On the `Host`, `F` is a [plain callback](./ui_handlers.md#on_event-and-on_request) instead. The UI can't register
  game listeners, which would receive `&mut CtxMut` and with it [write access](./host.md#the-ui-only-reads) to
  modules.

Because `request::Next<R>` knows `R::Response` from the `Request` impl, a request listener written as an `async fn`
only names the request type: `next: request::Next<PartyInputRequest>`. In closures both are inferred. `AsyncFn` needs
Rust 1.85.

A game listener can:

- **Modify or replace** the message before passing it on with `next.forward(ctx, msg)`. (Both `event::Next` and
  `request::Next` have a `forward` method; they differ in what it returns.)
- **Post-process** a request's response: run code after `next.forward(..)` returns, and change the value before
  returning it. This includes changing input that came back from a [host listener](./ui_handlers.md).
- **Override** the rest of the chain by not calling `next` at all (returning a response of its own, for requests).
  This can prevent the UI from ever seeing the message.
- **`await` anything that takes `&mut CtxMut`**, including emitting other events, making other requests, or running a
  whole minigame.
- **Register or unregister listeners,** including for its own message type. A listener registered while a message is
  on its way down the chain only sees later messages; see [Definitions and dispatch](#definitions-and-dispatch).

`forward` consumes the handle, so the rest of the chain runs at most once per listener invocation.

## Handles

**Decided:** registering a listener returns an **RAII handle**: an `event::Handle` from `on_event`, or a
`request::Handle` from `on_request`. **Dropping the handle unregisters the listener**, and drops its closure straight
away. A listener stays registered exactly as long as whoever owns its handle:

- **Listeners that should never be unregistered** give up their handles with **`.forever()`**, which consumes the
  handle without unregistering the listener, as in the setup above. The listener then stays registered for as long as
  the `Registry` exists, which is until the `Host` is dropped.
- **Listeners that last until something in the game changes** keep their handles **in a [module](./host.md#modules)**,
  on the object that represents that something, as `StatusModule` does below. Modules live exactly as long as the
  game, so the handle does too, unless the game drops it first.
- **Listeners that last as long as something else** keep their handles in whatever represents that thing: a field of a
  struct, or a local variable in an `async fn` that only listens while it runs.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// The party's status effects. Each effect owns the handles of the listeners that implement it.
pub struct StatusModule { pub poison: Option<Poison> }
impl Module for StatusModule {}

pub struct Poison { _on_move: event::Handle }

// While the party is poisoned, every step costs health.
let on_move = ctx.on_event(Priority::Rules, async |ctx: &mut CtxMut, ev: PartyMovedEvent, next| {
   ctx.borrow_mut::<OverworldModule>().damage_party(1);
   next.forward(ctx, ev).await
});
ctx.borrow_mut::<StatusModule>().poison = Some(Poison { _on_move: on_move });

// Later, curing the poison drops the `Poison`, and with it the handle, which unregisters the listener.
ctx.borrow_mut::<StatusModule>().poison = None;
```
<!-- SPDX-SnippetEnd -->

- **Handles are `#[must_use]`.** Writing `ctx.on_event(..);` on its own warns, because the handle would be dropped at
  the end of the statement, and the listener with it. The fix is to keep the handle, or to call `.forever()`.
- **Handles aren't generic.** An `event::Handle` doesn't name its event type, so one `Vec<event::Handle>` can hold the
  handles for any number of event types (and a `Vec<request::Handle>`, for any number of request types).
- **Dropping a handle is always safe.** It never borrows the registry, so it can happen anywhere: inside a listener
  (including the listener being unregistered), inside a host listener, or while the registry itself is being dropped
  (for example, a module's handles when its game ends). A handle that outlives its `Host` unregisters nothing.
- **Handles are neither `Clone` nor `Send`.** There is exactly one per registration.

## Ordering

**Decided:** a message's listeners run in order of **priority, greatest first**. **Among equal priorities, the most
recently registered runs first.** Within one priority, registering a listener therefore wraps the existing behavior,
and it decides whether to `forward` into it, like a stack of overrides. Across priorities, the priority type decides
which layer wraps which.

"Runs first" means outermost: the first listener sees the message first, and decides whether and when the rest of the
chain sees it. With the `Priority` enum above, a mod's listeners wrap the game's rules, and the rules wrap the UI.

**The UI is not special.** Host listeners registered through the `Host` are sorted into the same chain as game
listeners, by the same rule. A game that wants the UI to see a message last (after its rules have run) gives the
UI the lowest priority, as with `Priority::Ui` above. Several host listeners for one message type each get their place
in the chain; see [Handing control to the UI](./ui_handlers.md#sorted-into-the-chain).

Newest-first within a priority suits listeners that come and go during the game. A listener registered for the length
of a cutscene or a modal screen intercepts the messages at its priority, and gives control back simply by having its
handle dropped.

A richer way to state ordering (dependencies between module listeners, expressed through a builder or a procedural
macro) is [future work](./future_work.md).

## Reaching the end of the chain

A message reaches the end of its chain when no listener is registered for its type, or when the last listener passes
it on: a game listener calls `next.forward`, or a [host listener](./ui_handlers.md) lets an event continue or passes a
request on with `resume()`.

- **Unhandled events complete immediately. Decided.** An event is a broadcast; if nobody is listening, there is
  nothing to wait for, and `emit` (or the `next.forward`) simply returns `()` without suspending. A UI therefore only
  registers listeners for the events it actually presents, such as the ones it animates.
- **Unhandled requests panic. Decided.** The game can't continue without a response, so `posturn` panics with a
  message naming the request type.

## Definitions and dispatch

**Decided:** each message type's listeners live in a **definition**, which holds one sorted chain of **listeners**.
Each listener holds its priority and its **callback**, which is one of two kinds: `Callback::Game` (an `AsyncFn` that
runs inside the game, registered through `Registry` or `CtxMut`) or `Callback::Host` (a plain callback that runs
outside it, in the host application, registered through `Host`). Both kinds share one chain, one `Listener` type, and
one kind of handle.

The `Registry` stores definitions in a `HashMap` keyed by the **message type** (`TypeId::of::<E>()` or
`TypeId::of::<R>()`), as `Box<dyn event::AnyDef>` / `Box<dyn request::AnyDef>`. Dispatch downcasts to the concrete
`Def<E>` / `Def<R>`, so the real `Event`, `Request`, and `Response` types appear throughout, and no message is ever
boxed as `dyn Any` (see [Storing modules and definitions](./host.md#storing-modules-and-definitions)).

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
// In `posturn::event` (all private except `Next` and `Handle`):

/// The definition of event `E`.
struct Def<E: Event> {
   /// Sorted: greatest priority first; among equal priorities, the most recently registered first.
   chain: Vec<Rc<Listener<E>>>,
}

/// One registered listener.
struct Listener<E: Event> {
   priority: E::Priority,
   /// `None` once the listener's `Handle` has been dropped.
   callback: RefCell<Option<Callback<E>>>,
}

/// What a listener runs. Cloning one only clones an `Rc`. (`Clone` is implemented by hand, since deriving it would
/// require `E: Clone`.)
enum Callback<E: Event> {
   /// A game listener: the user's `AsyncFn(&mut CtxMut, E, Next<E>)`, wrapped so that each call boxes its future,
   /// which is also what lets a listener's `next.forward(..)` recurse.
   Game(Rc<dyn for<'a> Fn(&'a mut CtxMut, E, Next<E>) -> LocalBoxFuture<'a, ()>>),
   /// A host listener: the host application's `FnMut(Ctx<'_>, &E, Resume)`. See "Handing control to the UI".
   Host(Rc<RefCell<dyn FnMut(Ctx<'_>, &E, Resume)>>),
}

/// Any `Def<E>`, whatever its `E`: what the `Registry`'s `events` map stores.
trait AnyDef: Any {
   /// `std::any::type_name::<E>()`, for diagnostics.
   fn event_type(&self) -> &'static str;
}

impl dyn AnyDef {
   /// Downcasts to the definition of `E`. Inherent methods on the trait object (like `downcast_ref` on `dyn Any`),
   /// because a generic method can't be called through a trait object. They panic if this isn't `E`'s definition,
   /// which can't happen for a definition looked up under `TypeId::of::<E>()`.
   fn downcast_ref<E: Event>(&self) -> &Def<E>;
   fn downcast_mut<E: Event>(&mut self) -> &mut Def<E>;
}

/// The rest of the chain, handed to each game listener.
pub struct Next<E: Event> {
   /// A copy of the chain, taken when the event was emitted.
   chain: Vec<Rc<Listener<E>>>,
   /// The index of the next listener to try.
   at: usize,
}

impl<E: Event> Next<E> {
   pub async fn forward(mut self, ctx: &mut CtxMut, event: E) {
      while self.at < self.chain.len() {
         // Clone the callback out of its listener, so that the call keeps it alive even if the listener's handle is
         // dropped while it runs.
         let callback = self.chain[self.at].callback.borrow().clone();
         self.at += 1;
         match callback {
            // Its handle was dropped after the copy was taken: skip it.
            None => {}
            // A game listener decides for itself whether (and when) the rest of the chain runs.
            Some(Callback::Game(f)) => return f(ctx, event, self).await,
            // A host listener holds the game until its `Resume` drops; then the loop carries on with the same event.
            Some(Callback::Host(f)) => {
               let done = Slot::default();
               (f.borrow_mut())(ctx.ctx(), &event, Resume { done: done.clone() });
               Future { done }.await; // Records the current suspension while it waits.
            }
         }
      }
      // The end of the chain: the event simply completes.
   }
}
```
<!-- SPDX-SnippetEnd -->

### Dispatch

`ctx.emit(ev)`:

1. Borrows the `Registry`'s `events` map and looks up `TypeId::of::<E>()`. If there is no definition, the event is
   [unhandled](#reaching-the-end-of-the-chain) and `emit` returns immediately.
2. Calls `def.downcast_ref::<E>()` to get the `&Def<E>`, and **copies its chain** into a fresh `Next { chain, at: 0 }`.
   This clones the `Vec`, which bumps one `Rc` per listener. The map's borrow ends here, before any listener runs.
3. Calls `next.forward(ctx, ev)`, with `ev` still an `E`.

`forward` walks its copy of the chain in one loop:

- it **skips** listeners whose callback is `None`;
- it **hands over** to the first live **game** listener, passing it a `Next` that points past it, and returns whatever
  that listener returns. Each `Next` hands the same copy on, so the chain is copied once per `emit`, not once per
  listener;
- it **waits** at each **host** listener until its `event::Resume` drops, then carries on in the same loop;
- it reaches the [end of the chain](#reaching-the-end-of-the-chain) when no listeners are left.

**The copy is a snapshot.** Changes to the chain made while a message is on its way down it affect that message as
follows:

- **A listener registered mid-dispatch isn't in the copy,** so it only sees later messages. Registering never
  disturbs a dispatch in progress.
- **A listener whose handle drops mid-dispatch is skipped,** if the dispatch hasn't reached it yet. The handle clears
  the shared `Option` in the listener, which every copy of the chain sees.
- **A listener that is already running finishes normally**, even if its own handle is dropped while it runs: the call
  holds its own clone of the callback's `Rc`, so the callback is freed when the call returns.

### Registration

`Registry::on_event` and `CtxMut::on_event` wrap the user's `AsyncFn` into a `Callback::Game`, and `Host::on_event`
wraps the host application's callback into a `Callback::Host`. Then each borrows the `events` map mutably, gets or
creates `E`'s definition, downcasts it with `downcast_mut::<E>()`, and inserts the new listener:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
impl<E: Event> Def<E> {
   fn insert(&mut self, priority: E::Priority, callback: Callback<E>) -> Handle {
      // Forget listeners whose handles were dropped since the last registration.
      self.chain.retain(|listener| listener.callback.borrow().is_some());

      // Before the first listener of equal or lower priority: the newest of equals runs first.
      let at = self.chain.iter().position(|listener| listener.priority <= priority).unwrap_or(self.chain.len());
      let listener = Rc::new(Listener { priority, callback: RefCell::new(Some(callback)) });
      // A separate `let`, so that `Some(weak)` coerces the `Weak<Listener<E>>` to `Weak<dyn AnyListener>`. (Inline,
      // inference would ask `downgrade` for a `Weak<dyn AnyListener>` directly, and fail to compile.)
      let weak: Weak<Listener<E>> = Rc::downgrade(&listener);
      let handle = Handle { listener: Some(weak) };
      self.chain.insert(at, listener);
      handle
   }
}

/// Returned by `on_event`. Dropping it unregisters the listener.
#[must_use = "dropping the handle unregisters the listener immediately; call `.forever()` to keep it"]
pub struct Handle {
   /// A `Weak`, so that a handle never keeps its listener (or the `Registry`) alive. `None` after `forever`.
   listener: Option<Weak<dyn AnyListener>>,
}

impl Handle {
   /// Gives up the handle without unregistering the listener, which then stays registered for as long as the
   /// `Registry` exists.
   pub fn forever(mut self) {
      self.listener = None; // Nothing left for `drop` to clear.
   }
}

/// Lets a `Handle` clear its `Listener<E>` without knowing `E`.
trait AnyListener {
   fn clear(&self);
}

impl<E: Event> AnyListener for Listener<E> {
   fn clear(&self) {
      // `take` releases the `RefCell` before the callback drops, in case dropping it drops other handles.
      let callback = self.callback.take();
      drop(callback);
   }
}

impl Drop for Handle {
   fn drop(&mut self) {
      if let Some(listener) = self.listener.take().and_then(|weak| weak.upgrade()) {
         listener.clear();
      }
   }
}
```
<!-- SPDX-SnippetEnd -->

Cleared listeners stay in the chain until the next registration for the same message type prunes them; until then,
dispatch skips them. Nothing borrows the `events` map across an `.await` or a listener call, so registering from inside
a listener (or from the UI between steps) can never find it borrowed.

**Requests work the same way** in `posturn::request`. `request::Def<R>` holds `request::Listener<R>`s, and is stored as
`Box<dyn request::AnyDef>`. A `request::Callback::Game` returns `R::Response`, and the `Host` arm of `forward` either
returns the host's answer or carries on down the chain:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
enum Callback<R: Request> {
   Game(Rc<dyn for<'a> Fn(&'a mut CtxMut, R, Next<R>) -> LocalBoxFuture<'a, R::Response>>),
   Host(Rc<RefCell<dyn FnMut(Ctx<'_>, &R, Resume<R::Response>)>>),
}

impl<R: Request> Next<R> {
   /// Like `event::Next::forward`, but returns the response, and panics at the end of the chain.
   pub async fn forward(mut self, ctx: &mut CtxMut, request: R) -> R::Response;
}

trait AnyDef: Any {
   fn request_type(&self) -> &'static str;
}
```
<!-- SPDX-SnippetEnd -->

`ctx.submit(req)` looks up `R`'s definition and copies its chain the same way, then calls `next.forward(ctx, req)` and
gets an `R::Response` back directly. With no definition, or at the end of the chain, an unhandled request panics,
naming the request type.
