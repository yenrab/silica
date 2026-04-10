# 08.3 Topology detection (experiments)

Legacy / exploratory Silica for CPU topology-style APIs. The **self-hosted compiler** tracks topology in:

- Design: [`silica-compiler/design_documents/cpu_topology_implementation_plan.md`](../../silica-compiler/design_documents/cpu_topology_implementation_plan.md)
- Emitter: `silica-compiler/src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica`
- **Phase H** static verification: `silica-compiler/trials/cpu_discovery/phase_h_static.sh` (`make -C silica-compiler/trials/cpu_discovery phase-h`)

End-to-end `.scout` trials under `trials/cpu_discovery/` await **`sequence`** lowering in the emitter (see that README).
