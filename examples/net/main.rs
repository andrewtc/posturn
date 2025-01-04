mod snake;

use macroquad::prelude::*;
use miniquad::window::screen_size;
use snake::{draw_snake, Direction, Segment};

#[macroquad::main("Out West!")]
async fn main() {
   loop {
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

      draw_snake(
         (1,2),
         &vec![
            Segment(Direction::South, 3),
            Segment(Direction::East, 8),
            Segment(Direction::North, 2),
            Segment(Direction::West, 2)
         ],
         GREEN);
         
      draw_snake(
         (30,10),
         &vec![
            Segment(Direction::North, 5),
            Segment(Direction::West, 3),
            Segment(Direction::South, 4),
            Segment(Direction::East, 1)
         ],
         BLUE);

      next_frame().await;
   }
}