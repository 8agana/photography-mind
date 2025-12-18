# Connections & Endpoints

- **Database:** SurrealDB `ws://127.0.0.1:8000`, namespace `photography`, database `ops`, user `root`/`root`. Legacy env aliases `SURR_DB_*` also work for the CLI.
- **MCP server (photography_mcp):**
  - Stdio: default.
  - HTTP (optional): set `PHOTO_HTTP_ADDR` (e.g., `0.0.0.0:8788`). Health: `http://127.0.0.1:8788/healthz` (no auth). Tool list: `http://127.0.0.1:8788/mcp` (may require bearer token if configured).
- **Ports in use (local):** 8000 (SurrealDB), 8788 (photography_mcp HTTP when enabled).
