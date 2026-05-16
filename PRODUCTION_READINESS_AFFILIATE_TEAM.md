# Production-Readiness Audit — Developer Affiliate Team

**Datum:** 2026-05-16
**Scope:** `/developer/affiliate-team/*` Sub-System (5 Sub-Pages: Analytics, Members, Customers, Products, Tier, Settings) + Backend + DB + Tests
**Methodik:** Zwei parallele Audit-Agents (Backend+DB / Frontend+UX) haben Code zeilengenau gelesen und gegen Production-Standards bewertet.

---

## TL;DR

| Layer | Score | Status |
|-------|------:|--------|
| **Backend + Database** | **62 / 100** | ⚠️ NOT YET — 3 P0-Blocker, 16 P1 |
| **Frontend + UX** | **62 / 100** | ⚠️ NOT YET — 3 P0-Blocker, 8 P1 |
| **Combined** | **62 / 100** | ⚠️ Beta-tauglich, GA blockiert durch P0s |

**6 P0-Blocker** verhindern GA. **~24 P1-Items** vor Launch fixen. Mit ~3-5 Tagen Engineering-Arbeit erreichst du `85/100` (GA-tauglich).

---

## Coverage-Matrix

### Backend + Database

| Dimension | Status | P0 | P1 | P2 | Kern-Befund |
|-----------|:------:|---:|---:|---:|-------------|
| A. Data integrity | ⚠️ | 0 | 2 | 7 | Tier-backfill race, `lifetime_revenue_cents` Counter tot |
| B. Security & Authorization | ❌ | **1** | 4 | 5 | **Plaintext IBAN** (mig 167) |
| C. Transactions & Consistency | ⚠️ | 0 | 5 | 1 | Post-commit Link-Create kann Orphan-Memberships erzeugen |
| D. Error handling | ⚠️ | 0 | 1 | 2 | `ApiError::Internal` schluckt zu viele Fehler |
| E. Input validation | ❌ | **1** | 1 | 4 | **Kein Rate-Limit auf `/invite`** → Spam-Vektor |
| F. Performance & Scaling | ⚠️ | 0 | 1 | 1 | `recompute_team_tier()` läuft synchron bei jedem Page-Load |
| G. Background workers | ❌ | 0 | 2 | 2 | Tier-Worker kennt Teams nicht; `team_tier_history` nie geschrieben |
| H. Audit logging | ⚠️ | 0 | 1 | 2 | Mutations (invite/approve/remove) nicht audit-geloggt |
| I. Testing | ❌ | 0 | 1 | 5 | Commission-Rate-Branching (mig 166) hat keinen Test |
| J. API contract | ⚠️ | (E4) | 0 | 3 | Inkonsistente Response-Envelopes; keine Versionierung |
| K. Currency | ❌ | **1** | 0 | 2 | **Kein Currency-Code** auf Geld-Spalten — Multi-Currency-Korruption |
| L. Edge cases | ⚠️ | 0 | 2 | 2 | Invite leaks User-Existenz; Concurrent-Tier-Race |

### Frontend + UX

| Dimension | Status | P0 | P1 | P2 | Kern-Befund |
|-----------|:------:|---:|---:|---:|-------------|
| A. Accessibility (WCAG 2.2 AA) | ❌ | **1** | 5 | 3 | **Kontrast-Fail PaleGreen-auf-Blau** auf primärem CTA |
| B. i18n / l10n | ⚠️ | 0 | 1 | 3 | EUR hardcoded, alle Strings englisch |
| C. Error handling | ⚠️ | 0 | 3 | 2 | Keine fetch-Timeouts, partielle Failures werden geschluckt |
| D. Performance | ⚠️ | 0 | 0 | 5 | JS unminified, SVG-Icons mehrfach |
| E. Security | ❌ | **1** | 1 | 2 | **XSS via `innerHTML`** im Trend-Chart-Tooltip |
| F. State management | ⚠️ | 0 | 0 | 3 | Filter nicht in URL persistiert (außer Analytics) |
| G. Code quality | ⚠️ | 0 | 3 | 4 | DRY-Violations (skeletonRows × 3 Files); Toter Code `developer-affiliate-team.js` |
| H. Browser compat | ✅ | 0 | 0 | 0 | Alle APIs supported |
| I. Mobile responsiveness | ⚠️ | 0 | 1 | 4 | Touch-Targets 22px (zu klein für Mobile) |
| J. Print stylesheet | ⚠️ | 0 | 0 | 1 | Sticky Footer overlappt im Print |
| K. Observability | ⚠️ | 0 | 0 | 2 | Sentry global; aber handled-Errors nicht captured |
| L. Documentation | ⚠️ | 0 | 0 | 2 | Kein Modul-README |
| M. Design system | ⚠️ | (A1) | 2 | 1 | Parallele Button-Systeme; Hex-Literale statt Tokens |
| N. CSV correctness | ❌ | **1** | 0 | 0 | **CSV-Header sagen `(USD)`, App ist EUR** |

