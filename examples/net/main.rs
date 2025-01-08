mod game;
mod snake;

use std::time::Duration;

use futures::pin_mut;
use game::Game;
use genawaiter::Coroutine;
use macroquad::{prelude::*, time};
use miniquad::window::screen_size;
use snake::{draw_snake, Direction, Segment, Snake};

#[macroquad::main("Out West!")]
async fn main() {
   const PLAY_AREA_HALF_EXTENTS : (u8, u8) = (20, 15);
   const RANDOM_SEED : u64 = 12345;

   let snakes = vec![
      Snake {
         start: (-19,-13),
         facing: Direction::East,
         segments: vec![
            Segment(Direction::South, 3),
            Segment(Direction::East, 8),
            Segment(Direction::North, 2),
            Segment(Direction::West, 2),
         ].into(),
         color: GREEN,
      },
      Snake {
         start: (10,-5),
         facing: Direction::East,
         segments: vec![
            Segment(Direction::North, 5),
            Segment(Direction::West, 3),
            Segment(Direction::South, 4),
            Segment(Direction::East, 1),
         ].into(),
         color: BLUE,
      },
      Snake {
         start: (5,10),
         facing: Direction::North,
         segments: vec![
            Segment(Direction::South, 2),
            Segment(Direction::East, 2),
            Segment(Direction::North, 2),
            Segment(Direction::East, 2),
            Segment(Direction::South, 2),
            Segment(Direction::East, 2),
            Segment(Direction::North, 2),
            Segment(Direction::East, 2),
         ].into(),
         color: PURPLE,
      },
      Snake {
         start: (-9,-5),
         facing: Direction::West,
         segments: vec![
            Segment(Direction::North, 2),
            Segment(Direction::East, 2),
            Segment(Direction::North, 2),
            Segment(Direction::West, 2),
            Segment(Direction::North, 2),
            Segment(Direction::East, 2),
            Segment(Direction::North, 2),
            Segment(Direction::West, 2),
         ].into(),
         color: RED,
      },
      Snake {
         start: (20,15),
         facing: Direction::North,
         segments: vec![
            Segment(Direction::West, 8),
         ].into(),
         color: YELLOW,
      },
      Snake {
         start: (-3,-3),
         facing: Direction::North,
         segments: vec![
            Segment(Direction::West, 2),
         ].into(),
         color: ORANGE,
      },
   ];

   let host = posturn::Host::new(Game {
      play_area_half_extents: PLAY_AREA_HALF_EXTENTS,
      random_seed: RANDOM_SEED,
      snakes,
      player_index: 3,
   });

   let co = host.play().unwrap();
   pin_mut!(co);

   co.as_mut().resume_with(None);

   let mut paused = true;

   const TURN_DURATION : Duration = Duration::from_millis(75);
   let mut turn_time_elapsed = TURN_DURATION;

   loop {
      const KEY_PAUSE : KeyCode = KeyCode::Space;
      if is_key_pressed(KEY_PAUSE) {
         paused = !paused;
      }

      let input = 
         if is_key_down(KeyCode::Left) { Some(Direction::West) }
         else if is_key_down(KeyCode::Right) { Some(Direction::East) }
         else if is_key_down(KeyCode::Up) { Some(Direction::North) }
         else if is_key_down(KeyCode::Down) { Some(Direction::South) }
         else { None };

      if !paused {
         turn_time_elapsed += Duration::from_secs_f32(time::get_frame_time());
         if turn_time_elapsed >= TURN_DURATION {
            turn_time_elapsed -= TURN_DURATION;
            co.as_mut().resume_with(input);
         }
      }

      const BG_COLOR : Color = Color::new(0.73, 0.4, 0.17, 1f32);
      clear_background(BG_COLOR);

      const TITLE_TEXT : &str = "Out West!";
      let title_text_params = TextParams {
         font: None,
         font_size: 128,
         color: WHITE,
         ..Default::default()
      };

      let title_text_center = get_text_center(TITLE_TEXT, None, title_text_params.font_size, title_text_params.font_scale, title_text_params.rotation);
      let title_text_pos = (0.5f32 * Vec2::from(screen_size())) - title_text_center;
      draw_text_ex(TITLE_TEXT, title_text_pos.x, title_text_pos.y, title_text_params);

      let turn_progress = turn_time_elapsed.div_duration_f32(TURN_DURATION);
      host.with_game(|game| {
         for snake in &game.snakes {
            draw_snake(snake, turn_progress);
         }
      });

      next_frame().await;
   }
}