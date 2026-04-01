# Affiliate System — Bug & Gap Fix Plan
_Last updated: 2026-03-31_
_Implementation completed: 2026-03-31_ ✅

> **Status:** All items below have been implemented. See `database/076_affiliate_system_gaps.sql` for the required DB migration to run before deploying.

---

## Overview

The POOOL affiliate system covers the full journey from onboarding → attribution → commission tracking → holdback → payout. The core infrastructure is largely in place (schema, service logic, API routes, frontend pages, admin tooling), but a compilation error blocks the background worker, and a set of significant gaps remain before the system is fully production-ready.

**Overall readiness: ~85–90%**
**Estimated effort to close all gaps: 60–80 engineering hours (~3 weeks)**

---

## System Architecture (for context)

| Layer | Key Files | Status |
|---|---|---|
| Database | `database/072–075_affiliate_*.sql` | ✅ Complete |
| Backend service | `backend/src/rewards/service.rs` | ⚠️ 1 compilation bug |
| Backend routes | `backend/src/rewards/routes.rs` | ✅ 95% |
| Admin routes | `backend/src/admin/rewards.rs` | ⚠️ 2 missing endpoints |
| Frontend (user) | `frontend/platform/affiliate-*.html/js` | ⚠️ 85% |
| Frontend (admin) | `frontend/platform/admin/affiliate-*.html` | ⚠️ Partial |

### End-to-end happy path
```
Apply → Admin approves → Get referral link → Referred user registers → Invests
  → 30-day holdback → Commission qualifies → Affiliate requests payout
    → Admin releases payout → Cash lands in wallet → Affiliate withdraws
```

---

## 🔴 P0 — Critical (Blocking)

### BUG-01 · Compilation error in holdback worker
**File:** `backend/src/rewards/service.rs` · lines 1172–1179
**Function:** `run_affiliate_holdback_worker()`

The type-chaining on `sqlx::query_scalar!` is wrong — `.unwrap_or(None).flatten()` is called on a `bool`, which doesn't compile.

**Current code:**
```rust
let order_active: bool = sqlx::query_scalar!(
    "SELECT status IN ('completed', 'approved') FROM orders WHERE id = $1 LIMIT 1",
    commission.source_order_id
)
.fetch_optional(&pool)
.await
.unwrap_or(None)   // ← wrong: fetch_optional already returns Result<Option<_>>
.flatten()
.unwrap_or(false);
```

**Fix:**
```rust
let order_active: bool = sqlx::query_scalar!(
    "SELECT status IN ('completed', 'approved') FROM orders WHERE id = $1 LIMIT 1",
    commission.source_order_id
)
.fetch_optional(&pool)
.await?              // propagate error instead of swallowing
.flatten()           // Option<Option<bool>> → Option<bool>
.unwrap_or(false);
```

**Impact:** The entire holdback background worker (runs every 6 hours) cannot compile. No commissions will ever transition to `payable`. Zero affiliate payouts are possible until this is fixed.

**Effort:** ~15 min

---

## 🟠 P1 — Major Gaps

### GAP-01 · S2S postback not fired for commission events
**File:** `backend/src/rewards/service.rs`
**Function:** `run_affiliate_holdback_worker()`

`trigger_s2s_postback()` is correctly called on the `registration` event (line ~817), but is never called when a commission transitions to `qualified` or `payable`. Affiliates who integrate server-side tracking (e.g. HasOffers, Voluum, custom pixels) receive no signal when a commission vests.

**Fix:** After each `qualified` transition inside the holdback worker loop, add:
```rust
trigger_s2s_postback(
    pool.clone(),
    referral.affiliate_id,
    "commission_qualified".to_string(),
    subid.clone(),
    total_commission_cents,
).await;
```

**Effort:** 1–2 hours

---

### GAP-02 · Tier progression worker missing entirely
**File:** `backend/src/rewards/service.rs` (to be added)
**Database column:** `affiliates.current_tier`

The 8-tier system (Access → Bronze → Silver → Gold → Platinum → Diamond → Elite → Ambassador) is wired into the frontend but `current_tier` is never updated after initial approval — it stays `'Access'` forever. No worker queries lifetime qualified referral volume and advances tiers.

**Fix:** Create a new async function `run_affiliate_tier_progression_worker()`, registered in `main.rs` on a daily cadence:

```rust
pub async fn run_affiliate_tier_progression_worker(pool: PgPool) {
    loop {
        // 1. For each active affiliate:
        //    SELECT count(*) of qualified referrals → map to tier
        // 2. UPDATE affiliates SET current_tier = $1, commission_rate_bps = $2
        //    WHERE id = $3 AND current_tier != $1
        // 3. Write audit log on tier change
        tokio::time::sleep(Duration::from_secs(86_400)).await; // daily
    }
}
```

Tier thresholds and corresponding commission rates need to be defined as constants (or a config table).

**Effort:** 4–6 hours

---

### GAP-03 · Dynamic link generator UI missing
**File:** `frontend/platform/affiliate-dashboard.html` + `affiliate-dashboard.js`

The backend already parses `?subid=` and `?utm_source=` from referral URLs and stores them correctly. The JS controller has an `updateDynamicLink()` function, but it is not connected to any HTML form — affiliates have no way to generate custom-tagged links from the dashboard.

**Fix:** Add the following block inside the referral link card in `affiliate-dashboard.html`:

```html
<div class="link-generator mt-4">
  <label>Campaign / SubID</label>
  <input id="link-gen-subid" type="text" placeholder="e.g. twitter-may-2026">
  <label>Traffic Source</label>
  <input id="link-gen-utm" type="text" placeholder="e.g. twitter">
  <button onclick="updateDynamicLink()">Generate Link</button>
  <div id="dynamic-link-preview" class="code-block"></div>
</div>
```

Wire `updateDynamicLink()` to read both inputs, append query params to the base referral URL, and update the preview + copy button.

**Effort:** 2–3 hours

---

### GAP-04 · Admin missing endpoint: list pending applications with full detail
**File:** `backend/src/admin/rewards.rs` + router

There is no GET route at `/api/admin/rewards/affiliates/pending` that returns full application details (exam answers, traffic source, audience size, main URL, policy timestamps). The admin list view shows only a name/email grid with no expandable detail.

**Fix (backend):**
- Add `GET /api/admin/rewards/affiliates/pending` returning full `affiliates` rows where `status = 'pending_approval'`, joined with their policy acceptances.
- Wire it in `admin/mod.rs`.

**Fix (frontend):** In `admin-affiliate-applications.html`, make each row expandable/clickable to reveal the full application data in a side-drawer or modal.

**Effort:** 3–4 hours

---

### GAP-05 · Commission clawback endpoint not wired
**File:** `backend/src/admin/rewards.rs`

`ClawbackPayload` struct is defined (line ~1074) but no corresponding route or handler exists. Admins cannot reverse a fraudulent payout.

**Fix:** Implement `POST /api/admin/rewards/affiliates/:id/clawback`:
- Accept `{ commission_id, reason }` body
- Verify commission is in `paid` state
- Begin transaction:
  - Write negative `wallet_transactions` entry (debit affiliate cash wallet)
  - Update `affiliate_commissions.status = 'clawed_back'`
  - Write audit log with reason
- Return updated balance

**Effort:** 4–5 hours

---

## 🟡 P2 — Moderate Gaps

### GAP-06 · Referral state machine incomplete (intermediate states never set)
**File:** `backend/src/rewards/service.rs`

The schema defines states `registered → kyc_approved → first_investment_done → under_holdback → qualified / disqualified` but the code skips from `registered` directly to `under_holdback` on first purchase, ignoring KYC completion.

**Fix:** In the holdback worker and/or in `check_and_track_affiliate_commission()`, add transitional updates:
- When referred user's KYC flips to `approved` → set referral status to `kyc_approved`
- When first investment confirmed → set referral status to `first_investment_done` before `under_holdback`

This enables more granular funnel reporting.

**Effort:** 3–4 hours

---

### GAP-07 · Legacy referral system running in parallel (double-payout risk)
**File:** `backend/src/admin/rewards.rs` lines 82–106

Both the legacy `referral_tracking` table and the new `affiliate_referrals` / `affiliate_commissions` tables are queried in the admin rewards panel. If the same conversion is tracked in both, the affiliate could receive two payouts for one referred user.

**Fix:** Audit which users appear in both tables for the same conversion event. Once confirmed clean:
1. Add a deduplication guard in `check_and_track_affiliate_commission()` that checks legacy table before inserting.
2. Create a migration plan to deprecate `referral_tracking` writes once all legacy affiliates are migrated.

**Effort:** 8–10 hours (audit + migration)

---

### GAP-08 · Policy versioning not enforced
**Database:** `affiliate_policy_acceptances.policy_version` / `affiliates` table

All policy acceptances are stamped version `'1.0'`. If policies change (required for regulatory compliance), existing affiliates are not prompted to re-accept.

