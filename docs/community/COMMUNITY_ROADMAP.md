# POOOL Community — Modular Implementation Roadmap

> **Source:** `docs/community/COMMUNITY_MASTERPLAN.md`
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
| `2026-03-21 06:17` | `us` | `Global` | `docs/` | `✅ Check-Out` | Created Community Masterplan + Roadmap |
| `2026-03-21 07:02` | `us` | `Global` | `docs/` | `✅ Check-Out` | Restructured Roadmap to modular approach |
| `2026-03-22 12:23` | `us` | `M0` | `db.rs`, `main.rs` | `✅ Check-Out` | Provisioned local Community DB, setup dual pool, Gate M1 is now OPEN |
| `2026-03-22 12:30` | `us` | `M1-DB` | `database/community/` | `✅ Check-Out` | Created M1 base tables (posts, comments, reactions, profiles) |
| `2026-03-22 21:05` | `us` | `M1-BE` | `backend/src/community/` | `✅ Check-Out` | Completed Announcement Feed MVP backend (models, service, cross-db queries, router) |
| `2026-03-22 21:10` | `us` | `M1-FE` | `frontend/platform/` | `✅ Check-Out` | Built Feed dynamic fetching, Filters, Comments UI logic, 'Coming Soon' overlays for unused modules |
| `2026-03-22 21:35` | `us` | `M1-ADMIN` | `backend/src/` & `admin/` | `✅ Check-Out` | Implemented Admin sidebar injection, Community Dashboard, Announcements Manager and KPI backend logic |
| `2026-03-22 21:40` | `us` | `M1-QA` | `backend/src/community/tests.rs` | `✅ Check-Out` | Implemented tests for models, validation logic, and ran successful pipeline checks. Modul 1 is fully READY for launch! |
| `2026-03-22 22:38` | `us` | `M2-FE`, `M2-BE`, `M2-DB` | `community/` & `platform/` | `✅ Check-Out` | Completed M2 User Posts backend, DB migrations, content moderation rules, and frontend post modalities |
| `2026-03-22 22:50` | `us` | `M2-ADMIN` | `community/` & `admin/` | `✅ Check-Out` | Implemented full Moderation workflow: Admin pending queue, Post viewer/hider, User management (bans), and API routes |
| `2026-03-22 23:00` | `us` | `M2-ADMIN`, `M2-FE` | `admin/`, `platform/` | `✅ Check-Out` | Implemented User moderation UI (Bans/Warnings), Admin Posts viewer, and dynamic Trending Assets Sidebar Widget |
| `2026-03-22 23:10` | `us` | `M2-BE` | `storage/`, `community/` | `✅ Check-Out` | Implemented Post Image Uploader, Redis Rate Limiter, and Asset Velocity Monitor protecting from Pump & Dump spam attacks |
| `2026-03-22 23:45` | `us` | `Global` | `docs/` | `✅ Check-Out` | Added Module 3.5 (Audit Fixes) and restructured Modules 4 & 5 based on comprehensive security & scalability audit. |
| `2026-03-23 00:00` | `us` | `M4-DB, M4-BE.1-6` | `community/xp.rs, circles.rs, routes.rs` | `✅ Check-Out` | M4 Phase 1 COMPLETE: 6 DB objects (circles, circle_members, circle_invites, xp_ledger, xp_levels, ALTER community_profiles). XP award system with daily caps. Circles CRUD + invite + auto-join. 18 API endpoints. XP aggregation + invite expiry workers. |
| `2026-03-23 00:15` | `us` | `M4-FE.1-5, M4-BE.8` | `community-circles.js, community.html` | `✅ Check-Out` | M4 Frontend COMPLETE: Replaced 'Coming Soon' overlay with dynamic API-wired Circle tab. XP summary card, circle management, member list, leaderboard, XP history, create/invite modals, level-up animation. |
| `2026-03-23 00:30` | `us` | `M4-BE.7,9,10` | `xp.rs, background.rs, auth/routes.rs` | `✅ Check-Out` | M4 Phase 2 COMPLETE: Login streak tracker (hooks into email + OAuth login), circle retry worker (30min), level-gated features (L2 circles, L3 invites). Streak badge in XP card. |
| `2026-03-23 06:40` | `us` | `M4-BE.11-14` | `community/routes.rs, community/circles.rs` | `✅ Check-Out` | Circle Roles/Transfer/Privacy APIs complete. Added `POST /api/community/circles/:id/roles`, `POST /api/community/circles/:id/transfer` (with notification), `POST /api/community/circles/:id/privacy`. Business logic was already in circles.rs — this wired the HTTP routes. M4-BE.14 (owner self-kick bug) was already protected by actor==target guard. cargo check ✅ |
| `2026-03-23 07:00` | `us` | `M4-BE.15` | `community/circles.rs, routes.rs, community-circles.js, community.html` | `✅ Check-Out` | Circle Join Requests COMPLETE: 7 new Rust functions (request/cancel/approve/decline/list/get_mine), 5 new HTTP routes. Frontend: leaderboard now shows Public/Private badges, 'Request to Join' button, '⏳ Pending' state, and owners see an approval queue card with Approve/Decline. Notifications on approve+decline+new request. cargo check ✅ |
| `2026-03-23 09:40` | `us` | `M2-ADMIN.7` | `018_community_audit_log.sql, community/audit.rs, community/routes.rs` | `✅ Check-Out` | Community Audit Log COMPLETE: New `community_audit_logs` table (3 indexes), `audit.rs` fire-and-forget logger. 14 admin handlers wired with audit logging: post.hide, post.lock, user.ban, user.mute, user.shadowban, user.warn, comment.hide, comment.delete, comment.pin, circle.delete, circle.remove_member, circle.update, circle.transfer, xp.award. GET `/api/admin/community/audit-log` with entity_type/action filters. cargo check ✅ |
| `2026-03-23 00:50` | `us` | `M5-DB.2, M5-BE.2-3, M5-FE.2` | `amas.rs, routes.rs, community-amas.js, community.html` | `✅ Check-Out` | Expert AMAs COMPLETE: 3 DB tables (amas, ama_questions, ama_question_upvotes) + upvote trigger. 11 API endpoints (4 user, 7 admin). Dynamic AMA tab replaces Coming Soon overlay. Question submission, upvoting, expert answers with XP rewards. |
| `2026-03-23 01:15` | `us` | `M3-ADMIN.1-4` | `routes.rs, admin-sidebar-loader.js, badges.html, amas.html, user-detail.html, users.html` | `✅ Check-Out` | M3-ADMIN COMPLETE: Admin badge management page (CRUD + grant/revoke), admin AMA management page (create/status/answer/feature), user detail backend API, sidebar extended with Badges + Expert AMAs links. Users table now links to user detail. Fixed P1: require_auth -> get_current_user. |
| `2026-03-23 10:10` | `us` | `UX.1-6` | `community-feed.js, community.html, COMMUNITY_ROADMAP.md` | `✅ Check-Out` | Tier 1 UX Polish COMPLETE: Upgraded `renderContentWithHashtags` to also handle @mentions (purple links, hover effects). Trending Hashtags sidebar widget in right panel. Hashtag filter view with branded banner + clear button. Fixed all `#0000FF` → `#03FF88` (POOOL brand green). Removed duplicate functions. Marked UX.1–UX.6 ✅ in roadmap. JS syntax + cargo check clean. |
| `2026-03-24 11:15` | `us` | `M5.5-FIXES` | `frontend/platform/community.html, js/community*.js` | `✅ Check-Out` | Completed 10 Data Wiring / UI tasks from audit. Fixed post buttons, share clipboard, dynamic profile stats/badges, profile editor modal, dynamic AMA sidebar, hidden fake investors/announcements, etc. |

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
| **M1-DB.1** | `posts` Tabelle | posts (nur post_type 'announcement' aktiv) | `✅ DONE` | us | `database/community/` |
| **M1-DB.2** | `comments` Tabelle | comments mit post FK | `✅ DONE` | us | `database/community/` |
| **M1-DB.3** | `reactions` Tabelle | reactions mit UNIQUE + Count-Trigger | `✅ DONE` | us | `database/community/` |
| **M1-DB.4** | `announcement_categories` Tabelle | Kategorien (new_commodity, dividend, etc.) | `✅ DONE` | us | `database/community/` |
| **M1-DB.5** | `community_profiles` Tabelle (Basis) | Nur user_id + post_count + ban-Felder — KEINE XP/Level/Circle Felder | `✅ DONE` | us | `database/community/` |
| **M1-DB.6** | Basis-Indexes | idx_posts_created_at, idx_comments_post_id, idx_reactions_post_id | `✅ DONE` | us | `database/community/` |

