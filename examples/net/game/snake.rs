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
   pub amt_to_grow : u16,
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

      let len = Self::measure(segments.iter());
      let amt_to_grow = params.len.checked_sub(len.get()).expect("Snake is already longer than target length");

      let snake = Self {
         player_index,
         alive: params.alive,
         head_tile: params.head_tile_pos,
         segments,
         prev_tail_dir: None,
         amt_to_grow,
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
      let last_segment = self.segments.pop_back()?;
      let (front, back) = last_segment.split_at(last_segment.len);
      assert!(back.is_none(), "Expected to remove only last tile");
      self.prev_tail_dir = Some(last_segment.direction);
      self.segments.extend(front);
      Some(self)
   }

   /// Splits the [`Snake`] at the specified [`Segment`] and offset and returns the tail as a new [`Snake`].
   #[cfg(test)]
   pub fn split_off(&mut self, segment_index : usize, offset : NonZeroU8) -> Snake {
      let (back_corner, back_segment) = self.segments().nth(segment_index).expect("Segment index out of bounds");
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
   pub offset : u8,
}

#[cfg(test)]
mod tests {
   use super::*;
   use Direction::*;

   #[test]
   fn test_segment_from_tuple() {
      let segment : Segment = (East, 8).try_into().expect("Failed to create Segment");
      assert_eq!(segment, Segment { direction: East, len: NonZeroU8::new(8).unwrap() });
   }
   
   #[test]
   fn test_segment_from_tuple_zero_length() {
      Segment::try_from((East, 0)).expect_err("Length must be non-zero");
   }

   #[test]
   fn test_segment_with_facing() {
      let segment = Segment::with_facing(North);
      assert_eq!(segment, Segment { direction: South, len: NonZeroU8::MIN });
   }

   #[test]
   fn test_segment_facing() {
      let segment = Segment::with_facing(North);
      assert_eq!(segment.facing(), North);
   }

   #[test]
   fn test_segment_split() {
      let mut i = 0;

      let mut run_test = |start_len: u8, split_at: u8, (front, back): (Option<u8>, Option<u8>)| {
         i += 1;
         
         let segment : Segment = (Direction::West, start_len).try_into().unwrap();
         let actual = segment.split_at(split_at.try_into().unwrap());
         let make_segment = |maybe_len : Option<u8>| {
            maybe_len.and_then(|len| (segment.direction, len).try_into().ok())
         };
         let expected = (make_segment(front), make_segment(back));
         assert_eq!(expected, actual, "At iteration {i}, expected Segments to match");
      };

      run_test(1, 1, (None, None));
      run_test(2, 1, (None, Some(1)));
      run_test(2, 2, (Some(1), None));
      run_test(3, 2, (Some(1), Some(1)));
   }

   const SOME_PLAYER_INDEX : usize = 1;
   fn make_spawn_params(head_tile_pos : I16Vec2, len : u16) -> SpawnParams {
      SpawnParams {
         alive: true,
         head_tile_pos,
         color: BEIGE,
         len,
         ..Default::default()
      }
   }

   #[test]
   fn test_snake_spawn() {
      let params = make_spawn_params(i16vec2(2, -3), 12); // NOTE: Target length is LONGER than Snake length

      let segments = vec![
         (West, 3),
         (South, 2),
         (East, 4),
         (North, 1),
      ];

      let snake = Snake::try_spawn_with_segments(SOME_PLAYER_INDEX, params, segments)
         .expect("Expected Snake to spawn successfully");

      assert_eq!(snake.alive, true);
      assert_eq!(snake.head_tile(), i16vec2(2, -3));
      assert_eq!(snake.amt_to_grow, 1);

      const EXPECTED_SEGMENTS : [(I16Vec2, (Direction, u8)); 4] = [
         (i16vec2( 2, -3), (West,  3)),
         (i16vec2(-1, -3), (South, 2)),
         (i16vec2(-1, -1), (East,  4)),
         (i16vec2( 3, -1), (North, 1)),
      ];
      
      for (index, ((tiles, segment), (expected_tiles, expected_segment))) in snake.segments().zip(EXPECTED_SEGMENTS.iter()).enumerate() {
         assert_eq!(tiles, *expected_tiles, "Tiles for Segment {index} were incorrect");
         assert_eq!(*segment, Segment::try_from(*expected_segment).unwrap(), "Segment {index} was incorrect");
      }

      assert_eq!(snake.color, BEIGE);
   }

