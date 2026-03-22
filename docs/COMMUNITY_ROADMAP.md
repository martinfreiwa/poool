# POOOL Community — Modular Implementation Roadmap

> **Source:** `docs/COMMUNITY_MASTERPLAN.md`
> **Strategy:** Modularer Aufbau — einfach starten, Stück für Stück erweitern
> **Last Sync:** 2026-03-23
> **Kernprinzip:** Jedes Modul ist **eigenständig launchbar**. Modul 1 allein liefert bereits echten Wert für Nutzer. Jedes weitere Modul erweitert, ohne das Bestehende zu brechen.

---

## 🧱 Modularer Aufbau — Übersicht

```
┌──────────────────────────────────────────────────────────────────────┐
│  POOOL COMMUNITY — MODULARE ARCHITEKTUR                              │
│                                                                      │
│  MODULE 1: Announcement Feed (MVP)        ← LAUNCH IN ~2 WOCHEN     │
│  ┌────────────────────────────────────┐                              │
│  │ • Admins posten Announcements       │                              │
│  │ • User sehen Feed                   │                              │
│  │ • User können reagieren (🔥💡👏🌱) │                              │
│  │ • User können kommentieren          │                              │
│  │ • Basis-Profile (Name, Avatar)      │                              │
│  └────────────────────────────────────┘                              │
│              ↓ funktioniert standalone                                │
│  MODULE 2: User-Generated Content         ← +1-2 WOCHEN             │
│  ┌────────────────────────────────────┐                              │
│  │ • User können eigene Posts erstellen │                              │
│  │ • Content Moderation (Keyword-Filter)│                              │
│  │ • Investment-Disclaimer              │                              │
│  │ • Bild-Upload in Posts               │                              │
│  └────────────────────────────────────┘                              │
│              ↓ baut auf Modul 1 auf                                  │
│  MODULE 3: Social Layer                   ← +1-2 WOCHEN             │
│  ┌────────────────────────────────────┐                              │
│  │ • Follow-System                     │                              │
│  │ • Persönlicher Feed (nicht nur global│                              │
│  │ • User-Profile mit Bio              │                              │
│  │ • Badges (Investment + Community)    │                              │
│  └────────────────────────────────────┘                              │
│              ↓ baut auf Modul 1+2 auf                                │
│  MODULE 4: Circles & XP                   ← +2 WOCHEN               │
│  ┌────────────────────────────────────┐                              │
│  │ • Circle-System (Referral Auto-Join) │                              │
│  │ • XP / Erfahrungspunkte             │                              │
│  │ • Level-System (Newcomer → Legend)   │                              │
│  │ • Circle-Leaderboard                │                              │
│  └────────────────────────────────────┘                              │
│              ↓ baut auf Modul 1+2+3 auf                              │
│  MODULE 5: Advanced Features              ← +2-3 WOCHEN             │
│  ┌────────────────────────────────────┐                              │
│  │ • Asset Reviews & Ratings            │                              │
│  │ • Expert AMAs                        │                              │
│  │ • Challenges                         │                              │
│  │ • Notifications                      │                              │
│  └────────────────────────────────────┘                              │
└──────────────────────────────────────────────────────────────────────┘
```

### Warum modular?

| Vorteil | Erklärung |
|---|---|
| **Schneller liefern** | Modul 1 (Announcement Feed) kann in ~2 Wochen live gehen → sofort Nutzer-Feedback |
| **Weniger Bugs** | Jedes Modul wird isoliert getestet, bevor das nächste draufgebaut wird |
| **Grundkonstrukt stabil** | Die Basis (Posts, Comments, Reactions) wird unter realer Last getestet, bevor komplexe Features (XP, Circles) dazukommen |
| **Flexibel priorisieren** | Zwischen Modulen kann umpriorisiert werden. Wenn Reviews wichtiger sind als Circles → Reihenfolge ändern |
| **Kein Alles-oder-Nichts** | Wenn Modul 4 Probleme macht, sind Module 1-3 trotzdem live und funktional |

---

## 🤖 Agent Collaboration Protocol

Same protocol as `IMPLEMENTATION_ROADMAP.md`. Agents **MUST**:
1. Check Module Gates below
2. Check File Zone conflicts in Live Agent Logs
3. Claim task (update status + assignee)
4. Read `AGENT_DEVELOPMENT_PROMPT.md` + `COMMUNITY_MASTERPLAN.md`
5. Execute, test, check-out

> [!IMPORTANT]
> **Status Key:** `❌ NOT STARTED` | `🔄 IN PROGRESS` | `⏸️ BLOCKED` | `⚪ NOT READY` | `✅ DONE`

---

## 📡 Live Agent Logs

