# Atom-keyed actor registry — direct indexing design

## 1. Purpose

Silica currently has no built-in hash map type. Recursive user-defined tree shapes for associative lookup are awkward or forbidden under structural typing. At the same time, process registries keyed by **`atom`** and storing **`actor_ref`** need expected **O(1)** lookup—not **O(n)** scans over **`List`**.

This document specifies a **representation and protocol** based on Silica's **atom model**: atoms are backed by **sequential integers** assigned in **first-use order** (monotone, stable for the lifetime of the intern table). That property allows a **dense direct index**: no hashing, no tree, no chunk-spine traversal for lookup.

**Scope**

- Naming: “registry”, “ETS-like slot table”, **atom → actor_ref**.
- Targets user code (`runtime_modules/`, supervisory registry actors) and optionally future compiler/runtime helpers.

**Non-goals**

- General **`Map[K,V]`** in the surface language (this remains a specialised pattern keyed by **`atom`** only).
- String-based hashing of atom spellings at lookup time (**not needed** when the **`atom`** value already denotes an **`int64`**‑class identifier).

Normative Silica semantics for **`atom`**, **`actor_ref`**, and regions remain in [silica-specification.md](./silica-specification.md).

---

## 2. Preconditions (atom numbering)

Assume the compiler/runtime upholds:

1. **Intern table** assigns each distinct literal atom occurrence a distinct **numeric id**.
2. **First use** dictates order: **`id ∈ { 0 … N−1 }`** for **`N`** interned atoms, **contiguous**, **immutable** once assigned (atoms are **not** garbage‑collected in the Erlang sense).
3. **`atom`** values compared or passed at runtime carry that **`id`** (implementation detail elsewhere; surfaced as **`atom`** in types).

If any future change introduced **reuse of ids**, **holes** without bound, or non-numeric **`atom`** comparison, §3–§4 would need hashing or search instead—out of scope for this design while the precondition holds.

---

## 3. Core idea: parallel slot table (`actor_ref`)

Allocate a **single contiguous indexed store** keyed by **`atom_id`** (`int64`, non-negative):

- **`slots[i]`** holds the **`actor_ref`** registered under atom id **`i`**, **`0 ≤ i ≤ max_id_seen`**.

**Slots**

Use a distinguished **empty sentinel** **`actor_ref` value meaning “unset” only if Silica distinguishes it**; otherwise reserve:

- **`valid: bool`** beside **`actor_ref`**, **or**
- **`(atom, actor_ref)`** invariant where **`atom`** repeats the key row (redundant check), **or**
- Separate **bitmap/wordset** parallel to **`slots`**.

Prefer one row shape the type system can express cleanly, e.g. **`{ occupied: bool, pid: actor_ref }`** packed in **`buf`**, **`ref`**, or **`region`** per §4.

---

## 4. Storage and lifetime (regions)

Bindings must obey Silica **`mem(Space)`**: the slot table lives in a region created under the same **`S`** as the registry **`sequence`** (see [list_memory_space_and_effects.md](../tutorials_and_howtos/list_memory_space_and_effects.md) patterns).

Suggested shapes:

| Shape | Resize | Lookup |
|-------|--------|--------|
| **`buf(R, S, Cell, Capacity)`** (if grammar supports fixed capacity) | Realloc/migrate explicitly | **`load buf[id]`** after bounds check |
| **`ref`** to heap bump under **`region(R, S)`** | Grow by **allocate larger + copy + swap handle** rare path | Indexed load after bounds |
| Allocator helper (runtime C stub) exposing indexed buffer | Matches existing **`_silica_rt_region_alloc`**‑style heaps | Emitter load/store at **`base + id * stride`** |

**Growth**

Maintain **`capacity`** and **`high_water`** (largest **`atom_id`** ever written + 1, or **`N`** atoms from compiler static count when available):

- **`atom_id ≥ capacity`** → grow table (**double** until **`capacity > atom_id`**), copy old slots, initialise new tails to empty.
- **Amortised O(1)** insert if **`atom_id`** growth is monotone and growth factor is **`> 1`**.

Compile-time‑known maximum atom count (if the toolchain reports it): **allocate once** → no growth path during process lifetime.

---

## 5. Operations

### 5.1 Register / update

Given **`key: atom`**, **`id`** its integer backing, **`v: actor_ref`**:

```
if id >= capacity → grow_slots(id + 1);
slots[id] ← v (mark occupied per §3);
maybe update high_water;
```

Precondition **“no overwrite” vs “allow replace”**: policy owned by **`pid_registry`** message handler.

### 5.2 Lookup

```
if id >= capacity ∨ !occupied[id] → not_found convention;
else return slots[id];
```

**Single indexed access** ⇒ **O(1)** worst-case time per lookup after **`slots`** resides in **`cache`/RAM** (**no hashing**).

### 5.3 Delete

Clear **`occupied`**, optionally zero **`pid`**. Do not call **`remove_actor`** implicitly here unless product policy requires it (**remains higher-level**).

### 5.4 Unregister semantics

Deleting a registration entry does **not** shrink **`atom`** ids; **`high_water`** and table capacity unchanged unless a separate compaction phase is justified (normally **not**: ids are immortal).

---

## 6. Contrast with other approaches

| Approach | Complexity | Fits Silica today? |
|-----------|-------------|---------------------|
| **`List`** scan | **O(n)** | Yes, **`pid_register_runtime`**-style assoc list |
| **BST / tree** recursive type | **`O(log n)`** nominal | Structural typing friction; user trees painful |
| **Hash table (arbitrary keys)** | **`O(1)`** expectation | No language **`Map`**; hand-built open addressing works but unnecessary when **`atom_id`** is dense **`0…N`** |
| **Direct index (**this doc**)** | **`O(1)`** | Yes, **minimal code**, **minimal metadata** |

---

## 7. Integration points

- **`pid_registry_actor`** (or successors): swap internal **`List`** for **indexed buffer + `high_water` + grow path**; **`call`** handlers branch on **`atom → id`** (**`call`** payloads carry **`atom`**; **`==`** / id comparison aligns with **`atom`** lowering).
- **Compiler**: if **`atoms_count`/`max_atom_id`** is ever surfaced statically, allocate exact capacity once and skip the **`grow`** path in steady state.
- **`remove_actor`/GC**: orthogonal—registry row eviction does **not** recycle **`atom`** numbers.

---

## 8. Document history

| Version | Summary |
|---------|---------|
| 1.0 | Initial design: **`atom`** sequential **`id`** → **`O(1)`** **`actor_ref`** slot table, region-backed growth option, contrast with **`List`**, tree, generic map. |
