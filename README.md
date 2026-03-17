# POOOL Platform 🏦

Welcome to the **POOOL Platform** – an institutional-grade platform democratizing access to Fractional Real Word Assets (RWA) via tokenization. 

This repository operates as a **Monorepo**, housing the entire application stack engineered for high-security environments, specifically FinTech. It includes an Angular/Tailwind marketing frontend, an ultra-fast HTMX/Alpine.js developer dashboard, and a memory-safe Rust/Axum backend.

---

## 🏗 Repository Architecture

The project is structured logically separating domains:

```text
poool/
├── backend/          # Core API & Business Logic (Rust, Axum, SQLx, PostgreSQL)
├── frontend/
│   ├── platform/     # Investor/Admin Dashboard (HTMX, Server-Side Rendered)
│   └── www/          # Marketing Website (Angular + Tailwind CSS, High SEO Focus)
├── database/         # PostgreSQL Schema and Migrations
├── docs/             # Technical Specifications (Masterplan, DB Schema, Knowledge)
└── scripts/          # Python/NodeJS Utility & Setup Scripts
```

---

## 🚀 Quick Start (Local Development)

To spin up the entire application stack seamlessly on your local machine:

**1. Prerequisites:**
Ensure you have the following installed:
- [Rust & Cargo](https://rustup.rs/)
- [PostgreSQL](https://postgresql.org/) (Version 16 or newer)

**2. Database Initialization:**
Start your local PostgreSQL instance and import the secure database schema:
```bash
createdb poool
psql -d poool -f database/001_initial_schema.sql
```

**3. Environment Configuration:**
Configure backend environment secrets by copying the template file:
```bash
cd backend
cp .env.example .env
```
*(Optionally modify the `DATABASE_URL` inside `.env` to match your local setup).*

**4. Start the Application Server:**
Launch the Axum backend which serves both the API and the full Frontend stack:
```bash
cargo run
```

✅ The platform will be live at: **[http://localhost:8888](http://localhost:8888)**

---

## 🔒 Security & Performance Guidelines
Since POOOL executes financial transactions, this repo strictly enforces:
- **Zero Client-Side Business Logic:** All critical routing and verification happen in the Rust backend via SSR (Server-Side Rendering).
- **Strong Crypto:** Passwords use `Argon2id` hashing, and sessions mandate strict HTTP-only cookies.
- **ACID Transactions:** Every financial event is wrapped in a PostgreSQL transaction; no floats are allowed for monetary values (strictly integers).

## 📖 Extended Documentation
For deep technical integrations, consult our documentation hub:
- [`docs/MASTERPLAN.md`](docs/MASTERPLAN.md) — The global architecture vision & workflows.
- [`docs/DATABASE_SCHEMA.md`](docs/DATABASE_SCHEMA.md) — The exact definitions of the relational models.
- [`docs/KNOWLEDGE_HANDOFF.md`](docs/KNOWLEDGE_HANDOFF.md) — Historical context and architectural decisions.
