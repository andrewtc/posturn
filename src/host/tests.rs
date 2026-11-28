// SPDX-FileCopyrightText: 2026 Andrew T. Christensen <andrew@andrewtc.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::panic::AssertUnwindSafe;
use std::task::Poll;
use crate::host::{AdvanceError, Host, Setup};
use crate::module::{Borrow, BorrowMut, CtxMut, Module, Registry};

struct GuessingGameSetup<Game>{
    magic_number: i32,
    guessing_game: Game,
}

impl<Game> Setup for GuessingGameSetup<Game>
where
    Game: AsyncFnOnce(CtxMut) -> bool + 'static,
{
    type Outcome = bool;
    fn set_up(self, registry: &mut Registry) -> impl AsyncFnOnce(CtxMut) -> Self::Outcome + 'static {
        registry.insert(MagicNumber(self.magic_number));
        self.guessing_game
    }
}

struct MagicNumber(i32);
impl Module for MagicNumber { }

async fn boring_guessing_game(ctx: CtxMut) -> bool {
    let magic_number = ctx.borrow::<MagicNumber>().0;
    const GUESS: i32 = 123;
    GUESS == magic_number
}

async fn double_number_guessing_game(ctx: CtxMut) -> bool {
    let magic_number = &mut ctx.borrow_mut::<MagicNumber>().0;
    const GUESS: i32 = 42;
    *magic_number *= 2;
    GUESS == *magic_number
}

async fn never_ending_guessing_game(ctx: CtxMut) -> bool {
    let magic_number = ctx.borrow::<MagicNumber>().0;

    // Await a non-event future, which should cause a panic.
    let guess = std::future::pending::<i32>().await;
    guess == magic_number
}

#[test]
fn game_with_no_awaits_returns_ready() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 123,
        guessing_game: boring_guessing_game,
    });

    assert_eq!(host.advance(), Poll::Ready(true));
}

#[test]
fn game_can_write_modules() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 21,
        guessing_game: double_number_guessing_game,
    });

    assert_eq!(host.borrow::<MagicNumber>().0, 21);
    assert_eq!(host.advance(), Poll::Ready(true));
    assert_eq!(host.borrow::<MagicNumber>().0, 42);
}

#[test]
fn try_advancing_finished_game_returns_game_over() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 123,
        guessing_game: boring_guessing_game,
    });

    assert_eq!(host.advance(), Poll::Ready(true));
    assert_eq!(host.try_advance(), Err(AdvanceError::GameOver));
}

#[test]
#[should_panic(expected = "game ended")]
fn advancing_finished_game_panics() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 123,
        guessing_game: boring_guessing_game,
    });

    assert_eq!(host.advance(), Poll::Ready(true));
    let _ = host.advance();
}

#[test]
#[should_panic(expected = "awaiting a future not created by posturn")]
fn game_awaiting_foreign_future_panics() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 123,
        guessing_game: never_ending_guessing_game,
    });

    let _ = host.advance(); // panics
}

#[test]
fn game_panic_poisons_host() {
    let mut host = Host::new(GuessingGameSetup{
        magic_number: 123,
        guessing_game: never_ending_guessing_game,
    });

    std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = host.advance(); // panics
    })).expect_err("Expected game to panic");

    assert!(host.is_poisoned());
    assert_eq!(host.try_advance(), Err(AdvanceError::Poisoned));
}
