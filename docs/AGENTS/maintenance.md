# Maintenance & Ops

- **Restart service (launchd):** `launchctl kickstart -k gui/$(id -u)/com.legacymind.photography-mind`
- **Build + restart cycle:** `cargo build --release && launchctl kickstart -k gui/$(id -u)/com.legacymind.photography-mind`
- **Health check:** `curl http://127.0.0.1:8788/healthz` (expects `ok`).
- **Verify tool surface:** `curl http://127.0.0.1:8788/mcp` (may require bearer token if enabled).
- **DB connectivity:** SurrealDB at `ws://127.0.0.1:8000/rpc` (root/root, ns=db: photography/ops).
- **Logs:** (not standardized here) — check launchd logs in `~/Library/Logs/` if needed.
