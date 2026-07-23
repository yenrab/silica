# Phase E4A (`phase_e4a_permanent_one_for_one`) — TLS migration & supervisor `init()` stack bug (handoff)

This note captures **where debugging stopped**, **what changed**, **what was tried**, and **what remains** so a new session can continue without reopeninglldb transcripts.

## Symptoms

- **`phase_e4a_permanent_one_for_one`** exits badly (e.g. **138**) with little/no stdout.
- Under lldb: **`EXC_BAD_ACCESS`** executing **heap as code** (**`PC` ≈ supervisor ACB**), **not** TLV bootstrap.
- **Use the workspace-built binary**, not copies under `/tmp` (code signing quirks with lldb).

## What was verified (narrowing)

### Linker / TLV

- Replacing Mach-O TLV actor TLS with **`pthread_key_t`** + **`pthread_getspecific` / `pthread_setspecific`** (emitter-maintained AArch64 helpers) eliminated TLV-related hypotheses for this symptom.

### `pthread_create` / `actor_thread_main` / tramp

On thread #2 (`actor_thread_main`):

- **`pthread_create`** args are correct (**`actor_thread_main`**, **`X3` = ACB**).
- At **`blr`** into **`silica_supervisor_start_*`**, **`X19`/`X0`** and **`[+280]`/`[+8]`** are consistent — **tramp `blr`** is not blindly jumping into the wrong object from bad **`[acb+8]`**.

### Crash site (narrow)

- **`silica_supervisor_materialize_init_children`** and **`spawn_linked`** were **not** hit **before** the fault in traced runs focused on **`init`**.
- Fault correlates with **supervisor `init()` heap path** (**two** **`_silica_rt_region_alloc`** calls for return aggregate build + promote).

### lldb symbol names (Darwin)

lldb’s symtab often shows names **without** a leading **`_`** (e.g. `silica_supervisor_start_E4aSup`). **`breakpoint set --name _silica_...`** can stay **pending**; **`image lookup`** with the **lldb-style** spelling works.

**Useful breakpoints** on the workspace binary:

- `silica_supervisor_start_E4aSup`
- `silica_rt_supervisor_materialize_init_children`
- `silica_rt_actor_spawn_linked`
- `silica_rt_region_alloc`
- `init` (**address** breakpoints work; e.g. stop at **`init`** epilogue **`ldp`** / **`ret`**)

Example script ideas live under `e4a_step.lldb` / `e4a_at_ret.lldb` in this directory (may need tweaking).

## Emitter / IR root findings

### Cause A — **`tuple_make` + “rec flat” memcpy head under-allocating outer stack slab**

Flat pairing uses **`flat_alloc_size(count)`** with **`flat_alloc_size(2) = 16`** (see `prims_tuple.silica`).

The **`emit_flat_tuple_stores`** **rec_flat** path **memcopies** a **scalar `record_make`** blob of **`rsz` bytes** at offset **`0`** and lays the **next slot** at **`rsz`** (typically **`rsz + 8`** bytes needed).

**16 bytes &lt; `round16(rsz + 8)`** for realistic supervisor-flag records ⇒ **silent stack overrun** into saved frame / linkage.

### Cause B — **duplicate `ADD SP, SP, #48` at end of emitted `init`**

Current emitted **`phase_e4a_permanent_one_for_one.sams`** still shows, at end of **`init`**:

```asm
MOV X0, X4           ; promoted return in X4 → X0
ADD SP, SP, #16      ; end of aggregate_return_heap_promote_x0 scratch
ADD SP, SP, #48
ADD SP, SP, #48      ; duplicate → over-pops callee frame / corrupts LR
```

So even after widening the outer **`SUB`** for the return tuple slab, **`SP` bookkeeping is still wrong** at let tail.

**Hypothesis (not finished):**

- **`sequence → let` lowering** nests lets: outer **`%child`**, inner **`%children`**, **`right_expr`** = **`tuple_make(...)`**.
- **`let_tail_aggregate_stack_bytes`** only counts when **`tail.kind == tuple_make`**; for **`tail` = nested `let`** it returns **`0`**, so the **base == tail equality dedupe path never fires**.
- Separate theory: **`body_asm`** for an outer **`let`** already ends with **`promote + dealloc`** for the inner return tuple; the **outer **`let`** then appends **another **`dealloc`** sized like **`aggregate_callee_return_stack_bytes(inner RHS)`**, **double releasing** **`#48`** (same slab size coincidence).

Either way the **emitter must ensure exactly one POP** per logically distinct stack slab accumulated across **`rhs_asm`/`body_asm`/`promote`/`dealloc` concatenation.**

## Implemented (in tree) — **`term_emitter.silica`**

