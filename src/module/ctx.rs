// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{private, Borrow, BorrowMut, Registry};
use crate::module::private::AsRegistry;
use std::rc::Rc;

/// Allows temporary, immutable access to game [`Module`]s while the game is suspended.
#[derive(Clone, Copy)]
pub struct Ctx<'a>(pub(super) &'a Registry);

impl private::AsRegistry for Ctx<'_> {
    fn as_registry(&self) -> &Registry {
        self.0
    }
}

impl Borrow for Ctx<'_> {}

pub struct CtxMut(Rc<Registry>);

impl CtxMut {
    pub(crate) fn new(registry: Rc<Registry>) -> Self {
        Self(registry)
    }

    /// Returns a read-only view of [`Module`]s.
    pub fn ctx(&self) -> Ctx<'_> {
        Ctx(self.as_registry())
    }
}

impl private::AsRegistry for CtxMut {
    fn as_registry(&self) -> &Registry {
        &self.0
    }
}

impl Borrow for CtxMut {}
impl BorrowMut for CtxMut {}
