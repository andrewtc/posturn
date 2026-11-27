// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
use std::rc::Rc;

#[derive(Copy, Clone, Debug, PartialEq)]
struct CopyModule(u8);

impl Module for CopyModule {}

#[derive(Clone, Debug, PartialEq)]
struct CloneModule(Vec<u8>);

impl Module for CloneModule {}

#[test]
#[should_panic(expected = "CopyModule'. Another module of the same type")]
fn inserting_conflicting_module_panics() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(123));
    registry.insert(CopyModule(45)); // panics
}

#[test]
fn borrow_returns_inserted_module() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(123));
    assert_eq!(*registry.borrow::<CopyModule>(), CopyModule(123));
}

#[test]
fn borrow_mut_returns_module_ref() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(1));
    registry.borrow_mut::<CopyModule>().0 += 1;
    assert_eq!(registry.borrow::<CopyModule>().0, 2);
}

#[test]
fn ctx_borrow_mut_returns_module_ref() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(1));
    let ctx_mut = CtxMut::new(Rc::new(registry));
    ctx_mut.borrow_mut::<CopyModule>().0 += 1;
    assert_eq!(ctx_mut.borrow::<CopyModule>().0, 2);
}

#[test]
fn registry_permits_multiple_borrows() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(123));
    registry.insert(CloneModule(vec![1, 2, 3]));
    registry
        .borrow_mut::<CloneModule>()
        .0
        .push(registry.borrow::<CopyModule>().0);
    assert_eq!(
        *registry.borrow::<CloneModule>(),
        CloneModule(vec![1, 2, 3, 123])
    );
}

#[test]
#[should_panic(expected = "CopyModule'. No such module")]
fn borrow_panics_if_missing() {
    Registry::new().borrow::<CopyModule>(); // panics
}

#[test]
fn try_borrow_returns_none_if_missing() {
    assert!(Registry::new().try_borrow::<CopyModule>().is_none());
}

#[test]
fn try_borrow_mut_returns_none_if_missing() {
    assert!(Registry::new().try_borrow_mut::<CopyModule>().is_none());
}

#[test]
fn ctx_returns_same_module() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(123));
    assert_eq!(
        *registry.ctx().borrow::<CopyModule>(),
        *registry.borrow::<CopyModule>()
    );
}

#[test]
fn ctx_can_be_copied() {
    let mut registry = Registry::new();
    registry.insert(CopyModule(123));
    let ctx = registry.ctx();
    let copy = ctx;
    assert_eq!(*ctx.borrow::<CopyModule>(), *copy.borrow::<CopyModule>());
}
