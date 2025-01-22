mod draw;
mod game;

use std::time::Duration;

use futures::pin_mut;
use game::{Game, snake::{Direction, Segment, Snake, SpawnParams}};
use genawaiter::Coroutine;
use macroquad::{prelude::*, time};
use miniquad::window::{screen_size, set_window_size};

#[macroquad::main("Out West!")]
async fn main() {
   const WINDOW_WIDTH : u32 = 1024;
   const WINDOW_HEIGHT : u32 = 768;
   set_window_size(WINDOW_WIDTH, WINDOW_HEIGHT);

   const PLAY_AREA_HALF_EXTENTS : U16Vec2 = u16vec2(20, 15);
   const RANDOM_SEED : u64 = 12345;

   let snakes = [
      SpawnParams {
         alive: true,
         start: i16vec2(-19, -13),
         segments: vec![
            Segment::new(Direction::South, 3),
            Segment::new(Direction::East,  8),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::West,  2),
         ].into(),
         color: GREEN,
      },
      SpawnParams {
         alive: true,
         start: i16vec2(10, -5),
         segments: vec![
            Segment::new(Direction::North, 5),
            Segment::new(Direction::West,  3),
            Segment::new(Direction::South, 4),
            Segment::new(Direction::East,  1),
         ].into(),
         color: BLUE,
      },
      SpawnParams {
         alive: true,
         start: i16vec2(5, 10),
         segments: vec![
            Segment::new(Direction::South, 2),
            Segment::new(Direction::East,  2),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::East,  2),
            Segment::new(Direction::South, 2),
            Segment::new(Direction::East,  2),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::East,  2),
         ].into(),
         color: PURPLE,
      },
      SpawnParams {
         alive: true,
         start: i16vec2(-9, -5),
         segments: vec![
            Segment::new(Direction::North, 2),
            Segment::new(Direction::East,  2),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::West,  2),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::East,  2),
            Segment::new(Direction::North, 2),
            Segment::new(Direction::West,  2),
         ].into(),
         color: RED,
      },
      SpawnParams {
         alive: true,
         start: i16vec2(20, 15),
         segments: vec![
            Segment::new(Direction::West, 8),
         ].into(),
         color: YELLOW,
      },
      SpawnParams {
         alive: true,
         start: i16vec2(-3, -3),
         segments: vec![
            Segment::new(Direction::West, 1),
         ].into(),
         color: ORANGE,
      },
   ]
   .into_iter()
   .enumerate()
   .map(|(player_index, params)| Snake::spawn(player_index, params))
   .collect();

   let host = posturn::Host::new(Game {
      player_index: 3,
      play_area_half_extents: PLAY_AREA_HALF_EXTENTS,
      random_seed: RANDOM_SEED,
      snakes,
   });

   let co = host.play().unwrap();
   pin_mut!(co);

   co.as_mut().resume_with(None);

   let mut paused = true;

   const TURN_DURATION : Duration = Duration::from_millis(75);
   let mut turn_time_elapsed = Duration::ZERO;

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

      let turn_progress = turn_time_elapsed.div_duration_f32(TURN_DURATION);
      host.with_game(|game| {
         for snake in game.snakes.iter() {
            draw::draw_snake(snake, turn_progress, None);
         }
      });

      if paused {
         const TITLE_TEXT : &str = "Out West!";
         let title_text_params = TextParams {
            font: None,
            font_size: 128,
            color: WHITE,
            ..Default::default()
         };
         
         const PAUSED_TEXT : &str = "Press SPACE to pause or resume the game.";
         let paused_text_params = TextParams {
            font_size: 32,
            ..title_text_params
         };

         let screen_center = 0.5f32 * Vec2::from(screen_size());
         let title_text_center = get_text_center(TITLE_TEXT, None, title_text_params.font_size, title_text_params.font_scale, title_text_params.rotation);
         let title_text_pos = screen_center - title_text_center;
         draw_text_ex(TITLE_TEXT, title_text_pos.x, title_text_pos.y, title_text_params);

         let paused_text_center = get_text_center(PAUSED_TEXT, None, paused_text_params.font_size, paused_text_params.font_scale, paused_text_params.rotation);
         let paused_text_pos = (screen_center + Vec2{ x: 0.0, y: 64.0 }) - paused_text_center;
         draw_text_ex(PAUSED_TEXT, paused_text_pos.x, paused_text_pos.y, paused_text_params);
      }

      next_frame().await;
   }
}