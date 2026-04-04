# Upgrade Notes

This page tracks notable migration notes between releases.

## Current Notes

### Crate/Workspace Path Naming

Workspace directories and package targets use hyphenated names, for example:

- `oxide-framework-core`
- `oxide-framework-cli`
- `oxide-framework-db`
- `oxide-framework-macros`

Use these in command examples:

```bash
cargo run -p oxide-framework-core --example hello
cargo run -p oxide-framework-cli -- --help
```

Dependency names in Rust code remain underscore style (Cargo crate id rules),
for example `oxide_framework_core`.

### CLI Detection Paths

CLI workspace detection now checks hyphenated paths (for example
`oxide-framework-core/Cargo.toml`) for `oxide test` and `oxide bench` behavior.
