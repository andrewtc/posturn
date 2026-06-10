# Where we are

Before getting started, it's important to understand the current state of `posturn` and the key advantages it offers.

`posturn` gives you an easy way to separate all turn-based _game logic_ from all _client-facing_ concerns such as input processing, rendering, networking, etc. The idea is that all code for enforcing the rules of your game lives in a separate layer such that you can write a complete UI layer on top of it without changing a single line of code.

Event processing in `v0.3` of `posturn` looks something like this:

```rust,no_run
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

There are a couple things that currently make input and event processing cumbersome with `posturn`.
