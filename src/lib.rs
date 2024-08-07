// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT

pub mod host;
pub use host::Host;

#[cfg(test)]
mod tests;

use std::future::Future;

pub use genawaiter;
use genawaiter::rc::Co;

/// Allows a game coroutine to yield [`Event`s](Play::Event) to be processed by the UI layer. Also allows read/write
/// access to the game state via the shared [`host`].
pub struct Context<Game> where
   Game : Play,
{
   pub host : Host<Game>,
   co : Co<Game::Event, Game::Input>,
}

impl<Game> Context<Game> where
   Game : Play,
{
   /// Raises an [`Event`](Play::Event) to be processed outside of the turn-based game loop. The game itself will have
   /// the chance to react with [`handle_event`](Play::handle_event) before broadcasting.
   /// 
   /// ⚠️ **IMPORTANT:** Please remember to immediately `await` the `Future` returned by this function.
   /// 
   pub fn yield_event(&self, event : Game::Event) -> impl Future<Output = Game::Input> + '_ {
      // Allow the game to update itself in response to the event being emitted.
      self.host.process_event(&event);

      // "Yield" the event by returning a Future that will wait for the coroutine to be resumed.
      self.co.yield_(event)
   }
}

/// Trait defining a game that can be played via a [`Host`].
pub trait Play : Sized {
   /// An event emitted by the game to signal that the game state has been updated in some way and is waiting to be
   /// resumed.
   type Event : Sized;

   /// Player input which **must** be supplied whenever the game is resumed on a player's turn.
   type Input : Sized;

   /// The type representing the final outcome of the game. This will be returned from the "host" [`Coroutine`]
   /// whenever the game is finally over.
   type Outcome : Sized;

   /// Coroutine responsible for running the game. Think of this as the `main` function of the game. The implementation
   /// can use [`Context::yield_event`] to emit an [`Event`](Play::Event) whenever something happens that needs to be
   /// presented to the player. Doing this will yield control back to the main application (and typically the UI layer)
   /// to respond to the event in some way, e.g. by playing an animation, triggering a sound effect, asking the player
   /// for input, etc. Execution will be paused until execution is resumed by the main application using
   /// [`Co::resume_with`]
   fn play(ctx : Context<Self>) -> impl Future<Output = Self::Outcome>;

   /// Allows the game to update state in response to an event emitted internally from [`play`](Play::play) or supplied
   /// externally via [`Host::process_event`].
   fn handle_event(&mut self, _event : &<Self as Play>::Event) { }
}
