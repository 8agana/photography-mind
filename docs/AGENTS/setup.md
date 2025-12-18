# Setup / Quickstart

- Build: `cargo build --release`
- Run MCP (stdio): `cargo run --bin photography_mcp`
- Run MCP (HTTP): `PHOTO_HTTP_ADDR=0.0.0.0:8788 cargo run --bin photography_mcp`
- Run CLI example:
  ```bash
  SURR_DB_URL=127.0.0.1:8000 \
  SURR_DB_USER=root \
  SURR_DB_PASS=root \
  cargo run --bin photography -- import --competition "2025 Pony Express" --file roster.csv
  ```
- Schema init/update: `cargo run --bin photography_schema`
- Type/lint: `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, `cargo test`
- Env defaults: `PHOTO_DB_URL=ws://127.0.0.1:8000`, `PHOTO_DB_NS=photography`, `PHOTO_DB_NAME=ops`, `PHOTO_DB_USER=root`, `PHOTO_DB_PASS=root`; optional `PHOTO_HTTP_ADDR` to expose HTTP transport.
