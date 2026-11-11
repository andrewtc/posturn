# Worked example

An RPG with an overworld and a battle minigame, written against the planned API. This is the design target, not
compiling code yet.

## Modules and events

```rust,ignore
/// Overworld state the UI needs in order to draw the map.
pub struct OverworldModule {
   pub level: Box<Level>,
   pub time_of_day: GameTimeOfDay,
   pub party_location: Vector2i,
   pub party: Vec<Character>,
}
impl Module for OverworldModule { fn name(&self) -> &str { "overworld" } }

pub enum PartyInput {
   Move { direction: Direction },
   Rest { hours: NonZeroU8 },
   UseItem { item_idx: usize },
}

pub struct PartyInputRequested;
impl Event for PartyInputRequested { type Output = PartyInput; }

pub struct PartyMoved { pub direction: Direction }
impl Event for PartyMoved { type Output = (); }

pub struct PartyRested { pub hours: NonZeroU8 }
impl Event for PartyRested { type Output = (); }
```

## The game

```rust,ignore
async fn play(setup: GameSetup, host: &Host) -> Infallible {
   host.insert(OverworldModule::load(&setup));

   // Respond to our own move event, so that other listeners registered later can wrap or override the default.
   host.listen(|host, ev: PartyMoved, next| async move {
      on_party_moved(&host, &ev).await;
      next.run(ev).await
   });

   loop {
      match host.emit(PartyInputRequested).await {
         PartyInput::Move { direction } => {
            let moved = host.with_mut::<OverworldModule, _>(|o| o.try_move(direction));
            if moved {
               host.emit(PartyMoved { direction }).await;
            }
         }
         PartyInput::Rest { hours } => {
            host.with_mut::<OverworldModule, _>(|o| o.time_of_day += GameDuration::hours(hours.get()));
            host.emit(PartyRested { hours }).await;
         }
         PartyInput::UseItem { item_idx } => {
            // Delegate to another subsystem's async logic.
            inventory::use_item(host, item_idx).await;
         }
      }
   }
}

async fn on_party_moved(host: &Host, ev: &PartyMoved) {
   host.with_mut::<OverworldModule, _>(|o| o.time_of_day += GameDuration::minutes(1));

   // Scripts may play animations, ask questions, or start a battle before returning here.
   evaluate_triggers(host, ev).await;

   if host.with::<BattleModule, _>(|b| b.is_battle_in_progress()) {
      // The whole battle minigame runs here. The UI switches screens via its own handlers for battle events.
      let outcome = battle::play(host).await;
      if outcome.is_defeat() {
         host.with_mut::<OverworldModule, _>(|o| o.respawn_party());
      }
   }
}
```

Note that every module access is a short, closure-scoped borrow, so nothing is borrowed while the game is suspended.

## The UI

```rust,ignore
let host = Host::new();

// Bottom of each chain: the UI.
host.on_input(|_: PartyInputRequested, wait| ui.borrow_mut().prompt_party(wait));
host.on_suspend(|ev: PartyMoved, s| ui.borrow_mut().animate_walk(ev.direction, s));
host.on_suspend(|ev: PartyRested, s| ui.borrow_mut().fade_to_black(ev.hours, s));

let mut session = Session::new(&host, play(setup, &host));
loop {
   session.step();                       // No-op while the UI still holds a handle.
   ui.borrow_mut().update_and_render(&host);  // May resume a stored handle.
}
```

## A mod that overrides behavior

```rust,ignore
// Registered after the game's own listener, so it runs first.
host.listen(|host, ev: PartyMoved, next| async move {
   if host.with::<OverworldModule, _>(|o| o.is_on_ice()) {
      // Slide one extra tile, then let the default behavior run.
      host.with_mut::<OverworldModule, _>(|o| o.try_move(ev.direction));
   }
   next.run(ev).await
});

// Auto-rest when the party is exhausted, without ever showing the prompt.
host.listen(|host, ev: PartyInputRequested, next| async move {
   if host.with::<OverworldModule, _>(|o| o.party_is_exhausted()) {
      return PartyInput::Rest { hours: NonZeroU8::new(8).unwrap() };
   }
   next.run(ev).await
});
```
