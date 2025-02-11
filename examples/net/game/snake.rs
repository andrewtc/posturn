use std::{collections::{vec_deque, VecDeque}, num::{NonZeroU16, NonZeroU8, TryFromIntError}};
use macroquad::prelude::*;

use super::direction::{Direction, Offset};

/// A straight section of a [`Snake`], having a fixed length and facing a given [`Direction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Segment {
   pub direction : Direction,
   pub len : NonZeroU8,
}

impl Segment {
   pub const fn with_facing(facing : Direction) -> Self {
      Self { direction: facing.opposite(), len: NonZeroU8::MIN }
   }

   pub fn facing(&self) -> Direction {
      self.direction.opposite()
   }

   pub fn endpoints(&self, corner : I16Vec2) -> (I16Vec2, I16Vec2) {
      // Length is always measured from a CORNER, i.e. the endpoint of a previous Segment.
      let start = corner + self.direction;
      let end = corner.offset(self.direction, self.len.get() as i16);
      (start, end)
   }
}

impl TryFrom<(Direction, u8)> for Segment {
   type Error = TryFromIntError;
   fn try_from(value: (Direction, u8)) -> Result<Self, Self::Error> {
      let (direction, raw_len) = value;
      Ok(Self { direction, len: raw_len.try_into()? })
   }
}

#[derive(Debug, Default)]
pub struct SpawnParams {
   pub alive : bool,
   pub head_tile_pos : I16Vec2,
   pub len : u16,
   pub color : Color,
}

#[derive(Clone, Debug)]
pub struct Snake {
   pub player_index : usize,
   pub alive : bool,
   head_tile : I16Vec2,
   segments : VecDeque<Segment>,
   target_len : NonZeroU16,
   prev_tail_dir : Option<Direction>,
   pub color : Color,
}

impl Snake {
   pub fn spawn(player_index : usize, params : SpawnParams) -> Self {
      Self::try_spawn_with_segments(player_index, params, None).expect("Expected no Segment overlap")
   }
   
   pub fn try_spawn_with_segments<I>(player_index : usize, params : SpawnParams, segments : I) -> Result<Self, Overlap> where
      I : IntoIterator<Item = (Direction, u8)>,
   {
      let segments : VecDeque<Segment> = segments.into_iter()
         .map(|raw_parts| raw_parts.try_into().expect("Length cannot be zero"))
         .collect();

      // For ease of use, ensure that the length of the Snake always agrees with the target length.
      let len = Self::measure(segments.iter());
      let target_len = params.len.try_into().unwrap_or(len).max(len);

      let snake = Self {
         player_index,
         alive: params.alive,
         head_tile: params.head_tile_pos,
         segments,
         prev_tail_dir: None,
         target_len,
         color: params.color,
      };

      if let Some(overlap) = snake.find_body_overlap(snake.head_tile) {
         return Err(overlap);
      }

      Ok(snake)
   }

   pub fn grow_forward(&mut self) -> Result<(), Overlap> {
      // Move the head forward by one tile in the facing direction.
      let facing = self.facing();
      let next_head_tile = self.head_tile + facing;

      if let Some(overlap) = self.find_body_overlap(next_head_tile) {
         return Err(overlap);
      }

      self.head_tile = next_head_tile;

      if let Some(head) = self.segments.front_mut() {
         // We have a front Segment, implying that the Snake is facing in the direction of the front Segment.
         head.len = head.len.saturating_add(1);
      }
      else {
         // The Snake is short enough that we need to add a Segment in order to move.
         self.segments.push_back(Segment::with_facing(facing));
      }

      Ok(())
   }

   pub fn grow_cw(&mut self) -> Result<(), Overlap> {
      let cw = self.facing().cw();
      let next_head_tile = self.head_tile + cw;

      if let Some(overlap) = self.find_body_overlap(next_head_tile) {
         return Err(overlap);
      }

      self.head_tile = next_head_tile;
      self.segments.push_front(Segment::with_facing(cw));

      Ok(())
   }

   pub fn grow_ccw(&mut self) -> Result<(), Overlap> {
      let ccw = self.facing().ccw();
      let next_head_tile = self.head_tile + ccw;

      if let Some(overlap) = self.find_body_overlap(next_head_tile) {
         return Err(overlap);
      }

      self.head_tile = next_head_tile;
      self.segments.push_front(Segment::with_facing(ccw));

      Ok(())
   }

   pub fn shrink_tail(mut self) -> Option<Self> {
      let last_segment = self.segments.back()?;
      let new_segment_len = last_segment.len.get().saturating_sub(1);

      if new_segment_len > 0 {
         // Shorten the existing Segment.
         let last_segment_mut = self.segments.back_mut().unwrap();
         last_segment_mut.len = new_segment_len.try_into().unwrap();
         self.prev_tail_dir = Some(last_segment_mut.direction);
      }
      else {
         // We're at a corner, so the Segment needs to completely disappear.
         let last_segment = self.segments.pop_back().unwrap();
         self.prev_tail_dir = Some(last_segment.direction);
      }

      Some(self)
   }

   pub fn can_decap(&self, other : &Snake) -> bool {
      // Both Snakes must be alive and this Snake must be longer.
      self.alive && other.alive && other.len() <= self.len()
   }

   pub fn find_body_overlap(&self, tile : I16Vec2) -> Option<Overlap> {
      for (segment_index, (corner, segment)) in self.segments().enumerate() {
         let segment_delta = segment.direction.delta();
         let corner_to_tile = tile - corner;

         let offset : Option<u8> =
            if corner_to_tile.x == 0 && segment_delta.x == 0 { (corner_to_tile.y * segment_delta.y).try_into().ok() }
            else if corner_to_tile.y == 0 && segment_delta.y == 0 { (corner_to_tile.x * segment_delta.x).try_into().ok() }
            else { None };

         if let Some(offset) = offset {
            if offset > 0 && offset <= segment.len.get() {
               let overlap = Overlap { segment_index, offset };
               return Some(overlap.into());
            }
         }
         else {
            // Tile does not fall along direction vector of Segment. No overlap.
            continue;
         };
      }

      None
   }

