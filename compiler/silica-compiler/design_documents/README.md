<h1 align="left">
  <img src="../../../silica_icon_tiny.png" alt="" width="40" height="40" style="vertical-align: middle; margin-right: 0.35em;" />
  Silica compiler — design documents
</h1>

These files are **working documents**: specifications, plans, and design notes for the Silica language and the self-hosted compiler. They are **revised as the implementation evolves**; where they disagree with each other or with the code, treat the situation as “docs catching up” unless a document explicitly marks itself as normative for a given area.

Use the entries below for **quick navigation**. Each item links to a file in this directory, followed by a short summary of what it contains.

---

## [silica-specification.md](./silica-specification.md)

**Authoritative Silica language specification** (large document): syntax, types, effects, regions, actors, diagnostics, compiler interface, and normative semantics for the language as currently defined.

---

## [silica-specification-additional.md](./silica-specification-additional.md)

**Extra compile-time rules** that reject anti-patterns (dead bindings, duplicate computation, loop invariants, redundant algebra, etc.) and records the **actor execution contract** (runtime loop vs. user behavior) for tooling.

---

## [execution-environments-hosted-vs-bare-metal.md](./execution-environments-hosted-vs-bare-metal.md)

**Reader-oriented overview** of how **OS-hosted** processes vs **OS-free** / bare-metal targets affect **memory `Space` guarantees** and **actor core placement**, with pointers to normative spec sections and related plans.

---

## [actor_growable_stack_design.md](./actor_growable_stack_design.md)

Actor memory architecture: **growable per-actor stacks**, lazy page migration, isolation vs. the older per-actor heap sketch, NUMA-oriented behavior, and cleanup on actor termination.

---

## [actor_implementation_plan.md](./actor_implementation_plan.md)

**Actor implementation roadmap** aligned with the language spec: closing gaps (e.g. `ActorMessage`, behaviors, runtime loop), phased status, and pointers to related specs and the growable-stack design.

---

## [supervisors_implementation_development_plan.md](./supervisors_implementation_development_plan.md)

**Supervision roadmap** aligned with [silica-specification.md](./silica-specification.md) §15.4: `Supervisor` trait, `spawn_linked`, supervision ingress, exit payloads, restart strategies, `FailureReporter`, and `link` / `monitor` interactions—phased after the core actor runtime.

---

## [actor_spawn_core_affinity_os_semantics.md](./actor_spawn_core_affinity_os_semantics.md)

**OS semantics** for **`spawn`** with a **single core id** (`uint64`): how macOS, Linux, FreeBSD, Solaris/illumos, and Windows interpret affinity vs. hints, carrier threads vs. actors, and pointers to topology and emitter plans.

---

## [atom_actor_registry_direct_index_design.md](./atom_actor_registry_direct_index_design.md)

**O(1) registry** mapping atom intern ids to **`actor_ref`**: contiguous **`0…N−1`** ids, contiguous slot buffer keyed by id, region-backed growth; contrast **`List`** scan, trees, no surface **`Map`**.

---

## [beam_like_crash_containment_design_notes.md](./beam_like_crash_containment_design_notes.md)

**BEAM-like lightweight process** semantics on a **native** runtime: fault isolation between processes, MTE/hardware hooks, when the runtime may recover vs. abort, and the relationship to memory safety in user code.

---

## [bootstrap-analysis-state.md](./bootstrap-analysis-state.md)

Structured **analysis of the Rust bootstrap compiler** (`silica-bootstrap-compiler`): pipeline phases, file roles, and architectural patterns extracted for comparison and migration planning—not a user-facing tutorial.

---

## [brokered_ipc_isolation_architecture.md](./brokered_ipc_isolation_architecture.md)

**Process isolation** for using unsafe native libraries: safe application, **broker**, and untrusted worker; separate IPC channels, validation, no in-process FFI to the worker, policy and recovery.

---

## [crypto-proposal-introduction.md](./crypto-proposal-introduction.md)

High-level introduction to **language-level cryptographic guardrails**: secret vs. public labels, constant-time comparisons, control-flow and indexing rules, protected buffers, and `proc[secret]`-style constraints.

---

## [list_implementation_design.md](./list_implementation_design.md)

How **immutable, Erlang-style lists** are represented and lowered in the **Phase 2 self-hosted compiler**: regions, bundles, early desugaring, and integration with memory spaces—implementation detail that refines the language spec where noted.