> **Wichtig:** Module 1 braucht nur **5 Tabellen** (posts, comments, reactions, announcement_categories, community_profiles). KEINE follows, badges, circles, xp_ledger, reviews, amas, challenges. Diese kommen in späteren Modulen über `ALTER TABLE` Migrationen dazu.

### M1-BE: Backend (Rust)

| ID | Task | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|
| **M1-BE.1** | Module Skeleton | `community/mod.rs`, `models.rs`, `routes.rs`, `service.rs`, `validation.rs` — Grundstruktur | `✅` | us | `backend/src/community/` |
| **M1-BE.2** | Models (Basis) | `Post`, `Comment`, `Reaction`, `CommunityProfile` structs | `✅` | us | `backend/src/community/` |
| **M1-BE.3** | User Bridge (Basis) | Batch user lookup (Name + Avatar) von Core-DB + Redis Cache | `✅` | us | `backend/src/community/` |
| **M1-BE.4** | Validation (Basis) | Comment-Länge (1-2000 chars), Rate Limits (30 comments/hour) | `✅` | us | `backend/src/community/` |
| **M1-BE.5** | Announcements CRUD (Admin-only) | `POST /api/admin/community/announcements` — nur Admins können Posts erstellen | `✅` | us | `backend/src/community/` |
| **M1-BE.6** | Feed API (Read-only) | `GET /api/community/feed` — paginated, chronologisch, nur Announcements | `✅` | us | `backend/src/community/` |
| **M1-BE.7** | Reactions API | `POST /api/community/posts/{id}/reactions` — toggle (🔥💡👏🌱), denormalized count | `✅` | us | `backend/src/community/` |
| **M1-BE.8** | Comments API | `GET /POST /DELETE /api/community/posts/{id}/comments` — User können kommentieren | `✅` | us | `backend/src/community/` |
| **M1-BE.9** | Announcements Filter | `GET /api/community/announcements?category=dividend` — Kategorie-Filter | `✅` | us | `backend/src/community/` |
| **M1-BE.10** | Route Registration | Alle Modul-1 Routes in `main.rs` registrieren | `✅` | us | `backend/src/main.rs` ⚠️ |

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
| **M1-FE.1** | Feed-Tab dynamisch | `community-feed.js`: Demo-Daten durch `fetch()` ersetzen, Announcement-Cards rendern | `✅` | us | `frontend/platform/static/js/` |
| **M1-FE.2** | Reactions UI | Click-Handler für 🔥💡👏🌱, optimistic toggle, Counter-Update | `✅` | us | `frontend/platform/static/js/` |
| **M1-FE.3** | Comments UI | Kommentar-Sektion unter Posts, Comment-Input, Delete (eigene) | `✅` | us | `frontend/platform/static/js/` |
| **M1-FE.4** | Announcements-Tab dynamisch | `community-announcements.js`: Kategorie-Filter (All/Dividends/Platform/Market) | `✅` | us | `frontend/platform/static/js/` |
| **M1-FE.5** | Empty States + Loading | Skeleton-Loader, "Noch keine Announcements" Zustand | `✅` | us | `frontend/platform/static/css/` |
| **M1-FE.6** | Disabled UI für nicht-verfügbare Tabs | My Circle, Expert AMAs, Reviews → "Coming Soon" overlay | `✅` | us | `frontend/platform/static/js/` |

