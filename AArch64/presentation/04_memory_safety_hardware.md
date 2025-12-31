# Memory Safety Hardware: Zero-Cost Safety

## Memory Tagging Extension (MTE)
*Hardware-assisted bounds checking*

**How it works:**
- Every memory allocation gets a "tag" (4-bit identifier)
- Pointers carry their allocation's tag
- Hardware checks tag on every access
- Violation = immediate trap

**Erlang Equivalent:**
```erlang
% Like having the VM check list bounds automatically
List = [1, 2, 3],
element(10, List).  % Would trap instead of corrupt memory
```

**Performance Impact:** ≤5% overhead vs. unsafe C
**Safety:** Catches 80%+ of memory corruption bugs

---

## Pointer Authentication Codes (PAC)
*Hardware ROP protection*

**The Problem (in C/Rust):**
```c
void* ptr = malloc(100);
// Attacker overwrites return address
// ptr = (void*)some_evil_function;
free(ptr);
```

**The Solution (PAC):**
- Pointers get cryptographically signed
- Hardware validates signature on use
- Attacker can't forge valid pointers

**Functional Programming Impact:**
- Makes return-oriented programming attacks impossible
- Protects your function pointers automatically
- Zero runtime cost for validation
