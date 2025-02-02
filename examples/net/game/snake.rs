use std::{collections::{vec_deque, VecDeque}, iter::Enumerate, num::{NonZeroU16, NonZeroU8}, ops::{Add, RangeInclusive}};
use macroquad::prelude::*;

use super::direction::{Direction, Offset};

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

impl Add<Direction> for I16Vec2 {
   type Output = Self;
   fn add(self, direction: Direction) -> Self::Output {
      self.offset(direction, 1)
   }
}

#[derive(Debug)]
pub struct SpawnParams {
   pub alive : bool,
   pub head_tile_pos : I16Vec2,
   pub segments : VecDeque<Segment>,
   pub color : Color,
}

#[derive(Clone, Debug)]
pub struct Snake {
   pub player_index : usize,
   pub alive : bool,
   head_tile : I16Vec2,
   segments : VecDeque<Segment>,
   prev_tail_dir : Option<Direction>,
   pub color : Color,
}

impl Snake {
   pub fn spawn(player_index : usize, params : SpawnParams) -> Self {
      assert!(!params.segments.is_empty());
      Self {
         player_index,
         alive: params.alive,
         head_tile: params.head_tile_pos,
         segments: params.segments,
         prev_tail_dir: None,
         color: params.color,
      }
   }

   pub fn grow_forward(&mut self) {
      // Move the head forward by one tile.
      self.head_tile = self.head_tile + self.facing();
      let head = self.segments.front_mut().unwrap();
      head.len = head.len.saturating_add(1);
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

      let head_segment = self.segments.front().unwrap();
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
      if !self.segments.is_empty() { Some(self) } else { None }
   }

   pub fn shrink_tail(mut self) -> Option<Self> {
      let last_segment = self.segments.back().expect("Snake cannot be made shorter!");
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

      if !self.segments.is_empty() { Some(self) } else { None }
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
      self.segments.front().unwrap().facing()
   }

   pub fn len(&self) -> NonZeroU16 {
      // The length of the Snake is the length of its Segments...
      let len_segments = self.segments.iter()
         .map(|segment| segment.len.get() as u16)
         .sum();

      // ...plus one for the head.
      NonZeroU16::MIN.saturating_add(len_segments)
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