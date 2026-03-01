# Performance: Erlang Safety, C Speed

## Comparative Performance Claims

**Memory Safety Overhead:** ≤5% vs. unsafe C
- MTE provides bounds checking with minimal cost
- PAC prevents ROP attacks for free

**Concurrency Overhead:** ≤10% vs. thread-based C
- Hardware atomics eliminate locking
- Cache coherence accelerates message passing

**Vector Performance:** Equivalent to hand-tuned SIMD
- Automatic SVE/NEON utilization
- No manual intrinsics required

---

## Real-World Performance

**Benchmark Results (Target):**
- Silica vs. C: 95% performance retention
- Silica vs. Safe Rust: 115-125% performance
- Silica vs. Erlang: 10-50x faster (estimated)

**Why Silica Wins:**
- Zero GC overhead (unlike Erlang)
- Hardware-accelerated safety (unlike C)
- Lock-free concurrency (unlike traditional languages)

---

## The Functional Programming Advantage

**What Erlang/Elixir Developers Get:**
- Familiar actor model with hardware acceleration
- Immutable data + mutable performance
- Message passing without copying overhead
- Pattern matching on hardware primitives

**The Result:** Systems programming productivity with functional programming safety
