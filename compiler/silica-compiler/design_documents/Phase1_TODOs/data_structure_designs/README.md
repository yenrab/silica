# Silica Standard Data-Structure Design Suite

**Date:** 2026-06-29
**Status:** Normative detailed design; no implementation sequencing
**Parent authorities:** [`../data_structures_as_traits.md`](../data_structures_as_traits.md) and [`../data_structure_to_algorithms.md`](../data_structure_to_algorithms.md)

## Purpose

This directory turns the two parent decisions into construction-grade designs for Silica's purely functional standard data structures. Each public data structure has its own file. Shared representation files define the algorithms and invariants once so that public designs cannot silently choose different balancing rules, index conventions, or persistence semantics.

These documents describe:

- abstract values and observable behavior;
- Silica type parameters, constructor function records, trait surfaces, and generated module surfaces;
- logical and physical representation shapes;
- operation semantics, including duplicate, empty, invalid-input, and comparator behavior;
- representation invariants and validation predicates;
- persistence, region ownership, memory effects, and structural sharing;
- asymptotic time and allocation bounds.

They intentionally do **not** prescribe implementation phases, source-file ordering, trial ordering, staffing, or delivery milestones.

Every file inherits [`common_contract.md`](common_contract.md), which defines constructor resolution, comparator law, ordering provenance, region/arena ownership, integer safety, result conventions, and validation result shape.

## Authority and conflict rule

1. `data_structure_to_algorithms.md` controls algorithm-family choices.
2. `data_structures_as_traits.md` controls the trait-oriented architecture and constructor-record model.
3. This directory controls detailed behavior and representation invariants.
4. If an example in an older design document conflicts with these three levels, the older example is non-normative.

All Silica snippets are normative as interface shape but may use `TypeName` placeholders and expository recursive aliases for readability. Silica source remains structurally typed and writes composite types inline.

## Public structure designs

| Structure | Design | Primary representation |
|---|---|---|
| `OrderedSet` | [`ordered_set.md`](ordered_set.md) | Adams-family WBT |
| `OrderedMap` | [`ordered_map.md`](ordered_map.md) | Adams-family WBT map |
| `SearchTree` | [`search_tree.md`](search_tree.md) | same value as `OrderedSet` |
| `DirectedGraph` | [`directed_graph.md`](directed_graph.md) | live WBT; CSR/dense query backends |
| `UndirectedGraph` | [`undirected_graph.md`](undirected_graph.md) | symmetric live WBT; CSR/dense query backends |
| `WeightedGraph` | [`weighted_graph.md`](weighted_graph.md) | WBT target-to-payload maps |
| `Heap` | [`heap.md`](heap.md) | Brodal–Okasaki bootstrapped skew-binomial queue |
| `PriorityQueue` | [`priority_queue.md`](priority_queue.md) | same core over priority/value entries |
| `Tree` | [`tree.md`](tree.md) | rose tree with skew-binary child sequences |

## Representation designs

| Representation | Design | Consumers |
|---|---|---|
| Corrected Adams-family WBT | [`weight_balanced_tree.md`](weight_balanced_tree.md) | set, map, search tree, live graph, dense/CSR indexes |
| Skew binary random-access list | [`skew_binary_random_access_list.md`](skew_binary_random_access_list.md) | dense graph, rose-tree children |
| Brodal–Okasaki queue | [`brodal_okasaki_queue.md`](brodal_okasaki_queue.md) | heap, priority queue |
| Live WBT graph | [`live_wbt_graph.md`](live_wbt_graph.md) | directed, undirected, weighted graphs |
| CSR graph snapshot | [`csr_graph_snapshot.md`](csr_graph_snapshot.md) | frozen directed/undirected/weighted graphs |
| Dense matrix graph | [`dense_matrix_graph.md`](dense_matrix_graph.md) | fixed-vertex directed/undirected/weighted graphs |

## Suite-wide decisions

- All updates are persistent. A successful update returns a new root value; the old root and every node not on a changed path remain usable.
- Recursive nodes are region-allocated and referenced through optional recursive references. `:none` is the empty recursive position.
- Every constructor uses the canonical application-lifetime arena for its generated representation specialization and memory space.
- The collection value carries the canonical arena capability needed by its nodes.
- Comparators define identity as well as order: `:equal` means the two values occupy one ordered-key position.
- Comparator results other than `:less`, `:equal`, or `:greater` are invalid behavior and produce a deterministic collection error; they are never treated as an ordering branch.
- Counts and internal indexes are non-negative `int64`. Arithmetic that would overflow is rejected before allocation.
- Lookup status is expressed only by the atoms `:not_found | :found`; no named option type is introduced.
- Ordering compatibility uses exact function-value identity; separately created closures are incompatible even when behaviorally equivalent.
- A generated module name is representation-based, never type-width-based.
- Traits expose behavior. Representation modules expose construction and persistent updates.
- Graph algorithms use public graph traits, not WBT, CSR, or dense record fields.
- Phase 1 graph vertex IDs are `int64` in every public and concrete representation.
- CSR/dense slots and indexes are also `int64`, but a public vertex ID and an assigned dense slot are distinct domains and are never implicitly interchangeable.
- CSR/dense structural layouts are private to one compiler/standard-library version. They are not a stable source, FFI, serialization, or cross-version ABI.
- CSR buffer extents and dense cell counts are runtime-sized internal values, not public graph type parameters.
- CSR attributed/weighted forms use parallel neighbor and edge-data buffers. Dense unweighted forms use one boolean cell sequence; dense attributed/weighted forms use one tagged optional-data cell sequence.
- WBT, CSR, and dense graph families are distinct concrete generated types with static trait conformance; no runtime representation tag unifies them.
- CSR is a snapshot, not a mutation target.
- Dense graphs have a fixed vertex universe.
- Rose-tree child removal vacates a stable child slot; it does not renumber siblings.
- Priority queues do not expose arbitrary-entry deletion or decrease-key.
- Rose trees do not compact or renumber child slots.

## Shared terminology

- **size**: number of logical entries in a collection or subtree.
- **weight**: `size + 1` in the WBT balance equations.
- **logical edge**: one user-visible edge. An undirected non-loop logical edge has two adjacency entries.
- **adjacency entry**: one stored `(from, to)` direction.
- **ordering identity**: provenance token derived from the exact function values and orientation captured by a collection.
- **unchanged value**: the exact prior root may be returned when an operation is a semantic no-op.

## Bibliographic additions

The parent algorithm map remains the bibliography authority. The WBT detail additionally relies on:

- **[HY11]** Hirai, Y., & Yamamoto, K. (2011). *Balancing Weight-Balanced Trees*. Journal of Functional Programming, 21(3), 287–307.

[HY11] proves valid parameter ranges and identifies `(DELTA, GAMMA) = (3, 2)` as the unique integer pair for the original `size + 1` WBT formulation.
