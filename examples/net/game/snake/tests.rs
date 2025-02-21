mod segment {
   use super::super::*;
   use Direction::*;

   #[test]
   fn test_from_tuple() {
      let segment : Segment = (East, 8).try_into().expect("Failed to create Segment");
      assert_eq!(segment, Segment { direction: East, len: NonZeroU8::new(8).unwrap() });
   }

   #[test]
   fn test_from_tuple_zero_length() {
      Segment::try_from((East, 0)).expect_err("Length must be non-zero");
   }

   #[test]
   fn test_with_facing() {
      let segment = Segment::with_facing(North);
      assert_eq!(segment, Segment { direction: South, len: NonZeroU8::MIN });
   }

   #[test]
   fn test_facing() {
      let segment = Segment::with_facing(North);
      assert_eq!(segment.facing(), North);
   }

   #[test]
   fn test_split() {
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
}

mod snake {
   use super::super::*;
   use Direction::*;

   const SOME_PLAYER_INDEX : usize = 1;

   #[test]
   fn test_spawn() {
      let params = SpawnParams {
         alive: true,
         head_tile: i16vec2(2, -3),
         color: BEIGE,
         amt_to_grow: 1,
         segments: vec![
            (West, 3),
            (South, 2),
            (East, 4),
            (North, 1),
         ],
         ..Default::default()
      };

      let snake = Snake::try_spawn(SOME_PLAYER_INDEX, params).expect("Expected Snake to spawn successfully");

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
      let params = SpawnParams {
         alive: true,
         head_tile: I16Vec2::ZERO,
         color: BEIGE,
         amt_to_grow: 9,
         segments: vec![
            (North, 2),
            (West,  2),
            (South, 2),
            (East,  2), // Overlaps with head!
         ],
         ..Default::default()
      };

      let overlap = Snake::try_spawn(SOME_PLAYER_INDEX, params)
         .expect_err("Expected error when trying to spawn Snake");

      assert_eq!(overlap, Overlap { segment_index: 3, offset: 2.try_into().unwrap() });
   }

   struct SplitTestData {
      segment_index : usize,
      offset : u8,
      expected_front : SpawnParams,
      expected_back : SpawnParams,
   }

   #[test]
   fn test_split() {
      let params = SpawnParams {
         alive: true,
         head_tile: I16Vec2::ZERO,
         color: BEIGE,
         amt_to_grow: 0,
         segments: vec![
            (West, 2),
            (South, 2),
         ],
         prev_tail_dir: Some(East),
      };

      let snake_to_split = Snake::try_spawn(SOME_PLAYER_INDEX, params.clone())
         .expect("Expected Snake to spawn successfully");

      let mut i : usize = 0;
      let mut test_split = |test : SplitTestData| {
         i += 1;

         let mut front = snake_to_split.clone();
         let back = front.split_off(
         test.segment_index,
         test.offset.try_into().expect("Expected non-zero offset"));

         let expected_front_snake = Snake::try_spawn(SOME_PLAYER_INDEX, test.expected_front)
            .expect(&format!("At iteration {i}, expected front Snake params were invalid"));
         let expected_back_snake = Snake::try_spawn(SOME_PLAYER_INDEX, test.expected_back)
            .expect(&format!("At iteration {i}, expected back Snake params were invalid"));

         assert_eq!(front, expected_front_snake, "At iteration {i}, expected front of Snake to match");
         assert_eq!(back, expected_back_snake, "At iteration {i}, expected back of Snake to match");
      };
      
      test_split(SplitTestData {
         segment_index: 0,
         offset: 1,
         expected_front: SpawnParams {
            head_tile: i16vec2(0, 0),
            segments: vec![],
            prev_tail_dir: Some(West),
            ..params
         },
         expected_back: SpawnParams {
            alive: false,
            head_tile: i16vec2(-1, 0),
            segments: vec![(West, 1), (South, 2)],
            prev_tail_dir: Some(East),
            ..params
         },
      });

      test_split(SplitTestData {
         segment_index: 0,
         offset: 2,
         expected_front: SpawnParams {
            head_tile: i16vec2(0, 0),
            segments: vec![(West, 1)],
            prev_tail_dir: None,
            ..params
         },
         expected_back: SpawnParams {
            alive: false,
            head_tile: i16vec2(-2, 0),
            segments: vec![(South, 2)],
            prev_tail_dir: Some(East),
            ..params
         },
      });

      test_split(SplitTestData {
         segment_index: 1,
         offset: 1,
         expected_front: SpawnParams {
            head_tile: i16vec2(0, 0),
            segments: vec![(West, 2)],
            prev_tail_dir: None,
            ..params
         },
         expected_back: SpawnParams {
            alive: false,
            head_tile: i16vec2(-2, 1),
            segments: vec![(South, 1)],
            prev_tail_dir: Some(East),
            ..params
         },
      });

      test_split(SplitTestData {
         segment_index: 1,
         offset: 2,
         expected_front: SpawnParams {
            head_tile: i16vec2(0, 0),
            segments: vec![(West, 2), (South, 1)],
            prev_tail_dir: None,
            ..params
         },
         expected_back: SpawnParams {
            alive: false,
            head_tile: i16vec2(-2, 2),
            segments: vec![],
            prev_tail_dir: Some(East),
            ..params
         },
      });
   }
}