# `src_selfhost/` — parallel self-host compiler tree

Self-host migration edits land **only** in this directory.

## Freeze rules (until Phase 6 cutover)

- Production tree `../src/` stays untouched for self-host work.
- `silica-bootstrap-compiler` stays the default builder for `../src/`.
- **This tree** (`src_selfhost/`) compiles with the seed binary `../src/silica-compiler` (not `silica-boot`).
- Build is **config-driven** like trials: regenerate `silica.config.compiler` → copy to `silica.config` → run the seed with no argv → `.sams` → `.o` → link.
- Do not edit `../src/` to land alias removal, BST→WBT, dialect cleanup, or ABI cleanups aimed at self-host.

## Sync rule

Refresh from frozen `src/` only deliberately (re-copy or cherry-pick). Never reverse-merge self-host-only changes into `src/` until the §13 / Phase 6 cutover gate passes.

## Edit discipline

**Crafted edits only.** No tree-wide batch rewriters, bulk regex migrations, or unattended multi-file dialect passes. Change one scoped API or module at a time; review before the next step.

## Progress

1. Parallel tree re-frozen from `src/` (2026-07-17): includes seed block-comment lexer fix; BST→WBT + config-driven Makefile restored after re-copy.
2. Emitter `bst` → WBT via `data_structures/compiler_maps.silica` (atom / int / float pools, `int_rodata`). `data_structures/bst.silica` removed from this tree only.
3. Next (§11 Step 3): remove `type` aliases — crafted, file by file.
4. Later: named-struct / List dialect work and `build-selfhost` (still deferred; still crafted-only).