---

## [list_implementation_development_plan.md](./list_implementation_development_plan.md)

**Phased development plan** that tracks [list_implementation_design.md](./list_implementation_design.md): prerequisites, trials under `trials/list_addition/`, and incremental deliverables for the list pipeline.

---

## [list_chunk_vector_alignment_todo.md](./list_chunk_vector_alignment_todo.md)

**Open work** for making list **chunk data** sizing and codegen **follow target SIMD width** (**§4.5** bridge from design **§2/§4.1** to the current fixed **16-byte** minimum in **`prims_list.silica`**).

---

## [graph_representation_design.md](./graph_representation_design.md)

Graph representation designs for code generation without custom types: named design families for adjacency-list, compressed sparse row, dense matrix, and dense bitset graphs, with inline Silica shapes, construction/traversal operations, invariants, and when to use each family.

---

## [balanced_tree_and_heap_design.md](./balanced_tree_and_heap_design.md)

Balanced tree and heap representations for code generation without custom types: `NodeIDBTree` vs. `CsrBTree` guidance, inline B-tree and heap shapes, construction/search/update algorithms, invariants, naming rules, and generator requirements.

---

## [btree_set_design.md](./btree_set_design.md)

Generated B-tree set designs using the exact families `NodeIDBTreeSet` and `CsrBTreeSet`: when to use each, inline set shapes, construction/finalization paths, membership/insert/delete/range operations, invariants, naming rules, and generator requirements.

---

## [parser_design.jsonld](./parser_design.jsonld)

**Machine-readable** companion to [parser_design.md](./parser_design.md): structured design data (JSON-LD) for tooling and agents that consume the parser architecture graph.

---

## [parser_design.md](./parser_design.md)

Silica **parser architecture** based on **constraint propagation** (not classic LL/LR/PEG): roles on tokens, local constraints, one file per capability, and how the design stays modular as features grow.

---

## [recursion_implementation.jsonld](./recursion_implementation.jsonld)

**Machine-readable** companion to [recursion_implementation.md](./recursion_implementation.md).

---

## [recursion_implementation.md](./recursion_implementation.md)

Implementation strategy for **general recursion**: explicit frame stacks, tail and preemption (**fuel** / reductions), defunctionalized continuations, and tradeoffs vs. naive CPS—oriented toward schedulers and bounded stacks.

---

## [recursive_tuple_specification.md](./recursive_tuple_specification.md)

**Recursive tuple types**: `rec`, `:none`, region-backed recursive slots, typing with occurs-check, construction/decomposition, and ties to formal verification (recursive products).

---

## [region_memory_safety_todo.md](./region_memory_safety_todo.md)

**Implementation gaps** for the region memory model vs. the full specification: lifetime analysis phases, what is already implemented in the compiler, and what remains for sound region safety.

---

## [silica-compiler&language-specification.jsonld](./silica-compiler%26language-specification.jsonld)

**Structured (JSON-LD) language-specification graph** companion to the prose spec—intended for tools and machine-assisted authoring; not a substitute for reading [silica-specification.md](./silica-specification.md) for human semantics.

---

## [silica-compiler-code-organization.jsonld](./silica-compiler-code-organization.jsonld)

**Machine-readable** companion to [silica-compiler-code-organization.md](./silica-compiler-code-organization.md).

---

## [silica-compiler-code-organization.md](./silica-compiler-code-organization.md)

**Directory and file layout** of the Phase 2 Silica-in-Silica compiler: lexer, parser, type checker, SIR, emitter, naming conventions, and how to add new features without churn.

---

## [silica-formal-verification-specification.md](./silica-formal-verification-specification.md)

**Formal verification** framing: Curry–Howard view, value calculus, recursive product rules, region lifetime judgments—extends toward proof terms and checkable reasoning over Silica programs.

---

## [silica_sequence_blocks_tutorial_updated.md](./silica_sequence_blocks_tutorial_updated.md)

Tutorial for **`sequence` blocks** (`sequence` / `produces` / `pure` / monadic steps): where they appear vs. function bodies, effect declarations, and binding patterns—updated from older `do`-style tutorials.

---

## [utf8_support.md](./utf8_support.md)

Plan for **UTF-8 in char literals** and helpers such as `substring_until_char`: lexer/parser/SIR changes, constraints (e.g. no new Rust runtime calls), and correct handling of multi-byte characters in pure Silica.
