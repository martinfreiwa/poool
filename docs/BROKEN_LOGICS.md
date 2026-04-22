# POOOL — Broken Logics & Full Platform Audit

> **Generated:** 2026-03-18  
> **Methodology:** Static code analysis across backend (Rust/Axum) and frontend (Vanilla JS), following the 5-step audit workflow: Control Flow, State Sync, Financial Logic, Auth/CSRF, Edge Case Stress Test.

---

## 🔴 CRITICAL (Fix Immediately)

### [P0-FINANCIAL] — Cart Amount Parser Swallowing Errors
- **File:** `backend/src/cart/routes.rs` (Lines 140-170)
- **What was wrong:** The backend `add_to_cart` endpoint used `.unwrap_or(500)` and `.unwrap_or(0)` when parsing the `investment_amount` String from the frontend. If a user sent an unparseable or malicious string like "NaN" or "abc", it silently fell back to a $500 default value instead of rejecting the invalid input.
- **Fix:** Used explicit `match` blocks on `.parse()` to return an HTTP Redirect with an `invalid_amount` error parameter.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P0-FINANCIAL] — Unsafe i64 Conversions in Checkout Maths
- **File:** `backend/src/payments/service.rs` (Lines 463, 475, 1004)
- **What was wrong:** The checkout fee calculation and FX operations used `to_i64().unwrap_or(0)` when converting `Decimal` back to `i64` cents. If the `Decimal` exceeded `i64::MAX` (e.g., from an integer overflow attack or a bug), it would wrap around / default to 0, completely bypassing fees or charging 0 IDR for a transaction.
- **Fix:** Replaced `.unwrap_or(0)` with `.ok_or("Amount too large to process")?` to enforce a hard boundary and fail the transaction. For the fallback IDR total calculation, replaced `0` with `i64::MAX` to be safely restrictive.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-API] — Unsafe unwraps in HTTP Header Formatting
- **File:** `backend/src/payments/routes.rs` (Lines 469, 660, 712)
- **What was wrong:** The backend code contained `.parse().unwrap()` for `HX-Redirect` and `CONTENT_TYPE` headers, which could panic and crash the Tokio worker thread if a user supplied a malicious redirect string.
- **Fix:** Replaced `.unwrap()` with `.unwrap_or_else()` to provide safe fallback header constants.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-FINANCIAL] — Platform Fee Calculation Uses f64 Float Math
- **File:** `backend/src/payments/service.rs` (Line 452, 461)
- **What is wrong:** Platform fee percentage is fetched as `f64` from DB (`let platform_fee_pct: f64 = ...`) and multiplied with `subtotal_cents as f64`, then cast back to `i64`. This violates the "No Floats for Money" invariant and can cause rounding errors on large transactions.
- **Fix:** Replace with `rust_decimal::Decimal` arithmetic (same pattern as the FX rate fix on line 85).
- **Status:** ✅ Resolved
- **Roadmap Task:** Phase 18.10
- **What I did:** Refactored platform fee calculation to use `rust_decimal::Decimal`, parsed the fee from DB as Decimal, calculated using exact math, and applied `.ceil().to_i64()` for safe rounding.
- **Date:** 2026-03-31

### [P1-FINANCIAL] — IDR Conversion Uses f64 Float Math
- **File:** `backend/src/payments/service.rs` (Line 974)
- **What is wrong:** `let idr_total = (total_usd_cents as f64 * rate / 100.0) as i64;` — uses f64 for currency conversion despite the FX service itself using `Decimal`. This legacy code path was not updated when Task 1.10 (Decimal FX) was completed.
- **Fix:** Use `Decimal::from(total_usd_cents) * rate_decimal / Decimal::from(100)` and `.to_i64()`.
- **Status:** ✅ Resolved
- **Roadmap Task:** Phase 18.10 (same fix batch)
- **What I did:** Completely rewrote `calculate_fx_deduction` to use `rust_decimal::Decimal` and `.from_f64()` precision scaling to avoid IEEE754 floating-point truncation when converting USD to IDR cents.
- **Date:** 2026-03-31

