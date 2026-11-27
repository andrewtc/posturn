// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::module::{CtxMut, Registry};

/// Implemented for any structure that can be used to start a new game `async fn`.
pub trait Setup {
    /// The result finally returned by the game `async fn` when the game is complete.
    type Outcome: 'static;

    /// Transforms the [`Setup`] into a game `async fn` that can be polled to advance the game.
    fn set_up(self, registry: &mut Registry)
        -> impl AsyncFnOnce(CtxMut) -> Self::Outcome + 'static;
}
