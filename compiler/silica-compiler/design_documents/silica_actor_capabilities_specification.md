# Silica Actor Capabilities and Message Ordering Specification

**Status:** Draft extension. Adds protocol-typed actor references, capability splitting, and formal message-order guarantees.

**Related Documents**

| Document | Purpose |
|----------|---------|
| [silica-specification.md](silica-specification.md) | Core language specification |
| [silica-formal-verification-specification.md](silica-formal-verification-specification.md) | Formal verification framework |
| [silica-specification-additional.md](silica-specification-additional.md) | Compiler failure rules |

**Research Basis**

This extension is informed in part by Colin S. Gordon's work on actor capabilities and message ordering, especially:

- Gordon, Colin S. *Actor Capabilities for Message Ordering*.

---

## 1. Introduction

### 1.1 Overview
This document extends Silica's actor model with **actor capabilities**: protocol-typed actor references that restrict not only which message shapes may be sent, but also the **order** in which messages may be sent through a particular reference.

The goal is to statically prevent a class of actor bugs where messages arrive at a valid actor in an invalid lifecycle phase. Examples include sending `commit` before `begin`, `close` before `open`, or sending a one-shot control message twice.

### 1.2 Design Goals
- Preserve Silica's existing actor model while making protocol typing universal
- Reuse Silica's explicit effect system rather than introducing a wholly separate checking framework
- Support local reasoning: correctness should be checkable from the actor behavior and the capability type carried by the actor reference
- Integrate with existing FIFO mailbox semantics without changing runtime message ordering
- Fit Silica's no-generics, explicit-type, LLM-friendly design philosophy
- Keep opting out of ordering restrictions explicit rather than implicit

### 1.3 Non-Goals
This document does not:
- Introduce full multiparty session types
- Require changes to Silica's region-based memory model
- Change the runtime mailbox ordering semantics already defined by Silica
- Replace Silica's existing call/cast distinction with a new runtime messaging model

---

## 2. Summary of the Extension

### 2.1 New Surface Concept
Silica actor references are uniformly protocol-typed:

```silica
actor_ref with Protocol end
```

where `Protocol` is a compile-time protocol expression describing the allowed sequence of messages sent through that reference.

Examples:

```silica
actor_ref with :dynamic end
actor_ref with :open then :write repeat then :close end
actor_ref with :begin then :commit end
actor_ref with :heartbeat repeat end
actor_ref with :request then :reply_port_handoff end
```

### 2.2 Mandatory Protocol Typing
This extension adopts a **uniform mandatory protocol model**:
- every actor reference has the form `actor_ref with Protocol end`
- every spawned actor declares a protocol
- opting out of ordering restrictions is still explicit

The distinguished built-in protocol:

```silica
actor_ref with :dynamic end
```

means:

```text
no static ordering restriction beyond ordinary actor message typing and call/cast mode checks
```

### 2.3 Key Semantic Idea
A value of type `actor_ref with P end` is both:
1. a handle to an actor, and
2. a static capability that permits only message sequences described by `P`

Sending a message through such a reference **consumes** part of the protocol, except for the unrestricted built-in protocol `:dynamic`. Therefore, protocol actor references are **flow-sensitive**.

For `call()`-compatible protocol actors, the intended guarantee is actor-global: protocol conformance is checked against the order of messages actually received by the actor across all outstanding capabilities together, not merely against each individual capability in isolation.

For `cast()`-compatible protocol actors, the guarantee is weaker: the protocol constrains what may be sent through each capability, but the runtime does not defer, reorder, or phase-buffer incoming cast messages to force a global received-order protocol.

### 2.4 Separation of Unrestricted and Exhausted Protocols
This specification distinguishes two different built-in meanings:

- `:dynamic` — unrestricted protocol; ordering is not statically constrained
- `epsilon` — exhausted protocol; no further messages may be sent

These meanings must never be conflated.

---

## 3. Protocol Language

### 3.1 Protocol Expressions
The surface language introduces protocol expressions:

