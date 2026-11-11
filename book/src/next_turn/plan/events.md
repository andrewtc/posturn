# Events and listeners

## Events are types

**Decided:** each event is its own type, identified at compile time. Its _output_ is the type of value the game
receives when it awaits the event:

```rust,ignore
pub trait Event: 'static {
   /// What `emit` resolves to. `()` for pure notifications.
   type Output: 'static;
}

/// The party moved one step. A notification: nothing needs to come back.
pub struct PartyMoved { pub direction: Direction }
impl Event for PartyMoved { type Output = (); }

/// The game needs to know what the party does next.
pub struct PartyInputRequested;
impl Event for PartyInputRequested { type Output = PartyInput; }
```

The game emits an event through the `Host` and awaits its output:

```rust,ignore
match host.emit(PartyInputRequested).await {
   PartyInput::Move { direction } => { /* ... */ }
   PartyInput::Rest { hours } => { /* ... */ }
   PartyInput::UseItem { item_idx } => { /* ... */ }
}

host.emit(PartyMoved { direction }).await; // -> ()
```

This solves [input bloat](../input_bloat.md) and [null input](../null_input.md): each request has its own output
type, and notifications need no input at all. It also means an emit never needs a module borrow.

## Listeners form a chain

**Decided:** the `Host` keeps, for each event type (keyed by `TypeId`), an ordered chain of listeners. A listener is
an `async` function in **middleware style**: it receives the event by value and a `Next<E>` handle for the rest of
the chain.

```rust,ignore
host.listen(|host: &Host, ev: PartyMoved, next: Next<PartyMoved>| async move {
   // Code before the rest of the chain (e.g. advance the clock, run triggers)...
   evaluate_triggers(host, &ev).await;

   // Defer to the next listener, like calling the base implementation of a `virtual` fn.
   let out = next.run(ev).await;

   // Code after the rest of the chain, which may inspect or replace `out`...
   out
});
```

A listener can:

- **Modify or replace** the event before passing it on with `next.run(ev)`.
- **Post-process** the output: run code after `next.run(..)` returns, and change the value before returning it. This
  includes changing input that came back from the [UI handler](./ui_handlers.md).
- **Override** the rest of the chain by returning an `E::Output` without calling `next` at all. This can prevent the
  UI handler from ever seeing the event.
- **`await` anything `posturn` can suspend on**, including emitting other events or running a whole minigame.

`Next<E>::run` consumes `self`, so the rest of the chain runs at most once per listener invocation.

## Ordering

**Decided (for now):** the most recently registered listener runs first. Registering a listener therefore wraps the
existing behavior, and it decides whether to `continue` into it. This is the same as a stack of overrides.

The [UI handler](./ui_handlers.md) is just another link in the chain. Because it is normally registered before the
game starts, it sits at the bottom.

A richer way to state ordering (dependencies between module listeners, expressed through a builder or a procedural
macro) is [future work](./future_work.md).

## Unhandled events panic

**Decided:** if an event reaches the end of its chain (no listener is registered, or the last listener calls
`next.run`), `posturn` panics with a message naming the event type. An event never silently falls through to the UI.

## Sketch of the storage

Listeners are stored per event type without erasing `E`:

```rust,ignore
type Listener<E> = Rc<dyn Fn(Host, E, Next<E>) -> LocalBoxFuture<<E as Event>::Output>>;

// In the Host: TypeId::of::<E>() -> Box<dyn Any> holding a Vec<Listener<E>>, downcast on emit.
```

`emit` takes a snapshot of the chain (an `Rc<[Listener<E>]>`), so listeners registered or removed while an emit is in
progress don't affect it. `Next<E>` holds that snapshot and the index of the next listener.