| Date/Time (UTC) | Agent Name | Claimed Task ID | File Zone | Action / Status | Notes |
|:---|:---|:---|:---|:---|:---|
| `2026-03-21 06:17` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Created Community Masterplan + Roadmap |
| `2026-03-21 07:02` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Restructured Roadmap to modular approach |
| `2026-03-22 12:23` | `Antigravity` | `M0` | `db.rs`, `main.rs` | `✅ Check-Out` | Provisioned local Community DB, setup dual pool, Gate M1 is now OPEN |
| `2026-03-22 12:30` | `Antigravity` | `M1-DB` | `database/community/` | `✅ Check-Out` | Created M1 base tables (posts, comments, reactions, profiles) |
| `2026-03-22 21:05` | `Antigravity` | `M1-BE` | `backend/src/community/` | `✅ Check-Out` | Completed Announcement Feed MVP backend (models, service, cross-db queries, router) |
| `2026-03-22 21:10` | `Antigravity` | `M1-FE` | `frontend/platform/` | `✅ Check-Out` | Built Feed dynamic fetching, Filters, Comments UI logic, 'Coming Soon' overlays for unused modules |
| `2026-03-22 21:35` | `Antigravity` | `M1-ADMIN` | `backend/src/` & `admin/` | `✅ Check-Out` | Implemented Admin sidebar injection, Community Dashboard, Announcements Manager and KPI backend logic |
| `2026-03-22 21:40` | `Antigravity` | `M1-QA` | `backend/src/community/tests.rs` | `✅ Check-Out` | Implemented tests for models, validation logic, and ran successful pipeline checks. Modul 1 is fully READY for launch! |
| `2026-03-22 22:38` | `Antigravity` | `M2-FE`, `M2-BE`, `M2-DB` | `community/` & `platform/` | `✅ Check-Out` | Completed M2 User Posts backend, DB migrations, content moderation rules, and frontend post modalities |
| `2026-03-22 22:50` | `Antigravity` | `M2-ADMIN` | `community/` & `admin/` | `✅ Check-Out` | Implemented full Moderation workflow: Admin pending queue, Post viewer/hider, User management (bans), and API routes |
| `2026-03-22 23:00` | `Antigravity` | `M2-ADMIN`, `M2-FE` | `admin/`, `platform/` | `✅ Check-Out` | Implemented User moderation UI (Bans/Warnings), Admin Posts viewer, and dynamic Trending Assets Sidebar Widget |
| `2026-03-22 23:10` | `Antigravity` | `M2-BE` | `storage/`, `community/` | `✅ Check-Out` | Implemented Post Image Uploader, Redis Rate Limiter, and Asset Velocity Monitor protecting from Pump & Dump spam attacks |
| `2026-03-22 23:45` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Added Module 3.5 (Audit Fixes) and restructured Modules 4 & 5 based on comprehensive security & scalability audit. |
| `2026-03-23 00:00` | `Antigravity` | `M4-DB, M4-BE.1-6` | `community/xp.rs, circles.rs, routes.rs` | `✅ Check-Out` | M4 Phase 1 COMPLETE: 6 DB objects (circles, circle_members, circle_invites, xp_ledger, xp_levels, ALTER community_profiles). XP award system with daily caps. Circles CRUD + invite + auto-join. 18 API endpoints. XP aggregation + invite expiry workers. |
| `2026-03-23 00:15` | `Antigravity` | `M4-FE.1-5, M4-BE.8` | `community-circles.js, community.html` | `✅ Check-Out` | M4 Frontend COMPLETE: Replaced 'Coming Soon' overlay with dynamic API-wired Circle tab. XP summary card, circle management, member list, leaderboard, XP history, create/invite modals, level-up animation. |
| `2026-03-23 00:30` | `Antigravity` | `M4-BE.7,9,10` | `xp.rs, background.rs, auth/routes.rs` | `✅ Check-Out` | M4 Phase 2 COMPLETE: Login streak tracker (hooks into email + OAuth login), circle retry worker (30min), level-gated features (L2 circles, L3 invites). Streak badge in XP card. |
| `2026-03-23 00:50` | `Antigravity` | `M5-DB.2, M5-BE.2-3, M5-FE.2` | `amas.rs, routes.rs, community-amas.js, community.html` | `✅ Check-Out` | Expert AMAs COMPLETE: 3 DB tables (amas, ama_questions, ama_question_upvotes) + upvote trigger. 11 API endpoints (4 user, 7 admin). Dynamic AMA tab replaces Coming Soon overlay. Question submission, upvoting, expert answers with XP rewards. |
| `2026-03-23 01:15` | `Antigravity` | `M3-ADMIN.1-4` | `routes.rs, admin-sidebar-loader.js, badges.html, amas.html, user-detail.html, users.html` | `✅ Check-Out` | M3-ADMIN COMPLETE: Admin badge management page (CRUD + grant/revoke), admin AMA management page (create/status/answer/feature), user detail backend API, sidebar extended with Badges + Expert AMAs links. Users table now links to user detail. Fixed P1: require_auth -> get_current_user. |

---

## PHASE M0: Infrastructure Prerequisites

*These tasks live in the MAIN `IMPLEMENTATION_ROADMAP.md` — listed here for reference only.*

| ID | Task | Status in Main Roadmap | Notes |
|:---|:---|:---|:---|
| **0.2** | Cloud SQL Community DB Provisioning | `✅ DONE` | Done locally for dev. |
| **1.1** | Dual DB Pool Setup (`pool_community` in `db.rs`) | `✅ DONE` | Done locally for dev. |

> [!WARNING]
> **GATE:** Module 1 is now unblocked since BOTH 0.2 and 1.1 from the main roadmap are `✅ DONE`.

---

## 🟢 MODULE 1: Announcement Feed (MVP)

**Ziel:** Admins posten, User lesen + reagieren + kommentieren.
**Geschätzte Dauer:** ~2 Wochen
**Launch-fähig:** ✅ JA — dies allein liefert bereits echten Wert

> **Was User erleben:** Eine lebendige Community-Seite wo POOOL-Updates, Dividend-Nachrichten, neue Assets und Market News erscheinen. User können reagieren und kommentieren — wie ein internes Nachrichtenportal mit Social-Features.

### M1-DB: Database Migrations (nur das Nötigste)

| ID | Task | Tabellen | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M1-DB.1** | `posts` Tabelle | posts (nur post_type 'announcement' aktiv) | `✅ DONE` | Antigravity | `database/community/` |
| **M1-DB.2** | `comments` Tabelle | comments mit post FK | `✅ DONE` | Antigravity | `database/community/` |
| **M1-DB.3** | `reactions` Tabelle | reactions mit UNIQUE + Count-Trigger | `✅ DONE` | Antigravity | `database/community/` |
| **M1-DB.4** | `announcement_categories` Tabelle | Kategorien (new_commodity, dividend, etc.) | `✅ DONE` | Antigravity | `database/community/` |
| **M1-DB.5** | `community_profiles` Tabelle (Basis) | Nur user_id + post_count + ban-Felder — KEINE XP/Level/Circle Felder | `✅ DONE` | Antigravity | `database/community/` |
| **M1-DB.6** | Basis-Indexes | idx_posts_created_at, idx_comments_post_id, idx_reactions_post_id | `✅ DONE` | Antigravity | `database/community/` |

