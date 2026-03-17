# POOOL Broken Features Report

Generated: 2026-03-14 (Cleanup Update)

---

## AGENT SYNC BOARD

| Page/Route | Status | Agent | Timestamp |
|------------|--------|-------|------------|
| /admin/index.html | COMPLETED | Antigravity | 2026-03-09 01:15 local |
| /admin/users.html | COMPLETED | QA-Agent-001 | 2026-03-08 16:10 UTC |
| /admin/user-details.html | COMPLETED | QA-Agent-002 | 2026-03-08 16:40 UTC |
| /admin/developer-submissions.html | COMPLETED | QA-Agent-003 | 2026-03-08 16:40 UTC |
| /admin/developer-submission-review.html | COMPLETED | Antigravity | 2026-03-14 14:50 local |
| /admin/assets.html | COMPLETED | QA-Agent-001 | 2026-03-08 16:45 UTC |
| /admin/asset-details.html | COMPLETED | Antigravity | 2026-03-14 14:50 local |
| /admin/deposits.html | COMPLETED | Antigravity | 2026-03-09 local |
| /admin/orders.html | COMPLETED | Antigravity | 2026-03-09 local |
| /admin/treasury.html | COMPLETED | Antigravity | 2026-03-08T23:55:00 |
| /admin/kyc.html | COMPLETED | QA-Agent-001 | 2026-03-08 16:20 UTC |
| /admin/rewards.html | COMPLETED | Antigravity | 2026-03-09T00:00:00 |
| /admin/support.html | COMPLETED | Antigravity | 2026-03-09T00:05:00 |
| /admin/support-ticket.html | COMPLETED | Antigravity | 2026-03-09T00:05:00 |
| /admin/notifications.html | COMPLETED | Antigravity | 2026-03-09T00:50:00 |
| /admin/audit-logs.html | COMPLETED | Antigravity | 2026-03-09T00:50:00 |
| /admin/email-marketing.html | COMPLETED | Antigravity | 2026-03-09 01:15 local |
| /admin/reports.html | COMPLETED | Antigravity | 2026-03-14 14:50 local |
| /admin/system.html | COMPLETED | Antigravity | 2026-03-14 14:50 local |
| /admin/settings.html | COMPLETED | Antigravity | 2026-03-09T00:50:00 |
| /admin/admins.html | COMPLETED | Antigravity | 2026-03-09 00:50 local |
| /admin/roles.html | COMPLETED | Antigravity | 2026-03-09 00:50 local |
| /admin/approvals.html | COMPLETED | Antigravity | 2026-03-09 01:15 local |
| /admin/dividends.html | COMPLETED | Antigravity | 2026-03-09 01:15 local |

---

## Critical Failures (1)

### 1. Wallet Deposit Logic Verification
- **Feature:** Wallet deposit transaction
- **Issue:** Reports of missing transaction records after deposit. 
- **Status:** Requires verification. (Code exists but database sync may have lag or edge case failures).
- **Impact:** Deposits don't create wallet transaction history

---

## Warnings & Missing Implementations

### System Health & Operations Page (/admin/system.html)

| Feature | UI Expected | Backend Status |
|---------|-------------|----------------|
| Background Jobs | Table with job name, status, attempts, payload, scheduling | **IMPLEMENTED** ✅ |
| Retry Job | Retry button for failed jobs | **IMPLEMENTED** ✅ |
| Cancel Job | Cancel button for pending/processing jobs | **IMPLEMENTED** ✅ |
| Webhook Logs | Table with provider, endpoint, HTTP status, payload | **IMPLEMENTED** ✅ |
| Replay Webhook | Replay button for failed webhooks | **IMPLEMENTED** ✅ |
| Active Sessions | Table with user, IP, user agent, remember me, expiry | **IMPLEMENTED** ✅ |
| Revoke Session | Revoke button for active sessions | **IMPLEMENTED** ✅ |
| Bulk Revoke Sessions | Bulk revoke by IP pattern | **IMPLEMENTED** ✅ |
| Password Reset Tokens | Table with reset tokens and abuse detection | **IMPLEMENTED** ✅ |
| Database Table Stats | Table sizes and row counts | **IMPLEMENTED** ✅ |
| Storage Analytics | Detailed storage breakdown & trends | **IMPLEMENTED** ✅ |
| Server Cost KPI | Monthly cost estimates for Infra | **IMPLEMENTED** ✅ (Heuristic Estimates) |
| Clear Cache | Button to purge all caches | **NOT IMPLEMENTED** |
| Rotate Logs | Button to archive logs | **NOT IMPLEMENTED** |
| Maintenance Mode | Toggle button for maintenance mode | **IMPLEMENTED** ✅ |

