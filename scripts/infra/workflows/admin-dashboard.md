---
description: Create and configure the Admin Dashboard (Frontend & Backend) — World-class FinTech Admin Panel
---

# Admin Dashboard Workflow

> **Context:** POOOL is a regulated FinTech platform for fractional investment in Real World Assets. This admin panel provides complete operational oversight of every user action, financial flow, and compliance requirement. It is designed by cross-referencing every route in `main.rs`, every service function, every database table across 6 migrations, and the MASTERPLAN.

---

## Complete User Action Map (Source of Truth for Admin Coverage)

Every action a user can perform on the platform must be observable and governable from the admin panel. Here is the exhaustive list derived from the actual codebase:

### Authentication & Identity
| User Action | Backend Route | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| Sign up (email/password) | `POST /auth/signup` | `users`, `user_profiles`, `wallets`, `user_roles` | User creation monitoring, fraud detection |
| Verify email | `GET /auth/verify-email`, `POST /auth/resend-verification` | `users` (email_verified→true) | Unverified account monitoring |
| Log in (email/password) | `POST /auth/login` | `user_sessions` | Session monitoring, brute-force detection |
| OAuth login (Google/Facebook) | `GET /auth/google/callback`, `GET /auth/facebook/callback` | `users`, `oauth_accounts`, `user_sessions` | OAuth account linking audit |
| Logout | `GET /logout` | `user_sessions` (delete) | Session lifecycle |
| Forgot password | `GET /forgot-password` | `password_reset_tokens` | Password reset abuse monitoring |

### Profile & Settings
| User Action | Backend Route | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| Update profile (name, phone, country) | `POST /api/settings/profile` | `user_profiles` | Profile change audit trail |
| Change email (requires password) | `POST /api/settings/email` | `users` (email, email_verified→false) | Email change monitoring |
| Change password | `POST /api/settings/password` | `users` (password_hash) | Credential change audit |
| Change phone | `POST /api/settings/phone` | `user_profiles` (phone_number) | Contact change audit |
| Update preferences (language, currency, timezone) | `POST /api/settings/preferences` | `user_settings` (language, currency, timezone) | N/A (low risk) |

### Financial Actions
| User Action | Backend Route | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| Deposit via payment method | `POST /wallet/deposit` | `wallets`, `wallet_transactions`, `payment_methods` | **Critical** — deposit verification, fraud check |
| Deposit via bank wire (multi-currency) | `POST /api/payments/deposit` | `deposit_requests` | **Critical** — pending deposit queue |
| Deposit confirmed (webhook) | `POST /api/webhooks/payments` | `deposit_requests`, `wallets`, `wallet_transactions`, `audit_logs` | **Critical** — idempotency check, reconciliation |
| Withdraw to payment method | `POST /wallet/withdraw` | `wallets`, `wallet_transactions`, `payment_methods` | **Critical** — large withdrawal approval |
| Add card (Stripe tokenized) | `POST /api/payment-methods/card` | `payment_methods` | PCI compliance monitoring |
| Add bank account | `POST /api/payment-methods/bank` | `payment_methods` | Bank verification |
| Delete payment method | `DELETE /api/payment-methods/:id` | `payment_methods` | Audit trail |
| Set default payment method | `POST /api/payment-methods/:id/default` | `payment_methods` | N/A |

### Investment & Checkout
| User Action | Backend Route | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| Add to cart | `POST /cart/add` | `cart_items` | N/A |
| Update cart quantity | `POST /cart/update` | `cart_items` | N/A |
| Remove from cart | `POST /cart/remove` | `cart_items` | N/A |
| Execute checkout (atomic) | `POST /checkout` | `wallets` ↓, `assets` (tokens_available ↓), `orders`, `order_items`, `investments`, `invoices`, `cart_items` (cleared) | **Critical** — order monitoring, token inventory |
| Payment in progress | `GET /payment-in-progress` | — (read-only status polling) | Payment processing monitoring |
| Payment success | `GET /payment-success` | — (read-only confirmation) | Order confirmation audit |

### Developer Asset Submission (4-Step Wizard)
| User Action | Frontend Page | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| Step 1: Choose asset type | `/developer/add-asset` | `developer_projects` (draft) | N/A |
| Step 2: Property content | `/developer/property-content` | `assets` (draft data) | N/A |
| Step 3: Document upload | `/developer/document-upload-step3` | `asset_documents` | **Critical** — document review |
| Step 4: Application form (financials) | `/developer/application-form` | `assets` (financial fields), `asset_milestones` | **Critical** — financial verification |
| Submission success | `/developer/submission-success` | `developer_projects` (status→submitted) | **Critical** — triggers review queue |

### Rewards & Tiers
| User Action | Backend Route | DB Tables Affected | Admin Oversight Needed |
|---|---|---|---|
| View rewards overview | `GET /api/rewards` | `rewards_balances`, `user_tiers`, `tiers`, `referral_codes` | Rewards balance monitoring |
| View tier info | `GET /api/rewards/tiers` | `tiers` | Tier configuration |
| View tier detail page | `GET /tier` | `tiers`, `user_tiers` | Tier gamification monitoring |
| Referral code generation | Auto in rewards service | `referral_codes` | Referral fraud detection |
| Referral tracking | Auto on signup | `referral_tracking` | Referral payout verification |

### Browsing & Viewing (Read-Only)
| User Action | Frontend Page | Admin Relevance |
|---|---|---|
| Browse marketplace | `/marketplace` | Asset visibility control |
| View property detail | `/property` | N/A |
| Browse commodities | `/commodities-marketplace` | Asset visibility control |
| View commodity detail | `/commodity` | N/A |
| View portfolio | `/portfolio` | Investment monitoring |
| View wallet | `/wallet` | Balance monitoring |
| View transactions | `/transactions` | Transaction monitoring |
| View KYC page | `/kyc` | KYC status monitoring |
| View support page | `/support` | Ticket monitoring |
| Developer dashboard | `/developer/dashboard` | Developer activity |
| Developer assets list | `/developer/assets` | Submission pipeline |
| View tier detail | `/tier` | Tier gamification |
| Payment success | `/payment-success` | Order completion tracking |
| Payment in progress | `/payment-in-progress` | Payment processing tracking |
| Verify email | `/auth/verify-email` | Email verification monitoring |

---

## 1. Admin Layout, Navigation & Global UX

### 1.1 Structural Setup
- [x] Create directory: `frontend/platform/admin/`.
- [x] Create a base layout (`_admin-base.html`) with admin-specific sidebar, top bar, and breadcrumbs.
- [x] Admin sidebar is role-aware (see §16 RBAC) — hide sections the logged-in admin role cannot access.

### 1.2 Top Navigation Bar
- [x] **System Health Banner:** Real-time status indicators (green/amber/red) for: Database, Mangopay/Stripe, KYC Provider (Didit/Sumsub), Xendit (IDR), Postmark (Email), Redis. Auto-refreshes via HTMX polling.
- [x] **Global Search:** Unified search bar to find Users (by email, name, UUID), Assets (by title, slug), Orders (by order number `ORD-*`), Transactions (by `external_ref_id`), Deposit Requests (by `provider_reference`).
- [x] **Admin Notification Bell:** Critical alerts (KYC flagged, large withdrawal pending, failed webhook, deposit request expired).

### 1.3 Sidebar Navigation Links
- [x] Dashboard Overview
- [x] Users & Profiles
- [x] Developer Submissions
- [x] Assets (Live)
- [x] Orders & Investments
- [x] Financial / Treasury
- [x] Deposit Requests
- [x] KYC & AML
- [x] Rewards & Referrals
- [x] Support Tickets
- [x] Notifications Center
- [x] Email & Marketing
- [x] Audit Logs
- [x] Reports & Exports
- [x] System (Jobs, Webhooks)
- [x] Platform Settings

---

## 2. Dashboard Overview (The Control Center)

- [x] Page: `frontend/platform/admin/index.html`
- [x] **KPI Cards (real-time via HTMX polling):**
    - Total Users (count `users`) | New Users (24h) | Pending KYC (`kyc_records` WHERE status='pending')
    - Total AUM (sum `investments.current_value_cents`) | Deposits (24h from `wallet_transactions` WHERE type='deposit') | Withdrawals (24h)
    - Live Assets (count `assets` WHERE published=true) | Fully Funded (WHERE tokens_available=0) | Pending Approval (count `developer_projects` WHERE status='submitted')
    - Active Orders (count `orders` WHERE status='pending' OR 'processing') | Failed Transactions (24h from `wallet_transactions` WHERE status='failed')
    - Open Support Tickets (count `support_tickets` WHERE status='open') | Pending Deposit Requests (count `deposit_requests` WHERE status='pending')
    - Rewards Wallet Total (sum `rewards_balances.cashback + referrals + promotions`)
    - Email Delivery Rate (last 24h) | Open Rate (last 30d) | Click Rate (last 30d)
- [x] **Charts Section:**
    - [ ] **Interactive Visualizations:** Line/bar charts for "User Growth," "Deposit Volume (30d)," and "AUM Trends."
    - [ ] **Global Date Range Selector:** Persistent toggle (e.g., "7 Days," "30 Days," "Custom Range") to filter all KPI tiles and charts.
    - Deposit volume trend (30-day, sourced from `wallet_transactions` WHERE type='deposit').
    - User signup trend (30-day, sourced from `users.created_at`).
    - Asset funding progress (horizontal bars per live asset, data from `assets.tokens_total - tokens_available`).
- [x] **Recent Activity Feed:** Last 50 entries from `audit_logs` (e.g., "User X deposit confirmed", "Admin Y approved KYC for User Z").
- [x] **Action Shortcuts:** Quick-links to "Review next pending KYC", "Review next pending asset", "Approve next deposit", "View failed transactions".
- [x] **System Health Interaction:**
    - [ ] **Status Detail Modals:** Clickable health dots showing API response times, specific error messages, and last successful ping.
    - [ ] **Background Refresh:** 30-second background polling for high-priority metrics (e.g., Pending KYC count, Open Tickets).
- [x] Backend: `GET /api/admin/stats/overview`, `GET /api/admin/activity-feed`.

---

## 3. User Management & Deep-Dive Profiles

### 3.1 User Directory
- [x] Page: `frontend/platform/admin/users.html`
- [x] Table columns: Avatar, Display Name, Email, Roles (investor/developer/admin badges from `user_roles`), KYC Status (from `kyc_records`), Cash Balance (from `wallets` WHERE type='cash'), Rewards Balance (from `wallets` WHERE type='rewards'), Account Status, Tier Name (from `user_tiers` JOIN `tiers`), Joined Date.
- [x] **Advanced Search:** By Email, Name, UUID, Phone Number. Built-in support for syntax prefixes (e.g., `email:jane@doe.com` or `id:123`).
- [x] **Filters:** Role, KYC Status, Account Status, Tier, Has investments (Y/N), Registration date range, Balance range (High/Zero balance filters).
- [x] **Columns:** Added "Last Active" timestamp to identify inactive vs. power users.
- [x] **Bulk Actions:** Export CSV, Bulk suspend, Bulk notify, Bulk verify/approve KYC status.

### 3.2 User Detail View (Tabbed)
- [x] Page: `frontend/platform/admin/user-details.html?id={user_id}`