> **Wichtig:** Module 1 braucht nur **5 Tabellen** (posts, comments, reactions, announcement_categories, community_profiles). KEINE follows, badges, circles, xp_ledger, reviews, amas, challenges. Diese kommen in späteren Modulen über `ALTER TABLE` Migrationen dazu.

### M1-BE: Backend (Rust)

| ID | Task | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M1-BE.1** | Module Skeleton | `community/mod.rs`, `models.rs`, `routes.rs`, `service.rs`, `validation.rs` — Grundstruktur | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.2** | Models (Basis) | `Post`, `Comment`, `Reaction`, `CommunityProfile` structs | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.3** | User Bridge (Basis) | Batch user lookup (Name + Avatar) von Core-DB + Redis Cache | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.4** | Validation (Basis) | Comment-Länge (1-2000 chars), Rate Limits (30 comments/hour) | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.5** | Announcements CRUD (Admin-only) | `POST /api/admin/community/announcements` — nur Admins können Posts erstellen | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.6** | Feed API (Read-only) | `GET /api/community/feed` — paginated, chronologisch, nur Announcements | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.7** | Reactions API | `POST /api/community/posts/{id}/reactions` — toggle (🔥💡👏🌱), denormalized count | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.8** | Comments API | `GET /POST /DELETE /api/community/posts/{id}/comments` — User können kommentieren | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.9** | Announcements Filter | `GET /api/community/announcements?category=dividend` — Kategorie-Filter | `✅` | Antigravity | `backend/src/community/` |
| **M1-BE.10** | Route Registration | Alle Modul-1 Routes in `main.rs` registrieren | `✅` | Antigravity | `backend/src/main.rs` ⚠️ |

**API-Endpunkte Modul 1:** ~8 Endpunkte

| Method | Path | Wer | Beschreibung |
|---|---|---|---|
| `POST` | `/api/admin/community/announcements` | Admin | Announcement erstellen |
| `PUT` | `/api/admin/community/announcements/{id}/pin` | Admin | Announcement pinnen |
| `DELETE` | `/api/admin/community/posts/{id}` | Admin | Post löschen |
| `GET` | `/api/community/feed` | Alle | Announcements-Feed (paginiert) |
| `GET` | `/api/community/announcements?category=` | Alle | Gefilterte Announcements |
| `POST` | `/api/community/posts/{id}/reactions` | User | Reaction toggle |
| `GET` | `/api/community/posts/{id}/comments` | User | Kommentare laden |
| `POST` | `/api/community/posts/{id}/comments` | User | Kommentar erstellen |

### M1-FE: Frontend (Platform)

| ID | Task | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M1-FE.1** | Feed-Tab dynamisch | `community-feed.js`: Demo-Daten durch `fetch()` ersetzen, Announcement-Cards rendern | `✅` | Antigravity | `frontend/platform/static/js/` |
| **M1-FE.2** | Reactions UI | Click-Handler für 🔥💡👏🌱, optimistic toggle, Counter-Update | `✅` | Antigravity | `frontend/platform/static/js/` |
| **M1-FE.3** | Comments UI | Kommentar-Sektion unter Posts, Comment-Input, Delete (eigene) | `✅` | Antigravity | `frontend/platform/static/js/` |
| **M1-FE.4** | Announcements-Tab dynamisch | `community-announcements.js`: Kategorie-Filter (All/Dividends/Platform/Market) | `✅` | Antigravity | `frontend/platform/static/js/` |
| **M1-FE.5** | Empty States + Loading | Skeleton-Loader, "Noch keine Announcements" Zustand | `✅` | Antigravity | `frontend/platform/static/css/` |
| **M1-FE.6** | Disabled UI für nicht-verfügbare Tabs | My Circle, Expert AMAs, Reviews → "Coming Soon" overlay | `✅` | Antigravity | `frontend/platform/static/js/` |

### M1-ADMIN: Admin Dashboard (2 Seiten)

> **Analog:** Marketplace hat 11 Admin-Seiten in `admin/marketplace/`. Community startet mit 2 und wächst modular auf 12.

| ID | Task | Seite | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|:---|
| **M1-ADMIN.1** | Admin Sidebar: Community-Sektion | - | Neue "🫂 Community" Nav-Section in `admin-sidebar-loader.js` (nur Overview + Announcements sichtbar) | `✅` | Antigravity | `frontend/platform/static/js/` ⚠️ |
| **M1-ADMIN.2** | `community/index.html` | Community Dashboard | KPI-Karten (Posts, Comments, Reactions, Aktive User), Letzte Aktivität, Quick Actions | `✅` | Antigravity | `frontend/platform/admin/community/` |
| **M1-ADMIN.3** | `community/announcements.html` | Announcements verwalten | Announcement erstellen (Titel, Content, Kategorie, Bilder), Pin/Unpin, archivieren, Vorschau | `✅` | Antigravity | `frontend/platform/admin/community/` |
| **M1-ADMIN.4** | Admin Community Stats API | `GET /api/admin/community/stats` — KPI-Daten liefern | `✅` | Antigravity | `backend/src/admin/` |

### M1-QA: Tests für Modul 1

