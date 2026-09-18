---
title: Learn to Program
layout: default
permalink: /learn-programming/
---

# Learn to Program

**A first book of programming, taught in Silica**

Copyright © 2026 Lee Scott Barney

Use this book together with a large language model (an LLM — a chat program that can read these pages and answer questions) and the [Silica language documentation](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md). Point the LLM at both. Ask it to walk through examples, check your attempts, and explain a line you do not yet understand. Tell it what computer you are using. It will fill in the practical steps this book leaves out: how to compile a program, how to run it, and how to see the result on your machine. The book and the language documentation are the source of the rules; the LLM is there to help you learn to write programs and to understand how programming works.

This book is for people who have never written a computer program. You do not need to know what a compiler is, what a type is, or how a chip works. We start from one idea: a program is a group of small workers that send each other messages, watched over by someone who helps when one of them makes a mistake. We write those workers in [Silica]({{ '/' | relative_url }}).

If you already write software, this is the wrong book. Use [Silica for Programmers]({{ '/learn-silica/' | relative_url }}) instead.

## 1. You already know how this works

You have a lot of experience with how the world works. You have friends. You communicate with them. You ask them for things, and they ask you for things.

Some of your friends are so overwhelmed that they can only do one thing at a time. You ask, they do it, and only then can they hear the next request. Other friends can take on several things at once and get back to you when each one is done. You already know how to work with both kinds. You know that a message you send is not the same as the answer you get back. You know that if you ask for something while your friend is busy, the request waits its turn.

It is much easier to think about code when the code matches that experience. In Silica it does. The small workers that make up a program are called **actors**. Each actor is like a friend: it has its own memory, it does its own work, and it talks to other actors only by sending and receiving messages. Nobody reaches into anyone else's head. You ask; they answer, or they just get on with it.

There is one more relationship you already understand. Elephant mothers give birth to their children and then watch over them for decades — often for their entire lives. The children do their own things. They wander, they eat, they play, they make mistakes. The mother does not do those things for them. She watches, and when a child gets into trouble she steps in and helps it get going again.

In Silica that mother is called a **supervisor**. A supervisor starts actors — it gives birth to them — and then watches over them. The actors do the work. If one of them fails, the supervisor notices and starts it again, fresh. Supervisor-and-actor is elephant-mother-and-child. We will come back to this picture whenever it helps, which is often.

## 2. How to read this book

Read in order. Each chapter uses only ideas already introduced.

