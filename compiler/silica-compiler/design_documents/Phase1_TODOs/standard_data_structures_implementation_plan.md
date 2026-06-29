# Phase 1 Standard Data Structures — Dependency-Ordered Implementation Plan

**Date:** 2026-06-29
**Status:** Implementation sequencing authority
**Design authority:** [`data_structure_designs/README.md`](data_structure_designs/README.md) and every design linked from it

## 1. Purpose

This document defines the implementation order for the Phase 1 standard data structures. It does not redesign their APIs, algorithms, representations, or invariants. Those decisions belong to the detailed design suite.

The governing scheduling rule is:

> Implement and accept the deepest shared dependency first. Implement a consumer only after every dependency it uses has passed its acceptance gate. Implement terminal structures—structures with no downstream Phase 1 consumers—last.

This is a topological plan, not a list ordered by perceived API simplicity. A small public adapter can be late because it is a leaf, while a large internal representation can be early because several structures depend on it.

## 2. Scope and authority

### 2.1 In scope

- compiler and runtime support required by the detailed designs;
- canonical application-lifetime arenas for generated specializations;
- exact function-value ordering identity;
- recursive tuple allocation and references;
- trait and constructor-record support for all Phase 1 collection families;
- the WBT, skew binary random-access-list, and Brodal–Okasaki cores;
- all nine public data-structure traits;
- live WBT, CSR snapshot, and dense graph representations;
- generated module registration, build integration, new trials, validation, and cross-representation conformance.

### 2.2 Out of scope

- any old data-structure implementation or trial removed by the reset;
- B-trees, red-black trees, binary heaps, pairing heaps, adjacency-list graph replacements, or mutable alternatives;
- compatibility wrappers for removed modules;
- priority-queue arbitrary deletion or decrease-key;
- rose-tree compaction or child-slot reuse;
- graph vertex removal;
- algorithms not explicitly part of the detailed design suite.

### 2.3 Conflict rule

1. [`data_structure_to_algorithms.md`](data_structure_to_algorithms.md) controls algorithm-family choices.
2. [`data_structures_as_traits.md`](data_structures_as_traits.md) controls trait architecture and constructor-record resolution.
3. [`data_structure_designs/`](data_structure_designs/) controls exact APIs, representations, invariants, and failure behavior.
4. This file controls implementation order and acceptance gates only.

If this plan appears to specify behavior differently from a detailed design, the detailed design wins and this plan must be corrected.

## 3. Scheduling policy

### 3.1 Ready-work rule

An implementation unit enters the ready queue only when:

1. every predecessor in the dependency graph is implemented;
2. every predecessor has passed its local positive, negative, and invariant trials;
3. the unit's detailed design has no unresolved decision affecting its representation contract.

Among ready units, use this priority:

1. shared compiler/runtime substrate;
2. representation core with the greatest number of downstream consumers;
3. reusable generated backend;
4. nonterminal public trait implementation;
5. terminal public structure or representation.

Independent units at the same dependency depth may proceed in parallel. Parallel work must not bypass a gate.

### 3.2 What “accepted” means

A unit is accepted only when:

- its public and internal Silica types match the detailed design;
- it compiles through the normal compiler and standard-library build;
- its success, empty, duplicate, invalid-input, and incompatibility paths are tested;
- `validate` accepts generated valid values and rejects deliberately malformed test fixtures where fixtures can be constructed safely;
- persistence tests prove old roots remain observable after updates;
- allocation tests prove nodes use the correct canonical arena;
- no test relies on a removed implementation or removed trial.

### 3.3 Leaf-last rule

The following are terminal leaves in the Phase 1 dependency graph and must not be used to bootstrap their own prerequisites:

- `SearchTree`;
- `PriorityQueue`;
- `Tree`;
- CSR graph snapshots;
- dense matrix graphs.

Their trait declarations may be parsed earlier as compiler fixtures, but their complete generated modules and conformance trials belong in the terminal layers.

## 4. Dependency graph

```text
Language/runtime substrate
├── canonical arena registry and construction
├── exact function-value ordering identity
├── recursive tuples, ref?, and alloc_rec
├── traits, provided methods, and constructor records
├── collection type witnesses and generated-module registry
├── checked int64 arithmetic
└── runtime-sized immutable buffers
    │
    ├── WBT core
    │   ├── wbt_set ── OrderedSet ── SearchTree                     [leaf]
    │   ├── wbt_map ── OrderedMap
    │   └── live WBT graph core
    │       ├── DirectedGraph live module
    │       ├── UndirectedGraph live module
    │       └── WeightedGraph live modules
    │           └── CSR freeze modules                              [leaf]
    │
    ├── immutable List + skew binary random-access-list core
    │   ├── Tree                                                     [leaf]
    │   └── dense graph modules                                     [leaf]
    │
    └── immutable List + Brodal–Okasaki core
        ├── Heap
        └── PriorityQueue                                           [leaf]

WBT indexes + graph traits + random-access-list core
└── dense graph modules                                             [leaf]

WBT from_sorted + live graph modules + runtime-sized buffers
└── CSR freeze modules                                              [leaf]
```

