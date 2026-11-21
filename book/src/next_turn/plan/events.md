# Events, requests, and listeners

The game communicates through two kinds of message, which are deliberately kept apart.

By convention, the examples in this plan name event types with an `Event` suffix (`PartyMovedEvent`) and request types
with a `Request` suffix (`PartyInputRequest`), so it's obvious at every use which kind of message a type is.

## Events are broadcasts

**Decided:** an **event** is something that happened in the game that others may want to react to. Event types must
be **declared explicitly** by implementing the `Event` marker trait, so that only types designed to be events can be
emitted (and not, say, an arbitrary `Vec<HashSet<Widget>>`). Emitting one suspends the game until the event has been
handled (for example, until the UI has finished animating it), and returns nothing:

```rust,ignore
/// Marks a type as an event that the game can emit.
pub trait Event: 'static {}

/// The party moved one step.
pub struct PartyMovedEvent { pub direction: Direction }
impl Event for PartyMovedEvent {}

ctx.emit(PartyMovedEvent { direction }).await; // -> ()
```

## Requests expect a response

**Decided:** a **request** asks for something the game can't continue without, typically a player's decision. A
request type implements `Request`, whose associated `Response` is what the game receives:

```rust,ignore
pub trait Request: 'static {
   /// What `ctx.submit(..)` resolves to.
   type Response: 'static;
}

/// The game needs to know what the party does next.
pub struct PartyInputRequest;
impl Request for PartyInputRequest { type Response = PartyInput; }

match ctx.submit(PartyInputRequest).await {
   PartyInput::Move { direction } => { /* ... */ }
   PartyInput::Rest { hours } => { /* ... */ }
   PartyInput::UseItem { item_idx } => { /* ... */ }
}
```

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

## Listeners form a chain

