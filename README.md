# Photography Mind (ops)

Lean SurrealDB-backed CLI/MCP-ready code for photography operations (families/skaters/events/gallery status). No thinking tools/embeddings here; cognition lives in SurrealMind.

## Binaries
- `photography` (main CLI): import roster, list/show, update gallery status, thank-you flows, purchases, status reports.
- `photography_schema`: initialize photography schema.
- `photography_test_data`: seed sample data.
- `photography_verify`: validation helpers.
- `reembed_photography_kg`: (legacy) re-embed photography KG if present.

## Build
```bash
cargo build --release
```

## Run CLI example
```bash
SURR_DB_URL=127.0.0.1:8000 \
SURR_DB_USER=root \
SURR_DB_PASS=root \
cargo run --bin photography -- \
  import --competition "2025 Pony Express" --file /path/to/roster.csv
```

## Docs
- docs/Photography-Database-README.md
- docs/MAINTENANCE.md

## Notes
- Expects SurrealDB namespace `photography`, db `ops`.
- Keep embeddings/think tools out of this crate; this is ops-only. Cognition stays in SurrealMind.
