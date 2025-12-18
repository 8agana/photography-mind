# Tools

- `photography` (CLI) — import roster, lookups (email/contact/family/skater), status updates (mark sent/purchased), reports, thank‑you flows, ShootProof sync helpers.
- `photography_mcp` (MCP server) — Axum-based; exposes health/status plus ops tools: find_skater, get_family/contact, mark_gallery_sent, list_pending_galleries, competition_status, create_family/shoot, link_family_shoot, record_purchase, shootproof sync (orders/galleries), etc.
- `photography_schema` — initialize/update schema.
- `photography_test_data` — seed sample data.
- `photography_verify` — validation helpers.

Notes:
- Ops-only: no cognition/embeddings; thinking lives in SurrealMind.
- Default transport is stdio; HTTP is optional (set `PHOTO_HTTP_ADDR`, e.g., `0.0.0.0:8788`).
