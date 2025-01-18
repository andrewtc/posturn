use macroquad::prelude::*;

use crate::game::snake::{Direction, Snake};

const TILE_SIZE : f32 = 24f32;

pub fn grid_to_window(tile_pos : I16Vec2) -> Vec2 {
   let screen_half_extents = 0.5 * vec2(screen_width(), screen_height());
   screen_half_extents + tile_pos.as_vec2() * TILE_SIZE
}

pub fn draw_head(pos : Vec2, direction : Direction, color : Color, alive : bool) {
   draw_circle(pos.x, pos.y, TILE_SIZE / 2 as f32, color);

   const EYE_RADIUS : f32 = TILE_SIZE / 4f32;
   const EYE_SPACING : f32 = EYE_RADIUS * 1.5f32;
   let eye_offset = match direction {
      Direction::West  => vec2(0f32, EYE_SPACING),
      Direction::East  => vec2(0f32, -EYE_SPACING),
      Direction::North => vec2(EYE_SPACING, 0f32),
      Direction::South => vec2(-EYE_SPACING, 0f32),
   };

   if alive {
      // Draw the eyes.
      const EYE_COLOR : Color = WHITE;
      draw_circle(pos.x + eye_offset.x, pos.y + eye_offset.y, EYE_RADIUS, EYE_COLOR);
      draw_circle(pos.x - eye_offset.x, pos.y - eye_offset.y, EYE_RADIUS, EYE_COLOR);

      const PUPIL_RADIUS : f32 = EYE_RADIUS / 2f32;
      const PUPIL_COLOR : Color = BLACK;
      draw_circle(pos.x + eye_offset.x, pos.y + eye_offset.y, PUPIL_RADIUS, PUPIL_COLOR);
      draw_circle(pos.x - eye_offset.x, pos.y - eye_offset.y, PUPIL_RADIUS, PUPIL_COLOR);
   }
}

pub fn interp(from : f32, to : f32, progress : f32) -> f32 {
   from + progress * (to - from)
}

pub fn interp_pos(start : Vec2, end : Vec2, progress : f32) -> Vec2 {
   vec2(interp(start.x, end.x, progress), interp(start.y, end.y, progress))
}

pub fn draw_segment(start : Vec2, end : Vec2, color : Color) {
   draw_line(start.x, start.y, end.x, end.y, TILE_SIZE as f32, color);
   draw_circle(end.x, end.y, TILE_SIZE / 2 as f32, color);
}

pub fn draw_snake(snake : &Snake, turn_progress : f32, color_override : Option<Color>) {
   // Draw the body.
   let mut head_pos = grid_to_window(snake.start());
   let num_segments = snake.num_segments();
   let color = color_override.unwrap_or(snake.color);

   for (index, (tiles, segment)) in snake.segments().enumerate() {
      let mut from = grid_to_window(*tiles.start());

      if index == 0 && snake.alive {
         let next_start = *tiles.start() + segment.direction;
         from = interp_pos(
            grid_to_window(next_start),
            from,
            turn_progress);

         head_pos = from;
      };

      let mut to = grid_to_window(*tiles.end());

      if index + 1 == num_segments && snake.alive {
         let next_end = *tiles.end() + segment.facing();
         to = interp_pos(
            to,
            grid_to_window(next_end),
            turn_progress);
      }
            
      draw_segment(from, to, color);
   }

   // Draw the head on top of the rest of the body.
   draw_head(head_pos, snake.facing(), color, snake.alive);
}