You do not have to run every example on a computer to learn from this book. Reading the code and predicting what it means is real practice. When you are ready to run programs, the [project page](https://github.com/yenrab/silica#building-the-compiler) explains how to build the Silica compiler.

A few habits will help:

- Read every example slowly. One unfamiliar word is enough reason to pause.
- When you see a new symbol, treat it as vocabulary, not as decoration.
- When a chapter tells you to read only the lines that end with a `//` comment, do exactly that. The rest of the code will make sense later.
- Try the short exercises at the ends of chapters. They are small on purpose.
- If a later chapter feels sudden, go back one chapter. That is normal.

Silica is still growing. The [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) is the source of truth if this book and the compiler ever disagree.

## 3. Your first program

Here is a complete Silica program. It has two files. Do **not** try to understand all of it. Read only the comments — the notes that start with `//` — and look only at the lines that end with one. There are five such lines. Everything else is scaffolding you will learn to read over the next few chapters.

The first file is `main.silica`:

```silica
use Supervisor;
use adder;

impl AdderSupervisor for Supervisor;

fn init(initial_state: int64) -> (
    {
        strategy: :one_for_one | :one_for_all | :rest_for_one,
        allowed_restart_count: int64,
        restarts_time_frame: int64
    },
    List[
        {
            id: atom,
            agent_type: :worker | :supervisor,
            initial_state: int64,
            behavior: fn(msg: (int64, int64), state: int64) -> (:no_reply, int64),
            restart: :permanent | :temporary | :transient,
            shutdown: int64,
            flavor: :plain | :dangerous
        },
        mem(normal)
    ]
) {
    sequence proc[mem(normal)]
        flags: {
            strategy: :one_for_one | :one_for_all | :rest_for_one,
            allowed_restart_count: int64,
            restarts_time_frame: int64
        } <- { strategy: :one_for_one, allowed_restart_count: 3, restarts_time_frame: 5 };
        children: List[
            {
                id: atom,
                agent_type: :worker | :supervisor,
                initial_state: int64,
                behavior: fn(msg: (int64, int64), state: int64) -> (:no_reply, int64),
                restart: :permanent | :temporary | :transient,
                shutdown: int64,
                flavor: :plain | :dangerous
            },
            mem(normal)
        ] <- [
            {
                id: :adder,
                agent_type: :worker,
                initial_state: 0,
                behavior: adder@add_and_print,  // the mother gives birth to her calf, and will watch over it from now on
                restart: :permanent,
                shutdown: 0,
                flavor: :plain
            }
        ]
    produces
        pure (flags, children)
    end
}

fn main() -> int64 {
    sequence proc[concurrency, device_io]
        _: supervisor_ref <- spawn_registered_supervisor(AdderSupervisor, 0, :adder_supervisor, stack_policy(0, :keep_last_message));  // the elephant mother arrives, ready to have her calf
        _: boolean <- cast_registered(:adder, (2, 3) impl ActorMessage {});  // something in the world nudges the calf: "here are two numbers"
        _: int64 <- wait_for_exit()
    produces
        pure 0
    end
}
```

The second file is `adder.silica`:

```silica
export add_and_print/2;

fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        total: int64 <- left + right;  // the calf does its own work: it adds the two numbers
        _: atom <- print_int64(total);  // and shows the world what it did
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

When you run this program it prints:

```
5
```

and then waits. Type `exit` and press Enter, and it ends.

Now read the five commented lines again, in the order things happen.

1. In `main`, the elephant mother arrives: `spawn_registered_supervisor(...)` starts the supervisor. Its name is `AdderSupervisor`.
2. The supervisor's `init` function is the list of children she will give birth to. There is one child, called `:adder`, and the line `behavior: adder@add_and_print` says what that child does with its life: it runs the function `add_and_print` from the file `adder.silica`. The supervisor starts the child and watches it from then on.
3. Back in `main`, something in the world nudges the calf. `cast_registered(:adder, (2, 3) ...)` sends the child a message. The message is a pair of numbers, `2` and `3`. `cast` means "send this and do not wait for an answer" — like leaving a note.
4. In `adder.silica`, the calf does its own work. `total: int64 <- left + right` adds the two numbers from the message.
5. `print_int64(total)` shows the world what it did. That is the `5` you see.

The last line of `main`, `wait_for_exit()`, keeps the program alive so the calf has time to work and print. Without it, `main` would finish before the message was ever read.

That is the whole shape of every program in this book: a supervisor gives birth to actors, messages arrive from the world, the actors do their work, and the supervisor watches.

These two files, with a short README, live in the Silica repository at `docs/examples/first_program/`. If you want to run them, ask your LLM to help you compile the two files together with the standard-library file `Supervisor.silica` — the `use Supervisor;` line at the top of `main.silica` asks for it.

### Try this

Change `(2, 3)` to `(10, 32)` in `main.silica` and predict what the program prints.

Answer: `42`. Nothing about the calf changed. Only the message changed.

## 4. What a program is

A computer is a machine that follows instructions. It is fast, literal, and unimaginative. It will not guess what you meant.

A **program** is a written list of those instructions. The computer does not understand English. We write in a **programming language** — a language designed so that each sentence has one meaning.

Cooking is a useful picture. A recipe says what to do, in what order, with which ingredients. If the recipe says "add 2 eggs," the cook does not add three. A program is a recipe for a computer. In Silica the cooks are actors, and each actor's recipe is its **behaviour**: a function that says what to do with one message.

The language in this book is **Silica**. You will see words such as `fn`, `case`, and `sequence`. Those are part of Silica's grammar, the same way "stir" and "bake" are part of a recipe's grammar.

A program is stored in **files**. Silica program files end in `.silica`. The first program had two: `main.silica` and `adder.silica`. You can open them in any text editor. They are ordinary text, not a special secret format.

The computer cannot run the text of a Silica file directly. A program called a **compiler** reads your files, checks that they make sense, and turns them into something the machine can execute. If the compiler finds a mistake, it stops and explains. That is a gift. A stopped program cannot quietly do the wrong thing.

Silica's motto is: *secure by default at compile time — fail soft, never fail silent.* In beginner terms: the compiler tries to catch problems before they become surprises, and when a running actor does fail, its supervisor is there — the failure is noticed and handled, never hidden.

**How the rest of this book shows programs.** The supervisor's `init` function in Chapter 3 is long, and it is almost the same in every program. From now on, examples show the actor's behaviour and the lines of `main` that change. The supervisor is the one from Chapter 3 with three things swapped: the child's `id`, the child's `behavior`, and the behaviour's type, which is written in two places inside `init` (both say `behavior: fn(msg: ..., state: ...) -> ...`). When a program needs something else changed, the chapter says so.

## 5. Values

A **value** is a finished piece of data. `42` is a value. So is `true`. So is `"hello"`. The message in the first program, `(2, 3)`, is a value made of two smaller values.

Messages carry values. That is what a message is: a value sent from one place to another.

Silica has several everyday kinds of value.

**Whole numbers.** `0`, `1`, `42`, `-3`. These have type `int64` in ordinary programs.

**True or false.** `true` and `false`. These have type `boolean`. They answer yes-or-no questions. `cast_registered` in the first program gave back a boolean: `true` means the message was accepted.

**Text.** `"Ada"` and `"hello, world"` are **strings**. A string is a sequence of characters. The quotes are not part of the text; they mark where the text begins and ends.

**A single character.** `'A'` is a `char`. Notice the single quotes. `"A"` is a string of one character. `'A'` is the character itself.

**Nothing interesting.** `()` is the **unit** value. It means "there is no useful result here."

**Atoms.** `:adder`, `:ok`, and `:error` are **atoms**. An atom is a named label. It is not a string. Two atoms are the same when they have the same name. The first program used the atom `:adder` as the calf's name, so that `main` could send it a message without holding anything else. Beginners can treat atoms as stickers you put on things: "this one is the adder," "this one is okay," "this one is an error."

Inside a behaviour you can combine numbers with arithmetic:

```silica
2 + 3        // 5
10 - 4       // 6
3 * 7        // 21
20 / 6       // 3  (whole-number division, leftover is dropped)
20 % 6       // 2  (the leftover)
```

The marks `//` start a **comment**. The compiler ignores comments. They are notes for people. The first program's five important lines were marked by comments.

You can compare values. Comparisons produce booleans:

```silica
3 < 5        // true
3 == 5       // false
3 != 5       // true
```

`==` asks "are these the same?" `!=` asks "are these different?"

`and`, `or`, and `not` combine booleans:

```silica
true and false    // false
true or false     // true
not true          // false
```

Parentheses control order, as in arithmetic class:

```silica
(2 + 3) * 4       // 20, not 14
```

### Try this

1. The adder's behaviour computes `left + right`. If you changed it to `left * right` and sent `(2, 3)`, what would the calf print?
2. What is `8 / 3` in Silica? Why is it not `2.666…`?
3. What is `(true or false) and false`?

Answers: (1) `6`. (2) `2`, because `int64` division keeps only the whole part. (3) `false`.

## 6. Names

A value you will use again should have a **name**. In Silica you **bind** a name to a value with `<-`. The adder did this:

```silica
total: int64 <- left + right;
```

Read it left to right: "`total`, which is an `int64`, gets `left + right`."

The name is `total`. The type is `int64`. The value is whatever the sum is. After that line, you may use `total` wherever you would have written the sum — and the adder does, on the very next line, when it prints.

The semicolon ends the binding. The last line of a function is its result, and it has no semicolon.

A name in Silica is not a box you keep refilling. Once `total` is bound, it stays that value for the rest of that message. If you need a different value, you bind a new name, or you write a new binding that computes from the old one:

```silica
fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        total: int64 <- left + right;
        doubled: int64 <- total * 2;
        _: atom <- print_int64(doubled);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

With the message `(2, 3)` this calf prints `10`. `total` is still `5`. `doubled` is a new name.

Why so strict? Because a name that never changes is a name you can trust. If `total` is `5` on one line, it is still `5` ten lines later. That makes programs easier to read and easier for the compiler to check. It also matches the way actors work: each message is handled from a clean start, with its own names.

The name `_` is special. It means "I have to bind this, but I do not need it." The first program wrote `_: atom <- print_int64(total)` because printing gives back a small value nobody wants.

You choose names. Good names read like English: `total`, `left`, `guest_count`. Silica uses **snake_case**: words in lowercase, separated by underscores.

### Try this

What does this calf print when it receives `(4, 7)`?

```silica
fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        product: int64 <- left * right;
        _: atom <- print_int64(product);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

Answer: `28`.

## 7. Types

A **type** is a promise about a value. `int64` promises a whole number. `string` promises text. `boolean` promises `true` or `false`.

Silica writes types in the open. Function parameters have types. Bindings have types. Even the catch-all `_` has a type. That looks wordy at first. It is deliberate. You should never have to guess what kind of thing a name is.

Look at the first line of the adder's behaviour:

```silica
fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
```

It is a set of promises. `msg` is a pair of whole numbers — that is the message the calf accepts. `state` is a whole number — that is what the calf remembers between messages (Chapter 11). After the arrow, `(:no_reply, int64)` promises what the behaviour hands back: the atom `:no_reply` (this calf does not answer messages, it just acts on them) and a new `int64` to remember.

The supervisor makes the same promise in its `init`, on the line `behavior: fn(msg: (int64, int64), state: int64) -> (:no_reply, int64)`. The mother knows what her calf eats. If you wrote a behaviour that takes a `string` message and listed it in an `init` that promises `(int64, int64)`, the compiler would refuse. That refusal is the type system doing its job.

A few types you will see often:

| Type      | Example values  | Everyday meaning |
| --------- | --------------- | ---------------- |
| `int64`   | `0`, `42`, `-1` | whole numbers    |
| `boolean` | `true`, `false` | yes or no        |
| `string`  | `"hello"`       | text             |
| `char`    | `'x'`           | one character    |
| `atom`    | `:ok`, `:adder` | a named label    |
| `()`      | `()`            | no useful value  |

Silica also has smaller and larger number types (`int8`, `uint64`, `float64`, and others). You can ignore them until you have a reason. `int64` is the default whole number.

Types are not decoration. They are the first security feature you will feel. A calf that expects two numbers can never be sent a piece of text by mistake. The program that tries never becomes a running program. The compiler stops you at the door.

### Try this

Which line would the compiler reject, and why?

```silica
fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        name: string <- "Nia";
        total: int64 <- name + left;
        _: atom <- print_int64(total);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

Answer: `total: int64 <- name + left`. `name` is text. `left` is a number. Silica will not add them.

## 8. Functions

A function is a reusable piece of work with a name, inputs, and a result. A behaviour is a function — a special one that an actor runs once for every message. But you can write ordinary functions too, and a behaviour can call them.

```silica
fn add(x: int64, y: int64) -> int64 {
    x + y
}

fn double(n: int64) -> int64 {
    n * 2
}

fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        _: atom <- print_int64(add(double(left), double(right)));
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

`add` takes two `int64` values, called `x` and `y`, and returns their sum. `double` takes one and returns twice it. The behaviour **calls** them by writing the name and the values in parentheses. With the message `(3, 4)` this calf prints `14`: `double(3)` is `6`, `double(4)` is `8`, and `add(6, 8)` is `14`.

Read `fn add(x: int64, y: int64) -> int64` as: "a function named `add`. It needs `x` and `y`, both whole numbers. It gives back a whole number."

The names `x` and `y` belong to `add`. They are **parameters**. The values handed over in `add(6, 8)` are **arguments**. When the call happens, `x` is `6` and `y` is `8` for the duration of that call.

Functions keep programs small. Instead of copying the same arithmetic into ten behaviours, you write it once and every behaviour calls it.

Every function in Silica lives at the **top level** of a file. You do not nest one `fn` inside another. If a behaviour needs help, write another function next to it.

A function may have at most eight parameters. If you need more, group related values (Chapter 13).

Notice the split between the two kinds of function. `add` and `double` are pure arithmetic: give them numbers, get a number. `add_and_print` is a behaviour: it is the calf's whole life, one message at a time, and it is the only one of the three that touches the outside world.

### Try this

Write a function `triple` that multiplies its one `int64` parameter by `3`. Then change the behaviour so the calf prints `triple(left) + triple(right)`. What does it print for `(1, 2)`?

```silica
fn triple(n: int64) -> int64 {
    n * 3
}

fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        _: atom <- print_int64(triple(left) + triple(right));
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

Answer: `9`.

## 9. Decisions

Actors choose. "If the message says morning, say good morning. Otherwise, say something else."

Silica has no standalone `if` statement. Every choice is a `case` expression. You give `case` a value, and you list the shapes that value might have. The matching arm is the result.

A calf that greets according to the time of day receives an atom and chooses:

```silica
fn greeter(msg: atom, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- case msg of {
            :morning -> println("Good morning");
            :evening -> println("Good evening");
            _: atom -> println("Hello")
        }
    produces
        pure (:no_reply, state)
    end
}
```

In `init`, this child's type is `fn(msg: atom, state: int64) -> (:no_reply, int64)`, and `main` sends it atoms:

```silica
        _: boolean <- cast_registered(:greeter, :morning impl ActorMessage {});
        _: boolean <- cast_registered(:greeter, :evening impl ActorMessage {});
        _: boolean <- cast_registered(:greeter, :noon impl ActorMessage {});
        _: int64 <- wait_for_exit()
```

The calf prints `Good morning`, then `Good evening`, then `Hello`. Each arm has the form `pattern -> result`. Arms are separated by semicolons.

`_: atom` is the **catch-all**. The underscore means "I do not need the value." The type is still required. Silica will not accept a bare `_`.

The simplest choice is a boolean. A calf that says whether a number is even:

```silica
fn parity(msg: int64, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- case msg % 2 == 0 of {
            true -> println("even");
            false -> println("odd")
        }
    produces
        pure (:no_reply, state)
    end
}
```

You can match the number itself, with a **guard** after `if`. Here an ordinary function does the deciding and a behaviour calls it:

```silica
fn describe(n: int64) -> string {
    case n of {
        x: int64 if x > 0 -> "positive";
        x: int64 if x < 0 -> "negative";
        _: int64 -> "zero"
    }
}

fn describer(msg: int64, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        println(describe(msg))
    produces
        pure (:no_reply, state)
    end
}
```

`x: int64 if x > 0` means: "bind the number to `x`, but only take this arm when `x` is greater than zero." Sent `6`, `-4`, and `0`, this calf prints `positive`, `negative`, `zero`.

`case` must be **exhaustive**. If a value could arrive that no arm covers, the compiler rejects the program. That is another gift. A calf that has no idea what to do with a message it might receive is a famous source of bugs in other languages. In Silica it does not compile.

`case` is an expression, so it has a result. You can bind that result, as `greeter` does with `_: atom <- case ...`, or return it directly, as `describe` does.

### Try this

1. What does `describe(0)` return?
2. What does the `greeter` calf print if `main` sends it `:afternoon`?

Answers: (1) `"zero"`. (2) `Hello` — the catch-all arm.

## 10. Doing things in order, and being honest about it

Every behaviour so far has had this shape inside it:

```silica
    sequence proc[device_io]
        ...steps...
    produces
        pure (:no_reply, state)
    end
```

A **sequence** runs its steps from top to bottom and then produces a result. Read it as: do these bindings, in order. Then produce this. The word `pure` means the result itself does not start any new outside-world work. `end` closes the block. The `produces` line is the doorway out.

The steps can be plain arithmetic:

```silica
fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        next: int64 <- left + right;
        last: int64 <- next * 3;
        _: atom <- print_int64(last);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

Sent `(10, 2)`, this prints `36`: `12`, then `36`.

Now the part beginners skip and should not: `proc[device_io]`.

So far, most of an actor's work has been computing a value. Real actors also print, read files, and send messages to other actors. Those actions are **effects**. They change something you can notice outside the function's result.

Silica makes effects visible. You declare them on the sequence that performs them.

- `device_io` — this sequence may print or use files. The calf prints, so its sequence says `device_io`.
- `concurrency` — this sequence may start actors or send messages. `main` in the first program starts a supervisor and sends a message, so it says `proc[concurrency, device_io]`.
- `mem(normal)` — this sequence may ask for ordinary memory to hold something new, such as a growing list (Chapter 14). The supervisor's `init` builds its list of children, so it says `mem(normal)`.

Why the ceremony? Because a function that only adds numbers and a function that writes a file are different kinds of thing. Silica will not let you hide a print inside an innocent-looking helper. If something prints, a sequence above it must admit `device_io`. If something sends a message, a sequence above it must admit `concurrency`.

That is not bureaucracy for its own sake. It is honesty. When you read a program six months later, the word `device_io` tells you: this part talks to the world. The word `concurrency` tells you: this part talks to other actors.

`println` writes a line of text. `print_int64` writes a number without a line break, which is why the calf follows it with `println("")` to end the line. Silica also has `print`, which writes text without adding a new line. All of them need `device_io`.

### Try this

Change the adder so it prints `first`, then the total, then `second`, on three lines. Which effect does the sequence need, and why?

Answer: still only `device_io` — printing more does not need anything new. The steps are `println("first");`, then the two printing lines, then `println("second")` as the last step before `produces`.

## 11. What an actor remembers

Every behaviour has had a parameter called `state` that we have ignored. It is the actor's **memory**: what the calf remembers from one message to the next.

The rule is simple. The behaviour receives the old state along with the message, and hands back the new state. Whatever it hands back is what it will receive with the next message. The supervisor's `init` sets the very first state with `initial_state`.

A counting calf:

```silica
fn counter(msg: int64, state: int64) -> (:reply, int64, int64) {
    total: int64 <- state + msg;
    (:reply, total, total)
}
```

This behaviour has a new shape: `(:reply, int64, int64)`. The first program's calf did its work and stayed quiet — `:no_reply`. This one **answers**. The result `(:reply, total, total)` says: reply with `total`, and remember `total`.

Because it answers, `main` sends to it with `call_registered`, which waits for the answer, instead of `cast_registered`, which does not:

```silica
        first: int64 <- call_registered(:counter, 1 impl ActorMessage {});
        second: int64 <- call_registered(:counter, 4 impl ActorMessage {});
        _: atom <- print_int64(first);
        println("");
        _: atom <- print_int64(second);
        println("")
```

With `initial_state: 0` in `init`, this prints `1` and then `5`. The first message adds `1` to `0`; the calf answers `1` and remembers `1`. The second adds `4`; the calf answers `5` and remembers `5`.

This is the two kinds of friend from Chapter 2. `cast` is the note you leave and walk away from. `call` is the question you wait on. A behaviour picks one: `:no_reply` for calves that are cast to, `:reply` for calves that are called. The supervisor's `init` must promise the matching type: `fn(msg: int64, state: int64) -> (:reply, int64, int64)` for the counter.

Notice what state gives you. Names inside a behaviour are fresh for every message (Chapter 6). State is the one thing that lasts. It is also private. No other actor can look at the counter's `state`; they can only send a message and read the answer. That privacy is why actors are easy to reason about: the only way in is the mailbox.

One more thing to notice: `main` did not use `wait_for_exit()` here. `call_registered` waits for each answer, so by the time `main` prints, the calf has already done its work.

### Try this

If `main` called the counter with `10`, then `20`, then `30`, what would the three answers be?

Answer: `10`, `30`, `60`.

## 12. Text

Strings are values. Like any value, they can travel in messages, be bound to names, and be handed back as answers.

A greeting calf receives a name and prints a greeting:

```silica
fn greeter(msg: string, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        greeting: string <- concat("Hello, ", msg);
        println(greeting)
    produces
        pure (:no_reply, state)
    end
}
```

`concat` joins two strings. Sent `"Ada"` and then `"Nia"`:

```silica
        _: boolean <- cast_registered(:greeter, "Ada" impl ActorMessage {});
        _: boolean <- cast_registered(:greeter, "Nia" impl ActorMessage {});
        _: int64 <- wait_for_exit()
```

the calf prints `Hello, Ada` and `Hello, Nia`. The `impl ActorMessage {}` after a value is the marker that says "this value is allowed to be a message." You have seen it on every send so far.

Useful questions about text:

```silica
length_chars("Ada")              // 3
starts_with("Ada", "A")          // true
contains("Ada Lovelace", "Love") // true
```

`length_chars` counts characters. `length_bytes` counts the underlying UTF-8 bytes. For English letters they often match. For many other languages and for emoji they may not. When you care about "how many letters," use `length_chars`.

You can choose a piece of a string with `substring`. The positions count characters, starting at `0` for the first character.

Strings are not numbers. `"3"` and `3` are different values with different types. A calf whose behaviour promises a `string` message cannot be sent `3`, and a calf that promises `int64` cannot be sent `"3"`. The compiler checks every send against the promise.

### Try this

What does the greeter print if `main` sends it `"world"`?

Answer: `Hello, world`.

## 13. Grouping values

Sometimes two or three values belong together: a first and last name, a width and a height, a status and a number. Messages very often carry such groups. The first program's message, `(2, 3)`, was one.

A **tuple** is an ordered group. The position is what matters.

```silica
        (left: int64, right: int64) <- msg;
```

`(2, 3)` is a pair. `(int64, int64)` is its type. The binding above **unpacks** the pair into `left` and `right`. That is how the adder got at its two numbers.

A **record** is a group with field names. The name is what matters. A calf that works out the area of a rectangle:

```silica
fn area_printer(msg: { width: int64, height: int64 }, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- print_int64(msg.width * msg.height);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

and `main` sends it a record:

```silica
        _: boolean <- cast_registered(:area, { width: 3, height: 4 } impl ActorMessage {});
        _: int64 <- wait_for_exit()
```

The calf prints `12`. `{ width: 3, height: 4 }` is a record value. `{ width: int64, height: int64 }` is a record type. `msg.width` reads the `width` field.

Silica does not ask you to invent a new type name for every record. You write the shape where you need it. Two records with the same fields and field types are the same kind of thing. You have already seen a big one: each child in the supervisor's `init` is a record with fields `id`, `behavior`, `restart`, and so on.

Records are also the way to hand back a labeled result — an answer together with a note saying whether it is any good:

```silica
fn safe_divide(x: int64, y: int64) -> { ok: boolean, value: int64 } {
    case y == 0 of {
        true -> { ok: false, value: 0 };
        false -> { ok: true, value: x / y }
    }
}
```

`{ ok: true, value: 5 }` means "it worked, and the answer is 5." `{ ok: false, value: 0 }` means "it failed." Chapter 16 builds on this.

### Try this

What does the area calf print for `{ width: 2, height: 10 }`?

Answer: `20`.

## 14. Lists

A **list** is a sequence of values of the same type: three numbers, a dozen names, no items at all.

A calf that remembers every number it has ever been told keeps a list as its state:

```silica
fn keeper(msg: int64, state: List[int64, mem(normal)]) -> (:no_reply, List[int64, mem(normal)]) {
    sequence proc[device_io, mem(normal)]
        remembered: List[int64, mem(normal)] <- prepend[int64, mem(normal)](msg, state);
        _: atom <- print_int64(length[int64, mem(normal)](remembered));
        println("")
    produces
        pure (:no_reply, remembered)
    end
}
```

Three things are easy to miss.

The type is written `List[int64, mem(normal)]`, not just `List`. Every list knows its element type and which kind of memory it lives in. `mem(normal)` is ordinary memory, and it is the one you will use.

Growing a list needs memory, so the sequence declares `mem(normal)` alongside `device_io`.

Lists are **immutable**. `prepend` does not change `state`; it returns a new list with `msg` at the **front**. `state` is still the old list. The calf hands the new list back as its new state, and that is how its memory grows.

In `init` this child's `initial_state` is an empty list, `empty[int64, mem(normal)]()`, and its type is `fn(msg: int64, state: List[int64, mem(normal)]) -> (:no_reply, List[int64, mem(normal)])`. Sent `7`, `8`, and `9`, the calf prints `1`, `2`, `3` — the length after each message.

A list can also be written out in full, `[1, 2, 3]`, inside a sequence that declares `mem(normal)`.

Useful list operations:

- `prepend[int64, mem(normal)](x, xs)` — a new list with `x` at the front
- `length[int64, mem(normal)](xs)` — how many elements
- `empty[int64, mem(normal)]()` — a list with no elements

You cannot reach into the middle and pluck an item with a special "item 7" operation. You take a list apart from the front, with `case`, which is the next chapter. That sounds limiting. It is also simple. There is one way to take a list apart: look at the first element, then look at the rest.

### Try this

If the keeper has been sent `7`, `8`, and `9`, in that order, what is its state, front to back?

Answer: `[9, 8, 7]`. `prepend` adds at the front.

## 15. Repeating work

Many languages have loops: "do this ten times," "keep going while this is true." Silica does not. When you need to repeat work, you write a **recursive** function: a function that calls itself, each time on a smaller piece of the problem, until a simple case remains.

That sounds abstract. It is the same idea as a stack of plates. To wash the stack: wash the top plate, then wash the remaining stack. The empty stack is the simple case — you are done.

A calf that adds up every number in a list it is sent:

```silica
fn sum(numbers: List[int64, mem(normal)]) -> int64 {
    case numbers of {
        []: List[int64, mem(normal)] -> 0;
        [first: int64, rest: List[int64, mem(normal)]] -> first + sum(rest);
        _: List[int64, mem(normal)] -> 0
    }
}

fn summer(msg: List[int64, mem(normal)], state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- print_int64(sum(msg));
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

`case` takes the list apart. The pattern `[]` matches the empty list: the sum is `0`. The pattern `[first: int64, rest: List[int64, mem(normal)]]` matches a list with at least one element, binds the first element to `first` and everything after it to `rest`, and the sum is `first` plus the sum of the rest. The final `_` arm is the catch-all Silica requires so the `case` is complete.

`main` builds a list and sends it. Because it builds a list, its sequence now declares `mem(normal)` too:

```silica
    sequence proc[concurrency, device_io, mem(normal)]
        _: supervisor_ref <- spawn_registered_supervisor(SumSupervisor, 0, :sum_supervisor, stack_policy(0, :keep_last_message));
        numbers: List[int64, mem(normal)] <- [1, 2, 3];
        _: boolean <- cast_registered(:summer, numbers impl ActorMessage {});
        _: int64 <- wait_for_exit()
```

The calf prints `6`. Walk it:

- `1 + sum([2, 3])`
- `1 + (2 + sum([3]))`
- `1 + (2 + (3 + sum([])))`
- `1 + (2 + (3 + 0))`
- `6`

The empty list is what stops the repetition. Every recursive function needs a case that does **not** call itself. Forgetting that case is how you ask a computer to work forever.

Count down to zero, with no list at all:

```silica
fn sum_to(n: int64) -> int64 {
    case n <= 0 of {
        true -> 0;
        false -> n + sum_to(n - 1)
    }
}
```

`sum_to(3)` is `3 + 2 + 1 + 0`, which is `6`.

There is a second kind of repetition you have been using all along without naming it. An actor handles one message, then the next, then the next, for as long as messages arrive. The behaviour is written once; the mailbox makes it repeat. The keeper in Chapter 14 never wrote a loop, and yet it counted `1`, `2`, `3`. Recursion repeats work *inside* one message. The mailbox repeats work *across* messages.

Recursion is the ordinary way to walk data in Silica. The runtime is allowed to turn a well-written recursive function into an efficient loop internally. You still write the idea as "solve the small piece, then the rest."

### Try this

What does the summer calf print for `[4, 1]`?

Answer: `5`.

Write `product`, like `sum`, but multiply. The empty list should produce `1` (the number that does not change a product).

```silica
fn product(numbers: List[int64, mem(normal)]) -> int64 {
    case numbers of {
        []: List[int64, mem(normal)] -> 1;
        [first: int64, rest: List[int64, mem(normal)]] -> first * product(rest);
        _: List[int64, mem(normal)] -> 1
    }
}
```

## 16. When something goes wrong

Some questions have no good numeric answer. What is ten divided by zero? What is the first element of an empty list?

A sloppy language might crash the whole program, or invent `0`, or keep running with a corrupt value. Silica does two better things. Inside a behaviour, you can **return the situation as data**. And when a calf does fail, its **mother is watching**.

Start with the mother. A dividing calf:

```silica
fn divider(msg: int64, state: int64) -> (:reply, int64, int64) {
    (:reply, 100 / msg, state)
}
```

`main` calls it with `0`, then with `5`:

```silica
        _: int64 <- call_registered(:divider, 0 impl ActorMessage {});
        answer: int64 <- call_registered(:divider, 5 impl ActorMessage {});
        _: atom <- print_int64(answer);
        println("")
```

Dividing by zero is not something a calf can do. The first call makes the calf fail. When you run the program you see the runtime report it:

```
=== Silica Actor Failure ===
actor_id:        0x10362f470
...
call_stack:
  #0  main_divider
  #1  _actor_thread_main
=== End Silica Actor Failure ===
20
```

Read the last line. After the failure report, the program prints `20`. The calf died, the supervisor noticed, and because the child's `restart` field in `init` says `:permanent`, she started a new calf with the same name and the same `initial_state`. The second call reached the new calf, and `100 / 5` is `20`. The rest of the program did not fall over. That is the elephant mother doing exactly what Chapter 2 promised: the child made a mistake, and she helped it get going again.

The numbers in `init`'s `flags` set her patience: `allowed_restart_count: 3, restarts_time_frame: 5` means "if a child fails more than three times in five seconds, something is really wrong; stop restarting and report upward."

A restart is a fresh start. The new calf's state is the `initial_state` from `init`, not whatever the old calf remembered. If a calf's memory matters, it should be told again, or kept somewhere it can be asked for.

Now the other half. Failing and restarting is right for mistakes. For expected situations, do not fail at all — make the situation a value:

```silica
fn safe_divide(x: int64, y: int64) -> { ok: boolean, value: int64 } {
    case y == 0 of {
        true -> { ok: false, value: 0 };
        false -> { ok: true, value: x / y }
    }
}

fn print_answer(n: int64) -> atom {
    sequence proc[device_io]
        _: atom <- print_int64(n);
        println("")
    produces
        pure :ok
    end
}

fn careful_divider(msg: int64, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        result: { ok: boolean, value: int64 } <- safe_divide(100, msg);
        _: atom <- case result.ok of {
            true -> print_answer(result.value);
            false -> println("cannot divide by zero")
        }
    produces
        pure (:no_reply, state)
    end
}
```

Cast `0` and then `5` to this calf and it prints `cannot divide by zero` and then `20`. No failure report, no restart. This calf never dies. The caller must look at `ok`, and the compiler's exhaustiveness check helps: if you forget the `false` arm, the `case` is incomplete and the program does not compile.

The important habit is this: **do not pretend a failure is a success.** Make the two cases visible. The next person to read the program — including you — should see that both paths exist.

Which to use? If a situation is part of the job — a user typed zero — handle it as data. If a situation should have been impossible — a bug — let the calf fail and the supervisor restart it. Silica gives you both, and the second one is free: every child in an `init` list already has a mother.

### Try this

1. What does `safe_divide(10, 0)` return? What does `safe_divide(10, 2)` return?
2. If `restart` in `init` said `:temporary` instead of `:permanent`, would the second call in the `divider` program still reach a calf?

Answers: (1) `{ ok: false, value: 0 }` and `{ ok: true, value: 5 }`. (2) No. `:temporary` means "never restart"; there would be no calf to reach.

## 17. Splitting a program into pieces

A tiny program can live in one file. A larger program should not. Silica programs are made of **modules**. A module is a file. The file name is the module name.

You have been reading a two-module program since Chapter 3. `adder.silica` is the module `adder`. It begins:

```silica
export add_and_print/2;
```

`export add_and_print/2;` means "this module offers `add_and_print`, which takes two arguments." The `/2` is the number of parameters. Nothing a module does not export can be seen from outside it.

`main.silica` begins:

```silica
use Supervisor;
use adder;
```

`use adder;` makes the module available. Then, inside `init`, the supervisor names the calf's behaviour as `adder@add_and_print`: "the `add_and_print` function from `adder`." The `@` is the module qualifier. It keeps names honest. You can see where a function lives.

`use Supervisor;` is the same idea. `Supervisor` is a module from Silica's standard library. It provides the trait that `impl AdderSupervisor for Supervisor;` fills in — the promise that `AdderSupervisor` has an `init` function of the right shape, so the runtime can call it and become the mother.

Helpers split out the same way. A module of arithmetic:

```silica
export add/2;
export double/1;

fn add(x: int64, y: int64) -> int64 {
    x + y
}

fn double(n: int64) -> int64 {
    n * 2
}
```

saved as `math_helpers.silica`, and a behaviour in another file that uses it:

```silica
use math_helpers;

export add_and_print/2;

fn add_and_print(msg: (int64, int64), state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        (left: int64, right: int64) <- msg;
        _: atom <- print_int64(math_helpers@add(math_helpers@double(left), math_helpers@double(right)));
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

This is how programs stay readable as they grow: each file has a job. One file does arithmetic helpers. One file is a calf's behaviour. `main` holds the supervisor and assembles them.

One rule of today's compiler is worth knowing: the `init` function of a supervisor lives in the same file as the `spawn_registered_supervisor` call that starts it. That is why the first program's `main.silica` holds both.

### Try this

If `math_helpers@double(5)` is `10`, what does the calf above print for the message `(5, 1)`?

Answer: `12`.

## 18. Many workers at once

An elephant mother rarely has one calf. A supervisor rarely has one child.

Everything you know already covers this. `init` returns a **list** of children. Put two records in it and the mother gives birth to two calves:

```silica
fn doubler(msg: int64, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- print_int64(msg * 2);
        println("")
    produces
        pure (:no_reply, state)
    end
}

fn squarer(msg: int64, state: int64) -> (:no_reply, int64) {
    sequence proc[device_io]
        _: atom <- print_int64(msg * msg);
        println("")
    produces
        pure (:no_reply, state)
    end
}
```

and in `init`:

```silica
        ] <- [
            {
                id: :doubler,
                agent_type: :worker,
                initial_state: 0,
                behavior: doubler,
                restart: :permanent,
                shutdown: 0,
                flavor: :plain
            },
            {
                id: :squarer,
                agent_type: :worker,
                initial_state: 0,
                behavior: squarer,
                restart: :permanent,
                shutdown: 0,
                flavor: :plain
            }
        ]
```

Now `main` can send `cast_registered(:doubler, 6 impl ActorMessage {})` and see `12`, or send to `:squarer` and see `36`. Both children in one `init` list share one behaviour type, so both are `fn(msg: int64, state: int64) -> (:no_reply, int64)`.

The picture to hold is a kitchen with several cooks. Each cook has a private counter. They do not grab ingredients off each other's counters. They pass notes. In Silica those cooks are actors, and this is what each one has:

- its own memory (its state)
- its own work (its behaviour)
- a mailbox for messages

Two calves really do work at the same time. If `main` casts to both, and both print, the two lines can come out in either order — or even glued together, one calf's text landing in the middle of the other's line. Nothing is wrong when that happens. Each calf handled its own message correctly; the world just heard them at once. Within **one** calf, order is guaranteed: messages are handled one at a time, in the order they were sent. Between calves, nothing is promised. When order matters, one actor should do the work, or one actor should collect the results.

The `strategy` in `init`'s `flags` says how the mother reacts when one calf among several fails:

- `:one_for_one` — restart only the calf that failed. This is the one every program in this book uses.
- `:one_for_all` — restart all of them, because they depend on each other.
- `:rest_for_one` — restart the one that failed and every calf listed after it.

That is the whole of concurrency in this book. Isolation first. Separate workers, private memory, notes instead of shared drawers, and a mother watching every one of them. That is how Silica keeps programs with many workers understandable.

The longer story — supervisors inside supervisors, `call` versus `cast` in detail, pinning work to a core — is in [Silica for Programmers]({{ '/learn-silica/' | relative_url }}) and in the [actor spawning tutorial](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/actor_spawning_tutorial.md).

### Try this

1. In the kitchen picture, why is "pass a note" safer than "reach over and change the other cook's bowl"?
2. With `:one_for_one`, if the squarer fails, what happens to the doubler?

Answers: (1) Because two people changing the same bowl at once can ruin the dish, and neither may notice. A note is received and handled as one piece of work. (2) Nothing. Only the squarer is restarted.

## 19. What the compiler is trying to tell you

When Silica refuses to compile a program, it is not being rude. It is pointing at a specific disagreement between what you wrote and what the language allows.

A refusal looks like this:

```
main.silica
 line: 76
 column: 44
E1040

expected ';' after this statement before the next statement
See specification: spec:sect3
```

Read the human sentence first. Then look at the file, line, and column. Then follow the `See specification` pointer if you want the formal rule.

You will meet messages about:

- **Types.** You sent a calf a string when its behaviour promised an `int64`, or listed a behaviour in `init` whose type does not match the promise written there.
- **Exhaustiveness.** A `case` forgot a possible shape of message.
- **Effects.** A behaviour printed without `device_io`, sent a message without `concurrency`, or built a list without `mem(normal)` on its sequence.
- **Names.** You used a name that was never bound, or you bound a name and never used it. (That is why the examples write `_` for values they do not need.)
- **Punctuation.** A missing semicolon between steps, as above.

A compiler error is not a verdict on you. It is a fact about the text. Change the text so the fact no longer holds.

Silica also refuses some programs that other compilers would "fix for you": unused names, duplicate work, arithmetic that cancels itself out. The language would rather you write what you mean than hope an optimizer guesses.

There is a second voice you have now heard: the runtime's failure report from Chapter 16. It speaks when a calf fails while running. The compiler speaks before the program ever runs. Learn to tell them apart. A compiler message means "fix the text." A failure report means "a calf died and its mother is handling it — now find out why."

### Try this

Which kind of error is this: a behaviour declared `-> (:no_reply, int64)` whose sequence produces `pure (:no_reply, "done")`?

Answer: a type error. The promised state and the actual state do not match.

## 20. Where to go next

You now have the core of programming:

- programs are precise instructions
- a program is a supervisor and the actors it gives birth to
- messages carry values, and values have types
- names stand for values inside one message
- behaviours are functions; helpers are functions too
- `case` chooses
- sequences order steps and declare effects
- state is what an actor remembers
- tuples, records, and lists group data
- recursion repeats work inside a message; the mailbox repeats work across messages
- expected failures are data; unexpected failures are restarted by the supervisor
- modules split a program
- many actors work at once, in isolation

That is enough to read small Silica programs and to write your own.

When you want the same language, explained for people who already program, read [Silica for Programmers]({{ '/learn-silica/' | relative_url }}).

When you want the rules in full, read the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md).

When you want hands-on topics — actors, regions, foreign functions, project makefiles — start from the [tutorials](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/tutorials_and_howtos).

When you want to run programs, [build the compiler]({{ '/build-and-test/' | relative_url }}).

Programming is a craft. The first programs will feel stiff. That is expected. Write small behaviours. Let the compiler talk. Let the supervisor watch. Change one thing at a time. The machine is literal. You can learn to be literal too.

*End of Learn to Program.*

Copyright © 2026 Lee Scott Barney
