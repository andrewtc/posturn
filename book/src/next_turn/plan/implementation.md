<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Implementation steps

The crate is organized to match the [groups in the type diagram](./types.md): every public type lives in one of
`posturn::host`, `posturn::module`, `posturn::event`, or `posturn::request`, and `posturn::prelude` re-exports what a
game or UI typically needs (including the module traits, whose methods are only callable when they are in scope).

**Naming** follows Clippy's `module_name_repetitions`: a type doesn't repeat its module's name, and is written
module-qualified wherever it is used outside its module (`event::Handle`, not `EventHandle`; `module::Borrow`, not
`BorrowModule`). The exceptions are each module's namesake, `event::Event`, `request::Request`, `module::Module`, and
`host::Host`, which are re-exported from the crate root and the prelude. The crate enables the lint, with an `#[allow]`
on those four.

1. **Remove `genawaiter`.** Drop the dependency, the `pub use genawaiter`, `Play`, `Context`, and the generic
   `Host<Game>`. Bump the version to `0.4.0` and `rust-version` to `1.86`. Don't carry over the `module` feature flag
   from the `module` patch: modules are always available.
2. **`posturn::event`, part 1** (`event.rs`): the `Event` trait (with its `Priority`), `event::Slot`,
   `event::Resume` (continues on drop), and `event::Future`.
3. **`posturn::request`, part 1** (`request.rs`): the `Request` trait (with its `Response` and `Priority`),
   `request::State` (including `Passed`), `request::Slot<T>`, `request::Resume<T>` (`with` and `resume`, abandons on
   drop), and `request::Future<T>` (`Output = Option<T>`).
4. **`posturn::module`, part 1** (`module.rs`): the `Module` trait (building on the `module` patch), the sealed
   `module::Borrow` / `module::BorrowMut` traits, and the read-only `Ctx<'a>` view.
