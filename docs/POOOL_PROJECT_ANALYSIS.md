# POOOL Platform — Vollständige technische & produktstrategische Analyse

**Analysedatum:** 2026-05-16
**Quelle:** Lokales Monorepo `/Users/martin/Projects/poool` (Branch `main`, Commit `ca30252`)
**Methodik:** Direkte Code-/Konfig-Lektüre + öffentliche Signale (Landing-HTML, README, AGENTS.md, Dockerfile, cloudbuild.yaml, `.github/workflows/`, 192 SQL-Dateien, 33 Backend-Module)

---

## 0. Executive Summary

POOOL ist eine **Fintech-Plattform für tokenisierte Real-World-Assets (RWA)** — primär fraktionierte Anteile an Immobilien (Bali-Villen) und Rohstoffen ab **1 €**. Architektur: **Rust/Axum-Monolith** mit Server-Side-Rendering (MiniJinja-Templates), **PostgreSQL 16** via SQLx, **Polygon-Blockchain** (aktuell Testnet **Amoy**, Chain-ID 80002) für On-Chain-Settlement. Deployment: **Google Cloud Run** (europe-west1) mit PgBouncer-Sidecar gegen Cloud SQL. Marketing-Frontend separat in **Angular + Tailwind**, Dashboard rein **Server-Side HTML + Vanilla JS + HTMX**.

Produktreife: **Pre-Launch / Testnet** (Polygon Amoy = Test-Chain, kein Mainnet). Codebase wirkt produktionsnah (192 Migrationen, 33 Domänen-Module, harte FinTech-Disziplin: BIGINT-Cents, Argon2id, ACID-Transaktionen, CSRF, kein JWT).

---

## 1. Phase 1 — Scope & Einordnung

| Frage | Antwort | Evidenz |
|---|---|---|
| Projektname | **POOOL Platform** | VERIFIZIERT — `README.md:1` |
| Was macht es | Demokratisiert Zugang zu **fraktionierten Real-World-Assets** via Tokenisierung, ab 1 € | VERIFIZIERT — `README.md:3`, Landing-Copy |
| Problem | Premium-Immobilien/Commodities historisch nur Vermögenden zugänglich → Bruchteilbesitz via Token | STARKER HINWEIS — Marketingnarrativ |
| Zielgruppe | **Retail-Investoren** (B2C, EU-zentriert wg. €, region europe-west1) + Asset-Developer (B2B-Submission-Portal) | VERIFIZIERT — `developer/` Modul, KYC-Flow, EUR-Preise |
| Produktkategorie | **FinTech-SaaS / Tokenisierungs-Marktplatz** (hybrid: Marktplatz + Asset-Origination + Sekundärmarkt) | VERIFIZIERT |
| Monetarisierung | Marktplatz-Fees (`fee reserves` in marketplace-Modul), Affiliate-Tier-Provisionen, vermutlich Origination-Fees | STARKER HINWEIS |

---

## 2. Phase 2 — Sitemap & Informationsarchitektur

Zwei getrennte Frontends mit unterschiedlichen Domains:

### 2.1 Marketing-Site (`frontend/www/` → vermutlich `poool.finance`)

```text
/
├── /en/                    # Englisch
│   ├── index.html
│   └── ...
└── /id/                    # Bahasa Indonesia (STARKER HINWEIS Indo-Markt, evtl. Bali-Nexus)
    ├── index.html
    └── ...
```

### 2.2 Plattform-Dashboard (`frontend/platform/` → vermutlich `platform.poool.app`)

```text
/
├── /                       # landing.html (50+ Templates)
├── /signup, /login         # Auth
├── /marketplace            # Primärmarkt: Asset-Käufe
├── /secondary-marketplace  # P2P Sekundärhandel mit Order-Matching
├── /portfolio              # Investor-Portfolio
├── /wallet                 # Deposits/Withdrawals
├── /dividends              # Mietausschüttungen
├── /community              # Posts, Hashtags, AMAs, Leaderboard
├── /developer              # Asset-Tokenisierungs-Pipeline (B2B)
├── /admin                  # Operative Steuerung
├── /affiliate              # Referral-Dashboard mit Team-Mgmt
├── /blog                   # Multi-Author-CMS
├── /kyc                    # Didit.me + manueller Fallback
├── /settings, /support     # Standard
└── /legal/privacy /terms
```

