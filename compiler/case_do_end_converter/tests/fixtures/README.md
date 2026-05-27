# Case do/end conversion fixtures

Each fixture directory contains an `input.silica` source file and the
`expected.silica` output after converting case branch bodies from `do...end`
to `{...}`.

`nested_case_branch_do_end` covers the important migration pattern where a
case branch body is a `do...end` block, and that block contains another case
with its own `do...end` branch body.

`standalone_do_end_removed` covers ordinary `do...end` blocks that should be
unwrapped by removing the `do` and `end` tokens without inserting replacement
delimiters.

The other fixtures cover unchanged already-braced/plain-expression case
branches, nested standalone `do...end` unwrapping, ignored strings/comments,
and mixed case-branch conversion with an inner standalone `do...end`.
