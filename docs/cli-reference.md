# CLI Reference

Canonical command reference for `oxide-framework-cli/src/main.rs`.

## Install

```bash
cargo install --path oxide-framework-cli
```

## Run from workspace

```bash
cargo run -p oxide-framework-cli -- <subcommand>
```

## Commands

### `oxide new <name>`

- `--force`
- `--oxide <path=../oxide-framework-core|version=...>`

### `oxide generate controller <name>`

- `--prefix <path>` (default `/api`)
- `--style <macro|functional>`
- `--force`

### `oxide generate route <controller> <method> <path>`

- `--force`

### `oxide generate middleware <name>`

- `--force`

### `oxide run [-- <cargo args...>]`

Forwards to `cargo run`.

### `oxide test [-- <cargo args...>]`

- In Oxide repo root: `cargo test --workspace`
- Elsewhere: `cargo test`

### `oxide bench [-- <cargo args...>]`

- In Oxide repo root: benchmark + load-test flow
- Elsewhere: `cargo bench`

### `oxide migrate [-- <args...>]`

Reserved, currently stubbed.
