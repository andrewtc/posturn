# Events, requests, and listeners

The game communicates through two kinds of message, which are deliberately kept apart.

## Events are broadcasts

**Decided:** an **event** is something that happened in the game that others may want to react to. Event types must
be **declared explicitly** by implementing the `Event` marker trait, so that only types designed to be events can be
emitted (and not, say, an arbitrary `Vec<HashSet<Widget>>`). Emitting one suspends the game until the event has been
handled (for example, until the UI has finished animating it), and returns nothing:

```rust,ignore
/// Marks a type as an event that the game can emit.
pub trait Event: 'static {}

/// The party moved one step.
pub struct PartyMoved { pub direction: Direction }
impl Event for PartyMoved {}

ctx.emit(PartyMoved { direction }).await; // -> ()
```

## Requests expect a response

**Decided:** a **request** asks for something the game can't continue without, typically a player's decision. A
request type implements `Request`, whose associated `Response` is what the game receives:

```rust,ignore
pub trait Request: 'static {
   /// What `ctx.request(..)` resolves to.
   type Response: 'static;
}

/// The game needs to know what the party does next.
pub struct PartyInputRequested;
impl Request for PartyInputRequested { type Response = PartyInput; }

match ctx.request(PartyInputRequested).await {
   PartyInput::Move { direction } => { /* ... */ }
   PartyInput::Rest { hours } => { /* ... */ }
   PartyInput::UseItem { item_idx } => { /* ... */ }
}
```

Together these solve [input bloat](../input_bloat.md), [null input](../null_input.md), and
[events vs. input](../events_vs_input.md): every request has its own response type, events need no response at all,
and the two can never be confused. Both `emit` and `request` take `&mut CtxMut`, which is what makes holding a module
borrow across them a [compile error](./host.md#the-borrowing-rule).

### One type, one role

**Decided:** a type should be _either_ an `Event` _or_ a `Request`, never both. Nothing in the design breaks if a type
implements both traits (events and requests have entirely separate chains, listeners, handles, and futures, so
`ctx.emit(x)` and `ctx.request(x)` would simply travel different chains), and stable Rust can't forbid it without
negative trait bounds. But the two would share a name while having unrelated listeners, which is confusing to read
and easy to get wrong. The documentation for both traits will discourage it, and recommend separate types instead
(e.g. `PartyMoved` as the event and `MoveRequested` as the request).

## Listeners form a chain

**Decided:** for each event type and each request type (keyed by `TypeId`), the registry keeps an ordered chain of
listeners. A listener is an `async` function or async closure in **middleware style**: it receives the context, the
message by value, and a handle for the rest of the chain. Events and requests follow the same pattern, but with
separate types throughout: an event listener gets a `event::Next<E>` and returns `()`; a request listener gets a
`request::Next<R>` and returns `R::Response`.

Listeners are registered **only during [setup](./games.md)**, on `Registry`, with `on_event` and `on_request`. Once the
game has started, the chains can't change. (The UI's own callbacks are registered on the `Host`, also with
`on_event` and `on_request`; see [Handing control to the UI](./ui_handlers.md).)

```rust,ignore
fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Outcome + 'static {
   // An event listener, as an async fn item...
   registry.on_event(on_party_moved);

   // ...and a request listener, as an async closure.
   registry.on_request(async |ctx: &mut CtxMut, req: PartyInputRequested, next| {
      let input = next.run(ctx, req).await;
      // Code after the rest of the chain, which may inspect or replace the response...
      input
   });

   async move |ctx: CtxMut| play(ctx).await
}

async fn on_party_moved(ctx: &mut CtxMut, ev: PartyMoved, next: event::Next<PartyMoved>) {
   // Code before the rest of the chain (e.g. advance the clock, run triggers)...
   evaluate_triggers(ctx, &ev).await;

   // Defer to the next listener, like calling the base implementation of a `virtual` fn.
   next.run(ctx, ev).await
}
```

The bounds are:

- `on_event<E: Event>(&mut self, F)` with `F: AsyncFn(&mut CtxMut, E, event::Next<E>) + 'static`.
- `on_request<R: Request>(&mut self, F)` with `F: AsyncFn(&mut CtxMut, R, request::Next<R>) -> R::Response + 'static`.

Because `request::Next<R>` knows `R::Response` from the `Request` impl, a request listener written as an `async fn`
only names the request type: `next: request::Next<PartyInputRequested>`. In closures both are inferred. `AsyncFn` needs
Rust 1.85.

A listener can:

- **Modify or replace** the message before passing it on with `next.run(ctx, msg)`. (Both `event::Next` and
  `request::Next` have a `run` method; they differ in what it returns.)
- **Post-process** a request's response: run code after `next.run(..)` returns, and change the value before returning
  it. This includes changing input that came back from the [UI callback](./ui_handlers.md).
- **Override** the rest of the chain by not calling `next` at all (returning a response of its own, for requests).
  This can prevent the UI from ever seeing the message.
- **`await` anything that takes `&mut CtxMut`**, including emitting other events, making other requests, or running a
  whole minigame.

`run` consumes the handle, so the rest of the chain runs at most once per listener invocation.

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
last game listener calls `next.run` with no UI callback below it.

- **Unhandled events complete immediately. Decided.** An event is a broadcast; if nobody is listening, there is
  nothing to wait for, and `emit` (or the `next.run`) simply returns `()` without suspending. A UI therefore only
  registers callbacks for the events it actually presents, such as the ones it animates.
- **Unhandled requests panic. Decided.** The game can't continue without a response, so `posturn` panics with a
  message naming the request type.

## How a message travels the chain

The storage lives in [`Registry`](./host.md#inside-the-registry): a frozen map of game-listener chains per message type,
plus a separate map of UI callbacks. The "rest of the chain" handles carry everything needed to continue:

```rust,ignore
// In `posturn::event`:
pub struct Next<E: Event> {
   /// Snapshot of the game listeners for `E` (a clone of the chain's `Rc`).
   chain: Chain<E>,
   /// How many game listeners have not run yet. They run from the end of `chain` backwards (most recent first).
   remaining: usize,
   /// The UI callback for `E`, looked up when the event was emitted. Runs after the last game listener.
   ui: Option<Listener<E>>,
}

impl<E: Event> Next<E> {
   pub async fn run(self, ctx: &mut CtxMut, event: E);
}

// In `posturn::request`:
pub struct Next<R: Request> {
   chain: Chain<R>,
   remaining: usize,
   ui: Option<Listener<R>>,
}

impl<R: Request> Next<R> {
   pub async fn run(self, ctx: &mut CtxMut, request: R) -> R::Response;
}
```

`ctx.emit(ev)` (and likewise `ctx.request(req)`):

1. Looks up the chain for the message's `TypeId` (an empty chain if there is none) and clones its `Rc`.
2. Looks up the UI callback, if any, and clones its `Rc`.
3. Builds `event::Next { chain, remaining: chain.len(), ui }` (or `request::Next { .. }`) and calls `next.run(ctx,
   msg)`.

`run` consumes the handle and:

- if `remaining > 0`, calls `chain[remaining - 1]` with a handle whose `remaining` is one less;
- otherwise, if there is a UI callback, calls it (the callback's own handle is empty);
- otherwise has reached the [end of the chain](#reaching-the-end-of-the-chain): `event::Next::run` returns `()`, and
  `request::Next::run` panics.

`Registry::on_event` / `on_request` wrap the user's `AsyncFn` in an `Rc` and box each call's future, producing an
`event::Listener<E>` or `request::Listener<R>` (see [Inside the `Registry`](./host.md#inside-the-registry)). All of this
was exercised in the prototype.