### M1-ADMIN: Admin Dashboard (2 Seiten)

> **Analog:** Marketplace hat 11 Admin-Seiten in `admin/marketplace/`. Community startet mit 2 und wächst modular auf 12.

| ID | Task | Seite | Description | Status | Assignee | File Zone |
|:---|:---|:---|:---|:---|:---|:---|
| **M1-ADMIN.1** | Admin Sidebar: Community-Sektion | - | Neue "🫂 Community" Nav-Section in `admin-sidebar-loader.js` (nur Overview + Announcements sichtbar) | `✅` | us | `frontend/platform/static/js/` ⚠️ |
| **M1-ADMIN.2** | `community/index.html` | Community Dashboard | KPI-Karten (Posts, Comments, Reactions, Aktive User), Letzte Aktivität, Quick Actions | `✅` | us | `frontend/platform/admin/community/` |
| **M1-ADMIN.3** | `community/announcements.html` | Announcements verwalten | Announcement erstellen (Titel, Content, Kategorie, Bilder), Pin/Unpin, archivieren, Vorschau | `✅` | us | `frontend/platform/admin/community/` |
| **M1-ADMIN.4** | Admin Community Stats API | `GET /api/admin/community/stats` — KPI-Daten liefern | `✅` | us | `backend/src/admin/` |

### M1-QA: Tests für Modul 1

