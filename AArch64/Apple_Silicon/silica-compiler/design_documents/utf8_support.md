# UTF-8 Support for Char Literals in substring_until_char

## Problem

The `char_lexeme_to_int64_string` function in `sir_generator/terms/literals.silica` converts char literal lexemes (e.g. `"' '"`, `"','"`) to decimal strings for the `substring_until_char` helper. Currently it uses a hardcoded lookup table covering only a few ASCII characters (` `, `,`, `x`, `z`, `\n`, `\t`). Unlisted characters fall through to `"0"`, producing incorrect behavior.

## Constraint

**No calls out to Rust runtime functions.** All logic must be implemented in pure Silica that the bootstrap compiler can compile. We cannot add new built-ins like `byte_at` or `ord` that invoke the Rust runtime.

## Solution

### 1. Lexer: UTF-8-Aware Character Advancement

The lexer's `advance` advances by one byte (`position + 1`). For multi-byte UTF-8 characters (e.g. `'é'`), the lexer would treat the second byte as the closing quote and fail.

**Change:** Add UTF-8-aware advance in `read_char_literal` (or a shared helper in `lexer_string_literals.silica`):

- 1-byte: `0x00–0x7F` → advance 1
- 2-byte: `0xC2–0xDF` → advance 2
- 3-byte: `0xE0–0xEF` → advance 3
- 4-byte: `0xF0–0xF7` → advance 4

Use the first byte of `peek_char` to determine how many bytes to advance so the lexeme spans the full character.

### 2. char_lexeme_to_int64_string: 256-Case Lookup Table

We need to convert the first byte of the character to its numeric value (0–255) using only Silica primitives.

**Approach:** Add a `byte_to_decimal` function with a 256-case lookup table that maps each possible single-byte string to its decimal representation:

```silica
// Maps single-byte string to decimal string 0-255. No runtime calls.
fn byte_to_decimal(byte_str: string) -> string {
    case len(byte_str) of {
        0 -> "0";
        _: int64 -> case substring(byte_str, 0, 1) of {
            "\x00" -> "0";
            "\x01" -> "1";
            "\x02" -> "2";
            // ... 253 more cases ...
            "\xFF" -> "255";
            _: string -> "0"
        }
    }
}
```

Then update `char_lexeme_to_int64_string`:

```silica
fn char_lexeme_to_int64_string(lexeme: string) -> string {
    inner: string <- case len(lexeme) >= 3 of { true -> substring(lexeme, 1, len(lexeme) - 1); false -> "" };
    first_byte: string <- case len(inner) >= 1 of { true -> substring(inner, 0, 1); false -> "" };
    byte_to_decimal(first_byte)
}
```

For multi-byte characters, `inner` is the full character (1–4 bytes). We take only the first byte (`substring(inner, 0, 1)`) because the `substring_until_char` helper uses byte-by-byte comparison (`LDRB` / `CMP`) and compares against the first byte of the UTF-8 encoding.

### 3. Optional: substring_until_char Semantics

The assembly helper uses `LDRB` and compares single bytes. For multi-byte characters, passing the first byte of the UTF-8 encoding correctly stops at the start of that character (e.g. `'é'` → first byte `0xC3`). If full Unicode code-point matching were desired instead, the helper would need to be rewritten to compare UTF-8 sequences rather than single bytes.

## Summary

| Component | Change |
|-----------|--------|
| **Lexer** | Make `read_char_literal` advance by full UTF-8 character length (1–4 bytes) |
| **literals.silica** | Add `byte_to_decimal` (256-case lookup) and use it in `char_lexeme_to_int64_string` |
| **No Rust runtime** | All logic stays in pure Silica; no new built-ins or runtime calls |
