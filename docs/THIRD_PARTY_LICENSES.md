# Third-Party Dependency & Compliance Audit

This document outlines the software dependencies used by the POOOL platform and their respective licenses, ensuring legal compliance and transparency.

## 1. Compliance Executive Summary
- **Primary Licenses**: MIT / Apache 2.0 (Industry standard for Rust ecosystems)
- **Copyleft Risks**: None identified. No GPL/AGPL dependencies in the runtime binary.
- **Security Scanning**: Automated audits via `cargo-audit` are recommended for the CI/CD pipeline.

---

## 2. Core Dependencies & Licenses

| Dependency | Version | Purpose | License (Typical) |
| :--- | :--- | :--- | :--- |
| `axum` | 0.7.x | Web Framework | MIT |
| `tokio` | 1.x | Async Runtime | MIT |
| `sqlx` | 0.8.x | SQL Toolkit (PostgreSQL) | MIT / Apache 2.0 |
| `serde` | 1.x | Serialization | MIT / Apache 2.0 |
| `chrono` | 0.4.x | Date & Time | MIT / Apache 2.0 |
| `uuid` | 1.x | Unique Identifiers | MIT / Apache 2.0 |
| `argon2` | 0.5.x | Password Hashing | MIT / Apache 2.0 |
| `rust_decimal`| 1.x | Financial Arithmetic | MIT |

---

## 3. Dependency Inventory (Full List)

The following packages are included in the build graph (partial list):
- `allocator-api2 v0.2.21`
- `async-trait v0.1.89`
- `axum-core v0.4.5`
- `axum-extra v0.9.6`
- `base64 v0.22.1`
- `bitflags v2.11.0`
- `bytes v1.11.1`
- `futures-util v0.3.32`
- `hyper v1.8.1`
- `ring v0.17.14`
- `tracing v0.1.44`
- *(Total: 310 sub-dependencies)*

---

## 4. Security Recommendations

> [!IMPORTANT]
> **Vulnerability Management**
> It is mandated that `cargo audit` is run before every production deployment to catch known security advisories (RSEC) in open-source crates.

> [!TIP]
> **License Enforcement**
> Use `cargo-license` in build pipelines to enforce a "Permissive-only" policy, automatically blocking any accidental introduction of restrictive licenses (e.g., GPL-3.0) that could affect the platform's proprietary status.
