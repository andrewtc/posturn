<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# `Host`, contexts, and modules

**Decided:** all shared state lives in one [`Registry`](#inside-the-registry): the **modules**, a
[definition](./events.md#definitions-and-dispatch) for each event and request type (its game listeners and UI
callback), and the [current suspension](./coroutines.md#current-suspension).
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
| Hands over control | — | `emit(ev)`, `submit(req)`, `next.forward(ctx, msg)` (`&mut self`) | — | `step()` (`&mut self`) |

## Modules

A module is a piece of game state that the UI is allowed to inspect, such as an `OverworldModule` holding the level
and party position, or a `BattleModule` holding the current battle. This builds on the work-in-progress `module`
patch:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
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
<!-- SPDX-SnippetEnd -->

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

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
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
<!-- SPDX-SnippetEnd -->

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
let overworld = ctx.borrow::<OverworldModule>();      // Ref<'_, OverworldModule>
let mut battle = ctx.borrow_mut::<BattleModule>();    // RefMut<'_, BattleModule> (CtxMut / Registry only)
let time = ctx.module::<GameClock>();                 // M: Copy
let party = ctx.clone_module::<Party>();              // M: Clone
let mods = ctx.try_borrow::<ModSettings>();           // Option<Ref<'_, ModSettings>>: may not be registered

let overworld = host.borrow::<OverworldModule>();     // from the UI, between steps (read-only)
```
<!-- SPDX-SnippetEnd -->

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

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
/// A read-only view of the modules. Cheap to copy; borrows from whatever it was obtained from.
#[derive(Clone, Copy)]
pub struct Ctx<'a>(&'a Registry);

impl Registry { pub fn ctx(&self) -> Ctx<'_>; }
impl CtxMut { pub fn ctx(&self) -> Ctx<'_>; }
```
<!-- SPDX-SnippetEnd -->

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
- On the game side, everything that can suspend takes `&mut CtxMut`: `ctx.emit(..)`, `ctx.submit(..)`, and
  `next.forward(ctx, ..)`. A `Ref`/`RefMut` (or a `Ctx` from `ctx.ctx()`) still alive at one of those is a compile
  error. Because `CtxMut` is not `Clone` and only `Host::new` creates one, there is no second handle to borrow from
  instead.
- On the UI side, `host.step()` takes `&mut self`, so a `Ref` from `host.borrow(..)` that is still alive at the next
  `step()` is a compile error.

A prototype confirmed that each of these violations is rejected:

| Violation | Result |
| --- | --- |
| Game holds `ctx.borrow_mut::<M>()` across `ctx.emit(..).await` (or `ctx.submit(..)`) | `E0502`: cannot borrow `ctx` as mutable because it is also borrowed as immutable |
| Listener holds `ctx.borrow::<M>()` across `next.forward(ctx, ev).await` | `E0502` on `*ctx` |
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

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
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
<!-- SPDX-SnippetEnd -->

`Host::new` creates a `Registry` by value, lets the [`Setup`](./games.md) edit it through `&mut Registry`, and then
moves it into an `Rc`. **That move is the freeze:** no one can get a `&mut Registry` out of a shared `Rc`, so
`insert`, `on_event`, and `on_request` on `Registry` can never be called again, and the compiler enforces it. No staging
map is needed.

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
pub struct Registry {
   /// One module per type, keyed by the module's own type: `TypeId::of::<M>()`. Fixed after setup, so `borrow` can
   /// hand out a `Ref<'_, M>` straight from the map.
   modules: HashMap<TypeId, Box<RefCell<dyn Module>>>,

   /// One `event::Def<E>` per event type, keyed by the event's type: `TypeId::of::<E>()`.
   events: RefCell<HashMap<TypeId, Rc<dyn event::AnyDef>>>,

   /// One `request::Def<R>` per request type, keyed by the request's type: `TypeId::of::<R>()`.
   requests: RefCell<HashMap<TypeId, Rc<dyn request::AnyDef>>>,

   /// The slot the game is currently parked on, if any. See "Under the hood".
   current: RefCell<Option<Suspension>>,
}
```
<!-- SPDX-SnippetEnd -->

Notes:

- **Every map is a `HashMap<TypeId, _>` whose values are trait objects**, so only the right kind of thing can be
  stored: a `dyn Module`, a `dyn event::AnyDef`, or a `dyn request::AnyDef`. Each value can be used through its trait
  without downcasting (for example, `module.name()` in a panic message), and downcast to its concrete type after
  lookup when needed (see [Storing modules and definitions](#storing-modules-and-definitions)).
- **The key's source differs:** a module is keyed by **its own type** (`M`), while an event or request definition is
  keyed by **the message type it handles** (`E` or `R`), not by the `Def` type.
- **What changes after setup:** `modules` never does. `events` and `requests` sit in `RefCell`s only because
  `Host::on_event` / `Host::on_request` may add a `Def` (holding just a UI callback) for a message type the game
  registered no listeners for. Those maps are borrowed only long enough to clone an `Rc`, never across an `.await`.
  Game listeners can't be added after setup, since registering them needs `&mut Registry`.
- **Each `Def` holds its message's game listeners and its UI callback,** and dispatches itself; see
  [Events, requests, and listeners](./events.md#definitions-and-dispatch).

### Storing modules and definitions

**Modules.** `Module: Any`, so a module can be stored as `dyn Module` and still be downcast:

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
// Registry::insert (setup only):
let previous = self.modules.insert(TypeId::of::<M>(), Box::new(RefCell::new(module))); // coerces to dyn Module
// A duplicate panics, naming the module with `name()`, a trait call that needs no downcast.

// Registry::borrow (behind `module::Borrow`):
let cell = self.modules.get(&TypeId::of::<M>()).expect("module not registered");
Ref::map(cell.borrow(), |m| (m as &dyn Any).downcast_ref::<M>().unwrap())
```
<!-- SPDX-SnippetEnd -->

`m as &dyn Any` is trait upcasting (`&dyn Module` to `&dyn Any`), stable since Rust 1.86, the plan's MSRV. The
downcast always succeeds because the map is keyed by `TypeId::of::<M>()`. `borrow_mut` is the same with
`RefMut::map`.

**Definitions.** `event::Def<E>` implements the private marker trait `event::AnyDef` (just `Any` plus the event's type
name), so every `Def` fits in one map regardless of `E`; `request::Def<R>` and `request::AnyDef` likewise. Both
dispatch and registration downcast the `Rc<dyn event::AnyDef>` to `Rc<event::Def<E>>` first, with
`def.downcast::<E>()`: an inherent method on `dyn AnyDef` (like `downcast` on `dyn Any`) built on trait upcasting to
`Rc<dyn Any>`. It always succeeds because the key is `TypeId::of::<E>()`. After that,
everything is fully typed: `Def<E>::emit` takes an `E`, and `Def<R>::submit` takes an `R` and returns an
`R::Response`, with no `Box<dyn Any>` in sight.

**Why a `HashMap`, not a `HashSet` of trait objects?** A set would have to hash and compare entries by their
`TypeId`, which means borrowing each module's `RefCell` to ask for it (and panicking if an unrelated module is
mutably borrowed), and looking an entry up by `TypeId` would need a local wrapper type implementing
`Borrow<TypeId>`. Keeping the `TypeId` as the map's key avoids both, with the same guarantees.
