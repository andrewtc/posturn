# 🎲 Next Turn
> This contains documentation regarding the next upcoming release of `posturn`, **v0.4.0**.

## Problems with event processing
Event processing in `v0.3` of `posturn` looks something like this:

```rust
# use std::pin::pin;
# 
# struct Host<G> { game: G }
# struct MyGame;
# impl MyGame { fn default() -> Self { MyGame } }
# impl Host<MyGame> {
#     fn new(_: MyGame) -> Self { Host { game: MyGame } }
#     fn play(&self) -> Result<(), ()> { Ok(()) }
#     fn with_game<F, R>(&self, f: F) -> Result<R, ()> where F: FnOnce(&MyGame) -> R { Ok(f(&self.game)) }
# }
# enum UiEvent { PlayerInput(()), NewGame, Quit }
# fn wait_for_input(_out: &mut (), _game: &MyGame) -> UiEvent { UiEvent::PlayerInput(()) }
fn main() {
   // Every iteration of this loop starts a new game.
   'new_game : loop {
      // TODO: Change the current UI state to reflect that we're in a game.

      // Set up the game.
      let host = Host::new(MyGame::default());

      // Call the MyGame coroutine to start playing.
      let _co = host.play().unwrap();

      let mut player_input = ();
      let mut outcome = None;

      // NOTE: This loop is awkward mainly because we have to pass in player_input on the first iteration,
      // even though MyGame hasn't been prompted us for input!
      while outcome.is_none() {
         outcome = Some(());

         // Respond
         match host.with_game(|game| wait_for_input(&mut Default::default(), &game)).unwrap() {
            UiEvent::PlayerInput(input) => {
               // NOTE: Way down here is where we actually set player_input,
               // after the player chooses to perform an action on their turn.
               player_input = input;
            },
            UiEvent::NewGame => continue 'new_game, // The UI says we want to start a new game.
            UiEvent::Quit => break 'new_game, // The UI says we want to end the game and exit.
         };
      }
   }
}
```

There are a couple things that currently make input and event processing cumbersome with `posturn`:

## Null input
The game only expects meaningful input **after** it has prompted us for input. This is because the game is really yielding control _in response_ to an event which requires input. This means we have to supply "null" input, e.g. `Default::default()`, `None`, in cases where the game has **not** (yet) prompted the player but we want to resume the game, e.g. on the first `resume_with` in the example above.

It would be nice to reverse this. Instead of requiring the game loop to know when to supply invalid inputs, we should invert control. We should be able to resume the game without arguments in all cases, and the game should _ask_ for input when it needs it.

## Input bloat
You can imagine that a very small game like `TicTacToe` would have very simple inputs:

```rust
/// The player input for our TicTacToe game.
struct Input {
   /// The row on the board where the next piece should go.
   row : i8,

   /// The column on the board where the next piece should go.
   col : i8,
}
```

Each player taking a turn only needs to specify the position on the game board they want their next piece to occupy. Thus, the `Play` implementation can simply take this $$(x, y)$$ position as input. The UI is expected to supply this same type of input with every resume.

For a very complex game, e.g. an RPG battle simulator, this becomes much more complex due to an overwhelming number of states the game can be in. Players can use items, cast spells, attack enemies, change equipment, etc. The more distinct actions a player can take, the more complex input processing becomes on the game side.

However, the real bugbear is that the complexity of the input structure grows exponentially as the number of unique _states_ of the game increases. Consider an `RpgGame` where we have two distinct states of play:

1. Choosing what the party wants to do while traveling around a dungeon map
2. Battling enemies

When we are battling enemies, an action like "rest" may not even make sense. While navigating a dungeon, an input of "attack enemy 2" is similarly meaningless. Unfortunately, `posturn` only allows us to specify one `Input` type with which we can resume the game, so we end up with something like this:

