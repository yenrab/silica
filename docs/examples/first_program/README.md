# First program: a supervisor, one actor, and one message

The first program from *Learn to Program* (`docs/learn-programming.md`, Chapter 3).

## Files

- `main.silica` — starts the supervisor (the elephant mother), sends the actor one message carrying two numbers, and then waits so the program stays alive.
- `adder.silica` — the actor's behaviour: it adds the two numbers and prints the total.

`main.silica` also contains the supervisor's `init` function, which lists the one child the supervisor gives birth to. The compiler currently expects `init` in the same file as the `spawn_registered_supervisor` call.

The program also uses the standard-library `Supervisor` trait, `compiler/silica-compiler/stdlib/Supervisor.silica`. List it in `silica.config` beside these two files (the trials in `trials/supervisors_addition` show one way to do that).

## Expected output

```
5
```

After printing `5` the program waits. Type `exit` and press Enter (or close its input) and it ends with exit code `0`.

## Verified

Compiled and run with `binaries/silica-compiler` on 2026-09-18 (macOS, Apple silicon) through the trial `integrate` flow with a `.scout` golden of `5` followed by the exit code `0`.
