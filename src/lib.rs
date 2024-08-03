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
   /// can use [`Co::yield_`] to emit an [`Event`](Play::Event) whenever something happens that needs to be presented
   /// to the player. Doing this will yield control back to the main application (and typically the UI layer) to
   /// respond to the event in some way, e.g. by playing an animation, triggering a sound effect, asking the player for
   /// input, etc. Execution will be paused until execution is resumed by the main application using [`Host`]
   fn play(host : Host<Self>, co : Co<Self::Event, Self::Input>) -> impl Future<Output = Self::Outcome>;
}