```ebnf
protocol_expression ::= protocol_atom
                      | protocol_expression "then" protocol_expression
                      | protocol_expression "or" protocol_expression
                      | protocol_expression "interleave" protocol_expression
                      | protocol_expression "repeat"
                      | protocol_expression "optional"
                      | "(" protocol_expression ")"

protocol_atom ::= ":" identifier
                | "message" "(" record_type ")"
                | "epsilon"
```

### 3.2 Intended Reading
- `P then Q` — first a sequence in `P`, then a sequence in `Q`
- `P or Q` — either branch is allowed
- `P repeat` — zero or more repetitions of `P`
- `P optional` — zero or one occurrence of `P`
- `P interleave Q` — all shuffles of sequences from `P` and `Q`
- `epsilon` — the empty protocol

### 3.3 Required Surface Support
The implementation must support the full user-visible protocol surface defined in this document.

`interleave` is part of the required user-visible syntax and semantics, not merely an internal typing construct.

### 3.4 Protocol Labels
A protocol step denotes a **message class**. The compiler maps each sent message to a protocol label using the **atom-commanded record message** mechanism:

```silica
{ command: :open, path: string }
{ command: :write, bytes: buf(R, normal, uint8, N) }
{ command: :close }
```

Protocol labels are always drawn from the `command: atom` field of actor messages.

### 3.5 Canonical Message Label Rule
Every protocol-governed message must map its `command: atom` field to a protocol label. A conforming implementation must use atom-commanded record messages as the sole mechanism for protocol labeling.

Exact record-shape matching through `message({ ... })` is not used for protocol label determination.

---

## 4. New Types and Type Forms

### 4.1 Actor Reference Types
Silica uses a single actor reference form:

```silica
actor_ref with Protocol end
```

There is no unparameterized `actor_ref` in this extension.

### 4.2 Built-In Protocol Constants
The language defines the following distinguished protocol constants:

```text
:dynamic   unrestricted protocol
epsilon    exhausted protocol
```

`actor_ref with :dynamic end` may be used wherever the programmer wishes to opt out of static ordering restrictions.

`actor_ref with epsilon end` denotes a capability that cannot be used with `call()` or `cast()`.

### 4.3 Behavior Capability Types
This document introduces an internal behavior type form used by the compiler and formal specification:

```text
behavior[RecvProtocol, Mode, State, Reply]
```

where:
- `RecvProtocol` is the protocol the actor is prepared to handle across future messages
- `Mode` is `call_only` or `cast_only`
- `State` is the actor state type
- `Reply` is the reply type for `call_only` actors, or `unit` for `cast_only`

This internal form need not appear in user syntax.

### 4.4 Self Capability Type
Inside a protocol actor, `self()` has a protocol type:

```silica
self() -> actor_ref with SelfProtocol end
```

The exact `SelfProtocol` is tracked flow-sensitively by the compiler and may evolve between message-handling phases.

## 5. Actor Declaration and Spawn

### 5.1 Protocol-Governed Spawn
The surface language uses a protocol-delimited spawn form:

```silica
spawn with Protocol end (initial_state, behavior_fn) -> actor_ref with Protocol end
```

Example:

```silica
file_actor: actor_ref with :open then (:write repeat) then :close end <-
    spawn with :open then (:write repeat) then :close end (initial_state, file_behavior)
```

### 5.2 Mandatory Spawn Annotation
Every spawned actor must declare a protocol explicitly:

```silica
spawn with :dynamic end (initial_state, behavior_fn) -> actor_ref with :dynamic end
spawn with :open then (:write repeat) then :close end (initial_state, behavior_fn)
    -> actor_ref with :open then (:write repeat) then :close end
```

### 5.3 Spawn Typing Rule
If the result type is `actor_ref with P end`, then the compiler must verify that:
- the actor's behavior is protocol-compatible with `P`
- the actor's mailbox handling remains safe for every sequence allowed by `P`
- the actor's future behavior transitions do not invalidate already-issued capabilities

### 5.4 Required Implementation Rule
All protocol features described in this document are part of the required implementation surface.

No staged rollout, reduced subset, or partial protocol checker is assumed by this specification.

## 6. Sending Through Protocol References

### 6.1 Actor Operations
Silica actor operations are interpreted over protocol-typed actor references:

