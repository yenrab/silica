# Atomic Primitives: Lock-Free Concurrency

## Exclusive Load/Store (LDXR/STXR)
*Like optimistic concurrency control in Mnesia*

**The Atomic Transaction Pattern:**
1. `LDXR` - Load exclusive (begin transaction)
2. Modify value in registers
3. `STXR` - Store exclusive (attempt commit)
4. Check result: success or retry

**Why it's powerful:**
- No locks = no deadlocks
- Hardware-level transactions
- Foundation of lock-free data structures

---

## Erlang Comparison: Atomic vs. Mutex

**Traditional Mutex (Elixir):**
```elixir
# Blocking, can deadlock
def update_counter(counter) do
  :ets.update_counter(counter, 1)
end
```

**Atomic Operations (Silica/AArch64):**
```rust
// Lock-free, wait-free
fn increment_atomic(ref: &AtomicUsize) {
    loop {
        let current = LDXR(ref);     // Load exclusive
        let new = current + 1;
        if STXR(ref, new) {          // Try store exclusive
            break;                   // Success!
        }
        // Failed - retry
    }
}
```

**Result:** Silica actors communicate with zero locking overhead
