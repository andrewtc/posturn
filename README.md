<!--
SPDX-FileCopyrightText: 2024 Andrew T. Christensen <andrew@andrewtc.com>

SPDX-License-Identifier: CC-BY-SA-4.0
-->

[![Build Status](https://github.com/andrewtc/posturn/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/andrewtc/posturn/actions/workflows/rust.yml?branch=main)
[![Docs Status](https://docs.rs/posturn/badge.svg)](https://docs.rs/posturn)
[![REUSE status](https://api.reuse.software/badge/github.com/andrewtc/posturn)](https://api.reuse.software/info/github.com/andrewtc/posturn)

# 🏰 posturn 🎮
Build turn-based games with `async` Rust

This crate offers a simple way to create complex turn-based games. Instead of modeling the game as a monolithic state
machine with transitions, why not write a [`Coroutine`](https://doc.rust-lang.org/std/ops/trait.Coroutine.html)
instead?

`posturn` allows you to model your turn-based game logic as an `async` function that emits events. When an event is
emitted, the turn-based game enters into a wait-for-input state using `await`, pausing the game and returning control
to the UI layer. While paused, the UI can handle the event and re-render itself however it wishes. Once the player is
ready to continue, the UI resumes the turn-based game, supplying any player input necessary to take the next turn.

This approach has the advantage of keeping all UI logic separate from turn-based game code. Theoretically, a game
written in this way could be used with _any_ frontend capable of processing the events emitted by the game.

For a simple example, see the simple [`RoShamBo`](/src/tests.rs) game included with this library.

## 📃 Usage
To use `posturn`, simply add this to your `Cargo.toml`:

```toml
[dependencies]
posturn = "0.2.0"
```

Alternatively, run the following from your crate root:

```ps1
cargo add posturn@0.2.0
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
