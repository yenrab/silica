# Memory Region Types: A Tutorial

This tutorial explains the different **memory space types** in Silica's region-based memory model. Each type configures how memory is cached, shared, and accessed—affecting both performance and correctness.

---

## Overview

When you allocate a region with `alloc_region(space)`, you choose a **memory space**. The space determines:

- **Cache behavior**: Write-back, write-through, or non-cacheable
- **Visibility**: When writes become visible to other cores or devices
- **Atomicity**: Whether hardware atomic operations are supported

| Memory Space | Cache Policy | Typical Use |
|--------------|--------------|-------------|
| `normal` / `normal_writeback` | Write-back cacheable | General-purpose data (default) |
| `normal_writethrough` | Write-through cacheable | Data shared with other cores or DMA |
| `normal_noncacheable` | Non-cacheable | DMA buffers, device-shared memory |
| `atomic` | Write-back + atomic ops | Lock-free counters, concurrent structures |
| `device` | Device memory | Driver library only (reserved) |

---

## Lists and `mem` effects

`List[T]` values allocate storage in a **Silica memory region**. In the self-hosted compiler, list construction and growth are implemented as **region + chunk buffers** under a single **`mem(<space>)`** declaration (see **`design_documents/list_implementation_design.md`** §9.3).

**Surface syntax:** use a **`sequence`** block with **`sequence proc[mem(<space>)]`** … **`produces`** **`pure`** … **`end`** for any code that **constructs**, **grows**, or **pattern-matches** lists (effects are **not** declared on **function** **return** **types**—put **`sequence`** **inside** **named** **functions** **when** **needed**). **`<space>`** is the same memory-space name you would pass to **`alloc_region(<space>)`** (e.g. **`normal`**, **`normal_writethrough`**, **`atomic`**). The **same** **`mem(<space>)`** applies to **all** buffers allocated for that list’s spine. If the block also performs **console I/O**, add **`device_io`**, e.g. **`sequence proc[mem(normal), device_io]`**.

**Trials:** **`silica-compiler/trials/list_addition/`** — **`list_int64_mem_effect_sequence.silica`** uses **`mem(normal)`**; **`list_int64_mem_writethrough.silica`** uses **`mem(normal_writethrough)`**; **`list_int64_two_primaries_shared_suffix.silica`** uses **`mem(normal), device_io`** with **`print`**; **`list_int64_recursive_sum.silica`** uses **`sequence proc[mem(normal)]`** **inside** **`sum_list`**.

---

## normal / normal_writeback

**What it is:** Write-back cacheable memory. Reads and writes go through the CPU cache. Writes are buffered in cache and flushed to main memory on eviction or explicit sync. This is the **default** and **fastest** option for most workloads.

**When to use it:**
- General-purpose data structures (trees, lists, graphs)
- Local computation buffers
- Any data that is not shared with other cores or external devices
- When you don't need immediate visibility of writes

**Example:** A region for building a data structure or storing intermediate results:

```silica
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal) <- alloc_region(normal);
    cell: ref(L1, normal, int64) <- alloc_ref(r, 42);
produces pure 0 end
```

---

## normal_writethrough

**What it is:** Write-through cacheable memory. Writes update **both** the cache and main memory immediately. Reads can still be served from cache. Data written by one core becomes visible to other cores and DMA devices sooner than with write-back.

**When to use it:**
- Producer-consumer buffers where another core or device reads your writes
- Debug or instrumentation buffers that must reflect the latest state
- When you need stronger consistency guarantees without going fully non-cacheable

**Example:** A ring buffer where a DMA controller reads data written by the CPU:

```silica
sequence proc[mem(normal_writethrough)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal_writethrough) <- alloc_region(normal_writethrough);
    ring: buf(L1, normal_writethrough, int64, 256) <- alloc_buf(r, 256);
produces pure 0 end
```

---

## normal_noncacheable

**What it is:** Non-cacheable memory. All accesses go **directly to memory**, bypassing the cache. No cache coherency overhead. Required when hardware or protocols assume uncached access.

**When to use it:**
- **DMA buffers**: Memory that a device (GPU, NIC, disk controller) reads or writes directly
- **Shared memory with devices**: When a device and CPU share a buffer and caching would cause incoherent views
- **Memory-mapped I/O regions** (when not using `device` space)
- When cache coherency traffic would hurt performance more than cache misses

**Example:** A DMA buffer for a network card to receive packets:

```silica
sequence proc[mem(normal_noncacheable)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal_noncacheable) <- alloc_region(normal_noncacheable);
    dma_buf: buf(L1, normal_noncacheable, int64, 256) <- alloc_buf(r, 256);
produces pure 0 end
```

---

## atomic

**What it is:** Memory space that supports **hardware atomic operations**. Uses the same cache attributes as normal write-back, but allocations in this space can hold `atomic_ref` values. Use `alloc_atomic` to create atomic references for lock-free counters and similar patterns.

**When to use it:**
- **Lock-free counters**: Reference counts, statistics, event counts
- **Concurrent data structures**: When multiple actors or threads update shared state without locks
- **Compare-and-swap (CAS) and other atomics**: Spinlocks, queues, or coordination primitives

**Example:** A shared counter incremented by multiple actors:

```silica
sequence proc[mem(atomic)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, atomic) <- alloc_region(atomic);
    counter: atomic_ref(L1, atomic, int64) <- alloc_atomic(r, 0);
produces pure 0 end
```

---

## device

**What it is:** Device memory for memory-mapped I/O. Non-cacheable, non-gathering, non-reordering—suitable for hardware registers and device buffers. **Reserved for the future device driver library.** Application code should use `normal`, `normal_writethrough`, or `normal_noncacheable` instead.

**When to use it:** Do not use in application code. This space is for low-level driver infrastructure.

---

## Quick Reference

| Need | Choose |
|------|--------|
| Fast local data, no sharing | `normal` |
| Data visible soon to other cores/DMA | `normal_writethrough` |
| DMA buffers, device-shared memory | `normal_noncacheable` |
| Lock-free atomics, shared counters | `atomic` |
| Driver / MMIO (reserved) | `device` |

---

## Effect Declarations

The memory space you use appears in your sequence's effect list:

```silica
sequence proc[mem(normal)]           // normal memory
sequence proc[mem(normal_writethrough)]
sequence proc[mem(normal_noncacheable)]
sequence proc[mem(atomic)]           // atomic operations
```

For `alloc_atomic`, you need both `mem(atomic)` and `atomic`:

```silica
sequence proc[mem(atomic)]   // alloc_atomic requires mem(atomic)
    ...
```

See §4.4, §9, and §22.1 in the Silica specification for full details.
