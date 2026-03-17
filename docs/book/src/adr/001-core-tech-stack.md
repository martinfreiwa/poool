# ADR 001: Core Technology Stack - Rust, Axum, HTMX, and SQLx

*   **Status:** Accepted
*   **Date:** 2026-03-08
*   **Deciders:** Antigravity (AI Architect), Martin (User)

## Context and Problem Statement

The POOOL platform is a financial/RWA tokenization engine requiring high security, institutional-grade reliability, and a responsive but simple-to-maintain frontend. The previous agency provided a compiled frontend and lacked a robust backend implementation. We need a stack that ensures type safety, memory safety, and high performance while allowing for rapid iteration on the dashboard features.

## Decision Drivers

*   **Security (FinTech):** Memory safety and strong typing are non-negotiable for handling financial transactions.
*   **Maintainability:** The frontend needs to be highly interactive without the complexity of a heavy SPA framework (like React/Angular) which can lead to state synchronization issues.
*   **Performance:** Low latency for market data and asset processing.
*   **Developer Experience:** Efficient development workflow with strong compiler-level guarantees.

## Considered Options

1.  **Node.js/TypeScript + React**: Industry standard for web, but lacks the memory safety guarantees of Rust and introduces high JS-bundle complexity.
2.  **Go + Hugo/Template**: Excellent performance, but less expressive type system compared to Rust.
3.  **Rust (Axum) + HTMX + Alpine.js**: Leveraging Rust for the heavy lifting and HTMX for "Hypermedia as the Engine of Application State" (HATEOAS).

## Decision Outcome

Chosen option: **Option 3 (Rust + Axum + HTMX + SQLx)**.

### Consequences

*   **Good:**
    *   **Memory Safety:** Rust prevents entire classes of bugs (null pointers, data races) common in financial software.
    *   **SSR Simplicity:** HTMX allows us to keep business logic in the backend while providing a dynamic, SPA-like user experience.
    *   **SQL Safety:** SQLx provides compile-time verification of SQL queries against the actual database schema.
    *   **Institutional Trust:** Using a systems language like Rust signals a high standard of engineering to potential partners/investors.
*   **Bad:**
    *   **Learning Curve:** Rust has a steeper learning curve than Python or Node.js.
    *   **Compile Times:** Rust compilation is slower than Go or Node.js.

## Pros and Cons of the Chosen Option

### Pros
*   **Zero-cost abstractions.**
*   **Seamless HTMX Integration:** Small HTML fragments are returned via Axum handlers, making the UI extremely fast as only changed parts of the DOM are updated.
*   **PostgreSQL Native:** SQLx is built specifically for async PostgreSQL performance.

### Cons
*   Requires disciplined documentation (which we are currently addressing via this workflow).
*   Complexity in managing large HTMX partials if not modularized correctly.
