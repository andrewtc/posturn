use std::ops::Add;

use macroquad::prelude::*;

const TILE_SIZE : f32 = 16f32;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
   West,
   East,
   North,
   South
}

#[derive(Debug, Clone, Copy)]
pub struct Segment(pub Direction, pub u8);

impl Add<Segment> for (i16, i16) {
   type Output = Self;
   fn add(self, segment: Segment) -> Self::Output {
      let (tile_x, tile_y) = self;
      let Segment(direction, tile_size) = segment;
      match direction {
         Direction::West  => (tile_x - tile_size as i16, tile_y),
         Direction::East  => (tile_x + tile_size as i16, tile_y),
         Direction::North => (tile_x, tile_y - tile_size as i16),
         Direction::South => (tile_x, tile_y + tile_size as i16),
      }
   }
}

pub fn grid_to_window(tile_pos : (i16, i16)) -> (f32, f32) {
   ((tile_pos.0 as f32 + 0.5) * TILE_SIZE, (tile_pos.1 as f32 + 0.5) * TILE_SIZE)
}

fn draw_circle_at_tile(tile_pos : (i16, i16), color : Color) {
   let (draw_x, draw_y) = grid_to_window(tile_pos);
   draw_circle(draw_x, draw_y, TILE_SIZE / 2 as f32, color);
}

fn draw_segment(start : (i16, i16), segment : Segment, color : Color) {
   let (start_x, start_y) = grid_to_window(start);
   let (end_x, end_y) = grid_to_window(start + segment);
   draw_line(start_x, start_y, end_x, end_y, TILE_SIZE as f32, color);
}

pub fn draw_snake(start : (i16, i16), segments : &Vec<Segment>, color : Color) {
   let (mut tile_x, mut tile_y) = start;

   // Draw the head.
   draw_circle_at_tile((tile_x, tile_y), color);

   // Draw the body.
   for segment in segments {
      draw_segment((tile_x, tile_y), *segment, color);

      (tile_x, tile_y) = (tile_x, tile_y) + *segment;
      draw_circle_at_tile((tile_x, tile_y), color);
   }
}