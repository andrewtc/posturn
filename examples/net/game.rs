use macroquad::rand::{srand, RandomRange};
use posturn::Play;

use crate::snake::{Direction, Snake, Status};

#[derive(Debug, Clone, Copy)]
pub struct WaitForInput;

#[derive(Debug)]
pub struct Game
{
   pub play_area_half_extents : (u8, u8),
   pub random_seed : u64,
   pub snakes : Vec<Snake>,
   pub player_index : usize,
}

impl Play for Game {
   type Event = WaitForInput;
   type Input = Option<Direction>;
   type Outcome = ();

   fn play(ctx : posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         srand(ctx.host.borrow_game().random_seed);

         loop {
            let input = ctx.yield_event(WaitForInput).await;

            ctx.host.with_game_mut(|mut game| {
               let (play_area_half_width, play_area_half_height) = game.play_area_half_extents;
               let player_index = game.player_index;

               for (index, snake) in &mut game.snakes.iter_mut().enumerate() {
                  if snake.status == Status::Dead {
                     continue;
                  }

                  let old_facing = snake.facing;
                  let (next_head_x, next_head_y) = snake.start + snake.facing;
                  let (facing_delta_x, facing_delta_y) = snake.facing.delta();
                  let (cw, ccw) = (snake.facing.cw(), snake.facing.ccw());
                  let (play_area_rel_x, play_area_rel_y) = (
                     next_head_x / play_area_half_width as i16,
                     next_head_y / play_area_half_height as i16);

                  if play_area_rel_x != 0 || play_area_rel_y != 0 {
                     // This snake is going to be outside the play area. Turn around and move back toward the center.
                     let (cw_delta_x, cw_delta_y) = cw.delta();
                     let (ccw_delta_x, ccw_delta_y) = ccw.delta();

                     if play_area_rel_x < 0 && cw_delta_x > 0 ||
                        play_area_rel_x > 0 && cw_delta_x < 0 ||
                        play_area_rel_y < 0 && cw_delta_y > 0 ||
                        play_area_rel_y > 0 && cw_delta_y < 0 {
                        snake.facing = cw;
                     }
                     else if play_area_rel_x < 0 && ccw_delta_x > 0 ||
                        play_area_rel_x > 0 && ccw_delta_x < 0 ||
                        play_area_rel_y < 0 && ccw_delta_y > 0 ||
                        play_area_rel_y > 0 && ccw_delta_y < 0 {
                        snake.facing = ccw;
                     }
                     else if play_area_rel_x > 0 && facing_delta_x > 0 ||
                        play_area_rel_x < 0 && facing_delta_x < 0 ||
                        play_area_rel_y > 0 && facing_delta_y > 0 ||
                        play_area_rel_y < 0 && facing_delta_y < 0 {
                        snake.facing = cw;
                     }
                  }
                  else if index != player_index {
                     // Turn randomly to simulate player input.
                     const CHANCE_TO_TURN : f32 = 0.1;
                     if f32::gen_range(0.0, 1.0) <= CHANCE_TO_TURN {
                        snake.facing =
                           if u8::gen_range(0, 2) == 0 { cw }
                           else { ccw };
                     }
                  }
                  else if let Some(direction) = input {
                     // Allow the player to steer.
                     snake.facing = direction;
                  }

                  if snake.facing == old_facing.opposite() {
                     // Never allow snakes to turn 180 degrees in one turn.
                     snake.facing = old_facing;
                  }

                  snake.step();
               }
            });
         }
      }
   }
}