# POOOL Admin Dashboard — Comprehensive Features Audit

> **Last Audited:** 2026-03-11  
> **Total Pages:** 25 HTML pages (20 main + 5 detail/sub-pages)  
> **Total API Endpoints:** 60+ JSON endpoints under `/api/admin/*`  

This document provides a **granular, feature-by-feature audit** of the entire POOOL Admin Platform. Every page, tab, section, button, modal, table, filter, and interactive element is listed with its **current working status**.

### Status Legend

| Icon | Meaning |
|------|---------|
| ✅ | **Working** — Feature loads data and/or performs its action successfully |
| ⚠️ | **Partial** — Feature exists in the UI but has issues (e.g., loads but backend not fully implemented) |
| ❌ | **Not Working** — Button exists but clicking does nothing, or API is missing |
| 🔗 | **Navigation Link** — Clickable link that navigates to another page |
| 📋 | **Static/Display-only** — Shows data but has no interactive action |

---

## 🧭 Global Elements (Present on Every Admin Page)

### Sidebar Navigation
All sidebar links are **clickable and functional** (`admin-sidebar-loader.js` dynamically generates them).

| Nav Item | Target URL | Status |
|----------|-----------|--------|
| Dashboard | `/admin/` | ✅ Working |
| Users | `/admin/users.html` | ✅ Working |
| KYC & AML | `/admin/kyc.html` | ✅ Working |
| Support | `/admin/support.html` | ✅ Working |
| Submissions | `/admin/developer-submissions.html` | ✅ Working |
| Assets | `/admin/assets.html` | ✅ Working |
| Orders | `/admin/orders.html` | ✅ Working |
| Deposits | `/admin/deposits.html` | ✅ Working |
| Treasury | `/admin/treasury.html` | ✅ Working |
| Dividends | `/admin/dividends.html` | ✅ Working |
| Rewards | `/admin/rewards.html` | ✅ Working |
| Admin Directory | `/admin/admins.html` | ✅ Working |
| Roles & RBAC | `/admin/roles.html` | ✅ Working |
| Audit Logs | `/admin/audit-logs.html` | ✅ Working |
| Approval Queue | `/admin/approvals.html` | ✅ Working |
| Notifications | `/admin/notifications.html` | ✅ Working |
| Reports | `/admin/reports.html` | ✅ Working |
| Email Marketing | `/admin/email-marketing.html` | ✅ Working |
| Storage | `/admin/storage.html` | ✅ Working |
| System Health | `/admin/system.html` | ✅ Working |
| Settings | `/admin/settings.html` | ✅ Working |

> **Sidebar badges** for KYC, Deposits, Support show live counts — ✅ Populated from dashboard API.

### Top Bar (Present on All Pages)

| Element | ID / Selector | Type | Status | Notes |
|---------|--------------|------|--------|-------|
| Global Search Input | `#admin-global-search` | Text input | ✅ Working | Calls `/api/admin/users`, `/api/admin/assets`, `/api/admin/orders` simultaneously; shows results dropdown |
| Notification Bell | `.admin-notification-btn` | Button | ✅ Clickable | Navigates to `/admin/notifications.html` — badge shows count "7" (hardcoded) |
| System Health Dots | `#health-db`, `#health-psp`, `#health-kyc`, `#health-email` | Indicator | ✅ Working | Fetches from `/api/admin/system`; shows green/orange/red based on status |
| Date Range Selector | `#dashboard-range` | Select dropdown | ✅ Working | Options: Today, 7d, 30d, 90d, 1y, All Time — **Dashboard page only** |
| Breadcrumb Navigation | `.admin-breadcrumbs` | Links | ✅ Working | Shows Admin > [Page Name], links back to admin home |

---

## 📊 1. Dashboard (`/admin/` or `/admin/index.html`)

**API:** `GET /api/admin/stats/overview?range={today|7d|30d|90d|1y|all}`  
**JS:** `admin-dashboard.js`  
**Data Status:** ✅ Loads real data (48 users, $223.7K AUM, etc.)  
**Auto-refresh:** Every 30 seconds (stats), every 60 seconds (health)

### KPI Cards (8 total)

| KPI Card | Element ID | Data Source | Status |
|----------|-----------|-------------|--------|
| Total Users | `#kpi-total-users` | `total_users` from API | ✅ Populated |
| Assets Under Mgmt | `#kpi-aum` | `aum_cents` from API | ✅ Populated |
| Deposits (24h) | `#kpi-deposits-24h` | `deposits_range_cents` from API | ✅ Populated |
| Pending KYC | `#kpi-pending-kyc` | `pending_kyc` from API | ✅ Populated |
| Live Assets | `#kpi-live-assets` | `live_assets` from API | ✅ Populated |
| Pending Deposits | `#kpi-pending-deposits` | `pending_deposits` from API | ✅ Populated |
| Open Tickets | `#kpi-open-tickets` | `open_tickets` from API | ✅ Populated |
| Rewards Liability | `#kpi-rewards-liability` | `rewards_liability_cents` from API | ✅ Populated |

### Sparklines

| Sparkline | Element ID | Status |
|-----------|-----------|--------|
| User Trend (7-day) | `#trend-users` | ✅ SVG sparkline rendered |
| Deposit Trend (7-day) | `#trend-deposits` | ✅ SVG sparkline rendered |

### Sections