The CSR/dense representation decision is closed:

```text
compiler-version-private structural layouts
├── public vertex IDs: int64; internal slots: distinct int64 domain
├── runtime-sized internal extents, absent from public graph type parameters
├── CSR: parallel neighbor and attributed/weighted edge-data buffers
├── dense: boolean unweighted cells; one tagged attributed/weighted cell sequence
└── distinct WBT, CSR, and dense concrete generated types
```

These decisions are implementation inputs, not an additional scheduling barrier.

## 5. Layer summary

| Layer | Implementation units | Hard dependencies | Unlocks |
|---|---|---|---|
| 0 | baseline, design freeze, trial harness | detailed designs | controlled implementation |
| 1 | compiler/runtime substrate | Layer 0 | every representation |
| 2 | WBT, skew RAL, Brodal–Okasaki cores | Layer 1 | all generated backends |
| 3 | `wbt_set`, `wbt_map`, `OrderedSet`, `OrderedMap`, Heap | relevant Layer 2 core | search adapter, graphs, priority queue |
| 4 | live WBT graph core and graph traits/modules | WBT set/map | weighted and snapshot graphs |
| 5 | `SearchTree`, `PriorityQueue`, `Tree` | their complete branches | terminal API completion |
| 6 | CSR and dense graph modules | graph/index/buffer dependencies | representation completion |
| 7 | full integration and hardening | all prior layers | Phase 1 completion |

Layers define dependency order. Numbered work packages within one layer are not serial unless a dependency is stated.

## 6. Layer 0 — Establish the implementation baseline

### 6.1 Confirm normative inputs

Record the current revisions of:

- both parent design documents;
- every file in `data_structure_designs/`;
- recursive tuple and runtime-sized-buffer language designs;
- compiler collection type-witness and trait behavior.

Do not copy behavior from deleted source or deleted trials. The reset is the baseline.

### 6.2 Create a fresh trial hierarchy

Create a new Phase 1 trial root organized by dependency rather than by public structure alone:

```text
trials/standard_data_structures_phase1/
├── compiler_substrate/
├── wbt_core/
├── skew_ral_core/
├── brodal_okasaki_core/
├── ordered_collections/
├── live_graphs/
├── terminal_structures/
├── snapshot_graphs/
├── error_enforcement/
└── cross_representation/
```

Each directory must have normal compile/run integration and must distinguish:

- expected compile success;
- expected compile failure;
- expected runtime success;
- expected deterministic runtime collection error.

### 6.3 Create a requirements-to-trials ledger

For every numbered design section, record at least one of:

- implementing source unit and trial;
- compile-time-only assertion;
- explicitly non-executable mathematical invariant;
- out-of-scope marker copied from the design.

The ledger prevents apparently complete modules from omitting failure or compatibility behavior.

### 6.4 Record the closed CSR/dense representation contract

The requirements-to-trials ledger must record:

- compiler-version-private inline layouts that generated modules may inspect but user source may not;
- public `int64` vertex IDs translated to a distinct internal `int64` dense-slot domain;
- runtime-sized internal extents that do not participate in public graph type identity;
- parallel CSR neighbor and edge-data buffers for attributed/weighted forms;
- one boolean dense cell sequence for unweighted forms;
- one `:none | (:some, EdgeDataType)` dense cell sequence for attributed/weighted forms;
- distinct concrete WBT, CSR, and dense generated types, including their attributed/weighted specializations.

### Layer 0 exit gate

- The new trial root builds even while empty or with smoke fixtures.
- Every detailed design section appears in the coverage ledger.
- No new file imports or references a removed implementation.
- The closed CSR/dense representation contract appears in the coverage ledger.

## 7. Layer 1 — Implement the compiler and runtime substrate

This layer is the deepest dependency. No representation core starts until its relevant substrate trials pass.

### 7.1 Canonical arena registry

Implement the common constructor allocation rule:

