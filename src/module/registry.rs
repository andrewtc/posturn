// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{Ctx, Module};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{hash_map, HashMap};

/// Stores [`Module`]s for a game.
/// `Module`s are composable pieces of game functionality which are registered here during setup.
pub struct Registry {
    modules: HashMap<TypeId, Box<RefCell<dyn Module>>>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Registers a new [`Module`] with the [`Registry`] so that it can be accessed from within the game.
    /// Only one `Module` of the same type can be registered with the game.
    pub fn insert<M>(&mut self, module: M)
    where
        M: Module,
    {
        let module_id = TypeId::of::<M>();
        let entry = self.modules.entry(module_id);
        if let hash_map::Entry::Occupied(existing) = entry {
            panic!(
                "Cannot register module '{}'. Another module of the same type is already registered (existing: {}).",
                module.name(),
                existing.get().borrow().name());
        }
        self.modules
            .insert(module_id, Box::new(RefCell::new(module)));
    }

    pub(super) fn module_cell<M>(&self) -> Option<&RefCell<dyn Module>>
    where
        M: Module,
    {
        self.modules.get(&TypeId::of::<M>()).map(Box::as_ref)
    }

    /// Returns a read-only view of the [`Registry`], allowing immutable access to [`Module`]s.
    pub fn ctx(&self) -> Ctx<'_> {
        Ctx(self)
    }
}

impl super::private::AsRegistry for Registry {
    fn as_registry(&self) -> &Registry {
        self
    }
}

impl super::Borrow for Registry {}
impl super::BorrowMut for Registry {}
