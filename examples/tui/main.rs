// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

mod game;
use game::TicTacToe;

mod view;
use view::View;

use futures::pin_mut;
use std::io::{self, stdout};

use crossterm::{queue, terminal};
use posturn::genawaiter::Coroutine;

fn main() -> io::Result<()> {
   let mut out = stdout();
   queue!(out, terminal::EnterAlternateScreen)?;
   terminal::enable_raw_mode()?;
   
   'new_game : loop {
      let mut view = View::new(terminal::size()?);
      
      let host = posturn::Host::new(TicTacToe::default());
      let co = host.play().unwrap();
      pin_mut!(co);

      let mut pos = Default::default();
      let mut last_outcome = None;

      while last_outcome.is_none() {
         // NOTE: We need to call this once with a default argument to start the game, hence being at the top of the loop.
         last_outcome = match co.as_mut().resume_with(pos) {
            genawaiter::GeneratorState::Yielded(_) => None,
            genawaiter::GeneratorState::Complete(outcome) => Some(outcome),
         };

         match host.with_game(|game| view.wait_for_input(&mut out, &game))? {
            view::Event::TakeTurn(col, row) => {
               // Place a piece and update the turn-based game.
               pos = (col, row).try_into().expect("Invalid position");
            },
            view::Event::NewGame => continue 'new_game,
            view::Event::Quit => break 'new_game,
         };
      }
   }
   
   terminal::disable_raw_mode()?;
   queue!(out, terminal::LeaveAlternateScreen)?;
   Ok(())
}