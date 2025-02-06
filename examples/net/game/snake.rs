use std::{collections::{vec_deque, VecDeque}, iter::Enumerate, num::{NonZeroU16, NonZeroU8, TryFromIntError}, ops::RangeInclusive};
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
      Self::spawn_with_segments(player_index, params, None)
   }
   
   pub fn spawn_with_segments<I>(player_index : usize, params : SpawnParams, segments : I) -> Self where
      I : IntoIterator<Item = (Direction, u8)>,
   {
      let segments : VecDeque<Segment> = segments.into_iter()
         .map(|raw_parts| raw_parts.try_into().expect("Length cannot be zero"))
         .collect();

      // For ease of use, ensure that the length of the Snake always agrees with the target length.
      let len = Self::measure(segments.iter());
      let target_len = params.len.try_into().unwrap_or(len).max(len);

      Self {
         player_index,
         alive: params.alive,
         head_tile: params.head_tile_pos,
         segments: VecDeque::new(),
         prev_tail_dir: None,
         target_len,
         color: params.color,
      }
   }

   pub fn grow_forward(&mut self) {
      // Move the head forward by one tile in the facing direction.
      let facing = self.facing();
      self.head_tile = self.head_tile + facing;

      if let Some(head) = self.segments.front_mut() {
         // We have a front Segment, implying that the Snake is facing in the direction of the front Segment.
         head.len = head.len.saturating_add(1);
      }
      else {
         // The Snake is short enough that we need to add a Segment in order to move.
         self.segments.push_back(Segment::with_facing(facing));
      }
   }

   pub fn grow_cw(&mut self) {
      let cw = self.facing().cw();
      self.head_tile = self.head_tile + cw;
      self.segments.push_front(Segment::with_facing(cw));
   }

   pub fn grow_ccw(&mut self) {
      let ccw = self.facing().ccw();
      self.head_tile = self.head_tile + ccw;
      self.segments.push_front(Segment::with_facing(ccw));
   }

   pub fn shrink_head(mut self) -> Option<Self> {
      // If it had a head, now it doesn't.
      self.alive = false;

      let head_segment = self.segments.front()?;
      let new_len = head_segment.len.get().saturating_sub(1);

      let direction =
         if new_len == 0 {
            let segment = self.segments.pop_front().unwrap();
            segment.direction
         }
         else {
            let segment = self.segments.front_mut().unwrap();
            segment.len = new_len.try_into().unwrap();
            segment.direction
         };

      self.head_tile = self.head_tile + direction;
      Some(self)
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

   pub fn overlaps(&self, tile : I16Vec2) -> Overlaps<'_> {
      Overlaps { tile, inner: self.segments().enumerate() }
   }

   pub fn is_overlapping(&self, tile : I16Vec2) -> bool {
      self.overlaps(tile).next().is_some()
   }

   pub fn head_tile(&self) -> I16Vec2 {
      self.head_tile
   }

   pub fn segments(&self) -> Segments<'_> {
      Segments { last_segment_end_tile: self.head_tile, inner: self.segments.iter() }
   }

   pub fn num_segments(&self) -> usize {
      self.segments.len()
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

/// An iterator over the [`Segment`s](Segment) of a [`Snake`]. Also outputs the start and end location of the `Segment`
/// as a [`RangeInclusive`] of [`I16Vec2`].
#[derive(Debug)]
pub struct Segments<'iter> {
   last_segment_end_tile : I16Vec2,
   inner : vec_deque::Iter<'iter, Segment>,
}

impl<'iter> ExactSizeIterator for Segments<'iter> { }

impl<'iter> Iterator for Segments<'iter> {
   type Item = (RangeInclusive<I16Vec2>, &'iter Segment);

   fn size_hint(&self) -> (usize, Option<usize>) {
      self.inner.size_hint()
   }

   fn next(&mut self) -> Option<Self::Item> {
      self.inner.next().map(|segment| {
         let start = self.last_segment_end_tile + segment.direction;
         let end = self.last_segment_end_tile.offset(segment.direction, segment.len.get() as i16);
         self.last_segment_end_tile = end;
         (RangeInclusive::new(start, end), segment)
      })
   }
}

#[derive(Debug)]
pub struct Overlaps<'iter> {
   tile : I16Vec2,
   inner : Enumerate<Segments<'iter>>,
}

impl<'iter> Iterator for Overlaps<'iter> {
   type Item = (usize, &'iter Segment);

   fn size_hint(&self) -> (usize, Option<usize>) {
      self.inner.size_hint()
   }

   fn next(&mut self) -> Option<Self::Item> {
      loop {
         let (index, (tiles, segment)) = self.inner.next()?;
         let start = tiles.start();
         let end = tiles.end();

         if segment.direction.is_horizontal() && self.tile.y == start.y {
            let range_x = start.x.min(end.x) ..= start.x.max(end.x);
            if range_x.contains(&self.tile.x) {
               break Some((index, segment));
            }
         }
         else if segment.direction.is_vertical() && self.tile.x == start.x {
            let range_y = start.y.min(end.y) ..= start.y.max(end.y);
            if range_y.contains(&self.tile.y) {
               break Some((index, segment));
            }
         }
      }
   }
}

#[cfg(test)]
mod tests {
   use std::{num::NonZeroU8, ops::RangeInclusive};

   use macroquad::{color::Color, math::{i16vec2, I16Vec2}};

   use super::{Direction, Segment, Snake, SpawnParams};

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

   #[test]
   fn test_snake_spawn() {
      let beige = Color::from_rgba(245, 245, 220, 255);
      let params : SpawnParams = SpawnParams {
         alive: true,
         head_tile_pos: i16vec2(2, -3),
         color: beige,
         ..Default::default()
      };

      const PLAYER_INDEX : usize = 1;
      let segments = vec![
         (Direction::West, 3),
         (Direction::South, 2),
         (Direction::East, 4),
         (Direction::North, 1),
      ];

      let snake = Snake::spawn_with_segments(PLAYER_INDEX, params, segments);

      assert_eq!(snake.alive, true);
      assert_eq!(snake.head_tile(), i16vec2(2, -3));

      const EXPECTED_SEGMENTS : [(RangeInclusive<I16Vec2>, (Direction, u8)); 4] = [
         (i16vec2(1, -3)..=i16vec2(-1, -3), (Direction::West, 3)),
         (i16vec2(-1, -2)..=i16vec2(-1, -1), (Direction::South, 2)),
         (i16vec2(0, -1)..=i16vec2(3, -1), (Direction::East, 4)),
         (i16vec2(3, -2)..=i16vec2(3, -2), (Direction::North, 1)),
      ];
      
      for (index, ((tiles, segment), (expected_tiles, expected_segment))) in snake.segments().zip(EXPECTED_SEGMENTS.iter()).enumerate() {
         assert_eq!(tiles, *expected_tiles, "Tiles for Segment {index} were incorrect");
         assert_eq!(*segment, Segment::try_from(*expected_segment).unwrap(), "Segment {index} was incorrect");
      }

      assert_eq!(snake.color, beige);
   }
}