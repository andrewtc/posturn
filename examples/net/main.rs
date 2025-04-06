mod draw;
mod game;
mod player;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::pin_mut;
use game::{Game, direction::Direction};
use genawaiter::{Coroutine, GeneratorState};
use macroquad::{prelude::*, rand::RandomRange, time};
use miniquad::window::{screen_size, set_window_size};
use player::{Control, KeyControls};

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

   const PLAYERS : &'static [player::Control] = &[
      player::Control::Manual(&KeyControls::ARROW_KEYS),
      player::Control::Manual(&KeyControls::WASD),
      player::Control::Auto,
      player::Control::Auto,
      player::Control::Auto,
      player::Control::Auto,
   ];

   loop {
      let random_seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;
      println!("Random seed: {random_seed}");

      let player_count = PLAYERS.len().try_into().expect("Must spawn at least one player");
      let setup = game::Setup {
         play_area_half_extents: PLAY_AREA_HALF_EXTENTS,
         random_seed,
         player_count,
         spawn_amt_to_grow: 5,
      };

      let host = posturn::Host::new(Game::with_setup(setup));

      let co = host.play().unwrap();
      pin_mut!(co);

      // We want to see Snakes already spawned when we start the game.
      co.as_mut().resume_with(vec![]);

      // Start the game paused.
      let mut game_state = GameState::Paused;

      const TURN_DURATION : Duration = Duration::from_millis(100);
      let mut time_until_next_turn = Duration::ZERO;

      let mut inputs : Vec<Option<Direction>> = Vec::with_capacity(player_count.get());

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

         inputs.resize(player_count.get(), None);

         for (player, input) in PLAYERS.iter().zip(inputs.iter_mut()) {
            *input = input.or(match player {
               Control::Manual(key_controls) => key_controls.desired_direction(),
               Control::Auto => {
                  // Turn randomly to simulate player input.
                  const CHANCE_TO_TURN : f32 = 0.02;
                  if f32::gen_range(0.0, 1.0) <= CHANCE_TO_TURN {
                     match u8::gen_range(0, 4) {
                        0 => Some(Direction::West),
                        1 => Some(Direction::East),
                        2 => Some(Direction::North),
                        3 => Some(Direction::South),
                        _ => unreachable!("Random roll should always match a cardinal direction"),
                     }
                  }
                  else { None }
               },
            })
         }

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
               if let GeneratorState::Complete(_) = co.as_mut().resume_with(inputs.drain(..).collect()) {
                  // End the game and show the overlay.
                  game_state = GameState::GameOver;
               }
            }
         }

         const BG_COLOR : Color = Color::new(0.73, 0.4, 0.17, 1f32);
         draw::draw_play_area(host.borrow_game().play_area_half_extents(), BG_COLOR);

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