| Section | Status | Details |
|---------|--------|---------|
| **Recent Activity Feed** | ✅ Working | Shows audit log entries (e.g., Checkouts, Logins) with relative timestamps |
| "View All" button → Audit Logs | ✅ 🔗 Navigates to `/admin/audit-logs.html` |
| **Quick Actions** | ✅ Working | Contains "Send Notification" link |
| "Send Notification" link | ✅ 🔗 Navigates to `/admin/notifications.html` |
| **Recent Orders Table** | ✅ Working | Columns: Order #, User, Amount, Status |
| Order # link (e.g., `ORD-xxxx`) | ✅ 🔗 Links to `/admin/orders.html?id=` |
| "View All" button → Orders | ✅ 🔗 Navigates to `/admin/orders.html` |
| **Pending Deposits Table** | ✅ Working | Columns: User, Amount, Provider, Action |
| "Review" button on each deposit | ✅ 🔗 Links to `/admin/deposits.html` |
| "View All" button → Deposits | ✅ 🔗 Navigates to `/admin/deposits.html` |

---

## 👥 2. Users (`/admin/users.html`)

**API:** `GET /api/admin/users`  
**JS:** `admin-users.js`  
**Data Status:** ✅ Loaded 48 users with full data

### Summary KPI Cards (5 total)

| Card | ID | Status |
|------|----|--------|
| Total Users | `#stat-total` | ✅ Populated |
| Investors | `#stat-investors` | ✅ Populated |
| Developers | `#stat-developers` | ✅ Populated |
| KYC Verified | `#stat-verified` | ✅ Populated |
| Suspended | `#stat-suspended` | ✅ Populated |

### Filter Bar

| Filter | ID | Type | Status |
|--------|----|------|--------|
| Search (name/email/ID) | `#user-search-input` | Text input | ✅ Working, client-side filtering |
| Role filter | `#filter-role` | Select (All, Investor, Developer, Admin) | ✅ Working |
| KYC filter | `#filter-kyc` | Select (All, Approved, Pending, Rejected, None) | ✅ Working |
| Status filter | `#filter-status` | Select (All, Active, Suspended, Pending) | ✅ Working |
| Result count | `#user-count-label` | Text | ✅ Updated dynamically |

### Users Table

| Column | Sortable | Status |
|--------|----------|--------|
| ☑ Select All checkbox | — | ✅ Clickable |
| User (avatar + name + email) | `data-sort="name"` | ✅ Sortable |
| Roles | `data-sort="roles"` | ✅ Shows badges (investor, admin, etc.) |
| KYC Status | `data-sort="kyc_status"` | ✅ Color-coded badges |
| Balance | `data-sort="balance_cents"` | ✅ Formatted as USD |
| Status | `data-sort="status"` | ✅ Badge (active/suspended) |
| Joined | `data-sort="created_at"` | ✅ Formatted date |
| Actions | — | ✅ View (🔗 link to user-details) |

### Buttons

| Button | ID | Status | Notes |
|--------|----|--------|-------|
| Export CSV | `#export-users-btn` | ⚠️ Clickable, triggers client-side CSV download | Functional but basic |
| View User → Details | Per-row action | ✅ 🔗 Navigates to `/admin/user-details?id=` |
| Suspend/Activate toggle | Per-row action | ✅ Working | Calls `POST /api/admin/users/:id/status` |
| Previous / Next pagination | `#prev-page`, `#next-page` | ✅ Working | Client-side pagination |

---

## 👤 3. User Details (`/admin/user-details.html?id=`)

**API:** `GET /api/admin/users/:id`  
**JS:** `admin-user-details.js`  
**Data Status:** ✅ Full profile loads with all tabs

### User Header Card

| Element | Status | Notes |
|---------|--------|-------|
| Avatar (initials or image) | ✅ Rendered | Shows initials on colored circle |
| Full Name | ✅ Populated | |
| Email | ✅ Populated | |
| Role Badges | ✅ Shown | Investor, Developer, Admin, Super Admin |
| Cash Balance | ✅ Shown | Formatted as USD |
| Rewards Balance | ✅ Shown | |
| Joined Date | ✅ Shown | |

### Header Action Buttons

| Button | ID | API Call | Status |
|--------|----|----------|--------|
| Edit Roles (pencil icon) | `#btn-edit-roles` | Opens Edit Roles Modal | ✅ Working |
| Adjust Balance (+) | `#btn-edit-balance` | Opens Adjust Balance Modal | ✅ Working |
| Suspend/Activate | `#btn-suspend-trigger` | Opens Confirm Suspension Modal | ✅ Working |
| Freeze Account | `#btn-freeze-trigger` | `POST /api/admin/users/:id/status` | ✅ Working |
| Logout All Sessions | `#btn-logout-all` | `DELETE /api/admin/users/:id/sessions` | ✅ Working |

### Tabs (7 total)

#### Tab 1: Overview

| Section | Status | Details |
|---------|--------|---------|
| **Personal Information** card | ✅ Loaded | First Name, Last Name, DOB, Nationality, Phone, Tax ID, Address |
| "Edit" button | `#btn-edit-profile` | ✅ Opens Edit Profile Modal |
| **Account Settings** card | ✅ Loaded | Email, Status, Email Verified, 2FA (TOTP), Language, Created/Updated |
| **Payment Methods** table | ✅ Loaded | Columns: Type, Details, Status, Default, Added |

#### Tab 2: Wallets & Transactions

| Element | Status | Details |
|---------|--------|---------|
| Wallet KPI cards (Cash, Rewards, etc.) | ✅ Populated | Shows balance per wallet type |
| Transaction History table | ✅ Loaded | Columns: Date, Type, Amount, Status, Description, Reference |
| Transaction Type filter | `#filter-tx-type` | ✅ Working (Deposit, Withdrawal, Purchase, etc.) |
| Transaction Status filter | `#filter-tx-status` | ✅ Working (Completed, Pending, Failed, etc.) |

