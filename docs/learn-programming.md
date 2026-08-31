---
title: Learn to Program
layout: default
permalink: /learn-programming/
---

# Learn to Program

**A first book of programming, taught in Silica**

Copyright © 2026 Lee Scott Barney

Use this book together with a large language model (an LLM — a chat program that can read these pages and answer questions) and the [Silica language documentation](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md). Point the LLM at both. Ask it to walk through examples, check your attempts, and explain a line you do not yet understand. Tell it what computer you are using. It will fill in the practical steps this book leaves out: how to compile a program, how to run it, and how to see the result on your machine. The book and the language documentation are the source of the rules; the LLM is there to help you learn to write programs and to understand how programming works.

This book is for people who have never written a computer program. You do not need to know what a compiler is, what a type is, or how a chip works. We start from one idea: a program is a set of instructions a machine can follow exactly. We write those instructions in [Silica]({{ '/' | relative_url }}).

If you already write software, this is the wrong book. Use [Silica for Programmers]({{ '/learn-silica/' | relative_url }}) instead.

On this page

1. [How to read this book](#1-how-to-read-this-book)
2. [What a program is](#2-what-a-program-is)
3. [Your first program](#3-your-first-program)
4. [Values](#4-values)
5. [Names](#5-names)
6. [Types](#6-types)
7. [Functions](#7-functions)
8. [Decisions](#8-decisions)
9. [Doing things in order](#9-doing-things-in-order)
10. [Talking to the outside world](#10-talking-to-the-outside-world)
11. [Text](#11-text)
12. [Grouping values](#12-grouping-values)
13. [Lists](#13-lists)
14. [Repeating work](#14-repeating-work)
15. [When something can go wrong](#15-when-something-can-go-wrong)
16. [Splitting a program into pieces](#16-splitting-a-program-into-pieces)
17. [Many workers at once](#17-many-workers-at-once)
18. [What the compiler is trying to tell you](#18-what-the-compiler-is-trying-to-tell-you)
19. [Where to go next](#19-where-to-go-next)



## 1. How to read this book

Read in order. Each chapter uses only ideas already introduced.

You do not have to run every example on a computer to learn from this book. Reading the code and predicting what it means is real practice. When you are ready to run programs, the [project page](https://github.com/yenrab/silica#building-the-compiler) explains how to build the Silica compiler.

A few habits will help:

- Read every example slowly. One unfamiliar word is enough reason to pause.
- When you see a new symbol, treat it as vocabulary, not as decoration.
- Try the short exercises at the ends of chapters. They are small on purpose.
- If a later chapter feels sudden, go back one chapter. That is normal.

Silica is still growing. The [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md) is the source of truth if this book and the compiler ever disagree.

## 2. What a program is

A computer is a machine that follows instructions. It is fast, literal, and unimaginative. It will not guess what you meant.

A **program** is a written list of those instructions. The computer does not understand English. We write in a **programming language** — a language designed so that each sentence has one meaning.

Cooking is a useful picture. A recipe says what to do, in what order, with which ingredients. If the recipe says “add 2 eggs,” the cook does not add three. A program is a recipe for a computer.

The language in this book is **Silica**. You will see words such as `fn`, `case`, and `sequence`. Those are part of Silica’s grammar, the same way “stir” and “bake” are part of a recipe’s grammar.

A program is stored in a **file**. Silica program files end in `.silica`. You can open them in any text editor. They are ordinary text, not a special secret format.

The computer cannot run the text of a Silica file directly. A program called a **compiler** reads your file, checks that it makes sense, and turns it into something the machine can execute. If the compiler finds a mistake, it stops and explains. That is a gift. A stopped program cannot quietly do the wrong thing.

Silica’s motto is: *secure by default at compile time — fail soft, never fail silent.* In beginner terms: the compiler tries to catch problems before they become surprises.

## 3. Your first program

Here is a complete Silica program:

```silica
fn main() -> int64 {
    42
}
```

This program does one thing: it finishes with the number `42`.

Three ideas are already here.

`fn` means “here is a function.” A **function** is a named piece of work. Most of this book is about functions.

`main` is the name of this function. When a Silica program starts, it begins at `main`. Think of `main` as the front door.

`int64` is a type. It means “a whole number that fits in 64 bits.” You do not need the hardware story yet. For now: `int64` is the everyday whole-number type in Silica.

The braces `{` and `}` mark the body of the function — the actual work. The body here is a single number. That number is the **result** of `main`.

Programs that only compute a result are already useful. Calculators, tax forms, and many games are “give me numbers, I will give you a number back.” We will add printing and files later. First we learn to compute.

### Try this

What is the result of this program?

```silica
fn main() -> int64 {
    7
}
```

Answer: `7`. The program is the same shape as the first one. Only the number changed.

## 4. Values

A **value** is a finished piece of data. `42` is a value. So is `true`. So is `"hello"`.

Silica has several everyday kinds of value.

**Whole numbers.** `0`, `1`, `42`, `-3`. These have type `int64` in ordinary programs.

**True or false.** `true` and `false`. These have type `boolean`. They answer yes-or-no questions.

**Text.** `"Ada"` and `"hello, world"` are **strings**. A string is a sequence of characters. The quotes are not part of the text; they mark where the text begins and ends.

**A single character.** `'A'` is a `char`. Notice the single quotes. `"A"` is a string of one character. `'A'` is the character itself.

**Nothing interesting.** `()` is the **unit** value. It means “there is no useful result here.” You will see it when a function’s job is to *do* something rather than *compute* something.

**Atoms.** `:ok` and `:error` are **atoms**. An atom is a named label. It is not a string. Two atoms are the same when they have the same name. Beginners can treat atoms as stickers you put on answers: “this one is okay,” “this one is an error.”

You can combine numbers with arithmetic:

```silica
2 + 3        // 5
10 - 4       // 6
3 * 7        // 21
20 / 6       // 3  (whole-number division, leftover is dropped)
20 % 6       // 2  (the leftover)
```

The marks `//` start a **comment**. The compiler ignores comments. They are notes for people.

You can compare values. Comparisons produce booleans:

```silica
3 < 5        // true
3 == 5       // false
3 != 5       // true
```

`==` asks “are these the same?” `!=` asks “are these different?”

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

1. What is `8 / 3` in Silica? Why is it not `2.666…`?
2. What is `(true or false) and false`?

Answers: (1) `2`, because `int64` division keeps only the whole part. (2) `false`.

## 5. Names

A value you will use again should have a **name**. In Silica you **bind** a name to a value with `<-`.

```silica
fn main() -> int64 {
    answer: int64 <- 42;
    answer
}
```

Read the middle line left to right: “`answer`, which is an `int64`, gets `42`.”

The name is `answer`. The type is `int64`. The value is `42`. After that line, you may use `answer` wherever you would have written `42`.

The semicolon ends the binding. The last line of a function is its result, and it has no semicolon.

A name in Silica is not a box you keep refilling. Once `answer` is bound, it stays `42` in that function. If you need a different value, you bind a new name, or you write a new binding that computes from the old one:

```silica
fn main() -> int64 {
    start: int64 <- 10;
    next: int64 <- start + 1;
    next
}
```

This program’s result is `11`. `start` is still `10`. `next` is a new name.

Why so strict? Because a name that never changes is a name you can trust. If `start` is `10` on line one, it is still `10` ten lines later. That makes programs easier to read and easier for the compiler to check.

You choose names. Good names read like English: `year`, `price`, `guest_count`. Silica uses **snake_case**: words in lowercase, separated by underscores.

### Try this

What is the result?

```silica
fn main() -> int64 {
    a: int64 <- 4;
    b: int64 <- 7;
    a * b
}
```

Answer: `28`.

## 6. Types

A **type** is a promise about a value. `int64` promises a whole number. `string` promises text. `boolean` promises `true` or `false`.

Silica writes types in the open. Function parameters have types. Bindings have types. Catch-all patterns have types. That looks wordy at first. It is deliberate. You should never have to guess what kind of thing a name is.

```silica
fn main() -> int64 {
    count: int64 <- 3;
    label: string <- "apples";
    ready: boolean <- true;
    count
}
```

`count`, `label`, and `ready` are three different kinds of value. You cannot add `count` to `label`. The compiler will refuse. That refusal is the type system doing its job.

A few types you will see often:


| Type      | Example values  | Everyday meaning |
| --------- | --------------- | ---------------- |
| `int64`   | `0`, `42`, `-1` | whole numbers    |
| `boolean` | `true`, `false` | yes or no        |
| `string`  | `"hello"`       | text             |
| `char`    | `'x'`           | one character    |
| `atom`    | `:ok`, `:error` | a named label    |
| `()`      | `()`            | no useful value  |


Silica also has smaller and larger number types (`int8`, `uint64`, `float64`, and others). You can ignore them until you have a reason. `int64` is the default whole number.

Types are not decoration. They are the first security feature you will feel. A program that tries to treat text as a number never becomes a running program. The compiler stops you at the door.

### Try this

Which line would the compiler reject, and why?

```silica
fn main() -> int64 {
    name: string <- "Nia";
    name + 1
}
```

Answer: `name + 1`. `name` is text. `1` is a number. Silica will not add them.

## 7. Functions

A function is a reusable piece of work with a name, inputs, and a result.

```silica
fn add(x: int64, y: int64) -> int64 {
    x + y
}

fn main() -> int64 {
    add(2, 3)
}
```

`add` takes two `int64` values, called `x` and `y`, and returns their sum. `main` **calls** `add` by writing the name and the values in parentheses.

Read `fn add(x: int64, y: int64) -> int64` as: “a function named `add`. It needs `x` and `y`, both whole numbers. It gives back a whole number.”

The names `x` and `y` belong to `add`. They are **parameters**. The values `2` and `3` in `add(2, 3)` are **arguments**. When the call happens, `x` is `2` and `y` is `3` for the duration of that call.

Functions keep programs small. Instead of copying the same arithmetic in ten places, you write it once and call it.

Every function in Silica lives at the **top level** of a file. You do not nest one `fn` inside another. If `main` needs help, write another function next to it.

A function may have at most eight parameters. If you need more, group related values (Chapter 12).

Functions can call other functions:

```silica
fn double(n: int64) -> int64 {
    n * 2
}

fn add(x: int64, y: int64) -> int64 {
    x + y
}

fn main() -> int64 {
    add(double(3), double(4))
}
```

`double(3)` is `6`. `double(4)` is `8`. `add(6, 8)` is `14`. The result of `main` is `14`.

### Try this

Write a function `triple` that multiplies its one `int64` parameter by `3`. Then write a `main` that returns `triple(5)`.

```silica
fn triple(n: int64) -> int64 {
    n * 3
}

fn main() -> int64 {
    triple(5)
}
```



## 8. Decisions

Programs choose. “If the number is positive, do this. Otherwise, do that.”

Silica has no standalone `if` statement. Every choice is a `case` expression. You give `case` a value, and you list the shapes that value might have. The matching arm is the result.

The simplest choice is a boolean:

```silica
fn sign_label(n: int64) -> string {
    case n >= 0 of {
        true -> "non-negative";
        false -> "negative"
    }
}
```

`n >= 0` is `true` or `false`. The `case` picks the matching arm. Each arm has the form `pattern -> result`. Arms are separated by semicolons.

You can match the number itself, with a **guard** after `if`:

```silica
fn describe(n: int64) -> string {
    case n of {
        x: int64 if x > 0 -> "positive";
        x: int64 if x < 0 -> "negative";
        _: int64 -> "zero"
    }
}
```

`x: int64 if x > 0` means: “bind the number to `x`, but only take this arm when `x` is greater than zero.”

`_: int64` is the **catch-all**. The underscore means “I do not need the value.” The type is still required. Silica will not accept a bare `_`.

`case` must be **exhaustive**. If a value could arrive that no arm covers, the compiler rejects the program. That is another gift. Forgotten cases are a famous source of bugs in other languages.

`case` is an expression, so it has a result. You can bind that result:

```silica
fn abs(n: int64) -> int64 {
    result: int64 <- case n >= 0 of {
        true -> n;
        false -> 0 - n
    };
    result
}
```

Or you can return the `case` directly, as `sign_label` and `describe` do.

### Try this

1. What does `describe(0)` return?
2. What does `describe(-4)` return?

Answers: (1) `"zero"`. (2) `"negative"`.

## 9. Doing things in order

Some work has several steps. A **sequence** runs those steps from top to bottom and then produces a result.

```silica
fn main() -> int64 {
    sequence
        start: int64 <- 10;
        next: int64 <- start + 2;
        last: int64 <- next * 3;
    produces
        pure last
    end
}
```

Read this as: do these bindings, in order. Then produce `last`. The word `pure` means the result itself does not start any new outside-world work.

The result is `36`: `10`, then `12`, then `36`.

A function body can already hold more than one binding. Sequences become important when the steps have **effects** — when they touch the world outside the calculation. That is the next chapter. For now, a sequence is “a list of steps with a clearly marked result.”

Notice `end`. A sequence is a block. It starts with `sequence` and finishes with `end`. The `produces` line is the doorway out.

### Try this

What does this produce?

```silica
fn main() -> int64 {
    sequence
        a: int64 <- 5;
        b: int64 <- a + a;
    produces
        pure b
    end
}
```

Answer: `10`.

## 10. Talking to the outside world

So far every program has only computed a value. Real programs also print, read files, and send messages. Those actions are **effects**. They change something you can notice outside the function’s result.

Silica makes effects visible. You declare them on the sequence that performs them.

```silica
fn main() -> int64 {
    sequence proc[device_io]
        println("Hello, world");
    produces
        pure 0
    end
}
```

`proc[device_io]` says: this sequence may do device I/O. In beginner programs, that mostly means printing and files.

`println` writes a line of text. The quotes are a string. After this program runs, you should see `Hello, world` on the screen. The function still returns `0`, a conventional “everything went fine” number.

Why the ceremony? Because a function that only adds numbers and a function that writes a file are different kinds of thing. Silica will not let you hide a write inside an innocent-looking helper. If something prints, a sequence above it must admit `device_io`.

That is not bureaucracy for its own sake. It is honesty. When you read a program six months later, the word `device_io` tells you: this part talks to the world.

You can print more than once:

```silica
fn main() -> int64 {
    sequence proc[device_io]
        println("first");
        println("second");
    produces
        pure 0
    end
}
```

The lines appear in that order.

Silica also has `print`, which writes text without adding a new line, and helpers such as `print_int64` for numbers. All of them need `device_io`.

### Try this

Add a third `println` to the program above so it prints `first`, `second`, and `third` on three lines.

## 11. Text

Strings are values. You can pass them to functions, return them, and bind them to names.

```silica
fn greet(name: string) -> string {
    concat("Hello, ", name)
}
```

`concat` joins two strings. `greet("Ada")` is `"Hello, Ada"`.

Useful questions about text:

```silica
length_chars("Ada")           // 3
starts_with("Ada", "A")       // true
contains("Ada Lovelace", "Love")
```

`length_chars` counts characters. `length_bytes` counts the underlying UTF-8 bytes. For English letters they often match. For many other languages and for emoji they may not. When you care about “how many letters,” use `length_chars`.

You can choose a piece of a string with `substring`. The positions count characters, starting at `0` for the first character.

Strings are not numbers. `"3"` and `3` are different values with different types. If you need to turn a number into text, use a conversion such as `int_to_string`. If you need to turn text into a number, that can fail — `"xyz"` is not a number — and Chapter 15 shows how Silica represents that kind of failure.

### Try this

What does `greet("Nia")` return?

Answer: `"Hello, Nia"`.

## 12. Grouping values

Sometimes two or three values belong together: a first and last name, a width and a height, a status and a number.

A **tuple** is an ordered group. The position is what matters.

```silica
fn main() -> int64 {
    pair: (int64, int64) <- (3, 4);
    (x: int64, y: int64) <- pair;
    x + y
}
```

`(3, 4)` is a pair. `(int64, int64)` is its type. The second binding **unpacks** the pair into `x` and `y`. The result is `7`.

A **record** is a group with field names. The name is what matters.

```silica
fn area(rect: { width: int64, height: int64 }) -> int64 {
    rect.width * rect.height
}

fn main() -> int64 {
    box: { width: int64, height: int64 } <- { width: 3, height: 4 };
    area(box)
}
```

`{ width: 3, height: 4 }` is a record value. `{ width: int64, height: int64 }` is a record type. `rect.width` reads the `width` field.

Silica does not ask you to invent a new type name for every record. You write the shape where you need it. Two records with the same fields and field types are the same kind of thing.

Atoms pair well with tuples when you want a labeled result:

```silica
fn safe_divide(x: int64, y: int64) -> (atom, int64) {
    case y == 0 of {
        true -> (:error, 0);
        false -> (:ok, x / y)
    }
}
```

`(:ok, 5)` means “it worked, and the answer is 5.” `(:error, 0)` means “it failed.” Chapter 15 builds on this.

### Try this

What does `area({ width: 2, height: 10 })` return?

Answer: `20`.

## 13. Lists

A **list** is a sequence of values of the same type: three numbers, a dozen names, no items at all.

```silica
fn main() -> int64 {
    numbers: List[int64] <- [1, 2, 3]: List[int64];
    length[int64](numbers)
}
```

Two things are easy to miss.

The type is written `List[int64]`, not just `List`. Every list knows its element type.

The literal needs the same annotation: `[1, 2, 3]: List[int64]`. Silica will not guess the element type from the brackets alone.

Lists are **immutable**. Operations return a new list. The old list is still the old list.

```silica
fn demo() -> int64 {
    sequence proc[mem(normal)]
        numbers: List[int64] <- [2, 3]: List[int64];
        more: List[int64] <- prepend[int64](1, numbers);
    produces
        pure length[int64](more)
    end
}
```

`numbers` is still `[2, 3]`. `more` is `[1, 2, 3]`. `prepend` adds at the **front**. Growing a list needs memory, so this sequence declares `mem(normal)` — ordinary memory.

Useful list questions:

- `head[int64](numbers)` — the first element (do not call this on an empty list)
- `tail[int64](numbers)` — everything except the first element
- `is_empty[int64](numbers)` — `true` when there are no elements
- `length[int64](numbers)` — how many elements

You cannot reach into the middle and pluck an item with a special “item 7” operation. You work from the front, or you write a function that walks the list (Chapter 14). That sounds limiting. It is also simple. There is one way to take a list apart: look at the head, then look at the rest.

An empty list still has a type: `[]: List[string]` is an empty list that would hold strings.

### Try this

If `names` is `["Ada", "Nia"]: List[string]`, what is `head[string](names)`? What is `tail[string](names)`?

Answers: `"Ada"`, and `["Nia"]: List[string]`.

## 14. Repeating work

Many languages have loops: “do this ten times,” “keep going while this is true.” Silica does not. When you need to repeat work, you write a **recursive** function: a function that calls itself, each time on a smaller piece of the problem, until a simple case remains.

That sounds abstract. It is the same idea as a stack of plates. To wash the stack: wash the top plate, then wash the remaining stack. The empty stack is the simple case — you are done.

Sum a list of numbers:

```silica
fn sum(numbers: List[int64]) -> int64 {
    case is_empty[int64](numbers) of {
        true -> 0;
        false -> head[int64](numbers) + sum(tail[int64](numbers))
    }
}
```

If the list is empty, the sum is `0`. Otherwise the sum is the first number plus the sum of the rest.

Walk it with `[1, 2, 3]`:

- `1 + sum([2, 3])`
- `1 + (2 + sum([3]))`
- `1 + (2 + (3 + sum([])))`
- `1 + (2 + (3 + 0))`
- `6`

The empty list is what stops the repetition. Every recursive function needs a case that does **not** call itself. Forgetting that case is how you ask a computer to work forever.

Count down to zero:

```silica
fn sum_to(n: int64) -> int64 {
    case n <= 0 of {
        true -> 0;
        false -> n + sum_to(n - 1)
    }
}
```

`sum_to(3)` is `3 + 2 + 1 + 0`, which is `6`.

Recursion is the ordinary way to walk data in Silica. The runtime is allowed to turn a well-written recursive function into an efficient loop internally. You still write the idea as “solve the small piece, then the rest.”

### Try this

What is `sum([4, 1]: List[int64])`?

Answer: `5`.

Write `product`, like `sum`, but multiply. The empty list should produce `1` (the number that does not change a product).

```silica
fn product(numbers: List[int64]) -> int64 {
    case is_empty[int64](numbers) of {
        true -> 1;
        false -> head[int64](numbers) * product(tail[int64](numbers))
    }
}
```



## 15. When something can go wrong

Some questions have no good numeric answer. What is ten divided by zero? What is the first element of an empty list?

A sloppy language might crash, or invent `0`, or keep running with a corrupt value. Silica prefers that you **return the situation as data**.

You have already seen the pattern:

```silica
fn safe_divide(x: int64, y: int64) -> (atom, int64) {
    case y == 0 of {
        true -> (:error, 0);
        false -> (:ok, x / y)
    }
}

fn label(result: (atom, int64)) -> string {
    case result of {
        (:ok, n: int64) -> concat("answer is ready", "");
        (:error, _: int64) -> "cannot divide by zero"
    }
}
```

The caller must look at the atom. The compiler’s exhaustiveness check helps: if you forget `:error`, `case` is incomplete.

You can do the same with a record:

```silica
fn find_start(words: List[string]) -> { ok: boolean, value: string } {
    case is_empty[string](words) of {
        true -> { ok: false, value: "" };
        false -> { ok: true, value: head[string](words) }
    }
}
```

The important habit is this: **do not pretend a failure is a success.** Make the two cases visible. The next person to read the program — including you — should see that both paths exist.

Not every failure belongs in your own return value. If a function is called with a value that should have been impossible, that is a programming mistake. The compiler tries to catch those. Runtime crashes still exist for things like `head` on an empty list. Prefer a `case` that handles emptiness yourself.

### Try this

What does `safe_divide(10, 0)` return? What does `safe_divide(10, 2)` return?

Answers: `(:error, 0)` and `(:ok, 5)`.

## 16. Splitting a program into pieces

A tiny program can live in one file. A larger program should not. Silica programs are made of **modules**. A module is a file. The file name is the module name.

A helper module exports the functions other files may use:

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

`export add/2;` means “this module offers `add`, which takes two arguments.” The `/2` is the number of parameters. `double/1` takes one.

A second file can use that module:

```silica
use math_helpers;

fn main() -> int64 {
    math_helpers@add(2, math_helpers@double(5))
}
```

`use math_helpers;` makes the module available. `math_helpers@add` is “the `add` function from `math_helpers`.” The `@` is the module qualifier. It keeps names honest. You can see where a function lives.

This is how programs stay readable as they grow: each file has a job. One file does arithmetic helpers. One file does greetings. `main` assembles them.

### Try this

If `math_helpers@double(5)` is `10`, what is the result of `main` above?

Answer: `12`.

## 17. Many workers at once

So far one program has done one thing at a time. Many useful programs do many things at once: a server talking to several guests, a sensor watching a pin while another piece of code writes a log.

The picture to hold is a kitchen with several cooks. Each cook has a private counter. They do not grab ingredients off each other’s counters. They pass notes.

In Silica those cooks are **actors**. Each actor has:

- its own memory
- its own work
- a mailbox for messages

You start an actor with `spawn`. You send a message with `call` (wait for an answer) or `cast` (do not wait). The other actor’s function runs once per message and returns a new version of its private state.

```silica
fn counter(msg: int64, state: int64) -> (:reply, int64, int64) {
    total: int64 <- state + msg;
    (:reply, total, total)
}

fn main() -> int64 {
    sequence proc[concurrency]
        desk: actor_ref <- spawn(0, counter);
        first: int64 <- call(desk, 1 impl ActorMessage {});
        second: int64 <- call(desk, 4 impl ActorMessage {});
    produces
        pure second
    end
}
```

`spawn(0, counter)` starts an actor whose private number begins at `0`. Each `call` adds a number and gets the new total back. After `1` and then `4`, `second` is `5`.

`concurrency` is the effect that admits “I am creating actors or sending messages.” `impl ActorMessage {}` is the marker that says “this value is allowed to be a message.”

You do not need to master actors on the first reading. The idea to keep is: **isolation first.** Separate workers, private memory, notes instead of shared drawers. That is how Silica keeps concurrent programs understandable.

The longer story — supervisors, failure, pinning work to a core — is in [Silica for Programmers]({{ '/learn-silica/' | relative_url }}) and in the [actor spawning tutorial](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/tutorials_and_howtos/actor_spawning_tutorial.md).

### Try this

In the kitchen picture, why is “pass a note” safer than “reach over and change the other cook’s bowl”?

Answer: because two people changing the same bowl at once can ruin the dish, and neither may notice. A note is received and handled as one piece of work.

## 18. What the compiler is trying to tell you

When Silica refuses to compile a program, it is not being rude. It is pointing at a specific disagreement between what you wrote and what the language allows.

You will meet messages about:

- **Types.** You passed a string where an `int64` was required.
- **Exhaustiveness.** A `case` forgot a possible shape.
- **Effects.** You printed (or spawned, or allocated) without declaring the matching effect on a sequence.
- **Names.** You used a name that was never bound, or you bound a name and never used it.

Read the human sentence first. Then look at the file, line, and column. Then follow the `See specification` pointer if you want the formal rule.

A compiler error is not a verdict on you. It is a fact about the text. Change the text so the fact no longer holds.

Silica also refuses some programs that other compilers would “fix for you”: unused names, duplicate work, arithmetic that cancels itself out. The language would rather you write what you mean than hope an optimizer guesses.

### Try this

Which kind of error is this: a function declared `-> int64` but its body is `"hello"`?

Answer: a type error. The promised result and the actual result do not match.

## 19. Where to go next

You now have the core of programming:

- programs are precise instructions
- values have types
- names stand for values
- functions package work
- `case` chooses
- sequences order steps and declare effects
- lists and records group data
- recursion repeats work
- failures can be returned as data
- modules split a program
- actors isolate concurrent work

That is enough to read small Silica programs and to write your own.

When you want the same language, explained for people who already program, read [Silica for Programmers]({{ '/learn-silica/' | relative_url }}).

When you want the rules in full, read the [language specification](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/design_documents/silica-specification.md).

When you want hands-on topics — actors, regions, foreign functions, project makefiles — start from the [tutorials](https://github.com/yenrab/silica/tree/main/compiler/silica-compiler/tutorials_and_howtos).

When you want to run programs, [build the compiler](https://github.com/yenrab/silica#building-the-compiler).

Programming is a craft. The first programs will feel stiff. That is expected. Write small functions. Let the compiler talk. Change one thing at a time. The machine is literal. You can learn to be literal too.

*End of Learn to Program.*

Copyright © 2026 Lee Scott Barney