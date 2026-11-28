// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An [`Event`] is a simple payload emitted by a game `async fn` whenever it needs to:
//! 1. Cause side effects in the host application, e.g. play an animation, sound effect, etc.
//! 2. Trigger behavior in a [`Module`], e.g. a special rule that applies when an enemy takes damage

/// Marker `trait` for an event payload that can be yielded from a game.
/// Every type of event that can be emitted from a running game **must** implement this `trait`.
pub trait Event : 'static {
    /// The type used to represent the precedence of listeners for this [`Event`].
    /// Each listener is registered with a priority and listeners are processed in **descending order**.
    /// In other words, a listener with the highest priority always handles the [`Event`] first.
    type Priority: Ord + 'static;
}