#### Tab 3: KYC & Compliance

| Element | Status | Details |
|---------|--------|---------|
| KYC Records table | ✅ Loaded | Columns: Provider, Status, Document Type, PEP Check, Sanctions, Verified, Expires, Actions |
| Approve/Reject buttons | Per-row | ✅ Working | Call `/api/admin/kyc/:id/approve` or `reject` |

#### Tab 4: Investments

| Element | Status | Details |
|---------|--------|---------|
| Portfolio table | ✅ Loaded | Columns: Asset, Tokens, Purchase Value, Current Value, Total Rental, Status, Purchased |

#### Tab 5: Orders

| Element | Status | Details |
|---------|--------|---------|
| Order History table | ✅ Loaded | Columns: Order #, Total, Status, Payment Method, Created, Completed |
| Approve/Reject buttons | Per-row (pending only) | ✅ Working | Calls payments API |

#### Tab 6: Sessions & Security

| Element | Status | Details |
|---------|--------|---------|
| Active Sessions table | ✅ Loaded | Columns: IP Address, User Agent, Remember Me, Created, Expires |
| "Revoke All" button | `#btn-revoke-all-sessions` | ✅ Working | `DELETE /api/admin/users/:id/sessions` |
| OAuth Accounts table | ✅ Loaded | Columns: Provider, Email, Linked date |

#### Tab 7: Audit Trail

| Element | Status | Details |
|---------|--------|---------|
| Audit Log table | ✅ Loaded | Columns: ID, Action, Entity, IP Address, Timestamp, Details |

### Modals

| Modal | ID | Buttons Inside | Status |
|-------|----|---------------|--------|
| **Edit Profile** | `#edit-profile-modal` | "Cancel", "Save Changes" | ✅ Working — calls `POST /api/admin/users/:id/profile` |
| **Adjust Balance** | `#edit-balance-modal` | "Cancel", "Adjust Balance" (wallet selector, amount, reason) | ✅ Working — calls `POST /api/admin/users/:id/balance` |
| **Confirm Suspension** | `#suspend-modal` | "Confirm Suspension", "Cancel" | ✅ Working — calls `POST /api/admin/users/:id/status` |
| **Edit Roles** | `#edit-roles-modal` | Checkboxes (Investor, Developer, Admin, Super Admin), "Cancel", "Save Roles" | ✅ Working — calls `POST /api/admin/users/:id/roles` |

---

## 🔒 4. KYC & AML Compliance (`/admin/kyc.html`)

**API:** `GET /api/admin/kyc`  
**JS:** `admin-kyc.js`  
**Data Status:** ✅ Loaded (queue may be empty if no pending KYC)

### KPI Cards (5 total)

| Card | ID | Status |
|------|----|--------|
| Pending Review | `#kyc-pending` | ✅ Populated (amber color) |
| Approved | `#kyc-approved` | ✅ Populated (green) |
| Rejected | `#kyc-rejected` | ✅ Populated (red) |
| PEP Flags | `#kyc-pep` | ✅ Populated |
| Expiring Soon | `#kyc-expiring` | ✅ Populated |

### Tabs (6 total)

| Tab | `data-tab` | Status | Content |
|-----|-----------|--------|---------|
| Review Queue | `queue` | ✅ Active default | Shows pending KYC records |
| Approved | `approved` | ✅ Working | Filters to approved records |
| Rejected | `rejected` | ✅ Working | Filters to rejected records |
| PEP Flags | `pep` | ✅ Working | Shows Politically Exposed Person flags |
| Expiring Soon | `expiring` | ✅ Working | Shows KYC near expiry |
| All Records | `all` | ✅ Working | Full KYC table with search and filters |

### Review Queue Table

| Column | Status |
|--------|--------|
| User (clickable → user details) | ✅ 🔗 |
| Provider | ✅ Shown |
| Document | ✅ Shown |
| PEP Check | ✅ Shown |
| Sanctions | ✅ Shown |
| Submitted | ✅ Shown |
| Actions (Review button) | ✅ Opens KYC Modal |

### All Records Table (with filters)

| Filter | ID | Status |
|--------|----|--------|
| Search by email | `#kyc-search` | ✅ Working |
| Status filter | `#kyc-filter-status` | ✅ Working (Pending, In Review, Approved, Rejected, Expired) |
| Pagination | `#all-prev-page`, `#all-next-page` | ✅ Working |

### KYC Review Modal

| Element | ID | Status |
|---------|-----|--------|
| Modal title | `#kyc-modal-title` | ✅ Populated |
| User info text | `#kyc-modal-text` | ✅ Populated |
| Rejection Reason textarea | `#kyc-rejection-reason` | ✅ Working |
| **Cancel** button | `#kyc-modal-cancel` | ✅ Closes modal |
| **Reject** button | `#kyc-modal-reject` | ✅ Working → `POST /api/admin/kyc/:id/reject` |
| **Approve** button | `#kyc-modal-approve` | ✅ Working → `POST /api/admin/kyc/:id/approve` |

---

## 💰 5. Treasury & Financial Overview (`/admin/treasury.html`)

**API:** `GET /api/admin/treasury`  
**JS:** `admin-treasury.js`  
**Data Status:** ✅ Shows $1,012,983.24 across 36 wallets

### KPI Cards

| Card | Status |
|------|--------|
| Platform Cash Balance | ✅ Populated |
| Total User Wallets | ✅ Populated |
| Avg Wallet Balance | ✅ Populated |
| Rewards Outstanding | ✅ Populated |

### Sections

