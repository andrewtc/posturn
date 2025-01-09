use std::{collections::VecDeque, ops::Add};

use macroquad::prelude::*;

const TILE_SIZE : f32 = 24f32;

/// The state of a [Snake].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
   Moving,
   Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
   West,
   East,
   North,
   South
}

impl Direction {
   pub const fn opposite(&self) -> Self {
      match self {
         Self::West => Self::East,
         Self::East => Self::West,
         Self::North => Self::South,
         Self::South => Self::North,
      }
   }

   pub const fn cw(&self) -> Self {
      match self {
         Self::West => Self::North,
         Self::East => Self::South,
         Self::North => Self::East,
         Self::South => Self::West,
      }
   }

   pub const fn ccw(&self) -> Self {
      match self {
         Self::West => Self::South,
         Self::East => Self::North,
         Self::North => Self::West,
         Self::South => Self::East,
      }
   }

   pub const fn delta(&self) -> (i16, i16) {
      match self {
         Self::West => (-1, 0),
         Self::East => (1, 0),
         Self::North => (0, -1),
         Self::South => (0, 1),
      }
   }
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

impl Add<Direction> for (i16, i16) {
   type Output = Self;
   fn add(self, direction: Direction) -> Self::Output {
      self + Segment(direction, 1)
   }
}

#[derive(Debug)]
pub struct Snake {
   pub status : Status,
   pub start : (i16, i16),
   pub facing : Direction,
   pub segments : VecDeque<Segment>,
   pub color : Color,
}

impl Snake {
   pub fn step(&mut self) {
      self.start = self.start + self.facing;

      let needs_new_segment =
         if let Some(&mut Segment(direction, ref mut size)) = self.segments.front_mut() {
            if direction.opposite() == self.facing {
               *size += 1;
               false
            }
            else { true }
         }
         else { false };

      if needs_new_segment {
         self.segments.push_front(Segment(self.facing.opposite(), 1));
      }

      let last_segment_is_empty =
         if let Some(&mut Segment(_, ref mut size)) = self.segments.back_mut() {
            *size = size.saturating_sub(1);
            *size == 0
         }
         else { false };
      
      if last_segment_is_empty {
         self.segments.pop_back();
      }
   }
}

pub fn grid_to_window(tile_pos : (i16, i16)) -> (f32, f32) {
   let (screen_half_width, screen_half_height) = (0.5 * screen_width(), 0.5 * screen_height());
   (screen_half_width + tile_pos.0 as f32 * TILE_SIZE, screen_half_height + tile_pos.1 as f32 * TILE_SIZE)
}

fn draw_head(pos : (f32, f32), direction : Direction, color : Color, alive : bool) {
   let (center_x, center_y) = pos;
   draw_circle(center_x, center_y, TILE_SIZE / 2 as f32, color);

   const EYE_RADIUS : f32 = TILE_SIZE / 4f32;
   const EYE_SPACING : f32 = EYE_RADIUS * 1.5f32;
   let (eye_offset_x, eye_offset_y) = match direction {
      Direction::West  => (0f32, EYE_SPACING),
      Direction::East  => (0f32, -EYE_SPACING),
      Direction::North => (EYE_SPACING, 0f32),
      Direction::South => (-EYE_SPACING, 0f32),
   };

   if alive {
      // Draw the eyes.
      const EYE_COLOR : Color = WHITE;
      draw_circle(center_x + eye_offset_x, center_y + eye_offset_y, EYE_RADIUS, EYE_COLOR);
      draw_circle(center_x - eye_offset_x, center_y - eye_offset_y, EYE_RADIUS, EYE_COLOR);

      const PUPIL_RADIUS : f32 = EYE_RADIUS / 2f32;
      const PUPIL_COLOR : Color = BLACK;
      draw_circle(center_x + eye_offset_x, center_y + eye_offset_y, PUPIL_RADIUS, PUPIL_COLOR);
      draw_circle(center_x - eye_offset_x, center_y - eye_offset_y, PUPIL_RADIUS, PUPIL_COLOR);
   }
}

fn interp(from : f32, to : f32, progress : f32) -> f32 {
   from + progress * (to - from)
}

fn interp_pos(start : (f32, f32), end : (f32, f32), progress : f32) -> (f32, f32) {
   let (start_x, start_y) = start;
   let (end_x, end_y) = end;
   (interp(start_x, end_x, progress), interp(start_y, end_y, progress))
}

fn draw_segment(start : (f32, f32), end : (f32, f32), color : Color) {
   let (start_x, start_y) = start;
   let (end_x, end_y) = end;
   draw_line(start_x, start_y, end_x, end_y, TILE_SIZE as f32, color);
   draw_circle(end_x, end_y, TILE_SIZE / 2 as f32, color);
}

pub fn draw_snake(snake : &Snake, turn_progress : f32) {
   let (mut head_x, mut head_y) = grid_to_window(snake.start);
   
   if snake.status == Status::Moving {
      (head_x, head_y) = interp_pos(
         grid_to_window(snake.start + snake.facing.opposite()),
         (head_x, head_y),
         turn_progress);
   }
      
   // Draw the body.
   let (mut tile_x, mut tile_y) = snake.start;
   for (index, segment) in snake.segments.iter().enumerate() {
      let (from_x, from_y) = 
      if index == 0 {
         // Connect the first segment to the head
         (head_x, head_y)
      }
      else { grid_to_window((tile_x, tile_y)) };

      (tile_x, tile_y) = (tile_x, tile_y) + *segment;
      let (mut to_x, mut to_y) = grid_to_window((tile_x, tile_y));

      if index + 1 == snake.segments.len() && snake.status == Status::Moving {
         let direction = segment.0;
         (to_x, to_y) = interp_pos(
            (to_x, to_y),
            grid_to_window((tile_x, tile_y) + direction.opposite()),
            turn_progress);
      }
            
      draw_segment((from_x, from_y), (to_x, to_y), snake.color);
   }

   // Draw the head on top of the rest of the body.
   let alive = snake.status != Status::Dead;
   draw_head((head_x, head_y), snake.facing, snake.color, alive);
}