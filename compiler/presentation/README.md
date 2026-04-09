# AArch64 Hardware Primitives Presentation

## For Erlang/Elixir Developers

This presentation explains how Silica leverages AArch64 hardware primitives to achieve C-like performance with Erlang-like safety.

## Slide Order

1. **[01_title.md](01_title.md)** - Introduction and overview
2. **[02_memory_operations.md](02_memory_operations.md)** - Load/store instructions and memory ordering
3. **[03_atomic_primitives.md](03_atomic_primitives.md)** - Lock-free concurrency with LDXR/STXR
4. **[04_memory_safety_hardware.md](04_memory_safety_hardware.md)** - MTE and PAC for zero-cost safety
5. **[05_vector_processing.md](05_vector_processing.md)** - NEON and SVE for data parallelism
6. **[06_memory_model_ordering.md](06_memory_model_ordering.md)** - Memory ordering guarantees
7. **[07_architecture_features.md](07_architecture_features.md)** - 64-bit addressing and cache coherence
8. **[08_performance_comparison.md](08_performance_comparison.md)** - Performance benchmarks and claims
9. **[09_conclusion.md](09_conclusion.md)** - Key takeaways and future implications

## Converting to Presentation Format

These markdown files can be converted to slides using:
- **Marp**: `marp *.md --pdf` for PDF slides
- **reveal.js**: Convert markdown to HTML slides
- **Google Slides/PowerPoint**: Copy-paste content

## Target Audience

- Erlang/Elixir developers interested in systems programming
- Functional programmers curious about hardware acceleration
- Engineers evaluating Silica for high-performance applications

## Key Analogies Used

- ETS operations ↔ memory load/store
- gen_server calls ↔ acquire/release semantics
- Mnesia transactions ↔ sequential consistency
- Flow parallelism ↔ vector processing
- BEAM scheduler ↔ hardware core selection