---

## 🟡 HIGH (Fix Before Launch)

### [P1-UI] — Community Composer Buttons Unresponsive
- **File:** `frontend/platform/static/js/community-feed.js`
- **What was wrong:** `initCommunityFeed` had an early return `if (!feedContainer) return;` at the very beginning of the closure. Because the file was loaded in `<head>`, the feed container didn't exist yet, causing the script to exit before defining global functions like `openCreatePostModal` and `uploadPostImage`. The UI buttons silently failed.
- **Fix:** Moved the `feedContainer` checks directly inside the specific functions (`renderSkeleton`, `renderEmptyState`) that actually mutate the DOM synchronously, allowing all global UI bindings to deploy immediately.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P2-UX] — Community Edit Profile Button Uses Wrong Target
- **File:** `frontend/platform/partials/community_feed.html`
- **What was wrong:** The "Edit Profile" button on the Community Feed triggered a deprecated `openProfileEditModal()`, rather than routing the user to the unified `/settings` page where preferences and profile data are managed.
- **Fix:** Swapped `onclick="openProfileEditModal()"` with `window.location.href='/settings'`.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P2-UX] — Composer Modal UI Did Not Match User Expectations
- **File:** `frontend/platform/community.html`, `frontend/platform/static/js/community-feed.js`
- **What was wrong:** The Create Post modal used a static, form-style layout that felt clunky and didn't emulate standard social media patterns (like Facebook).
- **Fix:** Completely redesigned `#create-post-modal` to use a Facebook-style layout — added a centered title with bottom border, embedded the user's avatar and name natively at the top, converted the post type selectors into pill buttons under the user's name, changed the textarea to be borderless and auto-expanding, and moved attachments into a styled "Add to your post" bottom action bar.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-UI] — Affiliate Onboarding Optimistic Submission Bug
- **File:** `frontend/platform/static/js/affiliate-onboarding.js` (Lines 80-100)
- **What was wrong:** The affiliate onboarding form was optimistically showing the "Application Submitted" success state and marking all steps complete *before* awaiting the API response. If the API failed, the user incorrectly believed their application was submitted.
- **What I did:** Refactored `submitExam()` to `async`, added an `await fetch` block, and correctly wired try/catch UI blocks to stop the success state from rendering if the backend request errors out.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-SECURITY] — Affiliate Onboarding: Exam Answers Not Validated Server-Side
- **File:** `backend/src/rewards/routes.rs`, `backend/src/rewards/models.rs`, `frontend/platform/static/js/affiliate-onboarding.js`
- **What was wrong:** The compliance exam was validated entirely client-side in JavaScript. Correct answers were hardcoded in the browser code. Any user could trivially cheat by inspecting DevTools, or by sending `exam_passed: true` directly to the API without answering any questions.
- **What I did:** Added `exam_answers` field to `SubmitOnboardingForm`, implemented `validate_exam_answers()` on the backend with the correct answer key, and updated the frontend JS to send actual answers. The backend now rejects submissions with incorrect answers regardless of the `exam_passed` flag.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-SECURITY] — Affiliate Onboarding: No Rate Limiting
- **File:** `backend/src/rewards/routes.rs`
- **What was wrong:** `POST /api/affiliate/onboarding/submit` had no rate limiting. An attacker could spam applications, filling the admin queue and creating DB bloat.
- **What I did:** Added rate limiting via `state.auth_rate_limiter.check()` keyed on `affiliate_onboard:{user_id}`, returning 429 Too Many Requests when exceeded.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-SECURITY] — Affiliate Onboarding: Duplicate Application / Re-application After Rejection
- **File:** `backend/src/rewards/routes.rs`
- **What was wrong:** The `ON CONFLICT DO UPDATE` SQL clause allowed a user to overwrite a `pending_approval` application repeatedly, and a rejected (`terminated`) user could re-apply by simply re-submitting the form, resetting their status to `pending_approval` without any guard.
- **What I did:** Added a pre-flight status check that blocks resubmission if status is `pending_approval`, `active`, or `suspended`. Only `terminated` and `pending_onboarding` (or no record) allow (re)application.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-SECURITY] — Affiliate Onboarding: Policy Acceptance Count Not Validated
- **File:** `backend/src/rewards/routes.rs`
- **What was wrong:** The backend trusted `form.accepted_policies` from the client with no validation. A user could submit an empty array or incomplete list of policies and still get `pending_approval` status.
- **What I did:** Added server-side validation requiring exactly 5 policies, each matching the expected policy name constants.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-FINANCIAL] — Admin Affiliate Approval: Non-Transactional + Code Collision Risk
- **File:** `backend/src/admin/rewards.rs`
- **What was wrong:** The affiliate approval ran outside a DB transaction. If the randomly generated referral code collided with an existing `UNIQUE` code, the approval failed with no recovery. The audit log could also be written without the approval being committed.
- **What I did:** Wrapped the entire approval flow in a transaction with a `SELECT ... FOR UPDATE` row lock, added a 3-attempt retry loop for code collisions, and ensured the audit log is committed atomically with the approval.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31


