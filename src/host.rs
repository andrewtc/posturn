// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod setup;

#[cfg(test)]
mod tests;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
pub use setup::*;
use crate::module::{self, private, CtxMut, Registry};

/// Error returned by [`Host::try_advance`] .
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AdvanceError {
    /// Returned when a [`Host`] is advanced after the game previously panicked.
    /// This is an indicator that the `Host` is poisoned and the game needs to be restarted.
    Poisoned,

    /// Returned when the game has already ended and can no longer be advanced.
    GameOver,
}

impl std::error::Error for AdvanceError{ }

impl fmt::Display for AdvanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdvanceError::Poisoned => write!(f, "The game previously panicked and cannot be advanced. HINT: This is an indicator that the Host is poisoned and must be recreated."),
            AdvanceError::GameOver => write!(f, "The game ended and can no longer be advanced. HINT: Create a new Host in order to start a new game"),
        }
    }
}

/// Wrapper for a game `async fn`. Allows advancing the game as well as borrowing [`Module`] data.
pub struct Host<Outcome> {
    registry: Rc<Registry>,
    game: Option<Pin<Box<dyn Future<Output = Outcome>>>>,
    poisoned: bool,
}

impl<Outcome> Host<Outcome>
where
    Outcome: 'static,
{
    /// Consumes the provided [`Setup`] and sets up a new game with it.
    /// The game will **not** start until it is polled for the first time with [`advance`].
    pub fn new<S>(setup : S) -> Self
    where
        S: Setup<Outcome = Outcome>,
    {
        // Set up the game.
        let mut registry = Registry::new();
        let play = setup.set_up(&mut registry);

        // Freeze the Registry so that it can only be accessed via a Ctx/CtxMut or the Host.
        let registry = Rc::new(registry);
        let ctx_mut = CtxMut::new(Rc::clone(&registry));

        // Create the Future for the game, but don't poll it just yet.
        // The first call to Host::advance will start the game.
        let game = Box::pin(play(ctx_mut));

        Self {
            registry,
            game: Some(game),
            poisoned: false,
        }
    }

    /// Same as [`advance`] but returns returns [`Poisoned`] if the game previously panicked.
    pub fn try_advance(&mut self) -> Result<Poll<Outcome>, AdvanceError> {
        if self.poisoned {
            // The game previously panicked and can't be advanced further.
            return Err(AdvanceError::Poisoned);
        }

        let game = self.game.as_mut()
            .ok_or(AdvanceError::GameOver)?;

        // NOTE: The poisoned flag will stay set if polling the game function panics, guarding against future advances.
        // This is necessary because it is possible to catch the panic with catch_unwind.
        self.poisoned = true;
        let mut poll_ctx = Context::from_waker(Waker::noop());
        let poll = game.as_mut().poll(&mut poll_ctx);

        // TODO: Support awaiting `async` tasks that are not events, e.g. background tasks initiated by the game.
        assert!(!poll.is_pending(), "Game is awaiting a future not created by posturn. This is an error because it would cause the game to pause indefinitely. HINT: Remove the await");

        // NOTE: This MUST happen after any potential panics.
        self.poisoned = false;

        if poll.is_ready() {
            // The game Future has returned, so clear it.
            self.game.take();
        }

        Ok(poll)
    }

    /// Continues the game, returning an [`Outcome`] when the game is finally over.
    /// Each time this function returns without an [`Outcome`], the game is paused waiting for input.
    /// This allows the host application to redraw, animate, etc. in response to events until ready to continue.
    ///
    /// # Panics
    /// If the game previously panicked, this function will panic without advancing.
    /// See [`try_advance`](Self::try_advance) for a version of this function which returns an error instead if the game panics.
    pub fn advance(&mut self) -> Poll<Outcome> {
        self.try_advance()
            .unwrap_or_else(|error| panic!("Failed to advance Host: {}", error))
    }

    /// Returns `true` if the [`Host`] can no longer be advanced due to being poisoned.
    pub fn is_poisoned(&self) -> bool { self.poisoned }
}

impl<Outcome> private::AsRegistry for Host<Outcome>
where
    Outcome: 'static,
{
    fn as_registry(&self) -> &Registry { &self.registry }
}

impl<Outcome> module::Borrow for Host<Outcome>
where
    Outcome: 'static,
{ }
