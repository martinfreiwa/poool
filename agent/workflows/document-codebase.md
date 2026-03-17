---
description: Document the current codebase to industry standard and above
---

# Codebase Documentation Workflow (Premium Edition)

This workflow defines the process for achieving elite-level documentation for the POOOL platform. It transitions from basic descriptions to a comprehensive "Living Documentation" system that integrates directly with development.

## Step 1: Governance via ADRs (Architecture Decision Records)

To ensure the "Why" is never lost, we implement ADRs using the [MADR](https://github.com/adr/madr) or standard format.

1. **Initialize ADRs**: Create `/docs/adr/` to store architectural decisions.
2. **First ADR**: Document the choice of **Axum + HTMX + SQLx** as the core stack.
3. **Usage**: Every major architectural change (e.g., switching KYC providers, changing auth strategy) must have a corresponding ADR.

## Step 2: Rust Backend - Elite Documentation (`rustdoc`)

// turbo

1. **Enforce Documentation**: Add `#![deny(missing_docs)]` to `backend/src/main.rs` (or `lib.rs`) to ensure no public item is left undocumented.
2. **Rich Doc-Strings**: Use Markdown inside `///` comments to include examples, panics, and safety sections.
3. **Generate & Audit**:
```bash
cd /Users/martin/Projects/poool/backend && cargo doc --no-deps --document-private-items --open
```

## Step 3: Database & Schema Intelligence

Since we use **SQLx**, our database is our source of truth.

1. **Visual Schema**: Use a tool like `tbls` or `dbdocs` to generate an interactive ERD (Entity Relationship Diagram) from the PostgreSQL schema.
2. **SQLx Offline Metadata**: Ensure `sqlx-data.json` is updated so the schema can be audited without a live DB connection for CI/CD documentation checks.
3. **Migration Docs**: Every migration in `/database/migrations/` should contain comments explaining the business logic change, not just the SQL.

## Step 4: Security & Compliance Model (High Priority)

As a financial/RWA platform, documenting the security surface area is mandatory.

1. **Auth Flow**: Create a Mermaid diagram for the Argon2id hashing -> JWT issuance -> HTMX session management flow.
2. **RBAC Matrix**: Create a table in `docs/SECURITY.md` mapping roles (Admin, Investor, Developer) to specific `/api` endpoints and UI components.
3. **Sensitivity Map**: Document where PII (Personally Identifiable Information) and financial records are stored and how they are encrypted at rest.

## Step 5: HTMX & Alpine.js Component Explorer

In a multi-tool frontend (SSR + HTMX + Alpine), state management can become "invisible".

1. **Component Catalog**: Document each "HTMX Fragment" returned by the Rust backend.
2. **Alpine.js State**: Use JSDoc to define the interface of `x-data` objects in the custom platform scripts.
3. **Visual Guide**: Use a tool like **Storybook** (if applicable) or a custom "Dev/Components" page in the dashboard to list and preview UI primitives (buttons, cards, modals).

## Step 6: Automated Documentation Enforcement (CI/CD)

Standard documentation often rots. We prevent this via automation.

1. **Link Checking**: Run a markdown link checker to ensure no broken internal or external documentation links exist.
2. **API Drift Detection**: Verify that the implemented Axum routes match the OpenAPI specification automatically.
3. **Doc-Tests**: Run `cargo test --doc` to ensure that code examples inside documentation strings actually compile and work.

## Step 7: Dependency & Compliance Audit

Maintain a clear ledger of third-party dependencies and their licenses.

1. **Rust Audit**: Use `cargo-audit` to document known security vulnerabilities in dependencies.
2. **License Inventory**: Generate a `THIRD_PARTY_LICENSES.md` using `cargo-license` to ensure compliance (especially for AGPL/GPL dependencies).

## Step 8: Operational Resilience (Disaster Recovery & Monitoring)

Industry standards for FinTech require explicit documentation on how to handle failures.

1. **Service Map**: Create a document `docs/OPERATIONS.md` listing all service dependencies (PostgreSQL, Redis, Resend, Cloud SQL).
2. **Disaster Recovery (DR)**: Document the exact steps to restore the database from a GCloud snapshot and the Recovery Time Objective (RTO).
3. **Observability**: Document the alerting thresholds in Google Cloud (e.g., 5xx errors > 1%, High Latency > 500ms).

## Step 9: Contributor Excellence (Onboarding & Standards)

Standardize how the team interacts with the codebase.

1. **CONTRIBUTING.md**: Create a guide defining branch naming (e.g., `feat/`, `fix/`), commit message standards (Conventional Commits), and the PR review checklist.
2. **Local Setup Automation**: Document exactly how to use the `scripts/` directory to spin up a clean development environment in under 5 minutes.
3. **Coding Standards**: Document linting (`clippy`) and formatting (`rustfmt`) requirements for the CI/CD pipeline.

## Step 10: The "Single Source of Truth" Portal

1. **Portal**: Use **mdBook** (the Rust standard) to compile all markdown from `/docs`, ADRs, and READMEs into a searchable, premium-looking documentation site.
2. **Deployment**: Host the internal documentation at `docs.poool.app` (internal/VPN) or a specific admin-only route `/admin/docs`.