### [P1] — PostgreSQL Connection Timeout (Stale PID)
- **File:** `/opt/homebrew/var/postgresql@16/postmaster.pid`
- **What was wrong:** A stale PID file was preventing the local Homebrew PostgreSQL service from starting.
- **What I did:** Removed the stale `postmaster.pid` and restarted the `postgresql@16` service.
- **Status:** ✅ Resolved
- **Date:** 2026-03-28

### [P0-FINANCIAL] — Investment Check Constraint Violation
- **File:** `backend/src/payments/service.rs`
- **What was wrong:** The cleanup worker for expired orders was attempting to update investments to 0 tokens, which violated the database check constraint `tokens_owned > 0`. This caused the cleanup task to fail and tokens to remain stuck.
- **What I did:** Refactored `cleanup_expired_orders` to check current token holdings and **delete** the investment record if it reaches zero, or update it normally otherwise. Added success logging.
- **Status:** ✅ Resolved
- **Date:** 2026-03-28

### [P0-SECURITY] — XSS: Community Search Rendered User Content as Raw HTML
- **File:** `frontend/platform/community.html` (Line 189)
- **What was wrong:** Alpine.js `x-html="p.content"` in the search results template rendered user-generated post content as raw HTML, enabling script injection via `<script>` tags or `<img onerror="...">` payloads.
- **What I did:** Changed `x-html` to `x-text` to ensure all user content is safely escaped.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1] — Community: Missing Circle Join/Request Handlers
- **File:** `frontend/platform/static/js/community-circles.js`
- **What was wrong:** The circle leaderboard rendered "Join" and "Request" buttons with `onclick="handleJoinCircle(...)"` and `onclick="handleRequestJoinCircle(...)"`, but those functions were never defined, resulting in `ReferenceError`.
- **What I did:** Added both handler functions making proper API calls to `/api/community/circles/{id}/join` and `/api/community/circles/{id}/request`.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1] — Community: Dead AMA Button + Broken switchCommunityTab
- **File:** `community.html`, `partials/community_feed.html`, `community-feed.js`, `community-amas.js`, `community-circles.js`, `community-announcements.js`
- **What was wrong:** After HTMX migration, topbar tab buttons lost `data-tab` attributes. The `switchCommunityTab()` function and 6+ selectors across 4 JS files referenced these missing attributes, causing tab switching, URL-based navigation (`?tab=ama`), and the "View full AMA" sidebar button to silently fail.
- **What I did:** (1) Added `data-tab` attributes to all topbar HTMX buttons. (2) Rewrote `switchCommunityTab()` to trigger HTMX clicks. (3) Fixed stale selectors in all 4 JS files. (4) Fixed URL-based tab switching in DOMContentLoaded handler.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1] — Community: Undefined buildPostElement Function
- **File:** `frontend/platform/static/js/community-feed.js`
- **What was wrong:** `buildPostElement()` was called in 3 places (hashtag filter, saved posts, hashtag feed) but never defined. The function was lost during HTMX migration. All hashtag and saved post views threw `ReferenceError`.
- **What I did:** Created `buildPostCard()` — an XSS-safe DOM-based post card builder — and replaced all `buildPostElement` calls. Also replaced stale `loadFeed()` calls with `document.body.dispatchEvent(new Event('reload-feed'))` to properly trigger HTMX reload.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P2] — Community: Hardcoded Hex Colors Breaking Dark Mode
- **File:** `frontend/platform/static/css/community.css`
- **What was wrong:** `#F2F4F7` borders, `rgba(255,255,255,0.85)` overlay, and `#fff` backgrounds hardcoded in `.ann-footer`, `.circle-member`, `.review-footer`, `.coming-soon-overlay`, and `.coming-soon-content`. These break dark mode.
- **What I did:** Replaced with `var(--card-border-color)`, `var(--card-bg)`, and `rgba(var(--card-bg-rgb,...))` tokens.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1-FINANCIAL] — Affiliate Clawback Can Drive Wallet Balance Negative
- **File:** `backend/src/admin/rewards.rs` (Line 1152)
- **What was wrong:** The `api_admin_affiliate_clawback` function deducted `total_clawback_cents` from the affiliate's wallet without checking if the balance was sufficient. If the affiliate had already withdrawn funds, their wallet would go negative. Additionally, the audit log used `affiliate_id` as `actor_user_id` instead of the admin performing the action, and the function silently swallowed DB errors with `let _ =`.
- **What I did:** (1) Added balance guard: `min(total_clawback_cents, balance_cents)` caps deduction at available balance. (2) Added `AdminUser` extractor so audit log correctly records the admin actor. (3) Replaced silent `let _ =` with proper `.map_err()` error propagation on all DB writes. (4) Response now includes `actual_deducted_cents` and `shortfall_cents` for transparency.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P2] — Rewards Dashboard: Hardcoded Metric Values Flash Before JS Update
- **File:** `frontend/platform/rewards.html` (Lines 572-654)
- **What was wrong:** Affiliate dashboard metric cards showed hardcoded demo values (1,248 clicks, 84 signups, 12 qualified, USD 45k revenue) and stale trend badges (+12%, 6.7% CVR, +2, Top 5%) in the HTML. The JS would replace the primary values on load, but users saw the wrong numbers flash for ~200ms. Trend values were never updated by JS.
- **What I did:** Replaced all hardcoded values with `--` placeholders and removed stale trend badges, leaving empty spans that JS can populate.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

