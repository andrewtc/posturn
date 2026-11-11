# The `Host` and modules

**Decided:** the `Host` becomes a non-generic registry of **modules**, plus the registry of
[event listeners](./events.md). It is shared between exactly two parties on the same thread: the game coroutine and
the UI. It is never `Send` or `Sync`.

## Modules

A module is a piece of game state that the UI is allowed to inspect, such as an `OverworldModule` holding the level
and party position, or a `BattleModule` holding the current battle. This builds on the work-in-progress `module`
patch:

```rust,ignore
/// A piece of composable game state, published to the `Host` so that the UI can inspect it.
pub trait Module: Any + 'static {
   /// The name of the module, for diagnostics.
   fn name(&self) -> &str;
}
```

- **One instance per type.** Modules are keyed by `TypeId` (`BTreeMap<TypeId, Box<dyn Module>>` in the patch).
- **Accessors** mirror the old `*_game` family, but are generic over the module type:

  ```rust,ignore
  host.insert(OverworldModule::new(level));

  let overworld = host.borrow::<OverworldModule>();          // Ref<'_, OverworldModule>
  let mut battle = host.borrow_mut::<BattleModule>();        // RefMut<'_, BattleModule>
  let pos = host.with::<OverworldModule, _>(|o| o.party_location);
  let time = host.module::<GameClock>();                     // M: Copy
  let party = host.clone_module::<Party>();                  // M: Clone
  ```

- **A missing module panics** with a message naming the type (as in the patch).

## The borrowing rule

**Decided (goal):** the game must not hold a module borrow across an `.await`, and the UI must not hold a module
borrow while it steps the game.

How this is enforced is still an [open question](./open_questions.md#enforcing-the-borrowing-rule). The candidate
under consideration makes the borrow checker do it: module access borrows the context immutably while yielding
requires it mutably, and the UI reaches modules only through the driver, so stepping (`&mut self`) cannot overlap a
module borrow. The baseline, if no static scheme fits the event API, is per-module `RefCell`s whose runtime panics
name the module and the conflicting access.

Note that because [events are types](./events.md) registered on the `Host` rather than fields of a module, emitting an
event never requires borrowing a module.
