// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::any::{type_name, Any};
use std::cell::{Ref, RefMut};

mod ctx;
pub use ctx::{Ctx, CtxMut};

mod registry;
pub use registry::Registry;

#[cfg(test)]
mod tests;

pub(crate) mod private {
    pub trait AsRegistry {
        fn as_registry(&self) -> &super::Registry;
    }
}

/// Trait which allows borrowing [`Module`]s by type.
pub trait Borrow: private::AsRegistry {
    /// Looks up a [`Module`] by its type, returning a reference if it exists.
    /// If no `Module` of the specified type was registered when the game was set up, returns [`None`].
    fn try_borrow<M>(&self) -> Option<Ref<'_, M>>
    where
        M: Module,
    {
        let cell = self.as_registry().module_cell::<M>()?;
        Some(Ref::map(cell.borrow(), |module| {
            // NOTE: This is guaranteed to succeed, as we always key Modules by their exact TypeId.
            (module as &dyn Any).downcast_ref::<M>().unwrap()
        }))
    }

    /// Same as [`try_borrow`](Self::try_borrow), but panics if no matching [`Module`] is registered.
    fn borrow<M>(&self) -> Ref<'_, M>
    where
        M: Module,
    {
        self.try_borrow::<M>().unwrap_or_else(|| {
            panic!(
                "Cannot borrow module of type '{}'. No such module is registered",
                type_name::<M>()
            )
        })
    }
}

pub trait BorrowMut: Borrow {
    /// Looks up a [`Module`] by its type, returning a mutable reference if it exists.
    /// If no `Module` of the specified type was registered when the game was set up, returns [`None`].
    fn try_borrow_mut<M>(&self) -> Option<RefMut<'_, M>>
    where
        M: Module,
    {
        let cell = self.as_registry().module_cell::<M>()?;
        Some(RefMut::map(cell.borrow_mut(), |module| {
            // NOTE: This is guaranteed to succeed, as we always key Modules by their exact TypeId.
            (module as &mut dyn Any).downcast_mut::<M>().unwrap()
        }))
    }

    /// Same as [`try_borrow_mut`](Self::try_borrow_mut), but panics if no matching [`Module`] is registered.
    fn borrow_mut<M>(&self) -> RefMut<'_, M>
    where
        M: Module,
    {
        self.try_borrow_mut::<M>().unwrap_or_else(|| {
            panic!(
                "Cannot borrow module of type '{}'. No such module is registered",
                type_name::<M>()
            )
        })
    }
}

/// A composable piece of game behavior, which can be registered with a game host.
/// Each `Module` is intended to be used to add new pieces of functionality to the game.
/// This is usually in the form of new sets of rules (e.g. for an expansion to the base game)
/// but can also be more granular, e.g. a `tic_tac_toe::Board` module which holds X's and O's.
/// Any state that is visible to both the game and the host application (e.g. the UI) must be exposed in this way.
pub trait Module: Any {
    /// The display name of the [`Module`], which is intended for dev purposes.
    /// Defaults to the type name, but can be customized. **Not** guaranteed to be unique.
    fn name(&self) -> &str {
        type_name::<Self>()
    }
}
