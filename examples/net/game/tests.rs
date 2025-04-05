use std::num::NonZeroUsize;

use macroquad::{prelude::*, rand::srand};

use super::{direction::Direction, Game, SpawnArea};

#[test]
fn test_spawn_areas()
{
   struct TestData {
      play_area_half_extents : U16Vec2,
      expected_spawn_areas : [SpawnArea; 4],
   }

   let mut i = 0;

   let mut assert_spawn_areas_match = |test : TestData| {
      i += 1;

      let spawn_areas = Game::generate_spawn_areas(test.play_area_half_extents);

      for (index, (expected, actual)) in test.expected_spawn_areas.iter().zip(spawn_areas.iter()).enumerate() {
         assert!(expected == actual, "At iteration {i}, spawn area at index {index} does NOT match.\nExpected: {expected:?}\n  Actual: {actual:?}");
      }
   };

   use Direction::*;

   // 2 2 2
   // 1 . 3
   // 0 0 0
   assert_spawn_areas_match(TestData {
      play_area_half_extents: u16vec2(0, 0),
      expected_spawn_areas: [
         SpawnArea { corner: i16vec2( 1, 1), direction:  West, length: 2 },
         SpawnArea { corner: i16vec2(-1, 0), direction: North, length: 0 },
         SpawnArea { corner: i16vec2(-1,-1), direction:  East, length: 2 },
         SpawnArea { corner: i16vec2( 1, 0), direction: South, length: 0 },
      ]});

   // 2 2 2 2 2
   // 1 . . . 3
   // 1 . . . 3
   // 1 . . . 3
   // 0 0 0 0 0
   assert_spawn_areas_match(TestData {
      play_area_half_extents: u16vec2(1, 1),
      expected_spawn_areas: [
         SpawnArea { corner: i16vec2( 2, 2), direction:  West, length: 4 },
         SpawnArea { corner: i16vec2(-2, 1), direction: North, length: 2 },
         SpawnArea { corner: i16vec2(-2,-2), direction:  East, length: 4 },
         SpawnArea { corner: i16vec2( 2,-1), direction: South, length: 2 },
      ]});

   // 2 2 2 2 2 2 2
   // 1 . . . . . 3
   // 1 . . . . . 3
   // 1 . . . . . 3
   // 0 0 0 0 0 0 0
   assert_spawn_areas_match(TestData {
      play_area_half_extents: u16vec2(2, 1),
      expected_spawn_areas: [
         SpawnArea { corner: i16vec2( 3, 2), direction:  West, length: 6 },
         SpawnArea { corner: i16vec2(-3, 1), direction: North, length: 2 },
         SpawnArea { corner: i16vec2(-3,-2), direction:  East, length: 6 },
         SpawnArea { corner: i16vec2( 3,-1), direction: South, length: 2 },
      ]});

   // 2 2 2 2 2
   // 1 . . . 3
   // 1 . . . 3
   // 1 . . . 3
   // 1 . . . 3
   // 1 . . . 3
   // 0 0 0 0 0
   assert_spawn_areas_match(TestData {
      play_area_half_extents: u16vec2(1, 2),
      expected_spawn_areas: [
         SpawnArea { corner: i16vec2( 2, 3), direction:  West, length: 4 },
         SpawnArea { corner: i16vec2(-2, 2), direction: North, length: 4 },
         SpawnArea { corner: i16vec2(-2,-3), direction:  East, length: 4 },
         SpawnArea { corner: i16vec2( 2,-2), direction: South, length: 4 },
      ]});
}

#[test]
fn test_spawn_locations() {
   struct TestData {
      random_seed : u64,
      spawn_areas : [SpawnArea; 4],
      expected_spawn_points : Vec<I16Vec2>,
   }

   let mut i = 0;

   let mut assert_spawn_locations_match = |test : TestData| {
      i += 1;

      // The number of expected spawn points is the same as the number of players in the game.
      let expected_player_count : NonZeroUsize = test.expected_spawn_points.len().try_into().unwrap();

      srand(test.random_seed);

      let actual_spawn_points = Game::choose_random_spawn_locations(&test.spawn_areas, expected_player_count);
      let actual_player_count : NonZeroUsize = actual_spawn_points.len().try_into().expect("Must be at least one spawn point");

      assert!(actual_player_count == expected_player_count, "At iteration {i}, number of spawn points does NOT match. Expected {expected_player_count}, but got {actual_player_count}.");

      for (index, (expected, actual)) in test.expected_spawn_points.iter().zip(actual_spawn_points.iter()).enumerate() {
         assert!(expected == actual, "At iteration {i}, spawn point at index {index} does NOT match.\nExpected: {expected:?}\n  Actual: {actual:?}");
      }
   };
   
   use Direction::*;

   assert_spawn_locations_match(TestData {
      random_seed: 12345,
      spawn_areas: [
         SpawnArea { corner: i16vec2( 2, 2), direction:  West, length: 4 },
         SpawnArea { corner: i16vec2(-2, 1), direction: North, length: 2 },
         SpawnArea { corner: i16vec2(-2,-2), direction:  East, length: 4 },
         SpawnArea { corner: i16vec2( 2,-1), direction: South, length: 2 },
      ],
      expected_spawn_points: vec![
         i16vec2(-1,  2),
         i16vec2(-1, -2),
         i16vec2( 2,  0),
      ]});

   // . . X .
   // .     .
   // .     .
   // . . . .
   assert_spawn_locations_match(TestData {
      random_seed: 54321,
      spawn_areas: [
         SpawnArea { corner: i16vec2( 2, 2), direction:  West, length: 4 },
         SpawnArea { corner: i16vec2(-2, 1), direction: North, length: 2 },
         SpawnArea { corner: i16vec2(-2,-2), direction:  East, length: 4 },
         SpawnArea { corner: i16vec2( 2,-1), direction: South, length: 2 },
      ],
      expected_spawn_points: vec![
         i16vec2( 0, -2),
         i16vec2( 2,  1),
         i16vec2(-2,  2),
      ]});
}