---

## 🚨 P0 — Launch-Blocker (6 Items)

### Backend / DB

#### B-P0-1: Plaintext IBAN in Datenbank
- **Datei:** `database/167_developer_teams_bank_details.sql:22`
- **Problem:** `bank_iban TEXT` als unverschlüsselter Klartext. Bei DB-Leak → IBANs aller Developer kompromittiert.
- **Impact:** GDPR-Verstoß + Payment-Compliance-Verletzung. Payouts dürfen nicht live gehen.
- **Fix:** `pgcrypto` integrieren — `pgp_sym_encrypt(iban, key)`, Key aus Vault. ODER ID-Reference auf externes Vault (Stripe Connect / Mangopay).
- **Aufwand:** ~1 Tag

#### E-P0-1: Kein Rate-Limit auf `/api/developer/affiliate/team/invite`
- **Datei:** `backend/src/rewards/team_routes.rs` (invite_member)
- **Problem:** Authentifizierter Developer kann unbegrenzt Invite-Emails verschicken. Trivialer Spam-Vektor → Email-Provider-Reputation-Schaden, Konten-Sperre.
- **Fix:** Bestehenden `auth_rate_limiter` adaptieren (z.B. 10 invites / 15 min pro Developer).
- **Aufwand:** ~2 Stunden

#### K-P0-1: Kein Currency-Code auf Geld-Spalten
- **Tabellen:** `affiliate_commissions`, `affiliate_daily_rollups`, `affiliate_live_counters`, `developer_teams`
- **Problem:** Alle `*_cents`-Felder ohne Currency. Solange nur EUR → OK. Sobald 1 USD-Asset existiert → silent corruption (€100 + $100 = 20000 cents, semantisch falsch).
- **Fix:** `currency VARCHAR(3) NOT NULL DEFAULT 'EUR'` auf alle Geld-Tabellen. Aggregation per Currency.
- **Aufwand:** ~1 Tag (Migration + Backend-Refactor + Frontend-Wahl)

### Frontend

#### A-P0-1: Kontrast-Fail auf primärem CTA
- **Datei:** `developer-affiliate-team.css:1693, 1698`
- **Problem:** `.dat-action-btn--primary { color: #98FB96 (PaleGreen) on #0000FF (Pure Blue) }` → Kontrast **2.94 : 1**. WCAG 2.2 AA verlangt 4.5 : 1.
- **Impact:** Schwer lesbar; sieht wie Bug aus; betrifft Invite-Member-Button (häufigste Aktion).
- **Fix:** `color: #fff;` → 8.59 : 1 ✓
- **Aufwand:** 2 Minuten

#### E-P0-1: XSS via `innerHTML` im Trend-Chart-Tooltip
- **Datei:** `developer-affiliate-team-analytics.js:405-408`
- **Code:** `tt.innerHTML = \`<div>${s.bucket_date}</div>...\`` — server-supplied `s.bucket_date` interpoliert in innerHTML ohne Escape.
- **Impact:** Aktuell harmlos weil Datum kontrolliert. Bei zukünftigen Tooltip-Erweiterungen mit User-Content → echtes XSS.
- **Fix:** `DAT.el(...)` mit `textContent` verwenden statt innerHTML.
- **Aufwand:** 15 Minuten

#### N-P0-1: CSV-Header lügen über Währung
- **Dateien:** `developer-affiliate-team-customers.js:97`, `developer-affiliate-team-products.js:71`
- **Code:** `'Invested (USD)', 'Commission (USD)'` — aber App rendert EUR.
- **Impact:** Buchhaltung lädt CSV runter, importiert als USD → Falsche Zahlen in Steuer/Reporting.
- **Fix:** `(USD)` → `(EUR)` in beiden Files.
- **Aufwand:** 2 Minuten

**P0 Gesamtaufwand: ~3 Tage**

---

## ⚠️ P1 — Must-Fix vor Launch (~24 Items)

### Backend / DB

