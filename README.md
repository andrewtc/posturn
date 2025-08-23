<!--
SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

[![Build Status](https://github.com/andrewtc/posturn/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/andrewtc/posturn/actions/workflows/rust.yml?branch=main)
[![Docs Status](https://docs.rs/posturn/badge.svg)](https://docs.rs/posturn)
[![REUSE status](https://api.reuse.software/badge/github.com/andrewtc/posturn)](https://api.reuse.software/info/github.com/andrewtc/posturn)
[![Discord](https://img.shields.io/discord/1408632789347336316?logo=discord)](https://discord.gg/QyWYW6WB)

# 🏰 posturn 🎮
Build turn-based games with `async` Rust

This crate offers a simple way to model turn-based games. Instead of writing a monolithic state
machine, why not a [`Coroutine`](https://doc.rust-lang.org/std/ops/trait.Coroutine.html)
instead?

`posturn` allows you to write your turn-based game logic as an `async` function that emits events back to calling code.
When an event is emitted, the game enters a wait-for-input state using `await`, pausing and returning control to the UI
layer. The UI can handle the event and re-render itself however it wishes. When ready to continue, the UI resumes the
game. This calls back into the coroutine, providing all player input necessary to take the next turn.

This approach has the advantage of maintaining a clean separation between turn-based game logic and the UI layer. A game
written in this manner can be used with _any_ frontend capable of processing events emitted by the game. This makes it
possible to write, for instance, two game clients in entirely different game engines with no changes to the underlying
game logic.

For a very simple example, see [`RoShamBo`](/src/tests.rs) in the `tests` module.

For a complete game built with `posturn`, see [`examples/tui`](/examples/tui/README.md).

## 📃 Usage
To use `posturn`, simply add this to your `Cargo.toml`:

```toml
[dependencies]
posturn = "0.3.0"
```

Alternatively, run the following from your crate root:

```ps1
cargo add posturn@0.3.0
```

## ⚠️ Compatibility
This crate currently depends on [`genawaiter`](https://docs.rs/genawaiter/latest/genawaiter/) to provide a **stable**
implementation of Rust coroutines. Once the [`Coroutine`](https://doc.rust-lang.org/std/ops/trait.Coroutine.html) trait
has been stabilized ([RFC 2033](https://github.com/rust-lang/rust/issues/43122)), future versions of `posturn` may move
in the direction of using the `std` implementation, gated by a `feature` flag.

For now, the dependency on `genawaiter` is required. If you need `std` support for a `nightly` project, please see
[this](https://github.com/andrewtc/posturn/issues/1) issue or open a PR on GitHub.

## ⚖️ License
All Rust code is licensed under the [MIT](/LICENSES/MIT.txt) license.

Various other files (e.g. this `README`) are licensed under one of the following:
 - [Creative Commons Attribution-ShareAlike 4.0 International](/LICENSES/CC-BY-SA-4.0.txt)
 - [CC0 1.0 Universal](/LICENSES/CC0-1.0.txt)

`posturn` aims to be [REUSE compliant](https://reuse.software/). The `SPDX-License-Identifier` at the top of each file shows which license is associated with it.