1. **Widen tuple outer stack when scalar record is memcpy’d into slot 0 of a multi-element tuple**  
   Helpers: **`tuple_make_scalar_rec_flat_first`**, **`tuple_make_rec_flat_outer_alloc_bytes`**, **`tuple_make_rec_flat_nested_trim_bytes`**.
   **`tuple_make` `alloc_asm`** SUB uses **`tuple_make_rec_flat_outer_alloc_bytes`** instead of raw **`flat_alloc_size_str`**.
   **`flat_tuple_total_alloc`** uses the same outer size and **subtracts** the **record temp `rsz`** from **`flat_tuple_nested_alloc`** (**trim**) so **`let` tail deallocation** targets the persistent outer slab consistently.

2. **`let` tail duplicate-size heuristic (partial)** — when **`tuple_sret_outer` empty**: if **`tail_stack_raw == base_stack`** (aggregate byte count collision), **`tail_stack` ← 0**.

**Gap:** **`tail_stack_raw` is often 0** when **`right_expr`** is **not** top-level **`tuple_make`**, so the duplicate **`ADD`** remains.

## Verification commands

Rebuild **compiler**, then trials (assembler output is **`*.sams`**):

```bash
cd /Volumes/2T/silica/compiler/silica-compiler/src && make
cd /Volumes/2T/silica/compiler/silica-compiler/trials/supervisors_addition && make compile all
grep -n 'ADD SP' phase_e4a_permanent_one_for_one.sams | head -20
/path/to/workspace/.../phase_e4a_permanent_one_for_one; echo exit:$?
```

Inspect **`init:`** chunk in **`phase_e4a_permanent_one_for_one.sams`** until **`RET`**: **`SUB`/`ADD` pairs must reconcile** exactly once per temporary slab left live after **`promote`**.

## Next session — concrete todos

1. **Confirm duplication source in SIR/emitter glue**  
   For **`init`**, trace which **`kind: 2` let** emits the **terminal** **`concat(body, promote, dealloc)`** and whether **`dealloc`** is emitted **both** inside recursive **`term_to_asm`** (**inner `let`** tail) **and** again **on outer `let`**.  
   **Fix shapes:**
   - **Option A:** If **`term.right_expr.kind`** is **`let`** and inner body already **`promote+dealloc`s** matching **`aggregate_callee`** for the **function return slab**, suppress **outer **`dealloc`** for that overlapping byte count (**careful**: child vs return slabs differ!).  
   - **Option B:** Extend **`let_tail_aggregate_stack_bytes`** to **descend past nested `lets`** until it hits **`tuple_make`/`record_make`**, matching what **`sir_generator`** lowers from **`sequence ... produces`** (see `sir_generator/terms/terms.silica` **`lower_sequence_body_to_lets`**).

2. **Re-runlldb at `init`** **`ret`** after fix: **`register read lr sp`**; **`LR`** must be **`silica_supervisor_start_* + 4`**, **`SP`** must satisfy **`ldp`/`ldp`/ `ret`** prologue/epilogue symmetry.

3. **Regression sniff:** other supervisor trials (**`phase_e4e_one_for_all`**, etc.) that **`sequence`-build similar init tuples`; rebuild and spot-check **`ADD SP`** tail for **`INIT`-like helpers.

4. **Goldens**: project rule — **refresh `.scout`/golden outputs only with explicit approval** once behavior is confirmed.

## File touchpoints

| Area | File(s) |
|------|---------|
| Tuple stack alloc / **`flat_tuple_total_alloc`** | `compiler/silica-compiler/src/emitter/apple_silicon_mac/terms/term_emitter.silica` |
| **`flat_alloc_size`** formula | `compiler/silica-compiler/src/emitter/apple_silicon_mac/terms/prims/prims_tuple.silica` |
| **`rec_flat`** memcpy path | **`emit_flat_tuple_stores`** in `term_emitter.silica` |
| **`_silica_rt_region_alloc`** / TLS getters | Embedded runtime in **`prims_actors_runtime_asm.silica`** (+ trial **`.sams`**) |
| Sequence → let lowering | **`sir_generator/terms/terms.silica`** (**`lower_sequence_body_to_lets`**) |

## Related lldb helpers (trial dir)

- **`e4a_step.lldb`** — trace materialize/spawn_linked/region_alloc (symbol names **without leading `_`**).
- **`e4a_at_ret.lldb`** / **`e4a_verify_lr.lldb`** — stop at **`init`** epilogue (**address offsets may change after recompile**).

---

*Last updated from active debugging toward duplicate **`ADD SP,#48`** and **`init`** **`LR`** corruption on **`phase_e4a_permanent_one_for_one`**.**
