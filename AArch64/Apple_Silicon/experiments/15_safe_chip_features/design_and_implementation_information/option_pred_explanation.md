# OptionPred Explanation

## What is `option<Pred>`?

**Answer**: `option<Pred>` (now `OptionPred`) represents an **optional predicate value** - it can either have a predicate or not.

## Variant Type Syntax (No Generics)

In Silica, optional values are represented using **variant types** (sum types):

```silica
// Variant type definition
type OptionPred = Some(Pred) | None

// This means OptionPred can be:
// - Some(predicate) - has a Pred value
// - None - no value (empty/absent)
```

## How It Works

### With Predicate (Some)
```silica
// Create a predicate
pred: Pred <- create_pred_true(100);

// Wrap it in Some
opt_pred: OptionPred <- Some(pred);

// Use it - processes only elements where pred is true
vec: VecInt32 <- load_vector_int32(ptr, opt_pred);
```

### Without Predicate (None)
```silica
// Use None - processes ALL elements (no filtering)
vec: VecInt32 <- load_vector_int32(ptr, None);
```

## Pattern Matching

You can pattern match on OptionPred:

```silica
fn process_with_optional_pred(ptr: *int32, opt_pred: OptionPred) 
    -> proc[mem(normal)] VecInt32 {
    
    case opt_pred of {
        Some(pred) -> {
            // Has predicate - use it for filtering
            load_vector_int32(ptr, opt_pred)
        };
        None -> {
            // No predicate - process all elements
            load_vector_int32(ptr, None)
        }
    }
}
```

## Why Optional?

SVE operations can work in two modes:
1. **With predicate**: Process only elements where predicate is true
2. **Without predicate**: Process all elements

The `OptionPred` type allows the same function to handle both cases.

## Comparison to Other Languages

- **Rust**: `Option<Pred>` (generic)
- **Haskell**: `Maybe Pred` (generic)
- **Silica**: `OptionPred = Some(Pred) | None` (variant type, no generics)

## Summary

- `OptionPred` = variant type for optional predicates
- `Some(Pred)` = has a predicate
- `None` = no predicate (process all)
- No generics - uses variant type syntax instead
