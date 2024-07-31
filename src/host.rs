// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

use std::{cell::{Ref, RefCell, RefMut}, rc::Rc};

use genawaiter::{rc::{Co, Gen}, Coroutine};

use crate::Play;

/// Shared helper structure that keeps track of whether a game has been started and also tracks game state.
struct State<Game : Play> {
   is_in_progress : bool,
   game : Game,
}

impl<Game> From<Game> for State<Game> where
   Game : Play,
{
   fn from(game : Game) -> Self {
      Self {
         is_in_progress: false,
         game,
      }
   }
}

#[derive(Debug)]
pub enum PlayError {
   /// The game cannot be started because the game state is currently being accessed.
   InUse,

   /// The game has already been started, and cannot be started again.
   AlreadyStarted,
}

/// Manages a game, offering read/write access to the game state whenever the game is **not** currently being run.
pub struct Host<Game : Play> {
   state : Rc<RefCell<State<Game>>>,
}

impl<Game> Host<Game> where
   Game : Play,
{
   /// Creates a new [`Host`] to manage a game session, where `game` holds the initial state of the game "board". Any
   /// setup is expected to happen _before_ this, such that calling [`Host::play`] will initiate the first turn.
   pub fn new(game : Game) -> Self {
      let state = Rc::new(RefCell::new(game.into()));
      Self { state }
   }

   /// Starts a new game, returning a [`Coroutine`] that allows the caller to process [`Event`s](Play::Event)
   /// asynchronously as they are emitted. If the game has already been started or cannot be updated, returns a
   /// [`PlayError`].
   pub fn play(&self) -> Result<
      impl Coroutine<
         Yield = <Game as Play>::Event,
         Resume = <Game as Play>::Input,
         Return = <Game as Play>::Outcome>,
      PlayError>
   {
      if let Ok(state) = self.state.try_borrow_mut() {
         if state.is_in_progress {
            return Err(PlayError::AlreadyStarted);
         }
      }
      else {
         return Err(PlayError::InUse);
      }

      let run = move |co : Co<Game::Event, Game::Input>| Game::play(self.clone(), co);
      Ok(Gen::new(run))
   }

   /// Copies the game state out of the [`Host`]. Note that this is **only** available for game states implementing the
   /// [`Copy`] trait.
   pub fn game(&self) -> Game where
      Game : Copy,
   {
      self.with_game(|game| *game)
   }

   /// Grants temporary read access to the shared game state via a [`FnOnce`] transaction. 
   /// 
   /// # Safety
   /// This function will panic if the game state is already being accessed, i.e. if a transaction attempts to call
   /// [`with_game`](Self::with_game) or [`with_game_mut`](Self::with_game_mut) from inside itself.
   /// 
   pub fn with_game<F, R>(&'_ self, transact : F) -> R where
      for<'r> F : FnOnce(Ref<'r, Game>) -> R
   {
      let borrowed = Ref::map((*self.state).borrow(), |state| &state.game);
      transact(borrowed)
   }

   /// Grants temporary write access to the shared game state via a [`FnOnce`] transaction. 
   /// 
   /// # Safety
   /// This function will panic if the game state is already being accessed, i.e. if a transaction attempts to call
   /// [`with_game`](Self::with_game) or [`with_game_mut`](Self::with_game_mut) from inside itself.
   /// 
   pub fn with_game_mut<F, R>(&'_ self, transact : F) -> R where
      for<'r> F : FnOnce(RefMut<'r, Game>) -> R
   {
      let borrowed = RefMut::map((*self.state).borrow_mut(), |state| &mut state.game);
      transact(borrowed)
   }
}

impl<Game> Clone for Host<Game> where
   Game : Play,
{
   fn clone(&self) -> Self {
      Self { state: self.state.clone() }
   }
}