# Implementation steps

The crate is organized to match the [groups in the type diagram](./types.md): every public type lives in one of
`posturn::host`, `posturn::module`, `posturn::event`, or `posturn::request`, and `posturn::prelude` re-exports what a
game or UI typically needs (including the module traits, whose methods are only callable when they are in scope).

**Naming** follows Clippy's `module_name_repetitions`: a type doesn't repeat its module's name, and is written
module-qualified wherever it is used outside its module (`event::Listener`, not `EventListener`; `module::Borrow`, not
`BorrowModule`). The exceptions are each module's namesake, `event::Event`, `request::Request`, `module::Module`, and
`host::Host`, which are re-exported from the crate root and the prelude. The crate enables the lint, with an `#[allow]`
on those four.

1. **Remove `genawaiter`.** Drop the dependency, the `pub use genawaiter`, `Play`, `Context`, and the generic
   `Host<Game>`. Bump the version to `0.4.0` and `rust-version` to `1.86`. Don't carry over the `module` feature flag
   from the `module` patch: modules are always available.
2. **`posturn::event`, part 1** (`event.rs`): the `Event` marker trait, `event::Slot`, `event::Handle` (resumes on
   drop), and `event::Future`.
3. **`posturn::request`, part 1** (`request.rs`): the `Request` trait, `request::State`, `request::Slot<T>`,
   `request::Handle<T>` (abandons on drop), and `request::Future<T>`.
4. **`posturn::module`, part 1** (`module.rs`): the `Module` trait (building on the `module` patch), the sealed
   `module::Borrow` / `module::BorrowMut` traits, and the read-only `Ctx<'a>` view.
5. **`posturn::module`, part 2** (`module/registry.rs`): `Registry`, with the fields from
   [Inside the `Registry`](./host.md#inside-the-registry): `HashMap`s keyed by `TypeId` holding `RefCell<dyn Module>`,
   `Rc<dyn event::AnyDef>`, and `Rc<dyn request::AnyDef>`. Plus `insert` (panics on a duplicate type, naming it with
   `Module::name`), `on_event`, `on_request`, `ctx()`, and both module traits (downcasting via trait upcasting).
6. **`posturn::module`, part 3** (`module/ctx.rs`): `CtxMut`, the `Rc<Registry>` newtype with `ctx()`, `emit`,
   `submit`, and both module traits. Not `Clone`; constructible only inside the crate.
7. **`posturn::host`, part 1** (`host/setup.rs`): the `Setup` trait.
8. **`posturn::event` and `posturn::request`, part 2:** `event::Listener<E>`, `event::Def<E>` (listeners plus a
   `OnceCell` UI callback, with a typed `emit` method), the private marker trait `event::AnyDef` (with an inherent
   `downcast::<E>()` on `dyn AnyDef`), and
   `event::Next<E>`; likewise `request::Listener<R>`, `request::Def<R>` (with `submit`), `request::AnyDef`, and
   `request::Next<R>`. Each `Next` has `forward(self, &mut CtxMut, msg)` and
   its own end-of-chain behavior (unhandled events complete; unhandled requests panic).
9. **`posturn::host`, part 2** (`host.rs`): the `Suspension` enum and `Host<Outcome>` with:
   - `new`: runs `set_up` on a fresh `Registry`, moves it into an `Rc`, creates the `CtxMut`, and boxes the game;
   - `module::Borrow` (read-only);
   - `on_event` and `on_request`: wrap callbacks into listeners set in their definition's `ui` slot (creating the
     definition if needed), pass them a `Ctx`, and record the current `Suspension`; panic on a second callback for
     the same type, or on any registration after the first `step()`;
   - `step` (returning `Poll<Outcome>`), with the no-op/abandoned/foreign-`Pending` rules from [Driving the
     game](./driving.md).
   - poisoning: a drop guard in `step` that marks the `Host` poisoned during an unwind, and `is_poisoned()`.
10. **`posturn::prelude`:** re-exports the types a game or UI typically names, and imports `module::Borrow` and
    `module::BorrowMut` anonymously (`as _`) so their methods are callable without clashing with `std::borrow`.
11. **Tests:**
   - Notification-only game (rewrite `RoShamBo`) completes with `event::Handle`s dropped immediately.
   - Typed request round trip through `Host::on_request`.
   - Listener ordering: most recent first; `next.forward` reaches lower listeners; the UI callback runs last even though
     it is registered after setup.
   - Event chains and request chains are independent, even for the same type.
   - Override: a listener returns without calling `next`, so the UI callback never runs.
   - Post-processing: a listener changes the UI's input before the game sees it.
   - `step()` is a no-op while a handle is held.
   - Synchronous resume inside the callback doesn't suspend.
   - An event with no listeners or UI callback completes without suspending.
   - Panics: unhandled request, abandoned `request::Handle`, foreign `Pending`, `step()` after `Done`, missing module.
   - `try_borrow` / `try_borrow_mut` return `None` for a module that was never registered.
   - Emitting an event with no definition completes without suspending; submitting a request with no definition
     panics, naming the request type.
   - `dyn AnyDef::downcast` returns `Rc<Def<E>>` / `Rc<Def<R>>` for the message type a definition was registered
     under.
   - A panicking listener (and a panicking UI callback) poisons the `Host`: `is_poisoned()` becomes `true`, the next
     `step()` panics with the poisoning message, and modules can still be read.
   - After the `Host` is dropped, `request::Handle::resume` returns `Err(value)`, and dropping a leftover
     `event::Handle` or `request::Handle` doesn't panic.
   - Panic on inserting the same module type twice during setup.
   - Panic on registering a UI callback after the first `step()`.
   - **Compile-fail tests** (doc tests marked `compile_fail`) for each violation of the
     [borrowing rule](./host.md#the-borrowing-rule), for cloning `CtxMut`, for capturing `Registry` in the game, for
     calling `borrow_mut` on a `Ctx` or a `Host`, and for implementing `module::BorrowMut` outside the crate.
12. **Port `examples/tui`** (`TicTacToe`) and `RoShamBo` to the new API, each with its own `Setup` struct.
13. **Docs:** README, crate docs, and update the [Where we are](../where_we_are.md) example in this book.

## Acceptance criteria

- No dependencies; no `Send`/`Sync` bounds or impls in the public API; no `unsafe`.
- The [worked example](./example.md), reduced to something self-contained, compiles and runs as a doc test.
- All tests above pass, including the compile-fail tests; `examples/tui` plays a full game.
