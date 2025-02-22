#[cfg(test)]
mod tests;

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

   /// Splits the [`Segment`] into two at the given offset.
   pub fn split_at(self, at : NonZeroU8) -> (Option<Segment>, Option<Segment>) {
      assert!(at <= self.len, "Offset {at} is out of bounds of Segment (length: {})", self.len);
      let front = (self.direction, at.get().saturating_sub(1)).try_into().ok();
      let back = (self.direction, self.len.get().saturating_sub(at.get())).try_into().ok();
      (front, back)
   }
}

impl TryFrom<(Direction, u8)> for Segment {
   type Error = TryFromIntError;
   fn try_from(value: (Direction, u8)) -> Result<Self, Self::Error> {
      let (direction, raw_len) = value;
      Ok(Self { direction, len: raw_len.try_into()? })
   }
}

#[derive(Clone, Debug, Default)]
pub struct SpawnParams {
   pub alive : bool,
   pub head_tile : I16Vec2,
   pub amt_to_grow : u16,
   pub color : Color,
   pub segments : Vec<(Direction, u8)>,
   pub prev_tail_dir : Option<Direction>,
}

#[derive(Clone, Debug)]
pub struct Snake {
   pub player_index : usize,
   pub alive : bool,
   head_tile : I16Vec2,
   segments : VecDeque<Segment>,
   pub amt_to_grow : u16,
   prev_tail_dir : Option<Direction>,
   pub color : Color,
}

impl Snake {
   pub fn try_spawn(player_index : usize, params : SpawnParams) -> Result<Snake, Overlap> {
      let segments : VecDeque<Segment> = params.segments.into_iter()
         .map(|raw_parts| raw_parts.try_into().expect("Length cannot be zero"))
         .collect();

      let snake = Self {
         player_index,
         alive: params.alive,
         head_tile: params.head_tile,
         segments,
         prev_tail_dir: params.prev_tail_dir,
         amt_to_grow: params.amt_to_grow,
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

   pub fn shrink_head(mut self) -> Option<Self> {
      // If it had a head, now it doesn't.
      self.alive = false;

      let head_segment = self.segments.pop_front()?;
      let (front, back) = head_segment.split_at(NonZeroU8::MIN);
      assert!(front.is_none(), "Expected to remove only head of Snake");

      if let Some(segment) = back {
         self.segments.push_front(segment);
      }

      self.head_tile = self.head_tile + head_segment.direction;
      Some(self)
   }

   pub fn shrink_tail(mut self) -> Option<Self> {
      let last_segment = self.segments.pop_back()?;
      let (front, back) = last_segment.split_at(last_segment.len);
      assert!(back.is_none(), "Expected to remove only last tile");
      self.prev_tail_dir = Some(last_segment.direction);
      self.segments.extend(front);
      Some(self)
   }

   /// Splits the [`Snake`] at the specified [`Segment`] and offset and returns the tail as a new [`Snake`].
   pub fn split_off(&mut self, segment_index : usize, offset : NonZeroU8) -> Snake {
      let (back_corner, back_segment) = self.segments()
         .nth(segment_index)
         .expect(&format!("Segment index {segment_index} is out of bounds (count: {})", self.segments.len()));

      let back_head_tile = back_corner.offset(back_segment.direction, offset.get() as i16);

      // Cut off the tail of the Snake, keeping track of the middle Segment that we need to split.
      let mut back_segments = self.segments.split_off(segment_index);
      let segment_to_split = back_segments.pop_front().expect("Expected a Segment to split");

      // Split the middle Segment at the offset and divvy it up between the two Snakes.
      let (segment_front, segment_back) = segment_to_split.split_at(offset);
      if let Some(segment) = segment_front { self.segments.push_back(segment); }
      if let Some(segment) = segment_back { back_segments.push_front(segment); }

      let back_snake = Snake {
         alive: false,
         head_tile: back_head_tile,
         prev_tail_dir: self.prev_tail_dir,
         segments: back_segments,
         ..*self
      };

      self.prev_tail_dir = if self.segments.len() == 0 { Some(segment_to_split.direction) } else { None };
      self.amt_to_grow = 0;

      back_snake
   }

   pub fn can_decap(&self, other : &Snake) -> bool {
      // Both Snakes must be alive and this Snake must be longer.
      !other.alive || (self.alive && other.len() <= self.len())
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
               let overlap = Overlap { segment_index, offset: offset.try_into().unwrap() };
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

   pub fn is_growing(&self) -> bool {
      self.amt_to_grow > 0
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

impl PartialEq for Snake {
   fn eq(&self, other: &Self) -> bool {
      self.player_index == other.player_index &&
      self.alive == other.alive &&
      self.head_tile == other.head_tile &&
      self.segments == other.segments &&
      self.prev_tail_dir == other.prev_tail_dir &&
      self.amt_to_grow == other.amt_to_grow &&
      self.color == other.color
   }
}

impl Eq for Snake { }

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
   pub offset : NonZeroU8,
}