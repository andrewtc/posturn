// SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::future::Future;
use std::pin::Pin;

pub mod event;
pub mod host;
pub mod module;

/// Type alias for a pinned, boxed [`Future`] that lives at least as long as `'a`.
/// This is used to store [`Event`] listeners in such a way that the [`Host`] can visit listeners
/// recursively while the event chain is being processed.
pub(crate) type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