**Fix:**
- Add `accepted_policy_version` column to `affiliates` table.
- Add a `current_policy_version` config entry (env var or DB config table).
- In `get_affiliate_dashboard()`, return `policy_reacceptance_required: true` if versions differ.
- On the frontend, show a blocking modal requiring re-acceptance before the dashboard loads.

**Effort:** 3–4 hours

---

### GAP-09 · Fraud ring detection missing
**File:** `backend/src/rewards/service.rs`

Only basic self-referral + same-IP checks exist in `attribute_affiliate_referral()`. Circular rings (A refers B, B refers A) and IP network clusters are not detected.

**Fix:** Add a scheduled daily worker that queries:
```sql
-- Circular rings
SELECT ar1.affiliate_id AS affiliate_a, ar2.affiliate_id AS affiliate_b
FROM affiliate_referrals ar1
JOIN affiliates a ON ar1.referred_user_id = a.user_id
JOIN affiliate_referrals ar2 ON a.id = ar2.affiliate_id
WHERE ar2.referred_user_id = ar1.referred_user_id;
```
Flag matches in an admin alert queue and auto-suspend pending manual review.

**Effort:** 4–6 hours

---

### GAP-10 · Tax document collection missing
**File:** `frontend/platform/affiliate-onboarding.html` (Step 3)

Only a `tax_id` string is collected. IRS rules require a signed W-9 (US persons) or W-8BEN (non-US) for cumulative payouts exceeding $600/year. No upload UI or storage flow exists.

**Fix:**
- Add a file upload step to the onboarding wizard after tax ID entry.
- Upload to GCS with an object path like `affiliates/{user_id}/tax_docs/{filename}`.
- Store GCS path in a new `affiliates.tax_document_gcs_path` column.
- Block payout release in `api_admin_affiliate_batch_payout()` unless `tax_document_gcs_path IS NOT NULL`.

**Effort:** 6–8 hours

---

### GAP-11 · Custom marketing materials upload & approval board missing
**File:** `frontend/platform/affiliate-materials.html` + new admin page

Affiliates can only download preset assets. There's no mechanism for affiliates to upload custom creatives for brand-compliance review, and no admin board to approve/reject them.

**Fix:**
- Add drag-drop upload UI to `affiliate-materials.html` → POST to a new `/api/affiliate/materials/upload` endpoint.
- Store pending assets in GCS under `affiliates/{id}/materials/`.
- Create `admin-affiliate-materials.html` with a review queue (approve / reject with comment).
- Notify affiliate by email on decision.

**Effort:** 6–8 hours

---

## 🟢 P3 — Minor / Polish

### GAP-12 · Tier progression visual UI missing
**File:** `frontend/platform/affiliate-dashboard.html`

The current tier is displayed as a label, but there is no progress bar, tier table, or display of what's required to reach the next tier.

**Fix:** Add a tier progression widget showing all 8 tiers, current tier highlighted, qualified referral count vs. threshold for next tier, and estimated commission rate at next tier.

**Effort:** 2–3 hours

---

### GAP-13 · Click throttle may not be distributed across servers
**File:** `backend/src/rewards/routes.rs` — click-tracking handler

The 10-clicks-per-15-min-per-IP rate limiter uses `auth_rate_limiter`, which may be in-memory only. In a multi-instance deployment, each instance has its own counter, so a single IP could generate up to `10 × N` clicks per window where N = instance count.

**Fix:** Confirm the rate limiter backing store is Redis (shared across instances). If not, swap `auth_rate_limiter` for a Redis-backed equivalent on the click-tracking path.

**Effort:** 1–2 hours

---

### GAP-14 · No historical commission export / time-series filtering
**File:** `frontend/platform/affiliate-referrals.html` + backend

The referrals page shows a flat list of recent commissions with no pagination, no date-range filter, and no CSV/Excel export.

**Fix:**
- Add `?from=&to=&status=` query params to `/api/affiliate/dashboard` (or a new `/api/affiliate/commissions` endpoint).
- Add pagination (`?page=&limit=`).
- Add a "Export CSV" button on the frontend that calls a `/api/affiliate/commissions/export` endpoint.

**Effort:** 4–5 hours

---

### GAP-15 · SubID stats missing revenue dimension
**File:** `backend/src/rewards/routes.rs` — `api_affiliate_subid_stats()`

The SubID stats endpoint returns click and registration counts per SubID but does not include commission amounts — affiliates cannot see which campaigns drive the most revenue.

**Fix:** Extend the query to JOIN `affiliate_commissions` and sum `commission_cents` per subid, grouped by commission status.

**Effort:** 1–2 hours

