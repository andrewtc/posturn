# How the pieces fit

This page is a map of every type in the plan and how they relate. Each type is described in more detail on its own
page; the diagrams are the big picture.

**Decided:** the types are split across four Rust modules, each with its own color: `posturn::host` (purple),
`posturn::module` (green), `posturn::event` (blue), and `posturn::request` (orange).

- **API at a glance** is a simplified overview of the high-level API structure. It shows only the main **public**
  types that connect the modules, by name. Dotted arrows are dependencies (one type uses, creates, or hands out the
  other); solid arrows mean one type holds or refers to the other.
- **Each module's section** then has a full class diagram of all of that module's types, public and private, and
  the relationships between them.

**Names** don't repeat their module (per Clippy's `module_name_repetitions`), so types outside their own module's
diagram are written module-qualified: `event::Listener`, `request::Slot`, `module::Borrow`. Within a module's own
diagram, the bare name is used.

**Visibility** follows UML notation: a `-` prefix marks something private. Members already use it; types that are
private to the crate (never named by users of `posturn`) are also drawn with a `-` before their name, a darker fill,
and a dashed outline.

## API at a glance

```mermaid
%%{init: {"flowchart": {"nodeSpacing": 15, "rankSpacing": 25}}}%%
flowchart LR

    %% Layout: the order of declarations and edges was tuned by measuring rendered variants for size and edge
    %% crossings. Reordering lines here can change the layout noticeably.

    subgraph module ["posturn::module"]
        Ctx["Ctx#lt;'a#gt;"]
        CtxMut["CtxMut"]
        Registry["Registry"]
    end
    subgraph event ["posturn::event"]
        EventHandle["event::Handle"]
        Event["Event"]
    end
    subgraph request ["posturn::request"]
        Request["Request"]
        RequestHandle["request::Handle#lt;T#gt;"]
    end
    subgraph host ["posturn::host"]
        Setup["Setup"]
        Host["Host#lt;Outcome#gt;"]
    end

    Host -- "shares via Rc" --> Registry
    Host -- "owns (via game)" --> CtxMut
    CtxMut -. "emits" .-> Event
    Host -. "gives UI callbacks" .-> Ctx
    CtxMut -. "requests" .-> Request
    Setup -. "set_up edits" .-> Registry
    Host -. "on_request gives" .-> RequestHandle
    Host -. "on_event gives" .-> EventHandle

    style host fill:none,stroke:#b48cf0,stroke-width:4px,stroke-dasharray:1 9,stroke-linecap:round
    style module fill:none,stroke:#7fd082,stroke-width:4px,stroke-dasharray:1 9,stroke-linecap:round
    style event fill:none,stroke:#6cb4f0,stroke-width:4px,stroke-dasharray:1 9,stroke-linecap:round
    style request fill:none,stroke:#f0a060,stroke-width:4px,stroke-dasharray:1 9,stroke-linecap:round
    style Setup fill:#4b2e6b,stroke:#b48cf0,stroke-width:2px,color:#ffffff
    style Host fill:#4b2e6b,stroke:#b48cf0,stroke-width:2px,color:#ffffff
    style CtxMut fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style Registry fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style Ctx fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style Event fill:#1e4f7a,stroke:#6cb4f0,stroke-width:2px,color:#ffffff
    style EventHandle fill:#1e4f7a,stroke:#6cb4f0,stroke-width:2px,color:#ffffff
    style Request fill:#7a3e14,stroke:#f0a060,stroke-width:2px,color:#ffffff
    style RequestHandle fill:#7a3e14,stroke:#f0a060,stroke-width:2px,color:#ffffff
```

## `posturn::host`

```mermaid
%%{init: {"class": {"nodeSpacing": 25, "rankSpacing": 20}}}%%
classDiagram
    direction TB

    class Setup {
        <<trait>>
        type Outcome
        +set_up(self, &mut Registry) AsyncFnOnce(CtxMut)
    }

    class Host~Outcome~ {
        -registry: Rc~Registry~
        -game: Option~Pin~Box~dyn Future~~~
        -poisoned: bool
        -started: bool
        +new(setup: Setup) Host
        +on_event(callback)
        +on_request(callback)
        +step() Poll~Outcome~
        +is_poisoned() bool
    }

    class Suspension["-Suspension"] {
        <<enum>>
        Event(event_type, done: event::Slot)
        Request(request_type, status)
    }

    Host ..> Setup : new() consumes

    style Host fill:#4b2e6b,stroke:#b48cf0,stroke-width:2px,color:#ffffff
    style Setup fill:#4b2e6b,stroke:#b48cf0,stroke-width:2px,color:#ffffff
    style Suspension fill:#2a1a3d,stroke:#b48cf0,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
```

