// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

use std::u16;

/// Represents a player or player piece in a game of [`TicTacToe`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Player {
   /// The human player (who moves first).
   #[default]
   X,

   /// The AI opponent (who moves second).
   O,
}

impl std::fmt::Display for Player {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.write_str(match self {
         Self::X => "X",
         Self::O => "O",
      })
   }
}

impl Player {
   /// Returns the next player in the turn order.
   fn next(&self) -> Self {
      match self {
         Self::X => Self::O,
         Self::O => Self::X,
      }
   }
}

/// An array storing a [`TicTacToe`] board in row-major order.
pub type Board = [Option<Player>; (TicTacToe::BOARD_SIZE * TicTacToe::BOARD_SIZE) as usize];

/// Represents a position on a [`TicTacToe`] game board. Guaranteed to be valid.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pos(u16, u16);

impl Pos {
   /// Calculates the index of the tile to which a [`Pos`] corresponds. Guaranteed to be valid.
   pub fn index(&self) -> usize {
      ((self.1 * TicTacToe::BOARD_SIZE) + self.0) as usize
   }

   /// Returns a new [`Pos`] that is the same, except with the row flipped.
   pub fn flip_row(&self) -> Pos {
      Self(self.0, TicTacToe::BOARD_SIZE - self.1 - 1)
   }
}

impl TryFrom<(u16, u16)> for Pos {
   type Error = InvalidMove;

   /// Constructs a new game board position from a tuple. If the row or column was out of bounds, returns [`InvalidMove`].
   fn try_from(pos : (u16, u16)) -> Result<Self, Self::Error> {
      let (col, row) = pos;
      if col >= TicTacToe::BOARD_SIZE || row >= TicTacToe::BOARD_SIZE {
         Err(InvalidMove)
      }
      else {
         Ok(Self(col, row))
      }
   }
}

impl From<Pos> for (u16, u16) {
   fn from(value: Pos) -> Self {
      (value.0, value.1)
   }
}

/// Returned as an error when a move would not make sense, e.g. when a player tries to take an occupied space.
#[derive(Debug)]
pub struct InvalidMove;

/// Represents a straight line drawn across a [`TicTacToe`] board. Can be horizontal, vertical, or diagonal.
#[derive(Clone, Copy, Debug)]
pub enum Line {
   /// Represents a specific row of a [`TicTacToe`] board.
   Row(u16),

   /// Represents a specific column of a [`TicTacToe`] board.
   Col(u16),

   /// Represents one of the diagonals of a [`TicTacToe`] board. The `bool` field, if `true`, denotes that the diagonal
   /// is "flipped" over the Y-axis, i.e. starts at `(0, 2)` instead of `(0, 0)`.
   Diagonal(bool),
}

impl Line {
   /// Returns `true` if the [`Line`] overlaps with `pos` on a [`TicTacToe`] board. If `pos` does **not** overlap,
   /// returns `false`.
   pub fn contains(&self, pos : &Pos) -> bool {
      match self {
         Line::Row(row) => pos.1 == *row,
         Line::Col(col) => pos.0 == *col,
         Line::Diagonal(flip_row) => {
            let flipped = if *flip_row { pos.flip_row() } else { *pos };
            flipped.0 == flipped.1
         },
      }
   }
}

impl IntoIterator for Line {
   type Item = Pos;
   type IntoIter = TilesInLine;
   fn into_iter(self) -> Self::IntoIter {
      TilesInLine {
         line: self,
         next_offset: 0,
      }
   }
}

/// Iterator over each [`Pos`] in a [`Line`].
#[derive(Debug)]
pub struct TilesInLine {
   line : Line,
   next_offset : u16,
}

impl ExactSizeIterator for TilesInLine {
   fn len(&self) -> usize {
      // The number of tiles visited is ALWAYS the length/width of the board.
      TicTacToe::BOARD_SIZE as usize
   }
}

impl Iterator for TilesInLine {
   type Item = Pos;
   fn next(&mut self) -> Option<Self::Item> {
      let offset = self.next_offset;
      self.next_offset += 1;
      match self.line {
         Line::Row(row) => (offset, row).try_into().ok(),
         Line::Col(col) => (col, offset).try_into().ok(),
         Line::Diagonal(flip_row) => {
            let pos = (offset, offset).try_into().ok();
            if flip_row { pos.and_then(|pos : Pos| Some(pos.flip_row())) } else { pos }
         },
      }
   }
}

