# `OrderedMap` Detailed Design

**Public trait:** `OrderedMap`  
**Generated representation module:** `wbt_map`  
**Shared core:** [`weight_balanced_tree.md`](weight_balanced_tree.md)

## 1. Abstract value

`OrderedMap[KeyType, ValueType, mem(SpaceType)]` is a finite mapping from comparator-equivalence classes of keys to exactly one value.

Keys determine WBT placement. Values never affect placement or balance.

## 2. Constructor

```text
{
    compare_key: fn(KeyType, KeyType) -> atom,
    compare_value: fn(ValueType, ValueType) -> atom
}
```

`compare_value` supports trait-level `find_value` and deterministic value equality. It is not called by `get`, key insertion descent, deletion, or balancing.

```silica
users: OrderedMap[string, User, mem(normal)] <- wbt_map@empty({
    compare_key: compare_string,
    compare_value: compare_user
});
```

## 3. Trait contract

```text
export trait OrderedMap;
export contains_key/2;
export find_value/2;
export get/2;
export size/1;
export fold/3;
export compare_key/3;
export compare_value/3;

required {
    fn compare_key(map: OrderedMap, a: KeyType, b: KeyType) -> atom;
    fn compare_value(map: OrderedMap, a: ValueType, b: ValueType) -> atom;
    fn get(map: OrderedMap, key: KeyType)
        -> {status: :not_found | :found, value: ValueType};
    fn fold(
        map: OrderedMap,
        init: AccType,
        step: fn(AccType, KeyType, ValueType) -> AccType
    ) -> AccType;
}

provided {
    fn contains_key(map: OrderedMap, key: KeyType) -> boolean;
    fn find_value(map: OrderedMap, value: ValueType)
        -> {status: :not_found | :found, key: KeyType};
    fn size(map: OrderedMap) -> int64;
}
```

The WBT implementation overrides `size` with cached `O(1)` access. `find_value` remains an ascending in-order linear search and returns the smallest key whose value compares equal.

## 4. Generated module surface

```text
export empty/1;
export singleton/3;
export insert/3;
export delete/2;
export get/2;
export contains_key/2;
export find_value/2;
export size/1;
export fold/3;
export from_list/2;
export from_sorted/2;
export validate/1;
```

Result shapes:

```text
insert(map, key, value) -> {
    map: OrderedMap[KeyType, ValueType, mem(SpaceType)],
    inserted: boolean,
    replaced: boolean
}

delete(map, key) -> {
    map: OrderedMap[KeyType, ValueType, mem(SpaceType)],
    removed: boolean
}

get(map, key) -> {status: :not_found | :found, value: ValueType}
find_value(map, value) -> {status: :not_found | :found, key: KeyType}
```

Bulk input is a uniform list of inline records:

```text
List[{key: KeyType, value: ValueType}, SpaceType]
```

## 5. Key identity and replacement

Insertion of an absent key class creates one binding:

```text
inserted = true
replaced = false
```

Insertion of a comparator-equal key replaces only the value:

```text
inserted = false
replaced = true
```

The previously stored key representation remains canonical. `compare_value` is not used to suppress equal-value replacement because doing so could add an unexpected comparator call and because value equality need not imply representational identity.

## 6. Query semantics

`get` performs direct WBT search in `O(log n)`.

`contains_key` is logically `get(...).status == :found`, but a generated implementation need not synthesize an unused payload.

`find_value` traverses keys in ascending order and returns the first comparator-equal value. Invalid value-comparator results fail deterministically.

`fold` visits every `(key,value)` pair in ascending key order.

## 7. Delete semantics

Deletion is by key comparator class. It returns the old map unchanged if absent. Deleting a present binding removes key and value together.

No operation deletes by value. `find_value` returning a key may be followed by ordinary key deletion.

## 8. Bulk construction

`from_list` folds insert:

- later comparator-equal keys replace earlier values;
- the first key representation remains canonical;
- output ordering is independent of input order.

`from_sorted` requires keys to be strictly ascending and unique. It rejects duplicate classes rather than choosing a value.

## 9. Empty/failure behavior

| Case | Result |
|---|---|
| get empty/absent | `status=:not_found` |
| find value empty/absent | `status=:not_found` |
| delete absent | unchanged, `removed=false` |
| replacement | size unchanged |
| invalid key/value comparator atom | collection error |
| malformed sorted input | bulk result `valid=false` |
| count overflow | no new map published |

Payload fields accompanying `status=:not_found` are semantically inaccessible.

## 10. Invariants

1. WBT key ordering and balance are valid;
2. each key class has exactly one value;
3. cached size equals binding count;
4. key and value comparator bundles are preserved;
5. values remain paired with their keys through every rotation;
6. fold is strictly ascending by key;
7. `find_value` returns the least matching key.

## 11. Persistence

New-key insertion, replacement, and present deletion path-copy `O(log n)` nodes. Absent deletion returns the original root. Replacing a value shares both subtrees of the matched node.

## 12. Complexity

| Operation | Time | New nodes |
|---|---:|---:|
| get/contains key | `O(log n)` | `0` |
| insert/replace/delete | `O(log n)` | `O(log n)` on change |
| size | `O(1)` | `0` |
| find value | `O(n)` | `0` |
| fold | `O(n)` | accumulator-defined |
| from list | `O(n log u)` | persistent build |
| from sorted | `O(n)` | `O(n)` |
| validate | `O(n)` | diagnostics |

## 13. Example

```silica
m0: OrderedMap[string, int64, mem(normal)] <- wbt_map@empty({
    compare_key: compare_string,
    compare_value: compare_int64
});

r1 <- wbt_map@insert(m0, "answer", 41);
r2 <- wbt_map@insert(r1.map, "answer", 42);
q <- OrderedMap@get(r2.map, "answer");
```

`r1.inserted = true`; `r2.replaced = true`; `q` is found with value `42`; `r1.map` still maps the key to `41`.

## 14. Exclusions

No hash-map lookup, duplicate-key multimap, value index, mutable update, or comparator-free key mode is included.
