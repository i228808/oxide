# Contributing

Thanks for helping improve Oxide.

## Prerequisites

- Rust toolchain compatible with workspace edition `2024`
- Git

## Local Setup

```bash
git clone https://github.com/i228808/oxide.git
cd oxide
cargo check --workspace
```

## Development Workflow

1. Create a branch for your change.
2. Keep changes scoped (docs, CLI, core runtime, macros, db integration).
3. Run checks before opening a PR.

## Required Checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

For CLI changes, also run:

```bash
cargo run -p oxide-framework-cli -- --help
```

## Docs Changes

When touching public API, update docs in the same PR.

- Start from `docs/docs-drift-checklist.md`.
- Keep command examples runnable (`cargo run -p ...`, correct crate paths).
- Prefer small examples that compile conceptually with current APIs.

## Commit Style

Use concise, intention-first messages, for example:

- `feat: add request-scoped state extractor`
- `fix: correct auth middleware order docs`
- `docs: align CLI examples with crate rename`
- `chore: update benchmark command paths`

## Pull Requests

Include:

- what changed
- why it changed
- how you validated it

If behavior changed, include a test update (or explain why not applicable).