---

### Reports Page (/admin/reports.html)

| Feature | Status |
|---------|--------|
| Financial Reports section | **IMPLEMENTED** ✅ |
| Reports & Compliance Reports | **IMPLEMENTED** ✅ |
| Assets & Investment Reports | **IMPLEMENTED** ✅ |
| Operational Reports | **IMPLEMENTED** ✅ |
| Tax & Fiscal Reports | **INCOMPLETE** (Hardcoded 0 for Tax/Gains; Missing PDF) |
| Report Preview | **IMPLEMENTED** ✅ |
| Date Filtering | **IMPLEMENTED** ✅ |
| CSV/Excel Export | **NOT IMPLEMENTED** (Backend missing generator) |
| Download PDF | **NOT IMPLEMENTED** (Façade only) |

---

### Email Marketing Page (/admin/email-marketing.html)

| Feature | Status / Issue |
|---------|----------------|
| Template Editor | **IMPLEMENTED** ✅ (Includes HTML field) |
| Template Persistence | **IMPLEMENTED** ✅ (PUT /api/admin/emails/templates/:id working) |
| Campaign Sending | **INCOMPLETE** (Logs as 'sent' but no actual SMTP/Job trigger) |
| Delivery Logs | **IMPLEMENTED** ✅ (Returns from `email_logs`) |
| Audience Support | **PARTIAL** (Dormant/Tier Plus segments unimplemented in backend) |
| Analytics Stats | **MOCKED** (Open/Click/Bounce rates are hardcoded fallback values) |

---

### Navigation & Routing Issues

| Issue | Details |
|-------|---------|
| /checkout route | **NOT IMPLEMENTED** - needs backend implementation |
| /developer | ❌ 404 (Use `/developer/dashboard` or add naked redirect) |
| /legal | ❌ 404 (Use specific legal routes or add consolidated index) |
| /admin/support | ⚠️ 404 (Must use /admin/support.html or add route alias) |

---

## Recently Resolved Issues (Fixed)

- [x] **Login Endpoint:** Fixed 500 error on POST /auth/login.
- [x] **Admin Profile:** Fixed missing profile for admin@poool.finance.
- [x] **Admin Dashboard Redirection:** Fixed redirection logic in account switcher.
- [x] **Developer Submission Review:** Fixed layout on mobile and application ID truncation.
- [x] **Admin Image Upload:** Fixed silent failures and added drag-and-drop reordering.
- [x] **Local File Fallback:** Implemented local storage fallback for development.
- [x] **Storage Analytics:** Switched from dummy data to real database-backed analytics.
- [x] **Edit Roles Modal:** Fixed DOM positioning issue.
- [x] **Build Evidence Button:** Fixed missing API linkage.
- [x] **Table Sorting:** Fixed field name mismatches in Orders/Investments tables.
- [x] **Treasury Export:** Implemented client-side CSV export functionality.
- [x] **Context in Support:** Original user message now included in ticket threads.
- [x] **Dividend Rounding:** Switched to fixed-point integer math (`u128` cumulative) to prevent rounding loss.

---

## Summary

- **Critical Failures:** 1 (Verification pending)
- **Status:** Improving
- **Coverage:** ~85% of Admin features now have functional backend support.

---

## Next Steps / Recommendations

1. **Implement /checkout:** High priority for user conversion flow.
2. **PDF/CSV Generators:** Add real export capabilities for reports and tax forms.
3. **Bridge Marketing Gap:** Implement actual SMTP delivery and support for Dormant/Tiered segments.
4. **Maintenance Ops:** Implement "Clear Cache" and "Rotate Logs" buttons.
5. **Real-time Stats:** Shift Email Marketing analytics from hardcoded mocks to live aggregation.