```silica
call(actor: actor_ref with P end, message: ActorMessage) -> Reply proc[concurrency]
cast(actor: actor_ref with P end, message: ActorMessage) -> unit proc[concurrency]
```

provided the sent message matches the next allowed protocol step and the actor mode is compatible.

### 6.2 Flow-Sensitive Consumption Rule
After sending a message with protocol label `L` through `actor_ref with P end`, the reference is updated to:

```text
actor_ref with derive(P, L) end
```

where `derive(P, L)` is the protocol derivative: the remaining protocol after consuming one `L` step.

### 6.3 Example

```silica
sequence proc[concurrency]
    f: actor_ref with :open then (:write repeat) then :close end <- open_file_actor
    cast(f, { command: :open, path: "/tmp/x" } impl ActorMessage {})
    // f is now actor_ref with (:write repeat) then :close end

    cast(f, { command: :write, bytes: chunk } impl ActorMessage {})
    // f is still actor_ref with (:write repeat) then :close end

    cast(f, { command: :close } impl ActorMessage {})
    // f is now actor_ref with epsilon end
produces
    pure ()
end
```

### 6.4 Terminal Capability Rule
`actor_ref with epsilon end` is a valid value but cannot be used with `call()` or `cast()`.

Any attempt to send through `actor_ref with epsilon end` is a compile-time error.

### 6.5 Unrestricted Capability Rule
For the built-in unrestricted protocol:

```text
derive(:dynamic, label) = :dynamic
```

A value of type `actor_ref with :dynamic end` therefore remains `actor_ref with :dynamic end` after any well-typed `call()` or `cast()`.

### 6.6 Protocols Govern Outgoing Messages Only
Protocols do not govern replies from actors. The protocol `P` in `actor_ref with P end` constrains only the sequence of outgoing messages that may be sent through the reference. When a `call()` receives a reply, the reply type is determined by the actor's declared reply type, not by protocol rules.

Reply protocols are not modeled explicitly in the surface language.

## 7. Capability Splitting and Copying

### 7.1 Motivation
In Silica, some values are copyable while others are move-only (e.g., region handles cannot be copied). Protocol actor references, like region handles, can present similar constraints: unrestricted copying could duplicate permission to send one-shot messages.

### 7.2 Splitting Judgment
The compiler introduces an internal judgment:

```text
split(P) => (P1, P2)
```

which is valid only when:

```text
interleave(P1, P2) is contained in P
```

### 7.3 User-Level Rule
A value of type `actor_ref with P end` may be:
- moved freely
- copied freely

Each copy of a reference maintains its own independent protocol state.

### 7.4 Independent Protocol State Tracking
When a reference is copied, each copy tracks its own protocol state independently. The implementation details of how independent states are reconciled at the actor's mailbox are deferred to runtime and compiler design, but the requirement is:

**Each outstanding copy of `actor_ref with P end` maintains its own flow-sensitive protocol position, independently of other copies of the same underlying actor reference.**

### 7.5 Capability Splitting
Splitting, as defined in section 7.2, remains a valid operation for cases where a protocol should be deliberately partitioned across multiple references with complementary protocol obligations.

## 8. Interaction with Existing Call-Only and Cast-Only Actors

### 8.1 Existing Distinction
Silica already distinguishes actors whose behaviors are call-only or cast-only.

### 8.2 Protocol Consistency Rule
A protocol actor must also be mode-consistent:
- `call()` may only be used with call-compatible protocol actors
- `cast()` may only be used with cast-compatible protocol actors

Mode also affects the interpretation of protocol ordering:
- for `call()` actors, protocol conformance is checked against the order of messages actually received by the actor across all outstanding capabilities together
- for `cast()` actors, protocol conformance constrains capability use statically, but does not add runtime mailbox deferral, reordering, or phase-blocking behavior

### 8.3 Internal Typing Separation
The compiler must track both:
- **mode safety**: call-only vs cast-only
- **protocol safety**: message ordering and capability consumption

These are orthogonal checks.

### 8.4 Required Design Constraint
A protocol actor declaration must choose exactly one convention:

```text
protocol call actor
protocol cast actor
```

Mixed-mode protocol actors are not permitted by this specification.

---

## 9. Behavior Semantics for Protocol Actors

### 9.1 Behavior Transition Model
A protocol actor's behavior is modeled as a state machine over message commands.

Each handled message produces:
- the next actor state
- optionally a reply
- the next behavior phase

### 9.2 Surface-Level Programming Model
Silica does not need new runtime actor machinery. Protocol actors are a **static typing discipline** over the existing mailbox and behavior semantics.

Protocols constrain message sending statically through actor capabilities. They do not introduce runtime mailbox deferral, reordering, or phase-blocking semantics. Any message ordering guarantees arise from compile-time rejection of invalid sends and from proof obligations that the actor remains prepared for all sequences permitted by its outstanding capabilities.

For `call()` actors, the protocol discipline is interpreted globally over the actor's received message order across all outstanding capabilities.

For `cast()` actors, the discipline remains per-capability and static: casts are not buffered or delayed by the runtime to force a protocol phase order.

### 9.3 Recommended Source-Level Style
Protocol actors should be written using explicit state records whose `phase` field mirrors the protocol phase.

Example:

```silica
{ phase: :closed, handle: OptionHandle }
{ phase: :opened, handle: FileHandle }
```

The compiler should verify that message handling branches are consistent with the declared protocol.

### 9.4 Phase Consistency Rule
If a behavior issues a restricted self-capability or returns a future behavior phase, the actor must remain prepared to handle every message sequence permitted by all outstanding capabilities previously given to other actors.

### 9.5 Hot-Swapped Behavior Protocol Preservation
When an actor's behavior is hot-swapped (replaced with a new behavior function during execution), the new behavior must preserve the protocol declared at spawn time. The new behavior may not refine, widen, or otherwise alter the protocol that was promised to all outstanding capabilities.

The protocol remains invariant across behavior transitions.

---

## 10. Effects and Protocol Obligations

### 10.1 Reuse of the Existing Effect System
Silica already uses explicit effect declarations on sequence blocks. This extension reuses that design by introducing internal **protocol obligations** tracked alongside ordinary effects.

### 10.2 Internal Judgment Shape
The formal verification layer should extend typing judgments from:

```text
Γ; L; ScopDep ⊢ e : T; L'; ScopDep'
```

to:

```text
Γ; L; ScopDep; Ω ⊢ e : T; L'; ScopDep'; Ω'
```

where `Ω` is a protocol obligation environment.

### 10.3 Meaning of Ω
`Ω` records obligations induced by:
- issued self-capabilities
- protocol splits
- future messages the actor must remain prepared to receive

### 10.4 Local Soundness Goal
The system must guarantee:

> If a program type-checks, then every message sent through a well-typed `actor_ref with P end` is accepted by the target actor in a mailbox state consistent with the remaining protocol of that capability.

For `call()` actors, this guarantee is strengthened to the actor-global level: the actor's actual received message order across all outstanding capabilities must remain consistent with the protocol obligations justified by the type system.

For `cast()` actors, this guarantee remains static and capability-directed: the type system constrains which casts may be sent, but the runtime mailbox is not required to defer or reorder casts to satisfy protocol phases.

### 10.5 Required Obligation Tracking Strategy
Protocol obligations are a required part of the type system and formal model described by this specification.

A conforming implementation must track protocol obligations with enough precision to justify the soundness claims in this document.

## 11. Formal Verification Extension

### 11.1 New Verification Layer
The formal verification document should gain a new layer for actor protocols, above the current value calculus and region-lifetime framework.

Suggested layering:

1. Layer 1 — value calculus
2. Layer 2 — regions and lifetimes
3. Layer 3 — actor protocol capabilities

### 11.2 New Type Forms
The formal system adds:

```text
T ::= ... | ActorRef(P) | Beh(P, M, S, R)
```

where:
- `P` is a protocol language
- `M` is actor mode (`call_only` or `cast_only`)
- `S` is state type
- `R` is reply type

### 11.3 Protocol Language Domain
Define a protocol denotation:

