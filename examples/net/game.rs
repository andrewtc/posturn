pub mod direction;
pub mod snake;

use std::{cmp::Reverse, collections::{BTreeMap, BinaryHeap}, mem::swap, num::NonZeroUsize};

use macroquad::{math::{i16vec2, I16Vec2, U16Vec2}, rand::{srand, RandomRange}};
use posturn::Play;

use snake::{Overlap, Snake};
use direction::{Direction, Offset};

#[derive(Debug, Clone, Copy)]
pub struct WaitForInput;
#[derive(Clone, Debug)]
pub struct Setup {
   pub play_area_half_extents : U16Vec2,
   pub random_seed : u64,
   pub snakes_to_spawn : Vec<snake::SpawnParams>,
   pub player_index : usize,
}

#[derive(Debug)]
pub struct Game {
   play_area_half_extents : U16Vec2,
   random_seed : u64,
   snakes : Vec<Snake>,
   player_index : usize,
}

impl Game {
   pub fn with_setup(setup : Setup) -> Self {
      let player_count : NonZeroUsize = setup.snakes_to_spawn.len().try_into().expect("Must have at least one Snake to spawn");
      let spawn_locations = Self::choose_random_spawn_locations(setup.play_area_half_extents, player_count);

      let snakes = setup.snakes_to_spawn.into_iter()
         .enumerate()
         .zip(spawn_locations)
         .map(|((player_index, params), head_tile)| {
            Snake::try_spawn_at(head_tile, player_index, params)
               .expect(&format!("Spawn parameters for player {player_index}'s Snake were invalid"))
         })
         .collect();

      Self {
         play_area_half_extents: setup.play_area_half_extents,
         random_seed: setup.random_seed,
         snakes,
         player_index: setup.player_index,
      }
   }

   fn choose_random_spawn_locations(play_area_half_extents : U16Vec2, player_count : NonZeroUsize) -> Vec<I16Vec2> {
      struct SpawnArea {
         corner : I16Vec2,
         direction : Direction,
         length : usize,
      }

      // Trace a hollow box around the play area, one tile thick. These are our potential spawn locations.
      let spawn_area_half_extents = i16vec2(
         play_area_half_extents.x as i16 + 1,
         play_area_half_extents.y as i16 + 1);

      let spawn_areas = [
         SpawnArea {
            corner: spawn_area_half_extents * i16vec2(1, 1),
            direction: Direction::West,
            length: spawn_area_half_extents.x as usize * 2,
         },
         SpawnArea {
            corner: spawn_area_half_extents * i16vec2(-1, 1) - i16vec2(0, 1),
            direction: Direction::North,
            length: spawn_area_half_extents.y.saturating_sub(1) as usize * 2,
         },
         SpawnArea {
            corner: spawn_area_half_extents * i16vec2(-1, -1),
            direction: Direction::East,
            length: spawn_area_half_extents.x as usize * 2,
         },
         SpawnArea {
            corner: spawn_area_half_extents * i16vec2(1, -1) + i16vec2(0, 1),
            direction: Direction::South,
            length: spawn_area_half_extents.y.saturating_sub(1) as usize * 2,
         },
      ];

      // Determine how far each spawn location will be apart, based on how much space we have to work with.
      let spawn_tile_count = spawn_areas.iter().map(|area| area.length).sum();
      let spawn_tile_spacing = spawn_tile_count / player_count.get();

      // Choose a random tile offset for the first spawn location.
      let mut next_offset = RandomRange::gen_range(0, spawn_tile_count);

      let mut spawn_count = player_count.get();
      let mut spawn_locations = Vec::with_capacity(spawn_count);
      
      for SpawnArea { corner, direction, length } in spawn_areas.iter().cycle() {
         if next_offset > *length {
            // Skip over this SpawnArea.
            next_offset = next_offset.checked_sub(*length).unwrap();
            continue;
         }
         
         let offset : i16 = next_offset.try_into().expect("Offset too large");
         let spawn_location = corner.offset(*direction, offset);
         spawn_locations.push(spawn_location);
         
         spawn_count = spawn_count.checked_sub(1).expect("Ran out of Snakes to spawn");

         if spawn_count == 0 {
            // No more Snakes to spawn.
            break;
         }

         next_offset = next_offset.saturating_add(spawn_tile_spacing) % spawn_tile_count;
      }

      spawn_locations
   }

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

            if !snake.try_grow_backward() {
               // Unless we grew this turn, shrink the tail of the Snake after moving to keep it the same size.
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
            snake.lengthen(*points);
         }
      }

      swap(&mut self.snakes, temp_snakes);
   }

   fn is_game_over(&self) -> bool {
      let is_player_alive = self.snakes.iter()
         .find(|snake| snake.player_index == self.player_index && snake.alive)
         .is_some();
      
      let num_live_snakes = self.snakes.iter()
         .filter(|snake| snake.alive)
         .count();

      !is_player_alive || num_live_snakes <= 1
   }

   pub fn snakes(&self) -> impl Iterator<Item = &Snake> {
      self.snakes.iter()
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
            
            if ctx.host.borrow_game().is_game_over() {
               return;
            }
         }
      }
   }
}