- one canonical application-lifetime arena per generated representation specialization and memory space;
- repeated construction of that specialization resolves the same arena identity;
- different representation specialization, item/key/value/edge-data specialization, or memory space resolves a different canonical arena where the design requires it;
- collection values carry the arena capability needed by recursive nodes;
- updates allocate replacement nodes in the collection's arena;
- cross-value operations compare arena identity when they may make one result reference both operands.

Compiler/runtime work includes:

- a stable specialization key;
- application-lifetime arena creation and lookup;
- constructor lowering that requests the canonical arena rather than allocating an arena per call;
- emitted arena identity comparison for meld and subtree-sharing checks;
- deterministic diagnostics for a missing or mismatched canonical arena.

Acceptance trials:

- two constructor calls for the same specialization share an arena;
- different spaces do not share an arena;
- different concrete specializations do not alias accidentally;
- WBT path copying, heap meld, and tree subtree insertion can later consume the same primitive contract;
- application shutdown, not collection reachability, defines arena lifetime.

### 7.2 Exact function-value ordering identity

Implement the ordering provenance required by all ordered structures:

- a top-level function has canonical symbol identity;
- a closure identity contains exact code identity and exact captured-environment instance identity;
- two separately created closures are incompatible even when captures and behavior compare equal;
- a comparator bundle includes every ordering-relevant function value named by the detailed design;
- min/max orientation is part of heap ordering identity;
- no programmer-provided identity override exists.

Provide compiler/SIR/emitter support to:

- materialize a non-forgeable identity token for an exact function value;
- compare identity tokens without invoking the function;
- retain captured environments for at least as long as any collection carrying their identity;
- include ordering identity in generated collection values without exposing a public customization field.

Acceptance trials:

- repeated references to one top-level comparator are compatible;
- one closure value copied into two constructor records is compatible;
- two separately evaluated closure expressions are incompatible;
- min and max orientations are incompatible;
- meld and subtree sharing reject incompatible identity before allocating a result.

### 7.3 Recursive tuples and references

Implement the recursive structural encoding used by every core:

- parser support for recursive tuple declarations and recursive positions;
- `ref?` or the exact optional recursive-reference syntax selected by the language design;
- `alloc_rec` in the canonical arena;
- type equality with an occurs check;
- recursive record field projection;
- SIR representation;
- emitter layout, alignment, and reference operations;
- `:none` as the empty recursive position.

Acceptance trials:

- a one-node recursive value allocates and can be read;
- a multi-node structure traverses through optional references;
- type mismatch at a recursive position fails at compile time;
- malformed unguarded recursion fails deterministically;
- recursive nodes can contain function values, `List` values, and other recursive references needed by the three cores.

### 7.4 Trait dispatch and provided methods

Complete the trait machinery required by the parent design:

- first-argument receiver dispatch;
- `required` and `provided` methods;
- provided bodies calling only trait methods and ordinary public functions;
- associated placeholders for item, key, value, node ID, edge data, weight, priority, and memory space;
- exact structural matching of an implementation to a trait;
- deterministic ambiguity and missing-method diagnostics;
- generated link-name mangling that separates concrete specialization, trait, method, and memory space.

Acceptance trials:

- one concrete WBT-set fixture implements two independent traits;
- a provided `contains`-style method calls the required fold hook;
- a generated module may override a provided method;
- wrong receiver type, unresolved placeholder, ambiguous implementation, or missing required method is rejected;
- two specializations do not collide at link time.

### 7.5 Constructor function-record resolution

Implement constructor selection by:

- collection type witness;
- exact inline function-record shape;
- memory-space witness;
- generated representation family.

Cover all public families:

- `OrderedSet`;
- `OrderedMap`;
- `SearchTree`;
- `DirectedGraph`;
- `UndirectedGraph`;
- `WeightedGraph`;
- `Heap`;
- `PriorityQueue`;
- `Tree`.

Acceptance trials:

- field order does not change structural record meaning if Silica records are order-independent;
- a missing, extra, or wrongly typed function field rejects the constructor;
- `EdgeDataType` remains separate from the internal `{to, data}` wrapper;
- unweighted graph convenience construction resolves `EdgeDataType = unit`;
- constructor lowering obtains the canonical arena.

### 7.6 Collection type witnesses and registry

Extend parsing, type checking, code generation, and module registration to every public and concrete generated family. The registry must be representation-based, not type-width-based.

The registry must distinguish:

- public behavior trait;
- concrete generated record type;
- construction/update module;
- representation orientation or directedness;
- all type and memory-space parameters.

Acceptance trials:

- every bracketed public type form parses and type-checks;
- every concrete generated representation has one stable internal identity;
- unrelated records with coincidentally similar fields do not acquire collection behavior accidentally;
- emitted module/link names remain distinct across min/max, directed/undirected, weighted/unweighted, and memory spaces.