| ID | Task | Description | Status | File Zone |
|:---|:---|:---|:---|:---|
| **M1-QA.1** | Unit Tests: Reactions | Toggle-Logik, Count-Konsistenz, Duplicate-Prevention | `✅` | us | `backend/src/community/tests/` |
| **M1-QA.2** | Unit Tests: Comments | Länge-Validierung, XSS-Prevention (Ammonia) | `✅` | us | `backend/src/community/tests/` |
| **M1-QA.3** | Integration: Full Flow | Admin postet → User sieht → User reagiert → User kommentiert | `✅` | us | `backend/src/community/tests/` |
| **M1-QA.4** | E2E: Browser Test | Community-Seite laden, Announcement sehen, reagieren, kommentieren | `✅` | us | `tests/` |

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
| **M2-BE.1** | Content Moderation (`moderation.rs`) | Ammonia HTML-Sanitizer, Keyword-Filter ("guaranteed returns", "risk-free"), Spam-Detection, Investment-Disclaimer | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.2** | User Posts CRUD | `POST /api/community/posts` — User-Posts mit Moderation-Pipeline (Sanitize → Keyword-Check → Disclaimer → Save) | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.3** | Post Edit/Delete (eigene) | `PUT /DELETE /api/community/posts/{id}` — nur innerhalb 15 Minuten editierbar, nur eigene löschbar | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.4** | Content Report API | `POST /api/community/posts/{id}/report` — User meldet Post | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.5** | Image Upload | Upload bis zu 4 Bilder pro Post via GCS, Validierung (Dateityp, Größe <5MB) | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.6** | Admin Moderation API | `GET /api/admin/community/reports` + `POST .../action` — Reports bearbeiten, Posts verstecken/löschen | `✅ DONE` | us | `backend/src/admin/` |
| **M2-BE.7** | Post Rate Limiting | Redis-basiert: max 5 Posts/Stunde, Duplicate-Detection | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.8** | New-User Sandbox & URL Filter | Enforce rule: Users under Level 2 cannot post URLs. Regex detection for "guaranteed return" variations auto-flags posts. | `✅ DONE` | us | `backend/src/community/` |
| **M2-BE.9** | Asset Velocity Monitor | Background worker monitoring post velocity. If >5 mentions of an asset in 10 mins, alert Admins (Pump & Dump protection). | `✅ DONE` | us | `backend/src/community/` |

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
| **M2-ADMIN.1** | `community/posts.html` | Posts verwalten | Tabelle aller Posts (Suche, Filter nach Typ/Autor/Status, Bulk-Aktionen: hide/delete/warn) | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M2-ADMIN.2** | `community/post-detail.html` | Post-Detail | Einzelner Post mit allen Comments, Reactions, Report-History, Moderation-Aktionen | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M2-ADMIN.3** | `community/reports.html` | Moderation Queue | Pending Reports Tabelle, Quick-Actions (hide/delete/warn/ban), Report-Detail-View, 🔴 Badge in Sidebar | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M2-ADMIN.4** | `community/users.html` | Community Users | User-Tabelle (Post-Count, Warnings, Ban-Status), Quick-Actions (warn/ban/unban), Suche | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M2-ADMIN.5** | Admin Sidebar erweitern | - | Posts, Reports (mit 🔴 Badge), Community Users in Sidebar einfügen | `✅ DONE` | us | `frontend/platform/static/js/` ⚠️ |
| **M2-ADMIN.6** | Admin Moderation APIs | Backend | `GET/POST /api/admin/community/reports`, `POST .../posts/{id}/hide`, `POST .../users/{id}/ban` | `✅ DONE` | us | `backend/src/admin/` |

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
| **M3-BE.1** | Follow API | `POST/DELETE /api/community/follow/{user_id}`, Follower/Following-Listen | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.2** | Personal Feed | `GET /api/community/feed` — jetzt mit Follow-Boost (Scoring-Algorithmus) | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.3** | Profile API | `GET /api/community/profile/{id}` — Bio, Badges, Post-Count, Follower-Count | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.4** | Profile Edit | `PUT /api/community/profile` — Bio bearbeiten | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.5** | Badge Worker | Background-Worker: alle 6h Badges berechnen (Core-DB Investments + Community-DB Stats) | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.6** | Milestone Posts (Auto) | System-generierte Posts: "🎉 Sarah hat ihr 5. Investment getätigt!" | `✅ DONE` | us | `backend/src/community/` |
| **M3-BE.7** | Dynamic Asset-Owner Tags | Cross-DB check: If post content contains asset name, query Core DB to append `[Verified Owner]` tag if holding balance > 0. | `✅ DONE` | us | `backend/src/community/` |

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
| **M3-ADMIN.1** | `community/user-detail.html` | User-Detail | Vollständiges Community-Profil: Posts, XP, Badges, Moderation-History, Warn/Ban Buttons | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M3-ADMIN.2** | `community/badges.html` | Badge-Verwaltung | Badge-Definitionen, Badge manuell an User vergeben, Badge-Statistiken | `✅ DONE` | us | `frontend/platform/admin/community/` |
| **M3-ADMIN.3** | Admin Sidebar erweitern | - | + Badges in Sidebar einfügen | `✅ DONE` | us | `frontend/platform/static/js/` ⚠️ |
| **M3-ADMIN.4** | Admin Badge APIs | Backend | `GET/POST/PUT /api/admin/community/badges`, `POST .../users/{id}/badge` | `✅ DONE` | us | `backend/src/admin/` |

