# Open questions

These still need decisions before (or during) implementation.

## Enforcing the borrowing rule (prototyped)

The goal is decided: no module borrow across an `.await` in the game, and none held by the UI while it steps. The
mechanism under consideration is a **dedicated context type**, `Ctx`, which a throwaway prototype has confirmed lets
the borrow checker enforce the rule.

### The shape

- **The game receives `Ctx` by value**, and everything that can suspend takes `&mut Ctx`:

  ```rust,ignore
  let mut session = Session::new(|ctx| play(setup, ctx));

  async fn play(setup: GameSetup, mut ctx: Ctx) -> Outcome { /* ... */ }
  async fn on_party_moved(ctx: &mut Ctx, ev: PartyMoved, next: Next<PartyMoved>) { /* ... */ }
  ```

- **`Ctx` is not `Clone`.** It wraps the shared registry (`Rc<Inner>`), but there is only ever one `Ctx`, owned by
  the game future. `Session` holds the only other reference.
- **Module access takes `&self`**: `ctx.borrow::<M>() -> Ref<'_, M>` and `ctx.borrow_mut::<M>() -> RefMut<'_, M>`.
  Separate modules can be borrowed at the same time (per-module `RefCell`).
- **Suspending takes `&mut self`**: `ctx.emit(ev)`, `next.run(ctx, ev)`, `ctx.listen(..)`, and (likely)
  `ctx.insert(module)`. Any `Ref`/`RefMut` still alive at one of these is a **compile error**.
- **Listeners are `AsyncFn(&mut Ctx, E, Next<E>) -> E::Output + 'static`.** Both `async fn` items and async closures
  (`async |ctx, ev, next| { .. }`) work. `Next::run` takes the context explicitly: `next.run(ctx, ev).await`.
- **The UI reaches modules only through the session**: `session.host().borrow::<M>()` borrows `&session`, and
  `session.step()` takes `&mut self`, so holding a module borrow across a step is a **compile error**.
- **UI handlers are registered on the session** (`session.on_input(..)`, `session.on_suspend(..)`) before the first
  `step()`. That puts them below every listener the game registers, which happens during the first `step()`.

### What the prototype showed

All four violations fail to compile; the correct versions run:

| Violation | Result |
| --- | --- |
| Game holds `ctx.borrow_mut::<M>()` across `ctx.emit(..).await` | `E0502`: cannot borrow `ctx` as mutable because it is also borrowed as immutable |
| Listener holds `ctx.borrow::<M>()` across `next.run(ctx, ev).await` | `E0502` on `*ctx` |
| UI holds `session.host().borrow::<M>()` across `session.step()` | `E0502` on `session` |
| Game clones `Ctx` to sidestep the rule | `E0599`: no method named `clone` |

### Consequences and caveats

- **MSRV.** `AsyncFn` and `Waker::noop()` need Rust 1.85. (`dyn Module` → `dyn Any` upcasting needs 1.86; see below.)
- **Helpers that suspend must take `&mut Ctx`.** That is the point: the signature documents that the helper can
  suspend.
- **No concurrent suspensions inside the game.** Two emits can't be in flight at once, since each needs `&mut Ctx`.
  This matches the [one-suspension-at-a-time](./driving.md#one-suspension-at-a-time) rule.
- **Conflicts within one synchronous section stay runtime checks.** `borrow_mut::<A>()` twice without an `await` in
  between still panics, like any `RefCell`.
- **Inserting modules mid-game.** If `insert` takes `&mut Ctx`, the exclusive borrow proves that no `Ref` is
  outstanding anywhere, so the map can safely change. But returning a `Ref<'_, M>` straight out of a growable map needs
  stable addresses (boxed modules) and a small, well-contained `unsafe`. The prototype avoided this by registering
  all modules before the session starts. See [Module details](#module-details).
- **UI callbacks could get read access.** When a UI callback runs, every frame of the game is parked at an `emit` or
  `next.run`, all of which hold `&mut Ctx`, so no module borrows are live. It would therefore be safe to pass the
  callback a read-only view: `FnMut(&HostView, E, WaitForInput<R>)`.
- **Modules that hold their own `Rc<RefCell<..>>`** can of course still smuggle state around. That is out of scope.

## Locking granularity

The `module` patch puts all modules behind one `RefCell`, so borrowing any module locks all of them. Should each
module get its own `RefCell` so independent modules can be borrowed at the same time?

## MSRV and `dyn Module` → `dyn Any`

The patch casts `&dyn Module` to `&dyn Any`, which needs trait upcasting (stable since Rust 1.86). Either raise the
MSRV from 1.75 to at least 1.86, or add `as_any` / `as_any_mut` to `Module`. Related: async closures (stable in
1.85) would make `host.listen(async |host, ev, next| ..)` read more naturally than closures returning `async move`
blocks.

## Module details

- What is `Module::name` for? If it is only for diagnostics, it could default to `std::any::type_name::<Self>()`.
- Keep panicking on a missing module, or also offer `try_borrow` returning `Option`?
- Can the UI mutate modules, or is its access read-only?
- Who inserts modules, and when: the game as it runs, or the UI before the game starts?
- Does the `module` feature flag survive, now that modules are the only shared state?

## Listener details

- Can listeners be removed? (For example, `listen` returns a `ListenerHandle` that unregisters on drop.)
- Can there be more than one UI handler for the same event, or should the UI have a dedicated bottom-of-chain slot
  instead of relying on registration order?
- What happens when a listener panics mid-chain? (Probably: the panic propagates out of `step()` and the session is
  poisoned.)

## Session lifecycle

- What happens to a stored `Suspend`/`WaitForInput` when the `Session` is dropped (e.g. "New Game" mid-turn)?
  Dropping a `WaitForInput` after its session is gone should not count as an error.
- Should the driver be called `Session`, `Game`, or something else?