| ID | Task | Description | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M1-QA.1** | Unit Tests: Reactions | Toggle-Logik, Count-Konsistenz, Duplicate-Prevention | `✅` | Antigravity | `backend/src/community/tests/` |
| **M1-QA.2** | Unit Tests: Comments | Länge-Validierung, XSS-Prevention (Ammonia) | `✅` | Antigravity | `backend/src/community/tests/` |
| **M1-QA.3** | Integration: Full Flow | Admin postet → User sieht → User reagiert → User kommentiert | `✅` | Antigravity | `backend/src/community/tests/` |
| **M1-QA.4** | E2E: Browser Test | Community-Seite laden, Announcement sehen, reagieren, kommentieren | `✅` | Antigravity | `tests/` |

> **🚀 LAUNCH-GATE:** Modul 1 kann **live gehen** wenn ALLE M1 Tasks `✅ DONE` sind.
> **Was sieht der User?** Announcements-Feed mit Reactions und Comments. Die anderen Tabs zeigen "Coming Soon".

---

## 🟡 MODULE 2: User-Generated Content

**Ziel:** User können eigene Posts erstellen (nicht nur Admins).
**Voraussetzung:** Modul 1 `✅ DONE`
**Geschätzte Dauer:** +1-2 Wochen
**Warum jetzt?** Content Moderation ist KRITISCH — ohne Keyword-Filter und Sanitization ist die Plattform angreifbar. Deshalb kommt Moderation VOR User-Posts.

### M2-DB: Zusätzliche Migrationen

| ID | Task | Beschreibung | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M2-DB.1** | `content_reports` Tabelle | Report-System für User-gemeldete Inhalte | `✅ DONE` | `database/community/` |
| **M2-DB.2** | `ALTER TABLE posts` | post_type CHECK erweitern: + 'general', 'market_insight' | `✅ DONE` | `database/community/` |
| **M2-DB.3** | Bild-Upload Indexes | idx_posts_user_id, idx_posts_post_type | `✅ DONE` | `database/community/` |

### M2-BE: Backend

| ID | Task | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M2-BE.1** | Content Moderation (`moderation.rs`) | Ammonia HTML-Sanitizer, Keyword-Filter ("guaranteed returns", "risk-free"), Spam-Detection, Investment-Disclaimer | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.2** | User Posts CRUD | `POST /api/community/posts` — User-Posts mit Moderation-Pipeline (Sanitize → Keyword-Check → Disclaimer → Save) | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.3** | Post Edit/Delete (eigene) | `PUT /DELETE /api/community/posts/{id}` — nur innerhalb 15 Minuten editierbar, nur eigene löschbar | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.4** | Content Report API | `POST /api/community/posts/{id}/report` — User meldet Post | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.5** | Image Upload | Upload bis zu 4 Bilder pro Post via GCS, Validierung (Dateityp, Größe <5MB) | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.6** | Admin Moderation API | `GET /api/admin/community/reports` + `POST .../action` — Reports bearbeiten, Posts verstecken/löschen | `✅ DONE` | Antigravity | `backend/src/admin/` |
| **M2-BE.7** | Post Rate Limiting | Redis-basiert: max 5 Posts/Stunde, Duplicate-Detection | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.8** | New-User Sandbox & URL Filter | Enforce rule: Users under Level 2 cannot post URLs. Regex detection for "guaranteed return" variations auto-flags posts. | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M2-BE.9** | Asset Velocity Monitor | Background worker monitoring post velocity. If >5 mentions of an asset in 10 mins, alert Admins (Pump & Dump protection). | `✅ DONE` | Antigravity | `backend/src/community/` |

### M2-FE: Frontend (Platform)

| ID | Task | Description | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M2-FE.1** | "Create Post" Component | Post-Erstellungs-UI: Textarea, Bild-Upload, Post-Type Selector | `✅ DONE` | `frontend/platform/static/js/` |
| **M2-FE.2** | Post Report Button | "Report" Flag auf jedem Post, Report-Reason Modal | `✅ DONE` | `frontend/platform/static/js/` |
| **M2-FE.3** | Disclaimer Banner | Automatischer Disclaimer unter Investment-bezogenen Posts (CSS + JS) | `✅ DONE` | `frontend/platform/static/css/` |
| **M2-FE.4** | Trending Assets Widget | Sidebar widget showing top 3 most-discussed assets in the last 24h, linking to trading page. | `✅ DONE` | `frontend/platform/static/js/` |

### M2-ADMIN: Admin Dashboard (+4 Seiten → gesamt 6)

| ID | Task | Seite | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|:---|
| **M2-ADMIN.1** | `community/posts.html` | Posts verwalten | Tabelle aller Posts (Suche, Filter nach Typ/Autor/Status, Bulk-Aktionen: hide/delete/warn) | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M2-ADMIN.2** | `community/post-detail.html` | Post-Detail | Einzelner Post mit allen Comments, Reactions, Report-History, Moderation-Aktionen | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M2-ADMIN.3** | `community/reports.html` | Moderation Queue | Pending Reports Tabelle, Quick-Actions (hide/delete/warn/ban), Report-Detail-View, 🔴 Badge in Sidebar | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M2-ADMIN.4** | `community/users.html` | Community Users | User-Tabelle (Post-Count, Warnings, Ban-Status), Quick-Actions (warn/ban/unban), Suche | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M2-ADMIN.5** | Admin Sidebar erweitern | - | Posts, Reports (mit 🔴 Badge), Community Users in Sidebar einfügen | `✅ DONE` | Antigravity | `frontend/platform/static/js/` ⚠️ |
| **M2-ADMIN.6** | Admin Moderation APIs | Backend | `GET/POST /api/admin/community/reports`, `POST .../posts/{id}/hide`, `POST .../users/{id}/ban` | `✅ DONE` | Antigravity | `backend/src/admin/` |

### M2-QA: Tests

