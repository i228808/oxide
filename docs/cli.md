# Oxide CLI

Install the `oxide` binary from the workspace:

```bash
cargo install --path oxide-framework-cli
```

Or run without installing (from the Oxide repo root):

```bash
cargo run -p oxide-framework-cli -- <subcommand>
```

From a **scaffolded app** that is its own workspace (e.g. after `oxide new`), the `oxide-framework-cli` package is not a workspace member. Use an installed `oxide`, or:

```bash
cargo run --manifest-path /path/to/Oxide/oxide-framework-cli/Cargo.toml -- generate controller User
```

## Commands

### `oxide new <name>`

Creates `<name>/` with a minimal binary crate: `Cargo.toml` (standalone `[workspace]`, `oxide_framework_core` dependency), `app.yaml`, `src/main.rs`, and `src/controllers/hello_controller.rs` with `#[controller]` and a `// oxide-framework-cli:routes` marker for safe route generation.

| Option | Default | Description |
|--------|---------|-------------|
| `--oxide` | `path=../oxide-framework-core` | Dependency line: `path=../oxide-framework-core` or `version=0.1.0` |
| `--force` | off | Remove existing directory before creating (destructive) |

If `<name>` exists and is **empty**, it is removed and recreated without `--force`.

### `oxide generate controller <name>`

| Option | Default | Description |
|--------|---------|-------------|
| `--prefix` | `/api` | Controller URL prefix (`#[controller("...")]`) |
| `--style` | `macro` | `macro` = `#[controller]` impl in `src/controllers/`; `functional` = `OxideRouter` module under `src/routes/` |
| `--force` | off | Overwrite an existing file |

Updates `src/controllers/mod.rs` (macro) or `src/routes/mod.rs` (functional).

### `oxide generate route <controller> <method> <path>`

Appends a route handler **after** `// oxide-framework-cli:routes` inside the controller `impl` block (or falls back to inserting before the closing `}` of `impl <Controller>`). Generated handlers return `ApiResponse<Msg>`; adjust types as needed.

| Option | Description |
|--------|-------------|
| `--force` | Add even if the same `#[get("/path")]` (etc.) already exists |

Examples:

```bash
oxide generate route HelloController GET /items
oxide generate route UserController POST /register
```

### `oxide generate middleware <name>`

Creates `src/middleware/<name>.rs` with a no-op `tower::Layer` stub and updates `src/middleware/mod.rs`. You implement real behavior and attach with `App::layer(...)`.

### `oxide run`

Runs `cargo run` in the current directory. Forward Cargo flags after `--`:

```bash
oxide run -- --release
```

Environment variables (`PORT`, `RUST_LOG`, etc.) are inherited by the child process.

### `oxide test`

- **Inside the Oxide repository** (if `oxide-framework-core/Cargo.toml` exists): `cargo test --workspace`.
- **Elsewhere:** `cargo test` for the local crate.

### `oxide bench`

- **Inside the Oxide repository** (if `oxide-framework-core/benches/overhead.rs` exists): runs `cargo bench -p oxide-framework-core --bench overhead`, then `cargo run -p oxide-framework-core --release --example loadtest`.
- **Elsewhere:** `cargo bench` for the local crate.

### `oxide migrate`

Reserved. Prints a stub message until a database / ORM integration exists.

## Safety

- `oxide new` refuses to overwrite a **non-empty** directory unless `--force` is set.
- `oxide generate controller` refuses to overwrite an existing file unless `--force`.
- `oxide generate route` detects duplicate route attributes unless `--force`.

