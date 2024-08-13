// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, string::String};
use genawaiter::{Generator, GeneratorState};
use crate::{Context, Host, Play};

/// Represents input received from a player in a game of [`RoShamBo`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Choice {
   /// Wins against [`Scissors`](Choice::Scissors). Loses to [`Paper`](Choice::Paper).
   Rock,

   /// Wins against [`Rock`](Choice::Rock). Loses to [`Scissors`](Choice::Scissors).
   Paper,

   /// Wins against [`Paper`](Choice::Paper). Loses to [`Rock`](Choice::Rock).
   Scissors,
}

impl PartialOrd for Choice {
   fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
      match (self, other) {
         (&Choice::Rock, &Choice::Rock) => Some(Ordering::Equal),
         (&Choice::Rock, &Choice::Paper) => Some(Ordering::Less),
         (&Choice::Rock, &Choice::Scissors) => Some(Ordering::Greater),
         (&Choice::Paper, &Choice::Paper) => Some(Ordering::Equal),
         (&Choice::Paper, &Choice::Scissors) => Some(Ordering::Less),
         (&Choice::Paper, &Choice::Rock) => Some(Ordering::Greater),
         (&Choice::Scissors, &Choice::Scissors) => Some(Ordering::Equal),
         (&Choice::Scissors, &Choice::Rock) => Some(Ordering::Less),
         (&Choice::Scissors, &Choice::Paper) => Some(Ordering::Greater),
      }
   }
}

/// An event that occurs during a game of [`RoShamBo`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct Msg(String);

/// Outcome of a game of [`RoShamBo`]. Always relative to player 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
   /// Both players picked the same [`Choice`]. Neither player wins.
   Tie,

   /// Player 1's [`Choice`] beats player 2. Player 1 wins.
   Win,

   /// Player 2's [`Choice`] beats player 1. Player 2 wins.
   Loss,
}

/// The two fields are player 1's choice and player 2's choice, respectively.
#[derive(Clone, Copy, Debug)]
struct RoShamBo(Choice, Choice);

impl Play for RoShamBo {
   type Input = ();
   type Event = Msg;
   type Outcome = Outcome;

   fn play(ctx : Context<Self>) -> impl std::future::Future<Output = Self::Outcome> {
      async move {
         // Count down to the reveal of both choices...
         ctx.yield_event(Msg("Ro!".into())).await;
         ctx.yield_event(Msg("Sham!".into())).await;
         ctx.yield_event(Msg("Bo!".into())).await;

         // Assess the winner.
         let Self(player_1, player_2) = ctx.host.game();
         let outcome = match player_1.partial_cmp(&player_2).unwrap() {
            Ordering::Equal => Outcome::Tie,
            Ordering::Greater => Outcome::Win,
            Ordering::Less => Outcome::Loss,
         };

         // Tell the player what happened.
         let msg =
            match outcome {
               Outcome::Tie => format!("{player_1:?} ties with {player_2:?}."),
               Outcome::Win => format!("{player_1:?} beats {player_2:?}."),
               Outcome::Loss => format!("{player_2:?} beats {player_1:?}."),
            };

         ctx.yield_event(Msg(msg)).await;

         // Game over!
         outcome
      }
   }

   fn handle_event(&mut self, event : &mut <Self as Play>::Event) {
      // For easier debugging of tests, print messages to the console. Use `cargo test -- --nocapture` to display these.
      println!("{}", event.0)
   }
}

#[test]
fn it_works() {
   test_ro_sham_bo(RoShamBo(Choice::Rock, Choice::Rock), "Rock ties with Rock.".into(), Outcome::Tie);
   test_ro_sham_bo(RoShamBo(Choice::Rock, Choice::Paper), "Paper beats Rock.".into(), Outcome::Loss);
   test_ro_sham_bo(RoShamBo(Choice::Rock, Choice::Scissors), "Rock beats Scissors.".into(), Outcome::Win);
   test_ro_sham_bo(RoShamBo(Choice::Paper, Choice::Rock), "Paper beats Rock.".into(), Outcome::Win);
   test_ro_sham_bo(RoShamBo(Choice::Paper, Choice::Paper), "Paper ties with Paper.".into(), Outcome::Tie);
   test_ro_sham_bo(RoShamBo(Choice::Paper, Choice::Scissors), "Scissors beats Paper.".into(), Outcome::Loss);
   test_ro_sham_bo(RoShamBo(Choice::Scissors, Choice::Rock), "Rock beats Scissors.".into(), Outcome::Loss);
   test_ro_sham_bo(RoShamBo(Choice::Scissors, Choice::Paper), "Scissors beats Paper.".into(), Outcome::Win);
   test_ro_sham_bo(RoShamBo(Choice::Scissors, Choice::Scissors), "Scissors ties with Scissors.".into(), Outcome::Tie);
}

fn test_ro_sham_bo(game : RoShamBo, expected_msg : String, expected_outcome : Outcome) {
   use futures::pin_mut;
   
   // Think "host" as in the person in charge of running the game, rather than "host" as a networking term.
   let host = Host::new(game);
   
   let co = host.play().unwrap();
   pin_mut!(co);

   assert_eq!(co.as_mut().resume(), GeneratorState::Yielded(Msg("Ro!".into())));
   assert_eq!(co.as_mut().resume(), GeneratorState::Yielded(Msg("Sham!".into())));
   assert_eq!(co.as_mut().resume(), GeneratorState::Yielded(Msg("Bo!".into())));
   assert_eq!(co.as_mut().resume(), GeneratorState::Yielded(Msg(expected_msg)));
   assert_eq!(co.as_mut().resume(), GeneratorState::Complete(expected_outcome));
}