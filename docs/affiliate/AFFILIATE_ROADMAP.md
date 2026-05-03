# POOOL Affiliate & Referral Subsystem Implementation Roadmap
**(Extended Granularity & Expert Specification)**

This document provides a highly detailed, step-by-step roadmap for implementing Phase 18 and Phase 19 of the masterplan. It translates the legal/tax PDF requirements into precise database connections, frontend components, logic workflows, and UI buttons.

---

## Step 1. Database Architecture, Performance & Scalability
*Status: ✅ DONE (Migration 072 applied)*
**Robustness & Scale:** Das System ist darauf ausgelegt, massives Volumen zu bewältigen, ohne den Trading-Core zu verlangsamen.
- **Tables Created:** `affiliates`, `affiliate_referrals`, `affiliate_commissions`, `affiliate_policy_acceptances`, `investment_disclosures_log`.
- **Performance Optimizations (To be applied):** 
  - Partial Indexes (B-Tree) on `affiliate_referrals(status, holdback_expires_at)` where status is `under_holdback`. Dies garantiert, dass der nächtliche Cronjob in <10ms antwortet, selbst bei Millionen Referrals.
  - ACID-Transactions mit `SELECT ... FOR UPDATE` bei Auszahlungen verhindern Race Conditions, falls zwei Admins zeitgleich den Payout-Button drücken.

---

## Step 2. Backend Scaffolding & Routing
**Goal:** Create the high-performance core structure parallel to the legacy `src/rewards/` and wire it to Axum.

- **Files to Create:**
  - `backend/src/affiliate/mod.rs` (Module export)
  - `backend/src/affiliate/routes.rs` (Axum REST API endpoints)
  - `backend/src/affiliate/models.rs` (Structs for DB deserialization)
  - `backend/src/affiliate/service.rs` (Core validation logic)
  - `backend/src/affiliate/workers.rs` (Cron Jobs and State Machine runners)
- **Modifications:** Update `backend/src/main.rs` to register `/api/affiliate/*` routes.

---

## Step 3. Backend Workflows Sandbox (The "Engines")

**3.1 Attribution Middleware & Tracking:**
- **Trigger:** Request with `?ref=CODE`.
- **Logic:** Drop 30-day `HttpOnly` cookie. On `POST /auth/register`, lock user row and insert into `affiliate_referrals` with `status = 'registered'`.

**3.2 The 5-Stage Qualification State Machine:**
- **Trigger:** Checkout completion (`payments/service.rs`).
- **Logic:** Wenn Käufer in `affiliate_referrals` existiert ➔ Generiere `provisionally_tracked` Provision in `affiliate_commissions` ➔ Setze Referral `holdback_expires_at` auf `NOW() + 30 Days`.

**3.3 Nightly Holdback Worker (`workers.rs`):**
- **Trigger:** Tokio scheduler (1x Täglich).
- **Logic:** Scannt alle `under_holdback`. Checkt Trade-Status (storniert/aktiv). Wenn aktiv: Status ➔ `payable`. Löst Datenbank-Trigger aus, der Admin-Dashboard aktualisiert.

---

## Step 4. Frontend Ecosystem A: Public & Investor Flows

**4.1 Registration Flow & Pre-Investment Checkout Disclosures:**
- **Files:** `frontend/platform/register.html`, `frontend/platform/marketplace-trading-v3.html`
- **Workflow:** 
  - Registration zeigt Hinweis auf Tracking ("You have been referred by [Code]").
  - **Checkout Panel:** API fragt `is_referral_user` ab. Bei `true` entfaltet sich ein "Expandable Disclosure Panel" mit 6 (statt 3) Checkboxen.
  - **DB Connect:** Klick auf "Invest" schreibt IP, Timestamp und Check-Status in `investment_disclosures_log`. 

---

## Step 5. Frontend Ecosystem B: Affiliate Portal (Affiliate UI)

**5.0 Affiliate Promo & Blocked State (For Non-Affiliates):**
- **File:** `frontend/platform/affiliate-promo.html`
- **Workflow & Access Control:** 
  - Klickt ein "normaler" User im Menü auf "Affiliate" (Marketing), sind alle Dashboards und Links strikt gesperrt.
  - Er landet auf einer Landing-Page innerhalb des Portals, die das Programm "verkauft": Übersicht der 8 Tiers, Boni, Materialien. 
  - Call-to-Action Button: "Apply Now" (leitet ihn in den Onboarding Wizard).

