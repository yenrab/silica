# CHAMP (Compressed Hash-Array Mapped Prefix Trie) — Design Notes

**Status:** Design decisions for a Silica implementation under `data_structures/`.

**Related:** [list_implementation_design.md](../../silica-compiler/design_documents/list_implementation_design.md) (regions, bundles, sharing), [recursive_tuple_specification.md](../../silica-compiler/design_documents/recursive_tuple_specification.md) (trees, `alloc_rec`, `ref?`), [region_handles_and_references.md](../../silica-compiler/tutorials_and_howtos/region_handles_and_references.md), [memory_region_types.md](../../silica-compiler/tutorials_and_howtos/memory_region_types.md), [silica-specification.md](../../silica-compiler/design_documents/silica-specification.md) §4.4 (regions and memory spaces).

---

## 1. Purpose

This document records **memory-region and ownership decisions** for a **CHAMP**-style persistent map (or set) in Silica: **immutable** updates with **path copying**, **bitmap-compressed** child arrays, **structural sharing** of unchanged subtrees. It does not specify full algorithms or surface API.

---

## 2. Design summary

| Dimension | Decision |
|-----------|----------|
| **Arena** | **One** `region(L, Space)` per logical map instance; **all** trie nodes and **all** stored values are allocated **in** that region. |
| **Map value (bundle)** | **One** movable aggregate: **`region(L, Space)`** (ownership / allocation authority) **plus** a **root** `ref(L, Space, …)` (or empty sentinel). The **region handle** is **not** a cell allocated inside the region; it **owns** the arena. The **root** points **into** the region. |
| **Cross-region references** | **Not used.** Keys, values, and trie nodes stay in the **same** region so **one** region authority suffices; no `ref` into a **different** region for map payload. |
| **Stored values (`V`)** | **Same region** as the trie: **inline** in leaf/entry cells where types allow, or **`alloc_ref` / `alloc_rec`** in that region. |
| **Structural sharing** | Implemented as **multiple** `ref`s into the **same** region (unchanged subtrees reused). **Not** two independent owning region tokens to the same bytes (aligned with the list **bundle** idea). |
| **Reclamation** | **Region lifetime only** — no per-node GC or refcount for trie storage. Old roots remain valid until the **region** is dropped; path copies **add** nodes without freeing predecessors individually. |
| **Effects** | Operations that allocate use **`mem(Space)`** consistent with the region’s **Space** (e.g. `sequence proc[mem(normal)]` for `normal` storage). |
| **Memory space** | Default **`normal`** (or `normal_writeback`) for typical immutable, single-threaded use. **`atomic`** or other spaces only if a future design commits to specific concurrency or visibility requirements. |

---

## 3. Functional principles: non-destructive updates and reuse (not bulk copy)

This design implements the usual **persistent** functional-data-structure properties:

- **Non-destructive:** Existing trie cells are **not** mutated in place. An update **allocates new nodes** only along the **path** from the root to where the change occurs; nodes off that path stay unchanged. Holders of an **older root** `ref` still observe the **old** map.

- **Structural sharing:** Unchanged **subtrees** are **not** duplicated. New branch nodes along the path store the **same** child `ref`s as before for every slot that did not change—**one** physical subtree, **many** logical versions referencing it. Cost is **path-length** (new cells), not **size of untouched subtrees**.

In Silica, sharing is **reusing the same `ref` values** inside one region; reclamation remains **region-scoped** (§2, §6), not per-node GC.

---

## 4. Region layout

- **Trie graph** (branch, leaf, collision, and any compact child **buffers**) lives **entirely** in **one** region, consistent with **one region per spine** for lists in the compiler list design.
- **Inter-node** links are **in-region** `ref`s only.

---

## 5. Rationale for no cross-region refs

Holding data in another region while the “map” lives in this one would require **tracking that other region’s ownership** wherever a reference to its data remains valid. That composes poorly with **linear region** reasoning and encourages **multi-region bundles**. **Restricting payload to the same region** keeps a **single** owning story: the map’s **region handle** covers **both** trie structure **and** stored **values**.

---

## 6. Consequences

- **Version retention:** Until the region is freed, **all** path-copied history reachable from held roots remains allocated. Suitable for **scoped** or **batch** use; not fine-grained per-version reclamation without copying out or a different storage strategy.
- **API shape:** Expose **one** bundle type (region + root + optional metadata such as size) so callers do not duplicate owning tokens.

---

## 7. Implementation sketch (non-normative)

- **Nodes:** Variant or tagged representation: branch (bitmap + compact child array), leaf entries, collision handling as chosen.
- **Allocation:** `alloc_rec`, `alloc_buf`, `alloc_ref` under the same **`mem(Space)`** as `alloc_region(Space)`.
- **Hashing / segment width:** Fixed per implementation (e.g. 5-bit chunks, fan-out 32) — details left to the implementation file.

---

## 8. References

- Bagwell, *Ideal Hash Trees*; CHAMP as used in persistent collection libraries (bitmap + path copying).
- Silica: **§4.4** regions, **`ref` / `alloc_*`**, **`sequence proc[mem(S)]`** in tutorials and specification.
