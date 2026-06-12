# Where we are

Before getting started, it's important to understand the current state of `posturn` and the key advantages it offers.

`posturn` gives you an easy way to separate all turn-based _game logic_ from all _client-facing_ concerns such as input processing, rendering, networking, etc. The idea is that all code for enforcing the rules of your game lives in a separate layer such that you can write a complete UI layer on top of it without changing a single line of code.

For a framework that is all about event processing, the main game loop in `v0.3` of `posturn` leaves a lot to be desired. Let's take a look at a very simple example:

```rust,no_run
extern crate posturn;

#[macro_use]
extern crate futures;

# mod game {
#    pub struct PlayerInput;
#    impl PlayerInput { pub fn default() -> Self { Self } }
#    pub struct MyGame;
#    impl MyGame { pub fn new() -> Self { Self } }
#    impl posturn::Play for MyGame {
#       type Input = PlayerInput;
#       type Event = ();
#       type Outcome = ();
#       fn play(_ctx : posturn::Context<Self>) -> impl futures::Future<Output = ()> { async { } }
#    }
# }
# mod ui {
#    use crate::game::{MyGame, PlayerInput};
#    pub enum UiEvent { PlayerInput(PlayerInput), NewGame, Quit }
#    pub fn wait_for_input(_outcome: &Option<()>, _game: &MyGame) -> UiEvent { UiEvent::PlayerInput(PlayerInput::default()) }
#    pub fn handle_event(_event : ()) { }
# }
use game::{MyGame, PlayerInput};
use ui::{handle_event, UiEvent, wait_for_input};
use posturn::{genawaiter::{self, Coroutine}, Host};
use futures::pin_mut;

fn main() {
   // NOTE: Every iteration of this loop will kick off a new play session.
   'new_game : loop {

      // TODO: Show in the UI that we're now in a game.

      // Set up.
      let host = Host::new(MyGame::new());

      // Start the first turn.
      let co = host.play().unwrap();
      pin_mut!(co);

      let mut player_input = PlayerInput::default();
      let mut outcome = None;

      loop {
         // Take the next turn.
         // NOTE: This loop is awkward. Why are we passing player_input on the first turn when there isn't any?
         outcome = match co.as_mut().resume_with(player_input) {
            genawaiter::GeneratorState::Yielded(event) => {
               // Process event from game.
               handle_event(event);

               // The game is not over (yet).
               None
            },
            genawaiter::GeneratorState::Complete(outcome) => {
               // Game over!
               Some(outcome)
            },
         };

         // Keep rendering the UI until we receive player input.
         // NOTE: with_game enables wait_for_input to access to the game state, which is required for drawing.
         let ui_event = host.with_game(|game| wait_for_input(&outcome, &game));

         match ui_event {
            UiEvent::PlayerInput(input) => {
               // NOTE: Way down here is where we actually receive player_input for the *next* turn.
               player_input = input;
            },
            UiEvent::NewGame => continue 'new_game, // The UI says we want to start a new game.
            UiEvent::Quit => break 'new_game, // The UI says we want to end the game and exit.
         };
      }
   }
}
```

Using this example, let's analyze some of the things that currently make `posturn` cumbersome to use.