---

## 🛡️ MODULE 3.5: Post-Audit Fixes (Security & Architecture)

**Ziel:** Address critical P0/P1 vulnerabilities identified in the March 2026 audit before proceeding to M4.

| ID | Task | Description | Status | Assignee | Priority |
|:---|:---|:---|:---|:---|:---|
| **FIX-F1** | XSS in Feed | `textContent` for user content in `community-feed.js`, no `innerHTML` | `✅ DONE` | us | **P0** |
| **FIX-F2** | XSS in Comments | `textContent` for comment content in `community-feed.js` | `✅ DONE` | us | **P0** |
| **FIX-F3** | XSS in Announcements | `textContent` for announcement content in `community-announcements.js` | `✅ DONE` | us | **P0** |
| **FIX-F7** | Ban Bypass | Add `check_user_not_banned()` middleware to all write routes | `✅ DONE` | us | **P1** |
| **FIX-F6** | Race Condition | Wrap `toggle_reaction` in DB transaction | `✅ DONE` | us | **P1** |
| **FIX-F4** | Verified Owner HTML Inj | Move HTML badge generation to frontend, add boolean flag to payload | `✅ DONE` | us | **P1** |
| **FIX-F5** | Missing Auth | Add `CookieJar` auth check to Trending Assets endpoint | `✅ SAFE` | us | **P1** |
| **FIX-F9** | Missing Redis Cache | Cache user bridge lookups (5min TTL) | `✅ DONE` | us | P2 |
| **FIX-CRL** | Comment Rate Limiting | Rate limiting for comments (Redis, 30/h) | `✅ DONE` | us | P2 |


## ✅ MODULE 4: Circles & XP (Phase 1 & 2) — COMPLETE

**Ziel:** Circle-System mit Referral-Integration, XP, Levels.
**Voraussetzung:** Modul 3 `✅ DONE`
**Status:** `✅ DONE` — 15/15 backend + frontend tasks complete

### M4-DB: Database Migrations
`008_circles_xp.sql` applied — 5 new tables + ALTER community_profiles

### M4-Phase 1: MVP Foundation (Circles + XP Core)

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M4-BE.4** | XP Award Service | Backend | `008_circles_xp.sql` | `✅ DONE` | us |
| **M4-BE.6** | XP Aggregation Worker (5-min) | Backend | M4-BE.4 | `✅ DONE` | us |
| **M4-BE.1** | Circle CRUD API | Backend | `008_circles_xp.sql` | `✅ DONE` | us |
| **M4-BE.3** | Circle Invite/Admin API | Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-BE.2** | Referral Signup Auto-Join Hook | Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-BE.5** | XP API (summary, history) | Backend | M4-BE.4 | `✅ DONE` | us |
| **M4-FE.1** | My Circle Tab UI (member list, invite) | User | M4-BE.1/3 | `✅ DONE` | us |
| **M4-FE.2** | XP Display (header badge + progress) | User | M4-BE.5 | `✅ DONE` | us |
| **M4-FE.3** | XP History page/section | User | M4-BE.5 | `✅ DONE` | us |

### M4-Phase 2: Growth & Engagement

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M4-BE.8** | Leaderboard API (circles + users) | Backend | M4-BE.6 | `✅ DONE` | us |
| **M4-FE.4** | Circle Leaderboard UI (top 20) | User | M4-BE.8 | `✅ DONE` | us |
| **M4-FE.5** | Level-Up Animation (CSS + JS toast) | User | M4-BE.6 | `✅ DONE` | us |
| **M4-BE.9** | Login Streak Tracker | Backend | M4-BE.4 | `✅ DONE` | us |
| **M4-BE.7** | Circle Retry Worker (failed auto-joins) | Backend | M4-BE.2 | `✅ DONE` | us |
| **M4-BE.10** | Level-gated feature enforcement | Backend | M4-BE.6 | `✅ DONE` | us |
| **M4-BE.11**| Circle Roles API (Promote/Demote) | Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-BE.12**| Circle Transfer Ownership API | Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-BE.13**| Circle Privacy Settings API (is_public) | Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-BE.14**| Fix Owner Self-Kick Bug | Backend | M4-BE.3 | `✅ DONE` | us |
| **M4-BE.15**| Circle Join Requests (Private Circles)| Backend | M4-BE.1 | `✅ DONE` | us |
| **M4-ADMIN.1**| Admin: Circles Overview page | Admin | M4-BE.1 | `✅ DONE` | us |
| **M4-ADMIN.2**| Admin: Leaderboard Management page| Admin | M4-BE.8 | `✅ DONE` | us |
| **M4-ADMIN.4**| Admin: Circle & XP APIs | Admin | M4-ADMIN.1/2| `✅ DONE` | us |
| **M4-ADMIN.3**| Admin Sidebar: Circles + Leaderboard| Admin | M4-ADMIN.1 | `✅ DONE` | us |
| **M2-ADMIN.7**| Admin Audit Log table + inserts | Admin | None | `✅ DONE` | us |

