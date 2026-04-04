# Versioning

Oxide follows semantic versioning at the workspace release level.

## Policy

- **MAJOR**: breaking public API or behavioral compatibility changes
- **MINOR**: backward-compatible features
- **PATCH**: backward-compatible fixes and docs/tooling corrections

## Public API Scope

Public API includes:

- exports from `oxide-framework-core/src/lib.rs`
- documented CLI command surface in `oxide-framework-cli`
- documented extension traits and types in `oxide-framework-db`

Changes to undocumented internals may happen without deprecation guarantees.

## Deprecation Guidance

When practical:

1. mark old APIs as deprecated
2. document migration path
3. remove in a later major release

For behavior changes, update docs in the same pull request.