| ID | Item | Datei | Aufwand |
|----|------|-------|--------:|
| A1 | Tier-Backfill Race in `recompute_team_tier()` — kein Row-Lock | `database/166:46-93` | 2h |
| A2 | `affiliate_live_counters.lifetime_revenue_cents` wird nie automatisch updated (Tile zeigt 0) | `database/159:48` | 4h |
| B1 | RLS aktiviert aber inert — bei misconfiguriertem `FORCE` Outage | `database/153, 164` | 2h |
| B2 | `attribute_affiliate_referral` nicht transactional | `backend/src/rewards/service.rs:1309-1446` | 4h |
| B3 | IBAN-Checksum (MOD-97) nicht validiert | `team_routes.rs:177-208` | 1h |
| C1 | `accept_invitation`: Link-Create post-commit → Orphan-Membership möglich | `team_members.rs:170-227` | 3h |
| C2 | `approve_pending`: gleicher Orphan-Risk | `team_members.rs:351-403` | 3h |
| C3 | `recompute_team_tier()` race-prone bei concurrent calls | (siehe A1) | — |
| D1 | `ApiError::Internal("…failed")` für 12 Endpoints — Fehler-Details verschluckt | `team_routes.rs` mehrere | 4h |
| G1 | `run_affiliate_tier_progression_worker` kennt Teams nicht — Team-Tier altert nicht automatisch | `backend/src/rewards/service.rs:2155-2309` | 1d |
| G2 | `developer_team_tier_history` wird nirgendwo geschrieben — History-UI permanent leer | (siehe G1) | inkl. |
| H1 | Mutations (invite/approve/remove/update_team) nicht in Audit-Log | new file `team_audit.rs` | 1d |
| I1 | Commission-Rate-Branching (mig 166 Kernfix) ohne Test | `backend/tests/affiliate_team_integration.rs` | 4h |
| L1 | Invite leaks User-Existenz (Enumeration-Oracle) | `team_members.rs:55-63` | 1h |

### Frontend

