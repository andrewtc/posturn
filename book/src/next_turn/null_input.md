# Null input

The game only expects meaningful input **after** it has prompted us for input. This is because the game is really yielding control _in response_ to an event which requires input. This means we have to supply "null" input, e.g. `Default::default()`, `None`, in cases where the game has **not** (yet) prompted the player but we want to resume the game, e.g. on the first `resume_with` in the example above.

It would be nice to reverse this. Instead of requiring the game loop to know when to supply invalid inputs, we should invert control. We should be able to resume the game without arguments in all cases, and the game should _ask_ for input when it needs it.
