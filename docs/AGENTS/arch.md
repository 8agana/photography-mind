# Architecture & Guardrails

- Ops-only: no embeddings or cognition; all thinking lives in SurrealMind.
- Binaries: `photography` (CLI), `photography_mcp` (Axum MCP server), `photography_schema`, `photography_test_data`, `photography_verify`.
- MCP transport: stdio default; HTTP optional via `PHOTO_HTTP_ADDR`.
- Database: SurrealDB graph; avoid destructive edge rewrites. **Never** `DELETE ... RELATE` for edge updates—use `UPDATE ... WHERE in=$id AND out=$id` to preserve fields (gallery/request status).
- Family-first model: gallery status tracked on `family_competition`; families are delivery units.
- Code style: raw SurrealQL strings preferred for precise control of graph mutations.
