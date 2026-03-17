# Contributing to POOOL Platform

Welcome! This document outlines the standards and workflows required to maintain the high engineering quality of the POOOL platform.

---

## 1. Engineering Philosophy
- **Safety First**: Financial ledger mutations must be ACID-compliant. Always use PostgreSQL transactions for movement of funds.
- **Aesthetic Excellence**: Frontend interactions should be premium and fluid. Use micro-animations and smooth transitions (Alpine.js + CSS).
- **Self-Documenting Code**: Doc-comments (`///`) are mandatory for public modules, routes, and services.

---

## 2. Development Workflow

### Coding Standards (Backend)
1. **Type Safety**: Use explicit types (e.g., `i64` for cents) and avoid `f64` for financial values.
2. **Result Pattern**: Prefer `Result<T, E>` for error handling over `panic!`.
3. **Auditability**: Significant mutations should be logged to the `audit_logs` table.

### Coding Standards (Frontend)
1. **Separation of Concerns**: 
   - **Alpine.js**: Business logic & UI state.
   - **Vanilla CSS**: Premium styling and layout.
   - **HTMX**: Persistent data fetching and partial swaps.
2. **Accessibility**: Use semantic HTML5 elements (`<main>`, `<nav>`, `<section>`).

---

## 3. Pull Request (PR) Checklist

Before submitting a PR, ensure:
- [ ] `cargo check` and `cargo fmt` pass without errors.
- [ ] New components follow the **Hybrid Power** strategy (HTMX + Alpine).
- [ ] Code is documented with RustDoc comments.
- [ ] Any database schema changes include a SQL migration file in `/database`.
- [ ] PR description includes business context and screenshots/videos for UI changes.

---

## 4. Documentation Strategy
We maintain a "Documentation-as-Code" culture:
- **ADRs**: Architectural Decision Records for major system changes.
- **mdBook**: All Markdown files in `/docs` are synchronized to the [Internal Wiki](file:///Users/martin/Projects/poool/docs/book/book/index.html).

---

## 5. Branching & Deployment
- `main`: Production-ready code.
- `staging`: Integration testing for the next release.
- `feat/*` or `fix/*`: Individual development branches.

> [!TIP]
> **Commit Messages**
> Follow conventional commits: `feat: add deposit success UI`, `fix: handle edge-case null reference in wallet`.