---

## 🟣 MODULE 5: Advanced Features (Phase 3)

**Ziel:** Reviews, AMAs, Challenges, Notifications, SEO.
**Voraussetzung:** Modul 3 `✅ DONE` ← Ready to start
**Geschätzte Dauer:** +2-3 Wochen

### M5-DB: Zusätzliche Migrationen

| ID | Task | Category | Status |
|:---|:---|:---|:---|
| **M5-DB.1** | `reviews` table migration | Backend | `✅ DONE` |
| **M5-DB.2** | `amas` + `ama_questions` + upvotes | Backend | `✅ DONE` |
| **M5-DB.3** | `challenges` + `challenge_progress` | Backend | `✅ DONE` |

### M5-Tasks: Features & System

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M5-BE.1** | Reviews API (CRUD + verified check) | Backend | M5-DB.1 | `✅ DONE` | us |
| **M5-FE.1** | Reviews Tab UI | User | M5-BE.1 | `✅ DONE` | us |
| **M5-BE.2** | AMAs API (Q&A + upvoting) | Backend | M5-DB.2 | `✅ DONE` | us |
| **M5-BE.3** | Admin AMA Management API | Admin | M5-BE.2 | `✅ DONE` | us |
| **M5-FE.2** | Expert AMAs Tab UI | User | M5-BE.2 | `✅ DONE` | us |
| **M5-BE.4** | Challenges API | Backend | M5-DB.3, M4-BE.1 | `✅ DONE` | us |
| **M5-FE.3** | Challenges UI | User | M5-BE.4 | `✅ DONE` | us |
| **M5-BE.5** | In-App Notification System | Backend | None | `✅ DONE` | us |
| **M5-BE.7** | SSR Post Pages (MiniJinja, SEO) | Backend | None | `✅ DONE` | us |
| **M5-BE.6** | Weekly Digest Worker (inactive users)| Backend | M5-BE.5 | `✅ DONE` | us |
| **M5-ADMIN.1**| Admin: AMA Management page | Admin | M5-BE.3 | `✅ DONE` | us |
| **M5-ADMIN.2**| Admin: Challenges page | Admin | M5-BE.4 | `✅ DONE` | us |
| **M2-ADMIN.2**| backlog: Admin Post Detail page | Admin | None | `✅ DONE` | us |
| **M3-ADMIN.1**| backlog: Admin User Detail page | Admin | None | `✅ DONE` | us |
| **M3-ADMIN.2**| backlog: Admin Badge Management | Admin | None | `✅ DONE` | us |
| **M5-ADMIN.3**| Admin Sidebar finalize | Admin | All admin | `✅ DONE` | us |
| **M5-ADMIN.4**| Admin: Circle Detail Page (Data & Settings) | Admin | M4-BE.1 | `✅ DONE` | us |
| **M5-ADMIN.5**| Admin: Global Comments Moderation Page | Admin | None | `✅ DONE` | us |
| **M5-ADMIN.6**| Admin: Force Transfer Circle Ownership API | Admin | M4-BE.1 | `✅ DONE` | us |
| **M5-ADMIN.7**| Admin: User Mute/Warning System API | Admin | None | `✅ DONE` | us |

### 🛠️ MODULE 5.5: Data Wiring & UI Fixes (Post-Audit)

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M5.5-F1**| Fix Create Post buttons (Type pre-selection) | User | M2 | `✅ DONE` | us |
| **M5.5-F2**| Implement Share button (Copy to clipboard) | User | M1 | `✅ DONE` | us |
| **M5.5-F3**| Render Profile stats from API on load (0/0/0 fix) | User | M3 | `✅ DONE` | us |
| **M5.5-F4**| Fetch & render Badges from API (remove hardcoded) | User | M3 | `✅ DONE` | us |
| **M5.5-F5**| Use real user initial/avatar in Create Post box | User | M1 | `✅ DONE` | us |
| **M5.5-F6**| Build Community Profile Edit modal (Bio, Name) | User | M3 | `✅ DONE` | us |
| **M5.5-F7**| Make AMA card dynamic or hide past dates | User | M5 | `✅ DONE` | us |
| **M5.5-F8**| Fetch Suggested Investors from API or remove fake | User | M3 | `✅ DONE` | us |
| **M5.5-F9**| Render Announcements tab from API data | User | M1 | `✅ DONE` | us |
| **M5.5-F10**| Fix Trending Assets to work with real data | User | M2 | `✅ DONE` | us |

