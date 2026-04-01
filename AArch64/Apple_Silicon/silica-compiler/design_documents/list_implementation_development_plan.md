# List Implementation Development Plan (silica-compiler)

This plan implements [list_implementation_design.md](list_implementation_design.md). It applies **only** to **`silica-compiler`** and **`silica-compiler/trials/list_addition/`**. It does **not** schedule work on **silica-bootstrap-compiler** or **`AArch64/Apple_Silicon/experiments/`**.

---

## Prerequisites

- **Memory regions** and **buffer** trials can be **built and run** in the **same** pipeline as the compiler (see `trials/memory_region_addition/` and related).
- **Parser** and **type** scaffolding for **`List[T]`** and **list patterns** exists or is extended per [silica-compiler-code-organization.md](silica-compiler-code-organization.md) — e.g. `parser_expressions_lists.silica`, `parser_patterns_lists.silica`, `parser_types_lists.silica`, and matching **codegen** / **SIR** files as the project names them.

---

## Phase 0 — Lock bundle and region rules (short)

**Goal:** Confirm **implementation** **matches** [list_implementation_design.md](list_implementation_design.md) **§9** (**all** **subsections** **resolved**, **including** **§9.7**: **`List[T]`** **is** **the** **bundle**; **`case`** **uses** **primary** **cursor**; **other** **cursors** **via** **variables** **/** **scope**).

**Deliverable:** **Minimal** **`List[T]`** **layout** **sketch** **(region** **+** **cursors** **+** **metadata)** **in** **compiler** **notes** **or** **code** **as** **needed** **for** **Phase** **1**.

**Exit:** **No** **blocking** **ambiguity** **from** **design** **§9**; **lowering** **can** **assume** **§9.1–§9.7**.

---

## Phase 1 — Representation and early lowering

**Goal:** Define the **internal** layout of **chunks** (vector width, **packed** `T`), **empty** state, **chunk** linkage, and **cached length** fields. Implement **early desugaring** from **surface** `List` operations to **region + buffer** operations in **one** vertical slice (e.g. **int64** only).

**Tasks**

1. Specify **chunk header** / **buffer** layout **in** the compiler design notes or **inline** in the lowering module (aligned with **memory_region** conventions).
2. Lower **`empty`**, **`prepend`**, **`remove_head`** (or spec equivalents) for **one** `T` (e.g. `int64`).
3. Lower **`length`** with **memoization** on the chosen **bundle** or **metadata** shape.
4. Wire **region** allocation and **move** semantics **per** Phase 0 decisions.

**Trials:** Extend **`trials/list_addition/`** with cases that **only** use **Phase 1** ops (build on existing prepend/remove_head/literal trials). **Every** **trial** **uses** **`sequence proc[mem(<space>)] … produces pure … end`** (see **`list_implementation_design.md`** §7–§8); **non-**`normal` **`mem`** **is** **exemplified** **by** **`list_int64_mem_writethrough.silica`**.

**Exit:** Trials **pass** for **construct**, **head** ops, **length** on **shared-suffix** scenarios **as** allowed by the **bundle** model.

---

## Phase 2 — Pattern matching and literals

**Goal:** **`case`** on lists with **`[]`**, cons, and **`_`**; complete **literal** lowering for **one** element type.

**Tasks**

1. **Parser** / **typecheck** / **lower** for **list patterns** and **match** exhaustiveness as required by the spec.
2. Ensure **wildcard** `_` and **cons** patterns **compile** to the **same** **bundle** / **cursor** model.
3. **Literal** lowering: static **element** list → **region** + **chunks** (or **empty**).

**Trials:** New trials under **`list_addition/`** for **`case`** and **literals** with **multiple** branches.

**Exit:** **All** list **pattern** forms required by **v1** **compile** and **run** in trials.

---

## Phase 3 — Kernel: map, filter, reduce

**Goal:** **Materialized** **map**, **filter**, **reduce** over **`List[T]`** for **at least** one scalar `T`, with **correct** **region** behavior and **no** user-facing **view** types.

**Tasks**

1. Implement **compiler-known** primitives or **lowered** loops that **allocate** **new** lists / **values** per design §2 and §7.
2. **Document** that **chained** map/filter **may** allocate **intermediate** lists unless a **later** optimizer pass **fuses** (optional, not Phase 3).

**Trials:** **`list_addition/`** trials for **map**, **filter**, **reduce** (small **finite** inputs).

**Exit:** Trials **pass**; **behavior** matches **immutable** **functional** semantics.

---

## Phase 4 — Collectable and non-primitive elements

**Goal:** **Region references** in **chunks** for **non-primitive** `T` **per** spec; **Collectable** checks **in** the **typechecker**.

**Tasks**

1. **Extend** chunk layout **and** **trials** for **at least** one **non-scalar** `T` (e.g. **tuple** or **spec-approved** type).
2. **Verify** **no** invalid **pointer** use without **region** authority.

**Trials:** **`list_addition/`** trials for **non-primitive** `T` (as **soon** as **supported**).

**Exit:** At **least** one **non-primitive** trial **passes**.

---

## Phase 5 — Hardening and spec alignment

**Goal:** **Error** messages, **edge** cases (empty **map**/**filter**, **reduce** on empty), **alignment** with [silica-specification.md](silica-specification.md), and **cleanup** of **temporary** SIR **list** nodes if **any** were introduced for **staging**.

**Tasks**

1. **Audit** **implementation** **against** **design** **§9** (including **`case`** **/** **primary** **cursor** **and** **scope** **of** **secondary** **cursors**); **close** **gaps** **or** **defer** **with** **explicit** **notes**.
2. **Performance** sanity (optional): **vector** **load** path **matches** **chunk** **width** on **AArch64** trials.

**Exit:** **Feature** **complete** per **design** §2 **and** **§9**; **no** **undocumented** **divergence** **from** **resolved** **§9** **decisions**.

---

## Milestone summary

| Milestone | Deliverable |
|-----------|-------------|
| M0 | Phase 0 decisions recorded |
| M1 | Phase 1 + **list_addition** trials for **construct** + **length** |
| M2 | Phase 2 + **case** + **literals** trials |
| M3 | Phase 3 + **map**/**filter**/**reduce** trials |
| M4 | Phase 4 + **non-primitive** trial |
| M5 | Phase 5 + **spec** **audit** |

---

## Dependencies

```mermaid
flowchart LR
  P0[Phase 0 bundle rules]
  P1[Phase 1 lowering]
  P2[Phase 2 patterns]
  P3[Phase 3 map filter reduce]
  P4[Phase 4 non-primitive]
  P5[Phase 5 hardening]
  P0 --> P1 --> P2 --> P3 --> P4 --> P5
```

**Memory-region** work **feeds** Phase 1; **parser** **list** files **feed** Phase 2.

---

## Design doc alignment

**[list_implementation_design.md](list_implementation_design.md) §9** is **fully** **resolved** (**§9.1–§9.7**). **Phase** **0** **and** **Phase** **5** **use** **it** **as** **the** **checklist** **for** **representation**, **`case`**, **cursors**, **and** **lowering**.

---

## Document history

| Version | Summary |
|---------|---------|
| 1.0 | Initial phased plan, milestones, dependencies, pointer to open questions. |
| 1.1 | Align Phase 0 / review text with design §9.7; §9.1–§9.6 resolved (runtime, chunk fill, SIR lower-all-paths). |
| 1.2 | Design §9 complete (§9.7 `List[T]`/`case`); Phase 0 & “come back” updated. |
| 1.3 | Trials: `sequence proc[mem]` shape; pointer to writethrough trial. |
