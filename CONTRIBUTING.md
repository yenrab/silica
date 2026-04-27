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
   Specifically, before a pull request is submitted, `make integrate` (run from
   `compiler/silica-compiler/trials/`) must pass without updating the golden
   files for any trial other than the one being added. The only exception to
   this rule requires explicit approval of the community leaders.

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

## Rules

* We have a zero tolerance policy for failure to abide by our [code of conduct](CODE_OF_CONDUCT.md). It is very standard, but please make sure
  you have read it.
* Issues may be opened to propose new ideas, to ask questions, or to file bugs.
* Before working on a feature, please talk to the core team/the rest of the community via a proposal. We are
  building something that needs to be cohesive and well thought out across all use cases. Our top priority is
  supporting real life use cases like yours, but we have to make sure that we do that in a sustainable way. The
  best compromise there is to make sure that discussions are centered around the *use case* for a feature, rather
  than the proposed feature itself.
* Before starting work, please comment on the issue and/or ask in the discord if anyone is handling an issue. Be aware that if you've commented on an issue that you'd like to tackle it, but no one can reach you and/or demand/need arises sooner, it may still need to be done before you have a chance to finish. However, we will make all efforts to allow you to finish anything you claim.



## Questions

If something in this document or the build is unclear, open an issue so the
project can improve these instructions.