   pub fn head_tile(&self) -> I16Vec2 {
      self.head_tile
   }

   pub fn segments(&self) -> Segments<'_> {
      Segments { last_segment_end_tile: self.head_tile, inner: self.segments.iter() }
   }

   pub fn prev_tail_dir(&self) -> Option<Direction> {
      self.prev_tail_dir
   }

   pub fn facing(&self) -> Direction {
      self.segments.front()
         .map(|segment| segment.facing())
         .or_else(|| self.prev_tail_dir.map(|dir| dir.opposite()))
         .unwrap_or_default()
   }

   pub fn target_len(&self) -> NonZeroU16 {
      self.target_len
   }

   pub fn measure<'i, I>(segments : I) -> NonZeroU16 where
      I : IntoIterator<Item = &'i Segment>
   {
      // The length of the Snake is the length of its Segments...
      let len_segments = segments
         .into_iter()
         .map(|segment| segment.len.get() as u16)
         .sum();

      // ...plus one for the head.
      NonZeroU16::MIN.saturating_add(len_segments)
   }

   pub fn len(&self) -> NonZeroU16 {
      Self::measure(self.segments.iter())
   }
}

/// An iterator over the [`Segment`s](Segment) of a [`Snake`]. Also outputs an [`I16Vec2`] representing the **corner**
/// to which the [`Segment`] is attached, i.e the end tile of the previous [`Segment`].
#[derive(Debug)]
pub struct Segments<'iter> {
   last_segment_end_tile : I16Vec2,
   inner : vec_deque::Iter<'iter, Segment>,
}

impl<'iter> ExactSizeIterator for Segments<'iter> { }

impl<'iter> Iterator for Segments<'iter> {
   type Item = (I16Vec2, &'iter Segment);

   fn size_hint(&self) -> (usize, Option<usize>) {
      self.inner.size_hint()
   }

   fn next(&mut self) -> Option<Self::Item> {
      self.inner.next().map(|segment| {
         let corner = self.last_segment_end_tile;
         self.last_segment_end_tile = corner.offset(segment.direction, segment.len.get().into());
         (corner, segment)
      })
   }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Overlap {
   /// The index of the [`Segment`] that overlaps with the tile.
   pub segment_index : usize,

   /// Where the tile overlaps, measured in whole tiles from the **corner** to which the [`Segment`] is attached, i.e.
   /// the end tile of the previous [`Segment`].
   pub offset : u8,
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_segment_from_tuple() {
      let segment : Segment = (Direction::East, 8).try_into().expect("Failed to create Segment");
      assert_eq!(segment, Segment { direction: Direction::East, len: NonZeroU8::new(8).unwrap() });
   }
   
   #[test]
   fn test_segment_from_tuple_zero_length() {
      Segment::try_from((Direction::East, 0)).expect_err("Length must be non-zero");
   }

   #[test]
   fn test_segment_with_facing() {
      let segment = Segment::with_facing(Direction::North);
      assert_eq!(segment, Segment { direction: Direction::South, len: NonZeroU8::MIN });
   }

   #[test]
   fn test_segment_facing() {
      let segment = Segment::with_facing(Direction::North);
      assert_eq!(segment.facing(), Direction::North);
   }

   const SOME_PLAYER_INDEX : usize = 1;
   fn make_spawn_params() -> SpawnParams {
      SpawnParams {
         alive: true,
         head_tile_pos: i16vec2(2, -3),
         color: BEIGE,
         ..Default::default()
      }
   }

   #[test]
   fn test_snake_spawn() {
      let params = make_spawn_params();

      let segments = vec![
         (Direction::West, 3),
         (Direction::South, 2),
         (Direction::East, 4),
         (Direction::North, 1),
      ];

      let snake = Snake::try_spawn_with_segments(SOME_PLAYER_INDEX, params, segments)
         .expect("Expected Snake to spawn successfully");

      assert_eq!(snake.alive, true);
      assert_eq!(snake.head_tile(), i16vec2(2, -3));

      const EXPECTED_SEGMENTS : [(I16Vec2, (Direction, u8)); 4] = [
         (i16vec2(2, -3), (Direction::West, 3)),
         (i16vec2(-1, -3), (Direction::South, 2)),
         (i16vec2(-1, -1), (Direction::East, 4)),
         (i16vec2(3, -1), (Direction::North, 1)),
      ];
      
      for (index, ((tiles, segment), (expected_tiles, expected_segment))) in snake.segments().zip(EXPECTED_SEGMENTS.iter()).enumerate() {
         assert_eq!(tiles, *expected_tiles, "Tiles for Segment {index} were incorrect");
         assert_eq!(*segment, Segment::try_from(*expected_segment).unwrap(), "Segment {index} was incorrect");
      }

      assert_eq!(snake.color, BEIGE);
   }

   #[test]
   fn test_invalid_spawn() {
      let params = make_spawn_params();

      let segments = vec![
         (Direction::North, 2),
         (Direction::West,  2),
         (Direction::South, 2),
         (Direction::East,  2), // Overlaps with head!
      ];

      let overlap = Snake::try_spawn_with_segments(SOME_PLAYER_INDEX, params, segments)
         .expect_err("Expected error when trying to spawn Snake");

      assert_eq!(overlap, Overlap { segment_index: 3, offset: 2 });
   }
}