**Tab: Overview**
- Personal info from `user_profiles` (first_name, last_name, date_of_birth, nationality, full address, phone_number, tax_id).
- Account meta from `users` (email, email_verified, avatar_url, status, created_at, updated_at).
- Roles from `user_roles` JOIN `roles` (with toggle to grant/revoke).
- `user_settings`: language, currency, timezone, totp_enabled (2FA status).
- **Actions:** Change Role, Suspend/Activate, Force Password Reset, Force Logout All Sessions, Impersonate (super_admin only).

**Tab: Sessions & Security**
- Active sessions from `user_sessions` (session_token masked, ip_address, user_agent, remember_me, expires_at, created_at).
- OAuth accounts from `oauth_accounts` (provider, provider_email, created_at). Action: Unlink OAuth.
- Password reset history from `password_reset_tokens` (created_at, expires_at, used_at).
- Action: Revoke specific session, Revoke all sessions.

**Tab: KYC & Compliance**
- Full KYC history from `kyc_records` (provider, provider_ref_id, status, document_type, pep_check_passed, sanctions_check, rejection_reason, verified_at, expires_at).
- Link to external KYC provider dashboard (Didit/Sumsub).
- Action: Override KYC status (mandatory `rejection_reason`, logged to `audit_logs`).

**Tab: Wallets & Transactions**
- All wallets from `wallets` (wallet_type, currency, balance_cents).
- Full `wallet_transactions` history for this user's wallets (type, status, amount_cents, currency, description, external_ref_id, created_at, completed_at).
- Filters: type (deposit/withdrawal/purchase/sale/dividend/reward/refund/fee), status, currency, date range.
- Action: Manual credit/debit wallet (mandatory reason, logged to `audit_logs`).

**Tab: Payment Methods**
- All `payment_methods` (method_type: card/bank, processor_type, brand, last_four, expiry, holder_name, bank_country, routing_number masked, is_default, status).
- Action: Deactivate payment method, Flag as suspicious.

**Tab: Deposit Requests**
- All `deposit_requests` (currency, amount_cents, provider, provider_reference, status, payment_method, expires_at, paid_at, created_at).
- Action: Manually confirm deposit (calls `confirm_deposit` flow), Cancel expired deposit.

**Tab: Investments & Portfolio**
- All `investments` (asset title, tokens_owned, purchase_value_cents, current_value_cents, total_rental_cents, appreciation_pct_bps, status, payout_expected_at).
- `dividend_payouts` for this user (amount_cents, payout_type, status, scheduled_at, paid_at, linked wallet_tx).
- `investment_limits` (annual_limit_cents, invested_12m_cents, available_cents, limit_year). Action: Override annual limit.

**Tab: Orders**
- All `orders` (order_number, total_cents, status, currency, payment_currency, fx_rate, payment_method, payment_ref_id, created_at, completed_at).
- Expandable: `order_items` per order (asset title, tokens_quantity, token_price_cents, subtotal_cents).
- `invoices` linked to orders (invoice_number, subtotal, tax, total, currency, status, pdf_url link).
- Cart: Current `cart_items` (asset, tokens_quantity, token_price_cents).

**Tab: Rewards & Referrals**
- `rewards_balances` (cashback, referrals, promotions, total).
- `user_tiers` JOIN `tiers` (current tier, invested_12m, progress to next tier).
- `referral_codes` (code, generated URL).
- `referral_tracking` WHERE referrer_id = user (referred user, status: pending/qualified/paid, rewards amounts, qualified_at).
- `referral_tracking` WHERE referred_id = user (who referred them).
- Action: Manually credit rewards balance, Adjust tier.

**Tab: Support Tickets**
- All `support_tickets` (subject, status, priority, assigned_to, created_at, updated_at).

**Tab: Notifications**
- All `notifications` for this user (title, type, is_read, action_url, created_at).
- Action: Send direct notification.

**Tab: Audit Trail**
- Filtered `audit_logs` WHERE `actor_user_id = {user_id}` OR `entity_id = {user_id}`.

- [x] Backend: `GET /api/admin/users`, `GET /api/admin/users/{id}`, `GET /api/admin/users/{id}/{tab}`, `PUT /api/admin/users/{id}/status`, `PUT /api/admin/users/{id}/roles`, `POST /api/admin/users/{id}/wallet-adjust`, `POST /api/admin/users/{id}/rewards-adjust`.

---

## 4. Developer Submissions & Project Review Pipeline

- [x] Page: `frontend/platform/admin/developer-submissions.html`
- [x] Table of all `developer_projects` with columns: Project Name, Developer (link to User Detail), Linked Asset, Status (Draft/Submitted/In Review/Approved/Rejected/Live), Total Raised, Investors Count, Funding Progress, Created At.
- [x] **Filter by Status:** Focus queue on `submitted` and `in_review`.
- [x] **Review Detail View** (`frontend/platform/admin/developer-submission-review.html?id={project_id}`):
    - Developer profile card (Name, KYC status, other projects count).
    - Side-by-side view: Submitted asset data from `assets` table (title, description, asset_type, property_type, area, lease details, sizes, bedrooms/bathrooms, construction_status, location data, financial data — total_value, token_price, tokens_total, yield data, commodity-specific fields) vs. internal validation checklist.
    - **Document Data Room:** All `asset_documents` categorized by `document_type`:
        - Legal: `proof_of_title`, `legal_basis`, `building_permit`, `license_nib`, `id_card`
        - Tax: `tax_npwp`, `tax_pbb`, `tax_bphtb`, `owner_npwp`
        - Property: `site_plan`, `floor_plan`, `expose`
        - Financial: `appraisal`, `financial`
        - Inline PDF viewer or download links with `file_size_bytes`.
    - **Image Gallery:** All `asset_images` (sorted by `sort_order`, `is_cover` flagged).
    - **Milestones Review:** All `asset_milestones` (title, description, date, month_index, is_completed).
    - **Compliance Checklist** (manual admin checkboxes):
        - [x] Developer KYC is Approved.
        - [x] All required legal documents uploaded.
        - [x] All required tax documents uploaded.
        - [x] Financial projections reviewed and reasonable.
        - [x] SPV/Legal entity confirmed.
        - [x] Token price × total tokens = total_value_cents.
        - [x] Location data verified.
        - [x] Video tour reviewed (if `video_url` provided).
        - [x] Google Maps URL confirmed (if `google_maps_url` provided).
    - **Actions:** Approve (sets `developer_projects.status = 'approved'`, sets `assets.published = true`, `assets.funding_status = 'funding_open'`), Reject (with comments, creates notification for developer), Request Revision (notification to developer).
- [x] Backend: `GET /api/admin/developer-projects`, `GET /api/admin/developer-projects/{id}`, `POST /api/admin/developer-projects/{id}/review`.

---

## 5. Asset Management (Live Assets Lifecycle)

- [x] Page: `frontend/platform/admin/assets.html`
- [x] Table of all `assets` with columns: Title, Type, Funding Status, Total Value, Token Price, Tokens Available/Total, % Funded, Featured, Published, Developer (link), Created At.
- [x] **Filters:** Asset Type (real_estate, commercial_property, commodity, business, startup, land_plot), Funding Status, Featured, Published.
- [x] **Quick Controls:**
    - [ ] **Inline Featured Toggle:** Quick-switch in table to promote/demote assets instantly.
    - [ ] **Marketplace Preview:** "View on Site" button to quickly check the public-facing listing.
    - [ ] **Visual Progress:** CSS progress bars directly in the table cells for funding status.
    - [ ] **Performance Export:** Button to export per-asset stats (investor count, average ticket).

### 5.1 Asset Detail View (Tabbed)
- [x] Page: `frontend/platform/admin/asset-details.html?id={asset_id}`
**Tab: Overview**
- All core fields from `assets` table grouped by category:
**Tab: Overview & Full Asset Editor**
- [ ] **Full Edit Mode:** A master "Edit Asset" toggle that transforms the read-only overview into a comprehensive form, allowing admins to modify *every single field* of a live asset securely.
- All core fields from `assets` table grouped by editable categories:
    - Basic: title, slug, short_description, description, asset_type, property_type.
    - Property: area, lease_type, lease_term_years, land_size_sqm, building_size_sqm, bedrooms, bathrooms, construction_status, year_built.
    - Location: city, country, address, lat/lng, location_description, google_maps_url, video_url.
    - Financial: total_value_cents, token_price_cents, tokens_total, tokens_available.
    - Financial: total_value_cents, token_price_cents, tokens_total, tokens_available. *(Note: modifications to financials of live assets must trigger an audit warning and optional notifications to current token holders)*
    - Yield: annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps.
    - Commodity: operator_name, term_months, fixed_roi_bps, revenue range, expenses, net profit range, payout, splits.
    - Status: funding_status, featured, published, funding_start_at, funding_end_at.
- Toggle: Featured, Published.
- Funding status dropdown transition.
- **Audit Trail:** Any edits made via this full asset editor must be rigidly logged to `audit_logs` tracking `previous_state` and `new_state`.
**Tab: Media**
- `asset_images` gallery (reorder, set cover, add/remove).
- Video URL display/edit.
**Tab: Documents (Data Room)**
- All `asset_documents` with type labels and sizes.
**Tab: Financials (Monthly Performance)**
- `asset_financials` table (period_month, period_year, rental_income_cents, appreciation_cents, occupancy_rate_bps, expenses_cents, net_income_cents).
- **Admin input form:** Add/edit monthly financial records. This drives the portfolio view for all investors holding this asset.
**Tab: Milestones / Roadmap**
- `asset_milestones` — mark completed, add new, edit.
**Tab: Cap Table (Ownership)**
- `investments` JOIN `users` for this asset — investor name, tokens_owned, purchase_value, current_value, total_rental_cents, status.
- Summary: Total tokens distributed vs. available.
**Tab: Dividends**
- All `dividend_payouts` for this asset.
- **Trigger Payout Tool:** Calculate and distribute based on `asset_financials.net_income_cents` × each investor's fractional share. Creates `dividend_payouts` + `wallet_transactions`. Requires 2FA re-auth.
**Tab: Orders**
- All `order_items` JOIN `orders` referencing this asset.
**Tab: Invoices**
- All `invoices` linked to orders containing this asset.
**Tab: Danger Zone**
- Suspend secondary trading / unpublish from marketplace.
- Force-refund (reverses `investments`, credits `wallets`, updates `orders.status`, `assets.tokens_available`).
- Mark as `exited` (final lifecycle).
- [ ] Backend: `GET /api/admin/assets`, `GET /api/admin/assets/{id}`, `PUT /api/admin/assets/{id}`, `GET /api/admin/assets/{id}/cap-table`, `POST /api/admin/assets/{id}/distribute-dividend`, `POST /api/admin/assets/{id}/refund`.

---

## 6. Deposit Request Management

> ⚠️ **Missing from the previous workflow.** The `deposit_requests` table (migration 005) is the foundation of the multi-currency bank deposit flow. Users create deposit intents, and the system waits for PSP confirmation. THIS IS A MAJOR ADMIN FUNCTION.

- [x] Page: `frontend/platform/admin/deposits.html`
- [x] Table of all `deposit_requests`: User (link), Currency (USD/IDR), Amount, Provider (stripe/xendit/manual), Provider Reference, Status (pending/paid/expired/failed/cancelled), Payment Method, Expires At, Paid At, Created At.
- [x] **Priority Queue:** Highlighted rows for `status = 'pending'` deposits nearing `expires_at`.
- [x] **Filters:** Status, Currency, Provider, Date Range, Amount Range.
- [x] **Actions:**
    - **Manually Confirm:** For "manual" provider deposits (wire transfers) — admin clicks "Confirm" which calls `confirm_deposit()` flow atomically (credits wallet, logs transaction, creates audit log).
    - [ ] **Proof of Payment Viewer:** Interface to view uploaded payment slips/receipts for manual transfers.
    - [ ] **KYC Visibility:** Show user's current KYC status badge next to their name in the queue.
    - [ ] **Reference Copy:** One-click copy for `provider_reference` to speed up bank reconciliation.
    - [ ] **Accounting Export:** Specialized CSV formatted for easy reconciliation with accounting tools.
    - **Mark as Failed/Cancelled:** With reason.
    - **Extend Expiry:** Push `expires_at` forward (e.g., for slow international wires).
