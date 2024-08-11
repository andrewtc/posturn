// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

use std::{borrow::Borrow, io};

use crossterm::{cursor, event::{self, KeyCode}, queue, style::{self, Stylize}, terminal};

use crate::game::{Outcome, Pos, TicTacToe};

pub enum Event {
   /// The player wants to start a new game of Tic Tac Toe.
   NewGame,

   /// The player wants to place a piece on the game board at the specified tile
   TakeTurn(u16, u16),

   /// The player wants to quit the game.
   Quit,
}

/// Manages state for the Tic Tac Toe terminal UI.
pub struct View {
   was_resized : bool,
   terminal_size : (u16, u16),
   selected_tile : (u16, u16),
}

impl View {
   /// If the terminal is smaller than this, an error will be displayed.
   const MIN_SIZE : (u16, u16) = (40, 10);

   /// The text to draw to represent the game board in the terminal.
   const BG_TEXT : &'static str = " TIC TAC TOE
╔═══════════╗
║   ┃   ┃   ║
║━━━╋━━━╋━━━║
║   ┃   ┃   ║
║━━━╋━━━╋━━━║
║   ┃   ┃   ║
╚═══════════╝";

   /// The top left corner of the board, measured from the top left of the terminal.
   const TOP_LEFT : (u16, u16) = (2, 1);

   /// The row, column position of the top, leftmost **tile** on the game board.
   const TILE_OFFSET : (u16, u16) = (2, 2);
   
   /// The row, column spacing between individual tiles on the game board.
   const TILE_SPACING : (u16, u16) = (4, 2);
   
   /// The total number of tiles on the game board in each direction (columns, rows).
   const NUM_TILES : (u16, u16) = (3, 3);

   /// Used to pad the characters written in the prompt area.
   const PROMPT_MAX_WIDTH : usize = 20;

   /// The text displaying controls to the player.
   const CONTROLS_PROMPT : &'static str = "\
ENTER : Claim a tile
 ←↑→↓ : Move cursor
  ESC : Quit";
   
   /// The row, column position of the top left corner of the prompt text.
   const PROMPT_TOP_LEFT : (u16, u16) = (17, 3);

   const GAME_OVER_PROMPT : &'static str = "\
ENTER: Play again
  ESC: Quit";

   /// Creates and returns a new terminal UI for a Tic Tac Toe game.
   pub fn new(terminal_size : (u16, u16)) -> Self {
      Self {
         was_resized: true,
         terminal_size,
         selected_tile: (1, 1),
      }
   }

   /// Poll for input until the player does something that needs to update the state of the turn-based game.
   pub fn wait_for_input<W>(&mut self, out : &mut W, game : &TicTacToe) -> io::Result<Event> where
      W : io::Write,
   {
      loop {
         // Update to match the game board.
         self.redraw(out, game)?;
         out.flush()?;
   
         let event = event::read()?;
         match event {
            event::Event::Resize(width, height) => {
               self.was_resized = true;
               self.terminal_size = (width, height);
            },
            event::Event::Key(event::KeyEvent { code, kind: event::KeyEventKind::Press, .. }) => {
               let event_to_process = if let Some(_) = game.outcome() {
                  self.handle_game_over_key_press(code)
               }
               else {
                  self.handle_game_key_press(code)
               };

               if let Some(event) = event_to_process {
                  return Ok(event);
               }
            },
            _ => (),
         }
      }
   }

   /// Processes input for a Tic Tac Toe game. Returns an [`Event`] that is processed by the main application.
   fn handle_game_key_press(&mut self, code : KeyCode) -> Option<Event> {
      let mut tile_to_select = self.selected_tile;

      match code {
         KeyCode::Esc => {
            return Some(Event::Quit);
         },
         KeyCode::Right => {
            tile_to_select.0 = tile_to_select.0.saturating_add(1);
         },
         KeyCode::Left => {
            tile_to_select.0 = tile_to_select.0.saturating_sub(1);
         },
         KeyCode::Down => {
            tile_to_select.1 = tile_to_select.1.saturating_add(1);
         },
         KeyCode::Up => {
            tile_to_select.1 = tile_to_select.1.saturating_sub(1);
         },
         KeyCode::Enter => {
            // Place a piece on the game board.
            return Some(Event::TakeTurn(tile_to_select.0, tile_to_select.1));
         },
         _ => ()
      }
      
      self.select_tile(tile_to_select);
      None
   }

   /// Processes input between games of Tic Tac Toe. Returns an [`Event`] that is processed by the main application.
   fn handle_game_over_key_press(&self, code : KeyCode) -> Option<Event> {
      match code {
         KeyCode::Esc => Some(Event::Quit),
         KeyCode::Enter => Some(Event::NewGame),
         _ => None
      }
   }