| ID | Task | Description | Status |
|:---|:---|:---|:---|
| **M2-QA.1** | XSS Prevention | `<script>alert('xss')</script>` in Posts → sanitized output | `⚪` |
| **M2-QA.2** | Keyword Filter | "Guaranteed 28% returns" → auto-flagged + disclaimer | `⚪` |
| **M2-QA.3** | Rate Limit | 6. Post in einer Stunde → 429 Too Many Requests | `⚪` |
| **M2-QA.4** | Report Flow | User meldet Post → Admin sieht Report → Admin versteckt Post | `⚪` |

---

## 🟠 MODULE 3: Social Layer

**Ziel:** Follow-System, persönlicher Feed, Badges.
**Voraussetzung:** Modul 2 `✅ DONE`
**Geschätzte Dauer:** +1-2 Wochen

### M3-DB: Zusätzliche Migrationen

| ID | Task | Beschreibung | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M3-DB.1** | `follows` Tabelle | Unidirektionaler Follow mit self-follow CHECK | `✅ DONE` | `database/community/` |
| **M3-DB.2** | `badges` + `user_badges` Tabellen | Badge-Definitionen + Earned-Badges pro User | `✅ DONE` | `database/community/` |
| **M3-DB.3** | `ALTER TABLE community_profiles` | + bio, follower_count, following_count Felder | `✅ DONE` | `database/community/` |
| **M3-DB.4** | Seed: Badge Definitions | INSERT 16 Badge-Definitionen (Masterplan §2.3) | `✅ DONE` | `database/community/` |

### M3-BE: Backend

| ID | Task | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M3-BE.1** | Follow API | `POST/DELETE /api/community/follow/{user_id}`, Follower/Following-Listen | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.2** | Personal Feed | `GET /api/community/feed` — jetzt mit Follow-Boost (Scoring-Algorithmus) | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.3** | Profile API | `GET /api/community/profile/{id}` — Bio, Badges, Post-Count, Follower-Count | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.4** | Profile Edit | `PUT /api/community/profile` — Bio bearbeiten | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.5** | Badge Worker | Background-Worker: alle 6h Badges berechnen (Core-DB Investments + Community-DB Stats) | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.6** | Milestone Posts (Auto) | System-generierte Posts: "🎉 Sarah hat ihr 5. Investment getätigt!" | `✅ DONE` | Antigravity | `backend/src/community/` |
| **M3-BE.7** | Dynamic Asset-Owner Tags | Cross-DB check: If post content contains asset name, query Core DB to append `[Verified Owner]` tag if holding balance > 0. | `✅ DONE` | Antigravity | `backend/src/community/` |

### M3-FE: Frontend (Platform)

| ID | Task | Description | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M3-FE.1** | Follow-Button | Follow/Unfollow Button auf jedem User-Profil + Post-Header | `✅ DONE` | `frontend/platform/static/js/` |
| **M3-FE.2** | User Profile Modal | Click auf Username → Modal mit Bio, Badges, Posts, Follow-Button | `✅ DONE` | `frontend/platform/static/js/` |
| **M3-FE.3** | Feed Toggle | "All Posts" / "Following" Toggle im Feed | `✅ DONE` | `frontend/platform/static/js/` |
| **M3-FE.4** | Badge Display | Badges auf Profilen und neben Usernamen in Posts | `✅ DONE` | `frontend/platform/static/css/` |
| **M3-FE.5** | First-Time Onboarding UI | "Welcome" checklist modal encouraging users to set a bio, leave a comment, and earn their first 50 XP. | `✅ DONE` | `frontend/platform/static/js/` |

### M3-ADMIN: Admin Dashboard (+2 Seiten → gesamt 8)

| ID | Task | Seite | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|:---|
| **M3-ADMIN.1** | `community/user-detail.html` | User-Detail | Vollständiges Community-Profil: Posts, XP, Badges, Moderation-History, Warn/Ban Buttons | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M3-ADMIN.2** | `community/badges.html` | Badge-Verwaltung | Badge-Definitionen, Badge manuell an User vergeben, Badge-Statistiken | `✅ DONE` | Antigravity | `frontend/platform/admin/community/` |
| **M3-ADMIN.3** | Admin Sidebar erweitern | - | + Badges in Sidebar einfügen | `✅ DONE` | Antigravity | `frontend/platform/static/js/` ⚠️ |
| **M3-ADMIN.4** | Admin Badge APIs | Backend | `GET/POST/PUT /api/admin/community/badges`, `POST .../users/{id}/badge` | `✅ DONE` | Antigravity | `backend/src/admin/` |

---

## 🛡️ MODULE 3.5: Post-Audit Fixes (Security & Architecture)

**Ziel:** Address critical P0/P1 vulnerabilities identified in the March 2026 audit before proceeding to M4.

| ID | Task | Description | Status | Assignee | Priority |
|:---|:---|:---|:---|:---|:---|
| **FIX-F1** | XSS in Feed | `textContent` for user content in `community-feed.js`, no `innerHTML` | `✅ DONE` | Antigravity | **P0** |
| **FIX-F2** | XSS in Comments | `textContent` for comment content in `community-feed.js` | `✅ DONE` | Antigravity | **P0** |
| **FIX-F3** | XSS in Announcements | `textContent` for announcement content in `community-announcements.js` | `✅ DONE` | Antigravity | **P0** |
| **FIX-F7** | Ban Bypass | Add `check_user_not_banned()` middleware to all write routes | `✅ DONE` | Antigravity | **P1** |
| **FIX-F6** | Race Condition | Wrap `toggle_reaction` in DB transaction | `✅ DONE` | Antigravity | **P1** |
| **FIX-F4** | Verified Owner HTML Inj | Move HTML badge generation to frontend, add boolean flag to payload | `✅ DONE` | Antigravity | **P1** |
| **FIX-F5** | Missing Auth | Add `CookieJar` auth check to Trending Assets endpoint | `✅ SAFE` | Antigravity | **P1** |
| **FIX-F9** | Missing Redis Cache | Cache user bridge lookups (5min TTL) | `✅ DONE` | Antigravity | P2 |
| **FIX-CRL** | Comment Rate Limiting | Rate limiting for comments (Redis, 30/h) | `✅ DONE` | Antigravity | P2 |


