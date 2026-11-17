# 🎲 Next Turn

> ## ⚠️ Disclaimer: AI-drafted, temporary documentation
>
> **This section, including the whole of [the plan](./next_turn/plan.md), was drafted by Claude (an AI model) in
> collaboration with the author, and may contain inaccuracies.** Code in these pages is a design sketch, not tested
> code; names, signatures, and behavior may change or turn out to be wrong once they meet a compiler.
>
> **This documentation is temporary.** It will be replaced by real documentation as each feature is implemented.

> This contains documentation regarding the next upcoming release of `posturn`, **v0.4.0**.

This section explores improvements to event and input processing in `posturn`. It starts by looking at the problems
with the current design (null input, input bloat, the conflation of events and input, and a single monolithic game
state) and then lays out [the plan](./next_turn/plan.md) for addressing them.
