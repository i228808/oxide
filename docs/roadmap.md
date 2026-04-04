# Roadmap and Status

This page tracks API maturity so users can judge adoption risk quickly.

## Status Legend

- **Stable**: expected to be backward-compatible within SemVer policy.
- **Evolving**: usable now, but shape/behavior may still change.
- **Planned**: not implemented yet.

## Current Feature Status

| Area | Status | Notes |
|---|---|---|
| Core app builder (`App`) | Stable | Main entrypoint for runtime and middleware setup |
| Routing (`OxideRouter`, `Method`) | Stable | Includes merge/nest and route helpers |
| Responses (`ApiResponse`) | Stable | Standard success/error JSON envelope |
| Extractors (`Config`, `Data`, `Inject`) | Stable | Production-safe singleton state extraction |
| Request-scoped extractor (`Scoped`) | Evolving | Supported and documented; ergonomics may improve over time |
| Controllers (`#[controller]`) | Stable | Macro and trait behavior documented |
| Auth (`App::auth`, auth extractors) | Stable | HS256 + bearer/cookie + role extraction |
| SQL integration (`oxide-framework-db`) | Evolving | Supports lazy and strict readiness-backed connect modes |
| CLI `new/generate/run/test/bench` | Stable | Public command surface is documented |
| CLI `migrate` | Planned | Reserved command; currently a stub |
| Supabase integration (`oxide-framework-supabase`) | Evolving | PostgREST/RPC client + strict readiness mode |
| MongoDB integration (`oxide-framework-mongodb`) | Evolving | Client/database injection + strict readiness ping |

## Near-Term Focus

1. Improve SQL integration ergonomics and startup connectivity patterns.
2. Expand migration story (`oxide migrate`) once DB workflow is formalized.
3. Keep docs and command examples synced with release changes.

## Upgrade Safety

Before upgrading versions, check:

- `docs/upgrade-notes.md`
- `docs/versioning.md`
- `CHANGELOG.md`
