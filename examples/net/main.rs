mod draw;
mod game;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::pin_mut;
use game::{Game, direction::Direction, snake::SpawnParams};
use genawaiter::{Coroutine, GeneratorState};
use macroquad::{prelude::*, time};
use miniquad::window::{screen_size, set_window_size};

/// The state of the game window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameState {
   /// Show the paused overlay.
   Paused,

   /// Let players play the game.
   InProgress,

   /// Show the "Game Over!" overlay and allow players to restart the game.
   GameOver,
}

#[macroquad::main("Out West!")]
async fn main() {
   const WINDOW_WIDTH : u32 = 1024;
   const WINDOW_HEIGHT : u32 = 768;
   set_window_size(WINDOW_WIDTH, WINDOW_HEIGHT);

   const PLAY_AREA_HALF_EXTENTS : U16Vec2 = u16vec2(20, 15);

   let snakes_to_spawn = vec![
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: GREEN,
         ..Default::default()
      },
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: BLUE,
         ..Default::default()
      },
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: PURPLE,
         ..Default::default()
      },
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: RED,
         ..Default::default()
      },
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: YELLOW,
         ..Default::default()
      },
      SpawnParams {
         alive: true,
         amt_to_grow: 5,
         color: ORANGE,
         ..Default::default()
      },
   ];

   loop {
      let random_seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;
      println!("Random seed: {random_seed}");

      let setup = game::Setup {
         player_index: 3,
         play_area_half_extents: PLAY_AREA_HALF_EXTENTS,
         random_seed,
         snakes_to_spawn: snakes_to_spawn.clone(),
      };

      let host = posturn::Host::new(Game::with_setup(setup));

      let co = host.play().unwrap();
      pin_mut!(co);

      // We want to see Snakes already spawned when we start the game.
      co.as_mut().resume_with(None);

      // Start the game paused.
      let mut game_state = GameState::Paused;

      const TURN_DURATION : Duration = Duration::from_millis(100);
      let mut time_until_next_turn = Duration::ZERO;

      let mut desired_direction = None;

      'new_game: loop {
         const KEY_PAUSE : KeyCode = KeyCode::Space;
         if is_key_pressed(KEY_PAUSE) {
            game_state = match game_state {
               GameState::Paused => GameState::InProgress,
               GameState::InProgress => GameState::Paused,
               GameState::GameOver => {
                  // Don't allow restarting the game until the current turn animation has played out.
                  if time_until_next_turn == Duration::ZERO { break 'new_game; }
                  else { GameState::GameOver }
               },
            };
         }

         desired_direction = desired_direction.or(
            if is_key_pressed(KeyCode::Left) { Some(Direction::West) }
            else if is_key_pressed(KeyCode::Right) { Some(Direction::East) }
            else if is_key_pressed(KeyCode::Up) { Some(Direction::North) }
            else if is_key_pressed(KeyCode::Down) { Some(Direction::South) }
            else { None });

         if game_state != GameState::Paused {
            let time_elapsed = Duration::from_secs_f32(time::get_frame_time());
            let mut is_turn_over = false;

            time_until_next_turn = time_until_next_turn
               .checked_sub(time_elapsed)
               .unwrap_or_else(|| {
                  if game_state == GameState::GameOver {
                     // Let the current turn play out, but don't start a new turn.
                     Duration::ZERO
                  }
                  else {
                     // Start the next turn partially completed so that the animation is smooth.
                     is_turn_over = true;
                     TURN_DURATION - (time_elapsed - time_until_next_turn).min(TURN_DURATION)
                  }
               });

            if game_state != GameState::GameOver && is_turn_over {
               if let GeneratorState::Complete(_) = co.as_mut().resume_with(desired_direction.take()) {
                  // End the game and show the overlay.
                  game_state = GameState::GameOver;
               }
            }
         }

         const BG_COLOR : Color = Color::new(0.73, 0.4, 0.17, 1f32);
         clear_background(BG_COLOR);

         let turn_progress = 1f32 - time_until_next_turn.div_duration_f32(TURN_DURATION);
         host.with_game(|game| {
            for snake in game.snakes() {
               draw::draw_snake(snake, turn_progress, None);
            }
         });

         if game_state != GameState::InProgress {
            const PAUSED_TITLE_TEXT : &str = "Out West!";
            const GAME_OVER_TITLE_TEXT : &str = "Game Over!";
            let title_text_params = TextParams {
               font: None,
               font_size: 128,
               color: WHITE,
               ..Default::default()
            };
            
            const PAUSED_PROMPT_TEXT : &str = "Press SPACE to pause or resume the game.";
            const GAME_OVER_PROMPT_TEXT : &str = "Press SPACE to start a new game.";
            let prompt_text_params = TextParams {
               font_size: 32,
               ..title_text_params
            };

            let screen_center = 0.5f32 * Vec2::from(screen_size());
            let title_text = if game_state == GameState::GameOver { GAME_OVER_TITLE_TEXT } else { PAUSED_TITLE_TEXT };
            let title_text_center = get_text_center(title_text, None, title_text_params.font_size, title_text_params.font_scale, title_text_params.rotation);
            let title_text_pos = screen_center - title_text_center;
            draw_text_ex(title_text, title_text_pos.x, title_text_pos.y, title_text_params);

            let prompt_text = if game_state == GameState::GameOver { GAME_OVER_PROMPT_TEXT } else { PAUSED_PROMPT_TEXT };
            let prompt_text_center = get_text_center(prompt_text, None, prompt_text_params.font_size, prompt_text_params.font_scale, prompt_text_params.rotation);
            let prompt_text_pos = (screen_center + Vec2{ x: 0.0, y: 64.0 }) - prompt_text_center;
            draw_text_ex(prompt_text, prompt_text_pos.x, prompt_text_pos.y, prompt_text_params);
         }

         next_frame().await;
      }
   }
}