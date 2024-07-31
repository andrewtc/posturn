<!--
SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 🛞 posturn 🎮
Build turn-based games with `async` Rust

This crate offers a simple way to create complex turn-based games. Instead of modeling the game as a monolithic state
machine with transitions, why not write a [`Coroutine`](https://doc.rust-lang.org/std/ops/trait.Coroutine.html)?

## Usage
To use `posturn`, simply add this to your `Cargo.toml`:

```toml
[features]
posturn = "0.1.0"
```

Alternatively, run the following from your crate root:

```ps1
cargo add posturn@0.1.0
```

## ⚠️ Compatibility
This crate currently depends on [`genawaiter`](https://docs.rs/genawaiter/latest/genawaiter/) to provide a **stable**
implementation of Rust coroutines. Once the [`Coroutine`](https://doc.rust-lang.org/std/ops/trait.Coroutine.html) trait
has been stabilized ([RFC 2033](https://github.com/rust-lang/rust/issues/43122)), future versions of `posturn` may move
in the direction of using the `std` implementation, gated by a `feature` flag.

For `0.1.0`, the dependency on `genawaiter` is required. If you need `std` support for a `nightly` project, please see
[this](https://github.com/andrewtc/posturn/issues/1) issue or open a PR on GitHub.
