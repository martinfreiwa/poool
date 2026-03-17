---
description: Make the codebase more modular for maintaince
---

# Codebase Modularity & Refactoring Workflow (Industry Standard)

This workflow defines the process for transitioning the POOOL platform from a monolithic structure to a highly modular, Domain-Driven Design (DDD) architecture. This improves maintainability, testing isolation, and developer velocity by respecting our core tech stack: **Rust (Axum/SQLx)**, **HTMX/Alpine.js**, and **Vanilla CSS (BEM)**.

## Phase 0: Coordination & Tracking (Agent Sync Board) 🛰️

**Goal:** Prevent collisions and provide a clear audit trail of "who did what and when" using the **Agent Sync Board** pattern.

1.  **Initialize the Sync Board**:
    Every modularization project must start with an `[AGENT SYNC BOARD]` at the top of its task list (e.g., in `task.md` or a dedicated `<domain>_status.md`).
    
    *Example Template:*
    ```markdown
    ## [AGENT SYNC BOARD]
    - [ ] **Phase 1: Extraction** | *Status: TODO* | *Assignee: None*
    - [ ] **Phase 2: Service Layer** | *Status: TODO* | *Assignee: None*
    - [x] **Phase 3: DB Integration** | *Status: COMPLETED* | *Assignee: A-01* | *Verified: 2026-03-09T01:00*
    ```

2.  **Resolution Log**:
    Maintain a detailed log at the bottom of the tracking file to document specific actions and verification steps.
    
    *Format:*
    > **Task**: [Task Name]
    > **Status**: [State]
    > **Action Taken**: [Briefly describe the refactor/fix]
    > **Verification**: [Tool/method used to verify]
    > **Resolved By**: [Agent ID/Name] @ [ISO Timestamp]

3.  **Claiming Tasks**: 
    Before starting any tool calls for a task, the agent MUST update the Sync Board to set the status to `IN PROGRESS` and assign their ID.

## Phase 1: Domain-Driven Refactoring (Rust Backend) 🏛️

**Goal:** Extract logic from the 2,500+ line `main.rs` into feature-specific domains using Rust's trait and module systems.

1.  **Standardized Domain Structure**:
    For each domain (e.g., `support`, `wallet`, `admin`):
    - `backend/src/<domain>/mod.rs`: Domain entry point and Axum router nesting.
    - `backend/src/<domain>/handlers.rs`: Request extraction and response formatting (Axum `impl IntoResponse`).
    - `backend/src/<domain>/service.rs`: Pure business logic. Use Rust traits to abstract external deps (email, bank APIs).
    - `backend/src/<domain>/db.rs`: SQLx-driven repository layer.
    - `backend/src/<domain>/models.rs`: Strongly-typed Serde structs and SQLx record mappings.

2.  **Extraction Rules**:
    - **Handlers**: Must be thin. Use Axum extractors (`State`, `CookieJar`, `Json`, `Form`).
    - **Service**: Must be HTTP-agnostic. Use `anyhow::Result` or domain-specific errors.
    - **DB**: Use SQLx's `query_as!` macros for compile-time checked queries.

3.  **Modular Routing**:
    - Nest routers in `main.rs` to keep the entry point clean:
      ```rust
      .nest("/api/wallet", wallet::router(state.clone()))
      ```

## Phase 2: Professional Documentation & Language Standards 📝

**Goal:** Ensure elite-level documentation as per FinTech requirements.

1.  **Rust Standards**:
    - Every public item MUST have `///` rustdoc.
    - Use code examples in docs for complex service functions.
    - Document `# Panics` and `# Errors` sections.
2.  **Architectural Decision Records (ADRs)**:
    - Create ADRs in `/docs/adr/` for significant changes (e.g., "Refactoring Transaction Isolation in Wallet Service").
3.  **JS/Frontend Documentation**:
    - Use JSDoc for Alpine.js `x-data` objects and utility functions in `static/js/`.

## Phase 3: Frontend Modularity (HTMX & Alpine.js) 🧩

1.  **Fragment Strategy**:
    - Stop returning full pages for UI updates.
    - Create "Partial" templates that the Rust backend can serve via `Html` responses.
    - Move repeated fragments (e.g., `table-row`, `status-badge`) into a shared directory.
2.  **State Isolation**:
    - Use Alpine.js components to keep client-side logic scoped to the relevant HTMX fragment.
3.  **BEM CSS Encapsulation**:
    - Every domain should have its own `.css` file in `static/css/bem/` to prevent global style leakage.

## Phase 4: Shared Infrastructure 🏗️

1.  **Centralized AppState**:
    - Ensure `AppState` sharing across modules is efficient (using `Arc` where necessary).
2.  **Unified Middleware**:
    - Keep auth and audit logging middleware in `backend/src/common/`.

## Phase 5: Verification, Testing & QA 🛡️

**Goal:** Ensure zero regressions during the modularization process by implementing a robust, multi-layered testing strategy.

1.  **Static Analysis & Linting**:
    - **Action**: Run the Rust compiler and linter strictly.
    - **Command**:
      ```bash
      cd /Users/martin/Projects/poool/backend && cargo clippy --fix --allow-dirty && cargo fmt && cargo doc --no-deps
      ```

2.  **Rust Unit Testing (Service Layer)**:
    - **Action**: Every extracted `service.rs` MUST have a `#[cfg(test)]` module testing its core business logic in isolation.
    - **Implementation**: Use mock traits for external dependencies (e.g., `MockEmailService`, `MockPaymentProvider`).
    - **Standard**: Aim for >80% coverage on financial and calculation logic.

3.  **Database Integration Testing (SQLx)**:
    - **Action**: Test `db.rs` functions against a live test database.
    - **Implementation**: Use `sqlx::test` to automatically provision and roll back test databases for each test run.
      ```rust
      #[sqlx::test]
      async fn test_create_wallet(pool: PgPool) { /* ... */ }
      ```

4.  **API Integration Testing (Axum)**:
    - **Action**: Verify the fully assembled `<domain>::router` handles HTTP requests correctly.
    - **Implementation**: Use `axum::test_helpers` or `tower::ServiceExt` to send mock `axum::http::Request`s to the router and assert on the `axum::http::Response`. Test auth guards, validation errors (400), and success cases (200).

5.  **Global End-to-End (E2E) Testing (Python)**:
    - **Action**: Run the full platform E2E suite to ensure frontend-to-database flows remain intact after backend routes are moved.
    - **Command**:
      ```bash
      pytest /Users/martin/Projects/poool/tests/
      ```

6.  **QA & Review Checklist (Before Merging)**:
    - [ ] Handlers extracted to `handlers.rs` and lean.
    - [ ] Logic decoupled into `service.rs` with unit tests.
    - [ ] Database queries moved to `db.rs` with `sqlx::test` coverage.
    - [ ] Axum router nested correctly in `main.rs`.
    - [ ] APIs tested for correct JSON/HTML HTMX responses.
    - [ ] Rustdoc comments applied to all public items.
    - [ ] ADR created if architectural patterns shifted.
    - [ ] Local E2E tests (`pytest`) pass locally.
