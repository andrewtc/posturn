pub mod direction;
pub mod snake;

use std::mem::swap;

use macroquad::{math::{I16Vec2, U16Vec2}, rand::{srand, RandomRange}};
use posturn::Play;

use snake::Snake;
use direction::Direction;

#[derive(Debug, Clone, Copy)]
pub struct WaitForInput;

#[derive(Debug)]
pub struct Game
{
   pub play_area_half_extents : U16Vec2,
   pub random_seed : u64,
   pub snakes : Vec<Snake>,
   pub player_index : usize,
}

impl Game {
   fn handle_pre_collisions(&mut self, old_snakes : &Vec<Snake>) {
      for (snake_index, snake) in self.snakes.iter_mut().enumerate() {
         let dest = snake.head_tile() + snake.facing();

         if snake.alive {
            if snake.is_overlapping(dest) {
               // A Snake dies if it bites itself.
               snake.alive = false;
            }

            for (overlapping_index, overlapping_snake) in old_snakes.iter().enumerate() {
               if snake_index == overlapping_index {
                  continue;
               }

               if overlapping_snake.head_tile() == dest && overlapping_snake.can_decap(&snake)
               {
                  // A Snake dies if it runs into the head of a bigger Snake.
                  snake.alive = false;
               }
            }
         }
      }
   }
}

impl Play for Game {
   type Event = WaitForInput;
   type Input = Option<Direction>;
   type Outcome = ();

   fn play(ctx : posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         srand(ctx.host.borrow_game().random_seed);
         let mut old_snakes = vec![];

         loop {
            let input = ctx.yield_event(WaitForInput).await;

            ctx.host.with_game_mut(|mut game| {
               let play_area_half_extents = game.play_area_half_extents;

               old_snakes = game.snakes.clone();
               
               game.handle_pre_collisions(&old_snakes);
               
               swap(&mut game.snakes, &mut old_snakes);
               game.snakes.clear();

               let player_index =  game.player_index;
               for mut snake in old_snakes.drain(..) {
                  if !snake.alive {
                     game.snakes.push(snake);
                     continue;
                  }

                  let old_len = snake.len();
                  let facing = snake.facing();
                  let prev_head_tile = snake.head_tile();
                  let next_head_tile = prev_head_tile + facing;
                  let facing_delta = facing.delta();
                  let (cw, ccw) = (facing.cw(), facing.ccw());
                  let play_area_delta = next_head_tile.saturating_div(play_area_half_extents.as_i16vec2());

                  if play_area_delta != I16Vec2::ZERO {
                     // This snake is going to be outside the play area. Turn around and move back toward the center.
                     let cw_delta = cw.delta();
                     let ccw_delta = ccw.delta();

                     if play_area_delta.x < 0 && cw_delta.x > 0 ||
                        play_area_delta.x > 0 && cw_delta.x < 0 ||
                        play_area_delta.y < 0 && cw_delta.y > 0 ||
                        play_area_delta.y > 0 && cw_delta.y < 0 {
                        snake.grow_cw();
                     }
                     else if play_area_delta.x < 0 && ccw_delta.x > 0 ||
                        play_area_delta.x > 0 && ccw_delta.x < 0 ||
                        play_area_delta.y < 0 && ccw_delta.y > 0 ||
                        play_area_delta.y > 0 && ccw_delta.y < 0 {
                        snake.grow_ccw();
                     }
                     else if play_area_delta.x > 0 && facing_delta.x > 0 ||
                        play_area_delta.x < 0 && facing_delta.x < 0 ||
                        play_area_delta.y > 0 && facing_delta.y > 0 ||
                        play_area_delta.y < 0 && facing_delta.y < 0 {
                        snake.grow_cw();
                     }
                     else {
                        snake.grow_forward();
                     }
                  }
                  else if snake.player_index != player_index {
                     // Turn randomly to simulate player input.
                     const CHANCE_TO_TURN : f32 = 0.1;
                     if f32::gen_range(0.0, 1.0) <= CHANCE_TO_TURN {
                        if u8::gen_range(0, 2) == 0 { snake.grow_cw(); }
                        else { snake.grow_ccw(); }
                     }
                     else {
                        snake.grow_forward();
                     }
                  }
                  else if let Some(direction) = input {
                     // Allow the player to steer.
                     if direction == cw { snake.grow_cw(); }
                     else if direction == ccw { snake.grow_ccw(); }
                     else { snake.grow_forward(); }
                  }
                  else {
                     snake.grow_forward();
                  }

                  assert_ne!(prev_head_tile, snake.head_tile());

                  if snake.len() >= snake.target_len() {
                     // Unless we are growing, shrink the tail of the Snake during movement.
                     snake = snake.shrink_tail().unwrap();
                     assert_eq!(snake.len(), old_len, "The snake should always stay the same length");
                  }

                  game.snakes.push(snake);
               }
            });
         }
      }
   }
}