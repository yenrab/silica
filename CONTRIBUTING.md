# Contributing to Silica

Thank you for your interest in the Silica project. This document describes how
to participate and what to expect when you open issues or pull requests.

## License

By contributing code, documentation, or other materials to this repository,
you agree that your contributions are licensed under the **Apache License,
Version 2.0**, the same license as the project. See the [`LICENSE`](LICENSE) file
for the full text. If you add a new file, you may use the standard Apache 2.0
header for that file type (see the appendix in `LICENSE`).

## Before you start

- Read the [README](README.md) for project goals and build instructions.
- Design and specification material lives under
  [`compiler/silica-compiler/design_documents/`](compiler/silica-compiler/design_documents/).
- Tutorials and how-tos are indexed under
  [`compiler/silica-compiler/tutorials_and_howtos/`](compiler/silica-compiler/tutorials_and_howtos/).

## How to contribute

1. **Issues** — Use GitHub issues for bug reports, feature ideas, and design
   discussion. Include enough context to reproduce bugs (platform, commands,
   expected vs. actual behavior).
2. **Pull requests** — Keep changes focused on a single concern when possible.
   Reference related issues in the PR description. Update documentation or
   tests when your change affects behavior that users or contributors rely on.
3. **Commits** — Write clear commit messages in plain language. Group related
   edits so reviewers can follow the history.
4. **Continuous Integration** — Execution of all CI trials without regression are required before submition.

## Development workflow (summary)

- **Bootstrap compiler (Rust):** `compiler/silica-bootstrap-compiler/` — see
  its README for `cargo` build options.
- **Self-hosted compiler (Silica):** `compiler/silica-compiler/src/` — see the
  root README “Building the compiler” section for `make` and prerequisites.
- **CI-style trials:** `compiler/silica-compiler/trials/` — `make integrate`
  after building `silica-compiler`.

Platform support is currently focused on **Apple Silicon (arm64 macOS)** for the
full pipeline; see the README platform notice if you work on other targets.

## Code and documentation style

- Match the style of surrounding code and files in the same area of the tree.
- Prefer minimal, purposeful changes over large refactors unless coordinated
  through an issue or design discussion.
- For compiler-building agent graphs and tools, see
  [`compiler/silica-compiler/compiler-building-tools/`](compiler/silica-compiler/compiler-building-tools/).

## Community conduct

Be respectful and constructive in issues, pull requests, and reviews. Assume
good intent; disagree on technical merits clearly and professionally.

## Questions

If something in this document or the build is unclear, open an issue so the
project can improve these instructions.
