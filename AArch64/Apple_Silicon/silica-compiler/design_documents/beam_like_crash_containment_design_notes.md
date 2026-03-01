# BEAM-like Lightweight Process Crash Containment (Native Binary Runtime)

> Living design document — intended for iterative refinement by human + LLM.

---

## 1. Goal

Provide BEAM-like lightweight process (LP) crash semantics in a compiled, native-language runtime:

- Lightweight processes have independent stacks and heaps
- A failure in one LP must not crash or corrupt other LPs
- “Let it crash” semantics apply at the LP level
- System continues execution after LP failure
- No VM (compiled to native binary)
- Language is functionally pure / memory-safe by construction
- Target architecture: AArch64
- Memory safety relies on hardware memory-safety features (e.g., ARM MTE)

This document describes how BEAM-style fault isolation can be approximated within a single OS process.

---

## 2. Core Observation

BEAM does **not** survive OS-level memory corruption.

BEAM survives because:

- Erlang code cannot express memory-unsafe operations
- All common failures are language-level exceptions
- Memory corruption inside the VM itself is fatal

Therefore the correct strategy is:

- Convert memory violations into deterministic traps
- Map those traps into LP-level crashes
- Continue execution only if runtime invariants are intact

This is not about “catching segfaults in general”.
It is about controlling *which faults are possible*.

---

## 3. Definitions

### Lightweight Process (LP)

A user-level execution unit:

- Own stack
- Own heap / arena
- No shared mutable memory
- Communicates via message passing
- Scheduled by runtime

### Runtime / Scheduler

Trusted core responsible for:

- LP lifecycle management
- Scheduling
- Message routing
- Global invariants

Corruption here is unrecoverable.

### Mutator Code

User program code executing inside an LP.

### Critical Runtime Section

Any code manipulating:

- Scheduler queues
- Global allocators
- Runtime metadata

Faults here must terminate the entire process.

---

## 4. Memory Safety Model

### Language Guarantees

- No raw pointers exposed to user code
- All references are safe handles
- Bounds checks enforced
- No undefined behavior expressible in the language

### Hardware Guarantees (AArch64)

- Memory Tagging Extension (MTE) enabled
- Tagged allocations
- Tagged pointers
- Tag mismatch triggers synchronous hardware exception

This converts memory misuse from silent corruption into deterministic faults.

---

## 5. Fundamental Principle

Only recover from a fault if **all** are true:

1. Fault was hardware-detected (e.g., MTE tag mismatch)
2. Fault occurred while executing LP mutator code
3. Runtime was not in a critical section
4. Scheduler state is known to be intact

Otherwise:

→ generate diagnostics
→ abort entire process

This mirrors BEAM’s true safety boundary.

---

## 6. Scheduler Architecture

### Per-OS-Thread Scheduler

Each OS thread maintains:

- Scheduler loop
- Thread-local `current_lp`
- Thread-local flag `in_mutator`

When executing user code:

```
in_mutator = true
current_lp = lp
```

When executing runtime code:

```
in_mutator = false
current_lp = null
```

These flags define the recovery boundary.

---

## 7. LP Execution Model

Before running an LP:

- Establish a recovery point
  - setjmp / longjmp
  - or compiler-supported exception frame

Execution flow:

```
scheduler_loop:
    lp = dequeue_ready_lp()
    establish_recovery_point()
    run_lp(lp)
    handle_exit_or_crash()
```

Outcomes:

- Normal return → reschedule or terminate LP
- Trap → LP marked crashed → cleanup → continue scheduling

LP never resumes after a crash.

---

## 8. Handling Memory Safety Faults

### Fault Source

On Linux/AArch64, MTE violations typically arrive as:

- SIGSEGV
- SIGBUS

(delivered synchronously)

### Signal Handling Requirements

- Use `sigaction`
- Use `sigaltstack`
- Handler must be async-signal-safe
- No allocation
- No locks
- No complex runtime logic

### Signal Handler Decision Logic

On fault:

1. Identify current OS thread
2. Read TLS:
   - `current_lp`
   - `in_mutator`
3. If:
   - `in_mutator == true`
   - `current_lp != null`

   Then:
   - record crash reason in LP metadata
   - non-local jump to scheduler recovery point

4. Else:
   - runtime integrity uncertain
   - produce crash dump
   - abort process

This is the central containment rule.

---

## 9. Recovery Mechanism

Recovery is implemented via non-local control transfer:

- `sigsetjmp()` before LP execution
- `siglongjmp()` from signal handler

After recovery:

- Scheduler resumes safely
- Faulting LP is dead
- Optional: notify linked LPs
- Optional: supervision restart

No attempt is made to continue execution inside the LP.

---

## 10. Memory Isolation Strategy

Even with MTE, structural isolation is required.

### Per-LP Heap

- Dedicated arena per LP
- Prefer mmap-backed regions
- No shared mutable heap state

### Message Passing

- Copy semantics preferred
- Or immutable shared buffers with ownership rules

### Stack Isolation

- Separate stack per LP
- Guard page at stack end
- Stack overflow becomes deterministic fault

---

## 11. Guard Pages

Used in addition to MTE:

- Stack overflow detection
- Heap boundary overrun detection
- Convert linear overruns into immediate traps

Guard pages complement tag-based protection.

---

## 12. What This Enables

This design enables:

- LP-level crash isolation
- Deterministic memory safety faults
- “Let it crash” semantics
- Continued system execution

Behavior is BEAM-equivalent for:

- Language-level exceptions
- Hardware-detected memory violations

---

## 13. What This Does NOT Guarantee

Not recoverable:

- Runtime bugs
- Scheduler corruption
- Faults in critical runtime sections
- Control-flow corruption
- Kernel-delivered fatal signals unrelated to memory tagging

These must terminate the process.

This is identical in philosophy to BEAM.

---

## 14. Recommended Safety Policy

### Development / Debug Mode

- Abort entire process on fault
- Emit rich diagnostics:
  - LP id
  - fault address
  - instruction pointer
  - scheduler state

### Production Mode

- Recover only from:
  - hardware memory-safety faults
  - occurring in LP mutator code

- Fail fast otherwise

---

## 15. Summary

BEAM-like crash containment in a native binary is achievable when:

- The language is memory-safe
- Hardware provides synchronous fault detection
- Runtime strictly separates mutator and scheduler execution
- Recovery is attempted only when invariants hold

The system does not attempt to survive arbitrary segfaults.

Instead, it constrains the fault model so that most failures become:

→ LP crashes, not system crashes.

This is the same philosophy as BEAM — applied using modern hardware support.

---

## 16. Open Questions (for iteration)

- Scheduler model: single-threaded vs multi-threaded
- LP preemption strategy
- Supervision semantics
- Restart policies
- Interaction with FFI (if ever allowed)
- Debugging and observability hooks

(These sections intentionally left open for future edits.)