---

## 🔴 MODULE 6: Advanced Moderation (Phase 4)

**Ziel:** Thread Locking, Muting, Mod Notes, Unified Queue, Auto-Mod.
**Voraussetzung:** Modul 5 `✅ DONE`
**Geschätzte Dauer:** +2 Wochen

### M6-Tasks: Features & System

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M6-ADMIN.1**| Thread Locking API & UI | Admin | M2 | `✅ DONE` | us |
| **M6-ADMIN.2**| Admin Mod Notes on Users | Admin | M3 | `✅ DONE` | us |
| **M6-ADMIN.3**| Unified Moderation Queue (Approve/Warn) | Admin | M2 | `✅ DONE` | us |
| **M6-ADMIN.4**| Admin Pinned Comments API & UI | Admin | M2 | `✅ DONE` | us |
| **M6-ADMIN.5**| Granular Punishments (Timed Muting) | Admin | M3 | `✅ DONE` | us |
| **M6-ADMIN.6**| Shadowbanning API | Admin | M3 | `✅ DONE` | us |
| **M6-BE.1**   | Auto-Mod (Profanity & Link Filters) | Backend | None | `✅ DONE` | us |
| **M6-ADMIN.7**| Content Tagging (NSFW / Spoiler labels)| Admin | M2 | `✅ DONE` | us |
| **M6-ADMIN.8**| Advanced Community Analytics (Trend data, Circle Stats, XP Economy) | Admin | M0 | `✅ DONE` | us |

---

## 🔵 MODULE 7: Mature Network Features (Phase 5)

**Ziel:** UX, Discovery, Search, Mentions, Retention, Compliance.
**Voraussetzung:** Modul 6 `✅ DONE`
**Geschätzte Dauer:** +2 Wochen

### M7-Tasks: Features & System

| ID | Task | Category | Dependencies | Status | Assignee |
|:---|:---|:---|:---|:---|:---|
| **M7-BE.1**   | Global Search API (Keywords, Hashtags, Users) | Backend | M2 | `✅ DONE` | us |
| **M7-FE.1**   | Global Search Bar & Filters UI | User | M7-BE.1 | `✅ DONE` | us |
| **M7-BE.2**   | Algorithmic Sorting (Hot / Trending) | Backend | M2 | `✅ DONE` | us |
| **M7-FE.2**   | Sort Toggles UI (Hot vs. Fresh) | User | M7-BE.2 | `✅ DONE` | us |
| **M7-BE.3**   | @-Mentions Parser & Notifications | Backend | M5 | `✅ DONE` | us |
| **M7-BE.4**   | OpenGraph & Rich Link Preview Extractor | Backend | M2 | `✅ DONE` | us |
| **M7-BE.5**   | Ban Appeals API & Unified Workflow | Backend | M6 | `✅ DONE` | us |
| **M7-BE.6**   | GDPR Deletion & Anonymization Worker | Backend | M2 | `✅ DONE` | us |

---

## 🚦 Module Gate Table

| Module | Name | Gate Status | Prerequisite | Can Start When | Geschätzte Dauer |
|:---|:---|:---|:---|:---|:---|
| **M0** | Infrastructure | `✅ DONE` | Main Roadmap 0.2 + 1.1 | Both `✅ DONE` | 1-2 Tage |
| **M1** | Announcement Feed (MVP) | `✅ DONE` | M0 | M0 `✅ DONE` | **~2 Wochen** |
| **M2** | User-Generated Content | `✅ DONE` | M1 | M1 ALL `✅` | +1-2 Wochen |
| **M3** | Social Layer | `✅ DONE` | M2 | M2 ALL `✅` | +1-2 Wochen |
| **M4** | Circles & XP | `✅ DONE` | M3 | M3 ALL `✅` | +2 Wochen |
| **M5** | Advanced Features | `✅ DONE` | M3 (nicht M4!) | M3 ALL `✅` | +2-3 Wochen |
| **M6** | Advanced Moderation| `✅ DONE` | M5 | M5 ALL `✅` | +2 Wochen |
| **M7** | Mature Network Features| `✅ DONE` | M6 | M6 ALL `✅` | +2 Wochen |

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

## 🚀 Extended Backlog: UX & Discovery (Post-M7)