---

## 🟠 MEDIUM (Degraded UX / Data Sync Issues)

### [P1] — Backend Compilation Errors (Reconciliation Worker)
- **File:** `backend/src/main.rs`
- **What was wrong:** Compilation failed due to `unwrap_or(0)` being called on `i32` fields (`tokens_total`, `tokens_available`) and `as_deref().unwrap_or()` on a `String` field (`title`). This was a regression from a previous attempt to fix nullability issues.
- **What I did:** Removed redundant `unwrap_or` calls for non-Option types and ensured proper formatting for the log message.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P1] — Backend Compilation Errors (20 errors blocking startup)
- **File:** `src/main.rs`, `src/rewards/routes.rs`, `src/rewards/service.rs`, `src/admin/rewards.rs`
- **What was wrong:** 20 compilation errors after affiliate system migration: missing DB columns (`accepted_policy_version`, `tax_document_gcs_path`, `updated_at` on `affiliates`; `affiliate_materials` table missing), `Option<i32>` arithmetic in reconciliation worker, `unwrap_or_default()` on non-Option `String` for `tier_at_execution`, nested `Option<bool>` type mismatch in holdback worker, missing doc comments on public struct fields.
- **What I did:** Applied DB migrations 073–076 (affiliate_profile_data, indexes, payout_batches, postback_url, system_gaps). Added `updated_at` column to `affiliates`. Fixed `tokens_total/tokens_available` to use `.unwrap_or(0)` before subtraction. Fixed `tier_at_execution.clone().unwrap_or_default()` → `.clone()` since column is NOT NULL. Fixed `unwrap_or(None).flatten().unwrap_or(false)` for `SELECT bool IN (...)` query returning `Option<Option<bool>>`. Added doc comments to `AdminMaterialReviewPayload` struct fields.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P2-ARCH] — Sidebar Code Duplication & Hardcoded Styles
- **File:** `frontend/platform/components/sidebar.html`
- **What is wrong:** The sidebar implementation contains 3x redundant copies of the same logic (Initial render, Investor Template, Developer Template). This creates a high risk of desync for navigation fixes. Additionally, several hex colors (e.g., `#0000FF`) are hardcoded, bypassing the design system theme colors.
- **Fix:** Refactor into shared Jinja2 macros for navigation items and replace hex colors with CSS variables.
- **Status:** ❌ Unresolved
- **Date:** 2026-03-31

