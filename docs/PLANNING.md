# Photography Architecture Consolidation Plan

**Status:** DRAFT
**Date:** 2025-11-22
**Author:** Gemini (Architect)

## 1. Current State Assessment

The photography infrastructure has been successfully split into two primary domains, but legacy artifacts remain scattered.

### The Core Components
| Component | Technology | Role | Status |
|-----------|------------|------|--------|
| **`photography-mind`** | Rust | **The Database Authority.** Handles SurrealDB state, business logic (Status, TY), and CLI operations. | **Active / Production.** |
| **`photography-mcp`** | Python | **The AI & Web Integration Layer.** Handles Lightroom Auth, AI Culling, and legacy scripts. | **Active / Disorganized.** |

### The Legacy Artifacts (To Be Consolidated)
| Artifact | Location | Content | Destintation |
|----------|----------|---------|--------------|
| `shootproof-integration/` | Root | SP Auth scripts, CLI for exports. | `photography-mcp/src/shootproof/` |
| `lightroom-integration/` | Root | Web demo, docs. | `photography-mcp/src/lightroom/` |
| `samataganaphotography/` | Root | Website forms? | `work-in-progress/website/` |

## 2. The Architecture Strategy

We will adopt a **"Rust Core, Python Edge"** architecture.

-   **Rust (`photography-mind`)**: strict, type-safe, handles the "Truth" (Database). It consumes data (JSON) but avoids messy OAuth flows if possible.
-   **Python (`photography-mcp`)**: flexible, handles Web APIs (Lightroom, ShootProof), Auth flows, and AI interaction (Culling). It produces data (JSON) for Rust to consume.

### The "Air Gap" Workflow (ShootProof Example)
1.  **Python:** `shootproof-auth` runs, gets token, fetches data, dumps `galleries.json`.
2.  **Rust:** `photography sync-shootproof-galleries` reads `galleries.json`, updates DB state.
*Why?* Keeps the Rust binary clean of volatile API client logic.

## 3. Execution Plan

### Phase 1: The Cleanup (Organization)
1.  Create `work-in-progress/` directory in root.
2.  Move `lightroom-integration` and `shootproof-integration` into `work-in-progress/legacy/` initially to clear the root.
3.  Move `samataganaphotography` to `work-in-progress/website/`.

### Phase 2: The Python Restructure (`photography-mcp`)
1.  Convert `photography-mcp` from a script bucket to a proper package:
    ```text
    photography-mcp/
    ├── pyproject.toml
    ├── src/
    │   ├── photography_mcp/
    │   │   ├── main.py (Server entry)
    │   │   ├── lightroom/ (Auth, Culling)
    │   │   ├── shootproof/ (Auth, Export)
    │   │   └── utils/
    ```
2.  Migrate `lightroom_auth.py` and `codex_culling.py` into this structure.
3.  Migrate `shootproof_auth.py` and `shootproof-cli` logic into this structure.

### Phase 3: The Rust Polish (`photography-mind`)
1.  **Maintenance Tools:** Port the "Rescue Scripts" (`dedupe_edges`, `merge_duplicates`) into the Rust CLI as `photography maintenance ...` commands.
2.  **Validation:** Ensure `sync_shootproof_*` commands are fully wired to the Router and CLI.

## 4. Recommendation
Approve this plan to initiate the file system cleanup and Python refactoring.