### 7.7 Common result and error plumbing

Implement common conventions once:

- lookup status atoms are exactly `:not_found | :found`;
- the found payload remains a separate returned field or tuple member;
- no named option/result type is introduced for ordinary lookup;
- comparator calls accept only `:less | :equal | :greater`;
- a different comparator atom yields deterministic `:invalid_comparator_result`;
- incompatible orderings, arenas, indexes, capacities, and overflow have distinct deterministic errors as specified by the designs.

### 7.8 Checked arithmetic and runtime-sized buffers

Provide shared checked `int64` operations for:

- `size + 1`;
- subtree-size sums;
- edge and adjacency counts;
- `n * n`;
- prefix sums;
- buffer byte sizes and alignment;
- random-access-list weights;
- heap rank/size calculations.

Harden the existing runtime-sized immutable-buffer path needed later by CSR:

- runtime capacity survives type checking and lowering;
- exact element type and alignment are preserved;
- allocation rejects negative or overflowing sizes;
- freeze construction can fill a fresh buffer before publishing an immutable graph value;
- no completed snapshot performs in-place growth.

### Layer 1 exit gate

- Every substrate acceptance trial passes in each supported memory space.
- At least one minimal hand-written recursive fixture passes through parse, type check, SIR, emission, link, and run.
- Exact function identity and canonical arena identity are independently observable through test-only assertions.
- All nine constructor-record shapes resolve against stub concrete modules.
- No representation algorithm has been used to compensate for missing compiler support.

## 8. Layer 2 — Implement the three representation cores

The three tracks in this layer are independent after Layer 1 and may proceed in parallel. Their shared rule is to expose internal operations first and public trait wiring later.

## 8A. Corrected Adams-family WBT core

**Dependencies:** canonical arenas, exact comparator identity, recursive tuples, checked arithmetic.
**Downstream consumers:** set, map, search tree, every graph family, CSR/dense node indexes.

Implement in this order:

1. empty/reference representation and cached subtree size;
2. read-only helpers: `size`, `weight`, search, minimum, ordered fold;
3. the one smart-node constructor that checks arithmetic and recomputes size;
4. `(DELTA, GAMMA) = (3, 2)` balance predicates;
5. single and double rotations;
6. set insertion;
7. map insertion/replacement;
8. deletion, successor/minimum extraction, and rebalancing;
9. deterministic linear `from_sorted`;
10. optional internal join/split only if a Phase 1 consumer actually requires them;
11. full validation.

Required trials:

- empty, singleton, ascending, descending, and adversarial insertion;
- duplicate set insertion is a semantic no-op;
- comparator-equal map insertion replaces the value at one key position;
- deletion of absent, leaf, one-child, two-child, and root entries;
- persistence across every operation;
- cached-size, order, balance, and arena validation;
- invalid comparator result at every comparator call site;
- `from_sorted` accepts strict order and rejects disorder or duplicate equivalence classes;
- randomized operation traces checked against a simple test oracle.

The WBT core is accepted before `wbt_set`, `wbt_map`, or any graph module is started.

## 8B. Skew binary random-access-list core

**Dependencies:** canonical arenas, recursive tuples, immutable Silica `List`, checked arithmetic.
**Downstream consumers:** rose-tree child slots and dense graph storage.

Implement in this order:

1. leaf/node tree encoding;
2. digit record and immutable `List` forest spine;
3. `prepend`;
4. `head` and `tail`;
5. logarithmic lookup;
6. persistent logarithmic update;
7. deterministic bulk construction;
8. ordered fold and range traversal;
9. consumer orientation adapter, including reverse physical orientation where designed;
10. full validation.

Required trials:

- all sequence lengths across several skew-weight boundaries;
- repeated prepend/head/tail round trips;
- lookup/update at first, middle, last, and invalid indexes;
- persistence of previous sequences;
- digit-order, tree-weight, cached-size, and arena validation;
- overflow rejection during weight construction;
- bulk-build sequence equivalence to the abstract list.

Do not add lazy thunks. Both the forest spine and consumer-facing use are strict.

## 8C. Brodal–Okasaki bootstrapped queue core

**Dependencies:** canonical arenas, exact ordering identity, recursive tuples, immutable Silica `List`, checked arithmetic.
**Downstream consumers:** `Heap` and `PriorityQueue`.

Implement in this order:

1. strict skew-binomial tree;
2. immutable `List` child/deferred spines;
3. primitive skew queue insertion and linking;
4. primitive meld and rank normalization;
5. primitive delete-min normalization;
6. bootstrapped queue representation;
7. empty, length, and peek;
8. bootstrapped insert;
9. bootstrapped meld;
10. delete-min/pop;
11. min/max orientation adapter;
12. full validation.

Required trials:

- empty and singleton behavior;
- ascending, descending, duplicate, and adversarial priority streams;
- all rank-collision patterns reachable in bounded exhaustive tests;
- repeated meld trees, not just pairwise meld;
- pop order against a sorted test oracle;
- persistence of both meld operands and all older roots;
- rejection before allocation for incompatible ordering or arena identity;
- invalid comparator atom propagation;
- strictness: no hidden deferred thunk representation;
- rank, heap-order, cached-minimum, size, list-spine, and arena validation.

### Layer 2 exit gate

- Each core passes its complete invariant and randomized trace suite.
- Each core has a stable internal generated-type shape matching its detailed design.
- No public leaf structure has been implemented.
- WBT, skew RAL, and Brodal–Okasaki values can be compiled into the standard library without test-only runtime support.

## 9. Layer 3 — Build reusable backends and nonterminal public foundations

## 9A. `wbt_set` and `OrderedSet`

**Dependencies:** accepted WBT core and trait substrate.
**Downstream consumers:** `SearchTree`, live graph outer vertex sets/maps, graph neighbor sets, CSR/dense indexing.

Implement:

- the exact constructor function record;
- canonical-arena construction;
- generated `wbt_set` surface;
- required `OrderedSet` methods;
- optimized overrides for provided methods where the design permits;
- fold order, `from_sorted`, error behavior, and validation export.

Acceptance includes every operation and complexity-sensitive cached-size behavior in `ordered_set.md`.

## 9B. `wbt_map` and `OrderedMap`

**Dependencies:** accepted WBT core and trait substrate.
**Downstream consumers:** live weighted graphs, node-to-slot indexes, CSR/dense indexing.

Implement:

- distinct key and value parameters;
- key comparator as placement identity;
- value comparator only where the trait design calls for value search/equality;
- generated `wbt_map` surface;
- exact `:not_found | :found` lookup shape;
- replacement semantics for comparator-equal keys;
- deterministic `from_sorted`;
- validation export.

Acceptance includes a test proving value comparison is not called during key descent, insertion, deletion, or balancing.

## 9C. `Heap`

**Dependencies:** accepted Brodal–Okasaki core and trait substrate.
**Downstream consumer:** `PriorityQueue` shares this core and ordering machinery.

Implement:

- `brodal_okasaki_min` and `brodal_okasaki_max`;
- constructor resolution and orientation identity;
- required and provided `Heap` methods;
- `empty`, `push`, `peek`, `pop`, `meld`, `from_list`, and `validate`;
- exact incompatibility and empty behavior.

Acceptance must run identical abstract traces against min and max orientations and verify opposite pop order.

### Layer 3 exit gate

- `OrderedSet`, `OrderedMap`, and `Heap` pass their complete detailed-design suites.
- Their generated records and methods link without specialization collisions.
- WBT set/map backends are usable internally without dispatching through public traits where representation code requires direct operations.
- Search, priority-queue, tree, and graph leaves remain unimplemented.

## 10. Layer 4 — Implement live graph foundations

Live graphs are nonterminal because CSR freeze depends on them and because the weighted graph layers build on the graph storage model.

## 10.1 Define the generic live WBT graph core

**Dependencies:** accepted `wbt_set`, `wbt_map`, canonical arenas, graph constructor records, graph trait substrate.

Implement one parameterized storage core with:

- outer node-ID WBT;
- inner target-keyed WBT set/map;
- separate `EdgeDataType`;
- internal `{to: int64, data: EdgeDataType}` wrappers;
- directedness-specific update helpers;
- cached logical vertex, edge, and adjacency counts;
- deterministic vertex and neighbor folds;
- ordering bundle and arena identity;
- validation shared by all live graph variants.

The core must not:

- expose the internal edge wrapper as the public edge-data type;
- use `compare_edge_data` to place neighbors;
- silently add absent endpoint vertices unless the detailed design says so;
- implement `remove_vertex`;
- inspect CSR or dense records.

## 10.2 Directed live graph

Implement `graph_wbt_directed` and `DirectedGraph` conformance:

- vertex insertion and membership;
- directed edge insertion, replacement/no-op behavior, and removal;
- out-degree, neighbor traversal, edge fold, and connected query;
- exact vertex/edge counts;
- lookup and missing-endpoint behavior.

## 10.3 Undirected live graph