   pub fn redraw<W>(&mut self, out : &mut W, game : &TicTacToe) -> io::Result<()>
      where W : io::Write,
   {
      if self.was_resized {
         self.was_resized = false;
         queue!(
            out,
            style::SetBackgroundColor(style::Color::DarkBlue),
            terminal::Clear(terminal::ClearType::Purge),
            terminal::Clear(terminal::ClearType::All),
         )?;
      }

      if self.terminal_size.0 < Self::MIN_SIZE.0 ||
         self.terminal_size.1 < Self::MIN_SIZE.1
      {
         queue!(out, cursor::Hide)?;

         // Print an error message.
         let (min_width, min_height) = Self::MIN_SIZE;
         let msg = format!("Please resize your terminal.\nMust have {min_width} cols, {min_height} rows.");
         return Self::write_at(out, (0, 0), self.terminal_size.0 as usize, msg);
      }

      Self::write_at(out, Self::TOP_LEFT, Self::PROMPT_MAX_WIDTH, Self::BG_TEXT)?;

      let outcome = game.outcome();
      if let Some(outcome) = outcome {
         // Display who won to the player.
         let win_text = match outcome {
            Outcome::CatsGame => "Cat's Game".to_owned(),
            Outcome::Win(player, _) => format!("  {player}'s win!"),
         };

         let prompt = format!("  GAME OVER\n {win_text}\n\n{}", Self::GAME_OVER_PROMPT);
         Self::write_at(out, Self::PROMPT_TOP_LEFT, Self::PROMPT_MAX_WIDTH, prompt)?;
      }
      else {
         // Let the player know whose turn it is.
         let player = game.current_player();
         let prompt = format!("   {player}'s turn\n\n{}", Self::CONTROLS_PROMPT);
         Self::write_at(out, Self::PROMPT_TOP_LEFT, Self::PROMPT_MAX_WIDTH, prompt)?;
      }

      // Draw all pieces on the board.
      for row in 0..TicTacToe::BOARD_SIZE {
         for col in 0..TicTacToe::BOARD_SIZE {
            let pos : Pos = (col, row).try_into().unwrap();
            if let Some(player) = game.tile(pos) {
               let tile_pos = Self::calc_tile_pos((col, row));
               let piece = format!("{}", player);
               
               let stylized_piece = match outcome {
                  Some(Outcome::Win(_, line)) => {
                     if line.contains(&pos) { piece.bold() } else { piece.dim() }
                  },
                  Some(Outcome::CatsGame) => piece.dim(),
                  _ => piece.white(),
               };

               queue!(
                  out,
                  cursor::MoveTo(tile_pos.0, tile_pos.1),
                  style::PrintStyledContent(stylized_piece),

                  // This fixes a bug in some terminals where the background color gets reset by PrintStyledContent.
                  style::SetBackgroundColor(style::Color::DarkBlue),
               )?;
            }
         }
      }
      
      // Whenever we redraw, we update the cursor position to show the selected tile.
      let (cursor_x, cursor_y) = Self::calc_tile_pos(self.selected_tile);
   
      if outcome.is_some() {
         // Hide the cursor to indicate that cursor input is no longer being accepted.
         queue!(out, cursor::Hide)
      }
      else {
         // Blink the cursor to indicate that the player can claim the tile under the cursor.
         queue!(
            out,
            cursor::Show,
            cursor::SetCursorStyle::BlinkingBlock,
            cursor::MoveTo(cursor_x, cursor_y),
         )
      }
   }

   fn select_tile(&mut self, tile : (u16, u16)) {
      self.selected_tile = (
         tile.0.min(Self::NUM_TILES.0.saturating_sub(1)),
         tile.1.min(Self::NUM_TILES.1.saturating_sub(1)));
   }

   /// Calculates the row and column of the **center** of a tile, measured from the top left of the terminal.
   fn calc_tile_pos(tile : (u16, u16)) -> (u16, u16) {
      (
         Self::TOP_LEFT.0 + Self::TILE_OFFSET.0 + (tile.0 * Self::TILE_SPACING.0),
         Self::TOP_LEFT.1 + Self::TILE_OFFSET.1 + (tile.1 * Self::TILE_SPACING.1),
      )
   }

   /// Write all lines of the input string to the terminal with a top left offset.
   fn write_at<W, S>(out : &mut W, pos : (u16, u16), width : usize, text : S) -> io::Result<()> where
      W : io::Write,
      S : Borrow<str>,
   {
      queue!(out, cursor::MoveTo(pos.0, pos.1))?;

      for line in text.borrow().lines() {
         let with_padding = format!("{line:width$}");
         queue!(
            out,
            style::Print(with_padding),
            cursor::MoveToColumn(pos.0),
            cursor::MoveDown(1),
         )?;
      }

      Ok(())
   }
}