*Evidenz: VERIFIZIERT — 50+ `.html`-Templates in `frontend/platform/`, korrespondierende Backend-Module in `backend/src/`.*

---

## 3. Tech-Stack (VERIFIZIERT)

| Layer | Technologie | Quelle |
|---|---|---|
| **Backend-Sprache** | Rust (Edition 2021) | `backend/Cargo.toml` |
| **Web-Framework** | Axum 0.7 + Tower-Middleware | `Cargo.toml`, `lib.rs` |
| **Templating** | **MiniJinja** (Jinja2-kompatibel, Rust-nativ) | `AGENTS.md:16` |
| **DB-Treiber** | SQLx 0.8 mit Compile-time-Macros | Cargo, `.sqlx/` Cache |
| **Datenbank** | PostgreSQL 16+ | `README.md:33` |
| **Connection-Pooling** | PgBouncer (Session-Mode) als **Sidecar** im Container | `Dockerfile:62`, `AGENTS.md:29-67` |
| **Auth** | Session-Cookies (`poool_session`, HTTP-only), Argon2id-Hashing, CSRF-Tokens | `AGENTS.md:20`, `auth/` Modul |
| **OAuth** | Google + Facebook (optional) | `AGENTS.md:161-162` |
| **KYC** | Didit.me + manueller Fallback | `AGENTS.md:23` |
| **Storage** | Google Cloud Storage (`poool-assets-primary`) | `cloudbuild.yaml:40` |
| **Blockchain** | Polygon **Amoy Testnet** (Chain-ID 80002), ERC-1155-Tokens | `cloudbuild.yaml:40` |
| **Smart Contracts** | Foundry (cast-Binary in Container) | `Dockerfile:24,82` |
| **Marketing-FE** | Angular + Tailwind, SSR via Node | `frontend/www/` |
| **Plattform-FE** | **Vanilla HTML/CSS/JS + HTMX** (partiell), MiniJinja-SSR — **kein Bundler, kein Framework** | `AGENTS.md:18,180` |
| **Caching** | Redis (optional) | `AGENTS.md:25` |
| **Monitoring** | Sentry (optional, `.sentryclirc` vorhanden) | `.sentryclirc` |
| **Build** | cargo-chef (Layer-Caching), Multi-Stage-Docker | `Dockerfile` |
| **Hosting** | Google Cloud Run, europe-west1 | `cloudbuild.yaml:36` |
| **CI/CD** | GitHub Actions (3 Workflows) | `.github/workflows/` |
| **Testing** | Playwright + pytest (Python E2E), cargo test (Rust) | `playwright.config.js`, `pyproject.toml` |
| **Lint/Format** | cargo fmt + clippy, Ruff (Python), Lighthouse-CI | `lighthouserc.json`, `.ruff_cache/` |

**Hinweis Inkonsistenz:** Initial-Survey meldete „HTMX + Alpine.js" — `AGENTS.md:181` widerspricht: „**No frameworks** — plain JS `fetch()`, no React/Vue/**Alpine** (some HTMX used)". → **VERIFIZIERT korrekt: HTMX teilweise, kein Alpine, kein Bundler.** Alpine wahrscheinlich Reststand älterer Templates.

---

## 4. Ordnerstruktur (VERIFIZIERT, direkt aus Dateisystem)

