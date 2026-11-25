// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::any::{ Any, type_name };

/// A composable piece of game behavior, which can be registered with a game [`Host`].
/// Each `Module` is intended to be used to add new pieces of functionality to the game.
/// This is usually in the form of new sets of rules (e.g. for an expansion to the base game)
/// but can also be more granular, e.g. a `tic_tac_toe::Board` module which holds X's and O's.
/// Any state that is visible to both the game and the host application (e.g. the UI) must be exposed in this way.
pub trait Module : Any {
    /// The display name of the [`Module`], which is intended for dev purposes.
    /// Defaults to the type name, but can be customized. **Not** guaranteed to be unique.
    fn name(&self) -> &str { type_name::<Self>() }
}