- [x] Backend: `GET /api/admin/deposit-requests`, `POST /api/admin/deposit-requests/{id}/confirm`, `PUT /api/admin/deposit-requests/{id}/status`.

---

## 7. Orders & Investment Management

- [x] Page: `frontend/platform/admin/orders.html`
- [x] Table of all `orders`: Order Number (ORD-*), User (link), Total, Currency, Payment Currency, FX Rate, Status, Payment Method, Payment Ref, Created At, Completed At.
- [x] **Filters:** Status (pending/processing/completed/failed/cancelled/refunded), Payment Method, Currency, Date Range.
- [x] **Order Detail View:**
    - All `order_items` (Asset title/link, Tokens, Price, Subtotal).
    - Linked `invoices` (invoice_number, amounts, pdf_url download).
    - Actions: Cancel, Issue refund (reverses wallet deduction + investment).
- [x] **Global Investments Table:** All `investments` searchable by User or Asset.
- [x] Backend: `GET /api/admin/orders`, `GET /api/admin/orders/{id}`, `PUT /api/admin/orders/{id}/status`, `GET /api/admin/investments`.

---

## 8. Financial & Treasury Management

- [x] Page: `frontend/platform/admin/treasury.html`

### 8.1 Platform Ledger
- [x] Global view of ALL `wallet_transactions` across the platform.
- [x] Columns: User (link), Wallet Type (cash/rewards), Currency, Tx Type (deposit/withdrawal/purchase/sale/dividend/reward/refund/fee), Status, Amount (cents → formatted), Description, External Ref, Linked Order, Created At.
- [x] Filters: Type, Status, Currency, Date Range, Amount Range.

### 8.2 Platform Wallet Aggregates
- [x] Total cash held across all user `wallets` (grouped by currency: USD, IDR, EUR, etc.).
- [x] Total rewards held (sum of `rewards_balances` cashback + referrals + promotions).
- [x] Platform fee revenue (sum `wallet_transactions` WHERE type = 'fee').
- [x] Comparison against PSP (Mangopay/Stripe) reported balance — discrepancy highlighting.

### 8.3 Reconciliation Tool
- [x] Compare `wallet_transactions` (external_ref_id) vs. actual PSP records.
- [x] Highlight: local tx without PSP match, PSP tx without local match.
- [x] Action: Create manual adjustment (logged to `audit_logs`).

### 8.4 Withdrawal Approvals
- [x] Queue of `wallet_transactions` WHERE type = 'withdrawal' AND status = 'pending'.
- [x] Configurable threshold: withdrawals above X require manual approval.
- [x] Display user KYC status alongside withdrawal request.
- [x] Actions: Approve (triggers PSP payout), Reject (with reason, credits back), Flag for investigation.

### 8.5 Invoice Management
- [x] Table of all `invoices` (invoice_number, order link, user link, subtotal, tax, total, currency, company_entity, status, pdf_url, issued_at).
- [x] Actions: Void invoice, Reissue invoice, Download PDF.

- [x] Backend: `GET /api/admin/treasury/ledger`, `GET /api/admin/treasury/balances`, `GET /api/admin/treasury/reconciliation`, `GET /api/admin/treasury/withdrawals/pending`, `POST /api/admin/treasury/withdrawals/{id}/approve`, `GET /api/admin/invoices`.

---

## 9. KYC & AML Operations (Compliance Desk)

- [x] Page: `frontend/platform/admin/kyc.html`
- [x] **Pending Queue:** All `kyc_records` WHERE status IN ('pending', 'in_review').
- [x] **Review Interface:**
    - User profile summary.
    - Side-by-side: Self-declared data (`user_profiles`: name, DOB, nationality, address) vs. provider-extracted ID data.
    - `pep_check_passed` (Politically Exposed Person flag).
    - `sanctions_check` flag.
    - `document_type` (passport/id_card/drivers_license).
    - `provider_ref_id` — clickable link to the provider's dashboard (Didit/Sumsub).
    - Risk Score (fetched via Provider API).
- [x] **Actions:** Approve (sets status='approved', verified_at=NOW()), Reject (with `rejection_reason`, sends notification), Request Docs (notification), Escalate.
- [x] **Expiry Management:** `kyc_records` nearing `expires_at` — bulk reminder notifications.
- [x] **AML Stats:** Approved/Rejected/Pending/Expired counts, avg review time, PEP hits, sanctions hits.
- [x] Backend: `GET /api/admin/kyc/queue`, `GET /api/admin/kyc/{user_id}`, `POST /api/admin/kyc/{user_id}/review`, `GET /api/admin/kyc/stats`.

---

## 10. Rewards & Referral Management

> ⚠️ **Entirely missing from the previous workflow.** The rewards system has 5 database tables (`tiers`, `user_tiers`, `rewards_balances`, `referral_codes`, `referral_tracking`) and a full API layer.

- [ ] Page: `frontend/platform/admin/rewards.html`

### 10.1 Tier Configuration
- [ ] View/edit `tiers` table (name, min_invest, max_invest, cashback_pct, badge_color, sort_order).
- [ ] Add new tiers, modify thresholds, change cashback percentages.
- [ ] Preview: "If we change Plus threshold from $10K to $5K, how many users would upgrade?"

### 10.2 User Tier Overview
- [ ] Table of all `user_tiers` JOIN `tiers` JOIN `users`.
- [ ] Columns: User (link), Current Tier, Badge Color, Invested 12m, Progress to Next Tier.
- [ ] **Filters:** By tier, by investment range.
- [ ] Action: Manually override tier (e.g., VIP promotion).

### 10.3 Rewards Balances
- [ ] Table of all `rewards_balances` JOIN `users`.
- [ ] Columns: User, Cashback, Referrals, Promotions, Total.
- [ ] Action: Manual credit/debit (for promotions, corrections).
- [ ] Aggregate: Total platform-wide rewards liability.

### 10.4 Referral Program
- [ ] **Referral Codes:** Table of `referral_codes` (user, code, URL, created_at).
- [ ] **Referral Tracking:** Table of `referral_tracking` (referrer → referred user, status: pending/qualified/paid, reward amounts, created_at, qualified_at).
- [ ] **Filters:** Status (pending referrals awaiting qualification, qualified awaiting payout, paid).
- [ ] Action: Manually qualify referral, Mark as paid, Flag as fraudulent (e.g., self-referral detection).
- [ ] **Stats:** Total referrals, Conversion rate (pending → qualified), Total rewards paid out.

- [ ] Backend: `GET /api/admin/rewards/tiers`, `PUT /api/admin/rewards/tiers/{id}`, `GET /api/admin/rewards/balances`, `POST /api/admin/rewards/balances/{user_id}/adjust`, `GET /api/admin/rewards/referrals`, `PUT /api/admin/rewards/referrals/{id}/status`.

---

## 11. Support Ticket Management

- [x] Page: `frontend/platform/admin/support.html`
- [x] Table of all `support_tickets`: Subject, User (link), Status (open/in_progress/resolved/closed), Priority (low/normal/high/urgent), Assigned To (admin user), Created At, Last Updated.
- [x] **Filters:** Status, Priority, Assigned To, Date Range.
- [x] **Production-Ready List Enhancements:**
    - [x] **Server-Side Scalability:** Move pagination, filtering, and sorting entirely to the server-side (currently client-side fetches all tickets).
    - [x] **Bulk Actions:** Ability to select multiple tickets to change status, priority, or reassign instantly.
    - [x] **Loading States:** Implement skeleton loaders instead of plain text "Loading..." for a premium feel.
    - [x] **Time Date Filters:** Filter tickets by creation date (e.g., "Last 7 days").
- [x] **Ticket Detail View (`frontend/platform/admin/support-ticket.html`):**
    - Threaded conversation (Admin vs. Customer vs. Internal Notes).
    - User profile summary sidebar with quick links to KYC, Wallet, Investments.
    - Actions: Change Status, Change Priority, Assign To Admin.
- [x] **Production-Ready Detail Enhancements:**
    - [x] **Attachment Handler:** Support file uploads (screenshots, PDFs) for both agents and users.
    - [x] **Rich-Text Reply Editor:** Upgrade the basic textarea to allow formatting (bold, links, lists).
    - [x] **Canned Responses (Macros):** Quick-insert dropdown for common answers (e.g., "KYC Verification Steps").
    - [x] **Ticket Audit Log:** Show a timeline of state changes (e.g., "Status changed to Resolved by Admin") interweaved in the thread.
    - [x] **Optimized Assignees:** Ensure the assignee dropdown handles a large admin list gracefully (e.g. search/paginated).
    - [x] **Assignment & Routing / Merge:** Auto-assign by category, and ability to merge duplicate tickets.
- [ ] **Metrics:** Open tickets by priority, Average resolution time, Agent performance stats.
- [ ] Backend: `GET /api/admin/support` (Needs limits/offsets/filters), `GET /api/admin/support/{id}`, `POST /api/admin/support/{id}/messages`, `PATCH /api/admin/support/{id}`.

---

## 11A. User-Facing Support Page (`/support`)

> ⚠️ **Previously missing.** The admin workflow only covered the admin-side ticket management (§11). This section defines the **user-facing** `/support` page — how users submit tickets, track their history, access self-service resources, and contact support. This requires a new `support_ticket_replies` table and user-facing API endpoints.

### 11A.1 Database Migration (New Table: `support_ticket_replies`)

- [x] Create migration `007_support_ticket_replies.sql`:

```sql
-- support_ticket_replies — threaded conversation on a support ticket
CREATE TABLE support_ticket_replies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id       UUID NOT NULL REFERENCES support_tickets(id) ON DELETE CASCADE,
    author_id       UUID NOT NULL REFERENCES users(id),
    author_role     VARCHAR(20) NOT NULL DEFAULT 'user'
                    CHECK (author_role IN ('user', 'admin')),
    message         TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_ticket_replies_ticket ON support_ticket_replies(ticket_id);
CREATE INDEX idx_ticket_replies_author ON support_ticket_replies(author_id);
```

- [x] Also add a `category` column to `support_tickets`:

```sql
ALTER TABLE support_tickets
ADD COLUMN category VARCHAR(50) DEFAULT 'general'
CHECK (category IN (
    'general', 'account', 'deposits', 'withdrawals',
    'investments', 'kyc', 'technical', 'billing', 'other'
));
```

### 11A.2 Support Page Layout (`frontend/platform/support.html`)
- [x] Page: `frontend/platform/support.html` (already exists — refactored to dynamic).
- [x] **Page Structure:**
    1. **Hero Section:** "How can we help?" heading with support search bar (searches FAQ below).
    2. **Quick Category Cards:** 6 clickable category cards linking to filtered FAQ or pre-filled ticket form:
        - 💰 Deposits & Withdrawals
        - 📊 Investments & Portfolio
        - 🔐 Account & Security
        - 📋 KYC & Verification
        - 💳 Payment Methods
        - 🛠️ Technical Issues
    3. **FAQ / Knowledge Base Section** (Accordion)
    4. **My Tickets Section** (Expandable)
    5. **Contact / Submit Ticket Section**
    6. **Live Chat Widget (Placeholder)**

