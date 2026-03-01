SILICA LANGUAGE DESIGN NOTES
=====================

HIGH-LEVEL GOALS
----------------
- A functional systems language designed for AArch64 (baseline M4 / Apple Silicon possible extensions).
- First-class ARM hardware exploitation: SVE/SVE2, NEON (opt-in modules), memory tagging (MTE), PAC where applicable.
- No C interop, no “legacy baggage”.
- Message passing / actor model is primary concurrency.
- Atomics are secondary but fully exposed and powerful.
- No raw pointers. No unsafe. Strongly typed, region-based memory.
- No loops. Recursion only. (Runtime is allowed to loop internally.)
- Core model: processes as monads; actors are structured specializations.
- Syntax optimized for structured readability and robustness for LLMs. Erlang-like instead of Lisp-like.


CORE EXECUTION MODEL
--------------------
- Two fundamental abstractions:
  1) Pure functions (ordinary functional FP).
  2) Processes represented as monadic computations:
        proc[Effects] Type
- Processes represent sequential effectful computations.
- Effects are explicit and part of type signatures.
- Monadic binding expressed via `do ... end` with `<-`, `return`.
- No implicit mutation. All state exists as:
  - actor state, or
  - region memory, with strong typing, or
  - atomic shared structures.


EFFECT SYSTEM
-------------
- Effects are explicit in proc type:
    proc[effect1, effect2, ...] Result
- Standard effects include:
  - mem(Space)
  - mailbox(Msg)
  - concurrency
  - atomic
  - device_io
- Effect aliases allowed:
    effect actor_eff(Msg) = [mailbox(Msg), concurrency].
- Effects are enforced at compile time.
- Monadic sequencing ensures all effect flow is explicit.


ACTOR MODEL
-----------
- Message passing first-class.
- Actors built *on top of* proc; they are not primitive at syntax level, but a defined convention:
    Actor behavior:
        Msg, State -> proc[mailbox(Msg), concurrency, ...] State
- Runtime provides:
  - mailbox per actor
  - scheduling
  - message passing
  - supervision/linking possible in later design

Core Actor Ops:
----------------
spawn_actor(InitState, Behavior)
    : proc[Eff + concurrency] actor_ref(Msg)

send(ActorRef, Msg)
    : proc[concurrency] unit

recv()
    : proc[mailbox(Msg), concurrency] Msg

self()
    : proc[mailbox(Msg), concurrency] actor_ref(Msg)

Actor semantics:
- Runtime repeatedly:
  - recv message
  - run behavior(Msg, State)
  - swap new State
  - loop
- User cannot write loops, but runtime can.


MEMORY MODEL: REGIONS & REFERENCES
-----------------------------------
- No raw pointers.
- Region-based ownership and lifetime.
- Typed access only.

Types:
-------
region(R, Space)
ref(R, Space, T)
buf(R, Space, T, N)

Allocate:
----------
alloc_region(Space)
    : proc[mem(Space)] region(R, Space)

alloc_ref(Region, Init)
    : proc[mem(Space)] ref(R, Space, T)

Operations:
------------
read_ref(Ref)   : proc[mem(Space)] T
write_ref(Ref,V): proc[mem(Space)] unit


VECTOR / ISA INTEGRATION
------------------------
- Vector features are NOT default.
- No hidden auto-vectorization semantics at language level.
- Programmer explicitly chooses modules:

    use module arch.sve
or
    use module arch.neon

- Core language has abstract vector types like:
    Vec(T)
    Pred (mask)
- Concrete representations determined by module selection.
- SVE module = scalable vectors.
- NEON module = 128-bit fixed width.
- Apple Silicon extensions live in Apple-specific module namespaces.
- Pure computations: no process effects required for vector code.


PROCESS MODEL
-------------
- Core computations expressed as:
    proc[Effects] ResultType
- Sequencing via:

    do
        X <- expr1,
        Y <- expr2,
        return Result
    end

- Internally equivalent to bind/return.
- Fully explicit sequencing.
- No hidden order or side effects.


ATOMIC MODEL
------------
Purpose:
--------
- Secondary concurrency tool for low-level systems code.
- For runtime, schedulers, lock-free structures.
- Never replaces message passing, but fully supported.

Atomic Types:
--------------
atomic_ref(R, Space, T)

Orderings:
-----------
type order =
    relaxed
  | acquire
  | release
  | acq_rel
  | seq_cst.

Core Atomic Primitives:
------------------------
alloc_atomic(Region, Init)
    : proc[mem(Space)] atomic_ref(R, Space, T)

atomic_load(ARef, Order)
    : proc[mem(Space), atomic] T

atomic_store(ARef, Val, Order)
    : proc[mem(Space), atomic] unit

atomic_fetch_add(ARef, Delta, Order)
    : proc[mem(Space), atomic] T

atomic_compare_exchange(ARef, Expected, NewVal, Order)
    : proc[mem(Space), atomic] {ok, T} | {fail, T}

Effect Profile:
----------------
Processes using atomics explicitly list:
    proc[mem(normal), atomic, ...]


LOCK-FREE SPSC QUEUE
--------------------
Representation:

spsc_queue(R,T) =
{
  buf,      buf(R, normal, T, Int),
  capacity, Int,
  head,     atomic_ref(R, normal, Int),
  tail,     atomic_ref(R, normal, Int)
}

Producer:
---------
- load Tail (acquire)
- load Head (acquire)
- check full
- write element normally
- atomic_store Tail (release)

Consumer:
---------
- load Head (acquire)
- load Tail (acquire)
- check empty
- read element normally
- atomic_store Head (release)

Used for:
---------
- Runtime queues
- Actor pipelines
- High-throughput coordination


MEMORY MODEL (LANGUAGE LEVEL)
-----------------------------
Per-actor execution: sequentially consistent inside actor.

Cross-actor ordering:
----------------------
Guaranteed ONLY through:
- Actor messaging:
    send → recv implies happens-before for data reachable by message
- Atomics using order semantics

Order Semantics:
----------------
relaxed
- coherence only, no ordering guarantees

acquire
- prevents reordering after load moving before it
- pairs with release

release
- prevents operations before store from moving after it

acq_rel
- both acquire + release guarantees

seq_cst
- acq_rel + participates in global total order of seq_cst ops
- strongest

ARM Mapping (conceptual):
--------------------------
load acquire → LDAR
store release → STLR
relaxed → LDR / STR
RMW → LDXR/STXR loops + barriers
seq_cst → acq_rel + explicit barrier if needed


SYNTAX (ERLANG-LIKE)
---------------------
Key ideas:
- , separates dependent statements
- ; separates branches
- . ends top-level declarations
- case … of … end
- do … end for proc sequencing
- functions defined with fn

Example:

fn example(X)
    : proc[mem(normal)] Int ->
    do
        Y <- read_ref(X),
        return Y + 1
    end.

Actor example:

type msg =
    inc
  | {get, actor_ref(Int)}.

effect actor_eff(Msg) = [mailbox(Msg), concurrency].

fn actor(Msg, State)
    : proc[actor_eff(msg)] Int ->
    case Msg of
        inc ->
            return State + 1;
        {get, ReplyTo} ->
            do
                send(ReplyTo, State),
                return State
            end
    end.


SUMMARY PRINCIPLES
------------------
1) Functional + explicit effects
2) Actors first class
3) Atomics second but powerful
4) Typed region memory, no raw pointers
5) Explicit parallelism, no implicit magic
6) ISA exposed via modules
7) Syntax stable & punctuation-structured for LLM robustness
8) Designed specifically for ARM64 realities