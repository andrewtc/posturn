# Worked example

An RPG with an overworld and a battle minigame, written against the planned API. This is the design target, not
compiling code yet.

## Modules, events, and requests

```rust,ignore
/// Overworld state the UI needs in order to draw the map.
pub struct OverworldModule {
   pub level: Box<Level>,
   pub time_of_day: GameTimeOfDay,
   pub party_location: Vector2i,
   pub party: Vec<Character>,
}
impl Module for OverworldModule {} // `name()` defaults to the type name.

pub enum PartyInput {
   Move { direction: Direction },
   Rest { hours: NonZeroU8 },
   UseItem { item_idx: usize },
}

/// A request: the game can't continue until it knows what the party does next.
pub struct PartyInputRequested;
impl Request for PartyInputRequested { type Response = PartyInput; }

/// Events: declared explicitly with the `Event` marker trait.
pub struct PartyMoved { pub direction: Direction }
impl Event for PartyMoved {}

pub struct PartyRested { pub hours: NonZeroU8 }
impl Event for PartyRested {}
```

## Setup

```rust,ignore
/// Everything needed to start (or resume) a playthrough.
pub struct Setup {
   pub level: Box<Level>,
   pub party: Vec<Character>,
   pub inventory: Inventory,
}

impl posturn::Setup for Setup {
   type Outcome = Infallible; // The game never ends.

   fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Infallible + 'static {
      registry.insert(OverworldModule::new(self.level, self.party));
      registry.insert(InventoryModule::new(self.inventory));
      registry.insert(BattleModule::default()); // No battle in progress yet, but the module exists from the start.

      // React to our own move event, so that listeners registered later can wrap or override the default.
      registry.on_event(on_party_moved);

      async move |ctx: CtxMut| play(ctx).await
   }
}
```

## The game

```rust,ignore
async fn play(mut ctx: CtxMut) -> Infallible {
   loop {
      match ctx.request(PartyInputRequested).await {
         PartyInput::Move { direction } => {
            // A temporary borrow, dropped at the end of the statement.
            let moved = ctx.borrow_mut::<OverworldModule>().try_move(direction);
            if moved {
               ctx.emit(PartyMoved { direction }).await;
            }
         }
         PartyInput::Rest { hours } => {
            ctx.borrow_mut::<OverworldModule>().time_of_day += GameDuration::hours(hours.get());
            ctx.emit(PartyRested { hours }).await;
         }
         PartyInput::UseItem { item_idx } => {
            // Delegate to another subsystem's async logic.
            inventory::use_item(&mut ctx, item_idx).await;
         }
      }
   }
}

async fn on_party_moved(ctx: &mut CtxMut, ev: PartyMoved, next: event::Next<PartyMoved>) {
   ctx.borrow_mut::<OverworldModule>().time_of_day += GameDuration::minutes(1);

   // Scripts may play animations, ask questions, or start a battle before returning here.
   evaluate_triggers(ctx, &ev).await;

   if ctx.borrow::<BattleModule>().is_battle_in_progress() {
      // The whole battle minigame runs here. The UI switches screens via its own callbacks for battle events.
      let outcome = battle::play(ctx).await;
      if outcome.is_defeat() {
         ctx.borrow_mut::<OverworldModule>().respawn_party();
      }
   }

   // Let the rest of the chain (ultimately, the UI's walk animation) see the move.
   next.run(ctx, ev).await
}
```

Every module borrow here ends before the next `await`. Writing, say,
`let overworld = ctx.borrow_mut::<OverworldModule>();` and then awaiting an emit while `overworld` is still in use
would not compile.

## The UI

```rust,ignore
let ui = Rc::new(RefCell::new(Ui::new()));
let mut host = Host::new(rpg::Setup::from_save(&save));

// Bottom of each chain: the UI.
let u = ui.clone();
host.on_request(move |ctx, _: PartyInputRequested, handle| u.borrow_mut().prompt_party(ctx, handle));
let u = ui.clone();
host.on_event(move |_ctx, ev: PartyMoved, handle| u.borrow_mut().animate_walk(ev.direction, handle));
let u = ui.clone();
host.on_event(move |_ctx, ev: PartyRested, handle| u.borrow_mut().fade_to_black(ev.hours, handle));

loop {
   host.step(); // No-op while the UI still holds a handle.

   let overworld = host.borrow::<OverworldModule>();
   ui.borrow_mut().update_and_render(&overworld); // May resume a stored handle.
}
```

## A mod that overrides behavior

Because listeners can only be registered during setup, a mod is naturally a `Setup` that wraps another one: it lets
the inner setup register its modules and listeners first, then registers its own on top.

```rust,ignore
pub struct IceAndRest<S>(pub S);

impl<S: posturn::Setup> posturn::Setup for IceAndRest<S> {
   type Outcome = S::Outcome;

   fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> S::Outcome + 'static {
      let game = self.0.set_up(registry);

      // Registered after the game's own listener, so it runs first.
      registry.on_event(async |ctx: &mut CtxMut, ev: PartyMoved, next: event::Next<PartyMoved>| {
         if ctx.borrow::<OverworldModule>().is_on_ice() {
            // Slide one extra tile, then let the default behavior run.
            ctx.borrow_mut::<OverworldModule>().try_move(ev.direction);
         }
         next.run(ctx, ev).await
      });

      // Auto-rest when the party is exhausted, without ever showing the prompt.
      registry.on_request(async |ctx: &mut CtxMut, req: PartyInputRequested, next| {
         if ctx.borrow::<OverworldModule>().party_is_exhausted() {
            return PartyInput::Rest { hours: NonZeroU8::new(8).unwrap() };
         }
         next.run(ctx, req).await
      });

      game
   }
}

let mut host = Host::new(IceAndRest(rpg::Setup::from_save(&save)));
```