```text
⟦P⟧ ⊆ Label*
```

where `Label` is the finite set of message commands used by the program.

### 11.4 Derivative Operator
Add a derivative operator:

```text
derive(P, l)
```

with the property:

```text
⟦derive(P, l)⟧ = { w | l · w ∈ ⟦P⟧ }
```

### 11.5 Capability-Send Rule
Add a send rule of the form:

```text
Γ ⊢ e_msg : MsgType(label = l)
Γ(x) = ActorRef(P)
derive(P, l) ≠ ∅
────────────────────────────────
Γ ⊢ send(x, e_msg) : unit ⊣ Γ[x ↦ ActorRef(derive(P, l))]
```

### 11.6 Capability Split Rule
Add a split rule of the form:

```text
shuffle(P1, P2) ⊆ P
────────────────────────────────
Γ, x : ActorRef(P) ⊢ split x as x1, x2
      ⊣ Γ, x1 : ActorRef(P1), x2 : ActorRef(P2)
```

### 11.7 Behavior Soundness Rule
For any actor with behavior type `Beh(P, M, S, R)`, any message sequence admitted by outstanding capabilities to that actor must be accepted by the actor's future behavior evolution.

This is the central proof obligation of the extension.

---

## 12. Syntax Additions

### 12.1 Type Grammar Additions
Extend the type grammar with:

```ebnf
type ::= ... | "actor_ref" "with" protocol_expression "end"
```

### 12.2 Spawn Grammar Additions
```ebnf
spawn_expression ::= "spawn" "with" protocol_expression "end" "(" expression "," expression ["," expression] ")"
```

### 12.3 No Change to Existing Message Syntax
The actual runtime message syntax remains record-based and still uses `ActorMessage`.

---

## 13. Static Errors

### 13.1 New Error Category
Add **Protocol Capability Errors** with code range `E35xx`.

### 13.2 Required Errors

#### E3500 — ProtocolViolation
A message label is not permitted by the current protocol state of the actor reference.

Example:
```text
cannot send message commanded :close through actor_ref with :open then (:write repeat) then :close end before sending :open
```

#### E3501 — ProtocolExhausted
A send or call attempts to use `actor_ref with epsilon end`.

#### E3502 — IllegalCapabilityCopy
The program duplicates a non-copyable protocol actor reference.

#### E3503 — IncompatibleProtocolSpawn
A spawned behavior is not compatible with the declared protocol.

#### E3504 — IncompatibleProtocolMode
A protocol actor is declared or used inconsistently with `call()` vs `cast()` mode.

#### E3505 — ProtocolMatchIndeterminate
The compiler cannot determine a unique protocol label for the sent message.

---

## 14. Implementation Requirements

A conforming implementation must implement this specification as a complete feature set.

Required components include:
- parsing `actor_ref with P end`
- parsing `spawn with P end (...)`
- parsing and typing the full protocol language
- support for both atom-tag labels and `message({ ... })` protocol atoms
- protocol derivatives
- user-visible `interleave`
- flow-sensitive updates through `call()` and `cast()`
- copyability and capability splitting checks
- behavior/protocol compatibility checking
- mode consistency checking
- protocol obligation tracking in the formal and compiler-internal model

This specification does not define staged milestones, reduced compliance profiles, or phased subsets.

## 15. Design Constraints for Silica

### 15.1 No Generics
Because Silica avoids generics, protocol typing must remain explicit and concrete. This document therefore uses explicit `actor_ref with P end` syntax rather than generic actor API abstractions.

### 15.2 Explicit Types
No inference is required for user-visible protocols.

Mandatory protocol annotations are consistent with Silica's broader design preference for explicit programmer intent.

This specification assumes full explicit support for all protocol constructs at implementation time.

### 15.3 LLM-Friendly Syntax
The protocol surface syntax is intentionally verbose and keyword-based:
- `with`
- `then`
- `or`
- `repeat`
- `optional`
- `interleave`
- `end`

This is preferred over symbolic regex-like notation for readability and machine parsing.

### 15.4 Explicit Opt-Out
Opting out of ordering restrictions remains explicit through `:dynamic` as a surface protocol literal.