- **`Host<Outcome>`** (UI side). Created from a `Setup`. Holds one `Rc<Registry>` and the game future (pinned and boxed,
  `None` once finished). The UI registers its callbacks on it, reads modules through it between turns, and calls
  `step()` to advance the game. Implements `module::Borrow` only: the UI can't write modules. Module borrows take
  `&self` and `step()` takes `&mut self`, so the UI can't hold a borrow across a step. A panic during `step()`
  poisons it (`is_poisoned()`), and every later `step()` panics. See [`Host`, contexts, and modules](./host.md) and
  [Driving the game](./driving.md).
- **`Suspension`** (enum). What the game is currently parked on: an event (its `event::Slot`) or a request (the
  type-erased status of its `request::Slot`), plus the message's type name for error messages. Stored in the
  `Registry`; `Host::step()` checks it to decide whether to do nothing, panic, or poll. See
  [Under the hood](./coroutines.md#current-suspension).
- **`Setup`** (trait). Implemented by a struct holding everything needed to start one game. `set_up` registers the
  game's modules and listeners and returns the game as an async function. `Host::new` takes a `Setup` by value. See
  [Setting up a game](./games.md).

## `posturn::module`

```mermaid
%%{init: {"class": {"nodeSpacing": 10, "rankSpacing": 20}}}%%
classDiagram
    direction TB

    %% Layout: the order of declarations and the direction of each arrow (e.g. `A <.. B` instead of `B ..> A`, which
    %% means the same thing) were tuned by measuring rendered variants for size and edge crossings. Reordering lines
    %% or flipping arrows here can change the layout noticeably.

    class Module {
        <<trait>>
        +name() str = type_name
    }

    class Borrow {
        <<sealed trait>>
        +borrow~M~() Ref~M~
        +try_borrow~M~() Option~Ref~M~~
        +module~M: Copy~() M
        +clone_module~M: Clone~() M
    }

    class TypeMap["-TypeMap"] {
        -0: BTreeMap#lt;TypeId, Box#lt;dyn Any#gt;#gt;
        +insert~T~(value: T) bool
        +get~T~() Option~&T~
        +get_mut~T~() Option~&mut T~
    }

    class BorrowMut {
        <<sealed trait>>
        +borrow_mut~M~() RefMut~M~
        +try_borrow_mut~M~() Option~RefMut~M~~
    }

    class Registry {
        -modules: TypeMap
        -event_listeners: TypeMap
        -request_listeners: TypeMap
        -event_callbacks: RefCell~TypeMap~
        -request_callbacks: RefCell~TypeMap~
        -current: RefCell~Option~Suspension~~
        +ctx() Ctx
        +insert(module)
        +on_event(listener)
        +on_request(listener)
    }

    class Ctx~'a~ {
        <<Copy>>
        -0: &'a Registry
    }

    class CtxMut {
        -0: Rc~Registry~
        +ctx() Ctx
        +emit(event)
        +request(request) R::Response
    }

    Module "*" --o "1" Registry : keyed by type
    Borrow <|-- BorrowMut : extends
    Registry ..|> BorrowMut
    Registry <-- CtxMut : shares via Rc
    BorrowMut <|.. CtxMut
    TypeMap "5" --* "1" Registry : modules, listeners, callbacks
    Registry <-- Ctx : borrows
    Ctx ..|> Borrow : read-only

    style Registry fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style CtxMut fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style Module fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style Borrow fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style BorrowMut fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
    style TypeMap fill:#1a361b,stroke:#7fd082,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
    style Ctx fill:#2e5e30,stroke:#7fd082,stroke-width:2px,color:#ffffff
```

See [Looking up modules](./host.md#looking-up-modules).

- **`Registry`.** Holds all shared state: five `TypeMap`s (modules, game-listener chains, and UI callbacks) plus the
  current suspension; see [Inside the `Registry`](./host.md#inside-the-registry). Users only see it as `&mut Registry`
  during `set_up`, where they `insert` modules and register listeners with `on_event` / `on_request`. `Host::new` then
  moves it into an `Rc`, which freezes it: no `&mut` access ever again. Implements both module traits.
- **`CtxMut`** (game side). A non-`Clone` newtype around the other `Rc<Registry>`; only `Host::new` creates one. It is
  handed to the game's async function by value. Implements both module traits. Module borrows take `&self`;
  everything that can suspend (`emit`, `request`, and `run` on a `event::Next` / `request::Next`) takes `&mut self`, so
  the game can't hold a borrow across a suspension.
- **`Module`** (trait). A piece of game state published so that the UI can inspect it, such as `OverworldModule` or
  `BattleModule`. One instance per type, each stored as a `RefCell<M>` in the `Registry`. Its only method, `name()`,
  is for diagnostics and defaults to the type name, so `impl Module for MyModule {}` is usually enough.
- **`module::Borrow`** (sealed trait). Read access: `borrow`, `try_borrow`, `module`, `clone_module`. Implemented by
  `Registry`, `CtxMut`, `Ctx`, and `Host`, so all of them look up modules the same way.
- **`module::BorrowMut`** (sealed trait, extends `module::Borrow`). Write access: `borrow_mut`, `try_borrow_mut`.
  Implemented by `Registry` and `CtxMut` only.
- **`TypeMap`** (private). A map holding at most one value of each type, keyed by that type (e.g. one
  `RefCell<M>` per module type), built on `BTreeMap<TypeId, Box<dyn Any>>`. The `Registry` holds five of them. See
  [`TypeMap`](./host.md#typemap).
- **`Ctx<'a>`.** A read-only, `Copy` view of the modules, obtained with `.ctx()` from a `Registry` or `CtxMut`, and
  handed to every UI callback by the `Host`. Implements `module::Borrow` only. Borrows its source, so it inherits the
  source's restrictions under the [borrowing rule](./host.md#the-borrowing-rule).

## `posturn::event`

```mermaid
%%{init: {"class": {"nodeSpacing": 40, "rankSpacing": 35}}}%%
classDiagram
    direction TB

    %% Layout: the order of declarations and the direction of each arrow (e.g. `A <.. B` instead of `B ..> A`, which
    %% means the same thing) were tuned by measuring rendered variants for size and edge crossings. Reordering lines
    %% or flipping arrows here can change the layout noticeably.

    class Event {
        <<marker trait>>
    }

    class Handle {
        -done: Slot
        drop() resumes
    }

    class Listener["-Listener#lt;E#gt;"] {
        <<Rc dyn Fn>>
        +call(&mut CtxMut, E, Next~E~)
    }

    class Future["-Future"] {
        <<impl std::future::Future>>
        type Output = ()
        -done: Slot
    }

    class Next~E~ {
        -chain: Chain~E~
        -remaining: usize
        -ui: Option~Listener~E~~
        +run(&mut CtxMut, event)
    }

    class Slot["-Slot"] {
        <<Rc Cell bool>>
    }

    Next --> Listener : calls the next one
    Handle --> Slot : sets on drop
    Next <.. Listener : receives
    Slot <-- Future : waits on

    style Event fill:#1e4f7a,stroke:#6cb4f0,stroke-width:2px,color:#ffffff
    style Listener fill:#112d46,stroke:#6cb4f0,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
    style Next fill:#1e4f7a,stroke:#6cb4f0,stroke-width:2px,color:#ffffff
    style Handle fill:#1e4f7a,stroke:#6cb4f0,stroke-width:2px,color:#ffffff
    style Future fill:#112d46,stroke:#6cb4f0,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
    style Slot fill:#112d46,stroke:#6cb4f0,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
```

Events and requests are processed by separate types throughout. See
[Events, requests, and listeners](./events.md) and [Under the hood](./coroutines.md).

- **`Event`** (marker trait). Declares a type as an event. Events are broadcast with `ctx.emit(ev).await`, which
  returns `()`.
- **`event::Listener<E>`** (`Rc<dyn Fn(&mut CtxMut, E, event::Next<E>) -> LocalBoxFuture<()>>`). What an async function
  or async closure registered with `Registry::on_event` is wrapped into. `Host::on_event` callbacks are wrapped into
  one too.
- **`event::Next<E>`.** The rest of the chain: a snapshot of the game listeners, how many remain, and the UI callback.
  `next.run(ctx, ev)` calls the next one; at the end of the chain it simply returns.
- **`event::Handle`.** What a `Host::on_event` callback receives. It has no methods: dropping it lets the game continue.
- **`event::Future`** (`Future<Output = ()>`). What the UI callback's listener awaits; ready once the `event::Handle` is
  dropped.
- **`event::Slot`** (`Rc<Cell<bool>>`). Shared by an `event::Handle` and its `event::Future`; set to `true` on drop.

## `posturn::request`

```mermaid
%%{init: {"class": {"nodeSpacing": 10, "rankSpacing": 35}}}%%
classDiagram
    direction TB

    %% Layout: the order of declarations and the direction of each arrow (e.g. `A <.. B` instead of `B ..> A`, which
    %% means the same thing) were tuned by measuring rendered variants for size and edge crossings. Reordering lines
    %% or flipping arrows here can change the layout noticeably.

    class Request {
        <<trait>>
        type Response
    }

    class Next~R~ {
        -chain: Chain~R~
        -remaining: usize
        -ui: Option~Listener~R~~
        +run(&mut CtxMut, request) R::Response
    }

    class Future["-Future#lt;T#gt;"] {
        <<impl std::future::Future>>
        type Output = T
        -slot: Slot~T~
    }

    class Slot["-Slot#lt;T#gt;"] {
        <<Rc RefCell State>>
        Waiting
        Filled(T)
        Taken
        Abandoned
    }

    class Listener["-Listener#lt;R#gt;"] {
        <<Rc dyn Fn>>
        +call(&mut CtxMut, R, Next~R~) R::Response
    }

    class Handle~T~ {
        -slot: Slot~T~
        -registry: Weak~Registry~
        +resume(value: T) Result~(), T~
        drop() abandons
    }

    Future --> Slot : takes the value from
    Next <.. Listener : receives
    Slot <-- Handle : fills or abandons
    Next --> Listener : calls the next one

    style Request fill:#7a3e14,stroke:#f0a060,stroke-width:2px,color:#ffffff
    style Listener fill:#46230b,stroke:#f0a060,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
    style Next fill:#7a3e14,stroke:#f0a060,stroke-width:2px,color:#ffffff
    style Handle fill:#7a3e14,stroke:#f0a060,stroke-width:2px,color:#ffffff
    style Future fill:#46230b,stroke:#f0a060,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
    style Slot fill:#46230b,stroke:#f0a060,stroke-width:2px,stroke-dasharray:6 4,color:#c8c8c8
```

- **`Request`** (trait). Implemented by each request type. Its associated `Response` is what
  `ctx.request(req).await` resolves to.
- **`request::Listener<R>`** (`Rc<dyn Fn(&mut CtxMut, R, request::Next<R>) -> LocalBoxFuture<R::Response>>`). What a
  listener registered with `Registry::on_request` is wrapped into. `Host::on_request` callbacks are wrapped into one
  too.
- **`request::Next<R>`.** Like `event::Next`, but `next.run(ctx, req)` returns `R::Response`, and reaching the end of
  the chain panics.
- **`request::Handle<T>`** (`T = R::Response`). What a `Host::on_request` callback receives. It must be consumed with
  `resume(value)`; dropping it without resuming abandons the request, and the next `step()` panics. If the `Host` is
  already gone, `resume` returns `Err(value)` instead.
- **`request::Future<T>`** (`Future<Output = T>`). What the UI callback's listener awaits; takes the value out of the
  slot once it is filled.
- **`request::Slot<T>`** (`Rc<RefCell<request::State<T>>>`). Shared by a `request::Handle` and its `request::Future`:
  `Waiting`, `Filled(T)`, `Taken`, or `Abandoned`.