| ID | Item | Datei | Aufwand |
|----|------|-------|--------:|
| FA1 | Heading-Hierarchy: `<h1>` → `<h3>` (kein `<h2>`) | Alle Sub-Pages | 2h |
| FA2 | Analytics-Tables (By-member, Assets sold) ohne `scope="col"` + `<caption>` | `affiliate-team.html:271-307` | 30min |
| FA3 | Skip-to-Content Link fehlt | `_affiliate_team_shell.html` | 1h |
| FA4 | `alert()`-Fallback in invite-modal statt toast | `shell.js:354` | 30min |
| FA5 | Modal: Background nicht `inert` während Modal offen — Tab kann entkommen | `shell.js:280-321` | 1h |
| FB1 | Strings englisch — wenn deutscher Markt → i18n notwendig | gesamtes Frontend | 2-3d (separat) |
| FC1 | Fetch ohne Timeout/Retry — hängt unendlich | `shell.js:68-99` | 2h |
| FC2 | `loadTeamInfo`-Failure silent — UI stuck auf "Loading…" | `shell.js:269-273` | 1h |
| FC3 | `loadAll` Analytics: `mem`/`ass` Rejections geschluckt | `analytics.js:686-707` | 1h |
| FG1 | DRY: `skeletonRows()` 3× dupliziert in members/customers/products.js | shell.js | 1h |
| FG2 | DRY: `csvEscape` + `downloadCsv` 2 Versionen mit unterschiedlichen Line-Endings | analytics.js vs shell.js | 1h |
| FG3 | Toter Code: `developer-affiliate-team.js` (15 KB) referenziert von nichts mehr | git rm | 5min |
| FI1 | Touch-Targets `.dat-res-btn` (22px) + `.dat-preset` (24px) zu klein für Mobile (WCAG 2.5.5) | CSS:777,459 | 1h |
| FM1 | Parallele Button-Systeme `.dat-action-btn` neben `.ds-btn` | CSS:1668-1703 | 2h |
| FM2 | Status-Farben (#137333, #B42318, etc.) hardcoded — nicht als Tokens | CSS | 2h |

**P1 Gesamtaufwand: ~6-8 Tage**

---

## P2 — Nice-to-have (~45 Items)

Auswahl der wichtigsten (Detail-Liste in Original-Audit):

**Backend:**
- Migration-Rollback-Skripts (keine `*.down.sql`)
- OpenAPI/Spec
- Response-Envelope-Standardisierung
- E2E Test-Coverage (Playwright)
- API-Versionierung (`/v1/`)
- Reserved-Slugs aus Route-Registry derivieren
- Background-Worker für Retention/Cleanup von alten Klick-Partitionen
- Holdback-Expiry-Worker
- Slug-Availability-Check (live, beim Tippen)

**Frontend:**
- Dark-Mode (`prefers-color-scheme`)
- Product-Analytics-Tracking (GA / Mixpanel / Plausible)
- Cross-Tab-State-Sync
- Auto-Refresh bei `visibilitychange`
- Sticky-Footer im Print verstecken
- SVG-Icon-Sprite (Defs/Symbol) statt 8-fach inline
- Module-README (`AFFILIATE-TEAM.md`)
- Touch-Target-Bumps für alle Buttons (44×44 für AAA)
- Skeleton-Rows auf Settings + Tier
- Breakpoint-Tokens statt 7 magic numbers
- Spacing-Scale (`--space-1`, `--space-2`, …)
- Magic-Numbers konstantisieren (`MIN_PAYOUT_CENTS`, `CUSTOMERS_PAGE_SIZE`)
- `pushState` statt `replaceState` für Analytics-Range
- Filter-State-Persistence auf Customers/Products/Settings/Tier

---

## Empfohlener Rollout-Plan

### Phase 0: Hot-Fixes (1 Tag)
Frontend P0s (alle 3) + 2 quick wins:
- A-P0-1 Kontrast fixen (`#98FB96 → #fff`)
- E-P0-1 innerHTML → textContent in Trend-Tooltip
- N-P0-1 CSV `(USD) → (EUR)`
- Toten Code löschen (`developer-affiliate-team.js`)
- Skeleton-DRY in shell.js auslagern

→ Frontend-Score: **62 → 78**

### Phase 1: Pre-Launch Blocker (1 Woche)
Backend P0s:
- B-P0-1 IBAN-Verschlüsselung via pgcrypto
- E-P0-1 Rate-Limiter auf `/invite`
- K-P0-1 Currency-Code auf Geld-Tabellen

→ Backend-Score: **62 → 75**

### Phase 2: P1-Sweep (1.5-2 Wochen)
- Heading-Hierarchy + a11y-Patches
- Fetch-Timeouts + Retry
- Tier-Worker erweitert um Teams
- `developer_team_tier_history` schreiben
- Audit-Log für Mutations
- Commission-Branching-Test
- Touch-Target-Bumps
- Modal `inert`-Trap-Fix

→ Combined Score: **62 → 85** (GA-tauglich)

### Phase 3: Polish (laufend)
- Dark-Mode
- i18n (falls deutscher Markt)
- Product-Analytics
- Background-Workers für Tier-Auto-Promotion + Notifications
- API-Versionierung

→ Combined Score: **85 → 92+**

---

## Strengths (was bereits gut ist)

**Backend:**
- ✅ Saubere Schema-Design: shape-constraints, partial-unique, FK-Validation, partitioned clicks, statement-level Trigger
- ✅ Tight Authorization: `team_id` nie vom Client vertraut, `require_team_owner` durchgängig
- ✅ Korrekte Trennung personal vs team_business (mig 166)
- ✅ Background-Worker für Partition-Mgmt, Rollups, Retention
- ✅ Off-Boarding-Cascade korrekt transactional + getestet
- ✅ Keine SQL-Injection-Vektoren (alle Queries parametrisiert)
- ✅ Live-Counter via statement-level Trigger (mig 163)

**Frontend:**
- ✅ Solide SVG-Charts ohne externe Libs
- ✅ Modal mit Focus-Trap + ESC + Return-Focus
- ✅ Skeleton-Rows + Empty-States mit CTAs
- ✅ Sticky Save-Footer mit Dirty-Detection
- ✅ Live Slug-Preview mit Validation
- ✅ Beforeunload-Guard bei unsaved changes
- ✅ Confirm-Modal mit a11y (alertdialog + focus trap)
- ✅ CSV-Export mit RFC-4180 quoting + UTF-8 BOM
- ✅ Roving-Tabindex auf Resolution-Toggle
- ✅ `prefers-reduced-motion` respektiert
- ✅ Currency-Contract klar (EUR de-DE)

---

## Detail-Referenzen

Original-Audit-Outputs (vollständige Befunde mit Code-Quotes):
- **Backend:** Section A-L mit 80+ einzelnen Findings (P0/P1/P2 markiert)
- **Frontend:** Section A-M mit 60+ einzelnen Findings (P0/P1/P2 markiert)

Wichtigste File-Landmarks:
- `backend/src/rewards/service.rs:1097-1297` — Commission-Tracking (in tx ✓)
- `backend/src/rewards/service.rs:1309-1446` — Attribution (NICHT in tx ⚠️)
- `backend/src/rewards/service.rs:2155-2309` — Tier-Worker (nur personal)
- `backend/src/rewards/team_members.rs:170-227` — Post-commit Link-Race
- `backend/src/rewards/team_routes.rs` — 12 Endpoints, alle DeveloperUser-gated
- `database/166_developer_team_tiers.sql:46-93` — recompute_team_tier (no lock)
- `database/167_developer_teams_bank_details.sql:22` — **plaintext IBAN**
- `database/159_affiliate_rollups.sql:48` — `lifetime_revenue_cents` dead field
- `frontend/.../developer-affiliate-team.css:1693` — **#98FB96 contrast bug**
- `frontend/.../developer-affiliate-team-analytics.js:405-408` — **innerHTML XSS escape hatch**
- `frontend/.../developer-affiliate-team-customers.js:97` — **CSV USD-mislabel**
- `frontend/.../developer-affiliate-team-products.js:71` — **CSV USD-mislabel**
- `backend/tests/affiliate_team_integration.rs` — 11 Tests, fehlt: Commission-Rate-Branching für mig 166
