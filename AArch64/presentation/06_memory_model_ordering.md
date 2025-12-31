# Memory Model Ordering: Happens-Before Relationships

## The Ordering Hierarchy

```
Silica Ordering    AArch64 Instruction    Erlang Equivalent
relaxed            LDR/STR               Dirty reads (unsafe)
acquire            LDAR                  gen_server:call/2
release            STLR                  gen_server:cast/2
acq_rel            LDAXR/STLXR + barriers  Full transaction
seq_cst            DMB + LDAR/STLR       mnesia:transaction/1
```

---

## Why This Matters for Functional Programming

**Relaxed Ordering (LDR/STR):**
- Like reading ETS without synchronization
- Fast but racy
- Use only for thread-local or immutable data

**Acquire/Release (LDAR/STLR):**
- Like message passing semantics
- "Everything before this send is visible to the receiver"
- Foundation of actor model communication

**Sequential Consistency (DMB + LDAR/STLR):**
- Like single global order of operations
- Expensive but intuitive
- Like `mnesia:sync_transaction/1`

**Key Insight:** Silica's actor communication uses acquire/release semantics by default - just like Erlang's message passing!
