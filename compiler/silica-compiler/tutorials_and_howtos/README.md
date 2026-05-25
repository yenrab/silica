<h1 align="left">
  <img src="../../../silica_icon_tiny.png" alt="" width="40" height="40" style="vertical-align: middle; margin-right: 0.35em;" />
  Silica compiler — tutorials and how-tos
</h1>

These files are **working documents**: hands-on tutorials and explanatory notes for learning Silica and the self-hosted compiler. They are **revised as the language and tooling evolve**; where they disagree with [design_documents](../design_documents/) or with the code, prefer the **language specification** and implementation unless a tutorial explicitly states it is illustrative only.

Use the entries below for **quick navigation**. Each item links to a file in this directory, followed by a short summary of what it contains.

---

## [actor_spawning_tutorial.md](./actor_spawning_tutorial.md)

**Spawning actors** and choosing **migration strategies** (`lazy`, `eager_copy`, `static_core`, etc.): gen_server-style behaviors, how stack depth relates to message turns, decision tables, examples, and performance tradeoffs.

---

## [why_no_named_types.md](./why_no_named_types.md)

Explains **why Silica avoids classic named recursive type definitions**, what problems that style hides (allocation, layout, reclamation), and how **regions** and **recursive tuples** aim to replace linked structures with explicit memory and typing discipline.

---

## [do_end_tutorial.md](./do_end_tutorial.md)

Tutorial for **`do ... end` blocks**: how they relate to function bodies `{ }`, when to use a block in **expression position**, readability, effects, and examples. (The language also documents newer **`sequence`** block spelling—see [silica_sequence_blocks_tutorial_updated.md](../design_documents/silica_sequence_blocks_tutorial_updated.md) in design documents.)

---

## [memory_region_types.md](./memory_region_types.md)

**Memory spaces** for regions (`normal`, write-through, non-cacheable, `atomic`, `device`): cache behavior, visibility, and how they connect to **`alloc_region`**, **`List[T, S]`**, and **`sequence proc[mem(S)]`** with pointers to list design and trials.

---

## [region_handles_and_references.md](./region_handles_and_references.md)

Contrasts **region handles** (`region(L, Space)`) with **region references** (`ref(L, Space, T)`): arenas vs. pointers to cells, allocation with `alloc_ref` / `alloc_buf`, and read/write patterns—tutorial form of the region model.

---

## [list_memory_space_and_effects.md](./list_memory_space_and_effects.md)

Unifies **`sequence proc[mem(S)]`**, region APIs, and **`List[T, S]`**: why the same memory space `S` appears in types and effects, rule-of-thumb for the effect checker, and canonical code shapes linking to region and list implementation docs.

---

## [ffi_wrappers_and_makefiles.md](./ffi_wrappers_and_makefiles.md)

Hands-on guide for **outbound FFI**: C wrapper functions, `dangerous_exposure_source/` sidecar metadata, `dangerous_*` Silica wrapper modules, `external_danger` FFI worker actors, `W4001` warnings, and Makefile patterns for building archives, compiling Silica, linking, and running trials.

---

## [designing_apps_with_foreign_functions.md](./designing_apps_with_foreign_functions.md)

App-design tutorial for engineers new to Silica who must use **foreign functions**: keep dangerous code concentrated in a few `dangerous_*` adapter and worker modules, route requests through cast-only FFI worker actors, supervise dangerous workers, and avoid spreading wrapper calls through business logic.
