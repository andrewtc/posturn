use macroquad::prelude::*;

use crate::game::direction::Direction;

#[derive(Debug)]
pub struct KeyControls {
   pub west  : KeyCode,
   pub east  : KeyCode,
   pub north : KeyCode,
   pub south : KeyCode,
}

impl KeyControls {
   pub const ARROW_KEYS : Self = KeyControls { west: KeyCode::Left, east: KeyCode::Right, north: KeyCode::Up, south: KeyCode::Down };
   pub const WASD : Self = KeyControls { west: KeyCode::A, east: KeyCode::D, north: KeyCode::W, south: KeyCode::S };

   pub fn desired_direction(&self) -> Option<Direction> {
      if is_key_pressed(self.west) { Some(Direction::West) }
      else if is_key_pressed(self.east) { Some(Direction::East) }
      else if is_key_pressed(self.north) { Some(Direction::North) }
      else if is_key_pressed(self.south) { Some(Direction::South) }
      else { None }
   }
}

#[derive(Debug)]
pub enum Control {
   /// Allow controlling the [`Player`] with keyboard controls.
   Manual(&'static KeyControls),

   /// The AI controls the [`Player`].
   Auto,
}