   #[test]
   fn test_invalid_spawn() {
      let params = make_spawn_params(I16Vec2::ZERO, 9);

      let segments = vec![
         (North, 2),
         (West,  2),
         (South, 2),
         (East,  2), // Overlaps with head!
      ];

      let overlap = Snake::try_spawn_with_segments(SOME_PLAYER_INDEX, params, segments)
         .expect_err("Expected error when trying to spawn Snake");

      assert_eq!(overlap, Overlap { segment_index: 3, offset: 2 });
   }

   struct SplitTestData {
      segment_index : usize,
      offset : u8,
      expected_front : SnakeTestData,
      expected_back : SnakeTestData,
   }

   struct SnakeTestData {
      head_tile : I16Vec2,
      segments : Vec<(Direction, u8)>,
      prev_tail_dir : Option<Direction>,
   }

   impl SnakeTestData {
      pub fn into_snake(self, snake_to_clone : &Snake, alive : bool) -> Snake {
         let segments : Vec<Segment> = self.segments.iter()
            .copied()
            .map(|tuple : (Direction, u8)| -> Segment {
               tuple.try_into().expect("Expected Segment to have non-zero length")
            })
            .collect();

         let mut snake = Snake::try_spawn_with_segments(
            snake_to_clone.player_index,
            SpawnParams {
               head_tile_pos: self.head_tile,
               alive,
               len: Snake::measure(segments.iter()).get(),
               color: snake_to_clone.color,
            },
            self.segments)
            .expect("Expected Snake to spawn correctly");

         snake.prev_tail_dir = self.prev_tail_dir;
         snake
      }
   }

   #[test]
   fn test_split() {
      let params = make_spawn_params(I16Vec2::ZERO, 5);
      let segments = vec![
         (West, 2),
         (South, 2),
      ];

      let snake_to_split = {
         let mut snake = Snake::try_spawn_with_segments(SOME_PLAYER_INDEX, params, segments)
            .expect("Expected Snake to spawn successfully");
         snake.prev_tail_dir = Some(East);
         snake
      };

      let mut i : usize = 0;
      let mut test_split = |test : SplitTestData| {
         i += 1;

         let mut front = snake_to_split.clone();
         let back = front.split_off(
            test.segment_index,
            test.offset.try_into().expect("Expected non-zero offset"));

         let expected_front_snake = test.expected_front.into_snake(&snake_to_split, true);
         let expected_back_snake = test.expected_back.into_snake(&snake_to_split, false);

         assert_eq!(front, expected_front_snake, "At iteration {i}, expected front of Snake to match");
         assert_eq!(back, expected_back_snake, "At iteration {i}, expected back of Snake to match");
      };
      
      test_split(SplitTestData {
         segment_index: 0,
         offset: 1,
         expected_front: SnakeTestData { head_tile: i16vec2(0, 0), segments: vec![], prev_tail_dir: Some(West) },
         expected_back: SnakeTestData { head_tile: i16vec2(-1, 0), segments: vec![(West, 1), (South, 2)], prev_tail_dir: Some(East) },
      });
      test_split(SplitTestData {
         segment_index: 0,
         offset: 2,
         expected_front: SnakeTestData { head_tile: i16vec2(0, 0), segments: vec![(West, 1)], prev_tail_dir: None },
         expected_back: SnakeTestData { head_tile: i16vec2(-2, 0), segments: vec![(South, 2)], prev_tail_dir: Some(East) },
      });
      test_split(SplitTestData {
         segment_index: 1,
         offset: 1,
         expected_front: SnakeTestData { head_tile: i16vec2(0, 0), segments: vec![(West, 2)], prev_tail_dir: None, },
         expected_back: SnakeTestData { head_tile: i16vec2(-2, 1), segments: vec![(South, 1)], prev_tail_dir: Some(East) },
      });
      test_split(SplitTestData {
         segment_index: 1,
         offset: 2,
         expected_front: SnakeTestData { head_tile: i16vec2(0, 0), segments: vec![(West, 2), (South, 1)], prev_tail_dir: None },
         expected_back: SnakeTestData { head_tile: i16vec2(-2, 2), segments: vec![], prev_tail_dir: Some(East) },
      });
   }
}