| Section | Status | Details |
|---------|--------|---------|
| Transaction Volume by Type (chart) | ⚠️ Partial | Chart renders but may use simplified data |
| Dividend Payouts summary | ✅ Shown | |
| Recent Platform Transactions table | ✅ Loaded | Columns: User, Type, Amount, Status, Description, Date |

> ⚠️ **Known Issue:** Some report endpoints return all-time data regardless of date filters.

---

## 💳 6. Deposit Requests (`/admin/deposits.html`)

**API:** `GET /api/admin/deposits`  
**JS:** `admin-deposits.js`  
**Data Status:** ✅ Loaded (0 pending deposits in current data)

### KPI Cards (4 total)

| Card | ID | Status |
|------|----|--------|
| Pending | `#stat-pending` | ✅ Populated |
| Confirmed (24h) | `#stat-confirmed` | ✅ Populated |
| Expired | `#stat-expired` | ✅ Populated |
| Total Volume (30d) | `#stat-volume` | ✅ Populated |

### Tabs

| Tab | `data-tab` | Status |
|-----|-----------|--------|
| Deposit Requests | `requests` | ✅ Active default |
| Risk & Disputes | `disputes` | ✅ Working — loads from `/api/admin/disputes` |

### Deposit Requests Filter Bar

| Filter | ID | Status |
|--------|----|--------|
| Search | `#deposit-search` | ✅ Working |
| Status filter | `#filter-status` | ✅ Working (Pending, Confirmed, Expired, Failed, Cancelled) |
| Currency filter | `#filter-currency` | ✅ Working (USD, IDR) |
| Provider filter | `#filter-provider` | ✅ Working (Stripe, Xendit, Manual/Wire) |

### Deposit Actions (per row, for pending deposits)

| Button | API | Status |
|--------|-----|--------|
| **Confirm** | `POST /api/admin/deposits/:id/confirm` | ✅ Working — Opens confirm modal |
| **Cancel** | `POST /api/admin/deposits/:id/cancel` | ✅ Working |
| **Extend** | `POST /api/admin/deposits/:id/extend` | ✅ Working — extends expiry by 30 days |

### Confirm Deposit Modal

| Element | Status |
|---------|--------|
| Confirmation text | ✅ Populated |
| Notes textarea | ✅ Working (optional) |
| "Cancel" button | ✅ Closes modal |
| "Confirm Deposit" button | ✅ Working → calls API |

### Risk & Disputes Tab

| Column | Status |
|--------|--------|
| User, Provider, Amount, Status, Detected, Evidence | ✅ Rendered |
| "Build Evidence" button | ✅ Working → `POST /api/admin/disputes/:id/evidence` (mock) |
| "Resolve" button | ✅ Working → `PUT /api/admin/disputes/:id/status` |

### Other Buttons

| Button | ID | Status |
|--------|----|--------|
| Refresh | `#btn-refresh` | ✅ Reloads deposit data |
| Pagination (Prev/Next) | `#prev-page`, `#next-page` | ✅ Working |

---

## 📦 7. Orders & Investments (`/admin/orders.html`)

**API:** `GET /api/admin/orders`, `GET /api/admin/investments`  
**JS:** `admin-orders.js`  
**Data Status:** ✅ 64 orders loaded

### Tabs

| Tab | `data-tab` | Status |
|-----|-----------|--------|
| Orders | `orders` | ✅ Active default |
| Investments | `investments` | ✅ Working |

### Orders Table

| Column | Status |
|--------|--------|
| Order # | ✅ Shown |
| User (email) | ✅ Shown (properly HTML-escaped, XSS safe) |
| Items | ✅ Shown |
| Total | ✅ Formatted USD |
| Payment method | ✅ Shown |
| Status | ✅ Color-coded badge |
| Date | ✅ Formatted |
| Actions | ✅ Approve/Reject for pending orders |

### Order Action Buttons

| Button | API | Status |
|--------|-----|--------|
| **Approve** | `POST /api/admin/orders/:id/approve` | ✅ Working |
| **Reject** | `POST /api/admin/orders/:id/reject` | ✅ Working |

### Investments Table

| Column | Status |
|--------|--------|
| Investor, Asset, Tokens, Purchase Value, Current Value, Total Rental, Status, Date | ✅ All shown |

---

## 🏢 8. Live Assets (`/admin/assets.html`)

**API:** `GET /api/admin/assets`  
**JS:** `admin-assets.js`  
**Data Status:** ✅ 15 assets loaded

### Filter Bar

| Filter | Status |
|--------|--------|
| Search by name | ✅ Working |
| Type filter | ✅ Working (Residential, Commercial, Land, etc.) |
| Status filter | ✅ Working (Active/Draft) |

### Assets Table

| Column | Status |
|--------|--------|
| Asset (image + title) | ✅ Shown (some images 404) |
| Type | ✅ Badge |
| Value | ✅ Formatted |
| Funding Progress | ✅ Bar + percentage |
| Yield | ✅ Percentage |
| Location | ✅ Shown |
| Status | ✅ Badge |
| Featured ⭐ toggle | ✅ Working → `POST /api/admin/assets/:id/toggle-featured` |
| Actions → View Details | ✅ 🔗 → `/admin/asset-details.html?id=` |

---

## 📄 9. Asset Details (`/admin/asset-details.html?id=`)

**API:** `GET /api/admin/assets/:id/detail`  
**JS:** `admin-asset-details.js`  
**Data Status:** ✅ Full asset detail loads

### Sub-Navigation Tabs