/// Represents the result of a [`TicTacToe`] game.
#[derive(Debug, Clone, Copy)]
pub enum Outcome {
   /// Both players tied.
   CatsGame,

   /// A player won the game. Includes the specific player who won and the winning line.
   Win(Player, Line),
}

#[derive(Debug, Default)]
pub struct TicTacToe {
   current_player : Player,
   board : Board,
   outcome : Option<Outcome>,
}

impl TicTacToe {
   /// The width and height, in tiles, of the (square) board.
   pub const BOARD_SIZE : u16 = 3;

   /// Attempts to claim a tile on the board at the specified [`Pos`] for the current player.
   fn take_turn(&mut self, pos : Pos) -> Result<(), InvalidMove> {
      let index = pos.index();
      let tile = &mut self.board[index];

      if tile.is_some() {
         // Can't claim a tile that is already claimed.
         return Err(InvalidMove);
      }

      // Claim the tile.
      *tile = Some(self.current_player);

      // Now it's the next player's turn.
      self.current_player = self.current_player.next();

      Ok(())
   }

   /// Tests for an [`Outcome`] for the game. If a player owns three tiles in a [`Line`], returns
   /// [`Some(Outcome::Win)`](Outcome::Win) for the owning [`Player`]. If no players owned a [`Line`] and there are
   /// no available tiles, returns [`Some(Outcome::CatsGame)`](Outcome::CatsGame). Otherwise, returns [`None`],
   /// indicating that the game should continue.
   fn check_outcome(&self) -> Option<Outcome> {
      use genawaiter::{yield_, stack::let_gen};

      let_gen!(lines, {
         for offset in 0..TicTacToe::BOARD_SIZE {
            // Test all the rows and columns of the board.
            yield_!(Line::Row(offset));
            yield_!(Line::Col(offset));
         }

         // Test both diagonals.
         yield_!(Line::Diagonal(false));
         yield_!(Line::Diagonal(true));
      });

      for line in lines {
         if let Some(player) = self.check_line(line) {
            // A player owns an entire line of tiles. That player wins!
            return Some(Outcome::Win(player, line));
         }
      }

      let has_empty_tile = self.board.into_iter().any(|tile| tile.is_none());
      if !has_empty_tile {
         // No more possible moves.
         Some(Outcome::CatsGame)
      }
      else {
         // The game is still going.
         None
      }
   }

   /// If all three tiles in a [`Line`] are owned by the same [`Player`], returns the [`Player`] who owns the line.
   fn check_line(&self, line : Line) -> Option<Player> {
      let mut owner = None;
      for pos in line {
         let tile = self.tile(pos);
         if tile.is_none() || (owner.is_some() && *tile != owner) {
            // There is no clear owner of this line.
            owner = None;
            break;
         }
         else if owner.is_none() {
            owner = *tile;
         }
      }
      owner
   }

   /// Returns the player whose turn it is.
   pub fn current_player(&self) -> Player {
      self.current_player
   }

   /// Borrows the tile at the specified [`Pos`] on the game board.
   pub fn tile(&self, pos : Pos) -> &Option<Player> {
      let index = pos.index();
      &self.board[index]
   }

   /// Borrows the [`Outcome`] of the game, if any.
   pub fn outcome(&self) -> &Option<Outcome> {
      &self.outcome
   }
}

impl posturn::Play for TicTacToe {
   type Input = Pos;
   type Event = Result<Player, InvalidMove>;
   type Outcome = Outcome;

   fn play(ctx : posturn::Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         let mut last_turn_result = Ok(());

         loop {
            let next_event = last_turn_result.map(|_| {
               // Ask the player for an input.
               let current_player = ctx.host.borrow_game().current_player;
               current_player
            });

            // Wait for the player to supply a position to claim.
            let pos = ctx.yield_event(next_event).await.into();

            // Attempt to place a piece for the current player.
            last_turn_result = ctx.host.borrow_game_mut().take_turn(pos);

            if last_turn_result.is_err() {
               continue;
            }
            else if let Some(outcome) = ctx.host.with_game(|game| game.check_outcome()) {
               // Game over!
               ctx.host.borrow_game_mut().outcome = Some(outcome);
               return outcome;
            }
         }
      }
   }
}