Implement `graph_wbt_undirected` and `UndirectedGraph` conformance:

- two directional wrappers for each non-loop logical edge;
- one wrapper for a self-loop;
- atomic persistent update/removal of both directions;
- logical edge count distinct from adjacency-entry count;
- symmetry validation;
- `EdgeDataType = unit` convenience path for unweighted construction.

## 10.4 Weighted live graphs

**Dependencies:** accepted directed and undirected storage behavior plus `WeightedGraph` trait substrate.

Implement weighted directed and weighted undirected modules:

- edge data remains separate from the internal wrapper;
- weight extraction uses the designed function record;
- replacement updates edge data and weight atomically;
- undirected reverse wrappers carry comparator-equal data/weight;
- weight validity behavior matches the detailed design;
- a weighted value implements the applicable direction trait and the independent `WeightedGraph` trait.

## 10.5 Live-graph acceptance matrix

Run every graph operation against:

- empty, isolated-vertex, single-edge, self-loop, cycle, disconnected, and duplicate-edge cases;
- directed and undirected forms;
- unweighted and weighted forms;
- top-level comparator and captured-closure comparator identities;
- old roots after every update;
- validation of outer WBT, every inner WBT, wrappers, counts, symmetry, and arena.

Add randomized traces checked against a simple mathematical graph oracle. The oracle is test-only and must not become a standard-library implementation.

### Layer 4 exit gate

- All live graph modules pass the matrix.
- Query algorithms consume graph traits rather than generated record fields.
- Weighted values satisfy both independent trait contracts.
- The deterministic node/neighbor fold order needed by CSR freeze is stable.
- CSR and dense remain dependency-blocked until their graph/index/buffer prerequisites are accepted.

## 11. Layer 5 — Implement terminal structures on completed branches

These structures have no downstream Phase 1 consumer. Implement them only after their complete dependencies are accepted.

## 11.1 `SearchTree`

**Dependencies:** accepted `wbt_set`, `OrderedSet`, multi-trait conformance.

Implement `SearchTree` as the designed behavioral view:

- the concrete value is the same generated WBT-set value;
- one concrete type implements both `OrderedSet` and `SearchTree`;
- construction and updates remain `wbt_set` operations;
- search behavior, fold order, comparator identity, validation, and complexity are unchanged.

Do not create a second representation, copy the tree, or add an independent arena.

## 11.2 `PriorityQueue`

**Dependencies:** accepted Brodal–Okasaki core, Heap ordering machinery, trait substrate.

Implement:

- priority/value entry payloads that move together;
- the exact constructor bundle for priority and tie/value comparison;
- push, peek, pop, meld, bulk construction, and validation;
- deterministic duplicate/tie behavior;
- ordering and arena compatibility checks.

Do not implement arbitrary-entry deletion, handles, or decrease-key.

## 11.3 `Tree`

**Dependencies:** accepted skew RAL core, exact item comparator identity, canonical arenas, trait substrate.

Implement:

- `tree_rose` node and root construction;
- reverse-oriented skew-RAL child slots;
- stable path lookup;
- add child and add subtree;
- remove child by leaving a tombstone/vacant slot;
- replace and traversal;
- cached live-node counts and validation;
- compatibility checks before subtree sharing.

Do not reuse, compact, or renumber vacant child slots.

### Layer 5 exit gate

- All three terminal structures pass their detailed-design trials.
- SearchTree shares rather than duplicates `OrderedSet` representation.
- PriorityQueue exposes no decrease-key/deletion surface.
- Tree preserves every path index across removal and all later updates.

## 12. Layer 6 — Implement terminal CSR and dense graph representations

### Layer 6 entry gate

Do not enter this layer until:

- live WBT graph modules are accepted;
- WBT `from_sorted` and map indexing are accepted;
- skew RAL is accepted;
- runtime-sized buffer and checked-arithmetic trials pass;
- all graph query traits are accepted independently of representation.

## 12.1 Shared deterministic node-slot assignment

Implement one internal slot-assignment procedure used by CSR and dense construction:

1. fold live node IDs in comparator order;
2. assign dense slots `0..n-1`;
3. construct a node-ID-to-slot WBT map;
4. preserve a slot-to-node sequence;
5. reject overflow before allocating final storage;
6. retain ordering identity and canonical arena provenance required by the concrete design.

Acceptance proves identical live graph values receive identical slot assignments.

Public IDs and assigned slots are both `int64`, but test fixtures must include negative and sparse public IDs to prove that implementations never use an ID directly as a slot.

## 12.2 CSR snapshots