## ✅ MODULE 4: Circles & XP (Phase 1 & 2) — COMPLETE

**Ziel:** Circle-System mit Referral-Integration, XP, Levels.
**Voraussetzung:** Modul 3 `✅ DONE`
**Status:** `✅ DONE` — 15/15 backend + frontend tasks complete

### M4-DB: Database Migrations
`008_circles_xp.sql` applied — 5 new tables + ALTER community_profiles

### M4-Phase 1: MVP Foundation (Circles + XP Core)

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M4-BE.4** | XP Award Service | Backend | `008_circles_xp.sql` | `✅ DONE` | Agent |
| **M4-BE.6** | XP Aggregation Worker (5-min) | Backend | M4-BE.4 | `✅ DONE` | Agent |
| **M4-BE.1** | Circle CRUD API | Backend | `008_circles_xp.sql` | `✅ DONE` | Agent |
| **M4-BE.3** | Circle Invite/Admin API | Backend | M4-BE.1 | `✅ DONE` | Agent |
| **M4-BE.2** | Referral Signup Auto-Join Hook | Backend | M4-BE.1 | `✅ DONE` | Agent |
| **M4-BE.5** | XP API (summary, history) | Backend | M4-BE.4 | `✅ DONE` | Agent |
| **M4-FE.1** | My Circle Tab UI (member list, invite) | User | M4-BE.1/3 | `✅ DONE` | Agent |
| **M4-FE.2** | XP Display (header badge + progress) | User | M4-BE.5 | `✅ DONE` | Agent |
| **M4-FE.3** | XP History page/section | User | M4-BE.5 | `✅ DONE` | Agent |

### M4-Phase 2: Growth & Engagement

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M4-BE.8** | Leaderboard API (circles + users) | Backend | M4-BE.6 | `✅ DONE` | Agent |
| **M4-FE.4** | Circle Leaderboard UI (top 20) | User | M4-BE.8 | `✅ DONE` | Agent |
| **M4-FE.5** | Level-Up Animation (CSS + JS toast) | User | M4-BE.6 | `✅ DONE` | Agent |
| **M4-BE.9** | Login Streak Tracker | Backend | M4-BE.4 | `✅ DONE` | Agent |
| **M4-BE.7** | Circle Retry Worker (failed auto-joins) | Backend | M4-BE.2 | `✅ DONE` | Agent |
| **M4-BE.10** | Level-gated feature enforcement | Backend | M4-BE.6 | `✅ DONE` | Agent |
| **M4-ADMIN.1**| Admin: Circles Overview page | Admin | M4-BE.1 | `❌` | - |
| **M4-ADMIN.2**| Admin: Leaderboard Management page| Admin | M4-BE.8 | `❌` | - |
| **M4-ADMIN.4**| Admin: Circle & XP APIs | Admin | M4-ADMIN.1/2| `❌` | - |
| **M4-ADMIN.3**| Admin Sidebar: Circles + Leaderboard| Admin | M4-ADMIN.1 | `❌` | - |
| **M2-ADMIN.7**| Admin Audit Log table + inserts | Admin | None | `❌` | - |

---

## 🟣 MODULE 5: Advanced Features (Phase 3)

**Ziel:** Reviews, AMAs, Challenges, Notifications, SEO.
**Voraussetzung:** Modul 3 `✅ DONE` ← Ready to start
**Geschätzte Dauer:** +2-3 Wochen

### M5-DB: Zusätzliche Migrationen

| ID | Task | Category | Status |
|:---|:---|:---|:---|
| **M5-DB.1** | `reviews` table migration | Backend | `❌` |
| **M5-DB.2** | `amas` + `ama_questions` + upvotes | Backend | `✅ DONE` |
| **M5-DB.3** | `challenges` + `challenge_progress` | Backend | `❌` |

### M5-Tasks: Features & System

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M5-BE.1** | Reviews API (CRUD + verified check) | Backend | M5-DB.1 | `❌` | - |
| **M5-FE.1** | Reviews Tab UI | User | M5-BE.1 | `❌` | - |
| **M5-BE.2** | AMAs API (Q&A + upvoting) | Backend | M5-DB.2 | `✅ DONE` | Antigravity |
| **M5-BE.3** | Admin AMA Management API | Admin | M5-BE.2 | `✅ DONE` | Antigravity |
| **M5-FE.2** | Expert AMAs Tab UI | User | M5-BE.2 | `✅ DONE` | Antigravity |
| **M5-BE.4** | Challenges API | Backend | M5-DB.3, M4-BE.1 | `❌` | - |
| **M5-FE.3** | Challenges UI | User | M5-BE.4 | `❌` | - |
| **M5-BE.5** | In-App Notification System | Backend | None | `❌` | - |
| **M5-BE.7** | SSR Post Pages (MiniJinja, SEO) | Backend | None | `❌` | - |
| **M5-BE.6** | Weekly Digest Worker (inactive users)| Backend | M5-BE.5 | `❌` | - |
| **M5-ADMIN.1**| Admin: AMA Management page | Admin | M5-BE.3 | `✅ DONE` | Antigravity |
| **M5-ADMIN.2**| Admin: Challenges page | Admin | M5-BE.4 | `❌` | - |
| **M2-ADMIN.2**| backlog: Admin Post Detail page | Admin | None | `✅ DONE` | Antigravity |
| **M3-ADMIN.1**| backlog: Admin User Detail page | Admin | None | `✅ DONE` | Antigravity |
| **M3-ADMIN.2**| backlog: Admin Badge Management | Admin | None | `✅ DONE` | Antigravity |
| **M5-ADMIN.3**| Admin Sidebar finalize | Admin | All admin | `✅ DONE` | Antigravity |