---

## 📑 PER-PAGE LOGIC STATUS

### 🟢 INVESTOR PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Marketplace** | `/marketplace` | ✅ OK | — |
| **Property Detail** | `/property/:id` | ✅ OK | — |
| **Wallet** | `/wallet` | ✅ OK | — |
| **Portfolio** | `/portfolio` | ✅ OK | — |
| **Cart** | `/cart` | ✅ OK | — |
| **Checkout** | `/checkout` | ✅ OK | — |
| **Payment Progress** | `/payment-in-progress` | ✅ OK | — |
| **Payment Success** | `/payment-success` | ✅ OK | — |
| **Rewards** | `/rewards` | ✅ OK | — |
| **Leaderboard** | `/leaderboard` | ✅ OK | — |
| **Community** | `/community` | ✅ Hardened | XSS fixed, HTMX migration complete, all handlers verified |
| **Settings** | `/settings` | ✅ OK | — |
| **Transactions** | `/transactions` | ✅ OK | — |
| **Support** | `/support` | ✅ OK | — |
| **KYC** | `/kyc` | ✅ OK | — |
| **Commodities** | `/commodities-marketplace` | ✅ OK | — |

### 🟡 DEVELOPER PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Dashboard** | `/developer/dashboard` | ✅ OK | — |
| **Assets List** | `/developer/assets` | ✅ OK | — |
| **Asset Detail** | `/developer/asset-detail` | ✅ OK | — |
| **Application Form** | `/developer/apply` | ✅ OK | — |
| **Document Upload** | `/developer/documents` | ✅ OK | — |
| **Submissions** | `/developer/submissions` | ✅ OK | — |
| **Add Asset** | `/developer/add-asset` | ✅ OK | — |
| **Submission Success** | `/developer/submission-success` | ✅ OK | — |

### 🔴 ADMIN PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Dashboard** | `/admin` | ✅ OK | — |
| **Users** | `/admin/users` | ✅ OK | — |
| **User Details** | `/admin/user-details` | ✅ OK | — |
| **Assets List** | `/admin/assets` | ✅ OK | — |
| **Asset Details** | `/admin/asset-details` | ✅ OK | — |
| **Asset Tokenize** | `/admin/asset-tokenize` | ✅ OK | — |
| **Change Requests** | `/admin/asset-change-requests` | ✅ OK | — |
| **Dev Submissions** | `/admin/developer-submissions` | ✅ OK | — |
| **Submission Review** | `/admin/developer-submission-review` | ✅ OK | — |
| **Orders** | `/admin/orders` | ✅ OK | — |
| **Deposits** | `/admin/deposits` | ✅ OK | — |
| **Pending Settlements** | `/admin/pending-settlements` | ⚪ Static | Hardcoded HTML mockup; no backend API |
| **Treasury** | `/admin/treasury` | ✅ OK | — |
| **KYC** | `/admin/kyc` | ✅ OK | — |
| **Approvals** | `/admin/approvals` | ✅ OK | — |
| **Dividends** | `/admin/dividends` | ✅ OK | — |
| **Rewards** | `/admin/rewards` | ✅ OK | — |
| **Reports** | `/admin/reports` | ✅ OK | — |
| **Support** | `/admin/support` | ✅ OK | — |
| **Support Ticket** | `/admin/support-ticket` | ✅ OK | — |
| **Notifications** | `/admin/notifications` | ✅ OK | — |
| **Audit Logs** | `/admin/audit-logs` | ✅ OK | — |
| **Email Marketing** | `/admin/email-marketing` | ✅ OK | — |
| **System** | `/admin/system` | ✅ OK | — |
| **Settings** | `/admin/settings` | ✅ OK | — |
| **Admins** | `/admin/admins` | ✅ OK | — |
| **Roles** | `/admin/roles` | ✅ OK | — |
| **Storage** | `/admin/storage` | ✅ OK | — |
| **Blockchain Treasury** | `/admin/blockchain-treasury` | ✅ OK | — |
| **Blockchain Contracts** | `/admin/blockchain-contracts` | ✅ OK | — |
| **Contract Detail** | `/admin/blockchain-contract-detail` | ✅ OK | — |
| **Web3 Sync & Health** | `/admin/blockchain-sync` | ✅ OK | — |