| Tab | Status | Content |
|-----|--------|---------|
| Overview | ✅ | Property details, quick stats |
| Media | ✅ | Image gallery, video tour |
| Documents | ✅ | Data room documents |
| Financials | ✅ | Monthly performance table (Period, Rental Income, Expenses, Net Income, Occupancy) |
| Milestones | ✅ | Roadmap & construction milestones |
| Cap Table | ✅ | Ownership distribution table |
| Orders | ✅ | Orders referencing this asset |
| Settings | ✅ | Platform settings for this asset |

### Action Buttons

| Button | Status |
|--------|--------|
| Toggle Featured | ✅ Working → `POST /api/admin/assets/:id/toggle-featured` |
| Toggle Published | ✅ Working → `POST /api/admin/assets/:id/toggle-published` |
| "View on Marketplace" link | ✅ 🔗 External |
| Danger Zone section | ✅ Present |

---

## 🏗️ 10. Developer Submissions (`/admin/developer-submissions.html`)

**API:** `GET /api/admin/submissions`  
**JS:** `admin-submissions.js`  
**Data Status:** ✅ 4 submissions (1 pending, 1 approved, 2 rejected)

### Submissions Table

| Column | Status |
|--------|--------|
| Project name | ✅ |
| Type | ✅ |
| Developer (name/email) | ✅ |
| Linked Asset | ✅ |
| Status | ✅ Badge |
| Total Raised | ✅ |
| Investors | ✅ |
| Progress | ✅ |
| Submitted date | ✅ |
| Actions | ✅ "Review" button → detail page |

### Action Buttons

| Button | API | Status |
|--------|-----|--------|
| Quick Approve | `POST /api/admin/submissions/:id/approve` | ✅ Working |
| Quick Reject | `POST /api/admin/submissions/:id/reject` | ✅ Working |
| Review → Detail Page | `GET /admin/developer-submission-review.html?id=` | ✅ 🔗 Working |

---

## 📋 11. Developer Submission Review (`/admin/developer-submission-review.html?id=`)

**API:** `GET /api/admin/submissions/:id/detail`  
**JS:** `admin-submission-review.js`  
**Data Status:** ✅ Full submission data loads

### Sections

| Section | Status |
|---------|--------|
| Submission Summary | ✅ Loaded |
| Milestone Timeline | ✅ Table with Month, Milestone, Date, Status |
| Review Notes | ✅ Text area |

### Decision Buttons

| Button | Status |
|--------|--------|
| **Approve** | ✅ Working → `POST /api/admin/submissions/:id/approve` |
| **Reject** | ✅ Working → `POST /api/admin/submissions/:id/reject` |

---

## 💎 12. Dividend Distribution Tool (`/admin/dividends.html`)

**API:** `POST /api/admin/dividends/calculate`, `POST /api/admin/dividends/process`  
**JS:** `admin-dividends.js`  
**Data Status:** ✅ Asset dropdown loads, 1 pending distribution visible

### Distribution Form

| Element | Status |
|---------|--------|
| Asset selector dropdown | ✅ Populated with live assets |
| Amount input (USD) | ✅ Working |
| Distribution date | ✅ Date picker |
| "Preview Distributions" button | ✅ Working → calls `/api/admin/dividends/calculate` |

### Preview Table (after preview)

| Column | Status |
|--------|--------|
| Investor | ✅ |
| Tokens Owned | ✅ |
| Share % | ✅ |
| Payout Amount | ✅ |

### Process Buttons

| Button | Status |
|--------|--------|
| **Confirm & Distribute Funds** | ✅ Working → `POST /api/admin/dividends/process` |
| **Discard** | ✅ Working — clears preview |
| **Start New Distribution** | ✅ Working — resets form |
| **View All Approvals** | ✅ 🔗 → `/admin/approvals.html` |

### Distribution Request Tracking Table

| Column | Status |
|--------|--------|
| Distribution ID, Asset, Amount, Status, Date | ✅ All shown |

> ⚠️ **Known Issue:** Dividend distribution uses float math — potential rounding error risk.

---

## 🎁 13. Rewards & Referrals (`/admin/rewards.html`)

**API:** `GET /api/admin/rewards`  
**JS:** `admin-rewards.js`  
**Data Status:** ✅ Shows $1,225.00 total liability, 5 loyalty tiers

### Summary Section

| Element | Status |
|---------|--------|
| Total Rewards Liability | ✅ Shown |
| Tier Distribution Overview | ✅ Bar chart by tier |

### Loyalty Tiers Table

| Column | Status |
|--------|--------|
| Tier Name | ✅ |
| Min Investment | ✅ |
| Cashback % | ✅ |
| Referral Bonus % | ✅ |
| Edit button | ✅ Opens tier editor |

### Actions

| Button | API | Status |
|--------|-----|--------|
| **Add Tier** | `POST /api/admin/rewards/tiers` | ✅ Working |
| **Edit Tier** | `PATCH /api/admin/rewards/tiers/:name` | ✅ Working |
| **Adjust Reward Balance** | `POST /api/admin/rewards/balances/:user_id/adjust` | ✅ Working |
| **Override Tier** (per user) | `PATCH /api/admin/rewards/referrals/:ref_id` | ✅ Working |

### Referral Codes Table

| Column | Status |
|--------|--------|
| User, Code, Uses, Earnings, Status | ✅ All shown |

---

## 🛡️ 14. Platform Settings (`/admin/settings.html`)

**API:** `GET/POST /api/admin/settings`  
**JS:** `admin-settings.js` (Alpine.js data component `adminSettings`)  
**Data Status:** ✅ Settings load with platform name "POOOL"

### Tabs (6 total)

#### Tab 1: General

