use macroquad::{prelude::*, ui::{hash, root_ui}};
use miniquad::window::screen_size;

#[macroquad::main("Out West!")]
async fn main() {
   let mut ip_addr_text = "localhost:8357".to_string();

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

      root_ui().group(hash!(), vec2(350.0, 80.0), |ui| {
         ui.label(None, "Join Game");
         ui.input_text(hash!(), "IP Address", &mut ip_addr_text);
         ui.button(None, "Join");
      });
      
      root_ui().group(hash!(), vec2(350.0, 80.0), |ui| {
         ui.label(None, "Host Game");
         ui.button(None, "Host");
      });

      next_frame().await;
   }
}