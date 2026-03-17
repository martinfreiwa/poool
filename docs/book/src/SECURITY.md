# Security & Compliance Model (Institutional Standard)

This document details the security layers, data sensitivity, and access controls of the POOOL Platform.

## 1. Role-Based Access Control (RBAC) Matrix

| Feature | Endpoint | super_admin | admin | finance | compliance | support | developer | investor |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **All System Access** | `*` | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **User/KYC Review** | `/api/admin/kyc` | 🟢 | 🟢 | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Wallet/Deposits** | `/api/admin/deposits` | 🔵 | 🔵 | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Support Management** | `/api/admin/tickets` | 🟠 | 🟠 | ❌ | ❌ | ✅ | ❌ | ❌ |
| **Asset Listing** | `/api/assets` | 🟡 | 🟡 | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Buying/Investing** | `/api/checkout` | 🟣 | 🟣 | ❌ | ❌ | ❌ | ❌ | ✅ |

> [!NOTE]
> Symbols: ✅=Full Access, ❌=No Access, 🟢=Limited Oversight.

## 2. PII (Personally Identifiable Information) Sensitivity Map

POOOL handles high-stakes financial data. Data classification is as follows:

| Data Type | Field(s) | Storage | Encryption |
| :--- | :--- | :--- | :--- |
| **Credentials** | `password_hash` | `users` | **Argon2id** (m=19456, t=2, p=1) |
| **Documents** | `passport.pdf`, `id.jpg` | **GCS** | AES-256 (at rest) + Signed URLs |
| **Tax ID** | `tax_id` | `user_profiles` | AES-256 (at rest) |
| **Investment Ledger** | `balance_cents` | `wallets`, `wallet_transactions` | Immutable Rows (Logged in Audit) |

## 3. Compliance Enforcement (KYC/AML)
- **Status Gating**: The `/api/checkout` and `/api/wallets/withdraw` handlers MUST check `kyc_records.status = 'approved'` before execution.
- **Audit Logging**: Every action in the Admin Dashboard is captured in `audit_logs` including:
  - Timestamp
  - Actor ID
  - Client IP
  - Previous vs New State (JSON)

## 4. API Security Checklist
- [ ] **CSRF Protection**: All POST/PUT/DELETE requests via HTMX carry custom HX-headers managed by Axum cookie middleware.
- [ ] **XSS Prevention**: Rust `askama` or manual template escaping is used for all SSR content.
- [ ] **Rate Limiting**: Applied to `/auth/login` and `/api/deposits` to prevent brute force and DDoS.
- [ ] **Session Security**: Cookies are `HttpOnly`, `Secure`, and `SameSite=Lax`.