5. **`posturn::module`, part 2** (`module/registry.rs`): `Registry`, with the fields from
   [Inside the `Registry`](./host.md#inside-the-registry): `HashMap`s keyed by `TypeId` holding `RefCell<dyn Module>`,
   `Box<dyn event::AnyDef>`, and `Box<dyn request::AnyDef>`, the latter two each in a `RefCell`. Plus `insert`
   (panics on a duplicate type, naming it with `Module::name`), `on_event` and `on_request` (`&self`, returning
   handles), `ctx()`, and both module traits (downcasting via trait upcasting).
6. **`posturn::module`, part 3** (`module/ctx.rs`): `CtxMut`, the `Rc<Registry>` newtype with `ctx()`, `emit`,
   `submit`, `on_event`, `on_request`, and both module traits. Not `Clone`; constructible only inside the crate.
7. **`posturn::host`, part 1** (`host/setup.rs`): the `Setup` trait.
8. **`posturn::event` and `posturn::request`, part 2:** for events,
   - `event::Callback<E>`, an enum of `Game` (the boxed `AsyncFn`) and `Host` (the `FnMut` in an `Rc<RefCell<..>>`),
     with a hand-written `Clone`;
   - `event::Listener<E>` (a priority, and the callback in a `RefCell<Option<..>>`) and the private
     `event::AnyListener`;
   - `event::Def<E>` (a sorted `Vec` of listeners, with `insert`, which prunes cleared listeners and returns a handle);
   - the private marker trait `event::AnyDef`, with inherent `downcast_ref::<E>()` / `downcast_mut::<E>()` on
     `dyn AnyDef`;
   - `event::Handle` (`#[must_use]`; clears its listener on drop, unless given up with `forever()`) and
     `event::Next<E>`, which owns a copy of the chain.

   Likewise `request::Callback<R>`, `request::Listener<R>`, `request::AnyListener`, `request::Def<R>`,
   `request::AnyDef`, `request::Handle`, and `request::Next<R>`. Each `Next` has `forward(self, &mut CtxMut, msg)`,
   which loops over its copy of the chain: it skips cleared listeners, hands over to the next game listener, and waits
   at each host listener. Each also has its own end-of-chain behavior (unhandled events complete; unhandled requests
   panic).
9. **`posturn::host`, part 2** (`host.rs`): the `Suspension` enum and `Host<Outcome>` with:
   - `new`: runs `set_up` on a fresh `Registry`, moves it into an `Rc`, creates the `CtxMut`, and boxes the game;
   - `module::Borrow` (read-only);
   - `on_event` and `on_request` (`&self`): wrap callbacks into `Callback::Host`s, insert them into their definition's
     chain by priority (creating the definition if needed), and return handles. When they run, they pass the
     callback a `Ctx` and record the current `Suspension`;
   - `step` (returning `Poll<Outcome>`), with the no-op/abandoned/foreign-`Pending` rules from [Driving the
     game](./driving.md).
   - poisoning: a drop guard in `step` that marks the `Host` poisoned during an unwind, and `is_poisoned()`.
10. **`posturn::prelude`:** re-exports the types a game or UI typically names, and imports `module::Borrow` and
    `module::BorrowMut` anonymously (`as _`) so their methods are callable without clashing with `std::borrow`.
11. **Tests:**
   - Notification-only game (rewrite `RoShamBo`) completes with `event::Resume`s dropped immediately.
   - Typed request round trip through `Host::on_request`.
   - Listener ordering: greatest priority first; newest first among equal priorities; `next.forward` reaches lower
     listeners; host listeners and game listeners interleave by priority, whenever each was registered.
   - Several host listeners for one event each see it in turn; for a request, `resume.resume()` passes it on to the next
     listener, and `with` ends the chain.
   - Dropping a handle unregisters its listener and drops its closure at once; a listener whose handle is dropped while
     it runs finishes normally; a listener whose handle was given up with `forever()` stays registered.
   - Mid-dispatch: a listener registered while a message is in flight doesn't see that message; one whose handle is
     dropped before the dispatch reaches it is skipped.
   - Registering and unregistering from inside a listener (including for the same message type) doesn't panic.
   - Handles stored in a module are dropped with the `Host` without panicking.
   - Event chains and request chains are independent, even for the same type.
   - Override: a listener returns without calling `next`, so the host listener below it never runs.
   - Post-processing: a listener changes the UI's input before the game sees it.
   - `step()` is a no-op while a resume handle is held.
   - Synchronous resume inside the callback doesn't suspend.
   - An event with no listeners completes without suspending.
   - Panics: unhandled request (including one passed on with `resume()` by the last listener), abandoned
     `request::Resume`, foreign `Pending`, `step()` after `Done`, missing module.
   - `try_borrow` / `try_borrow_mut` return `None` for a module that was never registered.
   - Emitting an event with no definition completes without suspending; submitting a request with no definition
     panics, naming the request type.
   - `downcast_ref` / `downcast_mut` on `dyn AnyDef` return the `Def<E>` / `Def<R>` for the message type a definition
     was registered under.
   - A panicking listener (game or UI) poisons the `Host`: `is_poisoned()` becomes `true`, the next
     `step()` panics with the poisoning message, and modules can still be read.
   - After the `Host` is dropped, `request::Resume::with` returns `Err(value)`, and dropping a leftover
     `event::Resume`, `request::Resume`, `event::Handle`, or `request::Handle` doesn't panic.
   - Panic on inserting the same module type twice during setup.
   - A handle dropped straight away triggers the `#[must_use]` warning (a compile-fail doc test under
     `deny(unused_must_use)`).
   - **Compile-fail tests** (doc tests marked `compile_fail`) for each violation of the
     [borrowing rule](./host.md#the-borrowing-rule), for cloning `CtxMut`, for capturing `Registry` in the game, for
     calling `borrow_mut` on a `Ctx` or a `Host`, and for implementing `module::BorrowMut` outside the crate.
12. **Port `examples/tui`** (`TicTacToe`) and `RoShamBo` to the new API, each with its own `Setup` struct.
13. **Docs:** README, crate docs, and update the [Where we are](../where_we_are.md) example in this book.

## Acceptance criteria

- No dependencies; no `Send`/`Sync` bounds or impls in the public API; no `unsafe`.
- The [worked example](./example.md), reduced to something self-contained, compiles and runs as a doc test.
- All tests above pass, including the compile-fail tests; `examples/tui` plays a full game.
