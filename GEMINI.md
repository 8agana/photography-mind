# GEMINI.md: Photography Mind (Specialist Kernel)

**Current Status:** ACTIVE | **Mode:** SPECIALIST (Ops) | **Parent:** LegacyMind/GEMINI.md

This is the local context kernel for the `photography-mind` repository. It inherits all directives from `../../.gemini/GEMINI.md` but specializes in the operational logic of the photography business.

---

## 1. IDENTITY & SCOPE
**I am the Photography Ops Specialist.**
- **Role:** Manage the database of families, skaters, competitions, and shoots.
- **Domain:** `src/photography/`, `src/bin/`, and the SurrealDB `photography` namespace.
- **Philosophy:** "Data integrity above all." We handle purchases and delivery; mistakes here cost money and reputation.

---

## 2. ARCHITECTURE & TRUTH

### Canonical Sources
1.  **Schema:** `src/bin/photography_schema.rs` is the **ONLY** source of truth for the database structure. If it's not there, it doesn't exist.
2.  **Logic:** `src/photography/commands.rs` contains the core business logic (Raw SurrealQL).
3.  **API:** `src/bin/photography.rs` (CLI) and `src/bin/photography_mcp.rs` (Server) are the interfaces.

### The Graph Model
- **Nodes:** `family`, `skater`, `competition`, `event`, `shoot`.
- **Edges:**
    - `competed_in` (Skater -> Event): `gallery_status`, `request_status`, `skate_order`.
    - `family_competition` (Family -> Competition): `gallery_status`, `ty_requested`, `ty_sent`, `sent_date`.
    - `family_shoot` (Family -> Shoot): `gallery_status`, `purchase_amount`.

### Key Technical Decisions
- **Raw SurrealQL:** We prefer raw query strings over the builder pattern for complex updates to ensure exact control over the Graph logic.
- **Non-Destructive Updates:** **NEVER** use `DELETE` + `RELATE` to update a field on an edge. Use `UPDATE ... WHERE in=$id AND out=$id`.
    - *Lesson Learned (2025-11-29):* The previous `DELETE` pattern wiped `ty_requested` flags when setting gallery statuses.

---

## 3. CRITICAL WORKFLOWS

### The "Thank You" Loop
1.  **Request:** `request_ty` sets `ty_requested = true` on `family_competition`.
2.  **Fulfillment:** `send_ty` sets `ty_sent = true`, `ty_sent_date = time::now()`.
3.  **Status:** `check_status` reports on these flags.

### Roster Import
- **Input:** CSV (ShootProof/Registration data).
- **Logic:** `import_roster` in `commands.rs`.
- **Risk:** Sibling Disambiguation.
    - *Issue:* Logic uses `parsed.skaters[0].last_name` to generate Family ID.
    - *Watch:* Siblings with different last names (e.g., "Yang" vs "He") may split the family. Needs future refactoring.
    - *Policy:* Single skaters with `SignUp=TRUE` or `VIP` are now auto-promoted to `Family` entities to ensure delivery tracking.

---

## 4. DEVELOPMENT RULES

1.  **Verify Schema:** Before adding fields to code, check `photography_schema.rs`. Run it locally to test.
2.  **Safety First:** When writing `UPDATE` queries, always verify you aren't overwriting orthogonal fields (e.g., don't wipe `sent_date` when updating `purchase_amount`).
3.  **Tests:** Run `cargo check` after every logical change.

---

## 5. WORK LOG & MEMORY

### 2025-11-29
- **Incident:** Data loss in `family_competition` edges during status updates.
- **Fix:** Refactored `commands.rs` to use `UPDATE` instead of `DELETE`/`RELATE`.
- **Expansion:** Added `shoot` / `family_shoot` schema for non-competition work.
- **Server:** Fully implemented `photography_mcp` with `router.rs`.
- **Recovery:** Restored "Fall Fling" data.
    - Fixed parsing for single-word team names.
    - Fixed `skater` upsert to handle missing `created_at` in legacy data.
    - Deduplicated `competition` records (`2025_fall_fling` vs `fall_fling_2025`).
    - Implemented logic to auto-create families for single skaters who requested photos (`SignUp=TRUE`).