This preserves uniformity in the type system while still allowing programmers to choose unrestricted actor references when protocol checking is unnecessary. `:dynamic` is not replaced with a keyword or alternate syntax; the atom literal `:dynamic` is the explicit surface expression for unrestricted protocol behavior.

## 16. Required Feature Scope

A conforming Silica implementation must support all of the following:

1. **Explicit unrestricted actors**
   - `actor_ref with :dynamic end`
   - `spawn with :dynamic end (...)`

2. **One-shot capabilities**
   - `:grant_once`
   - `:shutdown`

3. **Linear request protocols**
   - `:begin then :commit`
   - `:open then :close`

4. **Repeatable message windows**
   - `:open then (:write repeat) then :close`

5. **Reply-port handoff protocols**
   - a client receives a restricted actor reference permitting exactly one follow-up message

6. **User-visible interleaving protocols**
   - protocol expressions using `interleave`

7. **Exact message-shape protocols**
   - protocol atoms of the form `message({ ... })`

8. **Mode-consistent protocol actors**
   - `call()` and `cast()` compatibility enforced together with protocol checking

## 17. Conformance

An implementation conforms to this specification if it:
- uses `actor_ref with P end` as the sole actor reference type form
- requires explicit protocol annotation at actor creation points
- correctly parses and type-checks `actor_ref with P end` and `spawn with P end (...)`
- treats `:dynamic` as the unrestricted built-in protocol (as a surface literal)
- treats `epsilon` as the exhausted protocol
- enforces atom-commanded message labels as the sole protocol labeling mechanism
- rejects protocol-invalid sends
- allows references to be freely copied, with each copy tracking independent protocol state
- enforces mode consistency with existing `call()`/`cast()` rules
- interprets protocol ordering for `call()` actors against actual received message order across all outstanding capabilities together
- interprets protocol ordering for `cast()` actors as a static capability discipline without runtime mailbox deferral or reordering
- ensures hot-swapped behaviors preserve the protocol declared at spawn time
- does not model or govern reply messages through the protocol system
- integrates protocol obligations into the formal verification framework

## 18. Design Decisions

The following design questions were resolved during specification development:

### 18.1 Protocol Labels
**Decision:** Protocol labels are always atom commands drawn from the `command: atom` field of actor messages. Exact record-shape matching is not used for protocol determination.

**Rationale:** Atom commands provide clear, unambiguous protocol labeling while maintaining consistency with Silica's explicit-type philosophy.

### 18.2 Reference Copyability
**Decision:** References of type `actor_ref with P end` are freely copyable. Each copy maintains its own independent protocol state.

**Rationale:** Copyability provides flexibility in reference management while allowing independent tracking of protocol consumption across multiple handles to the same actor.

### 18.3 Reply Protocols
**Decision:** Reply protocols are not modeled explicitly in the surface language. Protocols govern only outgoing messages; reply types are determined by the actor's declared reply type.

**Rationale:** Simplicity and orthogonality with existing call/cast typing. Protocol discipline applies to the sender's side only.

### 18.4 Behavior Hot-Swapping
**Decision:** Hot-swapped behaviors must preserve the protocol declared at spawn time. Behaviors may not refine, widen, or otherwise alter the protocol promised to outstanding capabilities.

**Rationale:** Protocol invariance ensures safety guarantees remain valid across behavior transitions.

### 18.5 Unrestricted Protocol Literal
**Decision:** `:dynamic` remains a surface protocol literal. No alternate keyword spelling is introduced.

**Rationale:** Consistency with Silica's explicit, literal-based design philosophy. Unrestricted protocol behavior is an explicit choice, not implicit or hidden.

## 19. References

1. Gordon, Colin S. *Actor Capabilities for Message Ordering*.

## 20. Proposed Companion Changes

After this document stabilizes, the following companion updates should be drafted:

1. An amendment to **silica-specification.md** referencing this extension in the actor and concurrency sections
2. An amendment to **silica-formal-verification-specification.md** adding Layer 3 protocol rules
3. A compiler-internal design note defining the protocol AST, derivative algorithm, and copy-checking rules

