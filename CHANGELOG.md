# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2025-11-29

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
