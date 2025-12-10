# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2025-12-10

### Fixed
- **MCP Parameter Mismatch:** Fixed `create_family` tool - was looking for `email` param but router schema defined `delivery_email`.
- **MCP Parameter Mismatch:** Fixed `create_shoot` tool - was looking for `shoot_date` param but router schema defined `date`.
- **Silent Failures:** `mark_gallery_sent` and `mark_shoot_sent` now verify edge exists before UPDATE, returning proper error instead of false success.
- **Duplicate Edges:** `link_family_shoot` now checks for existing edge before RELATE, prevents duplicate family-shoot relationships.
- **Async I/O:** Sync handlers (`sync_shootproof_galleries`, `sync_shootproof_orders`) now use `tokio::fs::read_to_string` instead of blocking `std::fs`.

### Changed
- **Status Tool:** Added `shoot` and `family_shoot` table counts to `handle_status` output.
- **Code Organization:** Extracted `PendingFamily` struct to `models.rs` (was duplicated in server.rs).
- **Dependencies:** Upgraded rmcp 0.9.0 → 0.11.0 (graceful shutdown fix for streamable-http transport, `_meta` field support).

---

## [0.1.0] - 2025-11-29

### Added
- **MCP Server:** Fully implemented `photography_mcp` server using Axum and RMCP for remote connectivity.
- **Router:** Added `router.rs` to handle MCP tool dispatching.
- **Schema:** Expanded `photography_schema.rs` to include `shoot`, `family_shoot`, and `shot_in` tables for non-competition photography (portraits, events, etc.).
- **Models:** Updated `models.rs` to support new shoot-related data structures.

### Changed
- **Safety Refactor:** Modified `commands.rs` to use non-destructive `UPDATE` queries for status changes (`mark_sent`, `request_ty`, `send_ty`, `record_purchase`, `set_status`). 
  - *Context:* Previously, these commands used a `DELETE` + `RELATE` pattern which inadvertently wiped out other fields on the edge (e.g., setting `gallery_status` would remove `ty_requested`).
- **CLI:** Updated `photography` CLI structure to support new schema fields and operations.

### Fixed
- **Data Loss:** Fixed critical bug where updating a family's status would delete their "Thank You" request history.