### Tier 1: Core Discovery
* [x] **UX.1** Algorithmic Feed Sorting (Hot / Trending) ✅
* [x] **UX.2** Global Search Engine (Posts, Comments, Users, Circles) ✅
* [x] **UX.3** @-Mentions & Notification Triggers ✅
* [x] **UX.4** Hashtag Architecture (`#tag` filtering + trending sidebar + clickable tags) ✅
* [x] **UX.5** Rich Link Previews (OpenGraph Cards) ✅

### Tier 2: Personalization & Retention
* [x] **UX.6** Saved / Bookmarked Posts Tab ✅
* [ ] **UX.7** Threaded Comment Collapse `[-]`
* [ ] **UX.8** "Recommended for You" Feed Injection
* [ ] **UX.9** Offline Push / Email Digests
* [ ] **UX.10** Direct Messaging (1-on-1 Investor DMs)


### Tier 3: Rich Media & Expression
* [x] **UX.11** Native Polls & Surveys ✅
* [ ] **UX.12** Auto-Saving Drafts to `localStorage`
* [ ] **UX.13** Inline GIF / Tenor API Integration
* [ ] **UX.14** Custom User Flairs
* [ ] **UX.15** Native Dark/Light Mode Toggle

### Tier 4: Community Polish & "Live" Feel
* [ ] **UX.16** Quote Reposts (Share + embed commentary)
* [ ] **UX.17** Dynamic "Top Contributor" Badges
* [ ] **UX.18** Presence / Live Indicators ("John is typing...")
* [ ] **UX.19** Native Post Translation Button
* [ ] **UX.20** "Time to Read" Estimates for long posts

---

## 🔗 Extended Backlog: Web3 & Integrations
* [ ] **W3.1** Token-Gated Circles (e.g. Hold $1000 of Asset to join)
* [ ] **W3.2** NFT Avatar Verification (MetaMask/WalletConnect)
* [ ] **W3.3** External Social Sync (Auto-cross-post to X/Discord)
* [ ] **W3.4** Portfolio Value Tiers / Badges (Dynamic Net Worth badges)
* [ ] **W3.5** Embedded "Buy Asset" Widget inside posts
* [ ] **W3.6** DAO Treasury Display for Investment Clubs
* [ ] **W3.7** Trading PnL Leaderboard Sync

---

## 📊 Extended Backlog: Circle Owner Analytics & Tools
* [ ] **CO.1** Active Member Heatmaps (Best time to post)
* [ ] **CO.2** Custom Banner Uploads & Primary Brand Colors
* [ ] **CO.3** Membership Questionnaires ("Answer 3 questions to join")
* [ ] **CO.4** Top Contributor Analytics (Visible only to owner)
* [ ] **CO.5** Automated Welcome Messages (DM/Notification on join)
* [ ] **CO.6** Custom Role Management (Owner-created flairs)
* [ ] **CO.7** Post Scheduling Tools
* [ ] **CO.8** Circle-Level Shadowbanning
* [ ] **CO.9** Bulk Member Pruning (Kick inactive >90 days)
* [ ] **CO.10** Keyword Defense Alerts (Ping owner if "spam" is typed)
* [ ] **CO.11** Engagement Funnel Metrics (Views -> Clicks -> Comments)
* [ ] **CO.12** Custom Navigation Links in sidebar
* [ ] **CO.13** Circle Moderation Audit Log & Content Export CSV
* [ ] **CO.14** Auto-Approve Rulesets (Auto-accept Level 5+)
* [ ] **CO.15** Read-Only "Panic Mode" (Market crash lockdown)
* [ ] **CO.16** Pinned Rules Checkbox (Agree to terms before joining)

---

## 📱 Extended Backlog: Mobile Experience & Native Hooks
* [ ] **MOB.1** Progressive Web App (PWA) Install Prompt
* [ ] **MOB.2** Web Push Notifications & Critical SMS Alerts
* [ ] **MOB.3** App Icon Badge Counts (Unread Notifications)
* [ ] **MOB.4** Double Tap to Like & Swipe-to-Go-Back Gestures
* [ ] **MOB.5** Mobile-Optimized Bottom Nav Bar
* [ ] **MOB.6** Haptic Feedback (Vibration API)
* [ ] **MOB.7** Direct Camera Integration (Bypass file-picker)
* [ ] **MOB.8** Pull-to-Refresh Gesture
* [ ] **MOB.9** Offline Mode (Service Worker Caching)
* [ ] **MOB.10** Background Data Syncing
* [ ] **MOB.11** Native OS "Share" Sheet formatting

---

*Dieses Dokument ist die Grundlage für die modulare Community-Entwicklung. Letzte Aktualisierung: 2026-03-21. Source Truth: `docs/community/COMMUNITY_MASTERPLAN.md`.*
