# Do...End Blocks in Silica

## Function Bodies vs. Do...End Blocks

Function bodies use `{ }` and can contain statements directly. You don't need `do ... end` inside them—the braces already form a block. Both `{ }` and `do ... end`:

- Accept a sequence of statements (bindings and expressions)
- Execute them in order
- Use the last expression as the result

The difference is **where** they appear:

- **`{ }`** — Used for function and lambda bodies. The grammar expects a body there.
- **`do ... end`** — Used when you need that same block in an **expression position** (e.g., the right-hand side of a case branch, or the right-hand side of a binding).

You can think of the function body `{ }` as a built-in block that does what `do ... end` does. Use `do ... end` when you need that block somewhere an expression is required.

---

## Why Do...End Blocks Are a Good Idea

In Silica, `do ... end` blocks let you write a sequence of steps where each step can use the results of the previous ones. Instead of nesting everything or losing track of intermediate values, you write steps in order and give names to the results you need later.

**Benefits:**

- **Readable flow**: Steps run top to bottom, like a recipe. No deep nesting.
- **Named intermediates**: You can bind results to names (`x: int64 <- compute_something()`) and reuse them.
- **Effect tracking**: The compiler sees which steps do I/O, memory, or concurrency, and checks that your function declares the right effects.
- **Explicit return value**: The last expression in the block (without a semicolon) is the value the block returns.

**Rule of thumb:** Use `do ... end` when you need to run several steps in order and use the result of one step in the next. Skip it when a single expression is enough.

---

## Good Examples

### Multiple Steps, Each Depends on the Last

```silica
fn parse_line(content: string, start: int64) -> string proc[DeviceIO] {
    case start >= len(content) of {
        true -> "";
        -- Need do...end: multiple steps, each depends on the last
        false -> do
            line: string <- substring_until_char(content, start, '\n');
            trimmed: string <- trim_leading(line);
            trimmed
        end
    }
}
```

**Why this is good:** You need `line` to compute `trimmed`, and `trimmed` is the final value. The `do` block sequences these steps and returns the last one.

---

### Multiple Steps in a Function Body

```silica
fn read_and_process(path: string) -> int64 proc[DeviceIO] {
    -- Need do...end: multiple steps, each depends on the last
    do
        content: string <- read_lines(path);
        trimmed: string <- trim_leading(content);
        len(trimmed)
    end
}
```

**Why this is good:** Three steps in order: read, trim, then return the length. Each step uses the previous result.

---

### Actor: Spawn, Send, Then Return

```silica
fn start_echo_actor() -> actor_ref proc[concurrency, device_io] {
    -- Need do...end: spawn, then send, each step uses the last
    do
        echo_ref: actor_ref <- spawn(
            EchoState { received: 0 },
            fn(msg: Response, state: EchoState) -> EchoState proc[device_io] {
                print_string("Received: ");
                EchoState { received: state.received + 1 }
            }
        );
        send(echo_ref, Response { result: 42 });
        echo_ref
    end
}
```

**Why this is good:** You need the `actor_ref` from `spawn` to call `send`, then you return that same ref. Multiple steps, each using the last.

---

### Do Block Returns a Value You Use

```silica
-- The do block returns actor_ref; that value is used
fn get_actor_and_log() -> int64 proc[concurrency, device_io] {
    do
        ref: actor_ref <- spawn(State {}, fn(msg: Msg, s: State) -> State { s });
        send(ref, Msg {});
        -- Last expression: the block returns this int64
        len("started")
    end
}

-- The do block's return value is passed to another function
fn process() -> string proc[device_io] {
    result: int64 <- do
        content: string <- read_lines("config.txt");
        len(content)
    end;
    concat("Length: ", int_to_string(result))
}
```

**Why this is good:** The `do` block is an expression. Its value (the last expression) is either returned directly or bound and passed to another function. The block both runs steps and produces a usable result.

---

## Bad Examples (Non-Examples)

### Single Expression, No Intermediate Steps

```silica
fn add(x: int64, y: int64) -> int64 {
    -- No do...end: one expression, no intermediate steps
    x + y
}

fn main() -> int64 proc[device_io] {
    -- No do...end: one expression, nothing to sequence
    42
}
```

**Why this is bad to wrap in do:** There is nothing to sequence. A single expression is enough; `do ... end` adds noise without benefit.

