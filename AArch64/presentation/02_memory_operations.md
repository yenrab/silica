# Memory Operations: The Foundation

## Load/Store Instructions (LDR/STR)
*Like reading/writing to an ETS table without locks*

**Relaxed Memory Ordering:**
- No synchronization guarantees
- Like `ets:insert/2` without transactions
- Fastest but least safe

**When you need this:**
- Reading immutable configuration
- Writing to thread-local data
- Performance-critical code where you control access

---

## Load-Acquire (LDAR) / Store-Release (STLR)
*Like gen_server calls with proper synchronization*

**Acquire Semantics (LDAR):**
- Ensures all subsequent reads see the loaded value
- Like receiving a message that establishes a happens-before relationship

**Release Semantics (STLR):**
- Ensures all prior writes are visible before the store
- Like sending a message that completes a transaction

**Erlang Equivalent:**
```erlang
% LDAR/STLR pair is like:
{ok, State} = gen_server:call(Server, get_state),  % acquire
gen_server:cast(Server, {update, NewState}),      % release
```
