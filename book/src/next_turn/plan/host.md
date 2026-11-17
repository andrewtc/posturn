# `Host`, contexts, and modules

**Decided:** all shared state lives in one [`Registry`](#inside-the-registry): the **modules**, the
[listener chains](./events.md), the UI callbacks, and the [current suspension](./coroutines.md#current-suspension).
During setup it is edited directly through `&mut Registry`; afterwards it is moved into an `Rc` and reached through
handles. None of these types is ever `Send` or `Sync`.

| | `Registry` | `CtxMut` | `Ctx<'a>` | `Host<Outcome>` |
| --- | --- | --- | --- | --- |
| What it is | the registry itself | the game's handle | a read-only view | the UI's handle |
| Who holds it | `set_up`, as `&mut Registry` | the game, by value | anyone, by value (`Copy`) | the UI |
| Created by | `Host::new` | `Host::new`, after setup (exactly one) | `.ctx()` on a `Registry` or `CtxMut`; given to UI callbacks | `Host::new(setup)` |
| `Clone`? | no | no | yes (`Copy`) | no |
| [`module::Borrow`](#looking-up-modules) (read) | yes | yes | yes | yes |
| [`module::BorrowMut`](#looking-up-modules) (write) | yes | yes | no | no: [read-only](#the-ui-only-reads) |
| Inserts modules | `insert` (`&mut self`) | no | no | no |
| Registers | game listeners: `on_event`, `on_request` (`&mut self`) | nothing | nothing | UI callbacks: `on_event`, `on_request` (`&mut self`) |
| Hands over control | — | `emit(ev)`, `request(req)`, `next.run(ctx, msg)` (`&mut self`) | — | `step()` (`&mut self`) |

## Modules

A module is a piece of game state that the UI is allowed to inspect, such as an `OverworldModule` holding the level
and party position, or a `BattleModule` holding the current battle. This builds on the work-in-progress `module`
patch:

```rust,ignore
/// A piece of composable game state, published so that the UI can inspect it.
pub trait Module: Any + 'static {
   /// The name of the module, used in diagnostics (e.g. "module `..` is already borrowed"). Defaults to the type's
   /// name; override it for something shorter or friendlier.
   fn name(&self) -> &str { std::any::type_name::<Self>() }
}

/// Most modules need nothing more than this:
impl Module for OverworldModule {}
```

- **Always available.** Modules are core to `posturn`, not behind a feature flag. **Decided.** (The `module` patch's
  `module` feature is dropped.)
- **One instance per type.** Modules are keyed by `TypeId`.
- **Each module has its own `RefCell`**, so independent modules can be borrowed at the same time.
- **Registered only during [setup](./games.md)**, through `Registry::insert`. The set of modules is fixed before the
  game starts, so a module that only matters part of the time is inserted up front in an empty state.
- **A missing module panics** with a message naming the type (as in the patch), unless it is looked up with
  `try_borrow` / `try_borrow_mut`, which return `None` instead. **Decided.**

## Looking up modules

**Decided:** every type that can see modules offers the **same interface**, through two traits:

```rust,ignore
// In `posturn::module`:

/// Read access to modules. Implemented by `Registry`, `CtxMut`, `Ctx<'_>`, and `Host<Outcome>`.
pub trait Borrow: sealed::Sealed {
   /// Panics if no module of type `M` was registered.
   fn borrow<M: Module>(&self) -> Ref<'_, M>;
   /// Returns `None` if no module of type `M` was registered.
   fn try_borrow<M: Module>(&self) -> Option<Ref<'_, M>>;
   fn module<M: Module + Copy>(&self) -> M { *self.borrow::<M>() }
   fn clone_module<M: Module + Clone>(&self) -> M { self.borrow::<M>().clone() }
}

/// Write access to modules. Implemented by `Registry` and `CtxMut` only.
pub trait BorrowMut: Borrow {
   fn borrow_mut<M: Module>(&self) -> RefMut<'_, M>;
   fn try_borrow_mut<M: Module>(&self) -> Option<RefMut<'_, M>>;
}
```

```rust,ignore
let overworld = ctx.borrow::<OverworldModule>();      // Ref<'_, OverworldModule>
let mut battle = ctx.borrow_mut::<BattleModule>();    // RefMut<'_, BattleModule> (CtxMut / Registry only)
let time = ctx.module::<GameClock>();                 // M: Copy
let party = ctx.clone_module::<Party>();              // M: Clone
let mods = ctx.try_borrow::<ModSettings>();           // Option<Ref<'_, ModSettings>>: may not be registered

let overworld = host.borrow::<OverworldModule>();     // from the UI, between steps (read-only)
```

- **`try_` means "may be missing", not "may be borrowed".** Unlike `RefCell::try_borrow`, the `try_` variants only
  handle a module that was never registered, which is useful for optional modules (e.g. one that only some `Setup`s,
  or some mods, insert). A conflicting borrow of the same module still panics, like `borrow` does.
- **No runtime cost.** The traits are only used with concrete types (or as `impl module::Borrow` bounds), so their
  generic methods are resolved and monomorphized at compile time, exactly like inherent methods. Generic methods do
  make the traits unusable as `dyn module::Borrow`, which nothing needs.
- **Sealed.** Both traits have a private supertrait whose one method returns the `&Registry`, and all the provided
  methods are written in terms of it. Users can't implement the traits themselves; otherwise they could wrap a
  read-only `Ctx`, hand back its registry, and gain write access through `module::BorrowMut`.
- **Functions that only read** can take `&impl module::Borrow`, which accepts any of the four (including `&host`), or a
  `Ctx<'_>`.
- **The traits must be in scope to call their methods,** which `use posturn::prelude::*;` takes care of. Their names,
  `module::Borrow` and `module::BorrowMut`, match `std::borrow::Borrow` and `std::borrow::BorrowMut`, so the prelude
  imports them anonymously (`pub use crate::module::{Borrow as _, BorrowMut as _};`): their methods become available
  without either name being brought into scope. Code that names a trait, e.g. in a bound, writes it module-qualified:
  `&impl module::Borrow`.

### `Ctx<'a>`: the read-only view

```rust,ignore
/// A read-only view of the modules. Cheap to copy; borrows from whatever it was obtained from.
#[derive(Clone, Copy)]
pub struct Ctx<'a>(&'a Registry);

impl Registry { pub fn ctx(&self) -> Ctx<'_>; }
impl CtxMut { pub fn ctx(&self) -> Ctx<'_>; }
```

A `Ctx` is what you pass to code that should only look: UI drawing helpers, the [UI callbacks](./ui_handlers.md), or
game helpers that compute something from state without changing it. Because it borrows its source, it inherits that
source's restrictions (see [the borrowing rule](#the-borrowing-rule)).

`Host` has no `ctx()`: it implements `module::Borrow` itself, so the UI reads with `host.borrow::<M>()` and passes
`&host` to read-only helpers. (A `Ctx` would add nothing there. Since `module::Borrow::borrow` returns a `Ref` tied to
the `&self` it was called on, borrowing straight from a temporary `Ctx`, as in `let o = x.ctx().borrow::<M>();`,
doesn't even compile.)

### Who can create what

`Registry` and `CtxMut` can create a `Ctx`, and the `Host` hands one to each UI callback. **Only `Host::new` creates a
`CtxMut`**, and only one. If any `&Registry` could produce a `CtxMut`, there could be two of them, and the compile-time
borrowing rule below would no longer hold.

### The UI only reads

**Decided:** the UI's access to modules is **read-only**. `Host` and `Ctx` implement `module::Borrow` but not
`module::BorrowMut`, and `Host` never exposes its `&Registry`. All changes to game state happen inside the game (its
async function and its listeners), so modules always reflect the rules of the game. The UI's only way to influence the
game is the one the game asks for: answering a request through `request::Handle`. State that belongs purely to the UI
(cursor position, selected menu item, camera) lives in the UI, not in a module.

## The borrowing rule

**Decided:** the game must not hold a module borrow across a suspension, and the UI must not hold a module borrow
while it steps the game. **The borrow checker enforces both.**

- Module borrows take `&self`, on every type.
- On the game side, everything that can suspend takes `&mut CtxMut`: `ctx.emit(..)`, `ctx.request(..)`, and
  `next.run(ctx, ..)`. A `Ref`/`RefMut` (or a `Ctx` from `ctx.ctx()`) still alive at one of those is a compile error.
  Because `CtxMut` is not `Clone` and only `Host::new` creates one, there is no second handle to borrow from instead.
- On the UI side, `host.step()` takes `&mut self`, so a `Ref` from `host.borrow(..)` that is still alive at the next
  `step()` is a compile error.

A prototype confirmed that each of these violations is rejected:

| Violation | Result |
| --- | --- |
| Game holds `ctx.borrow_mut::<M>()` across `ctx.emit(..).await` (or `ctx.request(..)`) | `E0502`: cannot borrow `ctx` as mutable because it is also borrowed as immutable |
| Listener holds `ctx.borrow::<M>()` across `next.run(ctx, ev).await` | `E0502` on `*ctx` |
| UI holds `host.borrow::<M>()` across `host.step()` | `E0502` on `host` |
| Game clones `CtxMut` to get around the rule | `E0599`: no method named `clone` |

What remains a runtime check: conflicting borrows of the **same** module within one synchronous stretch (e.g.
`borrow_mut::<A>()` twice with no `await` in between) still panic, like any `RefCell`. A module that itself holds an
`Rc<RefCell<..>>` can of course smuggle state around; that is out of scope.

## Why both a `Host` and a `CtxMut`?

`CtxMut` can't be merged into `Host`: the game needs a handle that can write, emit, and request but can't call
`step()` on itself, and the game future (which `Host` owns) has to own that handle. Everything else is collapsed into
`Host`: there is no separate driver type.

## Inside the `Registry`

`CtxMut` and `Host` are thin wrappers around a shared `Rc<Registry>`; `Ctx` is a plain reference to it:

```rust,ignore
/// The game's handle. Not `Clone`; only `Host::new` creates one.
pub struct CtxMut(Rc<Registry>);

/// A read-only view.
#[derive(Clone, Copy)]
pub struct Ctx<'a>(&'a Registry);

pub struct Host<Outcome> {
   registry: Rc<Registry>,
   /// `None` once the game has finished; `step()` panics after that.
   game: Option<Pin<Box<dyn Future<Output = Outcome>>>>,
   /// Set if a panic unwound out of `step()`; every later `step()` panics. See "Driving the game".
   poisoned: bool,
   /// Set by the first `step()`; `on_event` / `on_request` panic after that. See "Handing control to the UI".
   started: bool,
}
```

`Host::new` creates a `Registry` by value, lets the [`Setup`](./games.md) edit it through `&mut Registry`, and then
moves it into an `Rc`. **That move is the freeze:** no one can get a `&mut Registry` out of a shared `Rc`, so
`insert`, `on_event`, and `on_request` on `Registry` can never be called again, and the compiler enforces it. Nothing
else (no staging map, no `OnceCell`) is needed.

```rust,ignore
pub struct Registry {
   /// One `RefCell<M>` per module type `M`. Fixed after setup, so `borrow` can hand out a `Ref<'_, M>` straight from
   /// the map.
   modules: TypeMap,

   /// One `event::Chain<E>` (the game listeners) per event type `E`. Fixed after setup.
   event_listeners: TypeMap,

   /// One `request::Chain<R>` (the game listeners) per request type `R`. Fixed after setup.
   request_listeners: TypeMap,

   /// At most one `event::Listener<E>` (the UI callback) per event type `E`, registered by `Host::on_event`.
   event_callbacks: RefCell<TypeMap>,

   /// At most one `request::Listener<R>` (the UI callback) per request type `R`, registered by
   /// `Host::on_request`.
   request_callbacks: RefCell<TypeMap>,

   /// The slot the game is currently parked on, if any. See "Under the hood".
   current: RefCell<Option<Suspension>>,
}

// In `posturn::event`:

/// A listener for events of type `E`.
type Listener<E> = Rc<dyn for<'a> Fn(&'a mut CtxMut, E, Next<E>) -> LocalBoxFuture<'a, ()>>;

/// The game listeners for one event type, in registration order (the last one runs first).
type Chain<E> = Rc<Vec<Listener<E>>>;

// In `posturn::request`:

/// A listener for requests of type `R`, producing its response.
type Listener<R> = Rc<dyn for<'a> Fn(&'a mut CtxMut, R, Next<R>) -> LocalBoxFuture<'a, <R as Request>::Response>>;

/// The game listeners for one request type, in registration order (the last one runs first).
type Chain<R> = Rc<Vec<Listener<R>>>;
```

Notes:

- **Only three fields change after setup:** the two UI-callback maps (the `Host` registers callbacks after `new`) and
  the current suspension. Everything else is never mutated once the `Registry` is in its `Rc`.
- **UI callbacks are stored apart from the game's chains** rather than in a slot inside each chain, because the
  chain maps are frozen: a UI callback for a message type with no game listeners would otherwise need a new map
  entry after setup. Keeping them separate is also what puts the UI callback [at the
  bottom](./ui_handlers.md#always-at-the-bottom) regardless of registration order.
- **Each map is a [`TypeMap`](#typemap)** holding at most one value per type: the generic method that knows the
  concrete type (`borrow::<M>`, `emit::<E>`, `request::<R>`) looks up `RefCell<M>`, `event::Chain<E>`, etc. directly.
- **During setup, chains are uniquely owned**, so registering a listener pushes onto the `Vec` through `Rc::get_mut`.
  Afterwards, `emit` and `request` clone the `Rc` as a cheap snapshot for `event::Next` / `request::Next`.

### `TypeMap`

Each map in the `Registry` stores **at most one value per type**, and every value's type is unique to what it's for:
`RefCell<M>` for module `M`, `event::Chain<E>` for event `E`, and so on. So each map can be keyed by the stored value's
own type. **Decided:** `posturn` implements this small map itself rather than taking a dependency:

```rust,ignore
// In `posturn::module` (private):

/// A map holding at most one value of each type, keyed by that type.
struct TypeMap(BTreeMap<TypeId, Box<dyn Any>>);

impl TypeMap {
   /// Stores `value`. Returns `false` (and stores nothing) if a `T` is already present.
   fn insert<T: 'static>(&mut self, value: T) -> bool;

   /// The stored `T`, if any.
   fn get<T: 'static>(&self) -> Option<&T>;

   /// The stored `T`, mutably. Only used during setup (to push a listener onto a chain).
   fn get_mut<T: 'static>(&mut self) -> Option<&mut T>;
}
```

The key is `TypeId::of::<T>()`. The `Box<dyn Any>` is what lets one map hold values of many different types; `get`
turns an entry back into a `&T` with `downcast_ref`, which always succeeds because the key and the stored value have
the same type. Looking up module `M` is therefore `modules.get::<RefCell<M>>()`, and event `E`'s chain is
`event_listeners.get::<event::Chain<E>>()`.

This is the same shape as the `AnyMap` from the [`anymap3`](https://docs.rs/anymap3) crate (whose default map, like
ours, doesn't require `Send`), so switching to it later would be a drop-in change if `posturn` ever needs more, such as
an `Entry` API.