```rust
# use std::time::Duration;
# type Direction = ();
# type ItemId = ();
# type CharacterId = ();
# type EnemyId = ();
# 
/// The input for our RPG game.
pub enum Input {
   /// Inputs that make sense when we are navigating a dungeon.
   Dungeon(DungeonInput),

   /// Inputs that make sense when we are in battle.
   Battle(BattleInput),
}

pub enum DungeonInput {
   /// Move the party in a direction.
   Move(Direction),

   /// Rest for some period of time.
   Rest(Duration),

   /// Equip an item to **a specific character** (chosen by the player) in the party.
   Equip(ItemId, CharacterId),

   /// Unequip an item from **a specific character** (chosen by the player) in the party.
   Unequip(ItemId, CharacterId),

   // …
}

pub enum BattleInput {
   /// Move the party in a direction.
   Attack(EnemyId),

   /// Run away!
   Flee,

   /// Equip an item to the **currently active character**, consuming an action.
   Equip(ItemId),

   /// Unequip an item from the **currently active character**, consuming an action.
   Unequip(ItemId),

   // …
}
```

Our `Input` structure is already complex, with multiple levels of nesting, and we only have two possible states of the game. Each time we resume the game, we have to pass a structure which mirrors the current state of the game, e.g. `Input::Battle(BattleInput::Attack(enemy_id))`, and the game has to filter out any inputs that don't make sense:

```rust
# use std::time::Duration;
# type Direction = ();
# type ItemId = ();
# type CharacterId = ();
# type EnemyId = ();
# 
# enum Input { Dungeon(()), Battle(BattleInput) }
# enum BattleInput { Attack(usize), Flee, Equip(usize), Unequip(usize) }
#
# struct RpgGame { battle: Battle }
# struct Battle { num_remaining_actions: u8 }
# 
# impl RpgGame { 
#     fn new() -> Self { RpgGame { battle: Battle { num_remaining_actions: 1 } } } 
# }
# 
# impl Battle {
#     fn attack(&mut self, _: usize) {}
#     fn flee(&mut self) {}
#     fn try_equip(&mut self, _: usize) -> Result<(), ()> { Ok(()) }
#     fn try_unequip(&mut self, _: usize) -> Result<(), ()> { Ok(()) }
# }
#
impl posturn::Play for RpgGame {
    type Input = BattleInput;
    type Event = ();
    type Outcome = ();

    fn play(ctx: posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
        async move {
            // In RpgGame, while we are "in battle":
            while ctx.host.borrow_game().battle.num_remaining_actions > 0 {
                // Time to ask the current player what they want to do in battle.
                let battle_input = ctx.yield_event(()).await;
                let mut num_actions_consumed = 0;

                match battle_input {
                    BattleInput::Attack(enemy_id) => {
                        ctx.host.borrow_game_mut().battle.attack(enemy_id);
                        num_actions_consumed += 1;
                    },
                    BattleInput::Flee => {
                        ctx.host.borrow_game_mut().battle.flee();
                    },
                    BattleInput::Equip(item_id) => {
                        let result = ctx.host.borrow_game_mut().battle.try_equip(item_id);
                        if result.is_ok() {
                            num_actions_consumed += 1;
                        }
                    },
                    BattleInput::Unequip(item_id) => {
                        let result = ctx.host.borrow_game_mut().battle.try_unequip(item_id);
                        if result.is_ok() {
                            num_actions_consumed += 1;
                        }
                    }
                }

                // Consume actions.
                ctx.host.borrow_game_mut().battle.num_remaining_actions -= num_actions_consumed;
            }
        }
    }
}
```

We can make some of this complexity go away by defining `From`/`into` conversions which wrap or unwrap nested `enum` types. However, you can imagine how this `Input` structure would become increasingly unwieldy with each new feature we add to the game, e.g. a lock-picking minigame or overworld map.

Instead of requiring that a game have a unified `Input` structure, a game should specify what kind of input it expects each time it yields control to the main program. The main program should then _only_ be able to resume the game with an input that makes sense.