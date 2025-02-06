use std::num::NonZeroU16;

use macroquad::prelude::*;

use crate::game::{direction::Direction, snake::Snake};

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
   let mut last_segment_end_tile = snake.head_tile();
   let mut head_screen_pos = grid_to_window(last_segment_end_tile);

   if snake.alive {
      let previous_head_tile = last_segment_end_tile + snake.facing().opposite();
      head_screen_pos = interp_pos(
         grid_to_window(previous_head_tile),
         head_screen_pos,
         turn_progress);
   };

   let mut last_segment_end_pos = head_screen_pos;
   let color = color_override.unwrap_or(snake.color);

   for (tiles, _) in snake.segments() {
      let segment_end_pos = grid_to_window(*tiles.end());
      draw_segment(last_segment_end_pos, segment_end_pos, color);
      last_segment_end_tile = *tiles.end();
      last_segment_end_pos = grid_to_window(last_segment_end_tile);
   }

   if snake.alive && snake.len() != NonZeroU16::MIN {
      if let Some(tail_dir) = snake.prev_tail_dir() {
         let tail_start_pos = grid_to_window(last_segment_end_tile);
         let tail_end_tile = last_segment_end_tile + tail_dir;
         let tail_end_pos = interp_pos(
            grid_to_window(tail_end_tile),
            tail_start_pos,
            turn_progress);
            draw_segment(tail_start_pos, tail_end_pos, color);
      }
   }

   // Draw the head on top of the rest of the body.
   draw_head(head_screen_pos, snake.facing(), color, snake.alive);
}