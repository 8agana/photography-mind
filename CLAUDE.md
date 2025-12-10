# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Photography Mind is a Rust-based SurrealDB CLI + MCP server for photography business operations. It tracks families, skaters, competitions, shoots, and gallery delivery status for figure skating photography. This is **ops-only** - no cognition/embeddings; that lives in SurrealMind.

## Build & Run Commands

```bash
# Development build
cargo build

# Production build
cargo build --release

# Run CLI
SURR_DB_URL=127.0.0.1:8000 SURR_DB_USER=root SURR_DB_PASS=root \
  cargo run --bin photography -- <COMMAND>

# Run MCP server (stdio transport)
cargo run --bin photography_mcp

# Run MCP server with HTTP transport
PHOTO_HTTP_ADDR=0.0.0.0:8788 cargo run --bin photography_mcp

# Initialize/update schema
cargo run --bin photography_schema

# Type checking
cargo check
```

## Code Completion Checklist

**ALWAYS** run these before calling code complete:

```bash
# 1. Format code
cargo fmt

# 2. Lint (treat warnings as errors)
cargo clippy -- -D warnings

# 3. Type check
cargo check

# 4. Build production binary
cargo build --release

# 5. Restart remote MCP service and verify
launchctl kickstart -k gui/$(id -u)/com.legacymind.photography-mind
curl http://127.0.0.1:8788/healthz  # Returns "ok" (no auth required)
```

**ALWAYS** update CHANGELOG.md when work is completed.

```bash
# 6. Commit and push after testing
git add -A && git commit -m "Description of changes"
git push
```

## Architecture

### Binaries
- `photography` - Main CLI (import, query, status updates)
- `photography_mcp` - Axum-based MCP server (stdio + optional HTTP)
- `photography_schema` - Schema initialization
- `photography_test_data` - Sample data seeding
- `photography_verify` - Validation helpers

### Module Structure
```
src/
├── lib.rs              # Re-exports all modules
├── config.rs           # Environment config (PHOTO_DB_*)
├── db.rs               # SurrealDB connection helpers
├── router.rs           # MCP tool routing (ServerHandler impl)
├── server.rs           # PhotoMindServer with tool handlers
└── photography/
    ├── mod.rs          # DEFAULT_COMPETITION constant
    ├── commands.rs     # Core business logic (raw SurrealQL)
    ├── models.rs       # Data structures (RosterRow, etc.)
    └── utils.rs        # Helpers (name parsing, fuzzy match)
```

### Canonical Sources of Truth
1. **Schema**: `src/bin/photography_schema.rs` - THE database structure definition
2. **Business Logic**: `src/photography/commands.rs` - Raw SurrealQL queries
3. **API Surface**: `src/router.rs` - MCP tool definitions

## Database Schema (SurrealDB)

**Connection**: `ws://127.0.0.1:8000`, namespace `photography`, database `ops`

### Core Tables
- `family` - Client families (delivery unit)
- `skater` - Individual skaters
- `competition` - Figure skating competitions
- `event` - Events within competitions
- `shoot` - Non-competition photography work

### Key Relations (Graph Edges)
- `competed_in` (skater → event): `gallery_status`, `request_status`, `skate_order`
- `family_competition` (family → competition): `gallery_status`, `ty_requested`, `ty_sent`, `sent_date`
- `family_shoot` (family → shoot): `gallery_status`, `purchase_amount`
- `belongs_to` (skater → family): Links skaters to families

### Gallery Status Values
`pending` | `culling` | `processing` | `sent` | `purchased` | `not_shot` | `needs_research`

## Critical Development Rules

### Non-Destructive Updates
**NEVER** use `DELETE` + `RELATE` to update edge fields. Always use `UPDATE ... WHERE in=$id AND out=$id`.

This was a hard-won lesson - the DELETE pattern wiped `ty_requested` flags when setting gallery statuses.

```rust
// WRONG - destroys other fields
db.query("DELETE family_competition WHERE in=$family AND out=$comp")
db.query("RELATE $family->family_competition->$comp SET gallery_status='sent'")

// CORRECT - preserves orthogonal fields
db.query("UPDATE family_competition SET gallery_status='sent' WHERE in=$family AND out=$comp")
```

### Raw SurrealQL
The codebase prefers raw query strings over the builder pattern for complex updates. This ensures exact control over graph operations.

### Family-Level Status Tracking
Families are atomic delivery units. Gallery status is tracked at `family_competition` level, not individual skaters.

### Sibling Disambiguation Risk
Roster import uses `parsed.skaters[0].last_name` to generate Family ID. Siblings with different last names (e.g., "Yang" vs "He") may incorrectly split the family.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PHOTO_DB_URL` | `ws://127.0.0.1:8000` | SurrealDB WebSocket URL |
| `PHOTO_DB_NS` | `photography` | Database namespace |
| `PHOTO_DB_NAME` | `ops` | Database name |
| `PHOTO_DB_USER` | `root` | Auth username |
| `PHOTO_DB_PASS` | `root` | Auth password |
| `PHOTO_HTTP_ADDR` | (none) | Set to enable HTTP transport (e.g., `0.0.0.0:8788`) |

Legacy aliases `SURR_DB_*` also work for the CLI.

## MCP Tools (via photography_mcp)

Key tools exposed via the MCP server:
- `health`, `status` - Database connectivity and counts
- `find_skater`, `get_family`, `get_contact` - Lookups
- `mark_gallery_sent`, `list_pending_galleries`, `competition_status` - Competition workflow
- `create_shoot`, `mark_shoot_sent`, `shoot_status` - Shoot workflow
- `create_family`, `link_family_shoot`, `record_purchase` - Client management
- `sync_shootproof_galleries`, `sync_shootproof_orders` - ShootProof integration

## CLI Commands (via photography)

```bash
# Import
photography import --competition "2025 Pony Express" --file roster.csv

# Queries
photography query-skater "LastName"
photography get-email "LastName"
photography list-events-for-skater --skater "LastName"

# Status updates
photography mark-sent "LastName" "CompetitionName"
photography record-purchase "LastName" 48.25 "CompetitionName"
photography set-status "LastName" "CompetitionName" "sent"

# Reports
photography check-status "CompetitionName" --pending-only
photography competition-stats "CompetitionName"

# Thank-you workflow
photography request-ty "LastName" "CompetitionName"
photography send-ty "LastName" "CompetitionName"
```
