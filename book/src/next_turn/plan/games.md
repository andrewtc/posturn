# Games are just `async fn`s

**Decided:** the `Play` trait is removed. A game is any `async fn` (or any `Future`) that the user of the library
writes. Its arguments are whatever it needs, typically some setup data and access to the [`Host`](./host.md), and its
output is whatever the game produces when it finishes:

```rust,ignore
async fn play(setup: GameSetup, host: &Host) -> Outcome {
   // Register listeners, insert modules, then run turns by emitting events...
}
```

A game that never ends can return `!` (or `std::convert::Infallible` on stable).

Consequences:

- **There is no `Game` type parameter anywhere.** `Host` is not generic. `Event`, `Input`, and `Outcome` associated
  types are gone; their roles are taken by individual [event types](./events.md) and the future's own `Output`.
- **The game owns its private state.** Local variables in the `async fn` are the game's private state. Only state
  meant to be **inspected by the UI** is published to the `Host` as a [module](./host.md).
- **`handle_event` is gone.** Reacting to events is done by [listeners](./events.md), of which there can be many.
- **Sub-games compose naturally.** A battle minigame is just another `async fn` that the overworld `await`s:
  `let outcome = battle::play(host).await;`.
