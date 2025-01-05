mod game;
mod snake;

use futures::pin_mut;
use game::Game;
use genawaiter::Generator;
use macroquad::prelude::*;
use miniquad::window::screen_size;
use snake::{draw_snake, Direction, Segment, Snake};

#[macroquad::main("Out West!")]
async fn main() {
   let snakes = vec![
      Snake {
         start: (1,2),
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
         start: (30,10),
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
         start: (15,25),
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
         start: (11,10),
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
         start: (40,30),
         facing: Direction::North,
         segments: vec![
            Segment(Direction::West, 8),
         ].into(),
         color: YELLOW,
      },
   ];

   let host = posturn::Host::new(Game {
      snakes,
   });

   let co = host.play().unwrap();
   pin_mut!(co);

   loop {
      if is_key_pressed(KeyCode::Space) {
         co.as_mut().resume();
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

      host.with_game(|game| {
         for snake in &game.snakes {
            draw_snake(snake);
         }
      });

      next_frame().await;
   }
}