**Dependencies:** live graph modules, shared slot assignment, WBT map, runtime-sized immutable buffers.

Implement freeze as a staged construction:

1. validate or trust only an accepted live graph input;
2. assign slots;
3. count adjacency entries with checked arithmetic;
4. allocate exact offsets and adjacency buffers;
5. compute prefix sums;
6. fill neighbors in deterministic per-source comparator order;
7. fill edge data/weights in aligned corresponding positions;
8. publish the immutable snapshot only after construction succeeds;
9. expose graph query-trait conformance;
10. expose no mutation operation.

Required trials:

- empty, isolated, sparse, self-loop, directed, undirected, and weighted snapshots;
- offset monotonicity and final-offset equality;
- neighbor public-ID membership and deterministic ordering;
- logical edge/count equivalence with the source;
- query equivalence between the live graph and snapshot;
- source persistence and snapshot immutability;
- exact-size allocation and overflow rejection;
- malformed-buffer validation through safe test fixtures.

## 12.3 Dense matrix graphs

**Dependencies:** shared slot assignment, WBT node index, skew RAL, graph traits, checked `n * n`.

Implement:

- fixed vertex universe;
- row-major checked cell indexing;
- one skew-RAL persistent cell sequence;
- boolean cells for unweighted graphs;
- `:none | (:some, EdgeDataType)` cells for attributed/weighted graphs;
- directed and undirected edge updates;
- atomic symmetric update for non-loop undirected edges;
- weighted cell data where applicable;
- deterministic neighbor traversal;
- graph query-trait conformance and validation.

Required trials:

- `0x0`, `1x1`, and several rectangular index positions within square `n x n` storage;
- first/last cell and overflow boundaries;
- directed, undirected, self-loop, weighted, and missing-vertex behavior;
- old matrices after updates;
- symmetry and count invariants;
- live WBT versus dense query equivalence for the same fixed vertex universe.

### Layer 6 exit gate

- CSR and dense concrete types match the compiler-version-private representation contract.
- WBT, CSR, and dense values remain distinct concrete types without a runtime representation tag.
- Every representation returns identical abstract graph answers for shared fixtures.
- CSR exposes no mutation path.
- Dense vertex universes cannot grow through edge updates.
- No graph algorithm depends on WBT, CSR, or dense fields.

## 13. Layer 7 — Integrate and harden the complete suite

## 13.1 Standard-library build integration

Add the new source hierarchy to the standard-library build:

- trait modules;
- shared internal cores;
- generated representation modules;
- registry/configuration entries;
- emitted library artifacts;
- dependency declarations in build files.

Build order must mirror this plan:

1. common substrate;
2. representation cores;
3. reusable backends;
4. live graphs;
5. terminal structures;
6. CSR/dense;
7. cross-representation trials.

Do not restore deleted legacy source merely to satisfy an old build entry. Remove or replace stale entries with the new design-authoritative modules.

## 13.2 Full specialization matrix

Compile and run representative specializations across:

- primitive and structural keys/items;
- top-level and closure comparators;
- multiple memory spaces supported in Phase 1;
- min and max heaps;
- directed and undirected graphs;
- unweighted and separate edge-data types;
- weighted edge data;
- empty and large values.

The matrix should emphasize distinct layout shapes, not every combinatorial repetition.

## 13.3 Cross-representation graph conformance

For each applicable graph fixture:

1. construct a live WBT graph;
2. freeze it to CSR;
3. construct the equivalent dense graph;
4. run only public trait queries;
5. compare vertex membership, neighbors, connected, degrees, edge fold, counts, edge data, and weights;
6. compare deterministic traversal order where the designs promise it.

Any disagreement is a failed representation, not permission to weaken the trait.

## 13.4 Persistence and allocation stress

For every persistent update family:

- retain a sample of old roots;
- perform long branching update histories;
- re-query every retained root;
- validate structural sharing where observable through test-only instrumentation;
- verify allocations occur only in the canonical arena;
- verify semantic no-ops return the prior root where the detailed design promises that optimization.

## 13.5 Negative and diagnostic suite

Cover:

- invalid comparator result;
- comparator identity mismatch;
- heap orientation mismatch;
- canonical arena mismatch;
- wrong constructor record;
- unresolved trait placeholder;
- missing trait method;
- generated link-name collision attempts;
- invalid index/path;
- absent lookup and empty pop/peek;
- duplicate sorted input;
- integer overflow;
- malformed CSR offsets;
- dense-capacity overflow;
- graph endpoint and representation misuse.

Diagnostics must be deterministic and must not depend on allocator addresses.

## 13.6 Complexity guardrails