---

## Summary Table

| # | Severity | Area | Description | Effort |
|---|---|---|---|---|
| BUG-01 | 🔴 P0 | Backend | Compilation error in holdback worker (type mismatch) | 15 min |
| GAP-01 | 🟠 P1 | Backend | S2S postback not fired for commission qualified/payable events | 1–2 h |
| GAP-02 | 🟠 P1 | Backend | No tier progression worker — `current_tier` never updates | 4–6 h |
| GAP-03 | 🟠 P1 | Frontend | Dynamic link generator UI not wired to backend | 2–3 h |
| GAP-04 | 🟠 P1 | Backend + Frontend | No admin endpoint/view for pending application detail | 3–4 h |
| GAP-05 | 🟠 P1 | Backend | Clawback struct defined but endpoint not wired | 4–5 h |
| GAP-06 | 🟡 P2 | Backend | Referral state machine skips intermediate states | 3–4 h |
| GAP-07 | 🟡 P2 | Backend | Legacy + new referral systems running in parallel (double-payout risk) | 8–10 h |
| GAP-08 | 🟡 P2 | Backend + DB | Policy versioning not enforced — no re-acceptance flow | 3–4 h |
| GAP-09 | 🟡 P2 | Backend | No fraud ring detection (circular referral chains) | 4–6 h |
| GAP-10 | 🟡 P2 | Frontend + Backend | No W-9/W-8BEN tax document upload or payout gate | 6–8 h |
| GAP-11 | 🟡 P2 | Frontend + Backend | No custom marketing materials upload or admin approval board | 6–8 h |
| GAP-12 | 🟢 P3 | Frontend | No tier progression visual (progress bar, thresholds) | 2–3 h |
| GAP-13 | 🟢 P3 | Backend | Click throttle may be in-memory only (not distributed) | 1–2 h |
| GAP-14 | 🟢 P3 | Frontend + Backend | No commission export, pagination, or date filter | 4–5 h |
| GAP-15 | 🟢 P3 | Backend | SubID stats missing revenue (commission amount) dimension | 1–2 h |

**Total estimated effort: 58–80 hours**

---

## Recommended Sprint Plan

### Week 1 — Unblock & Core Completeness (~20–25 h)
1. **BUG-01** Fix compilation error (15 min) → unblocks all holdback testing
2. **GAP-01** Wire S2S postback for commission events (2 h)
3. **GAP-03** Dynamic link generator UI (3 h)
4. **GAP-02** Tier progression worker (6 h)
5. **GAP-04** Admin pending applications detail view (4 h)
6. **GAP-05** Commission clawback endpoint (5 h)

### Week 2 — Fraud, Compliance & Safety (~22–30 h)
7. **GAP-07** Audit & decouple legacy referral system (10 h)
8. **GAP-09** Fraud ring detection worker (6 h)
9. **GAP-08** Policy versioning + re-acceptance flow (4 h)
10. **GAP-13** Confirm distributed rate limiter for click throttle (2 h)

### Week 3 — UX Polish & Compliance Finish (~18–25 h)
11. **GAP-10** Tax document upload + payout gate (8 h)
12. **GAP-06** Fill in referral state machine transitions (4 h)
13. **GAP-11** Custom materials upload + admin board (8 h) _(can defer post-launch)_
14. **GAP-12** Tier progression UI widget (3 h)
15. **GAP-14** Commission export + pagination (5 h)
16. **GAP-15** SubID revenue dimension in stats (2 h)

---

## What's Already Working Correctly

The following components are solid and require no changes:

- **Commission calculation** — `order_total × rate_bps / 10_000`, correct 90-day first-conversion window
- **30-day holdback logic** — `holdback_expires_at`, worker scans and transitions correctly (pending BUG-01 fix)
- **KYC gating on approval** — `kyc_status == 'approved'` enforced before any referral code is issued
- **Payout execution engine** — atomic transactions, `SELECT FOR UPDATE SKIP LOCKED`, $50 minimum, audit trail
- **Self-referral + IP overlap fraud check** — enforced in `attribute_affiliate_referral()`
- **Rate limiting on onboarding** — 3 attempts per 15 min per IP
- **Duplicate application guard** — blocks reapplication if pending/active/suspended
- **Policy acceptance logging** — immutable log with IP and timestamp per policy
- **Email notifications** — all lifecycle events (applied, approved, rejected, payout released)
- **Referral cookie** — HttpOnly, 30-day expiry, correctly parsed on signup
- **Attribution flow** — sub_id and utm_source stored correctly on attribution
