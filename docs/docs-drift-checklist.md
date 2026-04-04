# Docs Drift Checklist

Use this checklist whenever Oxide APIs change to keep docs accurate and runnable.

## Core Surface Changes

- If `oxide-framework-core/src/lib.rs` exports change:
  - update `docs/api-reference.md`
  - update relevant topic docs (`docs/routing.md`, `docs/responses.md`, `docs/state.md`, `docs/auth.md`, `docs/controllers.md`)
- If `App` builder methods change in `oxide-framework-core/src/app.rs`:
  - update `docs/app-builder-reference.md`
  - update examples in `README.md` and `docs/getting-started.md`

## Routing / Controllers

- If `oxide-framework-core/src/router.rs` changes:
  - verify `Method` variants and `OxideRouter` method list in `docs/routing.md`
- If `oxide-framework-macros/src/controller.rs` changes:
  - verify supported route attributes in `docs/controllers.md`
  - verify `new`/`Default` fallback and controller middleware behavior

## Responses / Error Shapes

- If `oxide-framework-core/src/response.rs` changes:
  - update response factory tables in `docs/responses.md`
  - confirm example JSON envelope still matches implementation

## State / Extractors

- If `oxide-framework-core/src/extract.rs` or scoped injection changes:
  - update `docs/state.md`
  - update `docs/app-builder-reference.md` scoped-state section

## Auth

- If files in `oxide-framework-core/src/auth/` change:
  - update `docs/auth.md` token source precedence and rejection messages
  - verify role extractor behavior and examples

## CLI / Workspace Layout

- If crates are renamed or command flags change:
  - update `README.md` install commands
  - update `docs/cli.md`
  - update crate layout in `docs/architecture.md`

## Quick Verification Steps

Run after doc edits:

```bash
cargo check --workspace
```

Optional sanity checks:

```bash
cargo test --workspace
cargo run -p oxide-framework-cli -- --help
```

## CI Guardrails

This repository includes CI checks in `.github/workflows/ci.yml` for:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- docs link checking for `README.md` and `docs/**/*.md`