Use operation counters or bounded stress thresholds rather than wall-clock microbenchmarks to detect:

- accidental linear WBT search/update;
- accidental whole-tree copying;
- non-logarithmic skew-RAL lookup/update;
- accidental sorting during WBT `from_sorted`;
- repeated-growth CSR construction instead of exact allocation;
- full-matrix traversal for a query whose design promises a narrower bound;
- heap operations that flatten and rebuild the entire queue.

These guardrails verify algorithm shape; they are not a performance-tuning project.

## 13.7 Documentation synchronization

Before declaring completion:

- link this plan from the Phase 1 design index;
- ensure every generated module name matches the detailed designs;
- ensure examples compile against the final trait and constructor syntax;
- update design status only after its acceptance gate passes;
- leave exclusions explicit rather than creating placeholder APIs.

### Layer 7 exit gate

Phase 1 is complete only when:

- the normal compiler and standard-library build succeed from a clean checkout;
- every new trial passes;
- all nine public traits have accepted implementations;
- WBT live, CSR, and dense graph answers agree through public traits;
- all invariants and compatibility failures are covered;
- no source, build entry, documentation link, or trial depends on the removed implementation;
- the requirements-to-trials ledger has no unexplained gaps.

## 14. Concrete execution queue

The following is the default work queue. A later item may begin early only if it is at the same dependency depth and all of its own predecessors are accepted.

1. Baseline and fresh trial harness.
2. Canonical arena registry and construction lowering.
3. Exact function-value identity.
4. Recursive tuples/references/allocation.
5. Trait dispatch, provided methods, associated placeholders, and mangling.
6. Constructor-record resolution and all collection type witnesses.
7. Common status/error conventions and checked arithmetic.
8. Runtime-sized buffer hardening.
9. WBT core.
10. Skew binary random-access-list core.
11. Brodal–Okasaki core.
12. `wbt_set` and `OrderedSet`.
13. `wbt_map` and `OrderedMap`.
14. `Heap`.
15. Generic live WBT graph core.
16. Directed live graph.
17. Undirected live graph.
18. Weighted live graph variants.
19. `SearchTree`.
20. `PriorityQueue`.
21. `Tree`.
22. Shared graph node-slot assignment.
23. CSR snapshot variants.
24. Dense graph variants.
25. Full integration, negative matrix, persistence stress, and cross-representation conformance.

Items 2–8 are one hard substrate gate. Items 9–11 can run in parallel after that gate. Items 12–14 can run in parallel after their respective cores. Items 19–21 are deliberately delayed leaf work. Items 23–24 are the final representation leaves.

## 15. Definition of done by structure

| Public structure | Required accepted dependencies | Completion artifact |
|---|---|---|
| `OrderedSet` | substrate, WBT core | trait + `wbt_set` + full design trials |
| `OrderedMap` | substrate, WBT core | trait + `wbt_map` + full design trials |
| `Heap` | substrate, Brodal–Okasaki core | trait + min/max modules + full design trials |
| `DirectedGraph` | substrate, WBT set/map, live graph core | trait + live module; CSR/dense conformance after Layer 6 |
| `UndirectedGraph` | substrate, WBT set/map, live graph core | trait + symmetric live module; CSR/dense conformance after Layer 6 |
| `WeightedGraph` | directed/undirected live foundations, separate edge-data model | trait + weighted live variants; CSR/dense conformance after Layer 6 |
| `SearchTree` | accepted `OrderedSet` representation and multi-trait support | trait adapter over identical WBT-set value |
| `PriorityQueue` | accepted Brodal–Okasaki core and ordering bundle | trait + priority/value module, without decrease-key |
| `Tree` | accepted skew RAL and stable-slot semantics | trait + `tree_rose`, without compaction |

For graph traits, public-trait completion and representation-family completion are tracked separately. A live implementation can be accepted before CSR/dense, but the graph representation family is not complete until Layer 6 passes.

## 16. Stop conditions

Stop downstream implementation and return to the failed dependency when:

- a detailed design and executable type shape disagree;
- canonical arena identity cannot be represented without changing a public type;
- exact closure identity is not stable through lowering/emission;
- recursive type support requires an unplanned ownership or lifetime rule;
- a representation cannot validate its stated invariant;
- a consumer needs an operation excluded from its dependency's design;
- a proposed CSR/dense layout violates the closed compiler-version-private representation contract;
- a proposed convenience API would add behavior excluded by the detailed designs.

The remedy is to correct or explicitly revise the authoritative design, then resume from the affected gate. It is not to hide the discrepancy inside a generated module.