### 11A.3 FAQ / Knowledge Base (Accordion)
- [x] **Grouped by category.** Each category has 5–10 questions/answers.
- [x] Categories:
    - **Getting Started:** How to sign up, how to verify identity (KYC), how to navigate the platform.
    - **Deposits & Withdrawals:** How to deposit (card, bank transfer), deposit processing times per currency (USD vs IDR), how to withdraw, withdrawal limits and fees, multi-currency support.
    - **Investments:** How fractional tokens work, how to buy tokens, how to view portfolio, how dividends are calculated and paid, what happens when an asset is fully funded.
    - **Account & Security:** How to change password, how to enable 2FA (TOTP), how to change email/phone, what to do if account is locked, how OAuth login (Google/Facebook) works.
    - **KYC & Verification:** What documents are needed, how long KYC takes, what if KYC is rejected, PEP/Sanctions checks explanation.
    - **Rewards & Referrals:** How referral codes work, how cashback tiers work, how to redeem rewards balance.
    - **Developer / Asset Submission:** How to list a property, what documents are required for submission, review timeline, how funding works.
- [x] FAQ data can be loaded from a static JSON file (`/static/data/faq.json`) or served via API.
- [x] **Search:** Client-side search filters FAQ items as user types.

### 11A.4 My Support Tickets (Ticket History)
- [x] **Visible only to logged-in users** (use existing session auth).
- [x] Table/List of user's own `support_tickets` ordered by `updated_at DESC`.
- [x] Columns/Fields: Subject, Category, Status (badge: open=blue, in_progress=amber, resolved=green, closed=grey), Priority, Created At, Last Reply.
- [x] **Expandable Ticket Detail** (click to expand or navigate to detail view):
    - Original message.
    - Full conversation thread from `support_ticket_replies` (user messages vs. admin messages visually distinguished — left/right aligned like a chat, or color-coded).
    - **Reply Form:** Text area + submit button to add a reply (creates `support_ticket_replies` row with `author_role='user'`). ✅
    - **Reopen Button:** If ticket is `resolved` or `closed`, user can reopen (sets status back to `open`). ✅
- [x] **Empty State:** "You haven't submitted any tickets yet. Need help? Submit a request below."

### 11A.5 Submit a Ticket (New Ticket Form)
- [x] **Form Fields:**
    - **Category** (dropdown): General, Account, Deposits, Withdrawals, Investments, KYC, Technical, Billing, Other.
    - **Subject** (text input, required, max 255 chars).
    - **Priority** (dropdown, default "Normal"): Low, Normal, High, Urgent.
    - **Message** (textarea, required, rich-text or plain depending on implementation).
    - **Attachments** (optional, future — file upload for screenshots).
- [x] **On Submit:**
    - `POST /api/support/tickets` → creates `support_tickets` row with `user_id` from session.
    - Shows success toast: "Your ticket has been submitted. We'll respond within 24 hours."
    - Redirects to "My Tickets" section with new ticket visible.
    - Creates a `notifications` row for admin users (type='system', title='New support ticket from {user.email}').
- [x] **Validation:**
    - Subject: required, 5–255 characters.
    - Message: required, 20–5000 characters.
    - Rate limiting: max 5 tickets per user per 24h (prevents abuse).

### 11A.6 Contact Information & Support Hours
- [x] **Contact Card Section** at bottom of page:
    - 📧 Email: support@poool.finance
    - 💬 WhatsApp: Link to Jonas's WhatsApp (if applicable)
    - 🕐 Support Hours: Mon–Fri, 9:00–18:00 CET (with "Online" / "Offline" badge based on current time).
    - 📍 Company Address: POOOL registered entity address.
- [x] **Response Time Expectations:**
    - Normal: Within 24 hours.
    - High: Within 12 hours.
    - Urgent: Within 4 hours.