---

## 🚦 Module Gate Table

| Module | Name | Gate Status | Prerequisite | Can Start When | Geschätzte Dauer |
|:---|:---|:---|:---|:---|:---|
| **M0** | Infrastructure | `✅ OPEN` | Main Roadmap 0.2 + 1.1 | Both `✅ DONE` | 1-2 Tage |
| **M1** | Announcement Feed (MVP) | `🟢 OPEN` | M0 | M0 `✅ DONE` | **~2 Wochen** |
| **M2** | User-Generated Content | `🔒 LOCKED` | M1 | M1 ALL `✅` | +1-2 Wochen |
| **M3** | Social Layer | `🔒 LOCKED` | M2 | M2 ALL `✅` | +1-2 Wochen |
| **M4** | Circles & XP | `🔒 LOCKED` | M3 | M3 ALL `✅` | +2 Wochen |
| **M5** | Advanced Features | `🔒 LOCKED` | M3 (nicht M4!) | M3 ALL `✅` | +2-3 Wochen |

> **Paralleles Arbeiten möglich:**
> - **M4 + M5** können parallel gebaut werden (beide bauen auf M3 auf)
> - **M5-BE.4 (Challenges)** braucht M4, aber der Rest von M5 nicht

---

## 📊 Was der User nach jedem Modul sieht

### Nach Modul 1 (Launch):
```
USER-FACING:
  ✅ Announcements-Feed mit POOOL Updates
  ✅ Reactions (🔥💡👏🌱) auf Announcements
  ✅ Kommentare unter Announcements
  ✅ Kategorie-Filter (Dividends, Platform, Market News)
  🔒 "Create Post" → nicht sichtbar (nur Admins)
  🔒 "My Circle" / "Expert AMAs" / "Reviews" → "Coming Soon"

ADMIN-DASHBOARD (2 Seiten):
  ✅ /admin/community/ — Community Dashboard (KPIs, Quick Actions)
  ✅ /admin/community/announcements — Announcements erstellen & verwalten
```

### Nach Modul 2:
```
USER-FACING:
  ✅ Alles von Modul 1
  ✅ User können eigene Posts erstellen
  ✅ Posts mit Bildern (max 4)
  ✅ Investment-Disclaimer automatisch
  ✅ Posts melden (Report-Button)

ADMIN-DASHBOARD (+4 = 6 Seiten):
  ✅ /admin/community/posts — Posts suchen, filtern, Bulk-Aktionen
  ✅ /admin/community/post-detail — Einzelner Post moderieren
  ✅ /admin/community/reports — Moderation Queue (🔴 Badge)
  ✅ /admin/community/users — Community-User verwalten (Warn/Ban)
```

### Nach Modul 3:
```
USER-FACING:
  ✅ Alles von Modul 1+2
  ✅ Follow-Button auf Usern
  ✅ Persönlicher Feed (Following/All toggle)
  ✅ User-Profile mit Bio + Badges
  ✅ Investment-Badges (First Investor, Diversified, etc.)
  ✅ Milestone-Posts ("Sarah hat ihr 5. Investment getätigt!")

ADMIN-DASHBOARD (+2 = 8 Seiten):
  ✅ /admin/community/user-detail — Vollständiges User-Profil (Moderation)
  ✅ /admin/community/badges — Badge-Verwaltung & manuell vergeben
```

### Nach Modul 4:
```
USER-FACING:
  ✅ Alles von Modul 1+2+3
  ✅ "My Circle" Tab aktiv
  ✅ Circle-Members + Stats
  ✅ Referral-Link = automatischer Circle-Beitritt
  ✅ XP für jede Aktion (Post = 10 XP, Comment = 5 XP, etc.)
  ✅ User-Level (Newcomer → Legend)
  ✅ Circle-Leaderboard

ADMIN-DASHBOARD (+2 = 10 Seiten):
  ✅ /admin/community/circles — Circles verwalten, Anomalien erkennen
  ✅ /admin/community/leaderboard — Rankings prüfen, XP-Adjustments
```

### Nach Modul 5:
```
USER-FACING:
  ✅ Alles von Modul 1+2+3+4
  ✅ "Reviews" Tab aktiv (nur Verified Investors)
  ✅ "Expert AMAs" Tab aktiv
  ✅ Circle-Challenges
  ✅ In-App Notifications für Community-Events

ADMIN-DASHBOARD (+2 = 12 Seiten — KOMPLETT):
  ✅ /admin/community/amas — AMAs erstellen & verwalten
  ✅ /admin/community/challenges — Challenges erstellen & verwalten
```

---

## 🚀 MODULE 6: Advanced Engagement (Future Expansions)

**Ziel:** Tiefergehende Social-Features und Bettermode-ähnliche Funktionalität.
**Voraussetzung:** Modul 5 `✅ DONE`
**Geschätzte Dauer:** Offen (Phase 3 des Masterplans)

Diese Features erweitern das Kern-System um professionelle Community-Builder-Tools:

| ID | Task | Description | Status | 
|:---|:---|:---|:---|
| **M6-FEAT.1** | Spaces / Sub-Communities | Eigene Channels erstellen (z.B. "Cocoa Farm Investors", "Beginners") anstatt nur eines globalen Feeds. | `❌` |
| **M6-FEAT.2** | Ideation / Feedback Boards | Ein spezielles Board wo User Feature-Requests für die POOOL-Plattform einreichen und upvoten können. | `❌` |
| **M6-FEAT.3** | Rich Media Embeds | Unterstützung für YouTube, Loom und Figma Embeds in Posts (erfordert strikte CSP und Sandbox-Sicherheit). | `❌` |
| **M6-FEAT.4** | Global Member Directory | Ein durchsuchbares Verzeichnis aller Community-Nutzer mit Filtern für Interessen und Locations. | `❌` |
| **M6-FEAT.5** | Direct Messaging (DMs) | 1-to-1 Chats zwischen Usern (streng reguliert per Follow-Verification zur Scam-Prävention). | `❌` |
| **M6-FEAT.6** | Event RSVPs & Calendars | Ein Kalender-System für Live-AMAs, Webinare und Offline-Meetups inkl. RSVP-Tracking. | `❌` |

---

## 📂 File Zone Ownership Matrix

| File Zone | Description | Module |
|:---|:---|:---|
| `database/community/` | Community DB migrations | M1-M5 |
| `backend/src/community/mod.rs` | Module registration | M1 |
| `backend/src/community/models.rs` | Data structs (erweiterbar pro Modul) | M1-M5 |
| `backend/src/community/routes.rs` | API handlers (erweiterbar pro Modul) | M1-M5 |
| `backend/src/community/service.rs` | Business logic | M1-M5 |
| `backend/src/community/validation.rs` | Input validation | M1 |
| `backend/src/community/moderation.rs` | Content moderation | M2 |
| `backend/src/community/user_bridge.rs` | Cross-DB user lookup | M1 |
| `backend/src/community/xp.rs` | XP system | M4 |
| `backend/src/community/circles.rs` | Circle system | M4 |
| `backend/src/community/background.rs` | Background workers | M3-M4 |
| `backend/src/admin/community.rs` | Admin community APIs | M1-M5 |
| `backend/src/main.rs` | Route + worker registration | M1-M4 ⚠️ SHARED |
| `backend/src/auth/` | Signup hook for auto-circle-join | M4 ⚠️ CROSS-MODULE |
| `frontend/platform/community.html` | Community page template | M1 |
| `frontend/platform/static/js/community-*.js` | JS modules (1 pro Tab) | M1-M5 |
| `frontend/platform/static/js/admin-sidebar-loader.js` | Admin sidebar (Community-Sektion) | M1-M5 ⚠️ SHARED |
| `frontend/platform/static/css/community*.css` | Styles | M1-M5 |
| `frontend/platform/admin/community/index.html` | Community Dashboard | M1 |
| `frontend/platform/admin/community/announcements.html` | Announcements verwalten | M1 |
| `frontend/platform/admin/community/posts.html` | Posts verwalten | M2 |
| `frontend/platform/admin/community/post-detail.html` | Post-Detail | M2 |
| `frontend/platform/admin/community/reports.html` | Moderation Queue | M2 |
| `frontend/platform/admin/community/users.html` | Community Users | M2 |
| `frontend/platform/admin/community/user-detail.html` | User-Detail | M3 |
| `frontend/platform/admin/community/badges.html` | Badge-Verwaltung | M3 |
| `frontend/platform/admin/community/circles.html` | Circles-Übersicht | M4 |
| `frontend/platform/admin/community/leaderboard.html` | Leaderboard-Verwaltung | M4 |
| `frontend/platform/admin/community/amas.html` | AMA-Verwaltung | M5 |
| `frontend/platform/admin/community/challenges.html` | Challenge-Verwaltung | M5 |

---

## 🗓️ Timeline (Geschätzt)

```
WOCHE    MODUL     WAS PASSIERT
─────────────────────────────────────────────────────────────────
W1       M0        Infra: Community-DB aufsetzen, Dual Pool
W2-3     M1        MVP: Announcements + Reactions + Comments
         ──────── 🚀 LAUNCH: Community geht live! ────────────
W4-5     M2        User-Posts: Moderation, Create Post, Reports
W5-6     M3        Social: Follows, Profiles, Badges
         ──────── 🔄 Ab hier parallel möglich ─────────────────
W7-8     M4 ║ M5a  Circles+XP ║ Reviews+AMAs (parallel)
W8-9     M4 ║ M5b  Leaderboard ║ Challenges, Notifications
```

**Gesamtdauer: ~8-9 Wochen**, aber **Launch nach nur 3 Wochen!**

---

## ⚠️ Critical Warnings

> [!CAUTION]
> **Modul 1 darf KEINE User-Post-Erstellung haben.** Nur Admins posten in Modul 1. Content Moderation (Modul 2) MUSS existieren, bevor User posten dürfen — regulatorisches Risiko!

> [!CAUTION]
> **`backend/src/main.rs` ist ein Bottleneck-File.** Jedes Modul fügt dort Routes hinzu. Immer nur EIN Agent gleichzeitig an `main.rs`.

> [!IMPORTANT]
> **Die Community-DB Tabellen werden modular erweitert.** Modul 1 erstellt die Basis-Tabellen. Module 2-5 nutzen `ALTER TABLE` und `CREATE TABLE` um Features hinzuzufügen. Dieser modulare DB-Ansatz bedeutet: Keine Migrationen die bestehende Tabellen "breaken".

> [!IMPORTANT]
> **Die Community schreibt NIEMALS in die Core-DB.** Alle Cross-DB Zugriffe sind READ-ONLY. Einzige Ausnahme: Notifications (Core-DB) via separatem Service-Call.

> [!TIP]
> **"Coming Soon" Overlays:** In Modul 1 werden Tabs die noch nicht aktiv sind mit einem schönen "Coming Soon" Overlay versehen. Das setzt Erwartungen und zeigt den Usern, dass mehr kommt — ohne leere oder kaputte Seiten.

---

*Dieses Dokument ist die Grundlage für die modulare Community-Entwicklung. Letzte Aktualisierung: 2026-03-21. Source Truth: `docs/COMMUNITY_MASTERPLAN.md`.*