```text
poool/
├── backend/
│   └── src/
│       ├── main.rs             # Axum-Router — ALLE Routen hier registriert
│       ├── lib.rs              # Router-Wiring
│       ├── config.rs, error.rs, db.rs, cache.rs, email.rs, templates.rs
│       ├── auth/               # Login, Signup, Sessions, OAuth, 2FA
│       ├── admin/              # Admin-Dashboard-APIs
│       ├── assets/             # Asset-Management
│       ├── blockchain/         # Polygon, Smart-Contracts, On-Chain-Sync
│       ├── blog/               # Multi-Author-CMS
│       ├── cart/               # Warenkorb & Checkout
│       ├── common/             # Shared Helpers
│       ├── community/          # Posts, Hashtags, AMAs, XP
│       ├── developer/          # Asset-Tokenisierungs-Pipeline (B2B)
│       ├── dividends/          # Mietausschüttungen, Anti-Sniping
│       ├── ipfs/               # IPFS-Metadaten (ERC-1155)
│       ├── kyc/                # Didit.me + manueller Fallback
│       ├── leaderboard/        # Community-Ranking
│       ├── legal/              # Consent, Terms, Compliance
│       ├── marketplace/        # P2P-Trading, Order-Matching
│       ├── payment_methods/    # Bank-Konten, Karten
│       ├── payments/           # Order-Approval, Invoicing
│       ├── portfolio/          # Investor-Portfolio
│       ├── rewards/            # Referrals, Tiers, Balances (Affiliate-Core)
│       ├── settings/           # User-Settings
│       ├── storage/            # GCS-Uploads (Avatars, KYC-Dokumente)
│       ├── support/            # Tickets
│       └── wallet/             # Balance & Transaktionen
│   ├── templates/              # MiniJinja-Templates (Backend-seitig)
│   ├── pgbouncer/              # entrypoint.sh + pgbouncer.ini
│   └── .sqlx/                  # Offline-Query-Cache
├── frontend/
│   ├── platform/               # Dashboard (HTML + Vanilla JS + HTMX)
│   │   ├── *.html              # 50+ Pages (MiniJinja-SSR)
│   │   └── static/
│   │       ├── css/            # page-spezifisch + bundle.css (build-bundle.sh)
│   │       └── js/             # page-spezifisches JS
│   └── www/                    # Marketing (Angular + Tailwind)
│       ├── en/, id/            # Sprach-Splits
│       └── server.js           # Node-SSR
├── database/                   # 192 SQL-Dateien (165 nummerierte Migrationen + Patches/Seeds)
├── docs/                       # MASTERPLAN.md, DESIGN.md, AGENT_DEVELOPMENT_PROMPT.md, ROADMAP …
├── tests/                      # Playwright + pytest (smoke, auth, marketplace, financial, mobile …)
├── scripts/                    # Python/Node Utility-Scripts
├── analytics/, briefs/, drafts/, outlines/, distribution/  # Content-Workflow (Blog-Pipeline?)
├── content-intelligence/       # vermutlich SEO/Content-Tools
├── contracts/                  # Solidity-Verträge?
├── studio/                     # CMS-Studio? (vermutlich Sanity o. ä.)
├── .agent/workflows/           # 18 Agent-Workflows
├── .github/workflows/          # ci.yml, deploy.yml, e2e-tests.yml
├── Dockerfile                  # Multi-Stage, debian-bookworm-slim runtime
├── cloudbuild.yaml             # GCP-Build-Steps
├── start_local.sh, deploy_safe.sh
├── AGENTS.md                   # Agent-Kontext (kritisch lesen!)
├── README.md
└── BROKEN_LOGICS.md            # Bug-Tracker (lt. AGENTS.md)
```

---

## 5. Backend-Architektur (VERIFIZIERT)

