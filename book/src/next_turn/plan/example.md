<!--
SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Worked example

An RPG with an overworld and a battle minigame, written against the planned API. This is the design target, not
compiling code yet.

## Modules, events, and requests

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
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

/// Who sees a message first. The variant declared last runs first: mods, then the rules, then the UI.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Ui, Rules, Mods }

/// A request: the game can't continue until it knows what the party does next.
pub struct PartyInputRequest;
impl Request for PartyInputRequest {
   type Response = PartyInput;
   type Priority = Priority;
}

/// Events: declared explicitly with the `Event` trait.
pub struct PartyMovedEvent { pub direction: Direction }
impl Event for PartyMovedEvent { type Priority = Priority; }

pub struct PartyRestedEvent { pub hours: NonZeroU8 }
impl Event for PartyRestedEvent { type Priority = Priority; }
```
<!-- SPDX-SnippetEnd -->

## Setup

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
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

      // React to our own move event, so that mods (at a higher priority) can wrap or override the default. It's part
      // of the rules for the whole game, so it gives up its handle.
      registry.on_event(Priority::Rules, on_party_moved).forever();

      async move |ctx: CtxMut| play(ctx).await
   }
}
```
<!-- SPDX-SnippetEnd -->

## The game

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
async fn play(mut ctx: CtxMut) -> Infallible {
   loop {
      match ctx.submit(PartyInputRequest).await {
         PartyInput::Move { direction } => {
            // A temporary borrow, dropped at the end of the statement.
            let moved = ctx.borrow_mut::<OverworldModule>().try_move(direction);
            if moved {
               ctx.emit(PartyMovedEvent { direction }).await;
            }
         }
         PartyInput::Rest { hours } => {
            ctx.borrow_mut::<OverworldModule>().time_of_day += GameDuration::hours(hours.get());
            ctx.emit(PartyRestedEvent { hours }).await;
         }
         PartyInput::UseItem { item_idx } => {
            // Delegate to another subsystem's async logic.
            inventory::use_item(&mut ctx, item_idx).await;
         }
      }
   }
}

async fn on_party_moved(ctx: &mut CtxMut, ev: PartyMovedEvent, next: event::Next<PartyMovedEvent>) {
   ctx.borrow_mut::<OverworldModule>().time_of_day += GameDuration::minutes(1);

   // Scripts may play animations, ask questions, or start a battle before returning here.
   evaluate_triggers(ctx, &ev).await;

   if ctx.borrow::<BattleModule>().is_battle_in_progress() {
      // The whole battle minigame runs here. The UI switches screens when it sees the battle start; see "The UI".
      let outcome = battle::play(ctx).await;
      if outcome.is_defeat() {
         ctx.borrow_mut::<OverworldModule>().respawn_party();
      }
   }

   // Let the rest of the chain (ultimately, the UI's walk animation) see the move.
   next.forward(ctx, ev).await
}
```
<!-- SPDX-SnippetEnd -->

Every module borrow here ends before the next `await`. Writing, say,
`let overworld = ctx.borrow_mut::<OverworldModule>();` and then awaiting an emit while `overworld` is still in use
would not compile.

## The UI

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
let ui = Rc::new(RefCell::new(Ui::new()));
let mut host = Host::new(rpg::Setup::from_save(&save));

// The overworld screen's listeners, at the bottom of each chain. They stay registered while it's open.
let mut overworld_screen = OverworldScreen::open(&host, &ui);
let mut battle_screen = None;

loop {
   host.step(); // No-op while the UI still holds a resume handle.

   // Switch screens between steps. The UI can't register listeners from inside a callback, which can't reach the
   // `Host`; so the battle-start listener keeps its `event::Resume`, and the game waits here until the switch is done.
   if ui.borrow().battle_starting() && battle_screen.is_none() {
      battle_screen = Some(BattleScreen::open(&host, &ui)); // Registers the battle's listeners...
      ui.borrow_mut().release_battle_start(); // ...then drops the `event::Resume`, letting the battle begin.
   }
   if ui.borrow().battle_ended() {
      battle_screen = None; // Drops its handles, which unregisters its listeners.
   }

   let overworld = host.borrow::<OverworldModule>();
   ui.borrow_mut().update_and_render(&overworld); // May use a stored resume handle.
}

impl OverworldScreen {
   fn open(host: &Host<Infallible>, ui: &Rc<RefCell<Ui>>) -> Self {
      let u = ui.clone();
      let prompt = host.on_request(Priority::Ui, move |ctx, _: &PartyInputRequest, resume| {
         u.borrow_mut().prompt_party(ctx, resume)
      });
      let u = ui.clone();
      let walk = host.on_event(Priority::Ui, move |_ctx, ev: &PartyMovedEvent, resume| {
         u.borrow_mut().animate_walk(ev.direction, resume)
      });
      let u = ui.clone();
      let rest = host.on_event(Priority::Ui, move |_ctx, ev: &PartyRestedEvent, resume| {
         u.borrow_mut().fade_to_black(ev.hours, resume)
      });
      let u = ui.clone();
      let battle = host.on_event(Priority::Ui, move |_ctx, _: &BattleStartedEvent, resume| {
         u.borrow_mut().hold_battle_start(resume) // Kept until the battle screen is open.
      });
      OverworldScreen { requests: vec![prompt], events: vec![walk, rest, battle] }
   }
}
```
<!-- SPDX-SnippetEnd -->

## A mod that overrides behavior

A mod is naturally a `Setup` that wraps another one: it lets the inner setup insert its modules and register its
listeners, then registers its own. They run first because of their priority, `Priority::Mods`, not because they were
registered later. The mod's listeners last the whole game, so it gives up their handles with `.forever()`. (A mod
that could be switched off mid-game would keep them in a module instead, like `StatusModule` in
[Handles](./events.md#handles).)

<!--
SPDX-SnippetBegin
SPDX-SnippetCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
SPDX-License-Identifier: MIT
-->
```rust,ignore
pub struct IceAndRest<S>(pub S);

impl<S: posturn::Setup> posturn::Setup for IceAndRest<S> {
   type Outcome = S::Outcome;

   fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> S::Outcome + 'static {
      let game = self.0.set_up(registry);

      // At `Priority::Mods`, so it wraps the game's own listener.
      registry.on_event(Priority::Mods, async |ctx: &mut CtxMut, ev: PartyMovedEvent, next| {
         if ctx.borrow::<OverworldModule>().is_on_ice() {
            // Slide one extra tile, then let the default behavior run.
            ctx.borrow_mut::<OverworldModule>().try_move(ev.direction);
         }
         next.forward(ctx, ev).await
      }).forever();

      // Auto-rest when the party is exhausted, without ever showing the prompt.
      registry.on_request(Priority::Mods, async |ctx: &mut CtxMut, req: PartyInputRequest, next| {
         if ctx.borrow::<OverworldModule>().party_is_exhausted() {
            return PartyInput::Rest { hours: NonZeroU8::new(8).unwrap() };
         }
         next.forward(ctx, req).await
      }).forever();

      game
   }
}

let mut host = Host::new(IceAndRest(rpg::Setup::from_save(&save)));
```
<!-- SPDX-SnippetEnd -->