| Field | Binding | Status |
|-------|---------|--------|
| Platform Name | `x-model="settings.platform_name"` | ✅ Editable |
| Support Email | `x-model="settings.support_email"` | ✅ Editable |
| Enable Registrations | Checkbox | ✅ Toggleable |
| Require KYC | Checkbox | ✅ Toggleable |
| **Save General Settings** | Button | ✅ Working → `POST /api/admin/settings` |

#### Tab 2: Finance & Fees

| Field | Status |
|-------|--------|
| Platform Fee (%) | ✅ Editable |
| Withdrawal Fee (USD) | ✅ Editable |
| Referral Commission (%) | ✅ Editable |
| Min. Withdrawal (cents) | ✅ Editable |
| **Save Financial Settings** | ✅ Working |

#### Tab 3: Notifications

| Element | Status |
|---------|--------|
| Broadcast Title input | ✅ Working |
| Broadcast Message textarea | ✅ Working |
| Type selector (System, Info, Promotion, Warning) | ✅ Working |
| **Send to All Users** button | ✅ Working → `POST /api/admin/notifications/broadcast` |

#### Tab 4: Maintenance

| Element | Status |
|---------|--------|
| Maintenance Mode Toggle | ✅ Working → `POST /api/admin/settings/maintenance` |
| **Clear All Cache** button | ✅ Working → `POST /api/admin/maintenance/clear-cache` |
| **Run Log Rotation** button | ✅ Working → `POST /api/admin/maintenance/rotate-logs` |

#### Tab 5: Integrations

| Element | Status |
|---------|--------|
| Resend API Key input | ✅ Editable (password type) |
| **Save Integrations** button | ✅ Working |

#### Tab 6: Legal & Compliance

| Element | Status |
|---------|--------|
| Consent Overview (Total, Accepted, Pending) | ✅ Loaded from API |
| Terms & Conditions Version input | ✅ Editable |
| Privacy Policy Version input | ✅ Editable |
| Last Updated Date picker | ✅ Editable |
| Impact warning (amber callout) | 📋 Display only |
| **Save Legal Versions** button | ✅ Working → `POST /api/admin/legal/version` |
| View Terms ↗ | ✅ 🔗 Opens `/terms` in new tab |
| View Privacy ↗ | ✅ 🔗 Opens `/privacy-policy` in new tab |
| View Audit Log link | ✅ 🔗 → `/admin/audit-logs?action=legal_document.updated` |

---

## 🔑 15. Roles & RBAC Matrix (`/admin/roles.html`)

**API:** `GET /api/admin/roles`, `GET /api/admin/permissions`, `POST /api/admin/roles/permissions`  
**JS:** `admin-rbac.js`  
**Data Status:** ✅ 5 roles loaded (admin, compliance, finance, super_admin, support)

### Elements

| Element | Status |
|---------|--------|
| Permission Matrix table (roles vs. permissions) | ✅ Rendered with checkboxes |
| Permission checkboxes per role/domain | ✅ Toggleable |
| **Save Permissions** button | ✅ Working → `POST /api/admin/roles/permissions` |

### Create New Role

| Element | Status |
|---------|--------|
| Role Name input | ✅ Working |
| Description input | ✅ Working |
| Initial permissions checkboxes | ✅ Working |
| **Create Role** button | ✅ Working → `POST /api/admin/roles` |
| **Reset** button | ✅ Clears form |

### Available Permissions (21 total)

`users.view`, `users.edit`, `users.delete`, `kyc.read`, `kyc.write`, `kyc.override`, `treasury.read`, `treasury.write`, `financials.payout.draft`, `financials.payout.approve`, `assets.create`, `assets.edit`, `assets.publish`, `support.read`, `support.write`, `support.manage`, `settings.view`, `settings.edit`, `admins.manage`, `roles.edit`, `pii.view`, `all`

---

## 👔 16. Admin Directory (`/admin/admins.html`)

**API:** `GET /api/admin/admins`, `GET /api/admin/admins/invitations`  
**JS:** `admin-access.js`  
**Data Status:** ✅ 4 admins listed

### Tabs

| Tab | Status |
|-----|--------|
| Active Admins | ✅ Default — shows admin table |
| Pending Invitations | ✅ Shows invitations list |

### Active Admins Table

| Column | Status |
|--------|--------|
| Admin (name + email) | ✅ |
| Roles | ✅ Badges |
| Status | ✅ Badge |
| 2FA | ✅ Enabled/Disabled badge |
| Last Active | ✅ Relative time |
| Sessions | ✅ Count |
| Actions | ✅ Edit button |

### Admin Actions

| Button | API | Status |
|--------|-----|--------|
| **Edit Admin** | Opens modal with role editor | ✅ Working |
| **Revoke All Active Sessions** | Per-admin action | ✅ Working |
| **Force Password Reset** | `POST /api/admin/users/:id/force-password-reset` | ✅ Working |

### Invite New Admin

| Element | Status |
|---------|--------|
| Email input | ✅ Working |
| Role selector | ✅ Working (populated from roles API) |
| **Send Invitation** button | ✅ Working → `POST /api/admin/admins/invite` |

### Pending Invitations Table

| Column | Status |
|--------|--------|
| Email, Role, Invited By, Created, Expires | ✅ All shown |
| **Revoke** button | ✅ Working → `DELETE /api/admin/admins/invitations/:id` |
| **Resend** button | ✅ Working → `POST /api/admin/admins/invitations/:id/resend` |

---

## ✅ 17. Approval Queue (`/admin/approvals.html`)

**API:** `GET /api/admin/approvals`, `POST /api/admin/approvals`  
**JS:** `admin-approvals.js`  
**Data Status:** ✅ 1 pending request visible

### Tabs

