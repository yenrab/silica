# Crypto Proposal Explained (Simplified)

## What's Being Proposed: Making Cryptography Safer by Design

This proposal adds language-level protections to Silica so that cryptographic code is safer by default.

### The Core Problem It Solves

Today, crypto bugs often come from:
- **Timing leaks**: code that runs faster or slower based on secret data
- **Accidental exposure**: secrets ending up in logs or error messages
- **Control-flow leaks**: branching on secret values

These are hard to catch in code review and can slip into production.

### The Proposed Solution: Build Safety Into the Language

Instead of relying on guidelines and audits, the compiler enforces safety rules.

### Key Features

1. **Secret vs. public labels**
   - Values are marked `Secret` or `Public`
   - Secrets can't accidentally become public
   - The compiler tracks secrecy through the code

2. **Constant-time operations**
   - Comparisons of secrets return a special mask type (`CtMask`), not a boolean
   - This prevents timing-based leaks

3. **No secret-dependent branching**
   - Secrets can't be used in `case` expressions or guards (Silica has no `if` or loops; control flow is via `case` and recursion)
   - Recursion depth and which branch is taken must be determined by public values only—secrets can be *data* inside a recursive computation, but must not *control* how many steps run or which path is taken (that would leak via timing)
   - Prevents control-flow and work-factor leaks

4. **Safe memory access**
   - Secrets can't be used as indices into buffers or lists
   - Prevents address-based leaks

5. **Protected secret storage**
   - Special buffers for secrets that:
     - Can't be copied
     - Can't be printed/logged
     - Are overwritten (zeroized) by default when the buffer goes out of scope—the compiler enforces this behind the scenes so you don't have to remember
     - Can't use regular equality checks

6. **Special function contexts**
   - Functions marked with `proc[secret]` have extra restrictions:
     - No I/O or logging
     - No panics
     - No secret-dependent sizes or control flow

### What This Means in Practice

If you try to write insecure code, the compiler rejects it. For example:
- Using a secret in a `case` expression or guard → compile error
- Logging a secret value → compile error
- Using a secret as a buffer or list index → compile error

The compiler guides you toward safe patterns.

### The Big Picture

This shifts security from "hope developers follow guidelines" to "the compiler prevents mistakes." It's like a seatbelt: you can't forget to use it because it's built into the system.

The goal is to make it easier to write secure cryptographic code and harder to write insecure code, reducing entire classes of vulnerabilities at compile time.