**5.1 Onboarding Wizard & Compliance Quiz:**
- **File:** `frontend/platform/affiliate-onboarding.html`
- **Features & QoL:** Stepper/Progress Bar, Auto-Save bei Step 3, Mobile responsive.
- **Workflow:** Profile ➔ KYC (Didit API) ➔ Tax Setup ➔ Legal Acceptance (5 checkboxes pushing to `affiliate_policy_acceptances`).
- **The Compliance Quiz (Based strictly on Code of Conduct):**
  - Es müssen 100 % richtig beantwortet werden (sonst Reset und 10 Min Sperre).
  - *Q1:* Darfst du Investmentberatung für POOOL-Assets anbieten? (Lösung: Nein, nur Referral Base Information)
  - *Q2:* Darfst du feste Renditen oder Sicherheit garantieren? (Lösung: Niemals)
  - *Q3:* Wann wird deine Provision ausbezahlt? (Lösung: Nach 30 Tagen Holdback, KYC und Tax-Clearance)
  - *Q4:* Darfst du Kundengelder annehmen oder weiterleiten? (Lösung: Absolut verboten)
  - *Q5:* Erlaubt POOOL Kaltakquise (Cold Outreach) per WhatsApp/Telegram? (Lösung: Nein)

**5.2 Affiliate Dashboard Overview:**
- **File:** `frontend/platform/affiliate-dashboard.html`
- **Components & DB Connects:**
  - `Tier Progress Card`: Holt `current_tier` aus `affiliates`. Zeigt ProgressBar bis zum nächsten Tier.
  - `Link Widget`: Copy-to-Clipboard Button, QR-Code Generator.
  - `Earnings Card`: Zeigt *Provisional* (in holdback) und *Payable* (bereit). Connect via `SUM(provisional_amount_cents)` aus `affiliate_commissions`.

**5.3 Referrals & Payouts Funnel:**
- **File:** `frontend/platform/affiliate-referrals.html`
- **Components:** DataTable mit Tabs (Tracked | Under Review | Payable | Paid).
- **QoL Features:** Suchen, Filtern, Export to CSV.

**5.4 Marketing Materials Hub & Settings:**
- **Files:** `affiliate-materials.html`, `affiliate-settings.html`
- **Components:** Media-Grid für Banner-Downloads. Upload-Form (`Drag&Drop`) für Custom-Assets ➔ pusht in GCS und setzt Status in DB auf `pending_admin_approval`. Tax-Settings-Maske (friezt Account bei Änderung).

---

## Step 6. Frontend Ecosystem C: Admin Dashboard Expansion

**6.1 Affiliate Applications Desk:**
- **File:** `frontend/platform/admin/admin-affiliate-applications.html`
- **Components:** Grid aller Tracker-Steps (Hat KYC? Hat Quiz bestanden?). 
- **Buttons / Actions:** `Approve Affiliate` (ändert DB `status='active'`, triggert Email) / `Reject` (löscht Affiliate Datensatz soft).

**6.2 Finance & Tax Release Board:**
- **File:** `frontend/platform/admin/admin-affiliate-finance.html`
- **Components:** DataTable aller `payable` Commissions gruppiert nach `affiliate_id`.
- **Buttons / Actions:** 
  - `Dropdown (Tax Class)`: Weist *Indonesian Individual*, *Entity* etc. zu. Update in `affiliates`.
  - `Button: Mark Tax Ready`: Ändert `affiliates.is_tax_ready = true` (ohne diesen Klick ist der Payout-Button geblockt/grau).
  - `Button: Release Payout Batch`: Öffnet Confirmation Modal ➔ Startet ACID-Transaction im Backend (zieht von `POOOL_Treasury_Wallet` ab, bucht auf Affiliate Wallet, schreibt `wallet_transactions`, setzt Commission auf `paid`).
  - **Performance:** Bulk-Actions via Checkboxes für 50+ Auszahlungen in einer Tx.

**6.3 Compliance & Case Management:**
- **File:** `frontend/platform/admin/admin-affiliate-compliance.html`
- **Features:** Triage-Board für Complaints. Sidebar mit Eskalations-Buttons:
  - `Button: Freeze Link` (Keine neuen Trackings).
  - `Button: Clawback Commission` (Bucht bereits ausbezahlte Funds vom Affiliate zurück ins Treasury via `negative wallet_transaction`).
  - `Button: Suspend Account` (Hard-Block).

**6.4 Fraud Visualizer (Referral Rings):**
- **File:** `frontend/platform/admin/admin-affiliate-fraud.html`
- **Components:** Network Graph (z.B. vis.js) oder hierarchische Tree-View.
- **Logic / DB:** Sucht rekursiv nach *A wirbt B, B wirbt A* oder identischen IP-Adressen in `affiliate_referrals` und `users` und taggt diese Accounts rot.

**6.5 Materials Approval Board:**
- **File:** `frontend/platform/admin/admin-affiliate-materials.html`
- **Workflow:** Side-by-Side View (Upload vs. Policy Checklist). Buttons: `Approve` (Schaltet Datei in `affiliate-materials.html` für Affiliate frei) / `Reject w/ Reason` (Triggert Email).

---

## Step 7. Legacy Cleanup (Phase 19.18)

**7.1 Deprecation of legacy Rewards platform**
- **Action:** Delete old files (e.g., `frontend/platform/rewards.html`) and old API routes.
- **Constraint:** Execute only AFTER the new `affiliate-` ecosystem is 100% verified to prevent visually missing features.