**Decided:** for each event type and each request type, the registry keeps a **definition** ([`event::Def<E>` /
`request::Def<R>`](#definitions-and-dispatch)) holding an ordered chain of listeners. A listener is an `async` function
or async closure in **middleware style**: it receives the context, the message by value, and a handle for the rest of
the chain. Events and requests follow the same pattern, but with separate types throughout: an event listener gets a
`event::Next<E>` and returns `()`; a request listener gets a `request::Next<R>` and returns `R::Response`.

Listeners are registered **only during [setup](./games.md)**, on `Registry`, with `on_event` and `on_request`. Once the
game has started, the chains can't change. (The UI's own callbacks are registered on the `Host`, also with
`on_event` and `on_request`; see [Handing control to the UI](./ui_handlers.md).)

```rust,ignore
fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Outcome + 'static {
   // An event listener, as an async fn item...
   registry.on_event(on_party_moved);

   // ...and a request listener, as an async closure.
   registry.on_request(async |ctx: &mut CtxMut, req: PartyInputRequest, next| {
      let input = next.forward(ctx, req).await;
      // Code after the rest of the chain, which may inspect or replace the response...
      input
   });

   async move |ctx: CtxMut| play(ctx).await
}

async fn on_party_moved(ctx: &mut CtxMut, ev: PartyMovedEvent, next: event::Next<PartyMovedEvent>) {
   // Code before the rest of the chain (e.g. advance the clock, run triggers)...
   evaluate_triggers(ctx, &ev).await;

   // Defer to the next listener, like calling the base implementation of a `virtual` fn.
   next.forward(ctx, ev).await
}
```

The bounds are:

- `on_event<E: Event>(&mut self, F)` with `F: AsyncFn(&mut CtxMut, E, event::Next<E>) + 'static`.
- `on_request<R: Request>(&mut self, F)` with `F: AsyncFn(&mut CtxMut, R, request::Next<R>) -> R::Response + 'static`.

Because `request::Next<R>` knows `R::Response` from the `Request` impl, a request listener written as an `async fn`
only names the request type: `next: request::Next<PartyInputRequest>`. In closures both are inferred. `AsyncFn` needs
Rust 1.85.

A listener can:

- **Modify or replace** the message before passing it on with `next.forward(ctx, msg)`. (Both `event::Next` and
  `request::Next` have a `forward` method; they differ in what it returns.)
- **Post-process** a request's response: run code after `next.forward(..)` returns, and change the value before
  returning it. This includes changing input that came back from the [UI callback](./ui_handlers.md).
- **Override** the rest of the chain by not calling `next` at all (returning a response of its own, for requests).
  This can prevent the UI from ever seeing the message.
- **`await` anything that takes `&mut CtxMut`**, including emitting other events, making other requests, or running a
  whole minigame.

`forward` consumes the handle, so the rest of the chain runs at most once per listener invocation.

## Ordering

**Decided (for now):** among the game's listeners, the most recently registered runs first. Registering a listener
therefore wraps the existing behavior, and it decides whether to `continue` into it. This is the same as a stack of
overrides. Because all listeners are registered during setup, the order is fixed for the whole game.

The UI callback for a message type is **always at the bottom** of its chain, below every game listener, no matter
when it was registered. See [Handing control to the UI](./ui_handlers.md#always-at-the-bottom).

A richer way to state ordering (dependencies between module listeners, expressed through a builder or a procedural
macro) is [future work](./future_work.md).

## Reaching the end of the chain

A message reaches the end of its chain when no listener and no UI callback is registered for its type, or when the
last game listener calls `next.forward` with no UI callback below it.

- **Unhandled events complete immediately. Decided.** An event is a broadcast; if nobody is listening, there is
  nothing to wait for, and `emit` (or the `next.forward`) simply returns `()` without suspending. A UI therefore only
  registers callbacks for the events it actually presents, such as the ones it animates.
- **Unhandled requests panic. Decided.** The game can't continue without a response, so `posturn` panics with a
  message naming the request type.

## Definitions and dispatch

**Decided:** each message type's listeners live in a **definition**, which also holds the UI callback and dispatches
messages itself. The `Registry` stores definitions in a `HashMap` keyed by the **message type** (`TypeId::of::<E>()`
or `TypeId::of::<R>()`), as `Rc<dyn event::AnyDef>` / `Rc<dyn request::AnyDef>`. Dispatch first downcasts to the
concrete `Def<E>` / `Def<R>`, so `emit` and `submit` are ordinary methods with the real `Event`, `Request`, and
`Response` types in their signatures, and no message is ever boxed as `dyn Any` (see
[Storing modules and definitions](./host.md#storing-modules-and-definitions)).

```rust,ignore
// In `posturn::event` (all private except `Next`):

/// A game listener or the UI callback for event `E`, type-erased so that different closures and `async fn`s can
/// share one `Vec`. Each call boxes its future, which is also what lets a listener's `next.forward(..)` recurse.
type Listener<E> = Box<dyn for<'a> Fn(&'a mut CtxMut, E, Next<E>) -> LocalBoxFuture<'a, ()>>;

/// The definition of event `E`.
struct Def<E: Event> {
   /// Game listeners, in registration order. The last one runs first.
   listeners: Vec<Listener<E>>,
   /// The UI callback, which always runs last. A `OnceCell` because there's at most one; a second
   /// `Host::on_event::<E>` fails to `set` it and panics.
   ui: OnceCell<Listener<E>>,
}

impl<E: Event> Def<E> {
   /// Runs `event` through this definition's chain: the game listeners, most recent first, then the UI callback.
   async fn emit(self: Rc<Self>, ctx: &mut CtxMut, event: E) {
      Next::start(self).forward(ctx, event).await
   }
}

/// Any `Def<E>`, whatever its `E`: what the `Registry`'s `events` map stores. Only definitions implement it, and it
/// names its event type without downcasting.
trait AnyDef: Any {
   /// `std::any::type_name::<E>()`, for diagnostics.
   fn event_type(&self) -> &'static str;
}

impl<E: Event> AnyDef for Def<E> {
   fn event_type(&self) -> &'static str { std::any::type_name::<E>() }
}

impl dyn AnyDef {
   /// Downcasts to the definition of `E`. An inherent method on the trait object (like `downcast` on `dyn Any`),
   /// because a generic method can't be called through a trait object. Panics if this isn't `E`'s definition, which
   /// can't happen for an entry looked up under `TypeId::of::<E>()`.
   fn downcast<E: Event>(self: Rc<Self>) -> Rc<Def<E>> {
      (self as Rc<dyn Any>).downcast().expect("event definition stored under the wrong type")
   }
}

/// The rest of the chain, handed to each listener.
pub struct Next<E: Event> {
   def: Rc<Def<E>>,
   /// Game listeners not yet run (they run from the end of `def.listeners`, most recent first). Once it reaches zero,
   /// the UI callback runs, and after that the chain has ended.
   remaining: usize,
   ui_done: bool,
}

impl<E: Event> Next<E> {
   pub async fn forward(self, ctx: &mut CtxMut, event: E);
}
```

`ctx.emit(ev)`:

1. Looks up `TypeId::of::<E>()` in the `Registry`'s `events` map and clones the `Rc<dyn event::AnyDef>`, releasing
   the map's borrow straight away. If there is no definition, the event is
   [unhandled](#reaching-the-end-of-the-chain) and `emit` returns immediately.
2. Calls `def.downcast::<E>()` to get an `Rc<event::Def<E>>`. This always succeeds, because the map is keyed by
   `TypeId::of::<E>()`.
3. Calls `def.emit(ctx, ev)`, with `ev` still an `E`.

`forward` consumes the `Next` and:

- if `remaining > 0`, calls `def.listeners[remaining - 1]` with a `Next` whose `remaining` is one less;
- otherwise, if the UI callback hasn't run and there is one, calls it (its own `Next` is already at the end);
- otherwise has reached the [end of the chain](#reaching-the-end-of-the-chain).

Each `Next` holds an `Rc` of the definition, so it's cheap to hand along and stays valid however the chain unfolds.

**Requests work the same way** in `posturn::request`: `request::Def<R>` holds `Listener<R>`s that return
`R::Response`, and is stored as `Rc<dyn request::AnyDef>`:

```rust,ignore
impl<R: Request> Def<R> {
   async fn submit(self: Rc<Self>, ctx: &mut CtxMut, request: R) -> R::Response {
      Next::start(self).forward(ctx, request).await
   }
}

trait AnyDef: Any {
   fn request_type(&self) -> &'static str;
}

impl dyn AnyDef {
   fn downcast<R: Request>(self: Rc<Self>) -> Rc<Def<R>>;
}
```

`ctx.submit(req)` looks up and downcasts `R`'s definition the same way, then calls `def.submit(ctx, req)` and gets
an `R::Response` back directly. With no definition, or at the end of the chain, an unhandled request panics, naming
the request type.

**Registration.** `Registry::on_event` wraps the user's `AsyncFn` into an `event::Listener<E>` and pushes it onto
`E`'s definition, creating the definition first if needed. During setup the `Registry` is reached through `&mut`, so
the map needs no runtime borrow and each definition's `Rc` is uniquely owned: `Rc::get_mut`, plus a downcast to
`Def<E>`, gives `&mut` access to `listeners`. `Host::on_event` borrows the map mutably, gets or creates `E`'s
definition, and `set`s its `ui`. All of this follows the shape exercised in the prototype.