---

### Actor: Spawn and Return (Single Expression)

```silica
fn start_actor() -> actor_ref proc[concurrency] {
    -- No do...end: one expression, spawn and return
    spawn(InitialState {}, fn(msg: Msg, state: InitialState) -> InitialState { state })
}

-- Actor behavior: single expression, no intermediate steps
fn(msg: Ping, state: Counter) -> Counter {
    -- No do...end: just update and return state
    Counter { count: state.count + 1 }
}
```

**Why this is bad to wrap in do:** One operation, one expression. No need to sequence anything.

---

### Fire-and-Forget Send (Single Expression)

```silica
fn fire_and_forget(ref: actor_ref) -> atom proc[concurrency] {
    -- No do...end: single expression, return value of send() is discarded
    send(ref, Msg {})
}
```

**Why this is bad to wrap in do:** A single `send` call. No extra steps, no need for a block.

---

### Single Expression Already Returns What You Need

```silica
fn get_length(path: string) -> int64 proc[device_io] {
    -- No do...end: one expression, its value is the return value
    len(read_lines(path))
}
```

**Why this is bad to wrap in do:** One expression does the job. The function already returns the value you need.

---

### Redundant Do Around a Single Expression

```silica
fn redundant() -> int64 {
    -- Avoid: do...end with one expression; the expression alone is enough
    do
        42
    end
}
```

**Why this is bad:** The `do` block adds nothing. Use `42` directly as the function body.

---

## Summary

| Situation                          | Use `do ... end`? |
|------------------------------------|-------------------|
| Multiple steps with bindings       | Yes               |
| Case/if branch with several steps  | Yes               |
| Actor: spawn then send then return | Yes               |
| Block’s return value is used      | Yes               |
| Single expression                  | No                |
| One operation, one result          | No                |

---

## Appendix: Category-Theoretic View (Monads)

For readers familiar with category theory, Silica's `do ... end` blocks are syntactic sugar for Moggi's computational metalanguage—i.e., Kleisli composition in an **effect-indexed monad**.

### Computation Types as an Indexed Monad

Computation types have the form `proc[ε] A`, which corresponds to:

    T^ε = M^ε A

where:

- A is a value type (object in the value category),
- ε is an effect set: {device_io, concurrency, mem(S), mailbox(M), atomic},
- M^ε is the monad indexed by effects.

So M^ε : C → C is a family of endofunctors parameterized by ε.

### Monad Structure

**Return (unit):**

    η_A : A → M^{[]} A

A pure value `t : A` is interpreted as `return t : M^{[]} A`—a computation with no effects.

**Bind (Kleisli extension):**

    (>>=) : M^ε₁ A × (A → M^ε₂ B) → M^(ε₁∪ε₂) B

The `do` block `do x <- m; n end` is:

    m >>= λx. n

where m : M^ε₁ A and n : M^ε₂ B under the extended context Γ, x : A. The resulting computation has type M^(ε₁∪ε₂) B.

### Subeffecting

Subeffecting corresponds to a coercion/morphism between computation types:

    Γ ⊢ m : M^ε₁ A    and    ε₁ ⊆ ε₂    implies    Γ ⊢ m : M^ε₂ A

So the monad is contravariant in the effect index with respect to this ordering: a computation with fewer effects can be used where more effects are allowed.

### Kleisli Composition

Sequencing in a `do` block is Kleisli composition. For f : A → M^ε₁ B and g : B → M^ε₂ C:

    (g ∘_K f)(a) = f(a) >>= g

which has type A → M^(ε₁∪ε₂) C. Multiple statements in a `do` block are exactly this composition.

### Effectful Operations

Primitive effectful operations (e.g., `read_lines`, `spawn`, `send`) are arrows:

    op : A → M^ε B

for appropriate ε. They are the "algebra" that generates the effectful computations; the monad structure sequences and combines them.

### Summary

| Silica | Category Theory |
|--------|-----------------|
| `proc[ε] A` | M^ε A |
| `do x <- e1; e2 end` | e₁ >>= λx. e₂ |
| Pure expression `e` in do block | η(e) |
| Effect set in signature | Index ε on M^ε |
| Subeffecting | Coercion along ε₁ ⊆ ε₂ |