### 11A.7 Live Chat Widget (Phase 2 / Placeholder)
- [x] **For now:** Display a "Chat to support" button that opens the ticket submission form (same as the developer sidebar's "Chat to support" button).
- [x] **Future Integration:** Slot for Intercom, Crisp, or custom WebSocket-based live chat.
- [x] **Online/Offline indicator:** Based on support hours (§11A.6).

### 11A.8 Backend API Routes (User-Facing)

| Method | Route | Description | Auth |
|--------|-------|-------------|------|
| `GET` | `/api/support/tickets` | List current user's tickets (paginated, sorted by updated_at DESC) | Session |
| `GET` | `/api/support/tickets/{id}` | Get ticket detail + all replies (only if `user_id` matches session) | Session |
| `POST` | `/api/support/tickets` | Create new ticket (subject, category, priority, message) | Session |
| `POST` | `/api/support/tickets/{id}/reply` | Add user reply to ticket (creates `support_ticket_replies` row) | Session |
| `PUT` | `/api/support/tickets/{id}/reopen` | Reopen a resolved/closed ticket (sets status→open) | Session |
| `GET` | `/api/support/faq` | Return FAQ data (optional — could be static JSON instead) | Public |

- [x] **All routes** enforce that users can only see/modify their own tickets (`WHERE user_id = session.user_id`).
- [x] **POST /api/support/tickets** should also log to `audit_logs` (action: `support_ticket.created`).
- [x] **Rate limiting:** 5 ticket creations per user per 24h, 20 replies per user per hour.

### 11A.9 Notification Integration
- [x] **On ticket creation:** Admin notification (visible in admin notification bell, §1.2).
- [x] **On admin reply:** User notification (`notifications` row with type='system', action_url='/support#ticket-{id}').
- [x] **On ticket status change:** User notification ("Your ticket '{subject}' has been resolved").
- [x] **Email notification (via Postmark):** When admin replies, send email to user with reply preview + link to ticket.

### 11A.10 Mobile Responsiveness
- [x] FAQ accordion fully responsive (touch-friendly expand/collapse).
- [x] Ticket list scrollable on mobile with card layout instead of table.
- [x] Ticket form full-width on mobile.
- [x] "Chat to support" as a floating action button (FAB) on mobile.

---

## 12. Notifications Center (Admin → Users)

- [ ] Page: `frontend/platform/admin/notifications.html`
- [ ] **Send to Individual:** Target user by email/UUID, compose title, message, type (kyc/investment/payout/system/promo), optional action_url. Creates `notifications` row.
- [ ] **Broadcast:** Send to ALL users or filtered by role/tier. Bulk `notifications` insert.
- [ ] **Email Campaigns:** Draft and send via Postmark API for critical communications.
- [ ] **Notification Log:** View all sent notifications with delivery status.
- [ ] Backend: `POST /api/admin/notifications/send`, `POST /api/admin/notifications/broadcast`, `GET /api/admin/notifications/log`.

---

## 13. Audit Logs (Immutable Compliance Record)

- [ ] Page: `frontend/platform/admin/audit-logs.html`
- [ ] **Read-only view** of `audit_logs` table (BIGSERIAL id — never UUID, auto-increment, immutable).
- [ ] Columns: ID, Actor (user link), Action (e.g., `deposit.confirmed`, `kyc.approved`, `user.created`), Entity Type, Entity ID (clickable link), Previous State (expandable JSON), New State (expandable JSON), IP Address, User Agent, Timestamp.
- [ ] **Search & Filters:** Actor ID, action pattern (`deposit.*`, `kyc.*`), Entity Type, Entity ID, Date Range, IP Address.
- [ ] **Payload Search:** Ability to search within the JSON "State Change" payloads for specific values.
- [ ] **Traceability & Deep-links:**
    - [ ] Clickable "Actor" (linked to Admin/User Profile).
    - [ ] Clickable "Entity ID" (linked to Asset/User/Order details).
- [ ] **Export:** Download filtered audit log as CSV for external compliance review.
- [ ] Backend: `GET /api/admin/audit-logs`.

---

## 14. Reporting & Regulatory Exports

- [ ] Page: `frontend/platform/admin/reports.html`
- [ ] **Pre-built Reports:**
    - Monthly Financial Summary (sum deposits/withdrawals/investments/dividends/fees from `wallet_transactions` + `deposit_requests`).
    - KYC Status Report (counts by status from `kyc_records`, avg approval time).
    - Asset Performance (per-asset: funding %, yield, occupancy from `assets` + `asset_financials`).
    - User Growth (signups from `users.created_at`, active users from `user_sessions`).
    - AML/Compliance (flagged users, PEP hits, sanctions hits from `kyc_records`).
    - Rewards Liability (total outstanding from `rewards_balances`).
    - Referral Program Effectiveness (conversion rates from `referral_tracking`).
    - Multi-Currency Exposure (wallet balances by currency from `wallets`).
    - Invoice Summary (issued/void from `invoices`).
- [ ] Custom date range selector.
- [ ] **Export:** CSV, PDF (for BaFin / tax authorities).
- [ ] **Scheduled Reports:** Auto-generate and email weekly/monthly summaries.
- [ ] Backend: `GET /api/admin/reports/{report_type}`, `POST /api/admin/reports/schedule`.

---

## 15. System Health & Operations

- [x] Page: `frontend/platform/admin/system.html`

### 15.1 Background Jobs Monitor
- [x] View `background_jobs` table (from MASTERPLAN) — Job Name, Status (pending/processing/completed/failed), Attempts, Payload preview, run_at, created_at.
- [x] Actions: Retry, Cancel, View full payload.

### 15.2 Webhook Logs
- [x] Log viewer for: Mangopay/Stripe payment confirmations (hitting `POST /api/webhooks/payments`), KYC Provider status changes, Xendit IDR deposits.
- [x] Columns: Provider, Endpoint, HTTP Status, Payload preview, Processed (Y/N), Timestamp.
- [x] Action: Replay webhook (re-process the payload — calls `confirm_deposit` etc.).

### 15.3 Idempotency Keys
- [x] View `idempotency_keys` table (from MASTERPLAN) — Key, User, Request Path, Response Status, Created At.
- [x] Useful for debugging duplicate payments.

### 15.4 Active Sessions Monitor
- [x] Global view of `user_sessions` — all active sessions across the platform.
- [x] Columns: User (link), IP Address, User Agent, Remember Me, Expires At, Created At.
- [x] Action: Bulk revoke by IP pattern or user agent.

### 15.5 Password Reset Token Monitor
- [x] View `password_reset_tokens` — detect abuse patterns (multiple resets from same IP, rapid succession).

- [x] Backend: `GET /api/admin/system/jobs`, `POST /api/admin/system/jobs/{id}/retry`, `GET /api/admin/system/webhooks`, `GET /api/admin/system/sessions`, `GET /api/admin/system/password-resets`.

---

## 16. Platform Settings & Configuration

- [x] Page: `frontend/platform/admin/settings.html`

### 16.1 Financial Configuration
- [x] Platform deposit fee (%).
- [x] Platform withdrawal fee (flat or %).
- [x] Min/Max deposit and withdrawal amounts per currency (USD, IDR).
- [x] Large withdrawal threshold for manual approval.
- [x] Secondary market trading fee (when implemented).
- [x] FX rate source configuration (currently mock 15,500 IDR/USD — switch to live API).

### 16.2 Investment Configuration
- [x] Default annual investment limit per user (currently 25,000,000 cents = $250K from `investment_limits`).
- [x] Min/Max tokens per purchase.
- [x] Max tokens per user per asset.

### 16.3 Rewards Configuration
- [x] Referral bonus amounts (currently $30/$30 each from `referral_tracking` default).
- [x] Cashback percentages per tier (editable via §10.1).
- [x] Promotion budget limits.

### 16.4 API Integration Health
- [x] Health panels for: Stripe, Xendit/Midtrans, Mangopay, KYC Provider (Didit/Sumsub), Postmark, Sentry, Redis.
- [x] Last successful ping + error count (24h).

### 16.5 Feature Flags
- [x] Disable New Registrations (maintenance).
- [x] Pause Deposits / Withdrawals.
- [x] Pause Marketplace investing.
- [x] Enable/Disable Secondary Market.
- [x] Maintenance Banner text.

### 16.6 Legal Content Versioning
- [x] Terms & Conditions version tracker.
- [x] Privacy Policy / Currency Policy / Cookies Policy version tracker.
- [x] (References existing pages: `/terms`, `/privacy-policy`, `/currency-policy`, `/cookies`).

- [x] Backend: `GET /api/admin/settings`, `PUT /api/admin/settings`.

---

## 17. Advanced Admin Permissions & Zero Trust (RBAC)

> **FinTech Standard:** Admin access in a regulated platform must adhere to the principle of least privilege, incorporating immutable audit trails, and mandatory multi-party approvals for critical financial operations.

- [x] **Dynamic Roles Engine (`roles` table):**
    - Move beyond hardcoded roles to a dynamic permission mapping system.
    - Base templates:
        - **`super_admin`**: Full system access, but *cannot* approve their own financial multi-sig requests.
        - **`compliance_officer`**: Full read/write on KYC and Compliance tabs. View-only access on users and transactions. Masked PII when outside active review workflow.
        - **`support_agent`**: Scope limited to tickets and basic user profiles. PII masked. No financial rights.
        - **`finance_admin`**: Treasury read, draft payouts, issue invoices. *Cannot* approve payouts (requires Four-Eyes).
        - **`auditor_read_only`**: View-only access across the entire platform. Cannot modify any state. Useful for external regulators/auditors.
- [x] **Granular Permissions (Backend Junction Table):**
    - Implement a `role_permissions` junction mapping roles to specific actions (e.g., `feature:action` -> `kyc.approve`, `financials.payout.draft`, `financials.payout.approve`).
- [x] **Four-Eyes Principle (Multi-Party Authorization):**
    - Critical actions (e.g., executing a smart contract payload, distributing >$10k dividends, overriding locked accounts) require *two* authorized admins to approve.
    - State machine: `Draft` -> `Pending Approval` -> `Executed` / `Rejected`.
- [x] **Session & Context Awareness:**
    - Sidebar and dashboard components dynamically rebuild based on JWT `permissions` claims.
    - Admin sessions have a strict 2-hour TTL and 15-minute idle timeout.
- [x] Every API endpoint checks granular permissions via `AdminPermissionGuard` middleware.
- [x] Backend: Migration adding permission tables, `GET /api/admin/roles`, `PUT /api/admin/users/{id}/admin-role`.

---

## 18. Route Protection & Security

- [x] `AdminGuard` middleware in `backend/src/auth/`:
    - Verify session contains admin role.
    - Check granular RBAC per endpoint.
    - Log every admin API access to `audit_logs`.
- [x] Apply to all `/api/admin/*` routes.
- [x] **Re-Authentication** for critical actions: dividend payouts, user suspension, wallet adjustments, fee changes, deposit confirmations, large withdrawal approvals.
- [x] **Admin Session TTL:** 2 hours (vs. 24h for regular users).
- [x] **IP Allowlisting (optional):** Restrict admin panel to known IPs.

---

## 19. Testing & Validation

### Seed Data
- [x] Create test admin users with each RBAC role (super_admin, compliance_officer, support_agent, finance_admin).
- [x] Seed test data: 10+ users with varied KYC states, 5+ assets in different funding states, deposits in USD + IDR, referral chains.

### Authorization Tests
- [x] Investor accessing `/api/admin/*` → `403`.
- [x] `support_agent` accessing `/api/admin/treasury/*` → `403`.
- [x] `compliance_officer` accessing `/api/admin/settings` → `403`.
- [x] `finance_admin` accessing `/api/admin/kyc/*` → `403`.

### Functional E2E Tests
- [x] **Full Investment Flow:** Developer submits asset → Admin reviews → Approves → Asset appears on marketplace → User deposits → User adds to cart → Checkout → Order + Investment created → Admin sees in cap table.
- [x] **Deposit Confirmation Flow:** User creates deposit request → Admin manually confirms → Wallet credited → Transaction logged → Audit entry created.
- [x] **Dividend Distribution:** Admin inputs monthly financials → Triggers payout → All token holders receive proportional wallet credit → `dividend_payouts` records created.
- [x] **KYC Override:** Admin changes KYC status → Audit log entry created with previous_state/new_state.
- [x] **Referral Flow:** User A generates code → User B signs up with code → `referral_tracking` created → Admin qualifies → Rewards credited.
- [x] **User Suspension:** Admin suspends user → User cannot log in → Active sessions are revoked.

### UI/Performance Tests
- [x] All tables paginate correctly with 1000+ rows.
- [x] Global search returns within 500ms.
- [x] HTMX polling updates dashboard metrics without full page reload.
- [x] All monetary values display correctly (cents → formatted currency with correct symbol: $, Rp).

---

## 20. Email Verification Flow (`/auth/verify-email`)

> ⚠️ **Previously missing.** Every new signup triggers email verification. The page and backend routes exist but were not documented in the workflow.

### 20.1 User-Facing Page (`frontend/platform/verify-email.html`)
- [x] **Existing page** — already implemented with resend functionality.
- [x] **Page Content:**
    - POOOL logo.
    - Mail icon (envelope SVG in red circle).
    - "Check your email" heading.
    - "We sent a verification link to your email address. Please click the link to verify your account." message.
    - "Resend verification email" button (HTMX `POST /auth/resend-verification`).
    - "Back to log in" link → `/auth/login`.
- [x] **Flow:**
    1. User signs up → redirected to `/auth/verify-email`.
    2. User clicks link in email → `email_verified` set to `true` in `users` table.
    3. If link expired, user clicks "Resend" → new verification email sent.
    4. After verification → user can log in and access protected pages.

### 20.2 Backend Routes
| Method | Route | Description |
|--------|-------|-------------|
| `GET` | `/auth/verify-email` | Serve the verify-email page (public) |
| `POST` | `/auth/resend-verification` | Resend verification email (HTMX, returns HTML fragment) |

### 20.3 Admin Oversight
- [x] **Dashboard KPI:** Count of unverified users (`users` WHERE `email_verified = false`).
- [x] **User Detail → Overview tab:** Show `email_verified` status with badge (✅ Verified / ❌ Unverified).
- [x] **Action:** Force-verify email (admin override, logged to `audit_logs`).
- [x] **Alert:** Users who signed up > 7 days ago and still unverified → flag in admin notifications.

### 20.4 Testing
- [x] Signup → verify-email page displayed.
- [x] "Resend" button triggers new email without error.
- [x] Verification link sets `email_verified = true`.
- [x] Expired token shows error + prompts resend.
- [x] Admin can see verification status per user.

---

## 21. Payment Flow Pages

> ⚠️ **Previously missing.** Two critical pages in the post-checkout flow that users see during and after payment.

### 21.1 Payment Success Page (`/payment-success`)
- [x] Page: `frontend/platform/payment-success.html` (exists — fully dynamic).
- [x] **Triggered when:** Checkout completes successfully (wallet deducted, order created, investments recorded).
- [x] **Page Content:**
    - ✅ Success icon / checkmark animation (confetti optional).
    - **Order Summary Card:** Fetches `/api/orders/latest` — order number, items, total, payment method.
    - **CTAs:** "View your portfolio", "Download Invoice", "Continue browsing".
- [x] **Mobile:** Full-width card layout, prominent "View Portfolio" button.

### 21.2 Payment In Progress Page (`/payment-in-progress`)
- [x] Page: `frontend/platform/payment-in-progress.html` (exists — fully dynamic).
- [x] **Triggered when:** Bank deposit or wire transfer initiated but not yet confirmed.
- [x] **Page Content:**
    - ⏳ Animated spinner / progress indicator.
    - **Deposit details card:** Amount, currency, provider, reference, status badge, expected time.
    - **Bank Transfer Instructions** (if provider = 'manual'): Recipient bank, account number, SWIFT, reference.
    - **Auto-polling** (`payment-in-progress.js`): Polls `/api/deposits/{id}/status` every 10 seconds.
        - On `status = 'paid'` → auto-redirect to `/payment-success`.
        - On `status = 'failed'` or `status = 'expired'` → show error card.
    - **Timeout:** After 30 minutes (180 polls), shows "Still processing" message.
    - "Back to wallet" and "Contact Support" CTAs.
- [x] **Mobile:** Full-width, large spinner, scrollable bank transfer instructions.

### 21.3 Backend API Additions
| Method | Route | Description | Auth |
|--------|-------|-------------|------|
| `GET` | `/api/orders/{id}` | Return order details + items + invoice for success page | Session (own orders only) |
| `GET` | `/api/deposits/{id}/status` | Return deposit request status for polling | Session (own deposits only) |

### 21.4 Admin Oversight
- [x] Both pages are **read-only** for users — no admin actions needed on the pages themselves.
- [x] Admin monitors via §6 (Deposit Request Management) and §7 (Orders & Investment Management).
- [x] **Alert:** Deposits stuck in `pending` for > 48 hours → admin notification.

### 21.5 Testing
- [x] Successful checkout → redirect to `/payment-success` with correct order details.
- [x] Invoice download link works and returns PDF.
- [x] Bank deposit initiated → redirect to `/payment-in-progress`.
- [x] HTMX polling updates status without page reload.
- [x] Status change to `paid` → auto-redirect to success page.
- [x] Status change to `failed` → error message displayed.
- [x] Mobile layout responsive for both pages.

---

## 22. Tier Detail Page (`/tier`)

> ⚠️ **Previously missing.** The tier page exists as a fully designed static page (2952 lines) with 5 tier levels. Needs dynamic data binding and workflow documentation.

### 22.1 Page Structure (`frontend/platform/tier.html`)
- [x] Page: `frontend/platform/tier.html` (exists — needs dynamic data from API).
- [x] **Breadcrumbs:** Rewards → Tier.
- [x] **Layout:** Two-column (sidebar + main content).

### 22.2 Left Column: Current Tier Progress Card
- [x] **Current tier badge** with brand styling (Intro = light, Plus = blue, Pro = dark, Elite = gradient, Premium = gold).
- [x] **Investment amount** in last 12 months (from `user_tiers.invested_12m` or `investment_limits.invested_12m_cents`).
- [x] **Progress bar** showing percentage toward next tier.
- [x] **Hint text:** "Invest **USD X** to reach {next_tier}".
- [x] **Data source:** `GET /api/rewards/tiers` + `GET /api/rewards` (user's current tier from `user_tiers` JOIN `tiers`).

### 22.3 Right Column: Tier Stepper (Roadmap)
- [x] Vertical stepper with 5 tiers (from `tiers` table, sorted by `sort_order`):
    - **INTRO** (active for new users): USD 5 referral bonus, no processing fees.
    - **PLUS** (USD 4,000): USD 50 referral, RentReinvest access.
    - **PRO** (USD 10,000): USD 100 referral, 1% cashback USD / 0.5% cashback KSA.
    - **ELITE** (USD 30,000): USD 150 referral, 2% cashback IDR / 0.75% KSA, early access.
    - **PREMIUM** (USD 100,000): USD 200 referral, 3% cashback IDR / 1% referral cashback, dedicated rep, Bloomberg, private dinners.
- [x] **Visual states:**
    - ✅ Active tier: filled icon with inner dot, no lock.
    - 🔒 Locked tiers: lock icon with "USD X to unlock" text.
    - Previously unlocked tiers: checkmark icon.
- [x] Benefits displayed per tier with cumulative note: "All the benefits you've unlocked so far."

### 22.4 Dynamic Data Binding
- [x] Fetch current tier + progress from `GET /api/rewards`.
- [x] Fetch all tier definitions from `GET /api/rewards/tiers`.
- [x] Update progress card dynamically (JavaScript):
    - Set current invested amount.
    - Calculate progress bar width.
    - Set hint text with remaining amount to next tier.
    - Highlight correct stepper item as active.
- [x] **Currency formatting:** Display amounts in user's preferred currency (from `user_settings.currency`, with FX conversion if needed).

### 22.5 Admin Oversight (via §10)
- [x] Tier thresholds are editable by admin in §10.1 (Tier Configuration).
- [x] Admin can manually override a user's tier in §10.2 (User Tier Overview).
- [x] Changes to tier definitions should immediately reflect on the `/tier` page for all users.

### 22.6 Testing
- [x] New user sees Intro tier with 0% progress.
- [x] User with $5,000 invested sees Plus tier active, Pro as next target.
- [x] Progress bar accurately reflects investment amount vs. next tier threshold.
- [x] All 5 tiers render with correct benefits text.
- [x] Mobile responsive: stepper stacks vertically, progress card full-width.
- [x] Admin changes to tier thresholds reflect immediately on user page.

---

## 23. Core API & Utility Endpoints

> ⚠️ **Previously missing.** These foundational API endpoints power the entire platform but were not documented.

### 23.1 User Identity API (`GET /api/me`)
- [x] **Critical endpoint** — called by `user-data.js` on every page load to populate user name, avatar, email across all pages.
- [x] **Returns:**
    - `id`, `email`, `email_verified`, `avatar_url`, `status`, `created_at`.
    - `first_name`, `last_name`, `display_name` (from `user_profiles`).
    - `phone_number`, `city`, `country`, `postal_code`, `address_line_1` (from `user_profiles`).
    - `roles` (from `user_roles` JOIN `roles`).
    - `language`, `currency`, `timezone`, `totp_enabled` (from `user_settings`).
- [x] **Auth:** Session cookie required. Returns 401 if not authenticated.
- [x] **Used by:** Every protected page (marketplace, wallet, portfolio, settings, rewards, etc.).

### 23.2 User-Facing List APIs
| Method | Route | Description |
|--------|-------|-------------|
| `GET` | `/api/wallets` | List user's wallets (cash + rewards) with balances |
| `GET` | `/api/deposits` | List user's deposit requests (for wallet page) |
| `GET` | `/api/invoices` | List user's invoices (for orders/transactions page) |
| `GET` | `/api/cart` | Get current cart items with asset details |

### 23.3 Profile vs Settings Clarification
- [x] **`/profile`** route and handler exist in `main.rs` but no `profile.html` file exists — this appears to be a legacy/unused route.
- [x] **`/settings`** is the active user profile management page (separate `settings-page` workflow).
- [x] **Action:** Consider removing `/profile` route or redirecting to `/settings`.

---

## 24. Legal & Policy Pages

> ⚠️ **Previously only mentioned in §16.6** (admin versioning). This section defines the **user-facing behavior** of all legal pages.

### 24.1 Pages Overview
| Page | Route | File | Content Type |
|------|-------|------|-------------|
| Terms and Conditions | `/terms` | `frontend/platform/terms.html` | Static HTML (hardcoded) |
| Privacy Policy | `/privacy-policy` | `frontend/platform/privacy-policy.html` | Static HTML (hardcoded) |
| Currency Policy | `/currency-policy` | `frontend/platform/currency-policy.html` | Static HTML (hardcoded) |
| Cookie Policy | `/cookies` | `frontend/platform/cookies.html` | Static HTML (hardcoded) |

### 24.2 Current Implementation
- [x] All 4 pages are **full platform pages** with sidebar, mobile header, burger menu, and profile switcher.
- [x] Content is **hardcoded in HTML** — not served from database or CMS.
- [x] Pages are **protected** (require login via `serve_protected` in `main.rs`).
- [x] "Last updated" date is displayed at top of each legal page.

### 24.3 Required Enhancements
- [x] **Add "Last Updated" date** visible at top of each legal page (e.g., "Last updated: March 1, 2026").
- [x] **Table of Contents:** Anchor links for major sections (auto-generated via `legal-enhancements.js`).
- [x] **Print-friendly styling** (CSS `@media print` in `legal-enhancements.js`).
- [x] **"Back to top" floating button** for long pages.
- [x] **Links from signup:** "By signing up, you agree to our [Terms](/terms) and [Privacy Policy](/privacy-policy)" — present on signup.html.
- [x] **Links from footer:** All platform pages have footer links to legal pages via `legal-enhancements.js` `injectPlatformFooter()`.

### 24.4 Cookie Consent Banner (GDPR / ePrivacy)
- [x] **Cookie consent banner** shown on first visit (before login), stored in localStorage.
- [x] **Categories:**
    - Essential (always on — session cookies).
    - Analytics (Google Analytics — opt-in).
    - Marketing (future — opt-in).
- [x] **Accept All / Essential Only** buttons.
- [x] **Consent stored:** In `localStorage` with timestamp.
- [x] **Link:** "Learn more" → `/cookies` policy page.

### 24.5 Terms Acceptance on Signup
- [x] **Checkbox on signup form:** "I agree to the [Terms and Conditions](/terms) and [Privacy Policy](/privacy-policy)".
- [x] **Required** — signup cannot proceed without checkbox checked (server-side validation + inline error).
- [x] **Record acceptance:** Immutable `user_consents` table records `terms_version`, `accepted_at`, `ip_address`, `user_agent`.
- [x] **Re-acceptance flow:** If admin updates Terms version, users see a sticky banner: "Our Terms have been updated. [Review Changes](/terms)" — powered by `GET /api/user/legal-status`.

### 24.6 Admin Oversight (via §16.6)
- [x] Admin can update legal document version in Admin Settings → Legal & Compliance tab.
- [x] When version changes, all users who haven't accepted the new version will see a re-acceptance prompt.
- [x] Audit log entry: `legal_document.updated` with old_version/new_version recorded in `audit_logs`.

### 24.7 Future: Database-Driven Content
- [x] **Phase 2:** Move legal content to database (new `legal_documents` table).
- [x] Admin can edit legal page content via rich-text editor in Admin Settings.
- [x] Version history tracked with diff view.
- [x] This avoids requiring a code deployment for legal text updates.

---

## 25. KYC User-Facing Page (`/kyc`)

> ⚠️ **Admin-side KYC management is covered in §9.** This section documents the **user-facing** KYC verification experience.

### 25.1 Page Structure (`frontend/platform/kyc.html`)
- [x] Page: `frontend/platform/kyc.html` (exists — ~1352 lines, includes full platform navigation).
- [x] **Purpose:** Users submit their identity verification (KYC) documents.

### 25.2 KYC Submission Steps
- [x] **Step 1: Personal Information**
    - Full legal name (first + last name).
    - Date of birth.
    - Nationality / Country of citizenship.
    - Tax residency country.
    - Address (line 1, line 2, city, state/province, postal code, country).
- [x] **Step 2: Identity Document Upload**
    - Document type selector: Passport, National ID, Driving License.
    - Front photo / scan upload (required).
    - Back photo / scan upload (if applicable).
    - File types: JPEG, PNG, PDF. Max 5MB per file.
- [x] **Step 3: Selfie Verification**
    - Selfie with document (hold ID next to face).
    - Camera capture or file upload.
- [x] **Step 4: PEP/Sanctions Declaration**
    - "Are you a Politically Exposed Person (PEP)?" — Yes/No.
    - "Have you or an immediate family member held a senior public position?" — Yes/No.
    - Source of funds declaration (employment, business, inheritance, etc.).

### 25.3 KYC Status Flow
| Status | Badge Color | User Message |
|--------|-------------|-------------|
| `not_started` | Grey | "Complete identity verification to invest" |
| `pending` | Amber | "Your verification is being reviewed" |
| `approved` | Green | "Your identity has been verified ✅" |
| `rejected` | Red | "Verification failed — please resubmit" + rejection reason |
| `expired` | Grey | "Your verification has expired — please reverify" |

### 25.4 Backend Integration
- [x] **Submit KYC:** `POST /api/kyc/submit` → Creates `kyc_records` row with status `pending`.
- [ ] **Upload documents:** Files uploaded to cloud storage (S3/GCS), URLs stored in `kyc_records`.
- [x] **Check status:** `GET /api/kyc/status` → Returns current KYC status for logged-in user.
- [x] **Resubmit:** If rejected, user can submit new documents (creates new `kyc_records` row).

### 25.5 KYC Banner Integration
- [x] **KYC Banner** appears on all platform pages (marketplace, portfolio, cart, etc.) when KYC is not `approved`.
- [x] Banner text: "You have to [complete identity verification (KYC)](/kyc) to buy or get property tokens. It takes 2 mins."
- [x] **CTA:** "Complete KYC" button → `/kyc`, "Learn more" button.
- [x] Banner disappears once KYC is approved.

### 25.6 Admin Oversight (via §9)
- [x] Submitted KYC appears in admin queue (§9).
- [x] Admin reviews documents, approves or rejects with reason.
- [x] Status changes trigger user notification and email.

---

## 26. Checkout Page (`/checkout`)

> ⚠️ **The checkout route and handler exist but the page UX was not fully documented.**

### 26.1 Page Structure (`frontend/platform/checkout.html`)
- [x] Page: `frontend/platform/checkout.html` (exists — minimal 78-line page, needs enhancement).
- [x] **Current state:** Basic form with currency selector, bank details, and proof-of-transfer upload.

### 26.2 Page Content
- [x] **Order Summary** (populated from cart):
    - List of items: asset title, token qty, price per token, line total.
    - Subtotal, fees (if any), total.
    - Currency badge (USD / IDR).
- [x] **Payment Method Selection:**
    - **Wallet balance:** If sufficient balance, one-click "Pay with Wallet" button.
    - **Bank transfer:** Show bank details for selected currency.
        - USD: Chase Bank, Account Name: POOOL Inc., Account Number: 123456789, Routing: 987654321.
        - IDR: BCA, Account Name: PT POOOL Indonesia, Account Number: 0987654321.
    - **Card payment:** If Stripe integration is active (from payment-methods).
- [x] **Proof of Transfer Upload** (for bank transfer):
    - File input accepting images and PDF.
    - Required before submit.
- [x] **Submit:** `POST /checkout` → atomic checkout (§Investment & Checkout in User Action Map).

### 26.3 Post-Checkout Flow
```
/cart → /checkout → POST /checkout → /payment-success (wallet)
                                    → /payment-in-progress (bank transfer)
```

### 26.4 Enhancement Roadmap
- [x] Add proper platform navigation (currently a standalone page without sidebar/header).
- [x] Add order summary populated from `GET /api/cart`.
- [x] Integrate with saved payment methods (cards/banks from `payment_methods` table).
- [x] Show wallet balance with option to pay directly from wallet.
- [x] Add coupon/promo code field (future).
- [x] Add "Terms acceptance" checkbox (link to `/terms`).

---

## 27. Missing Admin Pages Checklist

> **Tracking:** Admin pages referenced in the workflow that still need to be created as HTML files and/or backend routes.

| Page | Workflow Section | HTML File | Backend Route | Status |
|------|-----------------|-----------|---------------|--------|
| `admin/developer-submission-review.html` | §4 | ✅ Created | ✅ Route exists | **Complete** — review pipeline with approve/reject |
| `admin/asset-details.html` | §5 | ✅ Created | ✅ Route exists | **Complete** — 8-tab detail view (Overview, Media, Docs, Financials, Milestones, Cap Table, Orders, Settings) |
| `admin/system.html` | §15 | ✅ Complete | ✅ Route exists | **Complete** — DB stats, background jobs, webhooks, sessions, password resets |
| `admin/admins.html` | §29 | ✅ Complete | ✅ Route exists | **Complete** — KPIs, 2FA coverage, lifecycle, sessions, invites |
| `admin/roles.html` | §29 | ✅ Complete | ✅ Route exists | **Complete** — RBAC matrix, SoD detection, role creation |
| `admin/rewards.html` | §10 | ✅ Complete | ✅ Route exists | **Complete** — tier config, balances, referrals |
| `admin/support.html` | §11 | ✅ Complete | ✅ Route exists | **Complete** — ticket list, filters |
| `admin/audit-logs.html` | §13 | ✅ Complete | ✅ Route exists | **Complete** — read-only log viewer |
| `admin/reports.html` | §14 | ✅ Complete | ✅ Route exists + dedicated reports API | **Complete** — 14 report types, CSV/JSON export, date-filtered `/api/admin/reports/:type` |
| `admin/notifications.html` | §12 | ✅ Complete | ✅ Route exists | **Complete** — send + broadcast |
| `admin/settings.html` | §16 | ✅ Complete | ✅ Route exists | **Complete** — financial, investment, rewards config |
| `admin/email-marketing.html` | §28 | ✅ Complete | ✅ Route exists | **Complete** — templates, campaigns, analytics |

### Priority Order for Admin Page Implementation
1. **`admin/rewards.html`** — Rewards & tier management (§10).
2. **`admin/support.html`** — Support ticket management (§11).
3. **`admin/audit-logs.html`** — Compliance requirement (§13).
4. **`admin/reports.html`** — Revenue/AUM reporting (§14).
5. **`admin/notifications.html`** — User communication (§12).
6. **`admin/admins.html`** — Admin Role & Permission Management (§29).
7. **`admin/settings.html`** — Platform configuration (§16).
8. **`admin/developer-submission-review.html`** — Deep-dive review page (§4).
9. **`admin/asset-details.html`** — Asset detail management (§5).
10. **`admin/system.html`** — System monitoring (§15) — lowest priority.
11. **`admin/email-marketing.html`** — Email Engine & Marketing Hub (§28).
12. **Tax & Fiscal Reporting** — Calculate and issue investor P&L statements (§31).
13. **Risk & Dispute Hub** — Chargeback mitigation and fraud monitoring (§32).
14. **Asset News & Roadmap** — Engagement and transparency updates (§34).
15. **Secondary Market Controls** — Liquidity management and kill switches (§33).
16. **Asset Document Expiry** — Lifecycle compliance alerts (§35).

---

## 28. Email Engine & Marketing Hub

> ⚠️ **New Addition.** A comprehensive, industry-standard FinTech email system, handling compliance, deliverability, and targeted marketing campaigns.

### 28.1 Database Migrations
- [x] Create `email_templates` table: ID, Subject, HTML_Template, Text_Template, Version, Description.
- [x] Create `email_logs` table: ID (BIGSERIAL), User_ID, Template_ID, Subject, Status (Sent, Delivered, Opened, Clicked, Bounced), Sent_At.

### 28.2 Admin Interface: Dashboard & Analytics (`frontend/platform/admin/email-marketing.html`)
- [x] **Analytics View:** High-level metrics (Delivery Rate, Open Rate, Click Rate, Bounce Rate).
- [ ] **Delivery Timeline (Chart):** Visualization of email volume over the last 30 days.
- [x] **Recent Failures:** Quick view of bounced or failed emails for immediate intervention.

### 28.3 Admin Interface: Template Editor
- [x] **Template List:** Overview of all system emails with version control.
- [x] **Interactive Editor:** HTML editor with live desktop/mobile preview.
- [x] **Dynamic Variables:** Inject placeholders like `{{first_name}}`, `{{asset_title}}`.
- [ ] **Test Sending:** "Send preview to me" button.

### 28.4 Admin Interface: Newsletter & Marketing Campaign Creator
- [x] **Audience Segmentation:** Filter recipients by Tier ("Plus" only), KYC status, investment activity, or portfolio bracket.
- [x] **Dynamic Content Blocks:** Conditional sections showing/hiding based on user profile.
- [ ] **A/B Testing:** Test subject lines and buttons to optimize engagement.
- [ ] **Scheduling & IP Warm-up:** Ensure large sends don't trip spam filters by staggering delivery.

### 28.5 FinTech Email Automations & Drips
- [x] **Onboarding Drip:** Welcome -> KYC Prompt -> First Deposit Guide -> First Investment.
- [x] **Abandonment Flows:** "You left an asset in your cart" / "Deposit Request expired".
- [x] **Win-back / Re-engagement:** Reach out to dormant users.
- [x] **Milestone Celebrations:** Upgrade to new tier, first dividend received.
- [x] **Event-Based Triggers:** "Asset is 90% funded".

### 28.6 Transactional Event Map (Source of Truth)
- [x] **Account & Security:** Welcome, Verify Email, Password Reset, 2FA setup, New Login (Device).
- [x] **Compliance:** KYC Approved, KYC Rejected (reason).
- [x] **Financial details:** Deposit Confirmed, Withdrawal Processed, Dividend Payout, Monthly Portfolio Statement.
- [x] **Investments:** Order Confirmation, Invoice Available, Asset Fully Funded.

### 28.7 Compliance, Security & Deliverability (10/10 Requirements)
- [x] **Granular Unsubscribe (RFC 8058):** Users opt-out of marketing but *must* retain transactional alerts.
- [x] **Immutable Audit Logging:** Proof of delivery for regulatory updates (T&Cs).
- [x] **BIMI & VMC Integration:** Display POOOL logo securely in user inboxes to prevent phishing.
- [x] **GDPR:** Tie email logs into global deletion/anonymization workflows.
- [x] **Hard Bounce & Rate Limits:** Auto-suspend bouncing emails and rate-limit sensitive endpoints to prevent abuse.

---

## 29. Enterprise Access Management (The Internal Control Plane)

> ⚠️ **New Addition / Enhanced.** A SOC2/ISO27001-compliant management interface for controlling internal access. Designed natively with Zero Trust principles.

### 29.1 Admin Directory & Lifecycle Management
- [x] Page: `frontend/platform/admin/admins.html`
- [x] **Complete Identity Lifecycle:**
    - Table of all internal users. Columns: Admin Name, Corporate Email, Assigned Roles, Security Health (2FA status, last password change), Status (Active/Suspended), Last Active IP, Created At.
- [x] **Actions:**
    - **Create/Invite Admin:** Must use enforced SSO (e.g., Google Workspace/Okta) or a complex secure invite link.
    - **Just-In-Time (JIT) Access (Future-proofing):** Allow temporary role escalation (e.g., support agent needs 4 hours of read-write to debug an issue).
    - **Suspend Admin (Kill Switch):** Immediate invalidation of all active JWTs, Session IDs, and WebSocket connections via Redis.
    - **Force Credential Rotation:** Require the admin to reset 2FA/Passwords on next login.

### 29.2 Role & Permission Matrix (Access Control GUI)
- [x] Page: `frontend/platform/admin/roles.html`
- [x] Visual matrix mapping default and custom roles against a unified permission dictionary.
- [x] **Strict Segregation of Duties (SoD):** The UI flags conflicts if an admin attempts to combine roles that violate SoD (e.g., the same role cannot `Draft` and `Approve` financial distributions).
- [x] **Permission Hierarchy (Resource-Action-Condition):**
    - **Resource:** `users`, `kyc`, `treasury`, `assets`, `audit_logs`
    - **Action:** `read`, `write`, `delete`, `approve`
    - **Condition:** (Optional) `own_region_only`, `under_10k_value`

### 29.3 Time-Based Access & Geo-Fencing (Security Posture)
- [ ] **Location Awareness:** Flag or block admin logins originating from IPs outside the corporate VPN or known allowed geographic regions.
- [ ] **Working Hours Policy:** (Optional Config) Restrict non-Super Admin roles to typical operational hours based on their assigned region. Attempted logins outside these hours trigger a high-severity alert to SecOps.

### 29.4 Dashboard Page Visibility Mapping (Resource Isolation)
- [x] Configure which sidebar links, dashboard widgets, and user-detail tabs are visible per role.
- [x] **Data Masking / PII Protection:** If an admin lacks `pii.view` (e.g., basic support), render `***-**-1234` instead of full SSN/Tax ID, and `martin.***@example.com`.
- [x] Logic relies on centralized `PAGE_PERMISSION_MAP` shared securely across the backend context.

### 29.5 Admin Invitation & Onboarding Security Flow
- [x] `POST /api/admin/invite` → Sends invite.
- [ ] **Hardware Key Support (FIDO2/WebAuthn):** Mandate U2F (Yubikey, Apple TouchID) for `super_admin` and `finance_admin` roles to protect against phishing.
- [ ] Initial login forces acceptance of internal security policies.

### 29.6 Zero Trust Backend Authorization (ABAC/RBAC)
- [x] `AdminPermissionGuard` middleware: Upgraded to check *Permissions*, not just *Roles*.
- [ ] **Step-up Authentication (2FA Challenge):** Critical API endpoints (`POST /api/admin/treasury/withdrawals/{id}/approve`) require an immediate TOTP or WebAuthn signature injected into the request header.
- [ ] **Immutable Audit Log Integration:** Every permission check success AND failure is logged. High-velocity failures (e.g., 5 'Forbidden' hits in 1 minute by an admin) auto-suspend the admin and alert security ops via webhook.

---

## 30. KYC Integration Strategy (Didit.me)

> **Regulated FinTech Requirement:** Identity verification is the gatekeeper of the platform. We must maintain a provider-agnostic core to allow for regional compliance shifts or commercial renegotiations. This architecture ensures we can integrate with [Didit](https://docs.didit.me/).

### 30.1 Generic KYC Abstraction Layer
- [x] **Provider Trait (`backend/src/kyc/provider.rs`):**
    - `fn create_session(user_id: Uuid) -> Result<String, AppError>`: Returns URL/Token.
    - `fn process_webhook(payload: Value) -> Result<KycStatusUpdate, AppError>`.
    - `fn get_result(external_id: &str) -> Result<KycExtractedData, AppError>`.
- [x] **Provider Configuration:** Runtime detection via env vars (`DIDIT_API_KEY` etc.) in `service::build_provider()`.
- [x] **Admin Control:** Admin Settings JS detects active provider, displays status, and provides connection test.

### 30.2 Phase 1: Didit Implementation
- [x] **API Client:** `DiditProvider` implementing `KycProvider` trait in `backend/src/kyc/didit.rs`.
- [x] **Secure Webhook:** `POST /api/webhooks/kyc/didit` with HMAC-SHA256 signature validation.
- [x] **Frontend Hand-off:** `kyc-page.js` handles redirect flow for Didit and manual fallback.
- [x] **Credentials configured:** API key, workflow ID, webhook secret all set in `.env`.

- [x] **Standardized DB Schema:** `kyc_records` stores `provider_ref_id` and `provider` columns.

---

## 31. Automated Tax Reporting & Fiscal Compliance (Regulatory Reporting)

> **Regulated FinTech Requirement:** As a platform distributing investment income, POOOL is responsible for providing users with clear tax documentation (Profit & Loss, Capital Gains) to ensure global compliance.

- [ ] **Logic:** Implement a `tax_reporting` service that calculates annual income, capital gains, and withholding tax (if applicable) for each user.
- [ ] **Admin Interface:**
    - **User Detail → Tax Tab:** View and regenerate a user's tax statements (PDF).
    - **Global Settings:** Configure tax rates per country and jurisdiction.
- [ ] **Scheduled Task:** Auto-generate annual P&L statements every January for the preceding fiscal year.

## 32. Chargeback, Dispute & Fraud Mitigation (Risk Control)

> **Financial Safety:** Credit card chargebacks or disputed bank transfers pose a systemic risk. We need a rapid-response workflow to protect platform equity.

- [ ] **Fraud Signal Hub:** Monitor Stripe/Xendit/PSP webhooks for any `dispute.created` or `payment.failed` signals.
- [ ] **Auto-Freeze Logic:** When a high-value dispute is detected, automatically flag the user account and freeze pending withdrawals (requires admin manual override for "unfreeze").
- [ ] **Admin Workflow:**
    - **Dispute Queue:** Table of all active disputes with direct links to upload evidence (e.g., invoices, KYC docs, signed terms) to the PSP.
    - **Evidence Builder:** Tool to auto-bundle a user's investment history and signed contracts into a single ZIP for dispute resolution.

## 33. Secondary Market Controls & Maintenance (Trading Desk)

> **Platform Resilience:** Once secondary trading (P2P asset swaps) is enabled, the platform needs tools to halt trading during market volatility or legal updates.

- [ ] **Kill Switch Logic:** Global or per-asset toggle to `disable_secondary_trading`.
- [ ] **Price Floor/Ceiling:** Admin settings to restrict trading within a % range of the last appraised NAV (Net Asset Value) to prevent wash trading.
- [ ] **Audit Trail:** Detailed ledger specifically for secondary market commissions and swap history.

## 34. Dynamic Asset News & Roadmap Updates (Investor Relations)

> **Long-term Engagement:** Investors need transparency throughout the lifecycle of the property or business.

- [ ] **Logic:** Implement an `asset_news_feed` table (asset_id, title, content, image, type, published_at).
- [ ] **Admin Workflow:**
    - **Asset Detail → News Feed Tab:** Post updates (e.g., "Construction Phase 2 Started", "New 5-year Lease Signed", "Maintenance Completed").
    - **Notification Bridge:** Option to "Notify All Token Holders" via Email/Push when a new update is posted.
- [ ] **Impact Monitoring:** Track how updates correlate with secondary market volume or sentiment.

## 35. Asset Documentation Lifecycle & Expiry Alerts (Compliance)

> **Operational Continuity:** Assets have documents that expire (e.g., Insurance, IMB/Permits, Lease Agreements, Business Licenses).

- [ ] **Expiry Tracker:** Add `expires_at` column to `asset_documents` table.
- [ ] **Admin Workflow:**
    - **Dashboard Alert:** Highlight documents expiring within 30/60/90 days in a "Compliance Widget".
    - **Auto-Reminder:** Send notification (Email/Push) to the developer/SPV manager to upload renewed documents 60/30/7 days before expiry.
    - **Compliance Lockdown:** If critical documents (e.g., Fire Safety Permit) expire, the asset status is automatically flagged and funding/trading can be restricted (requires admin manual override).

---

## 36. User Data Privacy & GDPR Compliance Hub (Subject Access Rights)

> **Global Privacy Standards:** As a global platform, POOOL must respect "Right to be Forgotten" and "Right to Data Portability" as mandated by GDPR, CCPA, and other regional laws.

- [ ] **Data Portability (SAR):**
    - **User Detail → Privacy Tab:** "Export All Data" button.
    - **Logic:** Generates a machine-readable JSON/CSV bundle of the user's entire history (Profile, KYCs, Wallets, Orders, Investments, Audit Logs).
    - **Audit Entry:** Log every data export request.
- [ ] **Right to be Forgotten (Anonymization):**
    - **User Detail → Privacy Tab:** "Anonymize Account" button.
    - **Logic:** Irreversibly scrub PII (Email, Name, Phone, Address) from the DB while preserving financial ledgers for tax/AML auditing (replace name with `ANON_USER_{UUID}`).
    - **Security:** Requires "Super Admin" role + Four-Eyes approval.
- [ ] **Privacy Settings Controller:** Toggle marketing consent, analytics tracking, and third-party data sharing.

---

## 37. Automated Transaction Reconciliation (Accounting Sync)

> **Financial Integrity:** The internal ledger (DB) must periodically match external reality (Bank accounts, Stripe, Xendit).

- [ ] **Reconciliation Engine:** High-level dashboard showing "Internal Balance Sum" vs "PSP Reported Balance Sum".
- [ ] **Manual Override / Correction:** Tools to create "Platform Adjustment" entries in the ledger to fix small discrepancies (e.g., rounding errors or bank fees).
- [ ] **Mismatch Alerts:** High-priority notification if a user's wallet doesn't match the sum of their transactions.
- [ ] **Accounting Export:** Specialized daily/weekly JSON feed for Xero/QuickBooks API integration.

---

## 38. Localization & Content Management (CMS) Desk

> **Global Reach:** Manage platform terminology, legal text, and marketing copy across multiple languages without code deployments.

- [ ] **Localization Matrix:** View and edit translation keys for the frontend platforms (Web/Mobile).
- [ ] **Dynamic Legal Text:** Move hardcoded `/terms` and `/privacy` content to the DB (version-controlled).
- [ ] **Banner Manager:** Create "Global Announcement" banners (e.g., "Scheduled Maintenance tomorrow at 2AM UTC") with targeting by region or language.
- [ ] **Image Asset CDN:** Manage and swap marketing images, property icons, and hero assets directly from the admin panel.

---

## 39. Internal CRM & Relationship Management (Admin Notes)

> **Operational Memory:** Admins need to share context about "High Net Worth" users or difficult support cases that isn't captured by structured data.

- [ ] **Admin Internal Notes:** Threaded "Staff Only" comments section on every User Detail and Asset Detail page.
- [ ] **Interaction Logging:** Record phone calls, external emails, or high-value meetings.
- [ ] **User Flags:** Custom status labels (e.g., "⚠️ Frequent Disputer", "💎 VIP Investor", "🏢 Developer Partner").
- [ ] **Relationship Owner:** Assign specific "Account Managers" to high-tier (Elite/Premium) users.

---

## 40. Advanced Monitoring & Observability Integration

> **Platform Stability:** Beyond "System Health," the admin needs visibility into application performance and cloud infrastructure costs.

- [ ] **Operational Monitoring:**
    - **Sentry Integration:** Display recent error rates directly in the "System" tab.
    - **API Performance:** 95th/99th percentile latency metrics for core API endpoints.
- [ ] **Infrastructure Health Check:**
    - Proactive alerts for DB storage limits, Redis memory usage, and SSL certificate expiration.
- [ ] **Cloud Costs Console:** (Optional) High-level month-to-date spend on GCP/AWS, Stripe fees, and Didit.me verification costs to monitor margins.

---

## 41. AML Behavior Monitoring & SAR Filing (FinTech Compliance)

> **Industry Standard:** Compliance shifts from "Static KYC" to "Dynamic AML." Monitoring *how* money moves is the ultimate defense against financial crime.

- [ ] **Heuristic Flagging Engine:**
    - Detect "Smurfing" (multiple small deposits just under tracking thresholds).
    - Detect rapid "Pass-through" (deposit followed by immediate withdrawal/transfer).
    - Detect "High Velocity" accounts (unusually frequent trading/transfers for the assigned tier).
- [ ] **Admin AML Hub:**
    - **Risk Queue:** A specialized table of users flagged by the engine, requiring manual human investigation.
    - **Case Management:** Create a "Compliance Case," attach notes, and mark as "Investigating," "Cleared," or "Suspicious."
- [ ] **SAR Filing Helper:** A "Generate SAR Data" button that bundles a suspicious user's ID docs, full transaction ledger, and login history into a format compatible with FinCEN/BaFin reporting requirements.

---

## 42. Developer/Sponsor Portal (B2B Administration)

> **Operational Scale:** As POOOL grows, property developers (Sponsors) need limited access to manage *their* specific assets without occupying internal staff time.

- [ ] **Sponsor Dashboard (Read-Only/Limited):**
    - A specialized login for external partners.
    - View cap tables of their properties only.
    - View monthly rental income and occupancy stats.
- [ ] **Submission Pipeline:**
    - Sponsors upload new listing details directly for admin review (§4).
    - Sponsors upload monthly performance documents (§34).
- [ ] **Admin Oversight:** Internal admins act as "Approvers" for everything a Sponsor submits.

---

## 43. External Auditor Access & Regulatory Evidence Bundle

> **Trust & Verification:** Regulators (e.g., BaFin) often require periodic audits. Providing a secure, dedicated environment is significantly safer than sharing screens or exporting tons of loose files.

- [ ] **Auditor Role:** A unique role that has `READ_ONLY` access to everything but `BLOCK` on all mutation endpoints and PII (except for specific sampled users).
- [ ] **Evidence Vault:**
    - A dedicated tab to generate "Audit Bundles" for a specific period.
    - Bundle includes: DB snapshot (sanitized), signed Terms history, proof of segregated funds, and immutable audit logs.
- [ ] **Regulatory Reporting (Periodic):** Automated quarterly reports for central banks or financial authorities.

---

## 44. SLA Management & Operational KPI Tracker

> **Customer Excellence:** "Industry standard" is not just about features, but about *speed*. Measuring internal performance is key to scaling.

- [ ] **SLA Trackers:**
    - **KYC Review Time:** Target < 24 hours.
    - **Deposit Confirmation Time:** Target < 2 hours (business hours).
    - **Support Ticket Response:** Target < 4 hours.
- [ ] **Admin Performance Dashboard:**
    - Visualize bottlenecks (e.g., "KYC queue has grown by 50% this week").
    - Performance metrics per admin (anonymized or manager-view) to ensure workload balance.

---

## 45. Disaster Recovery & Platform "Kill Switch" Orchestration

> **Platform Resilience:** In the event of a critical security breach or infrastructure failure, the admin team needs a "Red Button" workflow.

- [ ] **Platform Kill Switch:** Immediate 1-click global halt on all logins, withdrawals, and secondary market trading. Shows a friendly "Under Maintenance" page to users.
- [ ] **Data Integrity Verifier:** A system tool that checks the entire `ledgers` system to ensure `SUM(deposits) - SUM(withdrawals) == SUM(wallets)` across the whole DB.
- [ ] **Recovery Orchestration:**
    - Point-in-time recovery status from DB backups.
    - SSL & DNS health check dashboard.
    - "System Broadcast" to all users via post-incident email/push notification.


