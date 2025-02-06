use std::ops::Add;

use macroquad::prelude::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
   #[default]
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

   pub const fn delta(&self) -> I16Vec2 {
      match self {
         Self::West => i16vec2(-1, 0),
         Self::East => i16vec2(1, 0),
         Self::North => i16vec2(0, -1),
         Self::South => i16vec2(0, 1),
      }
   }

   pub const fn is_horizontal(&self) -> bool {
      self.delta().y == 0
   }

   pub const fn is_vertical(&self) -> bool {
      self.delta().x == 0
   }
}

impl Add<Direction> for I16Vec2 {
   type Output = Self;
   fn add(self, direction: Direction) -> Self::Output {
      self.offset(direction, 1)
   }
}

/// Trait allowing a type to be offset by a certain distance in a given [`Direction`].
pub trait Offset {
   type Output : Sized;
   type Offset : Sized;
   fn offset(self, dir : Direction, offset : Self::Offset) -> Self::Output;
}

impl Offset for I16Vec2 {
   type Output = Self;
   type Offset = i16;
   fn offset(self, dir : Direction, offset : i16) -> Self::Output {
      let Self { x, y } = self;
      match dir {
         Direction::West  => i16vec2(x - offset, y),
         Direction::East  => i16vec2(x + offset, y),
         Direction::North => i16vec2(x, y - offset),
         Direction::South => i16vec2(x, y + offset),
      }
   }
}

#[cfg(test)]
mod tests {
   use macroquad::prelude::*;

   use super::{Direction::*, Offset};

   #[test]
   fn test_direction_opposite() {
      assert_eq!(West.opposite(), East);
      assert_eq!(East.opposite(), West);
      assert_eq!(North.opposite(), South);
      assert_eq!(South.opposite(), North);
   }

   #[test]
   fn test_direction_cw() {
      assert_eq!(West.cw(), North);
      assert_eq!(East.cw(), South);
      assert_eq!(North.cw(), East);
      assert_eq!(South.cw(), West);
   }

   #[test]
   fn test_direction_ccw() {
      assert_eq!(West.ccw(), South);
      assert_eq!(East.ccw(), North);
      assert_eq!(North.ccw(), West);
      assert_eq!(South.ccw(), East);
   }

   #[test]
   fn test_direction_delta() {
      assert_eq!(West.delta(), i16vec2(-1, 0));
      assert_eq!(East.delta(), i16vec2(1, 0));
      assert_eq!(North.delta(), i16vec2(0, -1));
      assert_eq!(South.delta(), i16vec2(0, 1));
   }

   #[test]
   fn test_direction_is_horizontal() {
      assert_eq!(West.is_horizontal(), true);
      assert_eq!(East.is_horizontal(), true);
      assert_eq!(North.is_horizontal(), false);
      assert_eq!(South.is_horizontal(), false);
   }

   #[test]
   fn test_direction_is_vertical() {
      assert_eq!(West.is_vertical(), false);
      assert_eq!(East.is_vertical(), false);
      assert_eq!(North.is_vertical(), true);
      assert_eq!(South.is_vertical(), true);
   }

   #[test]
   fn test_add_direction() {
      const START : I16Vec2 = i16vec2(-1, 1);
      assert_eq!(START + West, i16vec2(-2, 1));
      assert_eq!(START + East, i16vec2(0, 1));
      assert_eq!(START + North, i16vec2(-1, 0));
      assert_eq!(START + South, i16vec2(-1, 2));
   }

   #[test]
   fn test_offset() {
      const START : I16Vec2 = i16vec2(-1, 1);
      assert_eq!(START.offset(West, 1), i16vec2(-2, 1));
      assert_eq!(START.offset(East, 1), i16vec2(0, 1));
      assert_eq!(START.offset(North, 1), i16vec2(-1, 0));
      assert_eq!(START.offset(South, 1), i16vec2(-1, 2));
   }
}