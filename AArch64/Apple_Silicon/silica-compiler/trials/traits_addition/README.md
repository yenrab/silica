# Trait trials (`traits_addition`)

Silica trait model per **`design_documents/silica-compiler&language-specification.jsonld`** (Section 30, `silica:Traits`, `silica:CollectableTrait`, `silica:ListType`).

| Trial | JSON-LD anchors | Intent |
|--------|-----------------|--------|
| `01_trait_required_methods_inline_type.silica` | `silica:Section30_1_1` — required methods; `silica:Section3_2` — `trait_declaration` / `impl_declaration` | Trait with **required** methods only; `impl` for an **inline record** type; factory builds values. |
| `02_multiple_traits_same_inline_type.silica` | `silica:Section30_1_2` — independent traits, same type | Two traits implemented separately for one inline record shape; illustrates **no trait inheritance**. |
| `03_trait_polymorphism_two_inline_types.silica` | `silica:Section30_2`, `silica:TraitBasedPolymorphism` | Same trait **interface**, two different inline types—**polymorphism via trait**. |
| `04_marker_trait_empty.silica` | `silica:Section30_1_1` — `markerTraits`; `silica:Traits` — `markerTraits` (ActorState, ActorMessage) | Empty trait body; `impl` with empty method set for an inline type (marker-style). |
| `05_collectable_language_rule_documentation.silica` | `silica:CollectableTrait` — `languageRule`, `automaticImplementation` | Documents that **explicit `impl Collectable` is disallowed** for built-in cases; compilable stub `main`. |
| `06_list_int64_collectable.silica` | `silica:ListType`, `silica:CollectableTrait` — primitives implement Collectable | Runnable stub: documents that list element types use Collectable and `int64` is automatic per jsonld. Full `List[int64, …]` executables: `../list_addition/`. |

**`silica.config`:** Lists **05** and **06** only so `make` / the compiler succeeds with today’s type checker. Trials **01–04** are design-faithful sources aligned with `silica:Section30_*`; append them to `silica.config` when `trait` / `impl` are accepted (same situation as `structs_addition/07_struct_traits.silica`).

**Best practice** (same as `structs_addition/07_struct_traits.silica`): use **factory functions** to construct inline records that participate in traits.

**Golden files (`.ascomp` / `.scout`):** Not checked in. After compilation stabilizes for a trial, capture assembly and stdout for `integrate` if desired.

**`rebuild-silica-configs.sh`:** That script lists every `*.silica` in the directory. If you run it here, restore **`silica.config`** to only **05** and **06** (or compilation will fail on **01–04** until the type checker accepts `trait`).
