use std::{collections::{vec_deque, VecDeque}, iter::Enumerate, num::{NonZeroU16, NonZeroU8}, ops::{Add, RangeInclusive}};
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

#[derive(Clone, Debug)]
pub struct Snake {
   pub alive : bool,
   start : I16Vec2,
   segments : VecDeque<Segment>,
   pub color : Color,
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

      self.start = self.start + direction;
      if !self.segments.is_empty() { Some(self) } else { None }
   }

   pub fn shrink_tail(mut self) -> Option<Self> {
      let len = self.segments.back().unwrap().len;
      let new_len = len.get().saturating_sub(1);

      if new_len > 0 {
         self.segments.back_mut().unwrap().len = new_len.try_into().unwrap();
      }
      else {
         self.segments.pop_back();
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

   pub fn is_overlapping_self(&self) -> bool {
      // Snakes only overlap with themselves if the head touches a body segment.
      self.overlaps(self.start).any(|(index, _)| index > 0)
   }

   pub fn start(&self) -> I16Vec2 {
      self.start
   }

   pub fn segments(&self) -> Segments<'_> {
      Segments { next_start: self.start, inner: self.segments.iter() }
   }

   pub fn num_segments(&self) -> usize {
      self.segments.len()
   }

   pub fn facing(&self) -> Direction {
      self.segments.front().unwrap().facing()
   }

   pub fn len(&self) -> NonZeroU16 {
      let len : u16 = self.segments.iter().map(|segment| segment.len.get() as u16).sum();
      len.try_into().unwrap()
   }
}

/// An iterator over the [`Segment`s](Segment) of a [`Snake`]. Also outputs the start and end location of the `Segment`
/// as a [`RangeInclusive`] of [`I16Vec2`].
#[derive(Debug)]
pub struct Segments<'iter> {
   next_start : I16Vec2,
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
         let start = self.next_start;
         let end = start + *segment;
         self.next_start = end;
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