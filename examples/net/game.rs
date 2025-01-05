use posturn::Play;

use crate::snake::Snake;

#[derive(Debug, Clone, Copy)]
pub struct WaitForInput;

#[derive(Debug)]
pub struct Game
{
   pub snakes : Vec<Snake>,
}

impl Play for Game {
   type Event = WaitForInput;
   type Input = ();
   type Outcome = ();

   fn play(ctx : posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         loop {
            let _input = ctx.yield_event(WaitForInput).await;

            ctx.host.with_game_mut(|mut game| {
               for snake in &mut game.snakes {
                  snake.step();
               }
            });
         }
      }
   }
}