| Tab | `data-tab` | Status |
|-----|-----------|--------|
| All | `all` | ✅ Working |
| Pending | `pending` | ✅ Default |
| Approved | `approved` | ✅ Working |
| Rejected | `rejected` | ✅ Working |
| Expired | `expired` | ✅ Working |

### Approvals Table

| Column | Status |
|--------|--------|
| Request ID, Type, Requester, Status, Details, Created | ✅ All shown |

### Approval Actions

| Button | API | Status |
|--------|-----|--------|
| **Approve & Execute** | `POST /api/admin/approvals/:id/approve` | ✅ Working |
| **Reject** | `POST /api/admin/approvals/:id/reject` | ✅ Working |

### Submit New Approval Request (Maker)

| Element | Status |
|---------|--------|
| Request Type selector | ✅ Working |
| Details textarea | ✅ Working |
| **Submit for Approval** button | ✅ Working → `POST /api/admin/approvals` |

---

## 📧 18. Email Engine & Marketing Hub (`/admin/email-marketing.html`)

**API:** `GET /api/admin/emails`  
**JS:** `admin-email-marketing.js`  
**Data Status:** ✅ Shows 42.5% open rate

### Tabs (4 total)

| Tab | Status | Content |
|-----|--------|---------|
| Dashboard & Analytics | ✅ | Open rate, send stats, recent failures |
| Template Editor | ✅ | Template CRUD form |
| Campaigns | ✅ | Campaign creation and sending |
| Delivery Logs | ✅ | Email delivery history |

### Dashboard Cards

| Card | Status |
|------|--------|
| Total Sent, Open Rate, Bounce Rate, Unsubscribes | ✅ Populated |

### Template Editor Actions

| Button | API | Status |
|--------|-----|--------|
| **Create Template** | `POST /api/admin/emails/templates` | ✅ Working |
| **Save Template** | `PUT /api/admin/emails/templates/:id` | ✅ Working |
| **Edit** (per template) | Opens editor | ✅ Working |

### Campaign Actions

| Button | API | Status |
|--------|-----|--------|
| **Send Campaign** | `POST /api/admin/emails/campaigns` | ✅ Working |

### Delivery Logs Table

| Column | Status |
|--------|--------|
| User, Subject, Status, Time | ✅ All shown |

> ⚠️ **Known Issue:** Template Editor's HTML content may not roundtrip perfectly from `GET /api/admin/emails`.

---

## 🔔 19. Notifications Center (`/admin/notifications.html`)

**API:** `GET /api/admin/notifications`, `POST /api/admin/notifications/broadcast`  
**JS:** `admin-notifications.js`  
**Data Status:** ✅ 82 notifications listed

### Broadcast Notification Form

| Element | Status |
|---------|--------|
| Title input | ✅ Working |
| Message textarea | ✅ Working |
| Type selector | ✅ Working |
| **Send Broadcast** button | ✅ Working → `POST /api/admin/notifications/broadcast` |

### Notifications Table

| Column | Status |
|--------|--------|
| User, Type, Title, Read (✓/✗), Date | ✅ All shown |

---

## 🎫 20. Customer Support Tickets (`/admin/support.html`)

**API:** `GET /api/admin/support`  
**JS:** `admin-support.js`  
**Data Status:** ✅ 10 tickets loaded

### Filters

| Filter | Status |
|--------|--------|
| Search by subject/user | ✅ Working |
| Status filter (Open, In Progress, Closed) | ✅ Working |
| Priority filter (Low, Medium, High, Urgent) | ✅ Working |

### Tickets Table

| Column | Status |
|--------|--------|
| ☑ Select checkbox | ✅ Clickable |
| Subject | ✅ |
| User | ✅ |
| Priority | ✅ Badge |
| Status | ✅ Badge |
| Created | ✅ |
| Updated | ✅ |
| Actions → View | ✅ 🔗 → `/admin/support-ticket.html?id=` |

### Bulk Actions

| Button | API | Status |
|--------|-----|--------|
| **Apply to Selected** (status change) | `PATCH /api/admin/support/bulk` | ✅ Working |

---

## 📝 21. Support Ticket Detail (`/admin/support-ticket.html?id=`)

**API:** `GET /api/admin/support/:id`, `PATCH /api/admin/support/:id`  
**JS:** `admin-support-ticket.js`  
**Data Status:** ✅ Full ticket details load

### Ticket Detail Panel

| Element | Status |
|---------|--------|
| Subject | ✅ Shown |
| Description | ✅ Shown |
| Status badge | ✅ Shown |
| Priority badge | ✅ Shown |
| Customer profile card | ✅ Loaded (name, email, join date) |

### Actions

| Button | API | Status |
|--------|-----|--------|
| **Update Ticket** (status/priority change) | `PATCH /api/admin/support/:id` | ✅ Working |
| **Send Reply** | `POST /api/admin/support/:id/messages` | ✅ Working |
| **Add Internal Note** | `POST /api/admin/support/:id/messages` (type=internal) | ✅ Working |

### Conversation Thread

| Element | Status |
|---------|--------|
| Message history | ✅ Rendered with timestamps |
| Internal notes (highlighted differently) | ✅ Distinguished visually |

---

## 📊 22. Audit Logs (`/admin/audit-logs.html`)

**API:** `GET /api/admin/audit-logs`  
**JS:** `admin-audit.js`  
**Data Status:** ✅ 316 entries loaded

### Filters

| Filter | Status |
|--------|--------|
| Search by action/entity | ✅ Working |
| Action type filter | ✅ Working |
| Date range filter | ✅ Working |

### Audit Logs Table

| Column | Status |
|--------|--------|
| ID | ✅ |
| Action | ✅ |
| Entity | ✅ |
| Actor | ✅ |
| IP Address | ✅ |
| Timestamp | ✅ |
| Detail (expand to view) | ✅ Click to open detail modal |

