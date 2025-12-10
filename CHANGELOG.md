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
- **Import Logic:** Updated `import_roster` to automatically create a `Family` entity if a skater has `SignUp="TRUE"` or `SignUp="VIP"`, even if they are a single entry without an email. This ensures all requested galleries are trackable in `check-status`.
- **Name Parsing:** Updated `utils.rs` to handle single-word skater names (e.g., "GriffonGliders") by treating them as `First: "Team", Last: {Name}` instead of failing.

### Fixed
- **Data Loss:** Fixed critical bug where updating a family's status would delete their "Thank You" request history.
- **Status Check:** Fixed case-sensitivity bug in `check_status` where `out.name` was not being lowercased, causing lookups to fail even when data existed.
- **Import Validation:** Fixed `Skater` import failure where existing records with missing `created_at` fields caused schema validation errors; now defaults to `time::now()`.