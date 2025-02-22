pub mod direction;
pub mod snake;

use std::{cmp::Reverse, collections::{BTreeMap, BinaryHeap}, mem::swap};

use macroquad::{math::{I16Vec2, U16Vec2}, rand::{srand, RandomRange}};
use posturn::Play;

use snake::{Overlap, Snake};
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
   fn handle_pre_collisions(&mut self, temp_snakes : &mut Vec<Snake>) {
      temp_snakes.clone_from(&self.snakes);

      for (snake_index, snake) in self.snakes.iter_mut().enumerate() {
         let dest = snake.head_tile() + snake.facing();

         if snake.alive {
            for (overlapping_index, overlapping_snake) in temp_snakes.iter().enumerate() {
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

   fn handle_movement(&mut self, input : Option<Direction>, temp_snakes : &mut Vec<Snake>) {
      let play_area_half_extents = self.play_area_half_extents;
      let player_index = self.player_index;

      temp_snakes.clear();

      for mut snake in self.snakes.drain(..) {
         if !snake.alive {
            temp_snakes.push(snake);
            continue;
         }

         let old_len = snake.len();
         let facing = snake.facing();
         let prev_head_tile = snake.head_tile();
         let next_head_tile = prev_head_tile + facing;
         let facing_delta = facing.delta();
         let (cw, ccw) = (facing.cw(), facing.ccw());
         let play_area_delta = next_head_tile.saturating_div(play_area_half_extents.as_i16vec2());

         let turned =
            if play_area_delta != I16Vec2::ZERO {
               // This snake is going to be outside the play area. Turn around and move back toward the center.
               let cw_delta = cw.delta();
               let ccw_delta = ccw.delta();

               if play_area_delta.x < 0 && cw_delta.x > 0 ||
                  play_area_delta.x > 0 && cw_delta.x < 0 ||
                  play_area_delta.y < 0 && cw_delta.y > 0 ||
                  play_area_delta.y > 0 && cw_delta.y < 0 {
                  snake.grow_cw().is_ok()
               }
               else if play_area_delta.x < 0 && ccw_delta.x > 0 ||
                  play_area_delta.x > 0 && ccw_delta.x < 0 ||
                  play_area_delta.y < 0 && ccw_delta.y > 0 ||
                  play_area_delta.y > 0 && ccw_delta.y < 0 {
                  snake.grow_ccw().is_ok()
               }
               else if play_area_delta.x > 0 && facing_delta.x > 0 ||
                  play_area_delta.x < 0 && facing_delta.x < 0 ||
                  play_area_delta.y > 0 && facing_delta.y > 0 ||
                  play_area_delta.y < 0 && facing_delta.y < 0 {
                  snake.grow_cw().is_ok()
               }
               else { false }
            }
            else if snake.player_index != player_index {
               // Turn randomly to simulate player input.
               const CHANCE_TO_TURN : f32 = 0.1;
               if f32::gen_range(0.0, 1.0) <= CHANCE_TO_TURN {
                  if u8::gen_range(0, 2) == 0 { snake.grow_cw().is_ok() }
                  else { snake.grow_ccw().is_ok() }
               }
               else { false }
            }
            else if let Some(direction) = input {
               // Allow the player to steer.
               if direction == cw { snake.grow_cw().is_ok() }
               else if direction == ccw { snake.grow_ccw().is_ok() }
               else { false }
            }
            else { false };
         
         if !turned && snake.grow_forward().is_err() && snake.grow_cw().is_err() && snake.grow_ccw().is_err() {
            // Snake can't move in any direction, so it is dead.
            snake.alive = false;
         }
         else {
            assert!(prev_head_tile != snake.head_tile() || !snake.alive, "Live Snakes should always move forward each turn");

            if snake.is_growing() {
               // Snake is growing, so it doesn't shrink this turn.
               snake.amt_to_grow -= 1;
            }
            else {
               // Shrink the tail of the Snake after moving, to keep it the same size.
               snake = snake.shrink_tail().unwrap();
               assert_eq!(snake.len(), old_len, "The snake should always stay the same length");
            }
         }

         temp_snakes.push(snake);
      }

      swap(&mut self.snakes, temp_snakes);
   }

   fn handle_post_collisions(
      &mut self,
      temp_snakes : &mut Vec<Snake>,
      temp_overlaps : &mut BinaryHeap<Reverse<Overlap>>,
      temp_points_by_player : &mut BTreeMap<usize, u16>)
   {
      temp_snakes.clear();
      temp_points_by_player.clear();

      for (snake_to_eat_index, snake_to_eat) in self.snakes.iter().enumerate() {
         temp_overlaps.clear();

         let mut decap = false;
         for (overlapping_snake_index, overlapping_snake) in self.snakes.iter().enumerate() {
            if snake_to_eat_index == overlapping_snake_index || !overlapping_snake.alive {
               continue;
            }

            let points = temp_points_by_player.entry(overlapping_snake.player_index);
            let add_point = || {
               points.and_modify(|value| *value = value.saturating_add(1)).or_insert(1)
            };

            if overlapping_snake.head_tile() == snake_to_eat.head_tile() && overlapping_snake.can_decap(snake_to_eat)
            {
               decap = true;
               add_point();
            }
            else if let Some(overlap) = snake_to_eat.find_body_overlap(overlapping_snake.head_tile()) {
               // Keep track of overlaps in reverse order, i.e. pop the smallest value first.
               temp_overlaps.push(Reverse(overlap));
               add_point();
            }
         }

         if !decap && temp_overlaps.is_empty() {
            temp_snakes.push(snake_to_eat.clone());
            continue;
         }

         // Remove the head, if necessary.
         let mut snake_to_eat =
            if decap { snake_to_eat.clone().shrink_head() }
            else { Some(snake_to_eat.clone()) };

         while let Some(Reverse(overlap)) = temp_overlaps.pop() {
            // Split each Snake that overlaps with another Snake into separate Snakes and place them on the board.
            let mut head_minus_tail = snake_to_eat.take().expect("Expected a tail to split");
            let tail = head_minus_tail.split_off(overlap.segment_index, overlap.offset);
            
            // Chomp the front of the tail as we add it back to the board.
            temp_snakes.extend(tail.shrink_head());

            snake_to_eat = Some(head_minus_tail);
         }

         temp_snakes.extend(snake_to_eat);
      }

      for snake in temp_snakes.iter_mut() {
         if !snake.alive { continue; }
         else if let Some(points) = temp_points_by_player.get(&snake.player_index) {
            // Add length to each Snake based on the number of points accumulated.
            snake.amt_to_grow = snake.amt_to_grow.saturating_add(*points);
         }
      }

      swap(&mut self.snakes, temp_snakes);
   }
}

impl Play for Game {
   type Event = WaitForInput;
   type Input = Option<Direction>;
   type Outcome = ();

   fn play(ctx : posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         srand(ctx.host.borrow_game().random_seed);

         let mut temp_snakes = vec![];
         let mut temp_overlaps = BinaryHeap::new();
         let mut temp_points_by_player = BTreeMap::new();

         loop {
            let input = ctx.yield_event(WaitForInput).await;

            ctx.host.with_game_mut(|mut game| {
               game.handle_pre_collisions(&mut temp_snakes);
               game.handle_movement(input, &mut temp_snakes);
               game.handle_post_collisions(&mut temp_snakes, &mut temp_overlaps, &mut temp_points_by_player);
            });
         }
      }
   }
}