### Buttons

| Button | Status |
|--------|--------|
| Export CSV | ✅ Working |
| Pagination | ✅ Working |

---

## 📈 23. Reports & Exports (`/admin/reports.html`)

**API:** `GET /api/admin/tax-reports`, `POST /api/admin/tax-reports/generate`  
**JS:** `admin-reports.js`  
**Data Status:** ✅ All report categories visible

### Date Range Toggles

| Button | Status |
|--------|--------|
| 30d | ✅ Working |
| 90d | ✅ Working |
| YTD | ✅ Working |

### Report Categories

| Category | Reports | Status |
|----------|---------|--------|
| **Financial** | Revenue Summary, Fee Analysis, Deposit Report, Withdrawal Report | ✅ Download/Preview buttons visible |
| **Users & Compliance** | User Growth, KYC Status, Referral Activity | ✅ Download/Preview buttons visible |
| **Operational** | Support Metrics, System Performance | ✅ Download/Preview buttons visible |
| **Tax & Fiscal** | Investor Tax Statements, Platform Tax Summary | ✅ Download/Preview buttons visible |
| **Assets & Investments** | Portfolio Performance, Occupancy Report | ✅ Download/Preview buttons visible |

### Report Actions per Item

| Button | Status |
|--------|--------|
| **Download** (CSV/PDF) | ⚠️ Clickable but generates client-side mock data |
| **Preview** | ⚠️ Shows preview modal with sample data |
| Close preview | ✅ Working |

> ⚠️ **Known Issue:** Most reports generate client-side — no real backend report generation.

---

## 🖥️ 24. System Health & Operations (`/admin/system.html`)

**API:** `GET /api/admin/system`  
**JS:** `admin-system.js`  
**Data Status:** ⚠️ Partial — some sections load, some tabs have missing APIs

### Overview Tab

| Element | Status |
|---------|--------|
| Environment info (version, build date) | ✅ Shown |
| API Latency (24ms) | ✅ Shown |
| Database connection status | ✅ Shown |
| Recent System Logs | ✅ Shown |
| Database Table Statistics | ⚠️ May get stuck on "Loading..." |

### Tab Status

| Tab | Expected Functionality | Status |
|-----|----------------------|--------|
| **Overview** | Environment, logs, DB stats | ✅ Working |
| **Background Jobs** | Job queue listing with retry/cancel | ❌ **NOT IMPLEMENTED** — API `/api/admin/system/jobs` missing |
| **Webhook Logs** | Webhook delivery history with replay | ❌ **NOT IMPLEMENTED** — API `/api/admin/system/webhooks` missing |
| **Active Sessions** | All platform sessions with revoke | ❌ **NOT IMPLEMENTED** — API `/api/admin/system/sessions` missing |
| **Password Resets** | Reset token management | ❌ **NOT IMPLEMENTED** — API `/api/admin/system/password-resets` missing |

### Maintenance Buttons

| Button | API | Status |
|--------|-----|--------|
| Clear Cache | `POST /api/admin/maintenance/clear-cache` | ✅ Working |
| Rotate Logs | `POST /api/admin/maintenance/rotate-logs` | ✅ Working |
| Maintenance Mode toggle | `POST /api/admin/settings/maintenance` | ✅ Working |

---

## 📦 25. Storage Analytics (`/admin/storage.html`)

**API:** `GET /api/admin/storage`  
**JS:** `admin-storage.js`  
**Data Status:** ✅ 3 total GCS files

### KPI Cards

| Card | Status |
|------|--------|
| Estimated Storage | ✅ Populated |
| Total GCS Files | ✅ Populated |
| Storage Cost / Month | ✅ Estimated |
| Operations Cost / Month | ✅ Estimated |

### Sections

| Section | Status |
|---------|--------|
| Upload Trend (6 Months) | ✅ Chart rendered |
| Storage by Type | ✅ Donut/bar chart |
| Recent Document Uploads table | ✅ Loaded |

### Recent Uploads Table

| Column | Status |
|--------|--------|
| Type, Document ID, User Account, Status | ✅ All shown |
| "Review" action | ✅ 🔗 → KYC queue |
| "View KYC Queue →" link | ✅ 🔗 → `/admin/kyc.html` |

---

## 🔧 Cross-Cutting Features

### Permission Guard (`admin-permission-guard.js`)
- ✅ Wraps `window.fetch` to intercept 403 responses
- ✅ Shows access denied message on permission failures
- ✅ RBAC-aware — endpoints check `check_permission()` for granular access

### Theme Support (`admin-theme.js`)
- ✅ Dark/light mode toggle available
- ✅ Persists preference to localStorage

### Debug & Seeding

| Endpoint | Status |
|----------|--------|
| `POST /api/admin/debug/seed` | ✅ Working — Seeds demo data |

---

## ❌ Summary: Known Non-Functional Features

| Area | Feature | Reason |
|------|---------|--------|
| System Health | Background Jobs tab | API not implemented |
| System Health | Webhook Logs tab | API not implemented |
| System Health | Active Sessions tab | API not implemented |
| System Health | Password Resets tab | API not implemented |
| Reports | Real backend report generation | Reports are client-side mocks |
| Email Marketing | Template HTML roundtrip | GET may not return full HTML body |
| Treasury | Date-filtered reports | Some endpoints return all-time data regardless |
| Dividends | Precision | Uses float math (rounding error risk) |
| Notification Bell | Badge count | Hardcoded to "7" instead of dynamic |
| Asset Images | Some villa/property images | 404 errors on some image paths |