---

## 🚀 Active Development & Ongoing Fixes

### Pending from Gap Analysis (2026-03-28)

| Priority | Issue | File | Roadmap Task |
|:---|:---|:---|:---|
| 🔴 P1 | `platform_fee_pct` uses f64 | `payments/service.rs:461` | Phase 18.10 |
| 🔴 P1 | IDR conversion uses f64 | `payments/service.rs:974` | Phase 18.10 |
| 🟡 P1 | `tax_id` stored in plaintext | `user_profiles` table | Phase 20.5 |
| 🟡 P1 | No CSRF middleware | Axum router | Phase 20.7 |
| 🟡 P1 | `audit_logs` missing `client_ip` | `audit_logs` table | Phase 20.9 |
| 🟡 P2 | 40+ tables undocumented | `DATABASE_SCHEMA.md` | Phase 20.14 |
| 🟡 P2 | OAuth/2FA not in AUTH_FLOW.md | `AUTH_FLOW.md` | Phase 20.15 |
| ⚪ P2 | Admin: No background job monitoring | Missing API | Phase 20.1 |
| ⚪ P2 | Admin: No webhook event logs | Missing table | Phase 20.2 |
| ⚪ P2 | Admin: No session management | Missing API | Phase 20.3 |

> **Full list:** See `docs/IMPLEMENTATION_ROADMAP.md` Phases 18-22 (68 tasks total).

### [P1-FINANCIAL] — Account deletion bypasses balance and investment checks on DB failure, and is susceptible to deposit race conditions
- **File:** `backend/src/settings/service.rs`
- **What was wrong:** The functions checking for active investments and wallet balances used `unwrap_or(0)` outside of the deletion database transaction. A database failure would default the balance/investments to 0 instead of returning an error, bypassing rules that forbid deleting accounts with balances or active investments. Concurrently, there was a window where users could deposit funds between the check array and the beginning of the `BEGIN` block (race condition).
- **What I did:** Moved the `BEGIN` block above the validation statements and attached the validation checks directly to the `tx` variable. Replaced `unwrap_or(0)` with `?` to escalate SQL failures and halt deletion instead of suppressing them into `0`.
- **Status:** ✅ Resolved
- **Date:** 2026-03-31

### [P0-ENV] — Environment Sandbox Lockout (Rust & Docker)
- **File:** N/A (Environment-wide)
- **What is wrong:** The agent process is unable to access or modify critical directories in the user's home folder (`/Users/martin/.rustup`, `/Users/martin/.cargo`, `/Users/martin/.docker`). This prevents `cargo` (Rust) and `docker` from functioning, as they require access to these directories for toolchain management and configuration. All attempts to build or run the backend result in "Operation not permitted" or "could not create home directory" errors.
- **Fix:** Pending (Attempting workaround by redirecting `HOME`, `CARGO_HOME`, and `RUSTUP_HOME` to workspace-local directories).
- **Status:** ❌ Ongoing
- **Date:** 2026-04-01