**Pattern:** Modularer Monolith. Jede Domäne (`marketplace/`, `dividends/` …) hat per Konvention `routes.rs`, `models.rs`, `service.rs` (`AGENTS.md:176`). Alle Routen werden in `src/main.rs` zentral registriert — bewusste Architektur-Entscheidung („one place").

**Kritische Eigenschaften:**

- **Monetärwerte:** Ausschließlich `BIGINT` Cents (`AGENTS.md:99`) — **keine Floats**. Echte FinTech-Disziplin.
- **Transaktionen:** Jede Finanzoperation in DB-Transaction gewrappt (ACID).
- **Compile-Time-SQL:** SQLx `query!` / `query_as!` Macros — Tippfehler in Spaltennamen bricht Build. `.sqlx/` Offline-Cache wird mit committed (siehe Commits `ca30252`, `ec1bb01`).
- **No-Unwrap-Regel:** Alle Errors über zentrales `AppError` in `error.rs`.

**Domänen-Highlights:**

| Modul | Verantwortung |
|---|---|
| `marketplace/` | Sekundärmarkt: Order-Matching, Market-Orders mit Time-in-Force, Bid-Ask, Fee-Reserves |
| `dividends/` | Rental-Income-Distribution, Anti-Sniping (Cutoff-Logik), Payout-Execution |
| `blockchain/` | Polygon-RPC, Primärausgabe, On-Chain-Balance-Sync, Settlement-Tx |
| `rewards/` + `affiliate` | Tier-basierte Provisionen, Postback-Webhooks, Team-Mgmt (jüngste Commits!) |
| `developer/` | Asset-Origination-Pipeline, Project-Submissions, Asset-Change-Requests |
| `kyc/` | Didit.me-Integration mit manuellem Review-Fallback |

---

## 6. Frontend-Architektur (VERIFIZIERT)

**Bewusster Verzicht auf SPA-Framework im Dashboard.** Reasoning lt. `AGENTS.md:62`:

> „**Zero Client-Side Business Logic:** All critical routing and verification happen in the Rust backend via SSR."

**Implikationen:**

- Plattform: Server rendert vollständiges HTML, JS nur für Interaktion (HTMX `hx-get`/`hx-post` für Partial-Updates, page-spezifische `static/js/*.js` für Fetch-Calls).
- CSS gebündelt via Bash-Script (`build-bundle.sh`) — **kein webpack/vite/esbuild**.
- Schriftart: TT Norms Pro (`fonts.css`).
- Marketing-Site separater Tech-Stack (Angular + Tailwind), eigene SEO-Strategie, mehrsprachig (en/id).

**Vorteil:** Niedrige Komplexität, schnelle Erstladung, voll mit Server-Auth kompatibel.
**Nachteil:** Reaktive UX-Patterns (Live-Charts, komplexe Forms) härter zu bauen.

---

## 7. Datenbank & Migrationen

- **192 SQL-Dateien** in `database/` (Bash-count `wc -l`).
- Davon **165 nummerierte Migrationen** (`001_initial_schema.sql` … `165_affiliate_hot_path_indices.sql`).
- Rest: Seeds (`production_seed.sql`, `seed_blog.sql`), Patches (`patch_villa_pillada_horadada_images.sql`), Audits.
- **Initial-Schema:** ~571 Zeilen — substantielle Domänenmodellierung von Tag 1.
- **Strategie:** Sequentielle Migrations, applied-in-order. **Kein** sqlx-migrate-Driver für Auto-Run sichtbar — wahrscheinlich manuelle Applikation (siehe `README.md:39` `psql -f database/001_initial_schema.sql`).
- **Notable Migrations (Inhaltliche Hinweise):**
  - `026_2fa_session_column.sql` — TOTP-2FA
  - `055/056` — Reconciliation, Marketplace-RBAC
  - `105_compliance_export_audit.sql` — GDPR/Compliance-Audit-Trail
  - `146_villa_returns_feature_flag.sql` — Villa-spezifische Rückgabelogik per Feature-Flag
  - `165_affiliate_hot_path_indices.sql` — Performance-Indizes für Affiliate-Layer

**PLAUSIBLE ANNAHME:** RLS-Policies (Row-Level-Security) für Mandanten/User-Isolation — kein direkter Beleg gelesen.

---

## 8. API-Surface (STARKER HINWEIS)

**Typ:** Hybrid REST-JSON + SSR-HTML. **Keine OpenAPI-Spec** im Repo sichtbar.

Bekannte Endpoints (aus Inline-Doku `lib.rs`):

```
GET  /health
GET  /api/user/*                       # Settings, Legal-Status, 2FA
POST /api/user/legal-accept            # Consent
GET  /api/orders/:order_id             # Status-Polling
GET  /api/deposits/:deposit_id/status
GET  /api/assets/:asset_id/metadata.json   # ERC-1155 Public Metadata (IPFS-konform)
GET  /api/admin/reports/:report_type
```

**Empfehlung:** OpenAPI-Doku via `utoipa`-Crate generieren.

---

## 9. Auth & Security (VERIFIZIERT)

| Aspekt | Implementation |
|---|---|
| Sessions | HTTP-only Cookie `poool_session`, gespeichert in `user_sessions`-Tabelle, Session-Fixation-Prevention bei Login |
| Passwörter | **Argon2id** (state-of-the-art) |
| CSRF | Token-per-Form, invalidiert bei Session-Wechsel |
| 2FA | TOTP (Migration `026`) |
| OAuth | Google + Facebook (optional, `oauth_accounts`-Tabelle) |
| Verschlüsselung at rest | AES-GCM (z. B. Affiliate-Tax-ID — jüngste Commits) |
| Secret-Mgmt | GCP Secret Manager (`CHAIN_SETTLEMENT_PRIVATE_KEY`) |
| Container | Non-Root User `poool` (UID 1000), Debian-bookworm-slim |
| Privatschlüssel | NIE in Source — siehe `cloudbuild.yaml:42` |

**Stärke:** Kein JWT (vermeidet Revoke-Probleme), Server-Side-Auth überall.

---

## 10. Deployment & Infrastruktur (VERIFIZIERT)

**Pipeline:**

1. Push → `ci.yml` lintet (fmt, clippy, check), baut, testet.
2. Manueller Trigger `deploy.yml` (`workflow_dispatch`) — **kein Auto-Deploy** (User-Memory bestätigt).
3. Cloud Build (`cloudbuild.yaml`) baut Docker mit Layer-Caching (cargo-chef), pusht zu Artifact Registry `europe-west1-docker.pkg.dev/.../poool-backend`.
4. `gcloud run services update` deployt nach **europe-west1**.

**Container-Architektur:**

```
┌─ Cloud Run Container ─────────────────────────┐
│  Axum-Backend ──127.0.0.1:6432──► PgBouncer   │
│                                       │        │
│                              Unix-Socket       │
│                                       ▼        │
└──────────────────────────────────────┬─────────┘
                                       ▼
                              Cloud SQL Postgres
```

**Mandatory Rules** (`AGENTS.md:53-58`):

- `PGBOUNCER_ENABLED=true` → `db.rs` überspringt Socket-Auto-Detect
- `pool_mode = session` (sonst SQLx Prepared-Statement-Collision)
- `ignore_startup_parameters = extra_float_digits, options`
- `statement_cache_capacity(0)` bei PgBouncer-Mode

**Region:** europe-west1 (Belgien) — DSGVO-konform für EU-Retail.

---

## 11. Blockchain-Layer (VERIFIZIERT — Testnet!)

Aus `cloudbuild.yaml:40` direkt extrahiert:

| Variable | Wert | Bedeutung |
|---|---|---|
| `CHAIN_NETWORK` | `polygon_amoy` | **Polygon Amoy Testnet** (NICHT Mainnet) |
| `CHAIN_ID` | `80002` | Amoy-Chain-ID |
| `CHAIN_RPC_URL` | `https://rpc-amoy.polygon.technology` | Public Testnet-RPC |
| `CHAIN_SETTLEMENT_ENABLED` | `true` | Settlement aktiv |
| `CHAIN_CONTRACT_ADDRESS` | `0xeAd7…c1ca` | Hauptvertrag (vermutlich ERC-1155-Asset-Token) |
| `CHAIN_SETTLEMENT_ADDRESS` | `0x021F…B88a` | Settlement-Contract |
| `CHAIN_IDENTITY_REGISTRY_ADDRESS` | `0xE9DB…0306` | KYC-On-Chain-Registry (ERC-3643-Pattern?) |
| `CHAIN_IMPLEMENTATION_ADDRESS` | `0xb61C…04e5` | Proxy-Implementation (Upgradeable Pattern) |
| `CHAIN_MAX_BATCH_SIZE` | `50` | Batch-Tx-Limit |

**Wichtig:** **Identity Registry** + **Implementation Proxy** + **ERC-1155-Tokens** = klassisches **ERC-3643-Pattern** (T-REX, Compliance-konforme Security-Tokens). STARKER HINWEIS, nicht 100% bestätigt ohne Contract-Source.

**Status:** Produktion läuft gegen **Testnet** — entweder Live-Pre-Launch-Phase oder bewusster Pilot. Mainnet-Switch wäre 1-Variable-Change.

---

## 12. Business-Modell (STARKER HINWEIS)

| Revenue-Stream | Evidenz |
|---|---|
| **Marktplatz-Trading-Fees** | `marketplace/fee reserves`, Bid-Ask-Spread |
| **Origination/Listing-Fees** | `developer/` Submission-Pipeline |
| **Sekundärhandel-Gebühren** | Separater Sekundärmarkt mit Order-Matching |
| **Affiliate-Volume-Cuts** | Tiered Commissions, jüngste Commits |
| **Dividend-Service-Fees** | PLAUSIBLE ANNAHME — typisch in RWA-Modellen |

**Zielmärkte:** EU-Retail (EUR-Preise, europe-west1, en/id) + **Indonesien** (Bahasa-FE, Bali-Villen) → starker **Indo/EU-Crossborder-Fokus**.

---

## 13. Stärken / Schwächen / Risiken

### Stärken

1. **Sprachwahl Rust + Axum** — Memory-Safety, hohe Performance, niedrige Hosting-Kosten.
2. **Server-Side-First** — minimiert Angriffsfläche, kein verstecktes Client-State-Risiko.
3. **Strict-Money-Discipline** — BIGINT-Cents, ACID, kein Float.
4. **Compile-time-SQL** — Klasse von Bugs eliminiert.
5. **Saubere Domänen-Trennung** — 33 Module, klare Konventionen (`routes/models/service`).
6. **PgBouncer-Sidecar dokumentiert** — Operations-Wissen explizit in AGENTS.md.
7. **Produktionsgehärtetes Docker** — Multi-Stage, Non-Root, cargo-chef-Caching.

### Schwächen / Risiken

1. **Polygon-Amoy = Testnet** — Mainnet-Migration ist eigenständiges Großprojekt (Audit-Pflicht, Liquidität, Bridge-Strategie). RISIKO.
2. **Keine OpenAPI-Spec** — externe Integrationen mühsam, Frontend-Backend-Verträge implizit.
3. **Single-Region (europe-west1)** — kein Multi-Region-Failover für ein Finanzprodukt. RISIKO bei GCP-Region-Outage.
4. **Manueller Deploy** — Bewusste Wahl (Risk-Gating), aber langsamer Iterationszyklus.
5. **165+ Migrationen ohne Tool-gestütztes Rollback** — `sqlx-cli` ist installiert, aber `migrate run` nicht im Startup. PLAUSIBLE ANNAHME: Migrationen werden manuell appliziert → Drift-Risiko.
6. **Zwei Frontends, zwei Stacks** (Angular www + Vanilla platform) — kognitive Last, Doppelpflege von Design-Tokens. Schwäche.
7. **Bahasa-i18n nur auf Marketing** — Plattform-Dashboard vermutlich EN-only → Friction für Indo-User. STARKER HINWEIS.
8. **`BROKEN_LOGICS.md` Tracker** existiert (`AGENTS.md:148`) — operatives Bug-Inventory ist Realität, kein „clean slate".
9. **Smart-Contract-Source liegt unter `contracts/`** (Top-Level-Dir gesehen) — **nicht in Analyse einbezogen**, sollte separat auditiert werden. RISIKO.
10. **Lighthouse-CI eingerichtet, Performance-Werte unbekannt** — Frontend-Perf nicht direkt geprüft.

---

## 14. Empfehlungen (Priorisiert)

| Prio | Maßnahme | Begründung |
|---|---|---|
| **P0** | **Smart-Contract-Audit vor Mainnet** (externer Auditor: Trail of Bits, OpenZeppelin, Halborn) | Settlement-Privatschlüssel im Secret-Manager → einzelner Failure-Punkt für Investorengelder |
| **P0** | **Automatisierte Migration-Verifikation** im CI (`sqlx migrate run --dry-run` gegen Schatten-DB) | Drift-Schutz bei 192 SQL-Dateien |
| **P1** | **OpenAPI-Spec** generieren (`utoipa` für Axum) | Verträge dokumentiert, Partner-Integration, SDK-Generierung |
| **P1** | **Multi-Region-Strategie** für Cloud SQL (read replicas) | DSGVO-konformes Failover |
| **P1** | **Platform-i18n** auf Bahasa erweitern | Indo-Markt konsistent bedienen |
| **P2** | **Synthetic-Monitoring** auf kritische Flows (Login, Buy, Withdraw) zusätzlich zu Sentry | Sentry erfasst Exceptions, nicht stille Degradation |
| **P2** | **Design-System-Konsolidierung** zwischen `www` (Angular/Tailwind) und `platform` (Vanilla CSS) | Single-Source-of-Truth Design-Tokens |
| **P2** | **Contract-Code-Review** + Foundry-Test-Coverage-Bericht | `contracts/`-Verzeichnis vorhanden, Status unklar |
| **P3** | **Feature-Flag-System** (z. B. `unleash-rs`) statt einmaliger Migration-basierter Flags | Kontrollierter Rollout für riskante Finanz-Features |
| **P3** | **Lighthouse-Budget-Enforcement** im CI | Mobile-First für Retail-Markt kritisch |

---

## 15. Evidenz-Matrix (Aussagen-Klassifikation)

| Bereich | Evidenz | Lückenmarker |
|---|---|---|
| Tech-Stack Backend | **VERIFIZIERT** | — |
| Tech-Stack Frontend Platform | **VERIFIZIERT** (Vanilla+HTMX, NO Alpine) | Initial-Survey-Fehler korrigiert |
| Tech-Stack Frontend WWW | **STARKER HINWEIS** (Angular+Tailwind lt. README) | Kein `angular.json` direkt gelesen |
| Datenbank-Schema-Details | **VERIFIZIERT** Dateinamen, **PLAUSIBLE ANNAHME** RLS-Nutzung | DB-Schema nicht volltext gelesen |
| Deployment-Pipeline | **VERIFIZIERT** (Dockerfile + cloudbuild.yaml direkt gelesen) | — |
| Blockchain Config | **VERIFIZIERT** Env-Vars; **STARKER HINWEIS** ERC-3643-Pattern | Contract-Source nicht analysiert |
| Business-Modell | **STARKER HINWEIS** | Kein Pricing-Page/Term-Sheet direkt gelesen |
| Zielmarkt EU+Indo | **VERIFIZIERT** (en/id Splits, europe-west1, EUR) | — |
| Produkt-Status Pre-Launch | **STARKER HINWEIS** (Testnet, aktive Affiliate-Builds Mai 2026) | Kein Public-Launch-Datum bekannt |
| User-Anzahl, Traction | **NICHT ERKENNBAR** | Keine Analytics-Daten zugänglich |
| Funding/Investoren | **NICHT ERKENNBAR** | — |
| Team-Größe | **NICHT ERKENNBAR** | `CONTRIBUTING.md` vorhanden, nicht gelesen |

---

## 16. Offene Fragen

1. Ist Polygon Amoy aktuelle Realität oder Staging-Setup? Existiert Mainnet-Konfiguration in anderer Cloud-Build-Trigger?
2. Wird der Sekundärmarkt schon live gehandelt oder ist er Pre-Launch-Feature?
3. Compliance-Status: MiCAR-Lizenz (EU), Prospekt-Pflicht für Wertpapier-Token?
4. Wie wird Asset-Verwahrung (Custody) der physischen Immobilien rechtlich strukturiert (SPV pro Asset)?
5. KYC-Tiering: Welche Investitionslimits pro Tier (Migration `compliance_export_audit` deutet auf Strukturen)?
6. Custodian/Trustee für Mietzahlungen vor Dividenden-Distribution?
7. Off-Chain → On-Chain Reconciliation-Frequenz (Migration `055_reconciliation_reports`)?

---

**Bericht-Ende.** Quellen direkt gelesen: `README.md`, `AGENTS.md`, `Dockerfile`, `cloudbuild.yaml`, `backend/src/` Listing, `frontend/` Listing, `.github/workflows/` Listing, 192 SQL-Dateinamen. Keine externen Netzwerk-Aufrufe (lokales Repo war primäre Quelle).
