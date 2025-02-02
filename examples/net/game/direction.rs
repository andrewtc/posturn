use macroquad::prelude::*;

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

   pub const fn is_horizontal(&self) -> bool {
      self.delta().y == 0
   }

   pub const fn is_vertical(&self) -> bool {
      self.delta().x == 0
   }
}

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