use std::{collections::VecDeque, num::{NonZeroU16, NonZeroU8}, ops::Add};

use macroquad::prelude::*;

const TILE_SIZE : f32 = 24f32;

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

   pub const fn delta(&self) -> IVec2 {
      match self {
         Self::West => ivec2(-1, 0),
         Self::East => ivec2(1, 0),
         Self::North => ivec2(0, -1),
         Self::South => ivec2(0, 1),
      }
   }
}

#[derive(Debug, Clone, Copy)]
pub struct Segment {
   pub direction : Direction,
   pub len : NonZeroU8,
}

impl Segment {
   pub fn new(direction : Direction, len: u8) -> Self {
      Self { direction, len: len.try_into().expect("Length cannot be zero") }
   }

   pub fn with_facing(facing : Direction) -> Self {
      Self { direction: facing.opposite(), len: NonZeroU8::MIN }
   }

   pub fn facing(&self) -> Direction {
      self.direction.opposite()
   }
}

impl Add<Segment> for I16Vec2 {
   type Output = Self;
   fn add(self, segment: Segment) -> Self::Output {
      let I16Vec2 { x, y } = self;
      let offset = segment.len.get() as i16;
      match segment.direction {
         Direction::West  => i16vec2(x - offset, y),
         Direction::East  => i16vec2(x + offset, y),
         Direction::North => i16vec2(x, y - offset),
         Direction::South => i16vec2(x, y + offset),
      }
   }
}

impl Add<Direction> for I16Vec2 {
   type Output = Self;
   fn add(self, direction: Direction) -> Self::Output {
      self + Segment{ direction, len: NonZeroU8::MIN }
   }
}

#[derive(Debug)]
pub struct SpawnParams {
   pub alive : bool,
   pub start : I16Vec2,
   pub segments : VecDeque<Segment>,
   pub color : Color,
}

#[derive(Debug)]
pub struct Snake {
   alive : bool,
   start : I16Vec2,
   segments : VecDeque<Segment>,
   color : Color,
}

impl Snake {
   pub fn new(params : SpawnParams) -> Self {
      assert!(!params.segments.is_empty());
      Self {
         alive: params.alive,
         start: params.start,
         segments: params.segments,
         color: params.color,
      }
   }

   pub fn grow_forward(&mut self) {
      // Move the head forward by one tile.
      self.start = self.start + self.facing();
      let head = self.segments.front_mut().unwrap();
      head.len = head.len.saturating_add(1);
   }

   pub fn grow_cw(&mut self) {
      let cw = self.facing().cw();
      self.start = self.start + cw;
      self.segments.push_front(Segment::with_facing(cw));
   }

   pub fn grow_ccw(&mut self) {
      let ccw = self.facing().ccw();
      self.start = self.start + ccw;
      self.segments.push_front(Segment::with_facing(ccw));
   }

   pub fn shrink_tail(&mut self) {
      let len = self.segments.back().unwrap().len;
      let new_len = len.get().saturating_sub(1);

      if new_len > 0 {
         self.segments.back_mut().unwrap().len = new_len.try_into().unwrap();
      }
      else {
         self.segments.pop_back();
      }
   }

   pub fn is_alive(&self) -> bool {
      self.alive
   }

   pub fn start(&self) -> I16Vec2 {
      self.start
   }

   pub fn facing(&self) -> Direction {
      self.segments.front().unwrap().facing()
   }

   pub fn len(&self) -> NonZeroU16 {
      let len : u16 = self.segments.iter().map(|segment| segment.len.get() as u16).sum();
      len.try_into().unwrap()
   }
}

pub fn grid_to_window(tile_pos : I16Vec2) -> Vec2 {
   let screen_half_extents = 0.5 * vec2(screen_width(), screen_height());
   screen_half_extents + tile_pos.as_vec2() * TILE_SIZE
}

fn draw_head(pos : Vec2, direction : Direction, color : Color, alive : bool) {
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

fn interp(from : f32, to : f32, progress : f32) -> f32 {
   from + progress * (to - from)
}

fn interp_pos(start : Vec2, end : Vec2, progress : f32) -> Vec2 {
   vec2(interp(start.x, end.x, progress), interp(start.y, end.y, progress))
}

fn draw_segment(start : Vec2, end : Vec2, color : Color) {
   draw_line(start.x, start.y, end.x, end.y, TILE_SIZE as f32, color);
   draw_circle(end.x, end.y, TILE_SIZE / 2 as f32, color);
}

pub fn draw_snake(snake : &Snake, turn_progress : f32) {
   // Draw the body.
   let mut start = snake.start();
   let mut head_pos = grid_to_window(start);

   for (index, segment) in snake.segments.iter().enumerate() {
      let mut from = grid_to_window(start);
      
      if index == 0 && snake.alive {
         from = interp_pos(
            grid_to_window(snake.start + segment.direction),
            from,
            turn_progress);

         head_pos = from;
      };

      start = start + *segment;
      let mut to = grid_to_window(start);

      if index + 1 == snake.segments.len() && snake.alive {
         to = interp_pos(
            to,
            grid_to_window(start + segment.facing()),
            turn_progress);
      }
            
      draw_segment(from, to, snake.color);
   }

   // Draw the head on top of the rest of the body.
   draw_head(head_pos, snake.facing(), snake.color, snake.alive);
}