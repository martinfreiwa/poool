# POOOL Secondary Marketplace & Trading Engine Masterplan

> **Kritische Warnung:** Ein Sekundärmarkt für reale Vermögenswerte (Real World Assets) ist kein einfaches Feature, das man programmieren kann. Es ist eine vollwertige Finanzbörse. Wenn die Infrastruktur versagt, wird Geld (Fiat) und Eigentum (Token) falsch zugeordnet. Dieser Plan geht tief ins technische Detail.

Dieses Dokument analysiert die bestehende Systemarchitektur, deckt Limitationen auf und definiert den tiefgreifenden technischen Bauplan für den neuen Sekundärmarkt. Der Plan beleuchtet gezielt die Lösungsansätze aus den unterschiedlichen Entwickler-Perspektiven.

---

## Inhaltsverzeichnis (Table of Contents)
1. [Analyse der aktuellen Systemstruktur](#1-analyse-der-aktuellen-systemstruktur)
    - [1.7. Subsystem-Wechselwirkungen & Gesamtarchitektur](#17-subsystem-wechselwirkungen--gesamtarchitektur-ehrliche-expertenmeinung)
    - [1.8. Kritische Fragen (Security & Best-Practice Audit)](#18-kritische-fragen-die-sich-niemand-stellt-aber-stellen-muss)
    - [1.9. Connection Pool Auto-Scaling & 2-vs-3 Datenbanken](#19-connection-pool-auto-scaling--die-frage-warum-nicht-3-datenbanken)
    - [1.10. Progressive Kostenoptimierung](#110-progressive-kostenoptimierung-klein-starten-intelligent-skalieren)
    - [1.11. 2FA-Security-Architektur](#111-2fa-security-architektur-authentifizierung-für-trades--withdrawals)
    - [1.12. Financial & Smart Contract Testing Strategy](#112-financial--smart-contract-testing-strategy)
2. [Die neue Markt-Architektur: Order Book & Trades](#2-die-neue-markt-architektur-order-book--trades)
    - [2.1. Überblick: Sekundärmarkt](#21-überblick-wie-funktioniert-ein-sekundärmarkt)
    - [2.2. Order-Typen](#22-order-typen)
    - [2.3. Redis Orderbook-Architektur](#23-redis-orderbook-architektur-speed-layer)
    - [2.4. Die Matching-Engine](#24-die-matching-engine-das-herzstück)
    - [2.5. Settlement-Pipeline (ACID)](#25-settlement-pipeline-postgresql-acid)
    - [2.6. Fee-Struktur](#26-fee-struktur-wie-poool-am-marketplace-verdient)
    - [2.7. P2P / OTC Trades](#27-p2p--otc-trades-direkte-angebote)
    - [2.8. Preisfindung & Candlestick-Charts](#28-preisfindung--candlestick-charts)
    - [2.9. WebSocket Live-Updates](#29-websocket-live-updates)
    - [2.10. Circuit Breaker, Konzentrationslimits & Großorder-Handling](#210-circuit-breaker-konzentrationslimits--großorder-handling)
    - [2.11. On-Chain Settlement (ERC-3643)](#211-on-chain-settlement-erc-3643--smart-contract)
    - [2.12. API-Endpunkte](#212-zusammenfassung-marketplace-api-endpunkte)
    - [2.13. Order-Lifecycle & Anti-Manipulation](#213-order-lifecycle-sicherheitsregeln--anti-manipulation)
    - [2.14. Regulatorische Compliance (OJK)](#214-regulatorische-compliance-ojk-indonesien)
3. [Entwickler-Perspektiven & Tiefe Implementierungs-Guides](#3-entwickler-perspektiven--tiefe-implementierungs-guides)
    - [3.1. Senior Rust / Backend Engineer (Trading Core & API)](#31-senior-rust--backend-engineer-trading-core--api)
    - [3.2. Smart Contract / Web3 Security Engineer (On-Chain Settlement)](#32-smart-contract--web3-security-engineer-on-chain-settlement)
    - [3.3. Database & DevOps Engineer (Daten, Backups & Infrastruktur)](#33-database--devops-engineer-daten-backups--infrastruktur)
    - [3.4. Frontend / UI Engineer (Data Visualization & Vanilla Web)](#34-frontend--ui-engineer-data-visualization--vanilla-web)
4. [Datenbank-Erweiterungen (PostgreSQL & Redis)](#4-datenbank-erweiterungen-postgresql--redis)
5. [Benötigtes Entwickler-Team (Hiring Plan)](#5-benötigtes-entwickler-team-hiring-plan)

---

## 0. Stakeholder-Entscheidungen (2026-03-20)

> **Kontext:** Diese Sektion hält die Entscheidungen fest, die zwischen den Gründern (Martin + Jonas) am 20.03.2026 getroffen wurden.

### E1. Transaktionsmodell: Instant Settlement (Bestätigt)

**Entscheidung:** Der Marketplace nutzt **Instant Settlement** via Wallet-Balance — KEIN Escrow-Modell mit manuellem Zahlungsnachweis.

**Begründung:**
- User zahlt Geld auf sein POOOL-Wallet ein (Deposit → bestehender Flow)
- Beim Kauf/Verkauf wird sofort Geld gegen Tokens getauscht (< 1 Sekunde)
- Auszahlung vom POOOL-Wallet auf Bankkonto (Withdrawal → bestehender Flow)
- Das ERC-3643 NFT-System läuft **asynchron im Hintergrund** als Eigentumsnachweis

### E2. Gebühren: 5% Plattformgebühr + Tier-Rabatte

**Entscheidung:** Standard-Fee auf dem Sekundärmarkt ist **5.0% (500 BPS)** — anpassbar via Admin-Dashboard.

**Tier-Rabatt-System (NEU):**

| Tier | Basis-Fee | Rabatt | Effektive Fee |
|---|---|---|---|
| Standard (kein Tier) | 5.0% | 0% | 5.0% |
| Silver | 5.0% | -0.5% | 4.5% |
| Gold | 5.0% | -1.0% | 4.0% |
| Platinum | 5.0% | -1.5% | 3.5% |
| Diamond | 5.0% | -2.0% | 3.0% |

Die Fee-Kaskade wird um Ebene 5 (Tier-Rabatt) erweitert. Details in Sektion 2.6.

### E3. Kaufgesuche für nicht gelistete Assets (NEU)

**Entscheidung:** Investoren können Kaufinteresse an Assets platzieren, die aktuell KEINE aktiven Sell-Orders haben.

**Mechanismus:**
1. Ein Bid-Order wird im Orderbook platziert, auch wenn kein Ask existiert
2. Alle Holder des Assets werden benachrichtigt (In-App + optional E-Mail)
3. Jeder Holder kann dann entscheiden, ob er verkauft
4. Die Marketplace-Übersichtsseite zeigt ALLE Assets der Plattform (inkl. ohne aktive Orders)

### E4. Sichtbarkeit aller Assets (NEU)

**Entscheidung:** Auf dem Sekundärmarkt sind ALLE Assets sichtbar, die jemals auf POOOL verkauft wurden — nicht nur solche mit aktiven Listings.

**Badges auf Asset-Cards:**
- 🟢 "X Angebote" — wenn Sell-Orders existieren
- 🔵 "X Kaufgesuche" — wenn Bid-Orders existieren
- ⚪ "Keine Angebote" — Asset trotzdem sichtbar

**Filter-Optionen:** Alle Assets / Nur mit Angeboten / Nur mit Kaufgesuchen / Asset-Typ / Standort / ROI

---

## 1. Analyse der aktuellen Systemstruktur (Ist-Zustand & Auslastung)

Aktuell ist POOOL als reiner **Primärmarkt** (Over-The-Counter / B2C) konzipiert. Die bestehende Infrastruktur ist für diesen statischen Anwendungsfall hochoptimiert, stößt aber bei einem dynamischen Live-Markt an klare Grenzen. Im Folgenden wird das aktuelle System mit präzisen technischen Details inventarisiert.

### 1.1. Backend-Technologie (Rust / Axum)

| Komponente | Technologie | Version | Zweck |
|---|---|---|---|
| **Programmiersprache** | Rust | Edition 2021 | Systemnahe, sichere Kernlogik ohne Garbage Collector |
| **Web-Framework** | Axum | 0.7 | HTTP-Routing, Middleware, Multipart-Uploads |
| **Async Runtime** | Tokio | 1.0 (full) | Nebenläufige Verarbeitung tausender Requests |
| **Templating (SSR)** | MiniJinja | 2.0 | Server-Side Rendering der HTML-Seiten |
| **Datenbank-Zugriff** | SQLx | 0.8 (Postgres, async) | Compile-time SQL-Validierung, Connection Pooling |
| **HTTP-Client** | Reqwest | 0.12 (rustls) | Externe API-Calls (OAuth, KYC-Webhooks) |
| **Authentifizierung** | Argon2 | 0.5 | Passwort-Hashing nach Industriestandard |
| **Sessions** | axum-extra (cookie) | 0.9 | HTTP-Only Session Cookies (`poool_session`) |
| **OAuth2** | oauth2 | 5.0 | Google, Facebook, Apple Login |
| **CSRF-Schutz** | axum_csrf | 0.11 | Cross-Site Request Forgery Prävention |
| **2FA / TOTP** | totp-rs | 5.6 | Zwei-Faktor-Authentifizierung inkl. QR-Code-Erzeugung |
| **File Storage** | google-cloud-storage | 0.22 | Uploads zu Google Cloud Storage (Avatare, KYC-Docs) |
| **Caching** | redis + deadpool-redis | 0.24 / 0.14 | ✅ Redis ist bereits als Dependency integriert! |
| **Error Monitoring** | Sentry | 0.34 | Echtzeit-Crash-Reporting und Performance-Tracking |
| **XSS-Schutz** | ammonia | 4.1 | HTML-Sanitization gegen Cross-Site Scripting |
| **Kompression** | tower-http | 0.6 (compression-full) | Brotli/Gzip Response-Komprimierung |

> **Wichtige Erkenntnis:** Redis (`redis` 0.24 + `deadpool-redis` 0.14) ist bereits als Cargo-Dependency installiert. Das bedeutet, der Backend-Code hat bereits eine Verbindung zu Redis. Das ist ein massiver Vorteil – wir müssen für das Orderbook keine komplett neue Library einbinden, sondern können die bestehende Redis-Verbindung direkt nutzen und erweitern.

### 1.2. Frontend-Technologie (Vanilla Web)

| Komponente | Technologie | Details |
|---|---|---|
| **HTML** | Vanilla HTML5 | SSR via MiniJinja-Templates im Rust-Backend |
| **CSS** | Vanilla CSS | Eigene Fonts (`TT Norms Pro`), Build-Script: `build-bundle.sh` erzeugt `bundle.css` |
| **JavaScript** | Vanilla ES6+ | Kein React/Vue/Angular, kein Webpack/Vite/Bundler |
| **Architektur** | Page-spezifisch | Pro Seite wird exakt eine `static/js/<page>.js` geladen |
| **Formulare** | Fetch API + HTMX (teilweise) | Server-Kommunikation via `fetch()`, einige HTMX-Interaktionen |

### 1.3. Datenbank-Infrastruktur (PostgreSQL 16)

| Eigenschaft | Aktueller Wert | Bewertung für Marketplace |
|---|---|---|
| **DBMS** | PostgreSQL 16 | ✅ Industriestandard, extrem stabil |
| **Zugriff** | SQLx (async, compile-time checked) | ✅ Optimal für Rust |
| **Migrationen** | 49 SQL-Dateien (`database/001_*` bis `database/049_*`) | ✅ Sauber versioniert |
| **Tabellen (aktuell)** | 26+ Kern-Tabellen | ⚠️ Kein Orderbook, keine Trade-History |
| **Geldbeträge** | `BIGINT` (Cents), niemals Float | ✅ Finanztauglich |
| **Backup-Strategie** | ❌ Keine PITR, keine Read-Replicas | 🔴 **KRITISCH** – Datenverlust bei Serverausfall möglich |
| **Schreib-Performance** | ~1.000-5.000 Writes/Sek (Single-Node) | ⚠️ Limitiert für Live-Orderbook |
| **Lese-Performance** | ~10.000+ Reads/Sek (mit Indexes) | ✅ Ausreichend für Portfolios und Dashboards |

**Bestehende Kerntabellen (Inventar):**
`users`, `user_profiles`, `user_sessions`, `oauth_accounts`, `kyc_records`, `wallets`, `wallet_transactions`, `assets`, `asset_images`, `asset_milestones`, `asset_documents`, `asset_financials`, `investments`, `cart_items`, `orders`, `order_items`, `dividend_payouts`, `notifications`, `support_tickets`, `user_settings`, `roles`, `user_roles`, `developer_projects`, `audit_logs`, `password_reset_tokens`, `investment_limits`

**Was explizit FEHLT (und für den Marketplace gebaut werden muss):**
- ❌ `market_orders` (Orderbuch: offene Bids und Asks)
- ❌ `trade_history` (abgewickelte Trades für Charts)
- ❌ `p2p_offers` (Private Direktangebote zwischen Nutzern)
- ❌ `asset_price_snapshots` (Historische Preis-Ticks für Candlestick-Charts)

### 1.4. Server & Cloud-Infrastruktur (Google Cloud Run)

| Eigenschaft | Aktueller Wert | Details |
|---|---|---|
| **Hosting-Plattform** | Google Cloud Run | Serverlose Container-Plattform (Stateless) |
| **Container-Image** | Multi-Stage Dockerfile (5 Stages) | `cargo-chef` → Build → `distroless/cc-debian12` Runtime |
| **Runtime-OS** | `gcr.io/distroless/cc-debian12` | Minimal-Image ohne Shell, maximale Sicherheit |
| **Build-Pipeline** | Google Cloud Build | Automatisiert per Push oder manuell |
| **Port** | 8080 (Production) / 8888 (Local Dev) | Konfiguriert via `SERVER_PORT` ENV |
| **Auto-Scaling** | 0 → N Instanzen (Cloud Run managed) | Skaliert bei Traffic-Spikes automatisch hoch |
| **Architektur** | Stateless (kein lokaler RAM-State) | ⚠️ Orderbook kann NICHT im lokalen Speicher liegen |
| **Non-Root User** | `poool` (UID 1000) | ✅ Security-Best-Practice im Container |
| **TLS** | Automatisch via Cloud Run | HTTPS ohne manuelle Zertifikate |
| **Region** | Einzelner GCP-Standort | ⚠️ Kein Cross-Region Failover konfiguriert |

**Das Docker-Build-System (5-Stage Pipeline):**
1. **Chef Stage:** `cargo-chef` cached die Rust-Dependencies für schnelle Re-Builds.
2. **Planner Stage:** Berechnet den Dependency-Tree (`recipe.json`).
3. **Builder Stage:** Kompiliert das Rust-Binary im `--release` Modus + baut das CSS-Bundle (`build-bundle.sh`).
4. **User-Setup Stage:** Erzeugt den Non-Root `poool` Systemuser.
5. **Runtime Stage:** Das fertige Binary + Frontend + Templates landen in einem `distroless`-Image (kein `apt`, kein `bash`, kein `ssh` = minimale Angriffsfläche).

### 1.5. Systemanforderungen für den Marketplace-Upgrade

#### Realistische Lastprofile (POOOL-spezifisch)

| Kennzahl | Erwarteter Wert (Start) | Skalierungsziel (12 Monate) |
|---|---|---|
| **Gleichzeitige Nutzer** | ~100 | ~500–1.000 |
| **Transaktionen pro Tag** | ~100–1.000 | ~5.000–10.000 |
| **Durchschnittliches Transaktionsvolumen** | ≥ €500 pro Trade | ≥ €500 pro Trade |
| **Offene Orders im Orderbuch (gleichzeitig)** | ~200–500 | ~2.000–5.000 |
| **Page-Loads pro Tag** | ~5.000–10.000 | ~50.000+ |
| **WebSocket-Verbindungen (gleichzeitig)** | ~50–100 | ~500–1.000 |

> **Wichtiger Kontext:** Da es sich bei POOOL um hochwertige Immobilien-Investments handelt (Ø >€500 pro Trade), ist die Transaktionsfrequenz vergleichsweise niedrig im Vergleich zu Krypto-Börsen (die Millionen Mikro-Trades pro Sekunde verarbeiten). Das bedeutet: Wir brauchen **kein** Hochfrequenz-Trading-System à la Nasdaq, aber wir brauchen eine Architektur, die **absolut zuverlässig und skalierbar** ist – denn bei €500+ pro Transaktion darf kein einziger Trade verloren gehen oder doppelt ausgeführt werden.

#### Infrastruktur-Anforderungen (Aktuell vs. Benötigt)

| Bereich | Aktuell (Primärmarkt) | Benötigt (Sekundärmarkt) | Aktion |
|---|---|---|---|
| **PostgreSQL** | Single-Node, keine Replicas | Primary + Read-Replica + PITR Backups | 🔴 Upgrade auf Cloud SQL mit HA |
| **Redis** | Cargo-Dependency vorhanden, optional genutzt | Dedizierter Redis-Cluster (Google Memorystore) mit Sentinel/Failover | 🔴 Separaten managed Redis aufsetzen |
| **TimescaleDB** | Nicht installiert | PostgreSQL-Extension für Preis-Zeitreihen (Candlestick-Charts) | 🟡 Extension aktivieren auf Cloud SQL |
| **WebSockets** | Nicht implementiert | Axum-native WS für Live-Orderbook-Updates an ~100 Clients | 🟡 Neues Axum-Modul bauen |
| **Backup-Strategie** | ❌ Keine automatischen Backups | PITR (Sekundengenau), tägliche Cross-Region Snapshots | 🔴 Sofort einrichten, BEVOR der Marktplatz live geht |
| **Monitoring** | Sentry (Error-Tracking) | Sentry + Cloud Monitoring Dashboards (Latenz, Error-Rate, DB-Load) | 🟡 GCP Monitoring Alerts konfigurieren |
| **Rate Limiting** | `tower::limit` vorhanden | Per-User Rate Limiting: max. 10 Orders/Minute pro Nutzer | 🟡 Middleware erweitern |
| **CPU/RAM pro Container** | Cloud Run Default (1 vCPU, 512MB) | 2 vCPU, 1GB RAM pro Instanz (reicht für ~100 gleichzeitige User) | 🟡 Cloud Run Konfiguration anpassen |
| **Min. Instanzen** | 0 (Cold Start möglich) | Mindestens 1 Instanz immer aktiv (kein Cold Start bei Trades) | 🟡 `--min-instances=1` setzen |
| **Max. Instanzen** | Cloud Run Default | Max. 5 Instanzen (skaliert automatisch bei >100 gleichzeitigen Nutzern) | 🟡 `--max-instances=5` setzen |
| **Smart Contracts** | Nicht vorhanden | ERC-3643 auf Base L2 (siehe Smart Contract Masterplan) | 🔴 Komplette Neuentwicklung |
| **Banking-API (Plaid)** | Nicht integriert | Automatische Fiat-Erkennung für Settlement | 🔴 Plaid-Account + Backend-Integration |

#### Bottleneck-Analyse (Wo bricht das System zuerst?)

Bei ~100 gleichzeitigen Nutzern und ~1.000 Trades/Tag muss das System folgende Engpässe vermeiden:

| Potentieller Bottleneck | Risiko | Lösung |
|---|---|---|
| **PostgreSQL Row-Locks bei parallelen Trades** | 🟡 Mittel – bei 100 Nutzern sind max. ~10 gleichzeitige Writes realistisch | Redis als Orderbuch-Puffer: Nur der finale, gematchte Trade wird in Postgres geschrieben (1 Write statt 100 konkurrierende) |
| **Cloud Run Cold Starts** | 🟡 Mittel – Wenn 0 Instanzen laufen, dauert der erste Request ~2-3 Sek | `--min-instances=1` eliminiert Cold Starts komplett |
| **WebSocket-Verbindungen über mehrere Cloud Run Instanzen** | 🔴 Hoch – Cloud Run routet Requests zufällig; WS-Verbindungen müssen sticky sein | Redis Pub/Sub als Event-Bus: Jede Cloud Run Instanz subscribt auf Redis und pusht Updates an ihre lokalen WS-Clients |
| **Datenbankausfall ohne Backup** | 🔴 Kritisch – Aktuell kein Failover, kein PITR | Cloud SQL HA mit automatischem Failover + sekundengenaue PITR-Backups |
| **Einzelne Redis-Instanz als Single Point of Failure** | 🟡 Mittel – Redis-Crash = Orderbook weg | Google Memorystore mit automatischem Failover (Standard-Tier) |
| **Skalierung über 1.000 gleichzeitige Nutzer** | 🟢 Gering (langfristig) – Cloud Run skaliert horizontal | Architektur ist von Tag 1 "shared-nothing": Jede Instanz ist identisch, der State liegt in Redis/Postgres |

> **Zusammenfassung der Ist-Analyse:** POOOL besitzt ein technisch sauberes, hochsicheres Fundament (Rust, distroless Docker, Argon2, CSRF). Für ~100 gleichzeitige Nutzer und ~1.000 Trades/Tag ist das System mit den richtigen Upgrades mehr als leistungsfähig. Die drei kritischen Infrastruktur-Bausteine, die fehlen: **(1)** Ein dedizierter Redis-Cluster als zentrales Orderbook zwischen allen Cloud Run Instanzen, **(2)** eine PostgreSQL High-Availability Konfiguration mit Backups (PITR) und Read-Replicas, und **(3)** WebSocket-Unterstützung mit Redis Pub/Sub als Event-Bus für Echtzeit-Updates.

### 1.6. Individuelle Entwickler-Analysen des Ist-Zustands

Jeder Entwickler betrachtet das bestehende System aus seiner spezifischen Rolle und identifiziert, was für ihn funktioniert, was fehlt und was seine persönliche Prioritätenliste ist.

---

#### 🦀 Perspektive: Senior Rust / Backend Engineer

**Was heute gut funktioniert:**
*   Das Axum-Framework (v0.7) mit Tokio (v1.0 full) ist eine hervorragende Basis. Die aktuelle Architektur verarbeitet HTTP-Requests asynchron und nicht-blockierend – das ist exakt die Grundlage, die wir für eine Matching-Engine brauchen. Wir müssen das Rad nicht neu erfinden.
*   SQLx (v0.8) mit compile-time checked Queries ist goldwert. Jede SQL-Query wird zur Kompilierzeit gegen die Datenbank validiert. Das verhindert Runtime-SQL-Fehler, die bei einem Trading-System zu Geldverlust führen könnten.
*   Der strikte `BIGINT`-Cents-Ansatz (kein Float) ist korrekt und muss 1:1 in die Matching-Engine übernommen werden.
*   Redis (`redis` 0.24 + `deadpool-redis` 0.14) ist bereits im `Cargo.toml`. Der Connection Pool steht. Ich muss "nur" die Orderbuch-Logik darauf aufbauen.

**Was ich umbauen/neu bauen muss:**
*   **Matching-Engine Modul:** Neues Rust-Modul `src/marketplace/` mit `orderbook.rs`, `matching.rs`, `trades.rs`. Das existiert heute nicht – der gesamte Kauf-Flow geht über `cart/` und `orders/`, was für Festpreise gedacht ist.
*   **WebSocket-Handler:** Axum unterstützt WebSockets nativ über `axum::extract::ws::WebSocket`. Ich muss einen `/ws/orderbook/{asset_id}` Endpoint bauen, der Live-Updates an alle verbundenen Clients sendet. Dafür brauche ich intern einen `tokio::sync::broadcast` Channel pro Asset, der von Redis Pub/Sub gefüttert wird.
*   **Atomare Trade-Execution:** Der kritischste Code im gesamten System. Ein Trade (Geld von Käufer abziehen → Geld an Verkäufer gutschreiben → Shares transferieren → Trade loggen) muss in einer einzigen PostgreSQL-Transaktion laufen. Wenn irgendein Schritt fehlschlägt, wird alles zurückgerollt. Hier darf kein `unwrap()` stehen – ausschließlich `AppError`-Propagation.
*   **Idempotency Keys:** Da es bei hochpreisigen Trades (>€500) vorkommen kann, dass ein Netzwerk-Timeout den Client dazu bringt, denselben Request zweimal zu senden, muss jeder Trade-Request einen Idempotency-Key mitschicken. Der Server prüft in Redis, ob dieser Key schon verarbeitet wurde, und verhindert Doppelausführungen.

**Logik-Analyse des Gesamtsystems (Stärken & Schwächen):**

| Aspekt | Bewertung | Begründung |
|---|---|---|
| **Error-Handling** | ✅ Stärke | Zentralisiertes `AppError`-Enum mit 8 Varianten (Internal, NotFound, BadRequest, Unauthorized, Forbidden, Conflict, Database, RateLimited). Interne Fehlerdetails werden *niemals* an den Client geleakt – stattdessen in Sentry geloggt. Das ist exakt das Pattern, das wir für €500+ Trades brauchen. |
| **Transaktionssicherheit** | ✅ Stärke | Im Code existieren bereits 11 Stellen mit `pool.begin().await` (explizite DB-Transaktionen) – vor allem in `payments/service.rs` (4x), `auth/service.rs` (4x) und `payment_methods/service.rs`. Das Team denkt bereits transaktional. Für die Matching-Engine muss diese Disziplin 1:1 übernommen werden. |
| **Background-Tasks** | ✅ Stärke | Es laufen bereits 5 `tokio::spawn`-Background-Workers (Email-Scheduler, SLA-Monitoring, Token-Reclaim, Leaderboard-Refresh, Session-Cleanup). Das Muster ist etabliert – der Blockchain-Indexer und die Matching-Engine werden als weitere Spawns hinzugefügt. |
| **`unwrap()` in Produktion** | ⚠️ Schwäche | Es existieren `unwrap()`-Aufrufe in `payments/routes.rs` und `auth/` Code-Pfaden. Jedes `unwrap()` ist ein potentieller Panic (Absturz). In einem Trading-System mit Echtgeld muss jedes einzelne durch `?`-Propagation oder `.unwrap_or_default()` ersetzt werden, bevor der Marketplace live geht. |
| **Concurrency Limit** | ✅ Stärke | Die Platform ist bereits auf 100 gleichzeitige Requests begrenzt (`ConcurrencyLimitLayer::new(100)`). Das verhindert, dass ein DDoS-Angriff den Server in die Knie zwingt. Für Trading-Endpoints brauchen wir zusätzlich ein Per-User Rate Limit. |
| **Monolithische `main.rs`** | ⚠️ Schwäche | `main.rs` hat 1.855 Zeilen und enthält noch direkte Report-Handler. Für den Marketplace sollte ein neues `src/marketplace/` Modul angelegt werden, das strikt getrennt ist (eigener Router, eigene Models, eigener Service-Layer). |
| **Kauf-Logik (Primärmarkt)** | ⚠️ Schwäche für Sekundärmarkt | Der aktuelle Kauf-Flow (`cart/ → orders/ → payments/`) ist für *Festpreis-Käufe vom Developer* ausgelegt (Like E-Commerce). Für den *Sekundärmarkt* (Nutzer↔Nutzer, dynamische Preise) kann dieser Flow *nicht* wiederverwendet werden – er muss durch eine komplett neue Order-Matching-Pipeline ersetzt werden. |

**Meine Einschätzung der Auslastung:**
Bei ~100 gleichzeitigen Nutzern und ~1.000 Trades/Tag ist die Rust/Tokio-Engine massiv überdimensioniert (im positiven Sinne). Tokio kann hunderttausende gleichzeitige Connections halten. Der Bottleneck wird *niemals* Rust sein, sondern immer PostgreSQL oder Redis.

---

#### ⛓️ Perspektive: Smart Contract / Web3 Security Engineer

**Was heute gut funktioniert:**
*   Die `investments`-Tabelle in PostgreSQL speichert korrekt, wer wie viele Tokens besitzt (`tokens_owned`). Diese Datenstruktur kann als "Off-Chain Ledger" weitergenutzt werden.
*   Das KYC-System (Didit.me) ist bereits integriert. Für ERC-3643 brauche ich exakt diese Information: "Ist User X KYC-verifiziert? Ja/Nein." Die `kyc_records`-Tabelle hat das Feld `status = 'approved'` – ich kann direkt darauf referenzieren.

**Was heute komplett fehlt (meine Domäne existiert noch nicht):**
*   **Kein einziger Smart Contract:** Es gibt keine Solidity-Dateien, kein `contracts/`-Verzeichnis, kein Foundry/Hardhat Setup. Die Blockchain-Integration ist bei Null.
*   **Keine Wallet-Infrastruktur:** Nutzer haben keine Blockchain-Wallets. Es gibt keine `user_wallets`-Tabelle und kein Google Cloud KMS Key-Management.
*   **Keine On-Chain Verification:** Im Sekundärmarkt tauschen zwei Nutzer Tokens. Ich muss sicherstellen, dass die `IdentityRegistry` auf der Base-Blockchain beide Parteien als KYC-verifiziert kennt, bevor ein Token-Transfer erlaubt wird.

**Logik-Analyse des Gesamtsystems (Stärken & Schwächen):**

| Aspekt | Bewertung | Begründung |
|---|---|---|
| **KYC-Integration** | ✅ Stärke | Das bestehende KYC-System (Didit.me) liefert mir über die `kyc_records`-Tabelle exakt das Ja/Nein-Signal, das die ERC-3643 `IdentityRegistry` braucht. Ich muss keine neue Identitätsinfrastruktur bauen, sondern nur die Bridge: "KYC approved in Postgres → `registerIdentity()` on-chain". |
| **Eigentums-Ledger** | ✅ Stärke | Die `investments`-Tabelle (mit `user_id`, `asset_id`, `tokens_owned`) ist ein sauberes Off-Chain-Ledger. Bei Batch-Settlements kann ich dieses Ledger als Ground-Truth nehmen und den On-Chain-State damit abgleichen. |
| **Keine Wallet-Abstraktion** | 🔴 Schwäche | Es gibt kein Konzept eines Crypto-Wallets im Backend. Kein `user_wallets`-Modul, kein Key-Management, keine Signatur-Logik. Das ist die größte Lücke in meiner Domäne – denn ohne Wallet kann kein einziger Token on-chain bewegt werden. |
| **Fiat↔Crypto Brücke** | 🔴 Schwäche | Es gibt keine Logik, die sagt: "Wenn der PostgreSQL-Trade abgeschlossen ist, dann führe auch den On-Chain-Transfer aus." Diese bidirektionale Synchronisation (Off-Chain Matching → On-Chain Settlement) muss von Grund auf gebaut werden, mit Retry-Mechanismen für fehlgeschlagene Blockchain-Transaktionen. |
| **Audit Trail** | ✅ Stärke | Die `audit_logs`-Tabelle ist als IMMUTABLE (kein UPDATE/DELETE) konzipiert. Das ist regulatorisch goldwert. Jede On-Chain-Settlement-Aktion muss ebenfalls hier geloggt werden (inkl. TX-Hash, Block-Number, Gas-Kosten). |

**Meine Einschätzung der Auslastung:**
Die Blockchain-Last ist minimal. Bei ~1.000 Trades/Tag werde ich diese nicht einzeln on-chain settlen (zu teuer und unnötig). Stattdessen baue ich einen "Batch Settlement": Einmal täglich (oder bei kritischer Masse) wird ein Merkle-Root aller Off-Chain-Trades auf die Base L2 geschrieben. Das kostet <$1 Gas pro Tag, egal wie viele Trades stattfanden.

---

#### 🛠️ Perspektive: Database & DevOps Engineer

**Was heute gut funktioniert:**
*   PostgreSQL 16 ist stabil und die 49 Migrationen sind sauber versioniert. Die Tabellenstruktur ist logisch und normalisiert.
*   Das Docker-Build-System (5-Stage mit `cargo-chef` und distroless Runtime) ist professionell. Build-Zeiten sind durch Dependency-Caching optimiert.
*   Cloud Run als Hosting ist für den aktuellen Primärmarkt-Bedarf perfekt dimensioniert.

**Was mich nachts wach hält (Kritische Lücken):**
*   **🔴 KEINE BACKUPS:** Das ist die größte Gefahr im gesamten System. Aktuell gibt es keine automatischen PostgreSQL-Backups, kein PITR, kein Failover. Wenn die Datenbank-Instanz abstürzt oder korrupt wird, sind ALLE Nutzerdaten, Investments und Wallet-Balances unwiederbringlich verloren. Für einen Primärmarkt riskant, für einen Sekundärmarkt mit Live-Trading absolut inakzeptabel.
*   **Kein Datenbank-Monitoring:** Es gibt keine Alerts für hohe CPU-Last, langsame Queries, oder Connection-Pool-Erschöpfung. Engpässe würden erst bemerkt, wenn Nutzer sich beschweren.
*   **Redis ist "optional":** Der Redis-Container (Memorystore) muss für den Marketplace zum absolut kritischen Infrastruktur-Baustein werden, nicht mehr "nice-to-have".

**Meine Prioritätenliste (in dieser Reihenfolge):**
1. **SOFORT:** Cloud SQL mit automatischem PITR-Backup + Read-Replica aktivieren.
2. **Woche 1:** Google Memorystore (Managed Redis) mit Standard-Tier (automatisches Failover) aufsetzen.
3. **Woche 2:** Cloud Monitoring Dashboards + Alert-Policies für DB-Load, Redis-Memory und API-Latenz.
4. **Woche 3:** TimescaleDB Extension für `trade_history` installieren, Continuous Aggregates für Chart-Abfragen konfigurieren.
5. **Woche 4:** Cross-Region Backup-Snapshots (z.B. Primary in `europe-west1`, Backup in `europe-west3`).

**Logik-Analyse des Gesamtsystems (Stärken & Schwächen):**

| Aspekt | Bewertung | Begründung |
|---|---|---|
| **Migrations-Disziplin** | ✅ Stärke | 49 SQL-Migrationen sind sauber nummeriert (`001_` bis `049_`), jede mit klarem Zweck. Das `run_migrations()`-System im `main.rs` wendet sie automatisch beim Start an und trackt den Status in `_schema_migrations`. Neue Marketplace-Tabellen fügen sich nahtlos als `050_marketplace_orders.sql` etc. ein. |
| **Daten-Integrität** | ✅ Stärke | CHECK-Constraints auf finanzkritischen Spalten (z.B. `balance_cents >= 0` auf `wallets`, `tokens_owned > 0` auf `investments`). Diese sind die letzte Verteidigungslinie gegen negative Salden – die Datenbank selbst verhindert physisch, dass ein Wallet unter 0 fällt, selbst wenn der Applikationscode einen Bug hat. |
| **Keine Partitionierung** | ⚠️ Schwäche | Wenn die `trade_history` wächst (bei 1.000 Trades/Tag = ~365.000 Zeilen/Jahr), werden Chart-Abfragen ohne Partitionierung langsamer. TimescaleDB löst das über automatische Hypertables mit Zeitpartitionen. |
| **Redis nur als Cache** | ⚠️ Schwäche | Aktuell wird Redis nur für Auth-Rate-Limiting genutzt (10 Versuche / 15 Min). Es gibt keine Redis-basierte Datenstruktur für Echtzeitdaten. Für den Marketplace muss Redis zum primären Datenhalter für das Live-Orderbook werden (Sorted Sets, Pub/Sub Channels). |
| **Kein Connection-Pool-Monitoring** | 🔴 Schwäche | Der SQLx Connection Pool hat keine konfigurierten Limits oder Health-Checks. Wenn bei einem Trading-Spike alle Pool-Connections blockiert sind (weil jede auf einen Row-Lock wartet), stürzt der gesamte Server ab – ohne Vorwarnung. Empfehlung: `max_connections`, `acquire_timeout` und Health-Check-Queries explizit konfigurieren. |
| **Housekeeping-Jobs** | ✅ Stärke | Bereits 4 Background-Jobs laufen (Session-Cleanup alle 6h, Token-Reclaim alle 15min, Leaderboard-Refresh, Rate-Limiter-Cleanup). Das Muster ist etabliert. Der Marketplace braucht: Order-Expiry-Job (offene Orders nach 30 Tagen auto-canceln), Trade-Settlement-Job (täglicher Batch für Blockchain-Sync). |

**Meine Einschätzung der Auslastung:**
~1.000 Writes/Tag in PostgreSQL ist trivial – die Datenbank kann das mit einer Hand erledigen. Das Problem ist nicht die Menge, sondern die Gleichzeitigkeit: Wenn 10 Trades in derselben Millisekunde matchen und alle die `investments`-Tabelle updaten, entstehen Row-Locks. Lösung: Redis nimmt die Orders entgegen, die Matching-Engine serialisiert die Trades, und Postgres bekommt sie nacheinander – kein Lock-Contention.

---

#### 🎨 Perspektive: Frontend / UI Engineer

**Was heute gut funktioniert:**
*   Die Vanilla JS + HTML + CSS Architektur ist extrem performant. Kein React-Overhead. Seitenladungen sind blitzschnell.
*   Die page-spezifische JS-Architektur (`static/js/<page>.js`) ist sauber. Ich kann einfach eine neue `static/js/marketplace-trading.js` anlegen, ohne andere Seiten zu beeinflussen.
*   Die `TT Norms Pro`-Schrift und das bestehende Design-System geben mir eine klare visuelle Grundlage.

**Was heute komplett fehlt (Frontend hat keine Trading-UI):**
*   **Kein Echtzeit-Daten-Flow:** Heute wird alles per `fetch()` geladen (Request → Response → fertig). Für ein Live-Orderbook brauche ich WebSockets. Native `new WebSocket('ws://...')` ist in Vanilla JS trivial, aber ich muss Reconnect-Logik, Heartbeats und Error-States selbst bauen (kein React-Hook der das abnimmt).
*   **Keine Chart-Library:** Für Candlestick-Charts muss ich `lightweight-charts` (von TradingView, ~45KB, pure JS, kein Framework nötig) integrieren. Das ist die leichteste professionelle Chart-Library und passt perfekt zum Vanilla-Ansatz.
*   **Kein Order-Eingabeformular:** Das "Buy/Sell"-Widget für den Marketplace existiert nicht. Ich muss ein Formular mit robuster Client-Side-Validation bauen (max. Balance prüfen, Ganzzahlen erzwingen, Debouncing gegen Doppel-Clicks bei €500+ Orders).
*   **Keine Cap-Table (Holder-Liste):** Die Ansicht "Welcher Nutzer besitzt wie viel Prozent" gibt es nicht. Das wird eine CSS-Grid-Tabelle mit anonymisierten Nutzern und einem "Privates Angebot senden"-Button pro Zeile.

**Logik-Analyse des Gesamtsystems (Stärken & Schwächen):**

| Aspekt | Bewertung | Begründung |
|---|---|---|
| **Page-Isolation** | ✅ Stärke | Jede Seite hat eine eigene JS-Datei. Das verhindert, dass ein Bug auf der Marketplace-Seite die Wallet- oder Portfolio-Seite kaputt macht. Für die Trading-UI kann ich `marketplace-trading.js`, `marketplace-charts.js` und `marketplace-p2p.js` sauber trennen. |
| **Kein State-Management** | ⚠️ Schwäche | Da es kein React/Vue gibt, existiert kein globaler State. Wenn der Nutzer einen Trade abschickt und gleichzeitig sein Wallet-Balance angezeigt wird, aktualisiert sich die Balance-Anzeige nicht automatisch. Lösung: Ein leichtgewichtiger Event-Bus (`CustomEvent` / `EventTarget`) im Browser, der über WebSocket-Messages gefüttert wird. |
| **Security Headers** | ✅ Stärke | Das Backend setzt einen umfassenden CSP-Header, der `connect-src 'self' https: wss:` erlaubt. Das bedeutet: WebSocket-Verbindungen (`wss://`) sind bereits in der Content-Security-Policy freigeschaltet – ich muss keine Server-Konfiguration ändern. |
| **Kein Optimistic UI** | ⚠️ Schwäche | Aktuell wartet das Frontend bei jedem `fetch()`-Call auf die Server-Antwort, bevor die UI aktualisiert wird. Bei einem Trading-System ist das zu langsam. Empfehlung: Optimistic Updates – die UI zeigt den Trade sofort als "Pending" an, und ein nachträglicher WebSocket-Event bestätigt oder revidiert ihn. |
| **Toast/Notification System** | ✅ Stärke | Es gibt bereits ein Toast-Notification-Pattern im Frontend. Dieses kann für Trade-Bestätigungen und Fehlerbenachrichtigungen wiederverwendet werden ("✅ Order ausgeführt: 50 Shares zu €105" / "❌ Nicht genug Guthaben"). |
| **Doppel-Click-Prävention** | 🔴 Schwäche | Es gibt aktuell keine systematische Debounce-Logik auf Submit-Buttons. Bei einem €2.000-Trade, der 2 Sekunden dauert, könnte ein ungeduldiger Nutzer doppelt klicken und zwei identische Orders erzeugen. Lösung: Frontend-seitig Button disablen + Backend-seitig Idempotency-Key (doppelte Absicherung). |

**Meine Einschätzung der Auslastung:**
~100 gleichzeitige WebSocket-Verbindungen sind für den Browser kein Problem. Meine Hauptsorge ist UX, nicht Performance: Wenn ein Nutzer eine Order für €2.000 abschickt, muss ich *sofort* visuelles Feedback geben (Spinner, Toast, Bestätigung) – und falls der Server 2 Sekunden braucht, darf die UI nicht einfrieren oder den User verleiten, nochmal zu klicken.

---

### 1.7. Subsystem-Wechselwirkungen & Gesamtarchitektur (Ehrliche Expertenmeinung)

POOOL baut nicht nur *ein* neues Feature. Es baut *drei massive Subsysteme* gleichzeitig auf dieselbe Infrastruktur:

| Subsystem | Status | Neue DB-Tabellen | Geschätzte API-Routes | Datenlast |
|---|---|---|---|---|
| **Primärmarkt** (E-Commerce) | ✅ Live | 26+ (existierend) | ~80+ (existierend) | Niedrig: ~50 Käufe/Tag |
| **Sekundärmarkt** (Trading Engine) | ❌ Noch nicht gebaut | ~4 neue Tabellen | ~15-20 neue | Mittel: ~1.000 Trades/Tag, davon ~10 gleichzeitig |
| **Community** (Social Media) | ❌ Nur statische Demo-HTML | **~20 neue Tabellen** | **~43 neue** | Hoch: Posts, Comments, Reactions, Follows, DMs, AMAs |
| **KYC / Compliance** | ✅ Live (Didit.me) | 3 (existierend) | ~8 (existierend) | Niedrig: ~10-50 Verifizierungen/Tag |

> **Die ehrliche Wahrheit:** Die Community ist vom Datenvolumen her das *größte* neue Subsystem – nicht der Marketplace. Ein Social-Media-Feed mit Posts, Comments, Reactions, Follows, Badges, DMs und AMAs erzeugt **deutlich mehr Datenbank-Schreibzugriffe** als 1.000 Trading-Transaktionen pro Tag. Wenn 1.000 Nutzer täglich Posts liken, kommentieren und einander folgen, sind das leicht 10.000-50.000 Writes/Tag – 10-50x mehr als der Marketplace.

#### A. Brauchen wir mehrere Datenbanken?

**Revidierte Antwort: JA – die Community braucht eine eigene, physisch getrennte Datenbank. Trades sind die absolute Priorität.**

Meine ursprüngliche Empfehlung (alles in einer PostgreSQL-Instanz) war aus reiner Performance-Sicht korrekt. Aber sie ignoriert das entscheidende Geschäftsrisiko: **Wenn die Community boomt und 50.000 Writes/Tag erzeugt, darf das niemals einen einzigen €500-Trade blockieren.**

**Das Problem im Detail:** PostgreSQL hat einen endlichen Connection Pool (typisch: 100-200 Verbindungen). Jeder Write (egal ob "Like auf einen Post" oder "€2.000-Trade") benötigt eine Connection aus diesem Pool. Wenn die Community viral geht und 50 parallele Like/Comment-Writes gleichzeitig eintreffen, belegen diese 50 Connections. Gleichzeitig versucht ein €2.000-Trade eine Connection zu bekommen – und wartet. Im schlimmsten Fall: *Timeout*, der Trade schlägt fehl, der Nutzer verliert Vertrauen.

**Worst-Case-Szenarien bei einer einzelnen Datenbank:**

| Szenario | Was passiert | Konsequenz für Trades |
|---|---|---|
| Viral Post: 500 Likes in 30 Sekunden | 500 `INSERT INTO reactions` Writes überfluten den Connection Pool | 🔴 Trade-Writes werden in die Warteschlange geschoben. Latenz steigt von <100ms auf >2.000ms |
| Community-DB-Migration (ALTER TABLE) | Lock auf `community.posts` blockiert die gesamte DB-Instanz | 🔴 **ALLE** Writes blockiert – auch Trades. Totaler Stillstand |
| Runaway Query (unoptimierter Feed-Algorithmus) | Ein `SELECT` ohne Index scannt 100.000 Zeilen | 🟡 CPU-Spike auf der DB-Instanz verlangsamt alles andere |
| Moderation: Admin löscht Spam (DELETE 10.000 Rows) | Massive Write-Amplification durch Index-Updates | 🔴 I/O-Sättigung der Festplatte – Trades schreiben langsamer |

**Die Lösung: Physische Trennung – 2 getrennte PostgreSQL-Instanzen:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloud Run (Rust-Backend)                  │
│     1 Applikation, 2 Connection Pools (SQLx Pools)          │
│                                                             │
│   pool_core:  → Core DB     pool_community:  → Community DB │
└──────┬──────────────────────────────┬───────────────────────┘
       │                              │
       ▼                              ▼
┌──────────────────┐      ┌──────────────────────┐
│  CORE DATABASE   │      │  COMMUNITY DATABASE  │
│  (Cloud SQL #1)  │      │  (Cloud SQL #2)      │
│                  │      │                      │
│  • users         │      │  • posts             │
│  • wallets       │      │  • comments          │
│  • investments   │      │  • reactions         │
│  • orders        │      │  • follows           │
│  • market_orders │      │  • badges            │
│  • trade_history │      │  • user_badges       │
│  • p2p_offers    │      │  • amas              │
│  • kyc_records   │      │  • ama_questions     │
│  • audit_logs    │      │  • reviews           │
│                  │      │  • messages          │
│  ABSOLUTE        │      │  • reports           │
│  PRIORITÄT       │      │                      │
│  Kein Community- │      │  Darf crashen/       │
│  Traffic hier!   │      │  langsam sein, ohne  │
│                  │      │  Trades zu beein-    │
│                  │      │  flussen             │
└──────────────────┘      └──────────────────────┘
       │                              │
       ▼                              ▼
 + Read-Replica              + eigene Read-Replica
 + PITR Backup               + PITR Backup
 + Redis (Orderbook)
```

**Warum 2 statt 1:**

| Eigenschaft | 1 Datenbank (alt) | 2 Datenbanken (neu) |
|---|---|---|
| **Trade-Isolation** | ❌ Community-Writes konkurrieren mit Trade-Writes um denselben Pool | ✅ Trade-Pool ist exklusiv – 0% Community-Einfluss |
| **DB-Migrationen** | ❌ `ALTER TABLE` auf Community-Tabelle blockiert Trading | ✅ Community-Migrationen betreffen nur Community-DB |
| **Skalierung** | ⚠️ Vertikales Scaling (größere Instanz) für alles | ✅ Community-DB kann unabhängig skaliert werden |
| **Backup/Restore** | ❌ Restore der Community = Restore der Trades (gefährlich) | ✅ Unabhängige Backups, unabhängige Restores |
| **Kosten** | ✅ Günstiger (1 Instanz) | ⚠️ ~$50-100/Monat mehr für die zweite Instanz |
| **Komplexität im Code** | ✅ Ein Pool | ⚠️ Zwei SQLx Pools im Rust-Code (`pool_core` + `pool_community`) |
| **Cross-System Joins** | ✅ Direkte JOINs möglich | ⚠️ Kein direkter JOIN. `user_id` wird im Rust-Code verknüpft (2 Queries) |

**Der Cross-Join-Tradeoff:** Wenn die Community-Seite den Usernamen zu einem Post anzeigen will, kann sie nicht einfach `JOIN users` machen. Stattdessen holt der Rust-Code die `user_id` aus dem Community-Post, und macht einen separaten Lookup in der Core-DB. Das ist ein minimaler Overhead (<1ms), der sich aber durch geschickte Batching-Patterns (`WHERE user_id IN (...)`) effizient lösen lässt.

> **Fazit: Der Mehrpreis von ~$50-100/Monat für eine zweite Cloud SQL Instanz ist eine Versicherung dafür, dass niemals ein Community-Spike einen €500+ Trade blockiert. Das ist keine Frage der Performance – es ist eine Frage der Geschäftspriorität.**

#### B. Wie viele Backup-Datenbanken und Server brauchen wir?

| Komponente | Spezifikation | Zweck | Monatl. Kosten (ca.) |
|---|---|---|---|
| **Core DB (Primary)** | Cloud SQL, 2 vCPU, 4GB RAM, HA | Trades, Wallets, Orders, KYC – ABSOLUTE PRIORITÄT | ~$120 |
| **Core DB Read-Replica** | Cloud SQL, 1 vCPU, 2GB RAM | Portfolio-Ansichten, Chart-Queries, Admin-Dashboards | ~$60 |
| **Community DB (Primary)** | Cloud SQL, 1 vCPU, 2GB RAM | Posts, Comments, Likes, Follows, AMAs, DMs | ~$60 |
| **Community DB Read-Replica** | Cloud SQL, 1 vCPU, 2GB RAM | Feed-Queries, Trending, Leaderboard | ~$60 |
| **Redis (Memorystore)** | Standard-Tier, 1GB RAM, Auto-Failover | Live-Orderbook, Rate-Limiting, WebSocket Pub/Sub | ~$50 |
| **Cloud Run (Rust-Backend)** | 2 vCPU, 1GB RAM, min 1 / max 10 | Der EINE Applikations-Server (skaliert automatisch) | ~$30-80 |
| **Google Cloud Storage** | Pay-per-use | Bilder, KYC-Docs, Community-Uploads | ~$5-20 |
| **PITR-Backups** | Automatisch auf beiden Cloud SQL Instanzen | Sekundengenau zurückspulen nach Hack oder Bug | Inkludiert |
| **Cross-Region Snapshots** | 1x täglich nach `europe-west3` | Katastrophenschutz | ~$10 |
| **Gesamt** | | | **~$395-$460/Monat** |

**Zusammenfassung: 3 Datenbank-Server (2 Primaries + 2 Read-Replicas) + 1 Redis + 1 Cloud Run Service + GCS.**

> Bei ~$400-460/Monat bekommt POOOL eine Infrastruktur, die eine Finanz-Handelsplattform MIT Social-Media-Layer betreibt – mit automatischen Backups, Failover und physischer Isolation der Trades. Zum Vergleich: Ein einziger Senior Developer kostet $10.000+/Monat. Die Infrastruktur ist der billigste Teil des gesamten Projekts.

#### C. Sicherheitsarchitektur: Was passiert wenn wir gehackt werden?

Die physische Trennung der Datenbanken hat einen zusätzlichen Sicherheitsvorteil – **Blast Radius Reduction:**

| Angriffsszenario | Betroffenes System | Schadensausbreitung | Gegenmaßnahme |
|---|---|---|---|
| **SQL-Injection** | Alle | 🔴 Theoretisch totaler DB-Zugriff | ✅ Bereits geschützt: SQLx compile-time Queries verhindern SQL-Injection auf Architektur-Ebene. Es ist physisch unmöglich. |
| **Session-Hijacking** | Auth | 🟡 Zugriff auf ein einzelnes User-Konto | ✅ HTTP-Only Cookies (nicht per JS auslesbar), CSRF-Middleware aktiv, Sessions in DB mit Ablaufzeit |
| **XSS über Community-Posts** | Community | 🟡 Bösartiger JS-Code in Posts | ✅ Ammonia HTML-Sanitizer (v4.1). Wichtig: Selbst wenn XSS durchkommt, hat die Community-DB KEINEN Zugriff auf die Trade-Daten (physisch getrennt!) |
| **Community-DB kompromittiert** | Community | 🟢 NUR Posts/Likes/Follows betroffen | ✅ **NEU: Die Trade-DB ist physisch unerreichbar vom Community-System.** Wallets, Orders, Investments sind sicher. |
| **Core-DB kompromittiert** | Trades, Wallets | 🔴 Finanzdaten betroffen | 🔴 PITR-Backup: Sekundengenau zurückspulen. Cross-Region Snapshot als letzte Rettungslinie |
| **Redis kompromittiert** | Marketplace Orderbook | 🟡 Orderbook manipuliert | ✅ Redis ist nur Cache – "Wahrheit" liegt in Core-DB. Orderbook wird aus `market_orders` wiederhergestellt |
| **Cloud Run gehackt** | Alle | 🟡 Momentaner Zugriff | ✅ Distroless Image: Keine Shell, kein SSH. Stateless: Neustart = sauber |
| **DDoS** | Frontend/API | 🟡 Seite nicht erreichbar | ✅ Cloud Run Auto-Scaling + ConcurrencyLimit + Rate-Limiting. Google Cloud Armor als WAF |

> **Entscheidender Vorteil der 2-DB-Architektur:** Wenn ein Angreifer über eine XSS-Schwachstelle in Community-Posts die Community-Datenbank kompromittiert, sind Wallets, Trades und Investments trotzdem sicher. Die Community-DB hat physisch keine Verbindung zur Core-DB – der Angreifer stößt auf eine verschlossene Tür.

#### D. Performance-Wechselwirkungen: Jetzt gelöst durch Trennung

| Ehemaliges Risiko | Mit 1 DB | Mit 2 DBs (neue Architektur) |
|---|---|---|
| Community-Likes überfluten den Connection Pool → Trade-Writes warten | 🔴 Reales Risiko | ✅ **Eliminiert.** Getrennte Pools, getrennte Instanzen |
| Community DB-Migration blockt Trading | 🔴 Reales Risiko | ✅ **Eliminiert.** Migrationen auf unabhängigen Instanzen |
| Runaway Community Query frisst CPU | 🟡 Mittel | ✅ **Eliminiert.** Eigene CPU, eigener RAM |
| Community Read-Traffic lastet Primary aus | 🟡 Mittel | ✅ **Eliminiert.** Eigene Read-Replica |
| Tägliche Blockchain-Settlement | 🟢 Gering | 🟢 Unverändert – läuft auf Core-DB, I/O-gebunden |

> **Fazit Gesamtarchitektur (revidiert):** Die physische Trennung in **Core-DB** (Trades, Wallets, Finanzen) und **Community-DB** (Social Media) ist die einzig verantwortbare Architektur, wenn Trades die absolute Priorität haben. Für ~$50-100/Monat Mehrkosten wird garantiert, dass kein Community-Spike, keine Community-Migration und kein Community-Hack jemals einen Finanztrade beeinflussen kann. Die Gesamtinfrastruktur (3 DB-Server + 1 Redis + Cloud Run + GCS) kostet ~$400-460/Monat und skaliert problemlos auf 1.000+ gleichzeitige Nutzer.

---

### 1.8. Kritische Fragen, die sich niemand stellt (Aber stellen MUSS)

Die folgenden 12 Fragen stellt sich ein Senior Security Architect bei einer Finanzplattform. Jede einzelne muss beantwortet sein, bevor der Marketplace mit Echtgeld live geht.

---

#### Frage 1: 🔴 Können zwei Nutzer gleichzeitig dieselben Shares kaufen? (Double-Spending / Race Condition)

**Ist-Zustand im Code:**
Das System nutzt bereits `SELECT ... FOR UPDATE` an 20+ Stellen – das ist sehr gut. In `payments/service.rs` wird beim Checkout der Asset-Row gelockt (`FOR UPDATE OF a`), bevor Tokens reduziert werden. Die `wallets`-Tabelle wird ebenfalls mit Row-Locks geschützt.

**Was fehlt (KRITISCH für den Sekundärmarkt):**
Der Primärmarkt hat nur einen Verkäufer (POOOL). Im Sekundärmarkt haben wir *tausende* von Verkäufern. Wenn Nutzer A seine 100 Shares zum Verkauf anbietet und zwei Käufer gleichzeitig 80 Shares kaufen wollen, muss die Matching-Engine serialisiert arbeiten – der erste Käufer bekommt 80, der zweite bekommt nur noch 20 (oder wird abgelehnt).

| Aspekt | Primärmarkt (Heute) | Sekundärmarkt (Neu) |
|---|---|---|
| Verkäufer | 1 (POOOL) | N (jeder Investor) |
| Lock-Ziel | `assets.tokens_available` | `market_orders.remaining_quantity` pro Seller |
| Risiko bei Race | ⚠️ Gering (1 Seller = wenig Konflikt) | 🔴 Hoch (N Seller × M Buyers = viele Konflikte) |
| Lösung | `FOR UPDATE OF a` (existiert) | Redis-basierte Matching-Engine mit serieller Verarbeitung + PostgreSQL als Persistence Layer |

> **Status: ⚠️ Primärmarkt ist sicher. Sekundärmarkt muss die Matching-Engine so bauen, dass sie atomares Matching garantiert – idealerweise über Redis RPOPLPUSH (Single-Thread, keine Race Conditions) mit anschließendem PostgreSQL-Persist.**

---

#### Frage 2: 🔴 Was passiert, wenn Geld im System "verschwindet"? (Reconciliation / Bilanzprüfung)

**Ist-Zustand:** Es gibt **KEINE Reconciliation-Logik** im gesamten Codebase. Das Wort "reconcil" taucht nirgends auf.

**Was das bedeutet:** Wenn ein Bug dazu führt, dass €500 von Wallet A abgezogen werden, aber nie bei Wallet B ankommen, merkt das niemand. Es gibt keinen automatisierten Check, der sagt: "Die Summe aller Wallet-Balances ≠ Summe aller Einzahlungen − Summe aller Auszahlungen – ALARM!"

**Best-Practice-Lösung:**

```sql
-- Täglicher Reconciliation-Job (als Background-Task)
-- Invariante: SUM(wallets.balance_cents) = SUM(deposits paid) - SUM(withdrawals completed) - SUM(purchases completed)
SELECT
    (SELECT COALESCE(SUM(balance_cents), 0) FROM wallets WHERE wallet_type = 'cash') AS total_balances,
    (SELECT COALESCE(SUM(amount_cents), 0) FROM deposit_requests WHERE status = 'paid') AS total_deposits,
    (SELECT COALESCE(SUM(amount_cents), 0) FROM withdrawal_requests WHERE status = 'completed') AS total_withdrawals,
    (SELECT COALESCE(SUM(total_cents), 0) FROM orders WHERE status = 'completed') AS total_purchases;
-- Wenn total_balances ≠ total_deposits - total_withdrawals - total_purchases → SOFORT Sentry-Alert + Admin-Benachrichtigung
```

> **Status: 🔴 KRITISCH. Ein Reconciliation-Job muss VOR dem Marketplace-Launch implementiert werden. Bei Finanzplattformen ist das nicht optional – es ist regulatorische Pflicht.**

---

#### Frage 3: 🔴 Gibt es ein Auszahlungslimit? (Fraud Prevention)

**Ist-Zustand:** Der Withdrawal-Handler in `wallet/routes.rs` hat ein Deposit-Limit (`MAX_DEPOSIT_CENTS = $1.000.000`), aber **KEIN Withdrawal-Limit pro Tag/Woche.** Ein Nutzer – oder ein Hacker, der eine Session kompromittiert hat – kann beliebig viel auf einmal auszahlen.

**Best-Practice-Lösung:**

| Limit-Typ | Schwelle | Aktion |
|---|---|---|
| **Per-Transaction Max** | €10.000 | Sofort ablehnen, Nutzer informieren |
| **Daily Withdrawal Cap** | €25.000 | Automatisch ablehnen, Admin-Alert |
| **Velocity Check** | >3 Auszahlungen in 1 Stunde | Auto-Freeze des Wallets, manuelle Review |
| **New Account Cooldown** | Kein Withdrawal in ersten 72h nach Ersteinzahlung | Fraud-Pattern: Deposit → Instant Withdraw mit gestohlener Karte |
| **Large Withdrawal** | >€5.000 | Erfordert 2FA-Bestätigung + 24h Cooling-Off-Periode |

> **Status: 🔴 KRITISCH. Ohne Withdrawal-Limits ist ein kompromittiertes Konto gleichbedeutend mit Totalverlust. Muss vor dem Marketplace implementiert werden.**

---

#### Frage 4: 🟡 Wie viele gleichzeitige Datenbankverbindungen können wir halten? (Connection Pool)

**Ist-Zustand:** `db.rs` konfiguriert den Pool mit `max_connections(10)`. Das bedeutet: **maximal 10 gleichzeitige Datenbank-Operationen.** Bei 100 gleichzeitigen Nutzern, die alle gleichzeitig Trades/Likes/Wallets abfragen, reicht das nicht.

**Problem:** Jeder `FOR UPDATE`-Lock hält eine Connection, bis die Transaktion committed wird. Wenn ein Checkout 200ms dauert (11 SQL-Steps), sind 10 Connections nach nur 50 gleichzeitigen Nutzern erschöpft. Die restlichen 50 Nutzer warten oder bekommen Timeouts.

**Empfehlung:**

| Umgebung | `max_connections` | Warum |
|---|---|---|
| **Core-DB Pool** | 30-50 | Trades + Wallets + Orders + KYC. Höher gewichtet weil finanzkritisch |
| **Community-DB Pool** | 15-25 | Posts + Comments + Likes. Weniger kritisch, höheres Volume |
| **Cloud SQL `max_connections`** | 100 (default bei Cloud SQL) | Muss mindestens Summe beider Pools + Headroom für Admin-Tools |

> **Status: 🟡 HOCH. Der aktuelle Pool von 10 ist für den Primärmarkt gerade so ausreichend, wird aber bei 100+ gleichzeitigen Nutzern zum Bottleneck. Muss auf 30-50 erhöht werden.**

---

#### Frage 5: 🔴 Was passiert, wenn die Datenbank 5 Minuten nicht erreichbar ist? (Graceful Degradation / Circuit Breaker)

**Ist-Zustand:** Es gibt **keine Circuit-Breaker-Logik** im Code. Wenn PostgreSQL nicht erreichbar ist, antwortet jeder Request mit einem 500 Internal Server Error. Die gesamte Platform ist tot – inklusive statischer Seiten.

**Best-Practice-Lösung:**

| Komponente | Ohne DB | Empfohlenes Verhalten |
|---|---|---|
| **Portfolio-Seite** | 🔴 Crash | 🟢 Zeige gecachte Daten (letzte bekannte Balance aus Redis/LocalStorage) |
| **Community-Feed** | 🔴 Crash | 🟢 Zeige "Feed wird geladen..." mit Retry-Button |
| **Trade-Submission** | 🔴 Crash | 🟡 Zeige "Trading vorübergehend nicht verfügbar." Kein stilles Fehlschlagen! |
| **Login** | 🔴 Crash | 🟡 Zeige "Anmeldung vorübergehend nicht möglich. Bitte versuchen Sie es in wenigen Minuten." |
| **Health-Check** | ✅ Antwortet immer | ✅ Return 503, damit Cloud Run kein Traffic an diesen Container sendet |

> **Status: 🔴 KRITISCH. Mindestens der Health-Check muss den DB-Status prüfen und 503 zurückgeben, damit Cloud Run automatisch auf gesunde Instanzen umleitet.**

---

#### Frage 6: 🟡 Idempotency – Was wenn der Server nach dem Wallet-Abzug crasht, aber VOR dem Trade-Commit?

**Ist-Zustand:** In `payments/service.rs` → `confirm_deposit()` existiert bereits eine Idempotency-Prüfung (`if status == "paid" → return Ok`). Das ist sehr gut. Aber der `execute_checkout()` hat diese Logik **NICHT** – es gibt keinen Idempotency-Key.

**Das Horror-Szenario:**
1. Nutzer klickt "Kaufen" (€2.000 Trade)
2. Server zieht €2.000 vom Wallet ab (Step 4 im Checkout)
3. Server crasht (Pod wird von Cloud Run recycled)
4. Nutzer klickt nochmal "Kaufen" (weil keine Bestätigung kam)
5. Ergebnis: €4.000 abgezogen, aber nur 1 Trade ausgeführt

**Lösung:** Jeder finanzielle Request braucht einen `Idempotency-Key` (UUID vom Client generiert):

```rust
// Vor dem Trade: Check ob Key schon existiert
let existing = redis.get(&format!("idempotency:{}", key)).await;
if existing.is_some() {
    return Ok(existing); // Selbes Ergebnis wie beim ersten Mal
}
// Nach dem Trade: Key speichern (TTL 24h)
redis.set_ex(&format!("idempotency:{}", key), result, 86400).await;
```

> **Status: 🟡 HOCH. Deposits sind geschützt, Checkouts und zukünftige Trades noch nicht. Muss vor dem Marketplace implementiert werden.**

---

#### Frage 7: 🟡 GDPR vs. Finanzaufbewahrungspflicht – Welches Gesetz gewinnt?

**Ist-Zustand:** Die Plattform hat eine `gdpr-data-request.html` und `account-deletion.html` Seite. Aber:

| GDPR-Anforderung | EU-Finanzregulierung | Konflikt? |
|---|---|---|
| "Lösche alle meine Daten" | KYC-Dokumente müssen 5-10 Jahre aufbewahrt werden | 🔴 JA – KYC-Daten dürfen NICHT gelöscht werden |
| "Gib mir alle meine Daten" (Export) | Transaktionslogs sind Geschäftsunterlagen | 🟡 Kein Konflikt – aber der Export muss vollständig sein |
| "Ich widerrufe meine Einwilligung" | Audit-Logs sind IMMUTABLE per Design | 🔴 JA – Audit-Logs dürfen nicht gelöscht werden |

**Lösung:** Account-Löschung muss *selektiv* sein:
- ✅ Lösche: Persönliche Profildaten (Name, Adresse, Telefon), Community-Posts, Follows
- ❌ Behalte (anonymisiert): KYC-Aufzeichnungen, Wallet-Transaktionen, Audit-Logs, Investments
- Nutzer wird in der DB zu `user_deleted_<hash>` anonymisiert, aber finanzielle Records bleiben

> **Status: 🟡 HOCH. Aktuell unklar ob die Account-Deletion alle finanziellen Aufbewahrungspflichten respektiert. Muss rechtlich geprüft werden.**

---

#### Frage 8: 🟡 Werden Secrets und Keys regelmäßig rotiert?

**Ist-Zustand:** Session-Secrets, API-Keys (Didit, Google OAuth, Stripe), Datenbank-Passwörter – keines davon hat eine dokumentierte Rotations-Policy.

| Secret | Rotations-Policy (Best-Practice) | Aktuell |
|---|---|---|
| **DB-Passwort** | Alle 90 Tage | ❌ Nie rotiert |
| **Session-Secret** | Alle 30 Tage | ❌ Nie rotiert |
| **Stripe API Key** | Bei Verdacht auf Kompromittierung | ❌ Keine Policy |
| **Google OAuth Secret** | Alle 6 Monate | ❌ Keine Policy |
| **Sentry DSN** | Nie (read-only) | ✅ OK |

> **Status: 🟡 MITTEL. Für den aktuellen Scale akzeptabel. Vor dem Marketplace-Launch sollte eine Key-Rotation-Policy dokumentiert und eingehalten werden.**

---

#### Frage 9: 🔴 Gibt es einen Disaster-Recovery-Plan? (RTO & RPO)

**Ist-Zustand:** Es gibt **keinen dokumentierten Disaster-Recovery-Plan.** Wenn die Datenbank morgen ausfällt, weiß niemand:
- Wie lange darf es dauern, bis das System wieder läuft? (RTO = Recovery Time Objective)
- Wie viele Daten dürfen maximal verloren gehen? (RPO = Recovery Point Objective)

**Empfehlung für POOOL:**

| Metrik | Zielwert | Wie erreichen |
|---|---|---|
| **RTO** (wie lange offline) | < 30 Minuten | Cloud SQL HA mit automatischem Failover (<1 Min). Manueller Code-Fix via Cloud Run Rollback |
| **RPO** (max. Datenverlust) | < 1 Sekunde | PITR-Backup auf Cloud SQL (sekundengenau) |
| **RTO Community** | < 4 Stunden | Niedrigere Priorität. Kann länger offline sein als Trading |
| **RTO Marketplace/Trades** | < 5 Minuten | Höchste Priorität. Jede Minute offline = Vertrauensverlust + potentieller Geldverlust |

> **Status: 🔴 KRITISCH. Ein dokumentierter DR-Plan mit klar definierten RTOs, RPOs und Verantwortlichkeiten muss erstellt werden.**

---

#### Frage 10: 🟡 FX-Rate-Handling – Ist das Float-basierte System sicher?

**Ist-Zustand:** In `payments/service.rs` wird der USD→IDR Wechselkurs als `f64` (Float) gespeichert (`CACHED_IDR_RATE: AtomicU64` storing `f64::to_bits()`). Die Umrechnung erfolgt mit Integer-Division: `(total_cents / 100) * rate_i64`.

**Das Problem:** `rate_i64 = rate as i64` schneidet die Nachkommastellen ab. Bei einem Kurs von 15.847,50 IDR/USD wird `15847` gespeichert – der Nutzer "verliert" bei jeder Transaktion 0.50 IDR pro Dollar. Bei einem €10.000-Trade sind das ~500 IDR Differenz (~$0.03). Kleiner Betrag, aber bei 1.000 Trades/Tag summiert sich das.

**Lösung:** Wechselkurse als `DECIMAL(18,6)` in der DB speichern und die gesamte FX-Berechnung als Decimal-Arithmetik durchführen (kein Float-Cast).

> **Status: 🟡 MITTEL. Funktioniert für den aktuellen Scale. Muss für den Marketplace auf Decimal-Arithmetik umgestellt werden.**

---

#### Frage 11: 🟡 Monitoring & Alerting – Wer bemerkt Probleme?

**Ist-Zustand:** Sentry ist konfiguriert (Fehler werden geloggt). Aber es gibt **keine aktiven Alerts** für:

| Was sollte alerten | Schwelle | Wen |
|---|---|---|
| **DB Connection Pool erschöpft** | >80% der Connections belegt für >30s | DevOps + Backend Lead |
| **Trade-Latenz >500ms** | P99 Latenz der `/api/marketplace/trade` Route | Backend Lead |
| **Wallet-Balance negativ** | balance_cents < 0 (sollte unmöglich sein) | SOFORT CEO + CTO |
| **Reconciliation-Mismatch** | >€1 Differenz | SOFORT CFO + CTO |
| **Error-Rate >5%** | >5% aller Requests returnieren 5xx | DevOps |
| **Redis nicht erreichbar** | Cloud Monitoring Redis Health | DevOps |
| **SSL-Zertifikat läuft ab** | <14 Tage bis Ablauf | DevOps |

> **Status: 🟡 HOCH. Cloud Monitoring Dashboards + Alert-Policies müssen VOR dem Marketplace-Launch aufgesetzt werden.**

---

#### Frage 12: 🟡 Testing-Strategie – Wie testen wir den Geld-Fluss?

**Ist-Zustand:** Es gibt `cargo test` (Unit-Tests) und E2E-Tests mit Playwright. Aber es gibt **keine dedizierten Finanztests**, die z.B. prüfen:

| Test | Beschreibung | Existiert? |
|---|---|---|
| **Double-Buy-Test** | Zwei parallele Checkout-Requests mit denselben Assets → nur einer erfolgreich | ❌ |
| **Insufficient-Balance-Test** | Withdrawal > Balance → muss cleaner Error sein, kein 500 | ❌ |
| **Reconciliation-Invariant-Test** | SUM(wallets) = SUM(deposits) − SUM(withdrawals) − SUM(purchases) | ❌ |
| **FX-Rounding-Test** | IDR-Conversion darf nie mehr als 1 Cent Differenz erzeugen | ❌ |
| **Concurrent-Trade-Test** | 10 parallele Trades auf denselben Asset → kein Oversell | ❌ |
| **Idempotency-Test** | Gleicher Request 2x gesendet → kein Doppeleffekt | ❌ |

> **Status: 🟡 HOCH. Eine dedizierte Financial-Test-Suite muss geschrieben werden, die alle oben genannten Szenarien abdeckt. Diese Tests sollten bei jedem Deploy automatisch laufen.**

---

> **GESAMTBEWERTUNG:** Von 12 kritischen Fragen sind **4 sofort kritisch** (🔴: Reconciliation, Withdrawal-Limits, Disaster-Recovery, Circuit Breaker) und **6 hoch-prioritär** (🟡: Connection Pool, Idempotency, GDPR, Monitoring, Testing, Key-Rotation). Das bestehende System hat starke Grundlagen (FOR UPDATE Locks, AppError, BIGINT-Cents), aber die Finanz-Best-Practices für eine Trading-Plattform sind noch nicht vollständig implementiert. Alle 4 roten Punkte müssen vor dem Marketplace-Launch geschlossen werden.

---

### 1.9. Connection Pool Auto-Scaling & Die Frage: Warum nicht 3 Datenbanken?

---

#### A. Auto-Scaling des Connection Pools (Industrie-Standard)

**Das Problem:** Der aktuelle Pool in `db.rs` hat `max_connections(10)` – fest, statisch, keine Skalierung. Wenn 80% belegt sind (8 Connections), wartet der nächste Request. Es gibt keinen Alarm und keine automatische Reaktion.

**Der Industrie-Standard: PgBouncer oder Cloud SQL Managed Connection Pooling (MCP)**

Es gibt zwei bewährte Lösungen, eine self-managed und eine Google-managed:

| Lösung | Typ | Beschreibung | Für POOOL? |
|---|---|---|---|
| **PgBouncer** | Open-Source Proxy | Sitzt zwischen App und DB. Multiplext Connections: 1.000 App-Connections → 50 echte DB-Connections. Industrie-Standard seit 15+ Jahren | ✅ Empfohlen (flexible, kostenlos, bewährt) |
| **Cloud SQL MCP** | Google Managed | Google-eigener Connection Pooler, auto-skaliert. Bis zu 5x Throughput, 85% weniger Latenz. Erfordert Enterprise Plus Edition | ⚠️ Teurer, aber weniger Wartung |

**Empfehlung: PgBouncer im Transaction-Mode als Sidecar auf Cloud Run**

```
┌─────────────────────────────────────────────────────────┐
│              Cloud Run Container                         │
│                                                         │
│  ┌──────────────┐       ┌──────────────┐                │
│  │ Rust Backend  │──────▶│  PgBouncer   │──────▶ Cloud SQL
│  │ (Axum)       │ n:1   │  (Sidecar)   │ m:1   │ (PostgreSQL)
│  │              │       │              │        │
│  │ SQLx Pool:   │       │ Pool:        │        │ max_connections:
│  │ max=100     │       │ default=30   │        │ 100
│  │ (virtuelle  │       │ reserve=10   │        │
│  │  Connections)│       │              │        │
│  └──────────────┘       └──────────────┘        │
└─────────────────────────────────────────────────────────┘
```

**Auto-Scaling Schwellenwerte (Industrie-Standard):**

| Metrik | Schwelle | Aktion |
|---|---|---|
| **Pool-Auslastung ≥ 80%** | `active_connections / max_connections ≥ 0.8` | ⚡ Cloud Run skaliert eine neue Instanz hoch (jede Instanz hat ihren eigenen PgBouncer + Pool). Dadurch verteilt sich die Last auf mehr Instanzen, statt den Pool zu vergrößern |
| **Pool-Auslastung ≥ 95%** | Fast alle Connections belegt | 🔴 Alert an DevOps. PgBouncer queued neue Requests (max 30s Wartezeit), statt sie sofort abzulehnen |
| **Pool-Auslastung ≤ 20%** | Pool fast leer | ⬇️ Cloud Run kann Instanz nach Cooldown-Periode herunterfahren (min-instances=1 bleibt immer) |
| **`cl_waiting` > 0 für > 5s** | PgBouncer-Queue läuft voll | 🟡 Warning: Pool-Size erhöhen oder Cloud Run max-instances hochsetzen |

**Die Formel für `default_pool_size` (pro PgBouncer-Instanz):**

```
default_pool_size = (Cloud SQL vCPUs × 2) + Disks / Anzahl Cloud Run Instanzen

Beispiel: Cloud SQL mit 2 vCPUs, 1 Disk, 3 Cloud Run Instanzen:
→ (2 × 2) + 1 = 5 → 5 / 1 (pro Instanz) = 5 (Minimum, wir runden auf 10)
→ Mit reserve_pool_size = 5 → effektiv 15 pro Instanz
→ Bei 5 Cloud Run Instanzen: 5 × 15 = 75 effektive Connections
→ Cloud SQL max_connections = 100 → Headroom für Admin-Tools ✅
```

**Konfiguration in `db.rs` (revidiert):**

```rust
// VORHER (zu klein, kein Scaling)
PgPoolOptions::new()
    .max_connections(10)         // ← Bottleneck!
    .acquire_timeout(Duration::from_secs(30))

// NACHHER (mit PgBouncer → kann höher sein, weil PgBouncer multiplexed)
PgPoolOptions::new()
    .max_connections(50)         // ← Virtuelle Connections zu PgBouncer
    .min_connections(5)          // ← Warm gehalten für sofortige Nutzung
    .acquire_timeout(Duration::from_secs(5))  // ← Schneller Timeout statt 30s Warten
    .idle_timeout(Duration::from_secs(120))   // ← Idle Connections schneller freigeben
```

> **Fazit:** Mit PgBouncer als Sidecar + Cloud Run Auto-Scaling löst sich das Connection-Problem automatisch: Mehr Last → Cloud Run startet neue Instanz → neuer PgBouncer-Pool → mehr Connections. Weniger Last → Instanzen werden heruntergefahren → weniger Connections. Die 80%-Schwelle ist der Trigger, den Cloud Run über CPU/Memory-Auslastung automatisch handhabt.

---

#### B. Kritische Analyse: 2 Datenbanken vs. 3 Datenbanken

**Deine Frage (zusammengefasst):** "Wenn die Community eine eigene DB bekommt, warum nicht auch der Marketplace? Der Marketplace braucht Millisekunden-Matching – warum teilt er sich die DB mit der Core-Plattform (Users, Wallets, Investments)?"

Das ist die wichtigste Architektur-Frage. Lass mich sie aus 3 Perspektiven beantworten:

---

**Perspektive 1: Warum die Community abgetrennt werden KANN**

Die Community hat eine entscheidende Eigenschaft: **Sie braucht keine atomare Konsistenz mit dem Core-System.**

Wenn ein Nutzer einen Post liked, passiert genau EINE Operation:
```
INSERT INTO community.reactions (post_id, user_id, type) VALUES (...);
```

Diese Operation braucht KEINEN Zugriff auf Wallets, Investments oder Orders. Der Post existiert unabhängig von der finanziellen Welt. Deshalb kann die Community in einer eigenen DB leben – die beiden Systeme sind *lose gekoppelt*.

---

**Perspektive 2: Warum der Marketplace NICHT abgetrennt werden kann**

Ein Trade auf dem Sekundärmarkt sieht so aus:

```
Nutzer B kauft 30 Shares von Nutzer A für je $105

Was ATOMAR (in EINER Transaktion) passieren MUSS:
────────────────────────────────────────────────────────
1. market_orders:    Seller A's Order updaten (remaining_quantity -= 30)
2. trade_history:    Trade-Eintrag erstellen
3. wallets:          Buyer B's Wallet: balance -= $3.150 (30 × $105)
4. wallets:          Seller A's Wallet: balance += $3.150
5. investments:      Buyer B bekommt 30 Shares (tokens_owned += 30)
6. investments:      Seller A verliert 30 Shares (tokens_owned -= 30)
7. wallet_transactions: Zwei Einträge (Kauf + Verkauf) loggen
8. audit_logs:       Trade dokumentieren
```

**Das Problem:** Steps 1-2 wären in einer "Marketplace-DB". Steps 3-8 müssten in der "Core-DB" passieren. Aber ALLE 8 Steps müssen **ATOMAR** sein – entweder passieren ALLE oder KEINER.

---

**📖 Was bedeutet "Atomar"? (Einfache Erklärung)**

Stell dir vor, du überweist €500 von deinem Konto auf das Konto eines Freundes. Dabei passieren zwei Dinge:
1. Dein Konto: -€500
2. Sein Konto: +€500

**"Atomar" bedeutet:** Diese zwei Schritte sind EIN unteilbarer Vorgang – wie ein Atom, das man (dachte man früher) nicht spalten kann. Es gibt nur zwei mögliche Ergebnisse:

| Ergebnis | Was passiert | OK? |
|---|---|---|
| ✅ **Alles klappt** | Dein Konto -€500, sein Konto +€500 | Perfekt |
| ✅ **Nichts klappt** | Dein Konto unverändert, sein Konto unverändert | OK – niemand verliert Geld |
| 🔴 **Halb klappt** | Dein Konto -€500, sein Konto UNVERÄNDERT | **KATASTROPHE – €500 verschwunden!** |

**Atomarität garantiert, dass der dritte Fall NIEMALS eintreten kann.** Egal ob der Server abstürzt, der Strom ausfällt oder die Festplatte explodiert – entweder passiert ALLES oder NICHTS. Nie etwas dazwischen.

**Das "A" in ACID:**

Jede seriöse Datenbank (PostgreSQL, Oracle, MySQL) befolgt die **ACID**-Regeln:

| Buchstabe | Bedeutung | EinfacheDeutsch |
|---|---|---|
| **A** – Atomicity | Unteilbarkeit | Alles oder nichts. Kein "halb fertig". |
| **C** – Consistency | Konsistenz | Die Daten sind immer gültig. Keine negativen Kontostände, keine fehlenden Einträge. |
| **I** – Isolation | Isolation | Gleichzeitige Transaktionen sehen sich nicht gegenseitig. Nutzer A sieht nie den Zwischenzustand von Nutzer B's Transaktion. |
| **D** – Durability | Dauerhaftigkeit | Einmal committed = für immer gespeichert. Auch nach einem Server-Neustart. |

**Warum ist das für POOOL lebenswichtig?**

Bei POOOL geht es um echtes Geld und echte Investments. Ein Trade hat 8 Schritte (Wallet abbuchen, Shares übertragen, Logs schreiben...). Wenn Step 3 (Geld abbuchen) klappt, aber Step 5 (Shares übertragen) fehlschlägt, hat der Käufer €3.150 bezahlt und KEINE Shares bekommen. Das ist kein Bug – das ist **Betrug durch Softwarefehler.**

**Wie funktioniert das im Code?**

```
BEGIN;                          ← Starte die atomare Klammer
  UPDATE wallets ... -€3.150;   ← Step 1
  UPDATE investments ... ;      ← Step 2
  INSERT trade_history ... ;    ← Step 3
  ...                           ← Steps 4-8
COMMIT;                         ← Alles hat geklappt → SPEICHERE ALLES

-- WENN irgendein Step fehlschlägt:
ROLLBACK;                       ← Mache ALLES rückgängig, als wäre nichts passiert
```

`BEGIN → COMMIT` ist die "atomare Klammer". Alles dazwischen ist ein unteilbarer Block. PostgreSQL garantiert: Wenn `COMMIT` erfolgreich ist, sind alle 8 Änderungen dauerhaft. Wenn irgendetwas schiefgeht, macht `ROLLBACK` alle Änderungen rückgängig.

**Vorteile von atomaren Transaktionen:**

| Vorteil | Erklärung |
|---|---|
| ✅ **Kein Geldverlust** | Unmöglich, dass Geld "verschwindet" (abgezogen aber nie gutgeschrieben) |
| ✅ **Kein Doppelkauf** | Unmöglich, dass zwei Käufer denselben Share bekommen (durch `FOR UPDATE`-Locks) |
| ✅ **Crash-Sicherheit** | Server kann mitten in der Transaktion abstürzen – PostgreSQL rollt beim Neustart automatisch zurück |
| ✅ **Einfacher Code** | Ein `BEGIN → COMMIT`-Block statt komplexe Fehlerbehandlung für jeden einzelnen Step |

**Nachteile von atomaren Transaktionen:**

| Nachteil | Erklärung | Unsere Lösung |
|---|---|---|
| ⚠️ **Geschwindigkeit** | Solange eine Transaktion läuft, sind die betroffenen Zeilen "gesperrt". Andere müssen warten | Redis als Speed-Layer: Matching ohne DB-Locks, DB nur für Settlement |
| ⚠️ **Nur innerhalb EINER Datenbank** | `BEGIN → COMMIT` funktioniert NUR in einer einzigen PostgreSQL-Instanz. Über zwei DBs hinweg braucht man teure "Distributed Transactions" | Deshalb bleiben Marketplace + Core in EINER DB |
| ⚠️ **Deadlocks möglich** | Wenn TX-A auf Zeile 1 wartet und TX-B auf Zeile 2, und beide die jeweils andere brauchen → Endlos-Blockade | PostgreSQL erkennt Deadlocks automatisch und bricht einen ab |

> **Das ist der Kern-Grund, warum der Marketplace NICHT in eine eigene Datenbank kann:** Atomare Transaktionen (`BEGIN → COMMIT`) funktionieren nur innerhalb EINER Datenbank. Ein Trade muss Wallets, Investments, Orders und Logs in EINEM atomaren Block ändern – also müssen alle diese Tabellen in der gleichen DB liegen.

---

> **🏁 ENTSCHEIDUNG (FINAL): 2 Datenbanken + Redis. Die 3-DB-Idee wurde kritisch analysiert und verworfen.**

**Was passiert bei 3 getrennten DBs:**

```
                    🔴 PROBLEM: Distributed Transaction nötig!
                    ╔══════════════════════════════════════╗
                    ║  Transaktion über 2 Datenbanken:     ║
                    ║                                      ║
┌─────────────┐     ║  1. BEGIN auf Marketplace-DB         ║     ┌─────────────┐
│ Marketplace │◀────║  2. UPDATE market_orders             ║────▶│  Core DB    │
│ DB (#3)     │     ║  3. INSERT trade_history             ║     │  (#1)       │
│             │     ║  4. BEGIN auf Core-DB                 ║     │             │
│ market_orders│    ║  5. UPDATE wallets (Buyer)            ║     │ wallets     │
│ trade_history│    ║  6. UPDATE wallets (Seller)           ║     │ investments │
│ price_data  │     ║  7. UPDATE investments (Buyer)        ║     │ users       │
│             │     ║  8. UPDATE investments (Seller)       ║     │ orders      │
└─────────────┘     ║  9. COMMIT auf Core-DB               ║     └─────────────┘
                    ║  10. COMMIT auf Marketplace-DB        ║
                    ║                                      ║
                    ║  ⚠️ Was wenn Step 9 erfolgreich,     ║
                    ║     aber Step 10 fehlschlägt?         ║
                    ║  → Geld bewegt, aber Trade nicht!    ║
                    ╚══════════════════════════════════════╝
```

**Die zwei Lösungen für Distributed Transactions – und warum beide schlecht sind:**

| Lösung | Beschreibung | Problem |
|---|---|---|
| **Two-Phase Commit (2PC)** | Beide DBs müssen erst "PREPARE" sagen, dann "COMMIT". Wenn eine ablehnt, wird alles zurückgerollt | 🔴 Blocking: Wenn eine DB zwischen PREPARE und COMMIT crashed, ist die andere DB *gesperrt* bis sie antwortet. Bei Geld: INAKZEPTABEL |
| **Saga Pattern** | Jede DB committed unabhängig. Bei Fehler: "Compensating Transactions" die rückgängig machen (z.B. "Erstatte $3.150 zurück") | 🔴 Extrem komplex: Jeder Step braucht eine Rückgängig-Funktion. Intermediate States sind sichtbar (Nutzer sieht kurz falsche Balance). Bei Geld: GEFÄHRLICH |

---

**Perspektive 3: Die richtige Architektur – Redis als Speed-Layer, PostgreSQL als Truth-Layer**

Die Lösung ist NICHT, den Marketplace in eine eigene DB zu stecken. Die Lösung ist, die *Geschwindigkeit* vom *Settlement* zu trennen:

```
┌──────────────────────────────────────────────────────────────────┐
│                      MARKETPLACE ARCHITEKTUR                     │
│                                                                  │
│  SPEED LAYER (Millisekunden)         TRUTH LAYER (ACID)          │
│  ┌────────────────────────┐         ┌──────────────────────┐     │
│  │    Redis (Memorystore)  │         │    Core DB (Cloud SQL)│     │
│  │                        │         │                      │     │
│  │  • Live Orderbook      │ Match!  │  • wallets           │     │
│  │  • Sorted Sets für     │────────▶│  • investments       │     │
│  │    Bid/Ask Preise      │ Settle  │  • market_orders     │     │
│  │  • Matching Engine     │ (Atomic │  • trade_history     │     │
│  │    (Single-Thread!)    │  TX)    │  • audit_logs        │     │
│  │  • Rate Limiting       │         │  • wallet_transactions│    │
│  │                        │         │                      │     │
│  │  Darf crashen →        │         │  Darf NIE             │     │
│  │  Wird aus PostgreSQL   │         │  inkonsistent sein!   │     │
│  │  wiederhergestellt     │         │                      │     │
│  └────────────────────────┘         └──────────────────────┘     │
│                                                                  │
│  ⚡ Latenz: <1ms (In-Memory)         💰 Latenz: 5-50ms (Disk)    │
│  🔄 Konsistenz: Eventual             ✅ Konsistenz: ACID          │
│  📊 Daten: Flüchtig (Cache)         📊 Daten: Persistent          │
└──────────────────────────────────────────────────────────────────┘
```

**Wie ein Trade WIRKLICH abläuft (2 Phasen):**

**Phase 1: Matching (Redis, Millisekunden, kein DB-Zugriff)**
```
1. Buyer B sendet Order: "Kaufe 30 Shares Asset X für max $105"
2. Redis Matching-Engine (Single-Thread) prüft:
   → Gibt es einen Ask ≤ $105? → JA: Seller A bietet 100 @ $105
3. Match gefunden! Redis schreibt Match-Event in eine Queue
4. → Keine DB-Connection verbraucht! Keine ACID nötig!
```

**Phase 2: Settlement (PostgreSQL, EINE atomare Transaktion)**
```
5. Background-Worker liest Match-Event aus Redis-Queue
6. EINE Transaktion auf der Core-DB:
   BEGIN;
     UPDATE wallets SET balance -= 3150 WHERE user_id = buyer_b;
     UPDATE wallets SET balance += 3150 WHERE user_id = seller_a;
     UPDATE investments ... (tokens transfer);
     INSERT INTO market_orders ... (status = 'filled');
     INSERT INTO trade_history ... ;
     INSERT INTO wallet_transactions ... (2 Einträge);
     INSERT INTO audit_logs ... ;
   COMMIT;
7. → ACID garantiert: Alles oder nichts! Kein Distributed Transaction!
```

**Warum das besser ist als 3 DBs:**

| Aspekt | 3 Datenbanken (Marketplace separat) | 2 DBs + Redis (unsere Architektur) |
|---|---|---|
| **Matching-Geschwindigkeit** | ⚠️ PostgreSQL: 5-50ms pro Match | ✅ Redis: <1ms pro Match (In-Memory, Single-Thread) |
| **Settlement-Atomizität** | 🔴 Distributed Transaction (2PC oder Saga) | ✅ Eine ACID-Transaktion in einer DB |
| **Code-Komplexität** | 🔴 Saga Pattern mit Compensating Transactions | ✅ Einfaches `pool.begin()` → 8 Queries → `tx.commit()` |
| **Daten-Konsistenz** | 🔴 Intermediate States sichtbar | ✅ Nie inkonsistent (Transaktion = alles oder nichts) |
| **Failure Recovery** | 🔴 Was wenn eine DB committed, die andere nicht? | ✅ Redis-Match-Queue wird replayed; DB ist die einzige Wahrheit |
| **Kosten** | ⚠️ 3 Cloud SQL Instanzen | ✅ 2 Cloud SQL + 1 Redis (billiger) |
| **DevOps-Overhead** | 🔴 3 DBs monitoren, backupen, skalieren | ✅ 2 DBs + Redis (Redis hat keine Daten die gebackupt werden müssen) |

---

**Zusammenfassung: Warum die 2-DB + Redis Architektur der Goldstandard ist**

```
┌─────────────────────────────────────────────────────┐
│              FINALE INFRASTRUKTUR                    │
│                                                     │
│  ┌─────────┐   ┌──────────┐   ┌──────────────────┐ │
│  │ Cloud   │   │ Redis    │   │ Community DB     │ │
│  │ SQL #1  │   │ Memory-  │   │ Cloud SQL #2     │ │
│  │ (Core)  │   │ store    │   │                  │ │
│  │         │   │          │   │ • posts          │ │
│  │ • users │   │ • order- │   │ • comments       │ │
│  │ • wallets│  │   book   │   │ • reactions      │ │
│  │ • invest-│  │ • match  │   │ • follows        │ │
│  │   ments │   │   queue  │   │ • badges         │ │
│  │ • orders│   │ • rate   │   │ • messages       │ │
│  │ • market_│  │   limits │   │                  │ │
│  │   orders│   │          │   │ Lose gekoppelt:  │ │
│  │ • trade_│   │ Speed    │   │ Nur user_id als  │ │
│  │   history│  │ Layer    │   │ Verbindung       │ │
│  │ • audit │   │          │   │                  │ │
│  │         │   │ Darf     │   │ Darf crashen     │ │
│  │ ABSOLUTE│   │ crashen, │   │ ohne Trades zu   │ │
│  │ PRIORITÄT│  │ wird     │   │ beeinflussen     │ │
│  │         │   │ rebuilt  │   │                  │ │
│  └─────────┘   └──────────┘   └──────────────────┘ │
│  + Read-Replica                + Read-Replica       │
│  + PITR Backup                 + PITR Backup        │
│                                                     │
│  Trade Flow:                                        │
│  User → API → Redis (Match) → Worker → Core DB     │
│                (< 1ms)                  (ACID TX)   │
└─────────────────────────────────────────────────────┘
```

> **Fazit:** Der Marketplace und die Core-Platform MÜSSEN in derselben Datenbank bleiben, weil ein Trade atomar Wallets, Investments, Orders und Audit-Logs ändern muss. Die *Geschwindigkeit* des Matchings wird nicht durch die Datenbank bestimmt, sondern durch Redis (In-Memory, <1ms). Die Community MUSS in einer eigenen DB leben, weil sie keine atomare Konsistenz mit den Finanzdaten braucht und ihr Traffic die Trades nie beeinflussen darf. Diese **2 DBs + Redis** Architektur ist exakt das, was Coinbase, Kraken und Robinhood in ähnlicher Form verwenden.

---

### 1.10. Progressive Kostenoptimierung: Klein starten, intelligent skalieren

Du hast absolut Recht: Es macht keinen Sinn, für 0 Nutzer eine Infrastruktur zu provisionieren, die für 1.000 Nutzer ausgelegt ist. Der Industrie-Standard ist **"Right-Sizing"** – starte mit dem Minimum und skaliere erst hoch, wenn Metriken es erfordern.

#### Phase 1: Launch (0-50 Nutzer) — ~$65-85/Monat

| Komponente | Spezifikation | Monatl. Kosten |
|---|---|---|
| **Core DB** | `db-f1-micro` (Shared CPU, 0.6GB RAM) | ~$10 |
| **Core DB Read-Replica** | `db-f1-micro` (von Anfang an testen!) | ~$10 |
| **Community DB** | `db-f1-micro` (Shared CPU, 0.6GB RAM) | ~$10 |
| **Community DB Read-Replica** | `db-f1-micro` (von Anfang an testen!) | ~$10 |
| **Redis** | `basic-M1` (1GB, KEIN Failover) | ~$25 |
| **Cloud Run** | 1 vCPU, 512MB, min 0 / max 3 | ~$0-15 (pay-per-use) |
| **GCS** | Pay-per-use | ~$1-5 |
| **PITR Backups** | Automatisch auf Cloud SQL | Inkludiert |
| **Gesamt** | | **~$65-85** |

> Die Read-Replicas kosten je nur ~$10/Monat als `db-f1-micro`. Für insgesamt $20 extra haben wir vom Tag 1 an die volle Infrastruktur getestet: Replica-Lag, Read/Write-Splitting, Failover-Szenarien. Das zu einem späteren Zeitpunkt einzurichten und dann erst zu testen wäre riskanter und teurer (Debugging unter Last).

#### Phase 2: Early Growth (50-100 Nutzer) — ~$120-170/Monat

**Trigger: Wenn die Cloud SQL CPU-Auslastung regelmäßig >70% ist ODER `acquire_timeout`-Fehler im Log auftauchen.**

| Änderung | Alt → Neu | Mehrkosten |
|---|---|---|
| **Core DB** | `db-f1-micro` → `db-g1-small` (1.7GB RAM) | +$15/Mo |
| **Core Read-Replica** | `db-f1-micro` → `db-g1-small` (mitscalieren) | +$15/Mo |
| **Redis** | `basic-M1` → `standard-M1` (+ Auto-Failover) | +$25/Mo |
| **Cloud Run** | min 0 → min 1 (kein Cold-Start mehr) | +$15/Mo |

#### Phase 3: Growth (100-1.000 Nutzer) — ~$300-460/Monat

**Trigger: Wenn >100 gleichzeitige Nutzer UND >500 Trades/Tag.**

| Änderung | Alt → Neu | Mehrkosten |
|---|---|---|
| **Core DB** | `db-g1-small` → `db-custom-2-4096` (2 vCPU, 4GB) | +$60/Mo |
| **Community DB** | `db-f1-micro` → `db-g1-small` | +$15/Mo |
| **Community Read-Replica** | ❌ → `db-f1-micro` | +$10/Mo |
| **PgBouncer** | ❌ → Als Sidecar in Cloud Run | $0 (Software) |
| **Cloud Run** | max 3 → max 10, 2 vCPU, 1GB | +$20-50/Mo |

#### Phase 4: Scale (1.000-5.000+ Nutzer) — ~$600-900/Monat

**Trigger: Wenn >1.000 gleichzeitige Nutzer UND >5.000 Trades/Tag.**

| Änderung | Alt → Neu | Mehrkosten |
|---|---|---|
| **Core DB** | `db-custom-2-4096` → `db-custom-4-8192` (4 vCPU, 8GB) | +$120/Mo |
| **Core Read-Replica** | `db-f1-micro` → `db-custom-2-4096` | +$50/Mo |
| **Community DB** | `db-g1-small` → `db-custom-2-4096` | +$50/Mo |
| **Cloud SQL HA** | ❌ → Core DB bekommt HA-Failover | +$120/Mo |
| **Cloud Armor (WAF)** | ❌ → Aktivieren gegen DDoS | +$10/Mo |

**Visualisierung der Kosten-Kurve:**

```
Monatliche Kosten ($)
│
900 ┤                                              ┌───── Phase 4: $600-900
    │                                         ╱
600 ┤                                    ╱
    │                               ╱
460 ┤                          ┌───── Phase 3: $300-460
    │                     ╱
300 ┤                ╱
    │           ╱
170 ┤      ┌───── Phase 2: $120-170
    │ ╱
 65 ┤─── Phase 1: $45-65
    │
  0 ┼──────┬──────┬──────┬──────┬──────▶ Nutzer
    0     50    100    500   1000   5000
```

**Skalierungs-Trigger (wann upgraden?):**

| Metrik | Schwelle | Aktion |
|---|---|---|
| **Cloud SQL CPU** | >70% für >10 Minuten | Nächsthöhere Instanz-Klasse |
| **Cloud SQL Connections** | >80% von `max_connections` | Pool-Size erhöhen oder Read-Replica hinzufügen |
| **Cloud SQL Storage** | >80% belegt | Auto-Scaling ist aktiviert – nur überwachen |
| **Cloud Run Latenz** | P95 >500ms | Mehr Instanzen (max-instances erhöhen) |
| **Cloud Run CPU** | >80% | CPU erhöhen (1 vCPU → 2 vCPU) |
| **Redis Memory** | >70% | Nächsthöhere Memory-Klasse |

> **Industrie-Standard:** AWS, Google und Azure empfehlen alle den 70-80% Threshold als Skalierungstrigger. Unter 70% → alles OK. 70-80% → beobachten. >80% → skalieren. >95% → Emergency.

> **Fazit Kosten:** Statt Day-1 $460/Monat zu zahlen, starten wir mit **$45-65/Monat** und skalieren nur hoch, wenn echte Metriken es erfordern. Der Break-Even-Punkt liegt bei ~100 Nutzern (Phase 2/3). Bis dahin zahlen wir weniger als ein Netflix-Abo für eine vollständige Finanz-Trading-Plattform mit Social-Media-Layer.

---

### 1.11. 2FA-Security-Architektur: Authentifizierung für Trades & Withdrawals

#### Was bereits existiert (TOTP-Infrastruktur)

Gute Nachricht: **Die 2FA-Infrastruktur ist bereits vollständig implementiert.** Der Code enthält:

| Komponente | Status | Datei |
|---|---|---|
| `totp-rs` (5.6) Library | ✅ Installiert | `Cargo.toml` |
| TOTP Secret Generation (QR-Code) | ✅ Implementiert | `auth/service.rs` → `generate_totp_secret()` |
| TOTP Verification | ✅ Implementiert | `auth/service.rs` → `verify_totp_code()` |
| Setup-Page (QR-Code anzeigen) | ✅ Implementiert | `GET /auth/2fa/setup` → `auth-2fa-setup.html` |
| Verification-Page (Code eingeben) | ✅ Implementiert | `GET /auth/2fa` → `auth-2fa.html` |
| Session-Flag `is_2fa_verified` | ✅ In DB & Session | `user_sessions.is_2fa_verified` |
| E-Mail-Benachrichtigung bei Setup | ✅ Implementiert | `email.rs` → Template `2fa_setup` |
| Admin kann 2FA-Status sehen | ✅ Implementiert | `admin/users.rs` → `totp_enabled` |

**Was fehlt: Step-Up Authentication für finanzielle Operationen.**

Aktuell wird 2FA nur beim **Login** verifiziert. Einmal eingeloggt, kann der Nutzer beliebig Geld abheben und traden – ohne erneute 2FA-Abfrage. Das ist das Sicherheitsrisiko.

#### Das Konzept: Step-Up Authentication

**Industrie-Standard** (Coinbase, Binance, Interactive Brokers): Bestimmte Aktionen erfordern eine **erneute** 2FA-Verifizierung, auch wenn der Nutzer bereits eingeloggt ist.

| Aktion | 2FA erforderlich? | Typ | Gültigkeitsfenster |
|---|---|---|---|
| **Login** | ✅ Ja (wenn 2FA aktiviert) | TOTP | Einmalig pro Session |
| **Withdrawal <$100** | ❌ Nein | – | – |
| **Withdrawal $100-$1.000** | ✅ Ja | TOTP | Code gültig für 30 Sekunden |
| **Withdrawal >$1.000** | ✅ Ja + 24h Cooling-Off | TOTP + E-Mail-Bestätigung | 24h Wartezeit vor Ausführung |
| **Trade <$500** | ❌ Nein (zu viel Friction) | – | – |
| **Trade $500-$5.000** | ✅ Ja | TOTP | Code gültig für 15 Minuten ("Trading-Session") |
| **Trade >$5.000** | ✅ Ja | TOTP + E-Mail-Bestätigung | Code gültig für 15 Minuten |
| **2FA-Einstellungen ändern** | ✅ Ja (Passwort + alter TOTP-Code) | Passwort + TOTP | Sofort |
| **Passwort ändern** | ✅ Ja | TOTP | Sofort |
| **Payment-Method hinzufügen** | ✅ Ja | TOTP | Sofort |

#### Die "Trading-Session" (Weniger Friction für aktive Trader)

**Problem:** Wenn ein Trader 20 Trades in 30 Minuten machen will, kann er nicht bei jedem Trade einen TOTP-Code eingeben. Das würde die UX zerstören.

**Lösung: 15-Minuten Trading-Session**

```
┌─────────────────────────────────────────────────────┐
│  TRADING SESSION FLOW                                │
│                                                     │
│  1. Nutzer will traden (>$500)                      │
│  2. Modal: "Bitte bestätigen Sie mit Ihrem          │
│     Google Authenticator Code"                       │
│  3. Nutzer gibt 6-stelligen Code ein                │
│  4. Server: Speichere Timestamp in Redis:            │
│     SET trading_session:{user_id} = NOW()            │
│     EXPIRE trading_session:{user_id} 900 (15 Min)    │
│  5. Für die nächsten 15 Minuten: Alle Trades         │
│     werden ohne erneute 2FA ausgeführt               │
│  6. Nach 15 Min: Neuer Code erforderlich            │
│                                                     │
│  ⚡ Ergebnis: 1x Code eingeben → 15 Min frei traden │
└─────────────────────────────────────────────────────┘
```

#### Implementierung im Rust-Backend (Konzept)

```rust
// Neuer Middleware-Guard: require_2fa_for_financial_ops
pub async fn require_step_up_2fa(
    session: &UserSession,
    redis: &RedisPool,
    action: FinancialAction, // Withdrawal, Trade, etc.
    amount_cents: i64,
) -> Result<(), AppError> {
    // 1. Prüfe ob 2FA für diesen Nutzer aktiviert ist
    if !session.totp_enabled {
        // 2FA nicht aktiviert → für jetzt durchlassen, aber:
        // Wenn amount > $1000, 2FA-Setup ERZWINGEN
        if amount_cents > 100_000 {
            return Err(AppError::Forbidden(
                "2FA is required for transactions over $1,000. Please enable 2FA in Settings.".into()
            ));
        }
        return Ok(());
    }

    // 2. Prüfe ob eine aktive Trading-Session existiert (Redis)
    let session_key = format!("trading_session:{}", session.user_id);
    if let Some(_) = redis.get::<Option<String>>(&session_key).await? {
        return Ok(()); // Trading-Session aktiv → durchlassen
    }

    // 3. Keine aktive Session → 2FA erforderlich
    Err(AppError::TwoFactorRequired)
    // → Frontend zeigt TOTP-Modal, sendet Code,
    //   Backend verifiziert und setzt Trading-Session in Redis
}
```

#### Pflicht vs. Optional: Wann wird 2FA erzwungen?

| Situation | 2FA-Status | Empfehlung |
|---|---|---|
| Neuer Nutzer, kein Investment | Optional (Empfehlung nach KYC) | Zeige Banner: "Schütze dein Konto mit 2FA" |
| Erstes Investment getätigt | Dringend empfohlen | Pop-up: "Du hast jetzt Geld investiert. Schütze es mit 2FA." |
| Wallet-Balance > $1.000 | **PFLICHT** | Redirect zu 2FA-Setup. Kein Trading/Withdrawal ohne 2FA |
| Marketplace-Trade | **PFLICHT** | Sekundärmarkt erfordert 2FA für alle Nutzer |
| Admin-Zugang | **PFLICHT** (bereits so) | Admins haben immer 2FA |

> **Fazit:** Die gesamte TOTP-Infrastruktur existiert bereits (Routes, QR-Code, Verification, Session-Flag). Was fehlt ist die **Step-Up Authentication**: Ein Middleware-Guard, der bei Withdrawals und Trades >$500 eine erneute 2FA-Verifizierung erzwingt, mit einem 15-Minuten Trading-Session-Window für aktive Trader. Die Implementierung ist ~200-300 Zeilen Rust-Code auf der existierenden Infrastruktur.

---

### 1.12. Financial & Smart Contract Testing Strategy

#### Warum spezielle Finanztests nötig sind

Standard-Unit-Tests (`assert_eq!`) können prüfen, ob eine Funktion das richtige Ergebnis liefert. Aber sie können NICHT prüfen:
- Ob zwei parallele Trades den gleichen Share doppelt verkaufen (Race Condition)
- Ob die Summe aller Wallets immer dem Geld im System entspricht (Invariante)
- Ob ein Smart Contract unter 10.000 zufälligen Inputs korrekt bleibt (Fuzz-Testing)

Dafür brauchen wir eine **mehrschichtige Test-Architektur**, die 4 Ebenen abdeckt:

```
┌──────────────────────────────────────────────────────────────┐
│              POOOL FINANCIAL TESTING PYRAMID                  │
│                                                              │
│          ┌──────────────┐                                    │
│         4│ Smart Contract│  Foundry (Solidity-native)        │
│          │ Invariant +   │  + Hardhat (JS Integration)       │
│          │ Fuzz Tests    │  10.000+ zufällige Inputs         │
│          └──────┬───────┘                                    │
│          ┌──────┴───────┐                                    │
│         3│ Cross-Layer   │  Backend ↔ Smart Contract         │
│          │ Integration   │  E2E Trade-Flow Tests             │
│          └──────┬───────┘                                    │
│       ┌─────────┴──────────┐                                 │
│      2│ Backend Financial   │  sqlx::test + proptest         │
│       │ Integration Tests   │  Concurrent Trades, Reconcil.  │
│       └─────────┬──────────┘                                 │
│    ┌────────────┴─────────────┐                              │
│   1│ Unit Tests (Rust)         │  Business Logic isoliert     │
│    │ + DB Schema Tests         │  cargo test                  │
│    └──────────────────────────┘                              │
└──────────────────────────────────────────────────────────────┘
```

---

#### Ebene 1: Rust Unit Tests + DB Schema Tests

**Framework:** `cargo test` + `sqlx::test` (Built-in)

`sqlx::test` ist der Industrie-Standard für Rust + PostgreSQL. Es erstellt automatisch eine **isolierte Test-Datenbank** für jeden Test, führt Migrations aus, und löscht sie danach. Kein Test kann einen anderen beeinflussen.

```rust
// tests/financial/wallet_tests.rs
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn test_deposit_credits_correct_amount(pool: PgPool) {
    // 1. Setup: Erstelle User + Wallet
    let user_id = create_test_user(&pool).await;
    create_wallet(&pool, user_id, "cash", "USD", 0).await;
    
    // 2. Execute: Bestätige Deposit von $500
    let deposit_ref = create_deposit(&pool, user_id, 50_000).await;
    confirm_deposit(&pool, &deposit_ref).await.unwrap();
    
    // 3. Assert: Wallet hat exakt $500
    let balance = get_wallet_balance(&pool, user_id, "cash", "USD").await;
    assert_eq!(balance, 50_000, "Balance should be exactly $500.00");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_deposit_idempotency_no_double_credit(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_wallet(&pool, user_id, "cash", "USD", 0).await;
    
    let deposit_ref = create_deposit(&pool, user_id, 50_000).await;
    
    // Bestätige denselben Deposit 3x
    confirm_deposit(&pool, &deposit_ref).await.unwrap();
    confirm_deposit(&pool, &deposit_ref).await.unwrap(); // Idempotent!
    confirm_deposit(&pool, &deposit_ref).await.unwrap(); // Idempotent!
    
    // Balance darf trotzdem nur $500 sein
    let balance = get_wallet_balance(&pool, user_id, "cash", "USD").await;
    assert_eq!(balance, 50_000, "Idempotent deposit must not double-credit");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_withdrawal_cannot_exceed_balance(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    create_wallet(&pool, user_id, "cash", "USD", 10_000).await; // $100
    
    // Versuch $200 abzuheben → muss fehlschlagen
    let result = execute_withdrawal(&pool, user_id, 20_000).await;
    assert!(result.is_err(), "Withdrawal exceeding balance must fail");
    
    // Balance unverändert
    let balance = get_wallet_balance(&pool, user_id, "cash", "USD").await;
    assert_eq!(balance, 10_000, "Balance must not change on failed withdrawal");
}
```

---

#### Ebene 2: Backend Financial Integration Tests (Concurrent + Reconciliation)

**Framework:** `sqlx::test` + `tokio::spawn` für parallele Execution + `proptest` für Property-Based Testing

**Test 2.1: Double-Buy Race Condition (KRITISCH)**

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_concurrent_checkout_no_oversell(pool: PgPool) {
    // Setup: Asset mit 10 verfügbaren Tokens
    let asset_id = create_test_asset(&pool, "Test Property", 10, 10_000).await; // 10 tokens @ $100
    
    // 5 Buyer, jeder will 3 Tokens → nur 10 verfügbar
    let mut handles = vec![];
    for i in 0..5 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let buyer = create_test_user_with_balance(&pool_clone, 50_000).await; // $500
            add_to_cart(&pool_clone, buyer, asset_id, 3).await;
            execute_checkout(&pool_clone, buyer, "USD", "wallet", None).await
        });
        handles.push(handle);
    }
    
    // Warte auf alle 5 parallelen Checkouts
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Zähle erfolgreiche + fehlgeschlagene
    let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
    let failures = results.iter().filter(|r| r.as_ref().unwrap().is_err()).count();
    
    // INVARIANTE: Maximal ⌊10/3⌋ = 3 Käufe dürfen erfolgreich sein
    assert!(successes <= 3, "Max 3 buyers can get 3 tokens from 10 available");
    assert!(successes + failures == 5, "All 5 attempts must resolve");
    
    // INVARIANTE: tokens_available darf nie < 0 werden
    let remaining: i32 = sqlx::query_scalar("SELECT tokens_available FROM assets WHERE id = $1")
        .bind(asset_id).fetch_one(&pool).await.unwrap();
    assert!(remaining >= 0, "tokens_available must NEVER go negative: got {}", remaining);
    assert_eq!(remaining as usize, 10 - (successes * 3), "Remaining tokens must match");
}
```

**Test 2.2: Reconciliation-Invariante (KRITISCH)**

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_reconciliation_invariant_after_full_lifecycle(pool: PgPool) {
    // 1. Setup: 3 Nutzer mit je $1.000 Einzahlung
    for _ in 0..3 {
        let user = create_test_user(&pool).await;
        deposit_and_confirm(&pool, user, 100_000).await; // $1.000
    }
    
    // 2. Nutzer 1 kauft Asset für $500
    let buyer = get_test_user(&pool, 0).await;
    add_to_cart_and_checkout(&pool, buyer, 50_000).await;
    
    // 3. Nutzer 2 hebt $300 ab
    let withdrawer = get_test_user(&pool, 1).await;
    execute_withdrawal(&pool, withdrawer, 30_000).await.unwrap();
    
    // 4. RECONCILIATION CHECK
    let total_balances: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance_cents), 0) FROM wallets WHERE wallet_type = 'cash'"
    ).fetch_one(&pool).await.unwrap();
    
    let total_deposits: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM deposit_requests WHERE status = 'paid'"
    ).fetch_one(&pool).await.unwrap();
    
    let total_withdrawals: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM withdrawal_requests WHERE status = 'completed'"
    ).fetch_one(&pool).await.unwrap();
    
    let total_purchases: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_cents), 0) FROM orders WHERE status = 'completed'"
    ).fetch_one(&pool).await.unwrap();
    
    // DIE GOLDENE INVARIANTE:
    assert_eq!(
        total_balances,
        total_deposits - total_withdrawals - total_purchases,
        "RECONCILIATION MISMATCH! Balances: {}, Expected: {} (deposits {} - withdrawals {} - purchases {})",
        total_balances, total_deposits - total_withdrawals - total_purchases,
        total_deposits, total_withdrawals, total_purchases
    );
}
```

**Test 2.3: Property-Based Testing mit `proptest`**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parse_dollars_to_cents_never_negative(
        input in "[0-9]{1,6}(\\.[0-9]{0,2})?"
    ) {
        let cents = parse_dollars_to_cents(&input);
        prop_assert!(cents >= 0, "parse_dollars_to_cents must never return negative for: {}", input);
    }
    
    #[test]
    fn test_format_usd_roundtrip(
        cents in 0i64..100_000_000i64 // $0 - $1.000.000
    ) {
        let formatted = format_usd(cents);
        // Must start with "USD " and contain exactly one "."
        prop_assert!(formatted.starts_with("USD "));
        prop_assert_eq!(formatted.matches('.').count(), 1);
    }
}
```

---

#### Ebene 3: Smart Contract Tests (Foundry + Hardhat)

Da POOOL ERC-3643 auf Base (Ethereum L2) verwendet, brauchen wir zwei Frameworks:

| Framework | Sprache | Stärke | Verwendung bei POOOL |
|---|---|---|---|
| **Foundry (forge)** | Solidity-native (Rust-based) | ⚡ Blitzschnell, eingebautes Fuzz-Testing, Invariant-Tests | Kern-Tests: Token-Transfer, Compliance, Ownership |
| **Hardhat** | JavaScript/TypeScript | 🔌 Pluginsystem, Mainnet-Forking, Frontend-Integration | Integration-Tests: Backend ↔ Smart Contract Flow |

**Test 3.1: Foundry – ERC-3643 Compliance Tests**

```solidity
// test/PooolToken.t.sol (Foundry)
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/PooolToken.sol";
import "../src/IdentityRegistry.sol";

contract PooolTokenTest is Test {
    PooolToken token;
    IdentityRegistry registry;
    
    address alice = makeAddr("alice");   // KYC'd investor
    address bob = makeAddr("bob");       // KYC'd investor
    address charlie = makeAddr("charlie"); // NOT KYC'd
    
    function setUp() public {
        registry = new IdentityRegistry();
        token = new PooolToken("POOOL Property 1", "PP1", registry);
        
        // Register KYC'd users
        registry.registerIdentity(alice, /* onchainID */ address(0x1));
        registry.registerIdentity(bob, /* onchainID */ address(0x2));
        // charlie is NOT registered → should not be able to receive tokens
        
        // Mint 1000 tokens to alice
        token.mint(alice, 1000);
    }
    
    function test_TransferToKYCdUser() public {
        vm.prank(alice);
        token.transfer(bob, 100);
        assertEq(token.balanceOf(bob), 100);
        assertEq(token.balanceOf(alice), 900);
    }
    
    function test_RevertTransferToNonKYCdUser() public {
        vm.prank(alice);
        vm.expectRevert("Transfer not compliant: receiver not verified");
        token.transfer(charlie, 100); // MUST revert!
    }
    
    function test_ForcedTransferByAdmin() public {
        // Simulate court order: transfer alice's tokens to treasury
        address treasury = makeAddr("treasury");
        registry.registerIdentity(treasury, address(0x99));
        
        token.forcedTransfer(alice, treasury, 500);
        assertEq(token.balanceOf(treasury), 500);
        assertEq(token.balanceOf(alice), 500);
    }
    
    function test_PauseBlocksAllTransfers() public {
        token.pause();
        vm.prank(alice);
        vm.expectRevert("Trading is paused");
        token.transfer(bob, 100);
    }
}
```

**Test 3.2: Foundry – Fuzz Testing (10.000+ zufällige Inputs)**

```solidity
contract PooolTokenFuzzTest is Test {
    PooolToken token;
    IdentityRegistry registry;
    
    function setUp() public { /* ... same as above ... */ }
    
    // Foundry generiert automatisch 256+ zufällige Werte für 'amount'
    function testFuzz_TransferNeverExceedsBalance(uint256 amount) public {
        // Bound: amount zwischen 0 und 10.000 (sinnvoller Bereich)
        amount = bound(amount, 0, 10_000);
        
        uint256 aliceBalance = token.balanceOf(alice);
        
        if (amount <= aliceBalance) {
            vm.prank(alice);
            token.transfer(bob, amount);
            assertEq(token.balanceOf(alice), aliceBalance - amount);
        } else {
            vm.prank(alice);
            vm.expectRevert();
            token.transfer(bob, amount);
        }
    }
    
    // INVARIANT: Gesamtsupply darf sich nur durch mint/burn ändern
    function testFuzz_TotalSupplyInvariant(uint256 amount) public {
        amount = bound(amount, 1, 1000);
        uint256 supplyBefore = token.totalSupply();
        
        vm.prank(alice);
        token.transfer(bob, amount);
        
        // Transfer darf totalSupply NICHT ändern
        assertEq(token.totalSupply(), supplyBefore, "Transfer must not change totalSupply");
    }
}
```

**Test 3.3: Foundry – Invariant Testing (Stateful Fuzz)**

```solidity
// Foundry's Invariant Tests: Führt HUNDERTE zufällige Funktionsaufrufe in zufälliger
// Reihenfolge aus und prüft nach JEDEM Aufruf ob die Invariante hält

contract PooolTokenInvariant is Test {
    PooolToken token;
    
    function setUp() public { /* ... */ }
    
    // Diese Invariante wird nach JEDER zufälligen Aktion geprüft:
    function invariant_balancesSumToTotalSupply() public {
        uint256 totalBalance = token.balanceOf(alice) + token.balanceOf(bob);
        assertEq(totalBalance, token.totalSupply(), 
            "Sum of all balances must always equal totalSupply");
    }
    
    function invariant_noNegativeBalances() public {
        // In Solidity sind uint256 immer >= 0, aber wir prüfen trotzdem
        // ob durch einen Bug ein Underflow entsteht
        assertTrue(token.balanceOf(alice) <= token.totalSupply());
        assertTrue(token.balanceOf(bob) <= token.totalSupply());
    }
}
```

**Test 3.4: Hardhat – Backend ↔ Smart Contract Integration**

```typescript
// test/integration/trade-settlement.test.ts (Hardhat + ethers.js)
import { expect } from "chai";
import { ethers } from "hardhat";

describe("Trade Settlement: Backend → Smart Contract", () => {
    it("should transfer tokens on-chain after backend trade settles", async () => {
        const [admin, seller, buyer] = await ethers.getSigners();
        
        // Deploy contracts
        const Token = await ethers.getContractFactory("PooolToken");
        const token = await Token.deploy("Property 1", "PP1", registry.address);
        
        // Seller has 100 tokens
        await token.connect(admin).mint(seller.address, 100);
        
        // Simulate: Backend matched a trade (30 tokens @ $105)
        // Backend calls smart contract to execute the on-chain transfer
        await token.connect(admin).forcedTransfer(
            seller.address, buyer.address, 30
        );
        
        expect(await token.balanceOf(seller.address)).to.equal(70);
        expect(await token.balanceOf(buyer.address)).to.equal(30);
    });
    
    it("should block transfer if buyer is not KYC'd on-chain", async () => {
        const [admin, seller, nonKYCBuyer] = await ethers.getSigners();
        // nonKYCBuyer is NOT in the IdentityRegistry
        
        await expect(
            token.connect(admin).forcedTransfer(seller.address, nonKYCBuyer.address, 10)
        ).to.be.revertedWith("Transfer not compliant: receiver not verified");
    });
});
```

---

#### Ebene 4: End-to-End Financial Flow Tests

**Framework:** Playwright (bereits vorhanden) + Custom Python/Rust Harness

```python
# tests/e2e/test_full_trade_lifecycle.py
import asyncio
import aiohttp

async def test_full_trade_lifecycle():
    """
    E2E: Deposit → Buy (Primary) → List on Marketplace → 
    Secondhand Buy → Withdrawal → Reconciliation Check
    """
    async with aiohttp.ClientSession() as session:
        # 1. Seller registriert + KYC + Deposit $1.000
        seller = await register_user(session, "seller@test.com")
        await complete_kyc(session, seller)
        await deposit_and_confirm(session, seller, 100_000)
        
        # 2. Seller kauft 10 Tokens auf dem Primärmarkt
        await add_to_cart(session, seller, asset_id="test-asset", qty=10)
        checkout = await execute_checkout(session, seller)
        assert checkout["status"] == "completed"
        
        # 3. Seller listet 5 Tokens auf dem Sekundärmarkt
        listing = await create_market_order(session, seller, 
            asset_id="test-asset", side="ask", qty=5, price=10500) # $105.00
        assert listing["status"] == "open"
        
        # 4. Buyer registriert + KYC + Deposit
        buyer = await register_user(session, "buyer@test.com")
        await complete_kyc(session, buyer)
        await deposit_and_confirm(session, buyer, 100_000)
        
        # 5. Buyer kauft 3 der 5 gelisteten Tokens
        trade = await create_market_order(session, buyer,
            asset_id="test-asset", side="bid", qty=3, price=10500)
        
        # 6. Prüfe Ergebnisse
        seller_portfolio = await get_portfolio(session, seller)
        buyer_portfolio = await get_portfolio(session, buyer)
        assert seller_portfolio["test-asset"]["tokens_owned"] == 7  # 10 - 3
        assert buyer_portfolio["test-asset"]["tokens_owned"] == 3
        
        # 7. RECONCILIATION CHECK
        reconciliation = await check_reconciliation(session)
        assert reconciliation["mismatch_cents"] == 0, \
            f"RECONCILIATION MISMATCH: {reconciliation}"
```

---

#### Test-Toolchain Übersicht

| Ebene | Tool | Sprache | Was wird getestet | Wann ausführen |
|---|---|---|---|---|
| **1. Unit** | `cargo test` | Rust | Business-Logic isoliert | Bei jedem `git push` |
| **2. Financial Integration** | `sqlx::test` + `proptest` | Rust | Race Conditions, Reconciliation, Invarianten | Bei jedem `git push` |
| **3a. Smart Contract (Core)** | `forge test` (Foundry) | Solidity | ERC-3643 Compliance, Fuzz, Invariants | Bei jeder Contract-Änderung |
| **3b. Smart Contract (Integration)** | `npx hardhat test` | TypeScript | Backend ↔ On-Chain Flow | Vor jedem Deploy |
| **4. E2E** | Playwright + Python | Python | Full User Journey | Vor jedem Production-Deploy |
| **5. Fuzz** | `cargo fuzz` + `forge fuzz` | Rust + Solidity | Edge Cases mit 10.000+ Inputs | Nightly CI-Run |

**CI/CD Pipeline:**

```
┌──────────────────────────────────────────────────────┐
│  GitHub Actions CI Pipeline                           │
│                                                      │
│  On Push:                                            │
│  ├── cargo fmt --check                               │
│  ├── cargo clippy                                    │
│  ├── cargo test                    (Ebene 1)         │
│  ├── cargo test --test financial   (Ebene 2)         │
│  └── forge test                    (Ebene 3a)        │
│                                                      │
│  On Pull Request (merge to main):                    │
│  ├── Alle obigen Tests                               │
│  ├── npx hardhat test             (Ebene 3b)         │
│  └── pytest tests/e2e/            (Ebene 4)          │
│                                                      │
│  Nightly (00:00 UTC):                                │
│  ├── cargo fuzz run -- -max_total_time=600           │
│  └── forge test --fuzz-runs 10000  (Ebene 5)         │
└──────────────────────────────────────────────────────┘
```

> **Fazit:** Die Financial-Test-Strategie hat 4+1 Ebenen. Die ersten zwei (Rust Unit + Financial Integration) nutzen `sqlx::test` – der Industrie-Standard für Rust + PostgreSQL, der automatisch isolierte Test-Datenbanken erstellt. Die Smart-Contract-Tests nutzen **Foundry** (schnell, Fuzz-Testing, Solidity-native) für die Kern-Logik und **Hardhat** (TypeScript, Plugin-Ökosystem) für die Integration mit dem Backend. Foundry's eingebautes Fuzz-Testing generiert automatisch tausende zufällige Inputs und ist damit der effektivste Weg, Edge Cases in ERC-3643-konformen Token-Contracts zu finden.

---

## 2. Die neue Markt-Architektur: Order Book & Trades

Um dynamische Preise zu erzeugen (Preise, die schwanken), benötigen wir eine Architektur wie bei einer modern skalierenden Börse (z.B. Coinbase oder Kraken). Basierend auf unserer Analyse in Sektion 1 steht fest: **2 Datenbanken (Core + Community) + Redis** ist die finale Infrastruktur. Der Marketplace lebt in der Core-DB (ACID-Garantie), das Matching läuft in Redis (Speed).

---

### 2.1. Überblick: Wie funktioniert ein Sekundärmarkt?

**Primärmarkt (existiert bereits):** POOOL verkauft Tokens direkt an Investoren. Ein fester Preis, ein Verkäufer.

**Sekundärmarkt (neu):** Investoren handeln untereinander. Der Preis entsteht durch Angebot und Nachfrage.

```
┌──────────────────────────────────────────────────────────────────────┐
│                    POOOL MARKETPLACE FLOW                            │
│                                                                      │
│  SELLER (hat Shares)              BUYER (will Shares)                │
│  ┌─────────────────┐              ┌─────────────────┐                │
│  │ "Verkaufe 30     │              │ "Kaufe 30 Shares │               │
│  │  Shares @ $105"  │              │  für max $105"   │               │
│  └────────┬────────┘              └────────┬────────┘                │
│           │ Ask-Order                      │ Bid-Order                │
│           ▼                                ▼                         │
│  ┌────────────────────────────────────────────────────┐              │
│  │              REDIS ORDERBOOK                        │              │
│  │                                                    │              │
│  │  Asks (Verkäufer):    Bids (Käufer):               │              │
│  │  $105.00 × 30 ← A    $105.00 × 30 ← B            │              │
│  │  $107.50 × 50 ← C    $103.00 × 20 ← D            │              │
│  │  $110.00 × 10 ← E    $100.00 × 100 ← F           │              │
│  │                                                    │              │
│  │  ⚡ MATCH! Ask $105 ≤ Bid $105                     │              │
│  └────────────────────┬───────────────────────────────┘              │
│                       │ Match-Event                                  │
│                       ▼                                              │
│  ┌────────────────────────────────────────────────────┐              │
│  │         POSTGRESQL SETTLEMENT (Core DB)             │              │
│  │                                                    │              │
│  │  BEGIN;                                            │              │
│  │    Buyer Wallet:  -$3,150                          │              │
│  │    Seller Wallet: +$3,150 (minus Fee)              │              │
│  │    Buyer Investment:  +30 Shares                   │              │
│  │    Seller Investment: -30 Shares                   │              │
│  │    trade_history: LOG                              │              │
│  │    audit_logs: LOG                                 │              │
│  │  COMMIT;                                          │              │
│  └────────────────────────────────────────────────────┘              │
│                       │                                              │
│                       ▼                                              │
│  ┌────────────────────────────────────────────────────┐              │
│  │         WEBSOCKET BROADCAST                        │              │
│  │  → Alle verbundenen Clients bekommen:              │              │
│  │    • Neuer Preis: $105.00                         │              │
│  │    • Order gelöscht aus dem Orderbook              │              │
│  │    • Candlestick-Update                           │              │
│  └────────────────────────────────────────────────────┘              │
└──────────────────────────────────────────────────────────────────────┘
```

---

### 2.2. Order-Typen

| Order-Typ | Beschreibung | Beispiel | Priorität |
|---|---|---|---|
| **Market Order** | "Kaufe sofort zum besten verfügbaren Preis" | Käufer will 30 Shares, egal zu welchem Preis | Phase 1 ✅ |
| **Limit Order** | "Kaufe nur wenn der Preis ≤ X ist" / "Verkaufe nur wenn ≥ Y" | Käufer will 30 Shares, aber maximal $105 | Phase 1 ✅ |
| **Stop-Loss Order** | "Verkaufe automatisch wenn der Preis unter X fällt" | Verkäufer will automatisch bei $95 verkaufen (Verlustbegrenzung) | Phase 2 ⏳ |
| **Good-Til-Cancelled (GTC)** | Order bleibt offen bis sie erfüllt oder manuell gelöscht wird | Standard-Verhalten | Phase 1 ✅ |
| **Immediate-or-Cancel (IOC)** | Order muss sofort (teilweise) erfüllt werden, Rest wird gelöscht | Für schnelle Trades | Phase 2 ⏳ |

**Phase 1 (MVP):** Nur Market Orders und Limit Orders. Das deckt 95% aller Use-Cases ab.

---

### 2.3. Redis Orderbook-Architektur (Speed Layer)

Das Orderbook lebt **komplett in Redis** – nicht in PostgreSQL. Warum? Eine Preisabfrage im Orderbook darf maximal ~1ms dauern (PostgreSQL: 5-50ms). Trader erwarten Echtzeit.

**Datenstruktur: Redis Sorted Sets (ZSET)**

```
Für jedes Asset existieren zwei Sorted Sets:

┌─────────────────────────────────────────────────────┐
│  asks:asset:{asset_id}    (Verkaufsangebote)         │
│  ─────────────────────────────────────────────       │
│  Score (Preis)  │  Member (Order-ID + Metadata)      │
│  ───────────────┼────────────────────────────        │
│  10500          │  order:abc123:user_A:30:1710900000 │
│  10750          │  order:def456:user_C:50:1710900100 │
│  11000          │  order:ghi789:user_E:10:1710900200 │
│                 │                                    │
│  Sortierung: NIEDRIGSTER Preis zuerst               │
│  → Käufer bekommen den billigsten Ask zuerst         │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│  bids:asset:{asset_id}    (Kaufangebote)             │
│  ─────────────────────────────────────────────       │
│  Score (Preis)  │  Member (Order-ID + Metadata)      │
│  ───────────────┼────────────────────────────        │
│  10500          │  order:xyz111:user_B:30:1710900050 │
│  10300          │  order:xyz222:user_D:20:1710900060 │
│  10000          │  order:xyz333:user_F:100:1710900070│
│                 │                                    │
│  Sortierung: HÖCHSTER Preis zuerst                  │
│  → Verkäufer bekommen den teuersten Bid zuerst       │
└─────────────────────────────────────────────────────┘
```

**Redis-Befehle im Detail:**

```redis
# Neue Sell-Order einfügen (Ask): User A verkauft 30 Shares @ $105.00
ZADD asks:asset:550e8400 10500 "order:abc123:user_A:30:1710900000"

# Neue Buy-Order einfügen (Bid): User B kauft 30 Shares @ $105.00
ZADD bids:asset:550e8400 10500 "order:xyz111:user_B:30:1710900050"

# Besten Ask abrufen (niedrigster Preis):
ZRANGEBYSCORE asks:asset:550e8400 -inf +inf LIMIT 0 1
→ "order:abc123:user_A:30:1710900000" (Score: 10500)

# Besten Bid abrufen (höchster Preis):
ZREVRANGEBYSCORE bids:asset:550e8400 +inf -inf LIMIT 0 1
→ "order:xyz111:user_B:30:1710900050" (Score: 10500)

# Match-Check: Bester Ask ≤ Bester Bid?
# 10500 ≤ 10500 → JA! MATCH!

# Nach Match: Order aus Redis entfernen
ZREM asks:asset:550e8400 "order:abc123:user_A:30:1710900000"
ZREM bids:asset:550e8400 "order:xyz111:user_B:30:1710900050"

# Aktuellen Spread anzeigen (für Frontend):
# Best Ask: ZRANGEBYSCORE asks:... -inf +inf LIMIT 0 1
# Best Bid: ZREVRANGEBYSCORE bids:... +inf -inf LIMIT 0 1
# Spread = Best Ask - Best Bid
```

**Warum Sorted Sets?**
- `ZADD` = O(log N) – auch bei 100.000 Orders blitzschnell
- `ZRANGEBYSCORE` = O(log N + M) – Top-10 Orders für die UI in <1ms
- Redis ist Single-Threaded → **keine Race Conditions** beim Matching
- Wenn Redis crasht → wird aus PostgreSQL (`market_orders` Tabelle) rebuildet

---

### 2.4. Die Matching-Engine (Das Herzstück)

**Algorithmus: Price-Time-Priority (FIFO)**

Das ist der Standard-Algorithmus, den NYSE, Coinbase und Kraken verwenden:
1. **Price Priority:** Der beste Preis gewinnt immer (niedrigster Ask, höchster Bid)
2. **Time Priority:** Bei gleichem Preis gewinnt die ältere Order (First-In, First-Out)

```rust
// Pseudo-Code der Matching-Engine (Rust)
// Läuft als Tokio-Task in einer Endlosschleife

async fn matching_engine(redis: &RedisPool, core_db: &PgPool) {
    loop {
        // Für jedes Asset mit offenen Orders:
        for asset_id in get_active_assets(redis).await {
            
            // 1. Hole besten Ask (niedrigster Verkaufspreis)
            let best_ask = redis.zrangebyscore(
                &format!("asks:asset:{}", asset_id), 
                "-inf", "+inf", 0, 1
            ).await;
            
            // 2. Hole besten Bid (höchster Kaufpreis)
            let best_bid = redis.zrevrangebyscore(
                &format!("bids:asset:{}", asset_id), 
                "+inf", "-inf", 0, 1
            ).await;
            
            // 3. Prüfe Match-Bedingung
            if best_ask.is_none() || best_bid.is_none() {
                continue; // Kein Match möglich
            }
            
            let ask = parse_order(best_ask);
            let bid = parse_order(best_bid);
            
            if ask.price_cents > bid.price_cents {
                continue; // Kein Match: Seller will mehr als Buyer zahlt
            }
            
            // ⚡ MATCH GEFUNDEN!
            let match_price = ask.price_cents; // Preis des Makers (Ask war zuerst)
            let match_qty = std::cmp::min(ask.remaining_qty, bid.remaining_qty);
            
            // 4. Settlement in PostgreSQL (ATOMAR!)
            match settle_trade(
                core_db, 
                &ask, &bid, 
                match_price, match_qty
            ).await {
                Ok(trade) => {
                    // 5. Orders in Redis aktualisieren
                    update_or_remove_order(redis, &ask, match_qty).await;
                    update_or_remove_order(redis, &bid, match_qty).await;
                    
                    // 6. WebSocket-Broadcast: Neuer Trade!
                    broadcast_trade(asset_id, &trade).await;
                    broadcast_orderbook_update(asset_id).await;
                },
                Err(e) => {
                    // Settlement fehlgeschlagen → Orders zurück ins Book
                    tracing::error!("Settlement failed: {}", e);
                    sentry::capture_error(&e);
                }
            }
        }
        
        // Kurze Pause um CPU nicht zu 100% auszulasten
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
```

**Teilausführungen (Partial Fills):**

Wenn Buyer 100 Shares will, aber nur ein Ask für 30 Shares existiert:

```
Vorher:
  Ask: User A verkauft 30 @ $105 (einziger Ask)
  Bid: User B kauft 100 @ $105

Match: 30 Shares @ $105 (Minimum von 30 und 100)

Nachher:
  Ask: GELÖSCHT (vollständig erfüllt)
  Bid: User B kauft 70 @ $105 (verbleibend: 100 - 30 = 70)
  
  → Bid bleibt offen im Orderbook bis weitere Asks kommen
```

---

### 2.5. Settlement-Pipeline (PostgreSQL ACID)

Wenn die Matching-Engine einen Match findet, wird das Settlement als EINE atomare Transaktion in der Core-DB ausgeführt:

```rust
async fn settle_trade(
    pool: &PgPool,
    ask: &MarketOrder,   // Seller
    bid: &MarketOrder,   // Buyer  
    match_price: i64,    // in Cents
    match_qty: i32,      // Anzahl Shares
) -> Result<Trade, String> {
    let total_cents = match_price * (match_qty as i64);
    
    // Dynamische Fee aus Kaskade (Promotion → Developer → Asset → Default)
    let fee_config = get_effective_fee(pool, ask.asset_id, ask.developer_id).await;
    let fee_cents = calculate_fee(total_cents, fee_config.taker_fee_bps);
    let seller_receives = total_cents - fee_cents;
    
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    
    // ══════════════════════════════════════════════════════════
    // ── Step 0: PRE-SETTLEMENT VALIDIERUNG (Gap-Fix) ──
    // ══════════════════════════════════════════════════════════
    
    // 0a. Asset noch handelbar?
    let asset_tradable: bool = sqlx::query_scalar(
        "SELECT trading_enabled FROM assets WHERE id = $1 FOR UPDATE"
    ).bind(ask.asset_id)
    .fetch_one(&mut *tx).await
    .map_err(|_| "Asset not found")?;
    
    if !asset_tradable {
        return Err("Asset trading is currently paused".into());
    }
    
    // 0b. Seller hat noch genügend Tokens? 
    // (Könnte seit Order-Submission über P2P verkauft haben)
    let seller_tokens: i32 = sqlx::query_scalar(
        "SELECT COALESCE(tokens_owned, 0) FROM investments 
         WHERE user_id = $1 AND asset_id = $2 FOR UPDATE"
    ).bind(ask.user_id).bind(ask.asset_id)
    .fetch_one(&mut *tx).await
    .map_err(|_| "Seller investment not found")?;
    
    if seller_tokens < match_qty {
        return Err("Seller no longer holds sufficient tokens".into());
    }
    
    // 0c. Beide Parteien noch KYC-verifiziert?
    let buyer_kyc: String = sqlx::query_scalar(
        "SELECT kyc_status FROM users WHERE id = $1"
    ).bind(bid.user_id).fetch_one(&mut *tx).await
    .map_err(|_| "Buyer not found")?;
    
    let seller_kyc: String = sqlx::query_scalar(
        "SELECT kyc_status FROM users WHERE id = $1"
    ).bind(ask.user_id).fetch_one(&mut *tx).await
    .map_err(|_| "Seller not found")?;
    
    if buyer_kyc != "approved" || seller_kyc != "approved" {
        return Err("KYC verification no longer valid for one or both parties".into());
    }
    
    // ── Step 1: Buyer's Wallet prüfen und belasten ──
    let buyer_wallet = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT id, balance_cents FROM wallets 
         WHERE user_id = $1 AND wallet_type = 'cash' FOR UPDATE"
    )
    .bind(bid.user_id)
    .fetch_one(&mut *tx).await
    .map_err(|_| "Buyer wallet not found")?;
    
    if buyer_wallet.1 < total_cents {
        return Err("Insufficient buyer balance".into());
    }
    
    sqlx::query("UPDATE wallets SET balance_cents = balance_cents - $1 WHERE id = $2")
        .bind(total_cents).bind(buyer_wallet.0)
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 2: Seller's Wallet gutschreiben (minus Fee) ──
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1 
         WHERE user_id = $2 AND wallet_type = 'cash'"
    )
    .bind(seller_receives).bind(ask.user_id)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 3: POOOL Treasury bekommt die Fee ──
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1 
         WHERE user_id = $2 AND wallet_type = 'fees'"
    )
    .bind(fee_cents).bind(POOOL_TREASURY_ID)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 4: Shares übertragen (Seller → Buyer) ──
    // Seller: tokens_owned -= match_qty
    sqlx::query(
        "UPDATE investments SET tokens_owned = tokens_owned - $1 
         WHERE user_id = $2 AND asset_id = $3 AND tokens_owned >= $1"
    )
    .bind(match_qty).bind(ask.user_id).bind(ask.asset_id)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // Buyer: tokens_owned += match_qty (upsert)
    sqlx::query(
        "INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, 
         current_value_cents, status)
         VALUES ($1, $2, $3, $4, $4, 'active')
         ON CONFLICT (user_id, asset_id) DO UPDATE 
         SET tokens_owned = investments.tokens_owned + $3,
             purchase_value_cents = investments.purchase_value_cents + $4,
             current_value_cents = investments.current_value_cents + $4"
    )
    .bind(bid.user_id).bind(ask.asset_id)
    .bind(match_qty).bind(total_cents)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 5: market_orders Status aktualisieren ──
    sqlx::query(
        "UPDATE market_orders SET quantity_filled = quantity_filled + $1, 
         status = CASE WHEN quantity_filled + $1 >= quantity THEN 'filled' 
                       ELSE 'partially_filled' END,
         updated_at = NOW()
         WHERE id = $2"
    )
    .bind(match_qty).bind(ask.order_id)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    sqlx::query(
        "UPDATE market_orders SET quantity_filled = quantity_filled + $1, 
         status = CASE WHEN quantity_filled + $1 >= quantity THEN 'filled' 
                       ELSE 'partially_filled' END,
         updated_at = NOW()
         WHERE id = $2"
    )
    .bind(match_qty).bind(bid.order_id)
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 6: Trade-History loggen (für Charts) ──
    let trade_id: Uuid = sqlx::query_scalar(
        "INSERT INTO trade_history (asset_id, buyer_user_id, seller_user_id, 
         price_cents, quantity, fee_cents, market_order_ask_id, market_order_bid_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id"
    )
    .bind(ask.asset_id).bind(bid.user_id).bind(ask.user_id)
    .bind(match_price).bind(match_qty).bind(fee_cents)
    .bind(ask.order_id).bind(bid.order_id)
    .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 7: Wallet-Transactions loggen ──
    sqlx::query(
        "INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, description)
         VALUES ($1, 'trade_buy', 'completed', $2, $3), 
                ($4, 'trade_sell', 'completed', $5, $6)"
    )
    .bind(buyer_wallet.0).bind(-total_cents)
    .bind(format!("Bought {} shares @ ${:.2}", match_qty, match_price as f64 / 100.0))
    .bind(seller_wallet_id).bind(seller_receives)
    .bind(format!("Sold {} shares @ ${:.2}", match_qty, match_price as f64 / 100.0))
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── Step 8: Audit-Log ──
    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'marketplace.trade', 'trade', $2, $3)"
    )
    .bind(bid.user_id).bind(trade_id)
    .bind(serde_json::json!({
        "buyer": bid.user_id, "seller": ask.user_id,
        "price_cents": match_price, "quantity": match_qty,
        "total_cents": total_cents, "fee_cents": fee_cents
    }))
    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    
    // ── COMMIT: Alles oder Nichts! ──
    tx.commit().await.map_err(|e| e.to_string())?;
    
    Ok(Trade { id: trade_id, price_cents: match_price, quantity: match_qty })
}
```

---

### 2.6. Fee-Struktur & Dynamisches Fee-Management

#### A. Standard-Fees (Defaults)

> **⚡ Stakeholder-Entscheidung E2:** Die Standard-Fee beträgt 5.0% (500 BPS), anpassbar via Admin-Dashboard. Tier-Rabatte reduzieren die Fee für Premium-Nutzer.

| Fee-Typ | Beschreibung | Default-Betrag | Wer zahlt |
|---|---|---|---|
| **Taker Fee** | Nutzer, der eine bestehende Order annimmt (Market Order) | **5.0%** (500 BPS) | Käufer |
| **Maker Fee** | Nutzer, der eine neue Order ins Buch stellt (Limit Order) | 0.0% (kostenlos) | – |
| **Listing Fee** | Erstmaliges Listen einer Sell-Order | $0 | – |
| **Withdrawal Fee** | Auszahlung auf Bankkonto | $2.50 flat | Seller |
| **P2P Trade Fee** | Direkte Angebote zwischen Nutzern | **5.0%** (500 BPS) | Käufer |

**Maker-Fee = 0%** ist Industrie-Standard bei neuen Börsen (Coinbase Zero, Robinhood) – es incentiviert Nutzer, Orders ins Buch zu stellen, was Liquidität schafft.

> **Hinweis:** Die 5.0%-Fee wird durch Tier-Rabatte reduziert (siehe Sektion 0, Entscheidung E2). Alle Fee-Werte sind über das Admin-Dashboard jederzeit anpassbar.

#### B. Dynamische Fee-Hierarchie (5 Ebenen – erweitert um Tier-Rabatt)

Fees sind NICHT statisch. POOOL braucht die Flexibilität, Gebühren je nach Situation anzupassen. Die Fees folgen einer **Kaskade** (höchste Priorität gewinnt):

```
┌─────────────────────────────────────────────────────────────┐
│  FEE-KASKADE (Priorität: höchste zuerst)                     │
│                                                             │
│  1. 🎯 Aktive Promotion         → z.B. "0% Fee bis 31.03" │
│     (Hat Start-/Enddatum, global oder asset-spezifisch)     │
│         │ Falls keine Promotion aktiv:                       │
│         ▼                                                   │
│  2. 🏢 Developer-Deal           → z.B. "Developer X: 0.3%" │
│     (Individueller Deal mit einem Developer/Anbieter)       │
│         │ Falls kein Developer-Deal:                        │
│         ▼                                                   │
│  3. 🏠 Asset-spezifische Fee    → z.B. "Asset Y: 3.0%"     │
│     (Premium-Assets, Sonder-Listings)                       │
│         │ Falls keine Asset-Fee:                            │
│         ▼                                                   │
│  4. ⭐ Tier-Rabatt (NEU)         → z.B. "Gold: -1.0%"      │
│     (Basierend auf dem Tier-Level des Users)                │
│     Wird auf die Basis-Fee angewendet (subtraktiv)          │
│         │ Falls kein Tier-Rabatt:                           │
│         ▼                                                   │
│  5. 🌐 Platform Default         → 5.0% Taker, 0% Maker     │
│     (Standard, gilt wenn nichts anderes konfiguriert)       │
└─────────────────────────────────────────────────────────────┘
```

> **Wichtig zur Tier-Rabatt-Logik:** Der Tier-Rabatt wird NACH der Basis-Fee-Ermittlung angewendet. Wenn eine Promotion (Level 1) die Fee auf 0% setzt, greift kein Tier-Rabatt. Wenn die Platform-Default 5% gilt und der User Gold-Tier hat, zahlt er 5% - 1% = 4%.

**Beispiele:**
- **Launch-Promotion:** "Erste 30 Tage: 0% Fee auf ALLE Trades" → Level 1 (Tier-Rabatt irrelevant)
- **Developer-Deal:** "Developer TrafficCreator bekommt 2% Fee für alle seine Assets" → Level 2
- **Premium-Asset:** "Luxury Villa Dubai: 3.0% Fee (Premium-Listing)" → Level 3
- **Gold-Tier User, kein Deal:** 5.0% Default - 1.0% Tier-Rabatt = **4.0%** → Level 4+5
- **Standard User, kein Deal:** 5.0% Default (Level 5)

#### C. Datenbank-Schema für dynamische Fees

```sql
-- Platform-weite und Asset-spezifische Fee-Konfiguration
CREATE TABLE fee_configurations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope           VARCHAR(20) NOT NULL CHECK (scope IN (
                        'platform',    -- Gilt für alle Assets
                        'asset',       -- Gilt für ein spezifisches Asset
                        'developer'    -- Gilt für alle Assets eines Developers
                    )),
    asset_id        UUID REFERENCES assets(id),
    developer_id    UUID REFERENCES users(id),
    taker_fee_bps   INTEGER NOT NULL DEFAULT 500,       -- 500 BPS = 5.00%
    maker_fee_bps   INTEGER NOT NULL DEFAULT 0,         -- 0 BPS = 0.00%
    withdrawal_fee_cents BIGINT NOT NULL DEFAULT 250,   -- $2.50
    p2p_fee_bps     INTEGER NOT NULL DEFAULT 500,
    listing_fee_cents BIGINT NOT NULL DEFAULT 0,
    reason          TEXT,                                -- "Developer Deal mit X"
    created_by      UUID REFERENCES users(id),
    is_active       BOOLEAN DEFAULT true,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (scope, asset_id, developer_id)
);

-- Zeitlich begrenzte Promotions (höchste Priorität)
CREATE TABLE fee_promotions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(100) NOT NULL,              -- "Launch Special: 0% Fees"
    description     TEXT,
    scope           VARCHAR(20) NOT NULL CHECK (scope IN ('global', 'asset')),
    asset_id        UUID REFERENCES assets(id),
    taker_fee_bps   INTEGER,                            -- NULL = Default beibehalten
    maker_fee_bps   INTEGER,
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ NOT NULL,
    created_by      UUID REFERENCES users(id),
    is_active       BOOLEAN DEFAULT true,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    CHECK (ends_at > starts_at)
);

-- Audit Trail: Wer hat wann welche Fee geändert?
CREATE TABLE fee_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_user_id   UUID REFERENCES users(id) NOT NULL,
    action          VARCHAR(30) NOT NULL,
    entity_type     VARCHAR(20) NOT NULL,
    entity_id       UUID NOT NULL,
    old_values      JSONB,
    new_values      JSONB,
    reason          TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

**Warum Basis-Punkte (BPS)?** 500 BPS = 5.00%. Integer-Arithmetik, keine Rundungsfehler. Industrie-Standard bei NYSE, Coinbase, allen Börsen.

#### D. Fee-Kaskade im Code (Rust)

```rust
/// Ermittelt die effektive Fee für einen Trade.
async fn get_effective_fee(pool: &PgPool, asset_id: Uuid, developer_id: Option<Uuid>) -> FeeConfig {
    // 1. Aktive Promotion? (höchste Priorität)
    if let Some(p) = sqlx::query_as::<_, FeePromotion>(
        "SELECT * FROM fee_promotions 
         WHERE is_active AND NOW() BETWEEN starts_at AND ends_at
         AND (scope = 'global' OR asset_id = $1)
         ORDER BY scope ASC LIMIT 1"  // asset-spezifisch vor global
    ).bind(asset_id).fetch_optional(pool).await.unwrap() {
        return FeeConfig::from_promotion(p);
    }
    // 2. Developer-Deal? → 3. Asset-Fee? → 4. Platform Default
    // (analog, Kaskade absteigend)
    FeeConfig::platform_default()
}

fn calculate_fee(total_cents: i64, base_fee_bps: i32, tier_discount_bps: i32) -> i64 {
    // Sicherstellen, dass die effektive Fee nie < 0 fällt (Max(0, base - discount))
    let effective_fee_bps = std::cmp::max(0, base_fee_bps - tier_discount_bps);
    (total_cents * effective_fee_bps as i64) / 10_000  // z.B. (500 BPS - 100 BPS) von $3,150 = $126.00
}
```

#### E. Admin Dashboard: Fee-Management Seite (`/admin/fees.html`)

```
┌──────────────────────────────────────────────────────────────┐
│  POOOL Admin > Fee Management                                │
│                                                              │
│  ┌── Platform Defaults ──────────────────────────────────┐   │
│  │  Taker Fee:     [5.00] %  ✏️                          │   │
│  │  Maker Fee:     [0.00] %  ✏️                          │   │
│  │  Withdrawal:    [$2.50]   ✏️                          │   │
│  │  P2P Fee:       [5.00] %  ✏️     [Speichern]         │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌── Aktive Promotions ──────────────────────────────────┐   │
│  │  🟢 "Launch Special: 0% Fees"                         │   │
│  │     Scope: Global | Taker: 0% | 01.04 - 30.04.2026   │   │
│  │     [Bearbeiten] [Deaktivieren]                       │   │
│  │                        [+ Neue Promotion erstellen]   │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌── Developer-Deals ────────────────────────────────────┐   │
│  │  Developer            Taker   Maker   Grund           │   │
│  │  TrafficCreator       0.00%   0.00%   "Partner-Deal"  │   │
│  │  RealEstatePro        0.30%   0.00%   "Volume-Deal"   │   │
│  │                           [+ Neuen Deal hinzufügen]   │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌── Asset-spezifische Fees ─────────────────────────────┐   │
│  │  Asset                Taker   Maker   Status          │   │
│  │  Luxury Villa Dubai   1.00%   0.00%   🟢 Custom       │   │
│  │  Berlin Apartment     5.00%   0.00%   ⚪ Default      │   │
│  │  (Klick → Inline-Edit)                                │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌── Audit Log ──────────────────────────────────────────┐   │
│  │  20.03 14:30  admin@poool.app → Promotion erstellt    │   │
│  │  19.03 10:15  admin@poool.app → Dev-Deal geändert     │   │
│  └───────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

#### F. Admin API-Endpunkte

| Methode | Route | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/admin/fees` | Alle Configs + Promotions | 🔒 Admin |
| `PUT` | `/api/admin/fees/platform` | Platform-Defaults ändern | 🔒 Superadmin |
| `POST` | `/api/admin/fees/asset/{id}` | Asset-spezifische Fee | 🔒 Admin |
| `POST` | `/api/admin/fees/developer/{id}` | Developer-Deal erstellen | 🔒 Superadmin |
| `POST` | `/api/admin/fees/promotions` | Neue Promotion | 🔒 Admin |
| `PUT` | `/api/admin/fees/promotions/{id}` | Promotion bearbeiten | 🔒 Admin |
| `DELETE` | `/api/admin/fees/promotions/{id}` | Promotion deaktivieren | 🔒 Admin |
| `GET` | `/api/admin/fees/audit-log` | Änderungs-Log | 🔒 Admin |

> **Sicherheitsregel:** Jede Fee-Änderung wird im `fee_audit_log` geloggt. Platform-Defaults und Developer-Deals → nur **Superadmin**. Asset-Fees und Promotions → auch reguläre Admins.

**Beispiel-Rechnung mit aktiver Promotion:**
```
Aktive Promotion: "Launch Special: 0% Taker Fee" (global)

Handelsvolumen:     30 × $105.00 = $3,150.00
Taker Fee (0.0%):   $0.00 (Promotion aktiv!)
POOOL erhält:       $0.00
Seller A erhält:    $3,150.00 (voller Betrag)
```

---

### 2.7. P2P / OTC Trades (Direkte Angebote)

Neben dem öffentlichen Orderbook gibt es private Angebote zwischen zwei bestimmten Nutzern:

```
┌──────────────────────────────────────────────────────┐
│  P2P TRADE FLOW                                      │
│                                                      │
│  1. Seller sieht in der Cap Table: "User B hat 20%"  │
│  2. Seller klickt "Direct Offer" auf User B          │
│  3. Modal: "Verkaufe 50 Shares für $110 an User B"   │
│  4. Server: INSERT INTO p2p_offers (...) STATUS=pending│
│  5. User B bekommt Notification                      │
│  6. User B prüft das Angebot und klickt "Accept"     │
│  7. Server: Selbes Settlement wie bei Orderbook-Trade│
│             (ACID Transaktion, Wallets, Investments)  │
│  8. Beide Nutzer sehen aktualisierte Portfolios      │
│                                                      │
│  ⚠️ P2P Trades gehen NICHT ins öffentliche Orderbook │
│  ⚠️ P2P Trades generieren trotzdem einen Tick        │
│     (→ beeinflusst den Marktpreis)                   │
└──────────────────────────────────────────────────────┘
```

**P2P-Regeln:**
- Angebote haben eine Ablaufzeit (Default: 48 Stunden)
- Buyer muss ausreichend Wallet-Balance haben (wird beim Accept geprüft)
- Beide Nutzer müssen KYC-verifiziert sein
- P2P-Trades generieren dieselbe Fee wie Orderbook-Trades

---

### 2.8. Preisfindung & Candlestick-Charts

Jeder Trade (egal ob Orderbook oder P2P) erzeugt einen **Tick**:

```sql
-- Jeder Trade generiert automatisch einen Tick in trade_history:
INSERT INTO trade_history (asset_id, price_cents, quantity, executed_at) 
VALUES ('asset_xyz', 10500, 30, NOW());

-- Candlestick-Aggregation (1-Stunden-Intervall):
SELECT
    date_trunc('hour', executed_at) AS period,
    MIN(price_cents) AS low,           -- Tiefster Preis in dieser Stunde
    MAX(price_cents) AS high,          -- Höchster Preis  
    (array_agg(price_cents ORDER BY executed_at ASC))[1] AS open,   -- Erster Trade
    (array_agg(price_cents ORDER BY executed_at DESC))[1] AS close, -- Letzter Trade
    SUM(quantity) AS volume            -- Gesamtes Handelsvolumen
FROM trade_history
WHERE asset_id = $1
GROUP BY date_trunc('hour', executed_at)
ORDER BY period DESC
LIMIT 168; -- Letzte 7 Tage (168 Stunden)
```

**Chart-Intervalle (Phase 1):**

| Intervall | Zweck | Datenquelle |
|---|---|---|
| **1 Minute** | Day-Trader (falls vorhanden) | Live-Query auf `trade_history` |
| **1 Stunde** | Intraday-Übersicht | Pre-aggregierte Tabelle `candles_1h` |
| **1 Tag** | Standard-Ansicht | Pre-aggregierte Tabelle `candles_1d` |
| **1 Woche** | Langzeit-Trend | Pre-aggregierte Tabelle `candles_1w` |

**Frontend-Integration:** `lightweight-charts.js` von TradingView (pure Vanilla JS, kein Framework, ~45KB).

---

### 2.9. WebSocket Live-Updates

Trader erwarten Echtzeit-Updates. Kein Neuladen der Seite.

```
┌──────────────────────────────────────────────────────┐
│  WEBSOCKET ARCHITEKTUR                                │
│                                                      │
│  Client (Browser)         Server (Axum + Tokio)      │
│  ┌─────────────────┐     ┌─────────────────┐        │
│  │ const ws = new   │────▶│ ws::upgrade()   │        │
│  │ WebSocket(       │     │                 │        │
│  │  '/ws/market/    │◀────│ Channels:       │        │
│  │   {asset_id}')   │     │ • orderbook     │        │
│  │                  │     │ • trades        │        │
│  │ ws.onmessage =  │     │ • ticker        │        │
│  │  (msg) => {     │     │                 │        │
│  │   updateUI(msg) │     │ Broadcast via   │        │
│  │  }              │     │ tokio::broadcast │        │
│  └─────────────────┘     └─────────────────┘        │
└──────────────────────────────────────────────────────┘
```

**Events die gebroadcastet werden:**

```json
// Event 1: Neuer Trade ausgeführt
{
    "type": "trade",
    "asset_id": "550e8400-...",
    "price": 10500,
    "quantity": 30,
    "buyer": "user_B",  // anonymisiert
    "timestamp": "2026-03-20T12:30:00Z"
}

// Event 2: Orderbook Update (neue Order oder gelöschte)
{
    "type": "orderbook",
    "asset_id": "550e8400-...",
    "bids": [
        {"price": 10300, "total_qty": 50},
        {"price": 10000, "total_qty": 100}
    ],
    "asks": [
        {"price": 10750, "total_qty": 50},
        {"price": 11000, "total_qty": 10}
    ],
    "spread": 450  // $4.50 Spread
}

// Event 3: Ticker (Zusammenfassung)
{
    "type": "ticker",
    "asset_id": "550e8400-...",
    "last_price": 10500,
    "change_24h": 250,     // +$2.50
    "change_pct": 2.44,    // +2.44%
    "volume_24h": 15000,   // 15.000 Shares gehandelt
    "high_24h": 10750,
    "low_24h": 10000
}
```

---

### 2.10. Circuit Breaker, Konzentrationslimits & Großorder-Handling ⏸️ (DEFERRED)

> **⏸️ ON HOLD:** Diese gesamte Sektion wird vorerst **NICHT implementiert**. Die Konzentrationslimits, das Großorder-Handling und der Circuit Breaker müssen noch weiter geprüft und an den Immobilien-Kontext angepasst werden. Die Inhalte bleiben als Entwurf erhalten und werden in einer späteren Phase entwickelt.

<details>
<summary>📋 Vollständiger Entwurf (zum späteren Prüfen aufklappen)</summary>

#### Warum Immobilien ≠ Aktien/Crypto

**POOOL ist KEIN Aktienmarkt.** Die Dynamiken sind fundamental anders:

| Merkmal | Crypto-Börse (Coinbase) | Immobilien-Markt (POOOL) |
|---|---|---|
| **Trades pro Tag/Asset** | 100.000+ | **1-10** |
| **Typische Ordergröße** | $50-$500 | **$1.000-$100.000** |
| **Preisvolatilität** | ±10% pro Stunde normal | **±1-3% pro Monat** |
| **Ein Nutzer kauft 80%** | 🔴 Marktmanipulation! | ✅ **Normaler Großinvestor** |
| **Preis springt 20%** | 🔴 Flash Crash! | ⚠️ Möglich bei Neubewertung |

**Das Problem:** Ein Circuit Breaker der bei ">10% Preisbewegung in 5 Minuten" auslöst, würde bei POOOL **bei jedem großen Trade** falschen Alarm schlagen. Wenn heute nur 2 Trades passieren und der zweite Trade den Preis um 15% nach oben bewegt (z.B. weil ein Gutachten den Immobilienwert erhöht hat), ist das KEIN Crash – das ist normale Marktbewertung.

#### A. Konzentrationslimits (Wem gehört wieviel?)

| Regel | Schwelle | Aktion | Grund |
|---|---|---|---|
| **Max-Konzentration** | Ein Nutzer darf max **80%** eines Assets besitzen | Order wird abgelehnt wenn 80% überschritten wird | Verhindert komplette Übernahme – min. 20% bleiben für andere Investoren |
| **Großorder-Schwelle** | Order betrifft >**20%** des gesamten Token-Supply | ⚠️ Order wird als "Pending Admin Review" markiert | Schützt vor Fehleingaben und erzwingt menschliche Prüfung bei großen Summen |
| **Ersteller-Lock** | Developer/Ersteller darf max **49%** zurückkaufen | Verhindert Rückkauf durch den Ersteller | Verhindert Interessenkonflikte |

```rust
/// Prüft Konzentrationslimits vor jeder Order
async fn check_concentration_limits(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    order_qty: i32,
) -> Result<(), OrderRejection> {
    let current_holding: i32 = sqlx::query_scalar(
        "SELECT COALESCE(tokens_owned, 0) FROM investments 
         WHERE user_id = $1 AND asset_id = $2"
    ).bind(user_id).bind(asset_id)
    .fetch_optional(pool).await?.unwrap_or(0);
    
    let total_supply: i32 = sqlx::query_scalar(
        "SELECT tokens_total FROM assets WHERE id = $1"
    ).bind(asset_id).fetch_one(pool).await?;
    
    let new_total = current_holding + order_qty;
    let concentration_pct = (new_total as f64 / total_supply as f64) * 100.0;
    
    if concentration_pct > 80.0 {
        return Err(OrderRejection::ConcentrationLimit {
            current_pct: (current_holding as f64 / total_supply as f64) * 100.0,
            requested_pct: concentration_pct,
            max_pct: 80.0,
        });
    }
    
    let order_pct = (order_qty as f64 / total_supply as f64) * 100.0;
    if order_pct > 20.0 {
        return Err(OrderRejection::RequiresAdminReview {
            order_pct,
            order_value_cents: order_qty as i64 * get_last_price(pool, asset_id).await?,
        });
    }
    
    Ok(())
}
```

**Beispiel:**
```
Asset: "Berlin Apartment", 1.000 Tokens, aktueller Preis $120/Token ($120.000 gesamt)

User A hat bereits 500 Tokens (50%)
User A will 350 weitere kaufen → Wäre 85% → 🔴 ABGELEHNT (Max 80%)
User A will 300 weitere kaufen → Wäre 80% → ✅ Erlaubt (genau am Limit)

User B (neu) will 250 kaufen (25%) → ⚠️ Großorder >20%
  → Geht in "Pending Admin Review" Warteschlange
  → Admin prüft → Admin klickt "Approve" → Order wird ins Orderbook gestellt
```

#### B. Immobilien-angepasster Circuit Breaker

Da die Handelsfrequenz niedrig ist (1-10 Trades/Tag statt 10.000/Sekunde), müssen die Zeithorizonte viel größer sein:

| Trigger | Schwelle | Aktion | Dauer |
|---|---|---|---|
| **Admin Alert** | Preis weicht >**15%** vom letzten Gutachten-Wert ab | ⚠️ E-Mail an Admins, Trading läuft weiter | Informativ |
| **Auto-Review** | Preis bewegt sich >**25%** innerhalb von **7 Tagen** | ⚠️ Neue Market Orders gehen in Admin-Queue | Admin muss freischalten |
| **Trading-Halt** | Preis bewegt sich >**40%** innerhalb von **30 Tagen** | 🔴 Trading für dieses Asset gestoppt | Admin-Review |
| **Global Halt** | Core DB oder Redis nicht erreichbar | 🔴 ALLE Trades gestoppt | Automatisch nach Recovery |

#### C. Großorder-Workflow (Admin-Genehmigung)

```
GROSSORDER FLOW (>20% eines Assets):
1. User reicht Order ein
2. Backend erkennt: >20% → Großorder!
3. Order wird mit status = 'pending_review' gespeichert
4. Admin bekommt Notification + E-Mail mit Review-Panel
5. Admin klickt "Genehmigen" → Order geht ins Orderbook
```

#### D. Konfigurierbare Limits (Admin-Dashboard)

| Parameter | Default | Regelbar pro Asset | Beispiel-Override |
|---|---|---|---|
| **Max Konzentration (%)** | 80% | ✅ | "Gewerbeimmobilie X: Max 100%" |
| **Großorder-Schwelle (%)** | 20% | ✅ | "Luxury Villa: 10%" |
| **Preis-Alert-Schwelle** | ±15% vs. Gutachten | ✅ | – |
| **Circuit Breaker (7-Tage)** | ±25% | ✅ | – |
| **Ersteller-Rückkauf-Limit** | 49% | ✅ | – |

</details>

---





### 2.11. On-Chain Settlement (ERC-3643 / Smart Contract)

Trades passieren **Off-Chain** (Redis + PostgreSQL) für Geschwindigkeit. Aber die **Eigentumsübertragung auf der Blockchain** passiert asynchron in Batches:

```
┌─────────────────────────────────────────────────────────┐
│  ON-CHAIN SETTLEMENT FLOW                                │
│                                                         │
│  Off-Chain (schnell, ~10ms)    On-Chain (dauerhaft)      │
│  ┌───────────────────────┐    ┌───────────────────────┐  │
│  │ Redis: Match           │    │ Base L2 Blockchain    │  │
│  │ PostgreSQL: Settlement │    │ ERC-3643 Token        │  │
│  │ → Sofortige Bestätigung│    │ → Unveränderlicher    │  │
│  │   für den User         │    │   Eigentumsnachweis   │  │
│  └───────────┬───────────┘    └───────────────────────┘  │
│              │                          ▲                 │
│              │  Settlement Batch        │                 │
│              │  (alle 1-24 Stunden)     │                 │
│              ▼                          │                 │
│  ┌───────────────────────────────────────┐               │
│  │  Rust Background Worker:               │               │
│  │  1. Sammle alle Trades seit letztem    │               │
│  │     Settlement                         │               │
│  │  2. Generiere Merkle Tree aller        │               │
│  │     Eigentumsänderungen                │               │
│  │  3. Sende Merkle Root → Base L2        │────────────┘  │
│  │  4. Führe forcedTransfer() auf         │               │
│  │     ERC-3643 Contract aus              │               │
│  │  5. Speichere TX-Hash in PostgreSQL    │               │
│  └────────────────────────────────────────┘               │
│                                                         │
│  Kosten pro Batch-Settlement auf Base L2: ~$0.01-0.10    │
│  (Hunderte Transfers in einer TX dank Batching)          │
└─────────────────────────────────────────────────────────┘
```

---

### 2.12. Zusammenfassung: Marketplace API-Endpunkte

| Methode | Route | Beschreibung | Auth | Rate Limit |
|---|---|---|---|---|
| `GET` | `/api/marketplace/{asset_id}/orderbook` | Aktuelles Orderbook (Top 20 Bids + Asks) | Public | 60/min |
| `GET` | `/api/marketplace/{asset_id}/trades` | Letzte 50 Trades (für Chart) | Public | 60/min |
| `GET` | `/api/marketplace/{asset_id}/ticker` | 24h Zusammenfassung (Preis, Volume, Change) | Public | 60/min |
| `GET` | `/api/marketplace/{asset_id}/candles?interval=1h` | Candlestick-Daten | Public | 30/min |
| `POST` | `/api/marketplace/orders` | Neue Order erstellen (Bid oder Ask) | 🔒 Auth + KYC + 2FA | **10/min** |
| `DELETE` | `/api/marketplace/orders/{order_id}` | Eigene Order stornieren | 🔒 Auth | **20/min** |
| `GET` | `/api/marketplace/orders/mine` | Eigene offene Orders | 🔒 Auth | 30/min |
| `POST` | `/api/marketplace/p2p/offer` | Privates P2P-Angebot senden | 🔒 Auth + KYC | 5/min |
| `POST` | `/api/marketplace/p2p/offer/{id}/accept` | P2P-Angebot annehmen | 🔒 Auth + KYC + 2FA | 5/min |
| `POST` | `/api/marketplace/p2p/offer/{id}/counter` | P2P Counter-Offer (Gegenangebot) | 🔒 Auth + KYC | 5/min |
| `GET` | `/api/marketplace/tax-report?year=2026` | Steuer-Report (alle Trades, CSV/JSON) | 🔒 Auth | 5/Stunde |
| `WS` | `/ws/market/{asset_id}` | WebSocket für Live-Updates | Public | 5 Conn/IP |
| `POST` | `/api/admin/marketplace/orders/{id}/approve` | Großorder genehmigen | 🔒 Admin | – |
| `POST` | `/api/admin/marketplace/orders/{id}/reject` | Großorder ablehnen | 🔒 Admin | – |

---

### 2.13. Order-Lifecycle, Sicherheitsregeln & Anti-Manipulation

> Dieser Abschnitt definiert alle Regeln die zwischen Order-Submission und Settlement greifen – das "Immunsystem" des Marktplatzes.

#### A. Balance-Hold System (Reservierung bei Order-Submit)

**Problem:** Zwischen Order-Submit und Settlement gibt es ein Zeitfenster. Ohne Reservierung könnte ein User:
1. $10.000 Balance haben
2. Eine Buy-Order für $10.000 erstellen
3. Gleichzeitig $10.000 per Withdrawal abheben
4. Settlement schlägt fehl → Seller bekommt kein Geld

**Lösung: Wallets bekommen ein zweites Feld `held_balance_cents`:**

```sql
-- Neues Feld in wallets Tabelle
ALTER TABLE wallets ADD COLUMN held_balance_cents BIGINT NOT NULL DEFAULT 0;

-- Verfügbare Balance = balance_cents - held_balance_cents
-- Zum Abheben/neueOrders verfügbar: balance_cents - held_balance_cents
```

```rust
// Beim ORDER-SUBMIT (Buy-Order):
sqlx::query(
    "UPDATE wallets 
     SET balance_cents = balance_cents - $1,
         held_balance_cents = held_balance_cents + $1
     WHERE user_id = $2 AND wallet_type = 'cash' 
     AND (balance_cents - held_balance_cents) >= $1"  // Nur verfügbare Balance!
).bind(order_total_cents).bind(user_id)
.execute(pool).await?;

// Beim SETTLEMENT: held_balance → Seller
sqlx::query(
    "UPDATE wallets 
     SET held_balance_cents = held_balance_cents - $1
     WHERE user_id = $2"  // Buyer: Hold auflösen (Geld geht an Seller)
).bind(total_cents).bind(bid.user_id)
.execute(&mut *tx).await?;

// Beim CANCEL: held_balance → zurück zu verfügbar
sqlx::query(
    "UPDATE wallets 
     SET balance_cents = balance_cents + $1,
         held_balance_cents = held_balance_cents - $1
     WHERE user_id = $2"
).bind(order_total_cents).bind(user_id)
.execute(pool).await?;
```

**Analog für Sell-Orders:** Seller's Tokens werden beim Order-Submit in einer `held_tokens` Spalte der `investments`-Tabelle reserviert:
```sql
ALTER TABLE investments ADD COLUMN held_tokens INTEGER NOT NULL DEFAULT 0;
-- Verfügbare Tokens = tokens_owned - held_tokens
```

---

#### B. Order-Expiry (Ablauf offener Orders)

**Regel:** Alle offenen Orders haben ein maximales Alter. Danach werden sie automatisch storniert.

| Order-Typ | Default-Expiry | Konfigurierbar |
|---|---|---|
| **Limit Order (GTC)** | 90 Tage | ✅ per Asset |
| **Market Order** | 24 Stunden (wenn nicht sofort ausführbar) | ❌ |
| **P2P Offer** | 48 Stunden | ✅ per Offer |
| **Großorder (Pending Review)** | 7 Tage (Admin muss reagieren) | ❌ |

**Implementierung: Täglicher Cleanup-Job:**

```rust
// Cron-Job: Läuft 1x pro Stunde
async fn expire_stale_orders(pool: &PgPool, redis: &RedisPool) {
    // 1. Abgelaufene Orders in PostgreSQL finden
    let expired = sqlx::query_as::<_, MarketOrder>(
        "UPDATE market_orders 
         SET status = 'expired', updated_at = NOW()
         WHERE status IN ('open', 'partially_filled')
         AND expires_at < NOW()
         RETURNING *"
    ).fetch_all(pool).await?;
    
    for order in &expired {
        // 2. Aus Redis Orderbook entfernen
        let key = if order.side == "sell" {
            format!("asks:asset:{}", order.asset_id)
        } else {
            format!("bids:asset:{}", order.asset_id)
        };
        redis.zrem(&key, &order.redis_member()).await?;
        
        // 3. Balance-Hold zurückgeben
        release_hold(pool, order).await?;
        
        // 4. User benachrichtigen
        notify_user(order.user_id, "Deine Order ist abgelaufen").await?;
    }
    
    tracing::info!("Expired {} stale orders", expired.len());
}
```

**Neues Feld in `market_orders`:**
```sql
ALTER TABLE market_orders ADD COLUMN expires_at TIMESTAMPTZ 
    DEFAULT NOW() + INTERVAL '90 days';
```

---

#### C. Wash Trading Prevention (Anti-Manipulation)

**Wash Trading** = Ein Nutzer handelt gegen sich selbst um künstliches Volumen zu erzeugen. Bei der OJK und jeder regulierten Börse **illegal**.

| Regel | Prüfung | Aktion |
|---|---|---|
| **Self-Trade Block** | `buyer_user_id == seller_user_id` | 🔴 Order wird automatisch blockiert |
| **Same-IP Alert** | Gleiche IP bei Buyer und Seller | ⚠️ Admin-Alert + Trade wird markiert |
| **Volume-Anomalie** | Ein User macht >50% des Tagesvolumens eines Assets | ⚠️ Täglicher Cron-Alert an Admins |
| **Rapid-Fire Detection** | >5 Orders pro Minute auf das gleiche Asset | ⚠️ Rate-Limit + Admin-Alert |

```rust
// Im Matching-Engine VOR dem Settlement:
async fn check_wash_trading(
    ask: &MarketOrder, 
    bid: &MarketOrder,
    redis: &RedisPool,
) -> Result<(), WashTradingDetected> {
    // 1. Gleicher User?
    if ask.user_id == bid.user_id {
        return Err(WashTradingDetected::SelfTrade);
    }
    
    // 2. Gleiche IP? (IP beim Order-Submit in Redis gespeichert)
    let ask_ip: Option<String> = redis.get(
        &format!("order_ip:{}", ask.order_id)
    ).await?;
    let bid_ip: Option<String> = redis.get(
        &format!("order_ip:{}", bid.order_id)
    ).await?;
    
    if ask_ip.is_some() && ask_ip == bid_ip {
        // Nicht blockieren, aber Alert senden
        sentry::capture_message(
            &format!("Wash Trading Verdacht: Same IP {} für Ask {} und Bid {}", 
                ask_ip.unwrap(), ask.order_id, bid.order_id),
            sentry::Level::Warning,
        );
    }
    
    Ok(())
}
```

---

#### D. Minimum & Maximum Order Sizes

| Parameter | Default | Konfigurierbar | Grund |
|---|---|---|---|
| **Min Order Value** | $10.00 (1000 Cents) | ✅ per Asset | Verhindert Spam-Orders mit Kleinstbeträgen |
| **Min Order Quantity** | 1 Token | ❌ | Tokens sind unteilbar |
| **Max Order Value (ohne Review)** | $50.000 | ✅ per Asset | >$50k → Großorder → Admin-Review |
| **Max Orders pro User pro Asset** | 10 offene Orders gleichzeitig | ✅ global | Verhindert Orderbook-Spam |

```rust
// Beim Order-Submit:
fn validate_order_size(order: &NewOrder, asset: &Asset) -> Result<(), OrderRejection> {
    let total_cents = order.price_cents * order.quantity as i64;
    
    if total_cents < asset.min_order_value_cents.unwrap_or(1000) {
        return Err(OrderRejection::BelowMinimum { 
            min_cents: 1000, actual_cents: total_cents 
        });
    }
    
    if order.quantity < 1 {
        return Err(OrderRejection::InvalidQuantity);
    }
    
    Ok(())
}
```

---

#### E. Cancel-Timing Lock (Race-Condition-Schutz)

**Problem:** User klickt "Cancel Order" genau in dem Moment wo die Matching-Engine die Order matched. Ohne Lock → doppelte Auszahlung oder inkonsistenter State.

**Lösung: Redis-Lock mit 5-Sekunden TTL:**

```rust
// Beim Cancel-Request:
async fn cancel_order(redis: &RedisPool, pool: &PgPool, order_id: Uuid) -> Result<()> {
    // 1. Lock setzen (atomisch mit NX = "nur wenn nicht existiert")
    let lock_key = format!("lock:order:{}", order_id);
    let locked: bool = redis.set_nx(&lock_key, "cancelling", 5).await?;
    
    if !locked {
        return Err("Order is currently being processed".into());
    }
    
    // 2. Order aus Redis entfernen
    // ... (ZREM)
    
    // 3. Status in PostgreSQL setzen
    sqlx::query("UPDATE market_orders SET status = 'cancelled' WHERE id = $1")
        .bind(order_id).execute(pool).await?;
    
    // 4. Balance-Hold zurückgeben
    release_hold(pool, &order).await?;
    
    // 5. Lock aufheben
    redis.del(&lock_key).await?;
    
    Ok(())
}

// In der Matching-Engine (VOR jedem Match):
let lock_key = format!("lock:order:{}", ask.order_id);
if redis.exists(&lock_key).await? {
    continue; // Order wird gerade gecancelt → überspringen
}
```

---

#### F. Rate-Limiting für Marketplace-Endpoints

| Endpoint-Kategorie | Limit | Mechanismus |
|---|---|---|
| **Public Reads** (Orderbook, Trades, Ticker) | 60 Requests/Min/IP | Redis Counter |
| **Order Submission** | 10 Orders/Min/User | Redis Counter + User-ID |
| **Order Cancel** | 20 Cancels/Min/User | Redis Counter |
| **P2P Offers** | 5 Offers/Min/User | Redis Counter |
| **WebSocket Connections** | 5 pro IP | Connection Counter |
| **WebSocket Subscriptions** | 10 Assets pro Connection | Server-side Limit |

```rust
// Redis-basiertes Rate-Limiting
async fn check_rate_limit(
    redis: &RedisPool, 
    key: &str,      // z.B. "rl:orders:user:{user_id}"
    max: u32,       // z.B. 10
    window_secs: u32 // z.B. 60
) -> Result<(), RateLimited> {
    let count: u32 = redis.incr(key).await?;
    if count == 1 {
        redis.expire(key, window_secs).await?;
    }
    if count > max {
        return Err(RateLimited { retry_after_secs: redis.ttl(key).await? });
    }
    Ok(())
}
```

---

#### G. Dividend-Stichtag-Regel

**Frage:** Wer bekommt die Dividende wenn Tokens gerade im Orderbook zum Verkauf stehen?

**Regel (Industrie-Standard "Record Date"):**

> **Tokens in offenen Sell-Orders gehören weiterhin dem Seller.** Dividenden werden basierend auf dem `investments.tokens_owned` Feld berechnet (nicht auf Basis des Orderbooks). Erst nach einem vollständig abgewickelten Trade (Settlement) wechselt das Dividendenrecht.

| Status | Dividende an | Grund |
|---|---|---|
| Tokens im Wallet (keine Order) | ✅ Owner | Normal |
| Tokens in offener Sell-Order | ✅ Seller (noch Owner) | Settlement ist noch nicht passiert |
| Trade settled, On-Chain-Batch ausstehend | ✅ Buyer (neuer Owner) | PostgreSQL ist Source of Truth |

---

### 2.14. Regulatorische Compliance (OJK Indonesien)

> ⚠️ **POOOL hat seinen Sitz in Indonesien.** Seit dem 10. Januar 2025 reguliert die **OJK (Otoritas Jasa Keuangan)** alle digitalen Finanzassets – einschließlich tokenisierter Immobilien. Ohne korrekte Lizenzierung riskiert POOOL eine **Betriebsuntersagung**.

#### A. Regulatorischer Rahmen (Stand 2026)

| Regulation | Datum | Relevanz für POOOL |
|---|---|---|
| **OJK Regulation 27/2024** | 10.01.2025 | Grundsätzliche Regulierung aller Digital Financial Assets (DFA) |
| **OJK Circular 20/2024** | 01/2025 | Technische Compliance-Guidelines für DFA-Plattformen |
| **Draft OJK Regulation (AKD)** | 09/2025 | Spezifisch für Public Offerings von tokenisierten Assets |
| **OJK Regulation 23/2025** | 12/2025 | Verschärfte Travel Rule, DFA-Derivate-Klassifizierung |

#### B. Pflichten für POOOL als DFA-Plattform

| Pflicht | Beschreibung | Status | Handlungsbedarf |
|---|---|---|---|
| **PT-Registrierung** | Anbieter muss eine indonesische PT (Perseroan Terbatas) sein | ❓ Klären | Rechtsanwalt in Jakarta konsultieren |
| **OJK-Lizenz** | Lizenz für den Handel mit Digital Financial Assets | ❓ Klären | Antragsprozess starten |
| **OJK-Genehmigung** | Angebote ≥ IDR 1 Mrd (~$60.000) brauchen OJK-Genehmigung | ❌ Fehlt | Pro Asset prüfen ob Schwelle erreicht |
| **Segregated Accounts** | Kundengelder müssen getrennt von Firmengeldern sein | ❌ Fehlt | Eigenes Bankkonto für Client Funds |
| **Travel Rule** | Herkunft + Ziel aller Transfers dokumentieren | ❌ Fehlt | Implementieren im Settlement-Log |
| **Consumer Protection** | Risiko-Hinweise, Investment-Disclaimer, Info-Docs mit OJK-Genehmigung | ❌ Fehlt | Legal-Team erstellt Dokumente |
| **AML/KYC** | Anti-Money-Laundering, Know-Your-Customer | ✅ Vorhanden | Didit.me bereits integriert |
| **Data Protection (UU PDP)** | Indonesisches Datenschutzgesetz (nicht nur GDPR) | ⚠️ Teilweise | Privacy Policy für UU PDP anpassen |
| **OJK Reporting** | Regelmäßige Reports über Handelsvolumen, Nutzer, Incidents | ❌ Fehlt | Reporting-API/Dashboard für OJK bauen |
| **IT Security Standards** | Governance, Kapitalanforderungen, IT-Sicherheit | ⚠️ Teilweise | Security Audit durchführen |

#### C. Technische Implementierung für OJK-Compliance

**1. Travel Rule – Wer schickt wem was?**
```sql
-- Neue Spalten in trade_history oder eigene Tabelle
CREATE TABLE travel_rule_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trade_id        UUID REFERENCES trade_history(id),
    
    -- Sender (Seller)
    sender_user_id  UUID REFERENCES users(id),
    sender_name     TEXT NOT NULL,           -- Vollständiger Name aus KYC
    sender_id_type  VARCHAR(30),            -- 'KTP', 'Passport'
    sender_id_number TEXT,                   -- KTP/Passport-Nummer
    
    -- Empfänger (Buyer)  
    receiver_user_id UUID REFERENCES users(id),
    receiver_name    TEXT NOT NULL,
    receiver_id_type VARCHAR(30),
    receiver_id_number TEXT,
    
    -- Transfer-Details
    amount_cents    BIGINT NOT NULL,
    asset_id        UUID REFERENCES assets(id),
    token_quantity  INTEGER NOT NULL,
    
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
```

**2. Segregated Accounts (Trennung von Kundengeldern):**
```
┌──────────────────────────────────────────────────┐
│  KONTENSTRUKTUR (OJK-konform)                     │
│                                                  │
│  Bankkonto A: "POOOL Client Trust Account"       │
│  → Alle Kundengelder (Deposits, Wallet-Balances) │
│  → POOOL darf NICHT zugreifen für eigene Ausgaben│
│                                                  │
│  Bankkonto B: "POOOL Operating Account"          │
│  → Fees, Einnahmen, Betriebskosten              │
│  → Nur aus Fee-Einnahmen befüllt                 │
│                                                  │
│  Tägliche Reconciliation:                        │
│  SUM(alle Wallet-Balances) == Kontostand(A)      │
└──────────────────────────────────────────────────┘
```

**3. Investment-Disclaimer (auf jeder Asset-Seite):**
```
⚠️ RISIKO-HINWEIS
Investitionen in tokenisierte Immobilien sind mit Risiken verbunden. 
Der Wert Ihrer Anlage kann steigen oder fallen. Vergangene Wertentwicklung 
ist kein verlässlicher Indikator für zukünftige Ergebnisse. 
Investieren Sie nur Geld, dessen Verlust Sie verkraften können.
Reguliert unter OJK Regulation 27/2024.
```

#### D. Steuer-Dokumentation für Investoren

Investoren brauchen exportierbare Steuer-Reports für die jährliche Steuererklärung:

| Report | Inhalt | Format | API |
|---|---|---|---|
| **Jahres-Trade-Report** | Alle Käufe/Verkäufe, Preise, Fees, Profit/Loss | CSV, PDF | `GET /api/marketplace/tax-report?year=2026` |
| **Dividend-Report** | Erhaltene Dividenden pro Asset | CSV, PDF | `GET /api/portfolio/dividend-report?year=2026` |
| **Kapitalertrags-Berechnung** | FIFO-basierte Gewinn/Verlust-Rechnung | CSV | Teil des Trade-Reports |

**FIFO-Berechnung (First-In, First-Out):**
```
Beispiel: User hat 3 Käufe gemacht:
  Kauf 1: 10 Tokens @ $100 = $1.000
  Kauf 2: 20 Tokens @ $110 = $2.200
  Kauf 3: 30 Tokens @ $105 = $3.150

User verkauft jetzt 25 Tokens @ $120:
  FIFO: Zuerst die ältesten Tokens verkaufen
  10 Tokens (Kauf 1): Gewinn = (120 - 100) × 10 = $200
  15 Tokens (Kauf 2): Gewinn = (120 - 110) × 15 = $150
  
  Gesamtgewinn: $350 (steuerpflichtig)
  Verbleibend: 5 Tokens (Kauf 2) + 30 Tokens (Kauf 3) = 35 Tokens
```

---

## 3. Entwickler-Perspektiven & Tiefe Implementierungs-Guides

Damit das Team exakt weiß, was auf sie zukommt, beleuchten wir den Plan aus den unterschiedlichen Entwickler-Rollen. Jeder Ingenieur trägt extrem hohe Verantwortung für Sicherheit und Konsistenz.

### 3.1. Senior Rust / Backend Engineer (Trading Core & API)
*Der Architekt der Finanzlogik. Verarbeitet Zehntausende Anfragen ohne zu stottern.*

> **Verantwortungsbereich:** Diese Person baut das gesamte `src/marketplace/` Modul von Grund auf – das Orderbook, die Matching-Engine, das Settlement, die WebSocket-Infrastruktur und alle API-Endpunkte. Sie ist die kritischste Rolle im gesamten Projekt, denn jeder Bug in ihrem Code kann zu realem Geldverlust führen.

---

#### 3.1.1. Modul-Architektur: `src/marketplace/`

Das neue Marketplace-Modul folgt exakt dem etablierten Pattern der bestehenden Codebase (`wallet/`, `payments/`, `cart/`): Jedes Domain-Modul hat `mod.rs` (Router), `models.rs` (Datenstrukturen), `routes.rs` (HTTP-Handler) und `service.rs` (Business-Logik).

```
backend/src/marketplace/
├── mod.rs              # Router-Definition (wie wallet/mod.rs)
├── models.rs           # Alle Structs: MarketOrder, Trade, P2POffer, etc.
├── routes.rs           # HTTP-Handler für alle /api/marketplace/* Endpoints
├── service.rs          # Core Business-Logik: Order-Validierung, Fee-Berechnung
├── orderbook.rs        # Redis Orderbook: ZADD, ZREM, Rebuild, Snapshot
├── matching.rs         # Die Matching-Engine (Tokio-Task, Price-Time-Priority)
├── settlement.rs       # Die 8-Step ACID Settlement-Transaktion
├── websocket.rs        # Axum WebSocket-Handler + Redis Pub/Sub Bridge
├── p2p.rs              # P2P/OTC Offer-Logik (Create, Accept, Counter)
├── charts.rs           # Candlestick-Aggregation + Chart-API
├── validation.rs       # Order-Validierung: Balance-Hold, Limits, Wash-Trading
└── background.rs       # Background-Workers: Expiry, Reconciliation, Settlement
```

**Warum diese Granularität?** Die `payments/service.rs` hat aktuell 1.107 Zeilen – das funktioniert, aber die Marketplace-Logik wäre bei gleicher Granularität ~3.000-4.000 Zeilen in einer Datei. Die Aufteilung in 8 spezialisierte Dateien hält jede unter ~500 Zeilen und ermöglicht parallele Code-Reviews.

**Integration in `main.rs`:**

```rust
// In main.rs – exakt wie die anderen Module:
mod marketplace;

// Im Router:
let platform_router = Router::new()
    // ... bestehende Routes ...
    .merge(marketplace::router())   // ← NEU
    // ...
```

**`mod.rs` – Der Router (Pattern von `wallet/mod.rs`):**

```rust
/// Marketplace domain – secondary market trading engine
pub mod models;
pub mod routes;
pub mod service;
pub mod orderbook;
pub mod matching;
pub mod settlement;
pub mod websocket;
pub mod p2p;
pub mod charts;
pub mod validation;
pub mod background;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post, delete},
    Router,
};

pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // ── Public Read APIs ────────────────────────────────
        .route("/api/marketplace/:asset_id/orderbook", get(api_orderbook))
        .route("/api/marketplace/:asset_id/trades", get(api_recent_trades))
        .route("/api/marketplace/:asset_id/ticker", get(api_ticker))
        .route("/api/marketplace/:asset_id/candles", get(api_candles))
        // ── Authenticated Trading APIs ──────────────────────
        .route("/api/marketplace/orders", post(api_submit_order))
        .route("/api/marketplace/orders/mine", get(api_my_orders))
        .route("/api/marketplace/orders/:order_id", delete(api_cancel_order))
        // ── P2P / OTC APIs ─────────────────────────────────
        .route("/api/marketplace/p2p/offer", post(api_create_p2p_offer))
        .route("/api/marketplace/p2p/offer/:id/accept", post(api_accept_p2p))
        .route("/api/marketplace/p2p/offer/:id/counter", post(api_counter_p2p))
        .route("/api/marketplace/p2p/offer/:id/decline", post(api_decline_p2p))
        // ── Tax & Reports ──────────────────────────────────
        .route("/api/marketplace/tax-report", get(api_tax_report))
        // ── Admin APIs ─────────────────────────────────────
        .route("/api/admin/marketplace/orders/:id/approve", post(api_admin_approve))
        .route("/api/admin/marketplace/orders/:id/reject", post(api_admin_reject))
        .route("/api/admin/marketplace/stats", get(api_admin_marketplace_stats))
        // ── WebSocket ──────────────────────────────────────
        .route("/ws/market/:asset_id", get(websocket::ws_handler))
        // ── HTML Pages (SSR) ───────────────────────────────
        .route("/marketplace", get(page_marketplace_overview))
        .route("/marketplace/:asset_id", get(page_asset_trading))
}
```

---

#### 3.1.2. Datenmodelle (`models.rs`) – Die Sprache der Trading-Engine

Jede Datenstruktur ist exakt auf die PostgreSQL-Tabellen und Redis-Formate abgestimmt. Kein `f64` – ausschließlich `i64` für Geldbeträge.

```rust
// marketplace/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════
// ── CORE ENUMS ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Seite der Order: Kauf oder Verkauf
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Typ der Order: Market (sofort zum besten Preis) oder Limit (Preisgrenze)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
}

/// Lifecycle-Status einer Order
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum OrderStatus {
    Open,               // Im Orderbook, wartet auf Match
    PartiallyFilled,   // Teilweise ausgeführt, Rest wartet
    Filled,             // Vollständig ausgeführt
    Cancelled,          // Vom Nutzer storniert
    Expired,            // Automatisch abgelaufen (90 Tage)
    PendingReview,      // Großorder wartet auf Admin-Genehmigung
    Rejected,           // Von Admin abgelehnt
}

// ═══════════════════════════════════════════════════════════════
// ── DATABASE MODELS ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Eine Order im Marketplace (entspricht `market_orders` Tabelle)
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MarketOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub asset_id: Uuid,
    pub side: String,                // "buy" | "sell"
    pub order_type: String,          // "market" | "limit"
    pub price_cents: i64,            // Preis pro Token in Cents
    pub quantity: i32,               // Gewünschte Anzahl Tokens
    pub quantity_filled: i32,        // Bereits ausgeführte Menge
    pub status: String,              // OrderStatus als String (DB-kompatibel)
    pub idempotency_key: Option<String>, // Client-generiert, verhindert Doppel-Orders
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MarketOrder {
    /// Berechnet die verbleibende Menge (noch offen)
    pub fn remaining_quantity(&self) -> i32 {
        self.quantity - self.quantity_filled
    }

    /// Formatiert die Order als Redis Sorted Set Member
    /// Format: "order:{id}:{user_id}:{quantity}:{timestamp_epoch}"
    pub fn redis_member(&self) -> String {
        format!(
            "order:{}:{}:{}:{}",
            self.id,
            self.user_id,
            self.remaining_quantity(),
            self.created_at.timestamp()
        )
    }

    /// Ist die Order vollständig ausgeführt?
    pub fn is_filled(&self) -> bool {
        self.quantity_filled >= self.quantity
    }
}

/// Ein ausgeführter Trade (entspricht `trade_history` Tabelle)
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Trade {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub buyer_user_id: Uuid,
    pub seller_user_id: Uuid,
    pub price_cents: i64,            // Ausführungspreis pro Token
    pub quantity: i32,               // Gehandelte Menge
    pub total_cents: i64,            // price_cents × quantity
    pub fee_cents: i64,              // POOOL-Gebühr
    pub market_order_ask_id: Uuid,   // Referenz auf die Sell-Order
    pub market_order_bid_id: Uuid,   // Referenz auf die Buy-Order
    pub executed_at: DateTime<Utc>,
}

/// Ein P2P/OTC Direktangebot
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct P2POffer {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub maker_user_id: Uuid,         // Ersteller des Angebots
    pub taker_user_id: Uuid,         // Ziel-Nutzer
    pub side: String,                // "buy" | "sell"
    pub offer_price_cents: i64,
    pub quantity: i32,
    pub status: String,              // "pending", "accepted", "declined", "expired", "countered"
    pub counter_price_cents: Option<i64>,  // Gegenangebot
    pub counter_quantity: Option<i32>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════
// ── API REQUEST/RESPONSE MODELS ───────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Request: Neue Order erstellen
#[derive(Debug, Deserialize)]
pub struct SubmitOrderRequest {
    pub asset_id: Uuid,
    pub side: String,                // "buy" | "sell"
    pub order_type: String,          // "market" | "limit"
    pub price_cents: Option<i64>,    // Pflicht bei Limit, ignoriert bei Market
    pub quantity: i32,               // Anzahl Tokens
    pub idempotency_key: String,     // Client-generierte UUID (Pflicht!)
}

/// Response: Order-Bestätigung
#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub order_id: Uuid,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immediate_fill: Option<ImmediateFillInfo>,
}

/// Info über sofortige (Teil-)Ausführung bei Market Orders
#[derive(Debug, Serialize)]
pub struct ImmediateFillInfo {
    pub filled_quantity: i32,
    pub average_price_cents: i64,
    pub total_cents: i64,
    pub remaining_quantity: i32,
}

/// Response: Orderbook-Snapshot für das Frontend
#[derive(Debug, Serialize)]
pub struct OrderbookSnapshot {
    pub asset_id: Uuid,
    pub bids: Vec<PriceLevel>,       // Kaufangebote (höchster zuerst)
    pub asks: Vec<PriceLevel>,       // Verkaufsangebote (niedrigster zuerst)
    pub spread_cents: Option<i64>,   // Differenz zwischen bestem Ask und Bid
    pub last_price_cents: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

/// Ein Preis-Level im Orderbook (aggregiert)
#[derive(Debug, Serialize)]
pub struct PriceLevel {
    pub price_cents: i64,
    pub total_quantity: i32,         // Summe aller Orders auf diesem Level
    pub order_count: i32,            // Anzahl Orders auf diesem Level
}

/// Response: 24h Ticker
#[derive(Debug, Serialize)]
pub struct TickerResponse {
    pub asset_id: Uuid,
    pub last_price_cents: Option<i64>,
    pub change_24h_cents: i64,       // Absolute Preisänderung
    pub change_24h_pct: f64,         // Prozentuale Änderung (einzige Stelle wo f64 OK ist: Display)
    pub high_24h_cents: Option<i64>,
    pub low_24h_cents: Option<i64>,
    pub volume_24h: i64,             // Gehandeltes Volumen in Tokens
    pub volume_24h_cents: i64,       // Gehandeltes Volumen in Cents
    pub trade_count_24h: i64,
}

/// Candlestick-Datenpunkt für Charts
#[derive(Debug, Serialize)]
pub struct Candle {
    pub time: i64,                   // Unix Timestamp (Sekunden)
    pub open: i64,                   // Erster Preis im Intervall (Cents)
    pub high: i64,                   // Höchster Preis
    pub low: i64,                    // Niedrigster Preis
    pub close: i64,                  // Letzter Preis
    pub volume: i64,                 // Gehandeltes Volumen
}

// ═══════════════════════════════════════════════════════════════
// ── WEBSOCKET MESSAGES ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// WebSocket-Nachricht an den Client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Neuer Trade ausgeführt
    #[serde(rename = "trade")]
    Trade {
        asset_id: String,
        price_cents: i64,
        quantity: i32,
        total_cents: i64,
        timestamp: String,
    },
    /// Orderbook hat sich verändert (neue/gelöschte Order)
    #[serde(rename = "orderbook")]
    OrderbookUpdate {
        asset_id: String,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
        spread_cents: Option<i64>,
    },
    /// 24h Ticker Update
    #[serde(rename = "ticker")]
    Ticker {
        asset_id: String,
        last_price_cents: i64,
        change_24h_cents: i64,
        change_24h_pct: f64,
        volume_24h: i64,
    },
}

// ═══════════════════════════════════════════════════════════════
// ── INTERNAL ENGINE TYPES ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Internes Match-Event (von Matching-Engine an Settlement-Worker)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    pub ask_order_id: Uuid,
    pub bid_order_id: Uuid,
    pub asset_id: Uuid,
    pub seller_user_id: Uuid,
    pub buyer_user_id: Uuid,
    pub match_price_cents: i64,
    pub match_quantity: i32,
    pub timestamp: DateTime<Utc>,
}

/// Ergebnis einer Order-Validierung
#[derive(Debug)]
pub enum OrderRejection {
    InsufficientBalance { available_cents: i64, required_cents: i64 },
    InsufficientTokens { owned: i32, requested: i32 },
    ConcentrationLimit { current_pct: f64, requested_pct: f64, max_pct: f64 },
    RequiresAdminReview { order_pct: f64, order_value_cents: i64 },
    BelowMinimum { min_cents: i64, actual_cents: i64 },
    InvalidQuantity,
    AssetNotTradable,
    KycNotApproved,
    SelfTradeBlocked,
    TooManyOpenOrders { max: i32, current: i32 },
    DuplicateIdempotencyKey,
    TwoFactorRequired,
    RateLimited { retry_after_secs: u64 },
}

impl OrderRejection {
    /// Konvertiert in eine benutzerfreundliche Fehlermeldung
    pub fn to_user_message(&self) -> String {
        match self {
            Self::InsufficientBalance { available_cents, required_cents } => {
                format!(
                    "Insufficient balance. Available: ${:.2}, Required: ${:.2}",
                    *available_cents as f64 / 100.0,
                    *required_cents as f64 / 100.0
                )
            }
            Self::InsufficientTokens { owned, requested } => {
                format!("Insufficient tokens. You own {}, requested {}", owned, requested)
            }
            Self::ConcentrationLimit { max_pct, .. } => {
                format!("Order would exceed the maximum concentration limit of {:.0}%", max_pct)
            }
            Self::RequiresAdminReview { .. } => {
                "This order requires admin approval due to its size. You will be notified once reviewed.".into()
            }
            Self::BelowMinimum { min_cents, .. } => {
                format!("Order value must be at least ${:.2}", *min_cents as f64 / 100.0)
            }
            Self::InvalidQuantity => "Quantity must be at least 1 token".into(),
            Self::AssetNotTradable => "This asset is currently not available for trading".into(),
            Self::KycNotApproved => "KYC verification is required to trade".into(),
            Self::SelfTradeBlocked => "You cannot trade against your own orders".into(),
            Self::TooManyOpenOrders { max, .. } => {
                format!("Maximum {} open orders per asset allowed", max)
            }
            Self::DuplicateIdempotencyKey => "This order has already been submitted".into(),
            Self::TwoFactorRequired => "Two-factor authentication required for this trade".into(),
            Self::RateLimited { retry_after_secs } => {
                format!("Too many orders. Please try again in {} seconds", retry_after_secs)
            }
        }
    }
}
```

> **Warum diese Model-Struktur kritisch ist:** Jedes Feld, jeder Typ, jede Enum-Variante ist eine formelle Schnittstelle zwischen Redis, PostgreSQL, den API-Endpoints und dem Frontend. Wenn `price_cents` versehentlich als `f64` definiert wird, entstehen Rundungsfehler die sich über tausende Trades zu realen Geldverlusten aufsummieren. Die klare Trennung in DB-Models, API-Models und interne Engine-Types verhindert, dass sensible DB-Felder (wie `user_id` eines Sellers) versehentlich an unbefugte API-Konsumenten geleakt werden.

---

#### 3.1.3. Error-Handling: Erweiterung von `AppError` für den Marketplace

Das bestehende `AppError`-Enum (`error.rs`) hat 8 Varianten. Für den Marketplace brauchen wir **3 neue Varianten**, die marketplace-spezifische Fehlerfälle sauber abbilden:

```rust
// In error.rs – Neue Varianten zum bestehenden AppError-Enum hinzufügen:

#[derive(Debug)]
pub enum AppError {
    // ... bestehende 8 Varianten bleiben unverändert ...
    Internal(String),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    Database(sqlx::Error),
    RateLimited(u64),

    // ── NEU: Marketplace-spezifische Fehler ────────────────
    
    /// Order wurde abgelehnt (mit benutzerfreundlicher Begründung)
    OrderRejected(String),
    
    /// 2FA Step-Up erforderlich (Frontend zeigt TOTP-Modal)
    TwoFactorRequired,
    
    /// Service vorübergehend nicht verfügbar (Redis down, DB-Failover)
    ServiceUnavailable(String),
}
```

**Warum genau diese 3?**

| Variante | HTTP-Status | Wann | Frontend-Reaktion |
|---|---|---|---|
| `OrderRejected(msg)` | `422 Unprocessable Entity` | Balance zu niedrig, Konzentrationslimit, KYC fehlt, Asset nicht handelbar | Zeige `msg` als roten Toast, keine Retry |
| `TwoFactorRequired` | `428 Precondition Required` | Trade >$500 ohne aktive Trading-Session | Zeige TOTP-Modal, nach Eingabe automatisch Retry |
| `ServiceUnavailable(msg)` | `503 Service Unavailable` | Redis nicht erreichbar, DB-Failover läuft | Zeige "Trading vorübergehend nicht verfügbar", Retry-Button |

```rust
// Ergänzung in IntoResponse für AppError:
AppError::OrderRejected(msg) => {
    (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
}
AppError::TwoFactorRequired => {
    (
        StatusCode::PRECONDITION_REQUIRED,
        "Two-factor authentication required. Please verify your identity.".to_string(),
    )
}
AppError::ServiceUnavailable(msg) => {
    tracing::error!("Service unavailable: {}", msg);
    sentry::capture_message(&msg, sentry::Level::Error);
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Trading is temporarily unavailable. Please try again shortly.".to_string(),
    )
}
```

> **Designentscheidung:** `OrderRejected` gibt die Nachricht direkt an den Client weiter (anders als `Internal`, das den echten Fehler versteckt). Das ist sicher, weil `OrderRejection::to_user_message()` nur kontrollierte, vordefinierte Strings erzeugt – niemals SQL-Fehlerdetails oder interne Systeminformationen.

---

#### 3.1.4. Redis Orderbook Modul (`orderbook.rs`) – Die Speed-Layer-Implementierung

Dieses Modul bildet die Brücke zwischen dem Rust-Backend und Redis. Es kapselt alle Redis-Operationen und stellt sicher, dass der Rest des Codes **niemals direkt Redis-Befehle** aufruft.

```rust
// marketplace/orderbook.rs
use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use uuid::Uuid;
use crate::error::AppError;
use super::models::*;

/// Redis Key-Schema:
///   asks:asset:{asset_id}   → Sorted Set (Score = price_cents, Member = order:...)
///   bids:asset:{asset_id}   → Sorted Set (Score = price_cents, Member = order:...)
///   lock:order:{order_id}   → String (TTL 5s, für Cancel/Match Race-Condition)
///   idempotency:{key}       → String (TTL 24h, Ergebnis des ersten Requests)
///   trading_session:{user}  → String (TTL 900s = 15min, 2FA Trading-Session)
///   order_ip:{order_id}     → String (TTL 24h, IP für Wash-Trading-Detection)
///   rl:orders:user:{user}   → Counter (TTL 60s, Rate-Limit)

const ASKS_PREFIX: &str = "asks:asset:";
const BIDS_PREFIX: &str = "bids:asset:";
const LOCK_PREFIX: &str = "lock:order:";
const IDEMPOTENCY_PREFIX: &str = "idempotency:";

/// Fügt eine Order in das Redis Orderbook ein
pub async fn insert_order(
    redis: &RedisPool,
    order: &MarketOrder,
) -> Result<(), AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = if order.side == "sell" {
        format!("{}{}", ASKS_PREFIX, order.asset_id)
    } else {
        format!("{}{}", BIDS_PREFIX, order.asset_id)
    };

    // ZADD mit Score = price_cents, Member = order:{id}:{user_id}:{qty}:{timestamp}
    redis::cmd("ZADD")
        .arg(&key)
        .arg(order.price_cents)  // Score
        .arg(order.redis_member())  // Member
        .query_async::<i32>(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZADD failed: {}", e)))?;

    Ok(())
}

/// Entfernt eine Order aus dem Redis Orderbook
pub async fn remove_order(
    redis: &RedisPool,
    order: &MarketOrder,
) -> Result<(), AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = if order.side == "sell" {
        format!("{}{}", ASKS_PREFIX, order.asset_id)
    } else {
        format!("{}{}", BIDS_PREFIX, order.asset_id)
    };

    redis::cmd("ZREM")
        .arg(&key)
        .arg(order.redis_member())
        .query_async::<i32>(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZREM failed: {}", e)))?;

    Ok(())
}

/// Holt den besten Ask (niedrigster Verkaufspreis) für ein Asset
pub async fn best_ask(
    redis: &RedisPool,
    asset_id: Uuid,
) -> Result<Option<ParsedOrderMember>, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", ASKS_PREFIX, asset_id);

    // ZRANGEBYSCORE key -inf +inf WITHSCORES LIMIT 0 1 → niedrigster Preis
    let result: Vec<(String, f64)> = redis::cmd("ZRANGEBYSCORE")
        .arg(&key)
        .arg("-inf")
        .arg("+inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(1)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis query failed: {}", e)))?;

    Ok(result.first().and_then(|(member, score)| {
        ParsedOrderMember::parse(member, *score as i64)
    }))
}

/// Holt den besten Bid (höchster Kaufpreis) für ein Asset
pub async fn best_bid(
    redis: &RedisPool,
    asset_id: Uuid,
) -> Result<Option<ParsedOrderMember>, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", BIDS_PREFIX, asset_id);

    // ZREVRANGEBYSCORE key +inf -inf WITHSCORES LIMIT 0 1 → höchster Preis
    let result: Vec<(String, f64)> = redis::cmd("ZREVRANGEBYSCORE")
        .arg(&key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(1)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis query failed: {}", e)))?;

    Ok(result.first().and_then(|(member, score)| {
        ParsedOrderMember::parse(member, *score as i64)
    }))
}

/// Baut das vollständige Orderbook für die Frontend-Anzeige (Top N Levels)
pub async fn get_orderbook_snapshot(
    redis: &RedisPool,
    asset_id: Uuid,
    depth: usize,  // Typisch: 20
) -> Result<OrderbookSnapshot, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let asks_key = format!("{}{}", ASKS_PREFIX, asset_id);
    let bids_key = format!("{}{}", BIDS_PREFIX, asset_id);

    // Alle Asks (niedrigster zuerst)
    let raw_asks: Vec<(String, f64)> = redis::cmd("ZRANGEBYSCORE")
        .arg(&asks_key).arg("-inf").arg("+inf")
        .arg("WITHSCORES").arg("LIMIT").arg(0).arg(depth * 5) // Mehr holen, dann aggregieren
        .query_async(&mut *conn).await.unwrap_or_default();

    // Alle Bids (höchster zuerst)
    let raw_bids: Vec<(String, f64)> = redis::cmd("ZREVRANGEBYSCORE")
        .arg(&bids_key).arg("+inf").arg("-inf")
        .arg("WITHSCORES").arg("LIMIT").arg(0).arg(depth * 5)
        .query_async(&mut *conn).await.unwrap_or_default();

    // Aggregiere zu Price-Levels (gleicher Preis → zusammenfassen)
    let asks = aggregate_price_levels(&raw_asks, depth);
    let bids = aggregate_price_levels(&raw_bids, depth);

    let spread = match (asks.first(), bids.first()) {
        (Some(best_ask), Some(best_bid)) => {
            Some(best_ask.price_cents - best_bid.price_cents)
        }
        _ => None,
    };

    Ok(OrderbookSnapshot {
        asset_id,
        bids,
        asks,
        spread_cents: spread,
        last_price_cents: None, // Wird vom Caller aus trade_history geholt
        timestamp: chrono::Utc::now(),
    })
}

/// Setzt einen Order-Lock (für Cancel/Match Race-Condition-Schutz)
pub async fn try_lock_order(
    redis: &RedisPool,
    order_id: Uuid,
    ttl_seconds: u64,
) -> Result<bool, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", LOCK_PREFIX, order_id);

    // SET NX (nur wenn Key nicht existiert) mit TTL
    let locked: bool = redis::cmd("SET")
        .arg(&key)
        .arg("locked")
        .arg("NX")
        .arg("EX")
        .arg(ttl_seconds)
        .query_async(&mut *conn)
        .await
        .unwrap_or(false);

    Ok(locked)
}

/// Prüft ob eine Order gerade gelockt ist
pub async fn is_order_locked(
    redis: &RedisPool,
    order_id: Uuid,
) -> Result<bool, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", LOCK_PREFIX, order_id);
    let exists: bool = conn.exists(&key).await.unwrap_or(false);
    Ok(exists)
}

/// Prüft Idempotency-Key (verhindert Doppel-Submissions)
pub async fn check_idempotency(
    redis: &RedisPool,
    key: &str,
) -> Result<Option<String>, AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let full_key = format!("{}{}", IDEMPOTENCY_PREFIX, key);
    let result: Option<String> = conn.get(&full_key).await.unwrap_or(None);
    Ok(result)
}

/// Speichert Idempotency-Ergebnis (TTL 24h)
pub async fn store_idempotency(
    redis: &RedisPool,
    key: &str,
    result: &str,
) -> Result<(), AppError> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let full_key = format!("{}{}", IDEMPOTENCY_PREFIX, key);
    conn.set_ex(&full_key, result, 86400).await // 24 Stunden
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis SET failed: {}", e)))?;
    Ok(())
}

/// Rebuilt das Redis Orderbook aus PostgreSQL (nach Redis-Crash)
pub async fn rebuild_from_postgres(
    redis: &RedisPool,
    pool: &sqlx::PgPool,
) -> Result<u32, AppError> {
    tracing::warn!("🔄 Rebuilding Redis orderbook from PostgreSQL...");

    let open_orders = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders WHERE status IN ('open', 'partially_filled')"
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let count = open_orders.len() as u32;

    for order in &open_orders {
        insert_order(redis, order).await?;
    }

    tracing::info!("✅ Redis orderbook rebuilt: {} orders restored", count);
    Ok(count)
}

// ── Interne Hilfsstrukturen ─────────────────────────────────────

/// Geparster Redis-Member mit extrahierten Feldern
#[derive(Debug, Clone)]
pub struct ParsedOrderMember {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub quantity: i32,
    pub timestamp: i64,
    pub price_cents: i64,
    pub raw_member: String,
}

impl ParsedOrderMember {
    /// Parst "order:{id}:{user_id}:{qty}:{timestamp}" + Score
    pub fn parse(member: &str, score: i64) -> Option<Self> {
        let parts: Vec<&str> = member.split(':').collect();
        if parts.len() != 5 || parts[0] != "order" {
            tracing::warn!("Invalid Redis member format: {}", member);
            return None;
        }
        Some(Self {
            order_id: Uuid::parse_str(parts[1]).ok()?,
            user_id: Uuid::parse_str(parts[2]).ok()?,
            quantity: parts[3].parse().ok()?,
            timestamp: parts[4].parse().ok()?,
            price_cents: score,
            raw_member: member.to_string(),
        })
    }
}

/// Aggregiert einzelne Orders zu Price-Levels für die Frontend-Anzeige
fn aggregate_price_levels(
    raw: &[(String, f64)],
    max_levels: usize,
) -> Vec<PriceLevel> {
    use std::collections::BTreeMap;

    let mut levels: BTreeMap<i64, (i32, i32)> = BTreeMap::new();

    for (member, score) in raw {
        let price = *score as i64;
        if let Some(parsed) = ParsedOrderMember::parse(member, price) {
            let entry = levels.entry(price).or_insert((0, 0));
            entry.0 += parsed.quantity;  // total_quantity
            entry.1 += 1;                // order_count
        }
    }

    levels
        .into_iter()
        .take(max_levels)
        .map(|(price, (qty, count))| PriceLevel {
            price_cents: price,
            total_quantity: qty,
            order_count: count,
        })
        .collect()
}
```

> **Kritische Designentscheidung: Redis als Cache, PostgreSQL als Source of Truth.** Die Funktion `rebuild_from_postgres()` ist die Versicherungspolice: Wenn Redis abstürzt oder Daten verliert, wird das Orderbook innerhalb von Sekunden aus der `market_orders`-Tabelle (WHERE status IN ('open', 'partially_filled')) vollständig wiederhergestellt. Das bedeutet: **Redis darf jederzeit seinen Speicher verlieren, ohne dass eine einzige Order verloren geht.** Die einzige Konsequenz wäre eine kurze Unterbrechung des Live-Matchings (~2-5 Sekunden für den Rebuild).

---

#### 3.1.5. Order-Submission Pipeline (`routes.rs` + `service.rs` + `validation.rs`)

Die Order-Submission ist der **sicherheitssensibelste Pfad** im gesamten System. Zwischen dem HTTP-Request und dem Redis-Insert liegen **10 Validierungsschritte**, die sequentiell abgearbeitet werden. Jeder Schritt kann die Order ablehnen.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ORDER SUBMISSION PIPELINE (10 Steps)                                    │
│                                                                          │
│  HTTP POST /api/marketplace/orders                                       │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │ Step 1:  Auth + Session Check (Middleware)                       │    │
│  │ Step 2:  Rate Limit Check (Redis: max 10 Orders/Min/User)       │    │
│  │ Step 3:  Idempotency Check (Redis: Key schon verarbeitet?)      │    │
│  │ Step 4:  Request Validation (JSON Schema, Enums, Ranges)        │    │
│  │ Step 5:  KYC Status Check (PostgreSQL: kyc_status = 'approved') │    │
│  │ Step 6:  2FA Step-Up Check (Redis: Trading-Session aktiv?)      │    │
│  │ Step 7:  Asset Tradability (PostgreSQL: trading_enabled = true)  │    │
│  │ Step 8:  Balance/Token Hold (PostgreSQL: FOR UPDATE + Hold)     │    │
│  │ Step 9:  Concentration Limits (PostgreSQL: Max 80%, >20% Admin) │    │
│  │ Step 10: Open Order Limit (PostgreSQL: Max 10 pro Asset)        │    │
│  └──────────────────────┬───────────────────────────────────────────┘    │
│                         │ Alle Checks bestanden                          │
│                         ▼                                                │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │ Step 11: INSERT INTO market_orders (PostgreSQL)                  │    │
│  │ Step 12: ZADD asks/bids:asset:{id} (Redis Orderbook)            │    │
│  │ Step 13: Store Idempotency Key (Redis, TTL 24h)                 │    │
│  │ Step 14: Store Order IP (Redis, TTL 24h, Wash-Trading)          │    │
│  │ Step 15: WebSocket Broadcast (Orderbook Update)                 │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Response: { order_id, status: "open", message: "Order placed" }         │
└──────────────────────────────────────────────────────────────────────────┘
```

**Der Route-Handler (`routes.rs`):**

```rust
// marketplace/routes.rs

/// POST /api/marketplace/orders — Neue Order erstellen
pub async fn api_submit_order(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SubmitOrderRequest>,
) -> Result<Json<OrderResponse>, AppError> {
    // Step 1: Auth
    let user = crate::auth::middleware::require_user(&jar, &state.db).await?;

    // Step 2: Rate Limit (Redis)
    let redis = state.redis.as_ref()
        .ok_or(AppError::ServiceUnavailable("Trading engine not available".into()))?;
    validation::check_rate_limit(redis, user.id, 10, 60).await?;

    // Step 3: Idempotency
    if let Some(existing) = orderbook::check_idempotency(redis, &req.idempotency_key).await? {
        return Ok(Json(serde_json::from_str(&existing)
            .unwrap_or(OrderResponse {
                order_id: Uuid::nil(),
                status: "duplicate".into(),
                message: "Order already submitted".into(),
                immediate_fill: None,
            })));
    }

    // Steps 4-10: Validierung (in service.rs)
    let order = service::validate_and_create_order(
        &state.db, redis, &user, &req, &headers,
    ).await?;

    // Step 15: WebSocket-Broadcast
    websocket::broadcast_orderbook_update(&state, order.asset_id).await;

    let response = OrderResponse {
        order_id: order.id,
        status: order.status.clone(),
        message: "Order successfully placed".into(),
        immediate_fill: None,
    };

    // Step 13: Idempotency speichern
    let response_json = serde_json::to_string(&response).unwrap_or_default();
    orderbook::store_idempotency(redis, &req.idempotency_key, &response_json).await?;

    Ok(Json(response))
}
```

**Die Validierung (`validation.rs`):**

```rust
// marketplace/validation.rs

/// Prüft alle Voraussetzungen für eine neue Order (Steps 4-10)
pub async fn validate_order(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: Uuid,
    req: &SubmitOrderRequest,
) -> Result<(), OrderRejection> {
    // Step 4: Request Validation
    if req.quantity < 1 {
        return Err(OrderRejection::InvalidQuantity);
    }
    if req.side != "buy" && req.side != "sell" {
        return Err(OrderRejection::InvalidQuantity); // Sollte BadRequest sein
    }
    if req.order_type == "limit" && req.price_cents.is_none() {
        return Err(OrderRejection::BelowMinimum { min_cents: 0, actual_cents: 0 });
    }

    // Minimum Order Value: $10.00
    let price = req.price_cents.unwrap_or(0);
    let total_cents = price * req.quantity as i64;
    if req.order_type == "limit" && total_cents < 1000 {
        return Err(OrderRejection::BelowMinimum {
            min_cents: 1000,
            actual_cents: total_cents,
        });
    }

    // Step 5: KYC Check
    let kyc_status: Option<String> = sqlx::query_scalar(
        "SELECT kyc_status FROM users WHERE id = $1"
    ).bind(user_id).fetch_optional(pool).await
    .map_err(|_| OrderRejection::KycNotApproved)?;

    if kyc_status.as_deref() != Some("approved") {
        return Err(OrderRejection::KycNotApproved);
    }

    // Step 6: 2FA Step-Up (für Orders > $500)
    if total_cents > 50000 {
        let session_key = format!("trading_session:{}", user_id);
        let mut conn = redis.get().await
            .map_err(|_| OrderRejection::TwoFactorRequired)?;
        let has_session: bool = conn.exists(&session_key).await.unwrap_or(false);
        if !has_session {
            return Err(OrderRejection::TwoFactorRequired);
        }
    }

    // Step 7: Asset Tradability
    let asset_tradable: Option<bool> = sqlx::query_scalar(
        "SELECT trading_enabled FROM assets WHERE id = $1"
    ).bind(req.asset_id).fetch_optional(pool).await
    .map_err(|_| OrderRejection::AssetNotTradable)?;

    if asset_tradable != Some(true) {
        return Err(OrderRejection::AssetNotTradable);
    }

    // Step 8: Balance/Token Hold
    if req.side == "buy" {
        let available: Option<i64> = sqlx::query_scalar(
            "SELECT (balance_cents - held_balance_cents) FROM wallets 
             WHERE user_id = $1 AND wallet_type = 'cash' FOR UPDATE"
        ).bind(user_id).fetch_optional(pool).await
        .map_err(|_| OrderRejection::InsufficientBalance {
            available_cents: 0, required_cents: total_cents
        })?;

        let avail = available.unwrap_or(0);
        if avail < total_cents {
            return Err(OrderRejection::InsufficientBalance {
                available_cents: avail,
                required_cents: total_cents,
            });
        }
    } else {
        // Sell: Prüfe ob Nutzer genug Tokens hat
        let tokens: Option<(i32, i32)> = sqlx::query_as(
            "SELECT tokens_owned, held_tokens FROM investments 
             WHERE user_id = $1 AND asset_id = $2 FOR UPDATE"
        ).bind(user_id).bind(req.asset_id)
        .fetch_optional(pool).await
        .map_err(|_| OrderRejection::InsufficientTokens {
            owned: 0, requested: req.quantity
        })?;

        let (owned, held) = tokens.unwrap_or((0, 0));
        let available_tokens = owned - held;
        if available_tokens < req.quantity {
            return Err(OrderRejection::InsufficientTokens {
                owned: available_tokens,
                requested: req.quantity,
            });
        }
    }

    // Step 9: Concentration Limits (nur für Buy-Orders)
    if req.side == "buy" {
        check_concentration_limits(pool, user_id, req.asset_id, req.quantity).await?;
    }

    // Step 10: Max Open Orders (10 pro User pro Asset)
    let open_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_orders 
         WHERE user_id = $1 AND asset_id = $2 
         AND status IN ('open', 'partially_filled')"
    ).bind(user_id).bind(req.asset_id)
    .fetch_one(pool).await
    .unwrap_or(0);

    if open_count >= 10 {
        return Err(OrderRejection::TooManyOpenOrders {
            max: 10,
            current: open_count as i32,
        });
    }

    Ok(())
}

/// Prüft Konzentrationslimits (Max 80%, >20% → Admin-Review)
async fn check_concentration_limits(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    order_qty: i32,
) -> Result<(), OrderRejection> {
    let current_holding: i32 = sqlx::query_scalar(
        "SELECT COALESCE(tokens_owned, 0) FROM investments 
         WHERE user_id = $1 AND asset_id = $2"
    ).bind(user_id).bind(asset_id)
    .fetch_optional(pool).await
    .ok().flatten().unwrap_or(0);

    let total_supply: i32 = sqlx::query_scalar(
        "SELECT tokens_total FROM assets WHERE id = $1"
    ).bind(asset_id).fetch_one(pool).await
    .map_err(|_| OrderRejection::AssetNotTradable)?;

    if total_supply == 0 {
        return Err(OrderRejection::AssetNotTradable);
    }

    let new_total = current_holding + order_qty;
    let concentration_pct = (new_total as f64 / total_supply as f64) * 100.0;

    if concentration_pct > 80.0 {
        return Err(OrderRejection::ConcentrationLimit {
            current_pct: (current_holding as f64 / total_supply as f64) * 100.0,
            requested_pct: concentration_pct,
            max_pct: 80.0,
        });
    }

    let order_pct = (order_qty as f64 / total_supply as f64) * 100.0;
    if order_pct > 20.0 {
        return Err(OrderRejection::RequiresAdminReview {
            order_pct,
            order_value_cents: order_qty as i64 * get_last_price(pool, asset_id).await,
        });
    }

    Ok(())
}
```

---

#### 3.1.6. Die Matching-Engine (`matching.rs`) – Das Herzstück

Die Matching-Engine ist ein **permanenter Tokio-Task**, der in einer Endlosschleife läuft und Redis nach matchbaren Orders abfragt. Sie ist der kritischste Code im gesamten System.

**Architektur-Prinzipien:**
1. **Single-Threaded Matching:** Redis ist Single-Threaded. Die Engine nutzt das: Pro Asset wird sequentiell gematched → keine Race Conditions.
2. **Separation of Concerns:** Die Engine matcht nur (Redis). Das Settlement passiert in einem separaten Worker (PostgreSQL). Dazwischen liegt eine Redis-Queue.
3. **Crash-Safety:** Wenn die Engine crasht, gehen keine Daten verloren. Unverarbeitete Match-Events bleiben in der Redis-Queue.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  MATCHING-ENGINE ARCHITEKTUR                                              │
│                                                                          │
│  ┌─────────────────────┐    ┌─────────────────────┐    ┌──────────────┐ │
│  │  Matching-Engine     │    │  Match Queue         │    │  Settlement  │ │
│  │  (Tokio-Task #1)     │───▶│  (Redis List)        │───▶│  Worker      │ │
│  │                      │    │                      │    │ (Tokio #2)   │ │
│  │  • Scannt Orderbooks │    │  match:queue          │    │              │ │
│  │  • Findet Matches    │    │  [MatchEvent JSON]    │    │ • Liest Queue│ │
│  │  • Prüft Wash-Trade  │    │  [MatchEvent JSON]    │    │ • BEGIN TX   │ │
│  │  • Prüft Order-Locks │    │  [...]                │    │ • 8 Steps    │ │
│  │                      │    │                      │    │ • COMMIT     │ │
│  │  Loop: 10ms Sleep    │    │  RPUSH / BLPOP       │    │ • WS Broadc. │ │
│  └─────────────────────┘    └─────────────────────┘    └──────────────┘ │
│                                                                          │
│  Warum 2 Tasks statt 1?                                                  │
│  • Matching ist CPU-gebunden (Redis Abfragen, Vergleiche)                │
│  • Settlement ist I/O-gebunden (PostgreSQL Transaktionen, 5-50ms)        │
│  • Wenn Settlement blockiert, darf das Matching NICHT stoppen            │
│  • Bei einem Settlement-Fehler bleibt das Match-Event in der Queue →     │
│    Retry beim nächsten Versuch                                           │
└──────────────────────────────────────────────────────────────────────────┘
```

**Spawn in `main.rs` (neben den bestehenden Background-Tasks):**

```rust
// In main.rs, nach den bestehenden tokio::spawn(...) Aufrufen:

// Marketplace: Matching-Engine (Core Trading Loop)
if state.redis.is_some() {
    let match_state = state.clone();
    tokio::spawn(async move {
        marketplace::matching::run_matching_engine(
            match_state.redis.as_ref().unwrap(),
            &match_state.db,
        ).await;
    });

    // Marketplace: Settlement-Worker (Match → ACID Transaction)
    let settle_state = state.clone();
    tokio::spawn(async move {
        marketplace::settlement::run_settlement_worker(
            settle_state.redis.as_ref().unwrap(),
            &settle_state.db,
        ).await;
    });

    // Marketplace: Order-Expiry (stündlich abgelaufene Orders bereinigen)
    let expiry_state = state.clone();
    tokio::spawn(async move {
        marketplace::background::run_order_expiry_worker(
            expiry_state.redis.as_ref().unwrap(),
            &expiry_state.db,
        ).await;
    });

    tracing::info!("🚀 Marketplace engine started (Matching + Settlement + Expiry)");
} else {
    tracing::warn!("⚠️ Redis not configured – Marketplace trading is DISABLED");
}
```

**Die Matching-Engine selbst:**

```rust
// marketplace/matching.rs

use super::orderbook;
use super::models::MatchEvent;
use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use sqlx::PgPool;

const MATCH_QUEUE: &str = "match:queue";

/// Hauptloop der Matching-Engine. Läuft endlos als Tokio-Task.
pub async fn run_matching_engine(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("Matching engine started");

    // Beim Start: Redis Orderbook aus PostgreSQL rebuilden (falls Redis leer)
    let mut conn = redis.get().await.expect("Redis required for matching engine");
    let key_count: i64 = redis::cmd("DBSIZE")
        .query_async(&mut *conn)
        .await
        .unwrap_or(0);
    drop(conn);

    if key_count == 0 {
        match orderbook::rebuild_from_postgres(redis, pool).await {
            Ok(n) => tracing::info!("Orderbook rebuilt with {} orders", n),
            Err(e) => tracing::error!("Orderbook rebuild failed: {}", e),
        }
    }

    loop {
        // Hole alle Assets mit offenen Orders
        let active_assets = get_active_asset_ids(pool).await;

        for asset_id in active_assets {
            // Pro Asset: Versuche Matches zu finden (bis keine mehr möglich)
            loop {
                match try_match_once(redis, asset_id).await {
                    Ok(Some(match_event)) => {
                        // Match gefunden → in Queue für Settlement-Worker
                        let event_json = serde_json::to_string(&match_event)
                            .unwrap_or_default();

                        if let Ok(mut conn) = redis.get().await {
                            let _: Result<i64, _> = conn
                                .rpush(MATCH_QUEUE, &event_json)
                                .await;
                        }

                        tracing::info!(
                            asset = %asset_id,
                            price = match_event.match_price_cents,
                            qty = match_event.match_quantity,
                            "⚡ Match found"
                        );
                    }
                    Ok(None) => break, // Kein weiteres Match für dieses Asset
                    Err(e) => {
                        tracing::error!("Matching error for {}: {}", asset_id, e);
                        break;
                    }
                }
            }
        }

        // 10ms Pause → ~100 Matching-Zyklen/Sekunde (mehr als genug für 1.000 Trades/Tag)
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Versucht EINEN Match für ein Asset zu finden
async fn try_match_once(
    redis: &RedisPool,
    asset_id: uuid::Uuid,
) -> Result<Option<MatchEvent>, String> {
    // 1. Besten Ask und Bid holen
    let best_ask = orderbook::best_ask(redis, asset_id).await
        .map_err(|e| e.to_string())?;
    let best_bid = orderbook::best_bid(redis, asset_id).await
        .map_err(|e| e.to_string())?;

    let (ask, bid) = match (best_ask, best_bid) {
        (Some(a), Some(b)) => (a, b),
        _ => return Ok(None), // Kein Match möglich
    };

    // 2. Match-Bedingung: Ask ≤ Bid
    if ask.price_cents > bid.price_cents {
        return Ok(None); // Kein Match
    }

    // 3. Self-Trade Prevention (Wash Trading)
    if ask.user_id == bid.user_id {
        tracing::warn!(
            "🚫 Self-trade blocked: user {} on asset {}",
            ask.user_id, asset_id
        );
        // Order mit niedrigerer Priorität aus dem Book entfernen
        // (die neuere Order wird gecancelt)
        if ask.timestamp > bid.timestamp {
            orderbook::remove_order_by_member(redis, asset_id, "sell", &ask.raw_member).await?;
        } else {
            orderbook::remove_order_by_member(redis, asset_id, "buy", &bid.raw_member).await?;
        }
        return Ok(None);
    }

    // 4. Order-Lock prüfen (wird gerade gecancelt?)
    if orderbook::is_order_locked(redis, ask.order_id).await.unwrap_or(false)
        || orderbook::is_order_locked(redis, bid.order_id).await.unwrap_or(false)
    {
        return Ok(None); // Überspringen, beim nächsten Zyklus erneut prüfen
    }

    // 5. Match-Parameter berechnen
    let match_price = ask.price_cents; // Maker-Preis (Ask war zuerst im Book)
    let match_qty = std::cmp::min(ask.quantity, bid.quantity);

    // 6. Match-Event erzeugen
    let match_event = MatchEvent {
        ask_order_id: ask.order_id,
        bid_order_id: bid.order_id,
        asset_id,
        seller_user_id: ask.user_id,
        buyer_user_id: bid.user_id,
        match_price_cents: match_price,
        match_quantity: match_qty,
        timestamp: chrono::Utc::now(),
    };

    // 7. Orders in Redis aktualisieren (entfernen oder Quantity reduzieren)
    update_order_after_match(redis, asset_id, &ask, &bid, match_qty).await?;

    Ok(Some(match_event))
}

/// Aktualisiert Orders in Redis nach einem Match
async fn update_order_after_match(
    redis: &RedisPool,
    asset_id: uuid::Uuid,
    ask: &orderbook::ParsedOrderMember,
    bid: &orderbook::ParsedOrderMember,
    matched_qty: i32,
) -> Result<(), String> {
    // Ask: Komplett gefüllt → entfernen. Teilweise → neuen Member mit reduzierter Qty
    if matched_qty >= ask.quantity {
        orderbook::remove_order_by_member(redis, asset_id, "sell", &ask.raw_member).await?;
    } else {
        // Entferne alten Member, füge neuen mit reduzierter Qty hinzu
        orderbook::remove_order_by_member(redis, asset_id, "sell", &ask.raw_member).await?;
        let new_member = format!(
            "order:{}:{}:{}:{}",
            ask.order_id, ask.user_id, ask.quantity - matched_qty, ask.timestamp
        );
        orderbook::insert_member(redis, asset_id, "sell", ask.price_cents, &new_member).await?;
    }

    // Analog für Bid
    if matched_qty >= bid.quantity {
        orderbook::remove_order_by_member(redis, asset_id, "buy", &bid.raw_member).await?;
    } else {
        orderbook::remove_order_by_member(redis, asset_id, "buy", &bid.raw_member).await?;
        let new_member = format!(
            "order:{}:{}:{}:{}",
            bid.order_id, bid.user_id, bid.quantity - matched_qty, bid.timestamp
        );
        orderbook::insert_member(redis, asset_id, "buy", bid.price_cents, &new_member).await?;
    }

    Ok(())
}

/// Holt alle Assets die offene Orders haben
async fn get_active_asset_ids(pool: &PgPool) -> Vec<uuid::Uuid> {
    sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT DISTINCT asset_id FROM market_orders 
         WHERE status IN ('open', 'partially_filled')"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
```

> **Warum die 10ms-Pause?** Ohne Pause würde die Engine die CPU zu 100% auslasten – sinnlos bei 1-10 Trades/Tag. 10ms bedeutet: Maximal 100 Matching-Zyklen pro Sekunde. Bei je ~1ms Redis-Latenz ergibt das ~10% CPU-Auslastung. Der Worst-Case (100 Orders rein in 1 Sekunde) wird in ~1 Sekunde abgearbeitet. Für POOOL's Lastprofil ist das 100x überdimensioniert – und genau so soll es sein.

---

#### 3.1.7. WebSocket-Server (`websocket.rs`)

Der WebSocket-Server nutzt **Redis Pub/Sub als Event-Bus** zwischen den Cloud Run Instanzen. Ohne Pub/Sub würde jede Instanz nur ihre eigenen lokalen Clients benachrichtigen – die Clients auf anderen Instanzen bekämen keine Updates.

```
┌──────────────────────────────────────────────────────────────────────┐
│  WEBSOCKET + REDIS PUB/SUB ARCHITEKTUR                                │
│                                                                      │
│  Cloud Run Instanz A          Redis           Cloud Run Instanz B    │
│  ┌──────────────────┐    ┌──────────────┐   ┌──────────────────┐    │
│  │ Client 1 ←─ WS   │    │ Channel:     │   │ Client 3 ←─ WS   │    │
│  │ Client 2 ←─ WS   │◀───│ market:{id}  │──▶│ Client 4 ←─ WS   │    │
│  │                   │    │              │   │                   │    │
│  │ tokio::broadcast  │    │ PUBLISH msg  │   │ tokio::broadcast  │    │
│  │ (lokal)           │    │              │   │ (lokal)           │    │
│  └──────────────────┘    └──────────────┘   └──────────────────┘    │
│                                                                      │
│  Trade auf Instanz A → PUBLISH → Redis → Instanz B empfängt →       │
│  broadcast an lokale WS-Clients                                      │
└──────────────────────────────────────────────────────────────────────┘
```

```rust
// marketplace/websocket.rs (Kern-Logik)

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use tokio::sync::broadcast;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Globaler Broadcast-Channel pro Asset (in-memory, pro Cloud Run Instanz)
type ChannelMap = Arc<RwLock<HashMap<uuid::Uuid, broadcast::Sender<String>>>>;

lazy_static::lazy_static! {
    static ref CHANNELS: ChannelMap = Arc::new(RwLock::new(HashMap::new()));
}

/// GET /ws/market/{asset_id} — WebSocket Upgrade Handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(asset_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, asset_id, state))
}

/// Handhabt eine einzelne WebSocket-Verbindung
async fn handle_ws_connection(
    mut socket: WebSocket,
    asset_id: uuid::Uuid,
    state: AppState,
) {
    // Channel für dieses Asset holen oder erstellen
    let rx = {
        let mut channels = CHANNELS.write().await;
        let tx = channels
            .entry(asset_id)
            .or_insert_with(|| broadcast::channel(256).0);
        tx.subscribe()
    };

    let mut rx = rx;

    // Heartbeat: Alle 30 Sekunden Ping senden
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            // Nachricht vom Broadcast-Channel → an Client senden
            Ok(msg) = rx.recv() => {
                if socket.send(Message::Text(msg)).await.is_err() {
                    break; // Client disconnected
                }
            }
            // Heartbeat → Ping senden
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
            // Nachricht vom Client (ignorieren, außer Pong)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Pong(_))) => {} // Heartbeat-Response
                    _ => {} // Andere Nachrichten ignorieren
                }
            }
        }
    }

    tracing::debug!("WebSocket disconnected for asset {}", asset_id);
}

/// Sendet ein Update an alle verbundenen Clients für ein Asset
pub async fn broadcast_orderbook_update(state: &AppState, asset_id: uuid::Uuid) {
    // 1. Aktuelles Orderbook aus Redis holen
    if let Some(redis) = &state.redis {
        if let Ok(snapshot) = orderbook::get_orderbook_snapshot(redis, asset_id, 20).await {
            let msg = WsMessage::OrderbookUpdate {
                asset_id: asset_id.to_string(),
                bids: snapshot.bids,
                asks: snapshot.asks,
                spread_cents: snapshot.spread_cents,
            };

            if let Ok(json) = serde_json::to_string(&msg) {
                // 2. An lokale Clients senden
                let channels = CHANNELS.read().await;
                if let Some(tx) = channels.get(&asset_id) {
                    let _ = tx.send(json.clone());
                }

                // 3. Via Redis Pub/Sub an andere Instanzen
                if let Ok(mut conn) = redis.get().await {
                    let channel = format!("market:{}", asset_id);
                    let _: Result<(), _> = redis::cmd("PUBLISH")
                        .arg(&channel)
                        .arg(&json)
                        .query_async(&mut *conn)
                        .await;
                }
            }
        }
    }
}
```

---

#### 3.1.8. Background-Workers (`background.rs`)

Drei Background-Worker laufen als permanente Tokio-Tasks neben der Matching-Engine:

| Worker | Intervall | Aufgabe |
|---|---|---|
| **Order-Expiry** | Stündlich | Abgelaufene Orders (>90 Tage) aus Redis + PostgreSQL entfernen, Balance-Holds zurückgeben |
| **Reconciliation** | Täglich 03:00 UTC | Prüfe: Σ(Wallets) = Σ(Deposits) − Σ(Withdrawals) − Σ(Purchases) − Σ(Fees). Bei Mismatch → Sentry-Alert |
| **Redis-Sync** | Alle 5 Minuten | Vergleiche Redis Orderbook mit `market_orders` WHERE status='open'. Fehlende Orders re-inserieren |

```rust
// marketplace/background.rs

/// Bereinigt abgelaufene Orders (stündlich)
pub async fn run_order_expiry_worker(redis: &RedisPool, pool: &PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;

        match expire_stale_orders(redis, pool).await {
            Ok(count) if count > 0 => {
                tracing::info!("⏰ Expired {} stale orders", count);
            }
            Err(e) => {
                tracing::error!("Order expiry failed: {}", e);
                sentry::capture_message(
                    &format!("Order expiry worker failed: {}", e),
                    sentry::Level::Error,
                );
            }
            _ => {} // 0 expired = normal
        }
    }
}

/// Täglicher Financial Reconciliation Check
pub async fn run_reconciliation_worker(pool: &PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
    loop {
        interval.tick().await;

        let total_balances: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(balance_cents), 0) FROM wallets WHERE wallet_type = 'cash'"
        ).fetch_one(pool).await.unwrap_or(-1);

        let total_deposits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_cents), 0) FROM deposit_requests WHERE status = 'paid'"
        ).fetch_one(pool).await.unwrap_or(-1);

        let total_withdrawals: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_cents), 0) FROM withdrawal_requests WHERE status = 'completed'"
        ).fetch_one(pool).await.unwrap_or(0);

        let total_purchases: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_cents), 0) FROM orders WHERE status = 'completed'"
        ).fetch_one(pool).await.unwrap_or(0);

        let expected = total_deposits - total_withdrawals - total_purchases;
        let mismatch = (total_balances - expected).abs();

        if mismatch > 0 {
            let msg = format!(
                "🔴 RECONCILIATION MISMATCH: Wallets={}, Expected={} (deposits {} - withdrawals {} - purchases {}), Diff={}",
                total_balances, expected, total_deposits, total_withdrawals, total_purchases, mismatch
            );
            tracing::error!("{}", msg);
            sentry::capture_message(&msg, sentry::Level::Fatal);
        } else {
            tracing::info!("✅ Reconciliation check passed: {} = {}", total_balances, expected);
        }
    }
}
```

---

#### 3.1.9. Zusammenfassung: Dateien, Zeilen, Abhängigkeiten

| Datei | Geschätzte Zeilen | Abhängigkeiten | Kritikalität |
|---|---|---|---|
| `mod.rs` | ~60 | Alle Sub-Module | 🟢 Routing |
| `models.rs` | ~350 | `serde`, `sqlx`, `chrono`, `uuid` | 🟡 Datenverträge |
| `routes.rs` | ~400 | `models`, `service`, `validation`, `orderbook` | 🟡 API-Layer |
| `service.rs` | ~300 | `models`, `validation`, `orderbook` | 🟡 Orchestrierung |
| `orderbook.rs` | ~450 | `redis`, `models` | 🔴 Speed-Layer |
| `matching.rs` | ~300 | `orderbook`, `models` | 🔴 **KRITISCHSTER CODE** |
| `settlement.rs` | ~350 | `sqlx`, `models` (→ Settlement aus Sektion 2.5) | 🔴 **Geld bewegt sich** |
| `websocket.rs` | ~250 | `axum::ws`, `redis`, `tokio::broadcast` | 🟡 Echtzeit-UX |
| `p2p.rs` | ~300 | `models`, `settlement`, `validation` | 🟡 P2P-Trades |
| `charts.rs` | ~150 | `sqlx`, `models` | 🟢 Read-Only |
| `validation.rs` | ~350 | `sqlx`, `redis`, `models` | 🔴 Sicherheits-Gate |
| `background.rs` | ~250 | `sqlx`, `redis`, `orderbook` | 🟡 Housekeeping |
| **Gesamt** | **~3.510** | | |

**Reihenfolge der Implementierung (Bottom-Up):**

```
Woche 1:  models.rs → validation.rs → error.rs Erweiterung
Woche 2:  orderbook.rs → matching.rs
Woche 3:  settlement.rs → service.rs → routes.rs
Woche 4:  websocket.rs → p2p.rs → charts.rs → background.rs
```

> **Die goldene Regel:** `matching.rs` und `settlement.rs` werden ausschließlich vom Senior Rust Engineer geschrieben und von mindestens einer weiteren Person Code-Reviewed. Kein Merge ohne Review. Diese beiden Dateien bewegen echtes Geld – ein Bug hier ist kein UX-Problem, sondern ein finanzieller Verlust.

### 3.2. Smart Contract / Web3 Security Engineer (On-Chain Settlement)

Dieses Kapitel definiert die Architektur des **On-Chain Settlement Layers** für den POOOL Secondary Marketplace. Während das Orderbook und die Matching-Engine Off-Chain im Rust-Backend betrieben werden (für ~10ms Latenz und Zero-Gas-Trading), fungiert die **Base L2 Blockchain** als unveränderliches, rechtlich bindendes "Buchführungssystem" (Ledger) für die verbrieften Immobilien-Anteile (Real World Assets - RWA).

Das Protokoll basiert auf dem **ERC-3643 Standard (T-REX)**, um absolute Compliance (z. B. OJK-Regularien in Indonesien, globale AML/KYC-Pflichten) auf Protokollebene zu garantieren.

---

#### 3.2.1. ERC-3643 Architektur (T-REX)

Da wir reale Vermögenswerte (Immobilien) tokenisieren, können wir keinen simplen ERC-20 Standard verwenden. Ein freier Handel auf dezentralen Exchanges (DEXs) wie Uniswap muss zwingend unterbunden werden, um zu garantieren, dass **ausschließlich verifizierte User** Token halten können.

Das T-REX Framework (ERC-3643) teilt die Zuständigkeiten in mehrere modulare Smart Contracts auf:

```
┌──────────────────────────────────────────────────────────────────────┐
│                   ERC-3643 SMART CONTRACT ARCHITEKTUR                 │
│                                                                      │
│  ┌────────────────────────────┐                                      │
│  │  POOOL Token Contract      │  ← ERC-20 kompatibel                │
│  │  (pro Immobilie 1 Token)   │     + Compliance-Hooks              │
│  │                            │                                      │
│  │  • name: "Berlin Apt 42"   │                                      │
│  │  • symbol: "BA42"          │                                      │
│  │  • totalSupply: 1000       │                                      │
│  │  • decimals: 0 (ganzzahlig)│                                      │
│  └──────────┬─────────────────┘                                      │
│             │ prüft vor jedem Transfer:                               │
│             ▼                                                        │
│  ┌────────────────────────────┐    ┌──────────────────────────────┐  │
│  │  Identity Registry          │    │  Compliance Module           │  │
│  │  ─────────────────────────  │    │  ────────────────────────── │  │
│  │  • Wallet → ONCHAINID Map  │    │  • Max 80% pro Investor     │  │
│  │  • KYC-Status prüfen       │◄──►│  • Max 1000 Investoren      │  │
│  │  • Sanktionsliste prüfen   │    │  • Jurisdiktion-Regeln      │  │
│  │  • Country-Code (ID/DE/US) │    │  • Transfer-Einschränkungen │  │
│  └──────────┬─────────────────┘    └──────────────────────────────┘  │
│             │                                                        │
│             ▼                                                        │
│  ┌────────────────────────────┐    ┌──────────────────────────────┐  │
│  │  ONCHAINID (ERC-734/735)   │    │  Trusted Issuers Registry   │  │
│  │  ─────────────────────────  │    │  ────────────────────────── │  │
│  │  • Dezentrale Identität    │    │  • Didit.me = Trusted Issuer│  │
│  │  • Claims: KYC ✅, AML ✅  │    │  • POOOL = Trusted Issuer   │  │
│  │  • Pro User 1 ONCHAINID   │    │  • Claim Topics: KYC, AML,  │  │
│  │  • Verifiable Credentials   │    │    Accredited Investor      │  │
│  └────────────────────────────┘    └──────────────────────────────┘  │
│                                                                      │
│  Blockchain: Base L2 (Ethereum Layer 2)                             │
│  Kosten: ~$0.001 - $0.01 pro Transfer (vs. $5-50 auf Ethereum L1)  │
└──────────────────────────────────────────────────────────────────────┘
```

**Architektur-Update (Phase 7 Pivot):** Um eine strikte rechtliche Trennung (SPV Ring-Fencing) für Regulatoren und Whitelabel-Fähigkeit für B2B-Kunden zu garantieren, wird jeder RWA-Token nicht als eigenständiger, teurer Contract deployed, sondern über das **Factory and Clones Pattern (EIP-1167)**. Eine zentrale `AssetFactory` hält die Logik (den Implementation Contract) und erzeugt für jede neue Immobilie eine winzige Minimal-Proxy-Adresse, die auf die Logik verweist. Dies spart ~90% der Gas-Kosten beim Deployment neuer Immobilien und garantiert trotzdem jedem SPV seine eigene unverwechselbare Contract-Adresse auf der Blockchain (Polygon PoS / Base).

**Die vier Kernkomponenten im Detail:**

1.  **`IdentityRegistry` (IDR):** Das Herzstück der On-Chain-Compliance. Dieser Contract speichert, welche Wallet-Adressen mit einer verifizierten On-Chain-Identität (ONCHAINID) verknüpft sind. Vor jedem `transfer()` oder `transferFrom()` fragt der Token-Contract den IDR: *"Ist `to` ein verifizierter User?"*. Nur wenn `isVerified(to) == true`, wird der Transfer durchgelassen.

2.  **`TrustedIssuersRegistry`:** Legt fest, welche Instanzen berechtigt sind, Identity-Claims auszustellen. Im POOOL-Kontext ist das Rust-Backend (bzw. ein eigens dafür vorgesehener KMS-Key) der **Trusted Issuer**, der das KYC-Ergebnis (von *Didit.me* aus der PostgreSQL) kryptographisch signiert und On-Chain als Claim registriert.

3.  **`Compliance` Smart Contract (Modular):** Beinhaltet länderspezifische Regeln oder Limitierungen. POOOL kann hier Regeln hinterlegen wie: *"Investoren aus Land X dürfen maximal Y Tokens halten"* oder *"Maximal 1.000 Investoren pro Immobilie"*. Compliance-Module sind austauschbar, ohne den Token-Contract neu deployen zu müssen.

4.  **`Token` Smart Contract:** Der eigentliche RWA-Token (z. B. `p-VILLA-BALI-01`). Bevor eine `transfer()` oder `transferFrom()` Funktion ausgeführt wird, fragt der Token den `Compliance` und `IdentityRegistry` Contract ab. Nur wenn beide grünes Licht geben, geht der Transfer durch.

**Datenfluss des KYC-Onboardings (Off-Chain → On-Chain):**

```
┌──────────────┐     ┌───────────────┐     ┌──────────────────┐
│  1. User      │     │  2. Didit.me  │     │  3. Rust Backend │
│  startet KYC  │────►│  verifiziert  │────►│  Webhook empfängt│
│  im Frontend  │     │  Identität    │     │  KYC-Ergebnis    │
└──────────────┘     └───────────────┘     └────────┬─────────┘
                                                     │
                                                     ▼
                                           ┌──────────────────┐
                                           │  4. PostgreSQL    │
                                           │  is_kyc_verified  │
                                           │  = true           │
                                           └────────┬─────────┘
                                                     │
                                                     ▼
┌──────────────────────────────────────────────────────────────┐
│  5. Identity-Worker (Rust Tokio Task)                        │
│                                                              │
│  a) Erzeugt ONCHAINID für User (falls noch nicht vorhanden) │
│  b) Signiert KYC-Claim mit dem POOOL Issuer Key (GCP KMS)   │
│  c) Ruft IdentityRegistry.registerIdentity(wallet, onchainId│
│     , country_code) auf Base L2 auf                          │
│  d) Speichert on_chain_identity_address in PostgreSQL        │
└──────────────────────────────────────────────────────────────┘
```

Erst ab dem Moment, in dem Schritt 5 abgeschlossen ist, ist die Wallet des Users fähig, POOOL-Tokens zu empfangen. Das bedeutet: **Kein KYC = keine Tokens. Protokollebene. Unumgehbar.**

**Warum ERC-3643 und nicht ERC-20?**

| Feature | ERC-20 | ERC-3643 |
|---|---|---|
| Transfer | Jeder → Jeder ❌ | Nur verifizierte Nutzer ✅ |
| KYC/AML | Nicht möglich | Nativ eingebaut ✅ |
| Compliance-Regeln | Nicht möglich | Modular, pro Token konfigurierbar ✅ |
| Forced Transfer | Nicht möglich | Admin kann bei Betrug eingreifen ✅ |
| Token Recovery | Verloren = verloren | Wallet-Recovery möglich ✅ |
| Regulierung (OJK) | Nicht konform | Konform ✅ |

---

#### 3.2.2. Token Smart Contract Design

Der Token-Contract implementiert den ERC-3643 Standard. Transfers sind standardmäßig geblockt und erfordern eine erfolgreiche Validierung gegen die `IdentityRegistry` und das `Compliance`-Modul.

**Smart Contract Struktur:**

```
contracts/
├── token/
│   └── POOOLPropertyToken.sol      # ERC-3643 Token (1 pro Immobilie)
├── identity/
│   ├── IdentityRegistry.sol         # Wallet → ONCHAINID Mapping
│   ├── IdentityRegistryStorage.sol  # Shared Storage für alle Tokens
│   ├── ClaimTopicsRegistry.sol      # Welche Claims sind nötig?
│   └── TrustedIssuersRegistry.sol   # Wer darf Claims ausstellen?
├── compliance/
│   ├── ModularCompliance.sol        # Haupt-Compliance-Contract
│   └── modules/
│       ├── MaxOwnershipModule.sol   # Max 80% pro Investor
│       ├── MaxInvestorsModule.sol   # Max Anzahl Token-Holder
│       └── CountryRestrictionModule.sol  # Geo-Blocking (z.B. US)
├── settlement/
│   ├── BatchSettlement.sol          # Batch-Settlement für Marketplace
│   └── MerkleVerifier.sol           # Merkle-Root Verifikation
└── test/
    ├── Token.t.sol
    ├── Settlement.t.sol
    └── Compliance.t.sol
```

**Token-Contract (Kernlogik):**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@erc3643/contracts/token/Token.sol";
import "@erc3643/contracts/registry/interface/IIdentityRegistry.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

/// @title POOOL RWA Property Token Contract (ERC-3643)
/// @notice Repräsentiert einen tokenisierten Immobilienanteil auf dem POOOL Marktplatz.
/// @dev Ein Contract-Deployment pro Immobilie. decimals = 0 (ganzzahlige Shares).
contract POOOLPropertyToken is Token, AccessControl {
    bytes32 public constant SETTLEMENT_AGENT_ROLE = keccak256("SETTLEMENT_AGENT_ROLE");
    bytes32 public constant LEGAL_ADMIN_ROLE = keccak256("LEGAL_ADMIN_ROLE");

    // Immobilien-Metadata (on-chain unveränderlich)
    string public propertyAddress;      // "Friedrichstr. 42, 10117 Berlin"
    string public propertyType;         // "apartment", "commercial", "house"
    uint256 public valuationCents;      // Gutachten-Wert in Cents (BIGINT-Prinzip)
    uint256 public valuationTimestamp;  // Wann zuletzt bewertet
    string public documentsCID;         // IPFS CID der Eigentumsdokumente
    
    // Settlement-Tracking
    uint256 public lastSettlementBatch;
    bytes32 public lastMerkleRoot;
    
    event BatchSettled(uint256 indexed batchId, bytes32 merkleRoot, uint256 tradeCount);
    event ValuationUpdated(uint256 oldValue, uint256 newValue, uint256 timestamp);

    /// @dev Initialisiert den Token als ERC-3643
    constructor(
        address _identityRegistry,
        address _compliance,
        string memory _name,
        string memory _symbol,
        uint8 _decimals,       // Immer 0 für Immobilien-Tokens
        address _onchainID
    ) {
        _setupRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _setupRole(LEGAL_ADMIN_ROLE, msg.sender);
        
        // Setup ERC-3643 Core Dependencies
        Token.init(
            _identityRegistry, _compliance,
            _name, _symbol, _decimals, _onchainID
        );
    }

    // ──────────────────────────────────────────────────────────────
    //  TRANSFER HOOKS — Das Herzstück der Compliance
    // ──────────────────────────────────────────────────────────────

    /// @notice Überschriebener Hook, der VOR jedem Transfer aufgerufen wird.
    /// @dev Die base _beforeTokenTransfer Funktion ruft automatisch
    ///      identityRegistry.isVerified(to) und compliance.canTransfer() auf.
    ///      Wenn einer der Checks fehlschlägt → revert. Kein Transfer möglich.
    function _beforeTokenTransfer(
        address from,
        address to,
        uint256 amount
    ) internal virtual override {
        // ERC-3643 Compliance Check Logik greift hier vollautomatisch:
        //   1. identityRegistry.isVerified(to) → Hat Empfänger KYC?
        //   2. compliance.canTransfer(from, to, amount) → Regeln erfüllt?
        super._beforeTokenTransfer(from, to, amount);
        
        // POOOL Custom Logic:
        // In unserer Architektur passieren Retail-Trades nur im Off-Chain Orderbook.
        // Der Settlement-Agent settled in Batches via forcedTransfer().
        // Direct Wallet-to-Wallet Transfers sind möglich, aber durchlaufen
        // ebenfalls den vollen Compliance-Check.
    }

    // ──────────────────────────────────────────────────────────────
    //  MINTING — Nur durch Settlement Agent bei Primary Offering
    // ──────────────────────────────────────────────────────────────

    /// @notice Erlaubt dem POOOL-Backend Settlement Agent das Minten neuer Tokens
    ///         bei einem Primary Offering (Erstausgabe).
    function mint(address to, uint256 amount) external onlyRole(SETTLEMENT_AGENT_ROLE) {
        require(identityRegistry().isVerified(to), "Empfaenger nicht in IdentityRegistry");
        _mint(to, amount);
    }

    // ──────────────────────────────────────────────────────────────
    //  BATCH SETTLEMENT — Gas-effizienter Batch-Transfer
    // ──────────────────────────────────────────────────────────────

    /// @notice Batch-Settlement: Mehrere Trades in einer Transaktion.
    /// @dev Nur vom Settlement Agent (Backend-Worker) aufrufbar.
    ///      Nutzt forcedTransfer() (ERC-3643 Agent-Recht), was Approvals
    ///      umgeht, aber Compliance-Checks NICHT umgeht.
    function settleBatch(
        address[] calldata froms,
        address[] calldata tos,
        uint256[] calldata amounts,
        bytes32 merkleRoot
    ) external onlyRole(SETTLEMENT_AGENT_ROLE) {
        require(
            froms.length == tos.length && tos.length == amounts.length,
            "Array length mismatch"
        );
        
        for (uint256 i = 0; i < froms.length; i++) {
            // forcedTransfer umgeht NICHT die Compliance-Checks!
            // IdentityRegistry + Compliance werden trotzdem geprüft.
            forcedTransfer(froms[i], tos[i], amounts[i]);
        }
        
        lastSettlementBatch++;
        lastMerkleRoot = merkleRoot;
        
        emit BatchSettled(lastSettlementBatch, merkleRoot, froms.length);
    }
    
    /// @notice Gutachten-Wert aktualisieren (nur durch Agent)
    function updateValuation(uint256 _newValueCents) external onlyRole(SETTLEMENT_AGENT_ROLE) {
        uint256 oldValue = valuationCents;
        valuationCents = _newValueCents;
        valuationTimestamp = block.timestamp;
        emit ValuationUpdated(oldValue, _newValueCents, block.timestamp);
    }
}
```

**Der entscheidende Sicherheitsmechanismus:** Die Funktion `identityRegistry().isVerified(to)` garantiert, dass Tokens nicht an ein anonymes Wallet fließen können. Selbst wenn ein Angreifer den Settlement-Agent-Key kompromittiert, kann er die Tokens **ausschließlich** an KYC-verifizierte Wallets senden. Das reduziert die Angriffsfläche enorm.

---

#### 3.2.3. Asynchrones Batch-Settlement (Rollup-Prinzip)

Das Off-Chain-Orderbook von POOOL kann an volatilen Tagen (z.B. Dividend-Auszahlung, Immobilien-News) **10.000+ Trades pro Immobilie** verarbeiten. Jeden Trade einzeln auf Base L2 abzuwickeln, ist (trotz L2) ökonomisch und technisch ineffizient und würde den "Null Gas-Fee" USP für den Nutzer zerstören.

**Das POOOL "Net-Settlement" Rollup-Prinzip:**

```
Off-Chain (Rust)                        On-Chain (Base L2)
┌─────────────────────────┐             ┌─────────────────────┐
│ 10.000 Trades/Tag       │             │                     │
│                         │   Netting   │  1 Transaktion      │
│ User A: +50, -30 = +20  │────────────►│  mit ~150 Netto-    │
│ User B: -20             │             │  Positionen         │
│ User C: +10, -10 = 0    │  (C fällt   │                     │
│ ...                     │   raus!)    │  Gas: ~$0.04-$0.20  │
└─────────────────────────┘             └─────────────────────┘
```

**Die drei Phasen:**

1.  **Aggregations-Phase (Off-Chain):** Das Rust-Backend sammelt alle Trades eines fixen Zeitfensters (z.B. alle 4 Stunden oder am Ende des Tages) aus der `trade_history` Tabelle.

2.  **Netting (Off-Chain):** Wenn User A heute 50 Token gekauft und 30 verkauft hat, beträgt sein *Net Difference* `+20`. User B hat 20 verkauft, also `-20`. User C hat 10 gekauft und 10 verkauft → Netto `0` → **fällt komplett raus**. Aus 10.000 Trades wird eine komprimierte Liste von z.B. 150 Usern, deren Netto-Balance sich tatsächlich verändert hat.

3.  **Array-Batching (On-Chain):** Für maximale Gas-Effizienz packen wir die gematchten Net-Balances in `calldata`-Arrays. Auf L2 übergeben wir parallele Arrays (`sellers`, `sellAmounts`, `buyers`, `buyAmounts`), deren Summen **immer identisch** sein müssen (Erhaltungssatz: kein Token darf aus dem Nichts entstehen oder verschwinden).

**Rust Settlement-Worker Implementation:**

```rust
// backend/src/worker/settlement_worker.rs

use alloy::providers::ProviderBuilder;
use alloy::network::EthereumWallet;
use alloy::sol;
use sqlx::PgPool;
use std::collections::HashMap;

// Generiere Rust Types aus dem Solidity ABI zur Compile-Zeit!
sol!(
    #[sol(rpc)]
    POOOLPropertyToken,
    "../contracts/out/POOOLPropertyToken.sol/POOOLPropertyToken.json"
);

/// Aggregiert alle un-gesettelten Trades zu Netto-Positionen.
/// Input:  10.000 individuelle Trades
/// Output: ~150 Users mit Netto-Veränderung != 0
async fn aggregate_net_positions(
    db: &PgPool,
    asset_id: uuid::Uuid,
) -> Result<NetPositions, AppError> {
    // Alle pending Trades für dieses Asset holen
    let trades = sqlx::query_as::<_, TradeForSettlement>(
        "SELECT th.id, th.buyer_user_id, th.seller_user_id,
                th.quantity,
                u_buyer.wallet_address AS buyer_wallet,
                u_seller.wallet_address AS seller_wallet
         FROM trade_history th
         JOIN users u_buyer ON th.buyer_user_id = u_buyer.id
         JOIN users u_seller ON th.seller_user_id = u_seller.id
         WHERE th.on_chain_status = 'pending'
         AND th.asset_id = $1
         ORDER BY th.executed_at ASC
         LIMIT 5000"  // Max 5000 Trades pro Batch-Run
    ).bind(asset_id).fetch_all(db).await?;

    if trades.is_empty() {
        return Ok(NetPositions::empty());
    }

    // Netting: Berechne Netto-Veränderung pro Wallet
    let mut net_changes: HashMap<String, i64> = HashMap::new();
    
    for trade in &trades {
        // Buyer bekommt Tokens → positive Veränderung
        *net_changes.entry(trade.buyer_wallet.clone()).or_default() 
            += trade.quantity as i64;
        // Seller gibt Tokens ab → negative Veränderung
        *net_changes.entry(trade.seller_wallet.clone()).or_default() 
            -= trade.quantity as i64;
    }

    // Entferne alle Wallets mit Netto-Veränderung = 0 (z.B. Day-Trader)
    net_changes.retain(|_, v| *v != 0);

    // Aufteilen in Sellers (negative) und Buyers (positive)
    let mut sellers = Vec::new();
    let mut sell_amounts = Vec::new();
    let mut buyers = Vec::new();
    let mut buy_amounts = Vec::new();

    for (wallet, change) in &net_changes {
        if *change < 0 {
            sellers.push(wallet.parse::<Address>()?);
            sell_amounts.push(U256::from(change.unsigned_abs()));
        } else {
            buyers.push(wallet.parse::<Address>()?);
            buy_amounts.push(U256::from(*change as u64));
        }
    }

    // INVARIANTE: Summe der Verkäufe MUSS = Summe der Käufe sein
    let total_sold: u64 = sell_amounts.iter().map(|a| a.as_limbs()[0]).sum();
    let total_bought: u64 = buy_amounts.iter().map(|a| a.as_limbs()[0]).sum();
    assert_eq!(total_sold, total_bought, "CRITICAL: Net positions don't sum to zero!");

    Ok(NetPositions {
        sellers, sell_amounts, buyers, buy_amounts,
        trade_ids: trades.iter().map(|t| t.id).collect(),
        trade_count: trades.len(),
    })
}

/// Hauptfunktion: Sendet den aggregierten Batch an Base L2.
pub async fn run_end_of_day_settlement(
    db: &PgPool,
    asset_id: uuid::Uuid,
) -> Result<(), AppError> {
    let positions = aggregate_net_positions(db, asset_id).await?;
    if positions.is_empty() {
        tracing::info!("No pending trades for settlement");
        return Ok(());
    }

    // Merkle Tree als Beweis-Root generieren
    let leaves: Vec<[u8; 32]> = positions.trade_ids.iter().map(|id| {
        keccak256(id.as_bytes())
    }).collect();
    let merkle_root = MerkleTree::new(&leaves).root();

    // Alloy Provider mit Google Cloud KMS Signer initialisieren
    let gcp_key = std::env::var("GCP_KMS_KEY_NAME")
        .map_err(|_| AppError::ConfigError("GCP_KMS_KEY_NAME not set"))?;
    let signer = GcpKmsSigner::new(gcp_key).await?;
    let wallet = EthereumWallet::from(signer);
    
    let rpc_url = std::env::var("BASE_L2_RPC_URL")?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url.parse()?);

    let contract_address = std::env::var("TOKEN_CONTRACT_ADDRESS")?.parse()?;
    let contract = POOOLPropertyToken::new(contract_address, provider);

    // ── On-Chain Transaktion senden ──
    let tx_builder = contract.settleBatch(
        positions.sellers,
        positions.buyers,  
        positions.buy_amounts,
        merkle_root,
    ).gas(5_000_000); // L2 Gas Limit

    let pending_tx = tx_builder.send().await
        .map_err(|e| AppError::Web3Error(e.to_string()))?;
    let receipt = pending_tx.get_receipt().await
        .map_err(|_| AppError::TxReverted)?;

    if receipt.status() {
        // DB aktualisieren: Orders → "confirmed"
        sqlx::query(
            "UPDATE trade_history 
             SET on_chain_status = 'confirmed',
                 on_chain_tx_hash = $1,
                 on_chain_confirmed_at = NOW()
             WHERE id = ANY($2)"
        ).bind(receipt.transaction_hash.to_string())
        .bind(&positions.trade_ids)
        .execute(db).await?;

        tracing::info!(
            "✅ Batch settled: {} trades, {} net transfers, TX: {}",
            positions.trade_count,
            positions.sellers.len() + positions.buyers.len(),
            receipt.transaction_hash
        );
    } else {
        tracing::error!("🔴 Settlement TX reverted on-chain!");
        // Retry-Logik oder Sentry-Alert
    }

    Ok(())
}
```

**Gas-Kosten auf Base L2 (Batching-Strategie):**

| Operation | Gas (Base L2) | Kosten L2 |
|---|---|---|
| `transfer()` (1 Trade) | ~65.000 | **~$0.001** |
| `settleBatch()` (10 Trades) | ~400.000 | **~$0.005** |
| `settleBatch()` (100 Trades) | ~3.500.000 | **~$0.04** |
| `settleBatch()` (500 Trades) | ~17.000.000 | **~$0.20** |
| Neuen Token deployen | ~5.000.000 | **~$0.06** |

> **Warum Base L2?** Base (von Coinbase) ist ein Ethereum Layer 2 mit identischer EVM-Kompatibilität. Gas-Kosten sind 100-1000x günstiger als Ethereum L1. Settlement von 500 Trades kostet ~$0.20 statt ~$50 auf L1.

**Dynamische Batching-Frequenz:**

| Szenario | Settlement-Frequenz | Grund |
|---|---|---|
| <10 Trades/Tag | 1x täglich (24h) | Gas sparen, Trades sammeln |
| 10-100 Trades/Tag | 2x täglich (12h) | Balance zwischen Speed und Kosten |
| >100 Trades/Tag | 4x täglich (6h) | Mehr Trades → häufigere Bestätigung |
| Admin-Trigger | Sofort | Manuelle Notfall-Settlement |

**Neues DB-Schema für Settlement-Tracking:**

```sql
-- Trade-History Erweiterung
ALTER TABLE trade_history ADD COLUMN on_chain_status VARCHAR(20) 
    DEFAULT 'pending' CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed'));
ALTER TABLE trade_history ADD COLUMN on_chain_tx_hash VARCHAR(66);
ALTER TABLE trade_history ADD COLUMN on_chain_batch_id UUID;  -- Referenz zur settlement_batches Tabelle
ALTER TABLE trade_history ADD COLUMN on_chain_confirmed_at TIMESTAMPTZ;

-- Settlement-Batch-Log (Audit)
CREATE TABLE settlement_batches (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID REFERENCES assets(id) NOT NULL,
    batch_id        BIGINT NOT NULL,
    merkle_root     VARCHAR(66) NOT NULL,
    tx_hash         VARCHAR(66),
    trade_count     INTEGER NOT NULL,
    net_transfer_count INTEGER NOT NULL,  -- Nach Netting
    gas_used        BIGINT,
    gas_cost_wei    BIGINT,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message   TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    settled_at      TIMESTAMPTZ,
    UNIQUE (asset_id, batch_id)
);
```

---

#### 3.2.4. Administrator & Force Transfers

Ein Kernfeature von Security Tokens (RWA) gegenüber Utility Tokens ist die **rechtliche Übersteuerbarkeit**. Da eine physische Immobilie gesetzlichen Regeln unterliegt (z.B. OJK oder Bappebti in Indonesien, BaFin in Deutschland), muss POOOL im Notfall in der Lage sein, Besitzverhältnisse On-Chain zu korrigieren.

**ERC-3643 Rollen-System:**

| Rolle | Wer | Berechtigungen |
|---|---|---|
| **Owner** | POOOL Multisig Wallet (3-von-5) | Deploy, Compliance-Module ändern, Agents ernennen |
| **Settlement Agent** | Backend-Worker Wallet (Hot Wallet) | `forcedTransfer()`, `settleBatch()` |
| **Identity Agent** | Admin Backend | `registerIdentity()`, `deleteIdentity()`, Claims verwalten |
| **Freeze Agent** | Admin (manuell) | `freeze()` einzelne Wallets bei Verdacht |
| **Recovery Agent** | POOOL Support (2-von-3 Multisig) | `recoveryAddress()` bei verlorenen Wallets |

**Szenario 1: Verlust des Private Keys (Key Recovery)**

```
1. Investor verliert Seed-Phrase / Handy gestohlen
2. User kontaktiert Support mit erneuter KYC-Verifikation (Didit.me Video-Re-Check)
3. Support erstellt Recovery-Request (2-von-3 Multisig nötig)
4. User gibt seine NEUE Wallet-Adresse an
5. ONCHAINID wird auf neue Wallet umregistriert:
   → IdentityRegistry.updateIdentity(alte_wallet, neue_wallet)
6. contract.recoveryAddress(alte_wallet, neue_wallet, user_onchainid)
   → Alle Tokens transferieren sich zur neuen Wallet
7. Alte Wallet wird automatisch frozen
8. Audit-Log: On-Chain Event + PostgreSQL Entry
```

**Szenario 2: Behördliche Enteignung / Gerichtsbeschluss**

```
1. Rechtsabteilung erhält offiziellen Gerichtsbeschluss (OJK / BaFin)
2. Freeze Agent friert Wallet sofort ein:
   → contract.freeze(user_wallet) — On-Chain Transaktion
   → PostgreSQL: UPDATE users SET wallet_frozen = true
3. Owner (Multisig) genehmigt Force-Transfer
4. 48h Timelock startet (User wird benachrichtigt per E-Mail)
5. Nach Timelock: Tokens werden an gerichtlich bestimmte Wallet übertragen
   → contract.forceTransfer(user_wallet, court_wallet, amount)
6. Komplett geloggt im Audit-Trail (On-Chain Events + Off-Chain DB)
```

**Szenario 3: Erbfall (Inheritance)**

```
1. Nachweis über Sterbeurkunde + Erbschein an Rechtsabteilung
2. Erben durchlaufen vollständigen KYC-Prozess (Didit.me)
3. ONCHAINID wird für Erben erstellt und in IdentityRegistry registriert
4. Owner (Multisig) genehmigt Force-Transfer an Erben-Wallet
5. Tokens werden proportional an alle Erben verteilt
```

**Sicherheitsregeln für Agent-Wallets:**

```
Settlement Agent (Hot Wallet via GCP KMS):
├── Separate Wallet NUR für Settlements
├── Minimaler ETH-Bestand (nur für Gas, ~0.01 ETH auto-nachfüllen)
├── Keine andere Berechtigung als forcedTransfer/settleBatch
├── Auto-Rotation: KMS Key wird alle 90 Tage rotiert
└── Alert: Wenn Balance > 0.1 ETH → Sentry-Alarm

Owner (Multisig — Gnosis Safe):
├── Gnosis Safe / Safe{Wallet} (3-von-5 Signers)
├── Signer: CEO, CTO, Lead Dev, Legal, Treuhänder
├── Timelock: 48h Verzögerung bei Compliance-Änderungen
└── Hardware-Wallets (Ledger) für alle Signer — PFLICHT
```

> **Goldene Regel:** Die `forceTransfer()`-Berechtigung liegt **niemals** auf einem Hot-Wallet des Rust-Backends für Admin-Zwecke. Für regulatorische Force Transfers (Enteignung, Erbfall) ist immer die Gnosis Safe Multisig-Unterschrift erforderlich. Der Settlement Agent darf `forcedTransfer()` nur im Kontext von `settleBatch()` aufrufen.

---

#### 3.2.5. Foundry & Fuzz Testing Strategy

Sicherheit hat bei POOOL oberste Priorität. Der Testing-Stack stützt sich vollständig auf **Foundry** (`forge`). Ein externer Smart Contract Auditor (z.B. Trail of Bits, OpenZeppelin, Halborn) wird vor allem die **Invarianten** des Systems prüfen.

```bash
# Testumgebung: Foundry (Forge + Cast + Anvil)
# Alle Tests laufen auf lokalem Anvil-Node (Fork von Base Sepolia)

forge test --fork-url https://sepolia.base.org -vvv
```

**Test-Kategorien:**

| Test-Kategorie | Was wird getestet | Prio |
|---|---|---|
| **Unit Tests** | Token mint/burn, transfer, freeze | 🔴 Kritisch |
| **Compliance Tests** | Transfer blockiert wenn KYC fehlt | 🔴 Kritisch |
| **Settlement Tests** | Batch-Settlement mit 1, 10, 100, 500 Trades | 🔴 Kritisch |
| **Recovery Tests** | Wallet-Recovery Prozess End-to-End | 🟡 Wichtig |
| **Gas Tests** | Gas-Verbrauch pro Operation messen + Regression | 🟡 Wichtig |
| **Edge Cases** | Transfer an sich selbst, 0-Amount, Overflow | 🔴 Kritisch |
| **Fuzz Tests** | Random Inputs über 10.000 Iterationen | 🟡 Wichtig |
| **Invariant Tests** | `totalSupply == SUM(balanceOf)` muss immer gelten | 🔴 Kritisch |

**Beispiel: Invarianten & Fuzz-Tests (`POOOLPropertyToken.t.sol`):**

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/token/POOOLPropertyToken.sol";

contract POOOLPropertyTokenTest is Test {
    POOOLPropertyToken token;
    
    address admin = address(0x1);
    address settlementAgent = address(0x2);
    address[] public holders;

    // ── Mock Contracts für IDR und Compliance ──
    MockIdentityRegistry mockIdr;
    MockCompliance mockCompliance;

    function setUp() public {
        vm.startPrank(admin);
        
        mockIdr = new MockIdentityRegistry();
        mockCompliance = new MockCompliance();
        
        token = new POOOLPropertyToken(
            address(mockIdr), address(mockCompliance),
            "POOOL Villa Bali", "pVB", 0, address(0)
        );
        
        token.grantRole(token.SETTLEMENT_AGENT_ROLE(), settlementAgent);
        
        // 5 verifizierte Test-User registrieren
        for (uint i = 10; i < 15; i++) {
            address user = address(uint160(i));
            mockIdr.addVerified(user);
            holders.push(user);
        }
        
        // Initial Mint: 1000 Tokens an User 0
        vm.stopPrank();
        vm.prank(settlementAgent);
        token.mint(holders[0], 1000);
    }

    // ══════════════════════════════════════════════════════════════
    //  INVARIANT TESTS — Müssen IMMER gelten, unter allen Umständen
    // ══════════════════════════════════════════════════════════════

    /// @notice totalSupply muss IMMER gleich der Summe aller Balances sein.
    function invariant_totalSupplyConsistency() external view {
        uint256 sum = 0;
        for (uint i = 0; i < holders.length; i++) {
            sum += token.balanceOf(holders[i]);
        }
        assertEq(
            token.totalSupply(), sum,
            "INVARIANT BROKEN: totalSupply != sum of balances"
        );
    }

    /// @notice Kein einzelner Holder darf mehr als totalSupply besitzen.
    function invariant_noHolderExceedsTotalSupply() external view {
        for (uint i = 0; i < holders.length; i++) {
            assertLe(
                token.balanceOf(holders[i]), token.totalSupply(),
                "INVARIANT BROKEN: holder balance > totalSupply"
            );
        }
    }

    // ══════════════════════════════════════════════════════════════
    //  FUZZ TESTS — Zufällige Inputs bombardieren die Funktionen
    // ══════════════════════════════════════════════════════════════

    /// @notice Fuzz-Test: Batch-Settlement mit zufälligen Mengen.
    ///         Verifiziert, dass ungleiche Arrays rigoros abgelehnt werden.
    function testFuzz_RevertOnMismatchedArrays(
        uint8 fromLen,
        uint8 toLen
    ) public {
        vm.assume(fromLen != toLen);
        vm.assume(fromLen > 0 && toLen > 0 && fromLen < 50 && toLen < 50);
        
        address[] memory froms = new address[](fromLen);
        address[] memory tos = new address[](toLen);
        uint256[] memory amounts = new uint256[](fromLen);

        vm.prank(settlementAgent);
        vm.expectRevert("Array length mismatch");
        token.settleBatch(froms, tos, amounts, bytes32(0));
    }

    // ══════════════════════════════════════════════════════════════
    //  COMPLIANCE TESTS — KYC-Enforcement
    // ══════════════════════════════════════════════════════════════

    /// @notice Transfer an nicht-verifizierte Wallet MUSS revertieren.
    function test_transferBlockedWithoutKYC() external {
        address unverifiedUser = makeAddr("unverified");
        // unverifiedUser ist NICHT in mockIdr registriert
        
        vm.expectRevert("Identity not found");
        vm.prank(holders[0]); // holders[0] hat 1000 Tokens
        token.transfer(unverifiedUser, 10);
    }

    /// @notice Mint an nicht-verifizierte Wallet MUSS revertieren.
    function test_mintBlockedWithoutKYC() external {
        address unverifiedUser = makeAddr("unverified2");
        
        vm.prank(settlementAgent);
        vm.expectRevert("Empfaenger nicht in IdentityRegistry");
        token.mint(unverifiedUser, 100);
    }

    /// @notice Transfer zwischen verifizierten Usern MUSS funktionieren.
    function test_transferBetweenVerifiedUsers() external {
        vm.prank(holders[0]);
        token.transfer(holders[1], 50);
        
        assertEq(token.balanceOf(holders[0]), 950);
        assertEq(token.balanceOf(holders[1]), 50);
    }

    // ══════════════════════════════════════════════════════════════
    //  EDGE CASE TESTS
    // ══════════════════════════════════════════════════════════════

    /// @notice Transfer von 0 Tokens sollte keinen Fehler werfen.
    function test_zeroAmountTransfer() external {
        vm.prank(holders[0]);
        token.transfer(holders[1], 0);
        // Kein Revert = Test bestanden
    }

    /// @notice Transfer an sich selbst.
    function test_selfTransfer() external {
        uint256 balanceBefore = token.balanceOf(holders[0]);
        vm.prank(holders[0]);
        token.transfer(holders[0], 100);
        assertEq(token.balanceOf(holders[0]), balanceBefore);
    }
}
```

**Worauf der SC Auditor besonders achten wird:**

1.  **Reentrancy im Batch-Settlement:** Kann ein bösartiger `receive()`-Hook im Empfänger-Wallet die `settleBatch()`-Schleife manipulieren? → CEI-Pattern (Checks-Effects-Interactions) muss eingehalten werden.
2.  **Access Control Bypass:** Kann ein Angreifer den `SETTLEMENT_AGENT_ROLE` umgehen, z. B. durch Proxy-Chains, `delegatecall` oder Storage-Collisions?
3.  **Integer Overflow/Underflow:** Obwohl Solidity ≥0.8 native Overflow-Checks hat, könnte `unchecked {}` versehentlich genutzt werden.
4.  **Front-Running:** Kann ein Miner/Sequencer die Reihenfolge der Batch-Transaktionen manipulieren? (Auf Base L2 limitiert, da Coinbase der einzige Sequencer ist.)
5.  **Storage Layout bei Upgrades:** Wenn UUPS Proxy genutzt wird, müssen Storage Gaps korrekt eingefügt sein. Keine Storage-Collisions zwischen Proxy und Implementation.

---

#### 3.2.6. Backend Integration via Alloy-rs

Das POOOL Rust-Backend (Axum) enthält einen **Settlement-Worker** (Tokio Task), der periodisch aufwacht, die PostgreSQL-Datenbank abfragt, alle un-gesettelten Orderbook-Transfers gruppiert und den `POOOLPropertyToken` Contract auf Base L2 aufruft.

Für die Blockchain-Interaktion nutzen wir **Alloy (alloy-rs)** — die High-Performance Nachfolger-Library von ethers-rs (deprecated seit 2024).

**Private Key Management (Google Cloud KMS):**

```
┌──────────────────┐     ┌───────────────────┐     ┌──────────────┐
│  Rust Backend    │     │  Google Cloud KMS  │     │  Base L2     │
│  (Settlement     │     │  (HSM-geschützt)   │     │  (Blockchain)│
│   Worker)        │     │                    │     │              │
│                  │     │  ┌──────────────┐  │     │              │
│  1. TX Hash ────────────►│ Private Key   │  │     │              │
│     berechnen    │     │  │ (verlässt    │  │     │              │
│                  │     │  │  HSM NIE)    │  │     │              │
│  2. Signatur ◄──────────│              │  │     │              │
│     empfangen    │     │  └──────────────┘  │     │              │
│     (v, r, s)    │     │                    │     │              │
│                  │     └───────────────────┘     │              │
│  3. Signierte ──────────────────────────────────►│ TX           │
│     TX senden    │                               │ ausführen    │
└──────────────────┘                               └──────────────┘
```

> **Sicherheitsaspekt:** Das Backend hat **niemals** den Private Key des Settlement Agents im Klartext in der `.env` Datei oder im Speicher. Der Key verlässt den Google HSM-Chip (Hardware Security Module) **nie**. Das Backend schickt den Transaction-Hash an GCP, bekommt die Signatur (V, R, S) zurück und sendet die fertig signierte Transaktion via Alchemy/Infura RPC an Base L2.

**Alloy-rs Typ-Sicherheit:**

Die `alloy::sol!` Makro generiert Rust-Typen direkt aus dem Solidity ABI zur Compile-Zeit. Änderungen im Smart Contract Code führen automatisch zu **Compile-Fehlern** im Rust-Backend — das macht die CI/CD Pipeline extrem robust:

```rust
// Compile-Zeit Garantie: Wenn sich die settleBatch() Signatur
// im Solidity ändert, kompiliert das Rust-Backend nicht mehr.
sol!(
    #[sol(rpc)]
    POOOLPropertyToken,
    "../contracts/out/POOOLPropertyToken.sol/POOOLPropertyToken.json"
);
```

**Neue Environment Variables:**

| Key | Required | Description |
|-----|----------|-------------|
| `BASE_L2_RPC_URL` | ✅ | RPC-Endpunkt für Base L2 (Alchemy/Infura) |
| `GCP_KMS_KEY_NAME` | ✅ | Google Cloud KMS Key Resource Name |
| `TOKEN_CONTRACT_ADDRESS` | ✅ | Adresse des deployed POOOLPropertyToken |
| `SETTLEMENT_BATCH_INTERVAL_SECS` | ❌ | Default: 86400 (24h) |

---

#### 3.2.7. Deployment & Audit Checklist

Bevor ein RWA-Contract im Base Mainnet live geht, muss das Web3 Engineering Team die folgende Checkliste **vollständig** abarbeiten. RWA hat null Fehlertoleranz.

**Deployment-Reihenfolge (strikt einhalten!):**

```
Phase 1: Testnet (Base Sepolia)
├── 1. ONCHAINID Factory deployen
├── 2. Claim Topics Registry deployen (Topic: KYC = 1, AML = 2)
├── 3. Trusted Issuers Registry deployen (Issuer: POOOL KMS public key)
├── 4. Identity Registry Storage deployen
├── 5. Identity Registry deployen (verbindet 2-4)
├── 6. Compliance Module deployen (MaxOwnership, CountryRestriction)
├── 7. POOOLPropertyToken deployen (verbindet 5 + 6)
├── 8. Settlement Agent Wallet als Agent registrieren
├── 9. Test-Identitäten registrieren (5 Test-User)
├── 10. Batch-Settlement testen (10 simulierte Trades)
│
Phase 2: Security Audit
├── Externer Audit durch Trail of Bits / OpenZeppelin / Halborn
├── Alle Findings fixen und re-testen
├── Audit-Report auf POOOL Homepage veröffentlichen
│
Phase 3: Mainnet (Base Mainnet)
├── Identische Deployment-Reihenfolge wie Testnet
├── Owner = Gnosis Safe Multisig (3-von-5)
├── Alle Contracts verifizieren auf Basescan (forge create --verify)
├── Timelock Contract deployen (48h Delay)
└── Monitoring: Settlement-Worker Health-Check alle 5 Minuten
```

**Security-Checkliste (vor Mainnet-Launch):**

| # | Check | Status | Verantwortlich |
|---|---|---|---|
| 1 | Externer Smart Contract Audit abgeschlossen | ❌ | Web3 Security Engineer |
| 2 | Alle Audit-Findings (High + Medium) gefixt und verifiziert | ❌ | Web3 Engineer |
| 3 | Owner-Wallet = Multisig (nicht einzelne Person) | ❌ | CTO + Legal |
| 4 | Settlement Agent = GCP KMS (kein Klartext-Key) | ❌ | DevOps |
| 5 | Timelock (48h) auf Compliance-Änderungen aktiv | ❌ | Web3 Engineer |
| 6 | Alle Contracts auf Basescan verifiziert (Source Code) | ❌ | Web3 Engineer |
| 7 | UUPS Proxy Pattern korrekt (Storage Layout geprüft) | ❌ | Web3 Engineer |
| 8 | Freeze-Funktionalität getestet (On-Chain + Off-Chain sync) | ❌ | QA |
| 9 | Recovery-Prozess E2E getestet (alte Wallet frozen, neue aktiv) | ❌ | QA + Support |
| 10 | Gas-Limit für settleBatch() korrekt (kein Out-of-Gas bei 500 Trades) | ❌ | Web3 Engineer |
| 11 | Monitoring: Alert wenn Settlement >24h ausstehend | ❌ | DevOps |
| 12 | Private Keys für alle Agent-Wallets in GCP KMS / Secret Manager | ❌ | DevOps |
| 13 | Audit-Report PDF auf POOOL Homepage veröffentlicht | ❌ | PM + Legal |
| 14 | Invariant Tests laufen in CI/CD Pipeline bei jedem Commit | ❌ | Web3 Engineer |
| 15 | Emergency Pause implementiert (circuit breaker bei Anomalien) | ❌ | Web3 Engineer |

---

#### 3.2.8. SPV Legal Wrapper & IPFS-Dokumenten-Permanenz

> **Kontext:** Dieses Kapitel basiert auf den Anforderungen aus der `SMART_CONTRACT_IMPLEMENTATION.md` (Sektion 1 & 9) und integriert sie in den Marketplace Masterplan. Ohne SPV gibt es keinen rechtsgültigen Eigentumsanspruch – der Token wäre ein leeres Versprechen.

**Das SPV-Modell (Special Purpose Vehicle):**

```
┌─────────────────────────────────────────────────────────────────┐
│  RECHTLICHE STRUKTUR (pro Immobilie)                             │
│                                                                  │
│  ┌──────────────┐                                                │
│  │  POOOL GmbH  │ ── Betreibt Plattform, NICHT Eigentümer       │
│  │  (Operator)  │    der Immobilien!                             │
│  └──────┬───────┘                                                │
│         │ gründet                                                │
│         ▼                                                        │
│  ┌──────────────────┐     ┌──────────────────────┐               │
│  │  SPV LLC/UG #1   │     │  SPV LLC/UG #2       │               │
│  │  "Villa Bali 01" │     │  "Apt Berlin 42"     │               │
│  │                  │     │                      │               │
│  │  Besitzt: 1 Villa│     │  Besitzt: 1 Apartment │               │
│  │  Token: p-VB-01  │     │  Token: p-AB-42      │               │
│  │  Supply: 1000    │     │  Supply: 3000        │               │
│  └──────────────────┘     └──────────────────────┘               │
│         ▲                          ▲                             │
│         │ Token repräsentiert      │ Token repräsentiert         │
│         │ Gesellschaftsanteile     │ Gesellschaftsanteile        │
│         │                          │                             │
│  ┌──────┴──────┐            ┌──────┴──────┐                     │
│  │  Investoren │            │  Investoren │                     │
│  │  (Token-    │            │  (Token-    │                     │
│  │   Holder)   │            │   Holder)   │                     │
│  └─────────────┘            └─────────────┘                     │
│                                                                  │
│  Wenn POOOL bankrott geht:                                       │
│  → SPVs bleiben intakte, separate Rechtseinheiten               │
│  → Token-Holder besitzen weiterhin ihre Anteile                 │
│  → Blockchain-Ledger ist unveränderlich                          │
└─────────────────────────────────────────────────────────────────┘
```

**SPV-Lifecycle (Wann wird was gegründet?):**

| Phase | Aktion | Verantwortlich | Abhängigkeit |
|---|---|---|---|
| 1 | SPV gegründet (LLC/UG) für die Immobilie | Legal / PM | Vor allem anderen |
| 2 | SPV Operating Agreement erstellt (PDF) | Anwalt | SPV gegründet |
| 3 | Immobilie wird auf SPV übertragen (Grundbuch) | Notar / Legal | SPV gegründet |
| 4 | SPV-Dokumente auf IPFS gepinnt (Pinata) | DevOps | Dokument fertig |
| 5 | IPFS CID wird im Token-Contract hinterlegt | Web3 Engineer | Deploy |
| 6 | Token deployed mit `documentsCID = ipfs://Qm...` | Web3 Engineer | CID vorhanden |
| 7 | Admin verknüpft Asset-DB mit Contract-Adresse | Backend | Token deployed |

**Notwendige DB-Felder auf `assets` Tabelle:**

```sql
-- Migration: 050d_alter_assets_blockchain.sql
ALTER TABLE assets ADD COLUMN contract_address VARCHAR(42) UNIQUE;
ALTER TABLE assets ADD COLUMN deployment_tx_hash VARCHAR(66);
ALTER TABLE assets ADD COLUMN spv_entity_name VARCHAR(200);
ALTER TABLE assets ADD COLUMN spv_jurisdiction VARCHAR(50);  -- 'ID', 'DE', 'US'
ALTER TABLE assets ADD COLUMN documents_ipfs_cid VARCHAR(100);
ALTER TABLE assets ADD COLUMN blockchain_status VARCHAR(20) DEFAULT 'draft'
    CHECK (blockchain_status IN ('draft', 'deploying', 'live', 'paused', 'frozen'));
```

**IPFS Pinning Workflow (Pinata):**

```
1. Admin uploaded SPV-Dokument im Admin-Dashboard
2. Backend sendet Datei an Pinata API:
   POST https://api.pinata.cloud/pinning/pinFileToIPFS
   → Response: { "IpfsHash": "QmX7b9f..." }
3. Backend speichert CID in PostgreSQL:
   UPDATE assets SET documents_ipfs_cid = 'QmX7b9f...' WHERE id = $1
4. Beim Token-Deploy: CID wird im Constructor hinterlegt
5. Verifizierung: Backend prüft IPFS-Gateway Erreichbarkeit:
   GET https://gateway.pinata.cloud/ipfs/QmX7b9f...
```

> **⚠️ KRITISCH:** SPV-Dokumente dürfen **NICHT** auf GCS/S3 gehostet werden. Wenn POOOL-Billing stoppt → 404 → Ownership-Beweis weg. IPFS ist dezentral und überlebt POOOL-Insolvenz. Für absolute Permanenz: Zusätzlich auf **Arweave** (einmalige Zahlung, permanente Speicherung).

**AssetFactory Contract (Automatisiertes Deploy):**

Bei 50+ Immobilien ist manuelles Deployment nicht skalierbar. Ein Factory-Contract deployed neue Token automatisch:

```solidity
// contracts/factory/AssetFactory.sol
contract AssetFactory is AccessControl {
    address public identityRegistry;
    address public compliance;
    
    event AssetDeployed(address indexed token, string name, string symbol, uint256 supply);
    
    function deployAsset(
        string calldata name,     // "POOOL Villa Bali 01"
        string calldata symbol,   // "p-VB-01"
        uint256 totalSupply,      // 1000
        string calldata docsCID   // "QmX7b9f..."
    ) external onlyRole(DEFAULT_ADMIN_ROLE) returns (address) {
        POOOLPropertyToken token = new POOOLPropertyToken(
            identityRegistry, compliance, name, symbol, 0, address(0)
        );
        token.updateDocumentsCID(docsCID);
        token.mint(msg.sender, totalSupply);  // Alle Tokens initial an Treasury
        
        emit AssetDeployed(address(token), name, symbol, totalSupply);
        return address(token);
    }
}
```

---

#### 3.2.9. Wallet Custody Model & Insolvenz-Schutz

> **Kontext:** Dieses Kapitel füllt eine kritische Lücke – wie genau bekommen POOOL-Nutzer eine Blockchain-Wallet, und was passiert bei POOOL-Insolvenz?

**Phase 1: Platform-Custodied Wallets (GCP KMS)**

```
┌──────────────────────────────────────────────────────────────────┐
│  WALLET-GENERIERUNG PRO USER                                     │
│                                                                  │
│  1. User wird KYC-verifiziert (Didit.me)                        │
│     └─→ is_kyc_verified = true in PostgreSQL                    │
│                                                                  │
│  2. Identity-Worker (Tokio Task) bemerkt neuen KYC-User:        │
│     a) Generiert secp256k1 Key via GCP KMS:                     │
│        gcloud kms keys create user-{user_id}                     │
│          --keyring=poool-user-wallets                            │
│          --location=global                                       │
│          --purpose=asymmetric-signing                            │
│          --default-algorithm=ec-sign-secp256k1-sha256            │
│                                                                  │
│     b) Leitet Public Key ab → Ethereum Address (0x...)           │
│                                                                  │
│     c) Speichert in PostgreSQL:                                  │
│        INSERT INTO user_wallets (user_id, wallet_address,        │
│          kms_key_id) VALUES ($1, $2, $3)                        │
│                                                                  │
│     d) Registriert Identity On-Chain:                            │
│        IdentityRegistry.registerIdentity(wallet, onchainId)      │
│                                                                  │
│  3. User hat jetzt eine Blockchain-Wallet                        │
│     → Private Key verlässt GCP HSM NIEMALS                      │
│     → User sieht nur: "🔒 Ihr Wallet ist gesichert"             │
└──────────────────────────────────────────────────────────────────┘
```

**DB-Tabelle `user_wallets`:**

```sql
-- Migration: 057_user_wallets.sql
CREATE TABLE user_wallets (
    user_id         UUID PRIMARY KEY REFERENCES users(id),
    wallet_address  VARCHAR(42) NOT NULL UNIQUE,
    kms_key_id      VARCHAR(255) NOT NULL,     -- GCP KMS Key Resource Name
    kms_key_ring    VARCHAR(255) NOT NULL,     -- GCP KMS Key Ring
    wallet_type     VARCHAR(20) NOT NULL DEFAULT 'custodial'
                    CHECK (wallet_type IN ('custodial', 'mpc', 'self_custody')),
    on_chain_identity_address VARCHAR(42),     -- ONCHAINID Contract Adresse
    is_frozen       BOOLEAN NOT NULL DEFAULT false,
    frozen_reason   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_wallets_address ON user_wallets(wallet_address);
```

**Kosten pro User-Wallet (GCP KMS):**
- Key-Erstellung: ~$0.03/Monat pro Key
- Signatur: ~$0.000003 pro Signing-Operation
- Bei 1.000 Usern: ~$30/Monat + vernachlässigbare Signatur-Kosten

**Phase 2 (Zukunft): MPC / Embedded Wallets**

Wenn POOOL das Custodial-Risiko reduzieren möchte (weniger regulatorische Anforderungen als Verwahrer), kann in Phase 2 ein MPC-Wallet-Provider integriert werden:

| Provider | Beschreibung | Integration |
|---|---|---|
| **Turnkey** | Infrastructure-as-Code für MPC Wallets | REST API, Rust SDK |
| **Privy** | Embedded Wallets (Social Login = Wallet) | SDK, einfachste UX |
| **Web3Auth** | Non-Custodial mit Social Recovery | SDK, weit verbreitet |

> **Empfehlung:** Phase 1 starten mit GCP KMS (simpelste Integration, volle Kontrolle). Phase 2 evaluieren sobald regulatorische Fragen geklärt sind.

**Insolvenz-Schutz (Bankruptcy Remoteness):**

Was passiert wenn POOOL als Unternehmen aufhört zu existieren?

```
┌──────────────────────────────────────────────────────────────────┐
│  INSOLVENZ-FALLBACK-KETTE (3 Stufen)                             │
│                                                                  │
│  Stufe 1: Escrow Trust (Vorbereitet)                             │
│  ├── POOOL hinterlegt bei einer Treuhänder-Firma:               │
│  │   "Wenn POOOL X Monate inaktiv, erhaltet ihr Zugang           │
│  │    zum GCP KMS Key Ring 'poool-user-wallets'"                 │
│  ├── Treuhänder exportiert alle Private Keys                     │
│  ├── Treuhänder sendet jedem User seine Seed-Phrase per          │
│  │   verifizierter E-Mail / Einschreiben                         │
│  └── User importiert Key in MetaMask → volle Kontrolle           │
│                                                                  │
│  Stufe 2: ERC-3643 forceTransfer (On-Chain)                      │
│  ├── Falls GCP KMS nicht mehr erreichbar:                        │
│  ├── Der SPV-Verwalter (oder Insolvenzverwalter) hat             │
│  │   Zugang zur Owner Multisig (Gnosis Safe)                     │
│  ├── forceTransfer() aller Tokens an neue Wallets                │
│  │   die von den Investoren selbst erstellt werden               │
│  └── Möglich dank ERC-3643 Agent-Rechte                          │
│                                                                  │
│  Stufe 3: Real-World Fallback (Letzte Instanz)                   │
│  ├── Blockchain = rechtsgültiger Beweis der Anteile              │
│  ├── Insolvenzverwalter nutzt letzten DB-Snapshot                │
│  │   + On-Chain Ledger zur Verifikation                          │
│  ├── Miet-Dividenden werden per SEPA-Überweisung                │
│  │   direkt vom SPV an die verifizierten Eigentümer gezahlt     │
│  └── SPV existiert weiter, unabhängig von POOOL                 │
└──────────────────────────────────────────────────────────────────┘
```

**Vertragliche Verpflichtung (Tag 1):**
- [ ] Escrow-Vereinbarung mit Treuhänder-Firma unterzeichnen
- [ ] KMS Key Ring Access-Policy dokumentieren
- [ ] Gnosis Safe Signer-Liste bei Notar hinterlegen
- [ ] Insolvenz-Verfahren im SPV Operating Agreement definieren

---

#### 3.2.10. Dividenden-Mechanik (Miet-Einnahmenverteilung)

> **Kontext:** Tokenisierte Immobilien generieren laufende Mieteinnahmen. Diese müssen proportional an alle Token-Holder verteilt werden. Dieses Kapitel definiert zwei Methoden – Phase 1 (Off-Chain Fiat) und Phase 2 (On-Chain USDC).

**Methode 1: Off-Chain Fiat Payouts (POOOL Standard – Phase 1)**

```
┌──────────────────────────────────────────────────────────────────┐
│  MONATLICHE DIVIDENDEN-AUSSCHÜTTUNG                              │
│                                                                  │
│  1. SPV erhält Miete (z.B. €5.000/Monat)                        │
│     └─→ Fließt auf SPV-Bankkonto                                │
│                                                                  │
│  2. Admin im Dashboard: "Dividende berechnen"                    │
│     └─→ Backend liest On-Chain Snapshot:                         │
│         SELECT uw.user_id, ob.balance, ob.balance::FLOAT         │
│           / a.tokens_total * 500000 AS dividend_cents            │
│         FROM onchain_balances ob                                 │
│         JOIN user_wallets uw ON ob.user_id = uw.user_id          │
│         JOIN assets a ON ob.asset_id = a.id                      │
│         WHERE ob.asset_id = $1 AND ob.balance > 0                │
│                                                                  │
│  3. Backend berechnet pro User:                                  │
│     User A: 100 von 1000 Tokens = 10% → €500,00                │
│     User B: 50 von 1000 Tokens = 5% → €250,00                  │
│     User C: 850 von 1000 Tokens = 85% → €4.250,00              │
│                                                                  │
│  4. Admin überprüft + genehmigt die Berechnung                  │
│                                                                  │
│  5. Backend credited jeden User:                                 │
│     UPDATE wallets SET balance_cents = balance_cents + $dividend │
│     WHERE user_id = $user_id                                     │
│                                                                  │
│  6. User sieht Dividende in Wallet → kann per SEPA auszahlen    │
└──────────────────────────────────────────────────────────────────┘
```

**Anti-Dividend-Sniping:**

Problem: Ein Trader könnte kurz VOR dem Snapshot kaufen und kurz DANACH verkaufen, um Dividende zu kassieren ohne langfristig investiert zu sein.

| Maßnahme | Beschreibung |
|---|---|
| **Snapshot-Zeitpunkt geheim** | Admin wählt den Snapshot-Block zufällig im Monat (nicht immer am 1.) |
| **Holding-Period-Requirement** | Optional: Nur Token-Holder die >7 Tage halten, bekommen Dividende |
| **Ex-Dividend-Date** | Wie bei Aktien: Ab dem Ex-Tag wird die Dividende beim Verkäufer verbucht |

**Notwendige DB-Tabelle:**

```sql
-- Migration: 060_dividend_distributions.sql
CREATE TABLE dividend_distributions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    total_amount_cents BIGINT NOT NULL CHECK (total_amount_cents > 0),
    snapshot_block  BIGINT,                    -- On-Chain Block für den Snapshot
    snapshot_at     TIMESTAMPTZ NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'calculated', 'approved', 'distributed')),
    distributed_at  TIMESTAMPTZ,
    approved_by     UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE dividend_payouts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    distribution_id UUID NOT NULL REFERENCES dividend_distributions(id),
    user_id         UUID NOT NULL REFERENCES users(id),
    tokens_held     INTEGER NOT NULL,
    payout_cents    BIGINT NOT NULL CHECK (payout_cents > 0),
    percentage_bps  INTEGER NOT NULL,          -- Anteil in Basis Points
    wallet_credited BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_div_payouts_dist ON dividend_payouts(distribution_id);
CREATE INDEX idx_div_payouts_user ON dividend_payouts(user_id);
```

**Methode 2: On-Chain USDC (Phase 2 – Zukunft)**

```
1. Backend berechnet Dividenden-Liste
2. Generiert Merkle Root aller Payouts
3. POOOL kauft USDC (Stablecoin) und sendet an DividendDistributor Contract
4. User klickt "Dividende einfordern" → On-Chain TX
5. Smart Contract verifiziert Merkle Proof → sendet USDC an User-Wallet
```

> **Phase-2-Entscheidung:** Erst implementieren wenn regulatorische Fragen zur Stablecoin-Verteilung geklärt sind. Phase 1 (Fiat-Credits) ist für den Launch ausreichend.

---

#### 3.2.11. Blockchain Event-Indexer (On-Chain → PostgreSQL Sync)

> **Kontext:** Der Blockchain Event-Indexer ist ein Tokio-Background-Task der 24/7 die Base L2 Blockchain beobachtet und relevante Events in PostgreSQL spiegelt. Ohne Indexer wäre das Frontend extrem langsam (Blockchain-Reads dauern 100-500ms vs. 1ms für PostgreSQL).

**Indexierte Events:**

| Event | Smart Contract | PostgreSQL-Aktion |
|---|---|---|
| `Transfer(from, to, amount)` | POOOLPropertyToken | `onchain_balances` aktualisieren |
| `BatchSettled(batchId, merkleRoot, count)` | POOOLPropertyToken | `settlement_batches.status = 'confirmed'` |
| `Frozen(wallet)` | POOOLPropertyToken | `user_wallets.is_frozen = true` |
| `Unfrozen(wallet)` | POOOLPropertyToken | `user_wallets.is_frozen = false` |
| `IdentityRegistered(wallet, identity)` | IdentityRegistry | `user_wallets.on_chain_identity_address` setzen |
| `IdentityRemoved(wallet)` | IdentityRegistry | Alert generieren + User sperren |
| `ComplianceModuleAdded(module)` | ModularCompliance | Audit-Log |

**PostgreSQL-Tabelle `onchain_balances` (Cache der Blockchain-Wahrheit):**

```sql
-- Migration: 058_onchain_balances.sql
-- Cached On-Chain Token-Balances für schnelle Frontend-Reads
CREATE TABLE onchain_balances (
    user_id         UUID NOT NULL REFERENCES users(id),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    balance         BIGINT NOT NULL DEFAULT 0 CHECK (balance >= 0),
    last_synced_block BIGINT NOT NULL,
    last_synced_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, asset_id)
);

-- Für Dividenden-Snapshot und Portfolio-Ansicht
CREATE INDEX idx_onchain_asset ON onchain_balances(asset_id) 
    WHERE balance > 0;
```

**Rust Event-Indexer Implementation:**

```rust
// backend/src/worker/event_indexer.rs

use alloy::providers::ProviderBuilder;
use alloy::rpc::types::Filter;
use alloy::primitives::Address;
use sqlx::PgPool;

/// Der Indexer läuft als Tokio-Task im Hintergrund.
/// Beim Server-Start: Liest den letzten synchronisierten Block aus der DB.
/// Dann: Pollt alle 5 Sekunden nach neuen Events.
pub async fn run_event_indexer(db: &PgPool) -> Result<(), AppError> {
    let rpc_url = std::env::var("BASE_L2_RPC_URL")?;
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);
    
    // Letzten synchronisierten Block aus DB lesen
    let last_block = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(last_synced_block), 0) FROM onchain_balances"
    ).fetch_one(db).await?;
    
    let mut from_block = last_block as u64 + 1;
    
    loop {
        let current_block = provider.get_block_number().await?;
        
        // Sicherheitsabstand: 3 Blöcke hinter HEAD bleiben (Re-org Schutz)
        let safe_block = current_block.saturating_sub(3);
        
        if from_block > safe_block {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        
        // Transfer-Events für alle POOOL Token-Contracts abfragen
        let contracts: Vec<Address> = get_all_token_contracts(db).await?;
        
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(safe_block)
            .address(contracts)
            .event("Transfer(address,address,uint256)");
        
        let logs = provider.get_logs(&filter).await?;
        
        for log in logs {
            process_transfer_event(db, &log).await?;
        }
        
        from_block = safe_block + 1;
        
        tracing::debug!(
            "Indexed blocks {} to {}, {} events processed",
            from_block, safe_block, logs.len()
        );
        
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

**Re-org Protection:**

| Strategie | Beschreibung |
|---|---|
| **Confirmation Depth = 3** | Indexer bleibt 3 Blöcke hinter der Chain-Spitze (99.99% sicher auf Base L2) |
| **Idempotente Updates** | `ON CONFLICT (user_id, asset_id) DO UPDATE` – gleicher Event zweimal verarbeiten ist sicher |
| **Block-Tracking** | `last_synced_block` wird pro `onchain_balances`-Eintrag gespeichert |
| **Startup-Replay** | Beim Server-Neustart: Alles ab `MAX(last_synced_block) + 1` neu indexieren |

**Settlement-Batch-Tracking (Audit-Log):**

```sql
-- Migration: 059_settlement_batches.sql
CREATE TABLE settlement_batches (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id            UUID NOT NULL REFERENCES assets(id),
    batch_number        INTEGER NOT NULL,
    merkle_root         VARCHAR(66) NOT NULL,
    tx_hash             VARCHAR(66),
    trade_count         INTEGER NOT NULL,
    net_transfer_count  INTEGER NOT NULL,
    gas_used            BIGINT,
    gas_cost_wei        BIGINT,
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'submitted', 'confirmed', 'failed')),
    error_message       TEXT,
    retry_count         INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at        TIMESTAMPTZ,
    UNIQUE (asset_id, batch_number)
);

CREATE INDEX idx_settlement_pending ON settlement_batches(status) 
    WHERE status IN ('pending', 'submitted');
```

**Failed Settlement Retry-Logik:**

```
1. Settlement TX wird gesendet → Status: 'submitted'
2. TX-Receipt prüfen:
   a) Erfolg → Status: 'confirmed', trade_history.on_chain_status = 'confirmed'
   b) Reverted → Status: 'failed', retry_count++
3. Bei Failure:
   → retry_count < 3: Automatischer Retry nach 60 Sekunden
   → retry_count >= 3: STOP + Sentry Critical Alert + Admin-Notification
   → Trades bleiben 'pending', werden im nächsten Batch-Run erneut aggregiert
4. Circuit-Breaker:
   → Wenn 3 aufeinanderfolgende Batches für dasselbe Asset feilen →
     Trading für dieses Asset automatisch pausieren (Kill-Switch)
```

### 3.3. Database & DevOps Engineer (Daten, Backups & Infrastruktur)
*Der Bewahrer der Wahrheit. Wenn Server brennen, darf nicht ein Cent fehlen.*

> **Verantwortungsbereich:** Diese Person provisioniert und betreibt die gesamte Daten-Infrastruktur: 2 Cloud SQL Instanzen (Core + Community), 1 Redis Memorystore, Backup-Strategien, Monitoring-Dashboards, Alert-Policies und den Disaster-Recovery-Plan. Sie schreibt die SQL-Migrationen für alle neuen Marketplace-Tabellen und stellt sicher, dass kein einziger Cent durch Infrastruktur-Fehler verloren geht.

---

#### 3.3.1. Cloud SQL Provisionierung (2-Datenbank-Architektur)

Die Architektur-Entscheidung aus Abschnitt 1.7 (physische Trennung Core-DB + Community-DB) muss hier konkret umgesetzt werden.

**Core-DB (Finanz-Priorität):**

```bash
# Provisionierung der Core-DB (Cloud SQL PostgreSQL 16)
gcloud sql instances create poool-core-db \
  --database-version=POSTGRES_16 \
  --tier=db-f1-micro \                     # Phase 1 (Launch: 0-50 User)
  --region=asia-southeast1 \               # Jakarta (Nähe zu indonesischen Nutzern)
  --availability-type=zonal \              # Phase 1: kein HA (Kosten sparen)
  --storage-type=SSD \
  --storage-size=10GB \
  --storage-auto-increase \                # Automatisch vergrößern wenn >80%
  --backup \                               # Tägliche automatische Backups
  --enable-point-in-time-recovery \        # 🔴 KRITISCH: PITR ab Tag 1!
  --retained-backups-count=14 \            # 14 Tage Backup-Retention
  --retained-transaction-log-days=7 \      # 7 Tage WAL-Logs für PITR
  --maintenance-window-day=SUN \
  --maintenance-window-hour=3 \            # Wartung: Sonntag 03:00 UTC
  --database-flags=\
    max_connections=100,\
    log_min_duration_statement=500,\        # Slow-Query-Log für >500ms
    shared_preload_libraries=timescaledb    # TimescaleDB Extension!
```

**Community-DB (Social-Media, niedrigere Priorität):**

```bash
gcloud sql instances create poool-community-db \
  --database-version=POSTGRES_16 \
  --tier=db-f1-micro \
  --region=asia-southeast1 \
  --availability-type=zonal \
  --storage-type=SSD \
  --storage-size=10GB \
  --storage-auto-increase \
  --backup \
  --enable-point-in-time-recovery \
  --retained-backups-count=7 \             # Kürzere Retention (weniger kritisch)
  --retained-transaction-log-days=3
```

**Skalierungs-Stufen (Referenz für Trigger-basiertes Upgrade):**

| Phase | Nutzer | Core-DB Tier | Community-DB Tier | HA? | Kosten |
|---|---|---|---|---|---|
| **Launch** | 0-50 | `db-f1-micro` (0.6GB) | `db-f1-micro` | ❌ | ~$20/Mo |
| **Growth** | 50-100 | `db-g1-small` (1.7GB) | `db-f1-micro` | ❌ | ~$35/Mo |
| **Scale** | 100-500 | `db-custom-2-4096` (2vCPU, 4GB) | `db-g1-small` | ✅ Core | ~$180/Mo |
| **Mature** | 500-1000 | `db-custom-4-8192` (4vCPU, 8GB) | `db-custom-2-4096` | ✅ Both | ~$400/Mo |

---

#### 3.3.2. Point-In-Time Recovery (PITR) & Backup-Strategie

**Warum PITR das absolute Minimum ist:** Cloud SQL mit aktiviertem PITR archiviert *jede einzelne Datenbankänderung* (Write-Ahead-Logs) in Echtzeit. Wenn um 14:32:17 ein Bug alle Wallets auf 0 setzt, kann die DB sekundengenau auf 14:32:16 zurückgespult werden – kein Cent geht verloren.

**Backup-Hierarchie:**

```
┌─────────────────────────────────────────────────────────────┐
│  BACKUP-STRATEGIE (3 Schichten)                              │
│                                                             │
│  Schicht 1: PITR (Echtzeit)                                 │
│  ├── WAL-Logs werden kontinuierlich geschrieben              │
│  ├── Granularität: sekundengenau                            │
│  ├── Retention: 7 Tage (Core-DB)                            │
│  └── Kosten: In Cloud SQL Preis inkludiert                  │
│                                                             │
│  Schicht 2: Tägliche Snapshots (Automatisch)                │
│  ├── Cloud SQL erstellt täglich um 03:00 UTC einen Snapshot  │
│  ├── Retention: 14 Tage (Core-DB), 7 Tage (Community-DB)   │
│  └── Kosten: In Cloud SQL Preis inkludiert                  │
│                                                             │
│  Schicht 3: Wöchentliche Cross-Region Snapshots             │
│  ├── Sonntags: pg_dump → Export in GCS Bucket               │
│  │   → Bucket in europe-west3 (Katastrophenschutz)          │
│  ├── Retention: 90 Tage                                     │
│  └── Kosten: ~$5-10/Mo (GCS Storage)                        │
└─────────────────────────────────────────────────────────────┘
```

**Cross-Region Backup-Job (als Cron auf Cloud Scheduler):**

```bash
#!/bin/bash
# weekly_cross_region_backup.sh
# Wird jeden Sonntag 04:00 UTC von Cloud Scheduler getriggert

DATE=$(date +%Y-%m-%d)
BUCKET="gs://poool-backups-eu-west3"

# Core-DB Export
gcloud sql export sql poool-core-db \
  "${BUCKET}/core-db/poool-core-${DATE}.sql.gz" \
  --database=poool \
  --offload  # Exportiert ohne die Primary zu belasten

# Community-DB Export
gcloud sql export sql poool-community-db \
  "${BUCKET}/community-db/poool-community-${DATE}.sql.gz" \
  --database=poool_community \
  --offload

# Alte Backups aufräumen (>90 Tage)
gsutil -m rm "gs://poool-backups-eu-west3/core-db/poool-core-$(date -d '-90 days' +%Y-%m-%d).sql.gz" 2>/dev/null
```

**Disaster-Recovery Runbook (Step-by-Step):**

| Szenario | RTO | RPO | Aktion |
|---|---|---|---|
| **Bug setzt Balances falsch** | <5 Min | 0 Sek | PITR: `gcloud sql instances clone poool-core-db poool-core-restored --point-in-time 2026-03-20T14:32:16Z` |
| **Core-DB Primary Ausfall** | <1 Min (mit HA) | 0 Sek | Cloud SQL HA: Automatisches Failover auf Standby. Kein manueller Eingriff |
| **Core-DB komplett korrupt** | <30 Min | <24h | Täglichen Snapshot restoren: `gcloud sql instances restore-backup poool-core-db --backup-id=...` |
| **GCP Region-Ausfall** | <4h | <7 Tage | Cross-Region SQL-Dump aus GCS laden und auf neue Instanz importieren |
| **Redis Memorystore Crash** | <2 Min | Kein Datenverlust | Redis ist nur Cache – Orderbook wird aus `market_orders` Tabelle in Postgres rebuildet |

---

#### 3.3.3. Read-Replicas & Dual-Pool Routing im Rust-Backend

**Read-Replicas reduzieren die Last auf der Primary:** Portfolio-Ansichten, Chart-Queries, Admin-Dashboards und alle `SELECT`-only-Queries laufen auf der Replica. Nur Writes (Trades, Deposits, Orders) gehen an die Primary.

**Provisionierung:**

```bash
# Read-Replica für Core-DB
gcloud sql instances create poool-core-db-replica \
  --master-instance-name=poool-core-db \
  --tier=db-f1-micro \                    # Kann kleiner sein als Primary
  --region=asia-southeast1
```

**Implementierung im Rust-Backend (`db.rs` erweitert):**

```rust
// db.rs – Erweitert um Dual-Pool (Primary + Replica) + Community-Pool

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Alle Datenbankpools der Applikation
pub struct DatabasePools {
    /// Core-DB Primary: Für alle Writes (Trades, Wallets, Orders)
    pub core_primary: PgPool,
    /// Core-DB Replica: Für alle Reads (Portfolios, Charts, Dashboards)
    pub core_replica: PgPool,
    /// Community-DB: Posts, Comments, Follows (komplett getrennt)
    pub community: PgPool,
}

impl DatabasePools {
    pub async fn from_env() -> Result<Self, sqlx::Error> {
        let core_primary_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let core_replica_url = std::env::var("DATABASE_REPLICA_URL")
            .unwrap_or_else(|_| core_primary_url.clone()); // Fallback: Primary
        let community_url = std::env::var("COMMUNITY_DATABASE_URL")
            .unwrap_or_else(|_| core_primary_url.clone()); // Fallback: Shared

        let core_primary = PgPoolOptions::new()
            .max_connections(30)              // Finanzkritisch, höchste Priorität
            .min_connections(5)               // Warm gehalten
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(120))
            .connect(&core_primary_url).await?;

        let core_replica = PgPoolOptions::new()
            .max_connections(20)              // Nur Reads, weniger Connections nötig
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(10))  // Replicas dürfen länger warten
            .connect(&core_replica_url).await?;

        let community = PgPoolOptions::new()
            .max_connections(15)              // Niedrigere Priorität
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&community_url).await?;

        Ok(Self { core_primary, core_replica, community })
    }
}
```

**Routing-Konvention im Code:**

```rust
// WRITE-Operationen → core_primary
let trade = settle_trade(&pools.core_primary, &ask, &bid, price, qty).await?;

// READ-Operationen → core_replica
let portfolio = get_user_portfolio(&pools.core_replica, user_id).await?;
let candles = get_candle_data(&pools.core_replica, asset_id, interval).await?;
let admin_stats = get_marketplace_stats(&pools.core_replica).await?;

// Community-Operationen → community
let posts = get_feed(&pools.community, user_id).await?;
let followers = get_followers(&pools.community, user_id).await?;
```

> **Wichtig: Replica-Lag beachten!** Read-Replicas können bis zu ~100ms hinter der Primary sein. Das bedeutet: Nach einem Trade darf das Frontend NICHT sofort die Replica für das aktualisierte Portfolio abfragen – der Trade wäre dort ggf. noch nicht sichtbar. Lösung: Nach einem Write wird für 2 Sekunden die Primary als Lese-Quelle verwendet (Read-Your-Writes Pattern), danach die Replica.

```rust
/// Read-Your-Writes: Nach einem Write für kurze Zeit von Primary lesen
pub async fn get_pool_for_read(
    pools: &DatabasePools,
    user_id: Uuid,
    redis: &RedisPool,
) -> &PgPool {
    let key = format!("recent_write:{}", user_id);
    if redis.exists(&key).await.unwrap_or(false) {
        &pools.core_primary  // Kürzlich geschrieben → Primary lesen
    } else {
        &pools.core_replica  // Normaler Read → Replica
    }
}

/// Nach jedem Write setzen:
pub async fn mark_recent_write(redis: &RedisPool, user_id: Uuid) {
    let key = format!("recent_write:{}", user_id);
    let _ = redis.set_ex(&key, "1", 2).await;  // 2 Sekunden TTL
}
```

---

#### 3.3.4. Redis Memorystore (High Availability)

**Warum Managed Redis (Memorystore) statt Self-Hosted:**
- Automatisches Failover (Standard-Tier): Wenn die Primary Redis-Node ausfällt, übernimmt die Replica innerhalb von Sekunden
- Kein Patching, kein Monitoring der Redis-Instanz selbst
- Automatische Verschlüsselung in-transit und at-rest

**Provisionierung:**

```bash
# Phase 1 (Launch): Basic-Tier (kein Failover, günstig)
gcloud redis instances create poool-redis \
  --size=1 \                           # 1GB RAM
  --region=asia-southeast1 \
  --tier=basic \                       # Basic = kein automatisches Failover
  --redis-version=redis_7_2

# Phase 2 (Growth, ab ~100 User): Upgrade auf Standard-Tier
gcloud redis instances update poool-redis \
  --tier=standard                      # Standard = automatisches Failover!
```

**Redis-Daten und ihre Kritikalität:**

| Redis-Key-Pattern | Inhalt | Verlierbar? | Rebuild-Strategie |
|---|---|---|---|
| `asks:asset:{id}` | Live-Orderbook (Sells) | ✅ Ja | Rebuild aus `market_orders` WHERE status='open' AND side='sell' |
| `bids:asset:{id}` | Live-Orderbook (Buys) | ✅ Ja | Rebuild aus `market_orders` WHERE status='open' AND side='buy' |
| `trading_session:{user_id}` | 2FA Trading-Session | ✅ Ja | Nutzer muss sich erneut per 2FA authentifizieren |
| `idempotency:{key}` | Doppel-Submit-Schutz | ⚠️ Ungern | 24h TTL, nach Redis-Restart besteht kurzes Doppel-Risiko |
| `rl:orders:user:{id}` | Rate-Limiting Counter | ✅ Ja | Wird automatisch neu aufgebaut |
| `lock:order:{id}` | Cancel-Lock (5s TTL) | ✅ Ja | 5s TTL, löst sich von selbst |
| `recent_write:{user_id}` | Read-Your-Writes Flag | ✅ Ja | Fallback: Primary lesen (sicher) |

**Orderbook-Rebuild (nach Redis-Crash oder Memorystore-Failover):**

```rust
/// Rebuildet das gesamte Orderbook aus PostgreSQL in Redis
/// Wird automatisch beim Server-Start aufgerufen (Startup-Check)
pub async fn rebuild_orderbook_from_postgres(
    pool: &PgPool,
    redis: &RedisPool,
) -> Result<u32, AppError> {
    // 1. Alle offenen Orders aus PostgreSQL laden
    let open_orders = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders 
         WHERE status IN ('open', 'partially_filled')
         ORDER BY created_at ASC"
    ).fetch_all(pool).await?;

    let mut count = 0u32;

    // 2. Jede Order in das richtige Redis Sorted Set einfügen
    for order in &open_orders {
        let key = match order.side.as_str() {
            "sell" => format!("asks:asset:{}", order.asset_id),
            "buy"  => format!("bids:asset:{}", order.asset_id),
            _      => continue,
        };
        let score = order.price_cents as f64;
        let member = order.redis_member();

        redis.zadd(&key, &member, score).await?;
        count += 1;
    }

    tracing::info!("Rebuilt orderbook from PostgreSQL: {} orders loaded into Redis", count);
    Ok(count)
}
```

---

#### 3.3.5. TimescaleDB für Candlestick-Charts

**Das Problem:** Die `trade_history`-Tabelle wächst mit ~1.000 Trades/Tag auf ~365.000 Zeilen/Jahr. Candlestick-Queries (`GROUP BY date_trunc('hour', ...)`) werden bei einem normalen PostgreSQL `SELECT` zunehmend langsamer, weil die gesamte Tabelle gescannt wird.

**Die Lösung: TimescaleDB Hypertable + Continuous Aggregates**

TimescaleDB ist eine PostgreSQL-Extension (kein separater Server!), die Zeitreihen-Daten automatisch in Partitionen ("Chunks") aufteilt und vorberechnete Aggregierungen (Continuous Aggregates) erstellt.

**Aktivierung auf Cloud SQL:**

```sql
-- TimescaleDB Extension aktivieren (einmalig)
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- trade_history in eine Hypertable konvertieren
-- (Partitioniert automatisch nach executed_at in 7-Tage-Chunks)
SELECT create_hypertable('trade_history', 'executed_at',
    chunk_time_interval => INTERVAL '7 days',
    migrate_data => true  -- Bestehende Daten werden automatisch migriert
);
```

**Continuous Aggregates (vorberechnete Candlestick-Daten):**

```sql
-- 1-Stunden Candlesticks (automatisch aktualisiert)
CREATE MATERIALIZED VIEW candles_1h
WITH (timescaledb.continuous) AS
SELECT
    asset_id,
    time_bucket('1 hour', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 hour', executed_at)
WITH NO DATA;

-- Automatische Aktualisierung: Alle 5 Minuten
SELECT add_continuous_aggregate_policy('candles_1h',
    start_offset => INTERVAL '3 hours',   -- Blickt 3h zurück (falls Daten nachgereicht)
    end_offset   => INTERVAL '1 minute',  -- Bis 1 Min vor jetzt
    schedule_interval => INTERVAL '5 minutes'
);

-- 1-Tag Candlesticks
CREATE MATERIALIZED VIEW candles_1d
WITH (timescaledb.continuous) AS
SELECT
    asset_id,
    time_bucket('1 day', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 day', executed_at)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_1d',
    start_offset => INTERVAL '3 days',
    end_offset   => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 1-Wochen Candlesticks
CREATE MATERIALIZED VIEW candles_1w
WITH (timescaledb.continuous) AS
SELECT
    asset_id,
    time_bucket('1 week', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 week', executed_at)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_1w',
    start_offset => INTERVAL '4 weeks',
    end_offset   => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day'
);
```

**Query-Performance-Vergleich:**

| Query | Ohne TimescaleDB | Mit Continuous Aggregates |
|---|---|---|
| 7-Tage Candlestick (1h-Intervall) | ~50-200ms (Scan 7.000 Rows) | **<5ms** (168 vorberechnete Rows) |
| 30-Tage Candlestick (1d-Intervall) | ~100-500ms (Scan 30.000 Rows) | **<2ms** (30 vorberechnete Rows) |
| 1-Jahr Candlestick (1w-Intervall) | ~500-2000ms (Scan 365.000 Rows) | **<2ms** (52 vorberechnete Rows) |

---

#### 3.3.6. Marketplace SQL-Migrationen

Alle neuen Tabellen werden als sauber versionierte Migrationen angelegt, die sich nahtlos in die bestehende `database/001_*` bis `database/049_*` Reihe einfügen.

**Migration 050: Marketplace Orders**

```sql
-- database/050_marketplace_orders.sql

-- Marketplace Orders (das Orderbook in PostgreSQL)
CREATE TABLE market_orders (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id),
    asset_id         UUID NOT NULL REFERENCES assets(id),
    side             VARCHAR(10) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type       VARCHAR(15) NOT NULL DEFAULT 'limit' 
                     CHECK (order_type IN ('market', 'limit')),
    price_cents      BIGINT NOT NULL CHECK (price_cents > 0),
    quantity         INTEGER NOT NULL CHECK (quantity > 0),
    quantity_filled  INTEGER NOT NULL DEFAULT 0 CHECK (quantity_filled >= 0),
    status           VARCHAR(20) NOT NULL DEFAULT 'open'
                     CHECK (status IN ('open', 'partially_filled', 'filled', 
                                       'cancelled', 'expired', 'pending_review', 
                                       'rejected')),
    idempotency_key  VARCHAR(64),
    expires_at       TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '90 days',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CHECK (quantity_filled <= quantity),
    UNIQUE (idempotency_key)
);

-- Indexes für häufige Queries
CREATE INDEX idx_market_orders_asset_status 
    ON market_orders(asset_id, status) WHERE status IN ('open', 'partially_filled');
CREATE INDEX idx_market_orders_user 
    ON market_orders(user_id, status);
CREATE INDEX idx_market_orders_expires 
    ON market_orders(expires_at) WHERE status IN ('open', 'partially_filled');

-- Balance-Hold Erweiterungen für bestehende Tabellen
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS 
    held_balance_cents BIGINT NOT NULL DEFAULT 0 CHECK (held_balance_cents >= 0);
ALTER TABLE investments ADD COLUMN IF NOT EXISTS 
    held_tokens INTEGER NOT NULL DEFAULT 0 CHECK (held_tokens >= 0);
```

**Migration 051: Trade History**

```sql
-- database/051_trade_history.sql

CREATE TABLE trade_history (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id            UUID NOT NULL REFERENCES assets(id),
    buyer_user_id       UUID NOT NULL REFERENCES users(id),
    seller_user_id      UUID NOT NULL REFERENCES users(id),
    price_cents         BIGINT NOT NULL CHECK (price_cents > 0),
    quantity            INTEGER NOT NULL CHECK (quantity > 0),
    total_cents         BIGINT GENERATED ALWAYS AS (price_cents * quantity) STORED,
    fee_cents           BIGINT NOT NULL DEFAULT 0 CHECK (fee_cents >= 0),
    market_order_ask_id UUID REFERENCES market_orders(id),
    market_order_bid_id UUID REFERENCES market_orders(id),
    on_chain_status     VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (on_chain_status IN ('pending', 'submitted', 
                                                    'confirmed', 'failed')),
    on_chain_tx_hash    VARCHAR(66),        -- 0x + 64 hex chars
    on_chain_batch_id   INTEGER,
    executed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primärer Index für Chart-Queries
CREATE INDEX idx_trade_history_asset_time 
    ON trade_history(asset_id, executed_at DESC);
-- Index für Settlement-Worker
CREATE INDEX idx_trade_history_onchain_pending 
    ON trade_history(on_chain_status) WHERE on_chain_status = 'pending';

-- TimescaleDB Hypertable (wenn Extension aktiviert)
-- SELECT create_hypertable('trade_history', 'executed_at',
--     chunk_time_interval => INTERVAL '7 days', migrate_data => true);
```

**Migration 052: P2P Offers**

```sql
-- database/052_p2p_offers.sql

CREATE TABLE p2p_offers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id          UUID NOT NULL REFERENCES assets(id),
    maker_user_id     UUID NOT NULL REFERENCES users(id),
    taker_user_id     UUID NOT NULL REFERENCES users(id),
    side              VARCHAR(10) NOT NULL CHECK (side IN ('buy', 'sell')),
    price_cents       BIGINT NOT NULL CHECK (price_cents > 0),
    quantity          INTEGER NOT NULL CHECK (quantity > 0),
    status            VARCHAR(20) NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending', 'accepted', 'declined', 
                                        'expired', 'countered', 'cancelled')),
    parent_offer_id   UUID REFERENCES p2p_offers(id),   -- Für Counter-Offers
    message           TEXT,                               -- Optionale Nachricht
    expires_at        TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '48 hours',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CHECK (maker_user_id != taker_user_id)  -- Keine Self-Offers erlaubt
);

CREATE INDEX idx_p2p_offers_taker 
    ON p2p_offers(taker_user_id, status) WHERE status = 'pending';
CREATE INDEX idx_p2p_offers_expires 
    ON p2p_offers(expires_at) WHERE status = 'pending';
```

**Migration 053: Fee-Konfiguration**

```sql
-- database/053_fee_configuration.sql

CREATE TABLE fee_configurations (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope                VARCHAR(20) NOT NULL 
                         CHECK (scope IN ('platform', 'asset', 'developer')),
    asset_id             UUID REFERENCES assets(id),
    developer_id         UUID REFERENCES users(id),
    taker_fee_bps        INTEGER NOT NULL DEFAULT 500,    -- 500 BPS = 5.00%
    maker_fee_bps        INTEGER NOT NULL DEFAULT 0,
    withdrawal_fee_cents BIGINT NOT NULL DEFAULT 250,     -- $2.50
    p2p_fee_bps          INTEGER NOT NULL DEFAULT 500,
    listing_fee_cents    BIGINT NOT NULL DEFAULT 0,
    reason               TEXT,
    created_by           UUID REFERENCES users(id),
    is_active            BOOLEAN NOT NULL DEFAULT true,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE (scope, asset_id, developer_id)
);

CREATE TABLE fee_promotions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    scope           VARCHAR(20) NOT NULL CHECK (scope IN ('global', 'asset')),
    asset_id        UUID REFERENCES assets(id),
    taker_fee_bps   INTEGER,     -- NULL = Default beibehalten
    maker_fee_bps   INTEGER,
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ NOT NULL,
    created_by      UUID REFERENCES users(id),
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CHECK (ends_at > starts_at)
);

-- Platform Default einfügen (einmalig)
INSERT INTO fee_configurations (scope, taker_fee_bps, maker_fee_bps, reason)
VALUES ('platform', 500, 0, 'Platform Default: 5.00% Taker, 0% Maker')
ON CONFLICT DO NOTHING;
```

---

#### 3.3.7. Monitoring, Alerting & Dashboards

**Die 3 Monitoring-Säulen:**

```
┌─────────────────────────────────────────────────────────────┐
│  MONITORING-ARCHITEKTUR                                      │
│                                                             │
│  1. Sentry (bereits aktiv)                                  │
│  ├── Error-Tracking: Jeder AppError wird geloggt            │
│  ├── Performance: Request-Latenz per Route                  │
│  └── Alerts: Slack bei >5% Error-Rate                       │
│                                                             │
│  2. Google Cloud Monitoring (NEU)                           │
│  ├── Cloud SQL Metriken: CPU, Connections, Disk I/O         │
│  ├── Redis Metriken: Memory, Connections, Evictions         │
│  ├── Cloud Run Metriken: Latenz, Instance Count, CPU        │
│  └── Custom Metriken: Trade-Latenz, Reconciliation-Delta    │
│                                                             │
│  3. Custom Healthcheck-Endpoint (NEU)                       │
│  ├── GET /health → 200 wenn alles OK, 503 wenn DB/Redis    │
│  │   nicht erreichbar                                       │
│  └── Cloud Run nutzt diesen Endpoint um unhealthy           │
│      Container aus dem Traffic zu nehmen                     │
└─────────────────────────────────────────────────────────────┘
```

**Alert-Policies (Cloud Monitoring):**

| Alert | Bedingung | Severity | Benachrichtigung |
|---|---|---|---|
| **Core-DB CPU >80%** | `cloudsql.googleapis.com/database/cpu/utilization > 0.8` für >5 Min | 🟡 Warning | Slack #devops |
| **Core-DB CPU >95%** | `> 0.95` für >2 Min | 🔴 Critical | Slack + SMS an DevOps |
| **Core-DB Connections >80%** | `cloudsql.googleapis.com/database/postgresql/num_backends > 80` | 🟡 Warning | Slack #devops |
| **Redis Memory >70%** | `redis.googleapis.com/stats/memory/usage_ratio > 0.7` | 🟡 Warning | Slack #devops |
| **Redis Memory >90%** | `> 0.9` | 🔴 Critical | Slack + SMS |
| **Cloud Run Error Rate >5%** | `run.googleapis.com/request_count{response_code_class="5xx"} > 5%` | 🔴 Critical | Slack + Sentry |
| **Cloud Run Latenz P95 >500ms** | `request_latencies P95 > 500ms` für >5 Min | 🟡 Warning | Slack #devops |
| **Trade-Settlement >1h ausstehend** | Custom Metric (Cron-Check) | 🔴 Critical | Slack + SMS an CTO |
| **Reconciliation Mismatch >$1** | Custom Metric (Daily Job) | 🔴 SOFORT | SMS an CEO + CTO + CFO |
| **Wallet-Balance negativ** | Custom Metric (Cron-Check) | 🔴 SOFORT | SMS an CEO + CTO |

**Custom Health-Endpoint (Rust):**

```rust
// In main.rs oder health.rs
async fn health_check(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.pools.core_primary)
        .await
        .is_ok();

    let redis_ok = state.redis
        .ping()
        .await
        .is_ok();

    if db_ok && redis_ok {
        (StatusCode::OK, Json(json!({
            "status": "healthy",
            "db": "ok",
            "redis": "ok"
        })))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "status": "unhealthy",
            "db": if db_ok { "ok" } else { "down" },
            "redis": if redis_ok { "ok" } else { "down" }
        })))
    }
}
```

---

#### 3.3.8. Connection Pool Tuning & Skalierung

**Aktuelle Konfiguration vs. Empfohlene Konfiguration:**

| Parameter | Aktuell (`db.rs`) | Empfohlen (Core Primary) | Empfohlen (Core Replica) | Empfohlen (Community) |
|---|---|---|---|---|
| `max_connections` | 10 | **30** | **20** | **15** |
| `min_connections` | (nicht gesetzt) | **5** | **2** | **2** |
| `acquire_timeout` | 30s | **5s** | **10s** | **10s** |
| `idle_timeout` | (nicht gesetzt) | **120s** | **120s** | **300s** |
| `max_lifetime` | (nicht gesetzt) | **30 Min** | **30 Min** | **30 Min** |

**Die Formel für max_connections:**

```
max_connections pro Pool ≤ Cloud SQL max_connections / Anzahl Cloud Run Instanzen

Beispiel: Cloud SQL hat 100 max_connections, 3 Cloud Run Instanzen laufen
→ 100 / 3 = 33 Connections pro Instanz
→ Davon 20 für Core Primary, 10 für Core Replica, 3 für Admin-Tools
```

> **Wichtig: Bei Cloud Run Auto-Scaling muss die Summe aller Pools über alle Instanzen UNTER dem Cloud SQL `max_connections`-Limit bleiben.** Wenn Cloud Run auf 5 Instanzen skaliert und jede 30 Connections öffnet = 150 → Cloud SQL Default von 100 ist überschritten. Lösung: Entweder PgBouncer als zentralen Proxy verwenden ODER `max_connections` auf Cloud SQL auf 200 erhöhen.

---

#### 3.3.9. Sicherheits-Checkliste (vor Marketplace-Launch)

| # | Aufgabe | Status | Verantwortlich |
|---|---|---|---|
| 1 | PITR-Backup auf Core-DB aktiviert und getestet | ❌ | DevOps |
| 2 | PITR-Backup auf Community-DB aktiviert | ❌ | DevOps |
| 3 | Read-Replica für Core-DB provisioniert und getestet | ❌ | DevOps |
| 4 | Redis Memorystore auf Standard-Tier (Auto-Failover) | ❌ | DevOps |
| 5 | Orderbook-Rebuild aus PostgreSQL getestet (Redis-Crash-Szenario) | ❌ | DevOps + Backend |
| 6 | Cloud Monitoring Dashboards erstellt (DB, Redis, Cloud Run) | ❌ | DevOps |
| 7 | Alle 10 Alert-Policies aktiv und in Slack/SMS verifiziert | ❌ | DevOps |
| 8 | Health-Check Endpoint `/health` in Cloud Run Startup-Probe | ❌ | DevOps + Backend |
| 9 | Disaster-Recovery-Runbook dokumentiert und 1x durchgespielt | ❌ | DevOps + CTO |
| 10 | Cross-Region Backup-Job (wöchentlich) aktiv | ❌ | DevOps |
| 11 | `max_connections` auf Cloud SQL an Cloud Run Instanzen angepasst | ❌ | DevOps |
| 12 | TimescaleDB Extension aktiviert, Continuous Aggregates getestet | ❌ | DevOps + Backend |
| 13 | Alle Marketplace-Migrationen (050-053) auf Staging getestet | ❌ | DevOps + Backend |
| 14 | Reconciliation-Job (täglicher Balance-Check) implementiert | ❌ | Backend + DevOps |
| 15 | Secret Rotation Policy für DB-Passwörter dokumentiert (90 Tage) | ❌ | DevOps + CTO |

---

#### 3.3.10. Zusammenfassung: Wochenplan für den DevOps Engineer

```
Woche 1: Cloud SQL (Core + Community) provisionieren + PITR aktivieren
         Redis Memorystore aufsetzen (Basic-Tier)
         Migrationen 050-053 schreiben und auf Staging testen
         
Woche 2: Read-Replica aktivieren + Dual-Pool im Rust-Backend
         Health-Check Endpoint implementieren
         Cloud Monitoring Dashboards erstellen
         
Woche 3: TimescaleDB Extension aktivieren + Continuous Aggregates
         Alert-Policies konfigurieren (alle 10 Alerts)
         Orderbook-Rebuild-Test (Redis flushen, aus Postgres laden)
         
Woche 4: Disaster-Recovery-Runbook schreiben + 1x durchspielen
         Cross-Region Backup-Job einrichten
         Connection-Pool-Tuning unter Last testen
         Redis Standard-Tier Upgrade (wenn Budget erlaubt)
```

> **Die goldene Regel:** Kein einziger Trade darf live gehen, bevor PITR aktiviert ist. PITR ist die Versicherungspolice des gesamten Systems – ohne sie ist jeder Datenbankfehler permanent und unwiderruflich. Diese Aufgabe hat die höchste Priorität vor allen anderen Infrastruktur-Arbeiten.

### 3.4. Frontend / UI Engineer (Data Visualization & Vanilla Web)
*Der Gestalter des Vertrauens. Baut aus komplizierten Rohdaten eine lebensechte, fesselnde Markterfahrung.*

> **Verantwortungsbereich:** Diese Person baut die gesamte Trading-UI mit Vanilla HTML + CSS + JS (kein React, kein Vue, kein Bundler). Sie integriert die Candlestick-Charts, das Live-Orderbook via WebSockets, das Buy/Sell-Formular mit robuster Validierung, die P2P-Offer-Flows und die Cap Table. Sie ist dafür verantwortlich, dass die UI bei einem €2.000-Trade sofort Feedback gibt – kein Spinner, kein Einfrieren, kein Doppelklick-Risiko.

---

#### 3.4.1. Datei-Architektur: Neue Marketplace-Dateien

Das bestehende Pattern (pro Seite eine eigene `.html` + `.js` + `.css`) wird exakt beibehalten:

```
frontend/platform/
├── marketplace.html                    # Übersichtsseite: Alle handelbaren Assets
├── marketplace-trading.html            # Asset-Detailseite: Chart + Orderbook + Buy/Sell
├── static/
│   ├── css/
│   │   ├── marketplace.css             # Styles für Übersichtsseite
│   │   └── marketplace-trading.css     # Styles für Trading-Seite (Chart, Orderbook, Forms)
│   ├── js/
│   │   ├── marketplace.js              # Übersichtsseite: Asset-Liste, Ticker-Daten
│   │   ├── marketplace-trading.js      # HAUPTDATEI: Chart, Orderbook, Buy/Sell
│   │   ├── marketplace-websocket.js    # WebSocket-Client (Reconnect, Heartbeat, Event-Bus)
│   │   ├── marketplace-charts.js       # Lightweight-Charts Integration
│   │   ├── marketplace-orderbook.js    # Orderbook-Rendering + Live-Updates
│   │   ├── marketplace-p2p.js          # P2P-Offer Modals + Flow
│   │   └── marketplace-event-bus.js    # Leichtgewichtiger Event-Bus (State-Sync)
│   └── vendor/
│       └── lightweight-charts.standalone.production.mjs  # TradingView Charts (~45KB)
```

**Warum separate JS-Dateien statt einer Monolith-Datei?** Die `marketplace-trading.js` orchestriert die Sub-Module. Jedes Sub-Modul (WebSocket, Charts, Orderbook, P2P) ist unabhängig testbar und hat eine klare Verantwortung. Bei einem Bug im Orderbook-Rendering muss der Entwickler nur `marketplace-orderbook.js` öffnen – nicht eine 3.000-Zeilen-Datei durchsuchen.

```html
<!-- marketplace-trading.html – Script-Loading (am Ende des <body>) -->
<script src="/static/vendor/lightweight-charts.standalone.production.mjs" type="module"></script>
<script src="/static/js/marketplace-event-bus.js"></script>
<script src="/static/js/marketplace-websocket.js"></script>
<script src="/static/js/marketplace-charts.js"></script>
<script src="/static/js/marketplace-orderbook.js"></script>
<script src="/static/js/marketplace-p2p.js"></script>
<script src="/static/js/marketplace-trading.js"></script>
```

---

#### 3.4.2. Event-Bus (State-Sync ohne Framework)

**Das Problem:** Ohne React/Vue gibt es keinen globalen State. Wenn ein Trade ausgeführt wird, müssen gleichzeitig: das Wallet-Balance-Display aktualisiert, die Orderbook-Tabelle neu gerendert, der Chart aktualisiert und ein Toast angezeigt werden. Ohne zentralen Mechanismus muss jede Komponente die andere kennen – Spaghetti-Code.

**Die Lösung: Ein leichtgewichtiger Event-Bus auf Basis von `EventTarget` (~30 Zeilen):**

```javascript
// marketplace-event-bus.js
// Leichtgewichtiger Event-Bus für Cross-Component State-Sync
// Ersetzt React State / Vue Reactive ohne Framework-Overhead

class MarketplaceEventBus extends EventTarget {
    /**
     * Event abonnieren
     * @param {string} eventName - z.B. 'trade:executed', 'orderbook:updated'
     * @param {Function} callback - Handler-Funktion
     */
    on(eventName, callback) {
        this.addEventListener(eventName, (e) => callback(e.detail));
    }

    /**
     * Event einmalig abonnieren (auto-unsubscribe nach erstem Aufruf)
     */
    once(eventName, callback) {
        this.addEventListener(eventName, (e) => callback(e.detail), { once: true });
    }

    /**
     * Event auslösen
     * @param {string} eventName - z.B. 'trade:executed'
     * @param {Object} data - Payload
     */
    emit(eventName, data) {
        this.dispatchEvent(new CustomEvent(eventName, { detail: data }));
    }

    /**
     * Event-Listener entfernen
     */
    off(eventName, callback) {
        this.removeEventListener(eventName, callback);
    }
}

// Singleton: Eine Instanz für die gesamte Seite
window.marketBus = new MarketplaceEventBus();
```

**Event-Katalog (alle Events die der Bus transportiert):**

| Event-Name | Auslöser | Payload | Konsumenten |
|---|---|---|---|
| `ws:connected` | WebSocket-Client | `{ url }` | Status-Indicator |
| `ws:disconnected` | WebSocket-Client | `{ code, reason }` | Status-Indicator, Reconnect-UI |
| `trade:executed` | WebSocket oder API-Response | `{ price, quantity, buyer, seller, timestamp }` | Chart, Orderbook, Ticker, Toast |
| `orderbook:updated` | WebSocket | `{ bids: [...], asks: [...], spread }` | Orderbook-Tabelle |
| `ticker:updated` | WebSocket | `{ last_price, change_24h, volume_24h }` | Header-Ticker |
| `order:submitted` | Buy/Sell-Form | `{ side, price, quantity, status }` | My-Orders-Tabelle, Toast |
| `order:cancelled` | My-Orders-Tabelle | `{ order_id }` | Orderbook, Balance-Display |
| `balance:changed` | Trade oder Deposit | `{ new_balance_cents }` | Balance-Display, Buy-Form Max |
| `p2p:offer_received` | WebSocket | `{ offer }` | Notification-Badge, Toast |
| `error:api` | Jeder `fetch()`-Aufruf | `{ status, message, endpoint }` | Toast-System |

**Nutzung in den Sub-Modulen:**

```javascript
// In marketplace-charts.js:
window.marketBus.on('trade:executed', (trade) => {
    chart.update({ time: trade.timestamp, value: trade.price / 100 });
});

// In marketplace-orderbook.js:
window.marketBus.on('orderbook:updated', (data) => {
    renderBids(data.bids);
    renderAsks(data.asks);
    updateSpread(data.spread);
});

// In marketplace-trading.js (nach erfolgreichem Order-Submit):
window.marketBus.emit('order:submitted', { side: 'buy', price: 10500, quantity: 30 });
window.marketBus.emit('balance:changed', { new_balance_cents: walletBalance - totalCents });
```

---

#### 3.4.3. WebSocket-Client (Reconnect, Heartbeat, Multiplexing)

**Warum ein Custom-Client statt raw `new WebSocket()`?** In Vanilla JS gibt es keinen `useWebSocket`-Hook. Der Client muss selbst: automatisch reconnecten bei Verbindungsverlust, Heartbeats senden damit Cloud Run die Verbindung nicht wegen Inaktivität schließt, und eingehende Messages an den Event-Bus weiterleiten.

```javascript
// marketplace-websocket.js
// WebSocket-Client mit Auto-Reconnect, Heartbeat und Event-Bus-Integration

class MarketplaceWebSocket {
    constructor(assetId) {
        this.assetId = assetId;
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 10;
        this.reconnectDelay = 1000;    // Start: 1s, exponential backoff
        this.heartbeatInterval = null;
        this.isIntentionallyClosed = false;
    }

    connect() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${location.host}/ws/market/${this.assetId}`;

        this.ws = new WebSocket(url);

        this.ws.onopen = () => {
            console.log(`[WS] Connected to ${this.assetId}`);
            this.reconnectAttempts = 0;
            this.reconnectDelay = 1000;
            this.startHeartbeat();
            window.marketBus.emit('ws:connected', { url });

            // Connection-Status UI aktualisieren
            this._updateStatusIndicator('connected');
        };

        this.ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                this._handleMessage(msg);
            } catch (e) {
                console.warn('[WS] Invalid message:', event.data);
            }
        };

        this.ws.onclose = (event) => {
            console.log(`[WS] Disconnected: ${event.code} ${event.reason}`);
            this.stopHeartbeat();
            this._updateStatusIndicator('disconnected');
            window.marketBus.emit('ws:disconnected', { 
                code: event.code, reason: event.reason 
            });

            if (!this.isIntentionallyClosed) {
                this._reconnect();
            }
        };

        this.ws.onerror = (error) => {
            console.error('[WS] Error:', error);
            // onerror wird immer von onclose gefolgt – Reconnect passiert dort
        };
    }

    _handleMessage(msg) {
        switch (msg.type) {
            case 'trade':
                window.marketBus.emit('trade:executed', msg);
                break;
            case 'orderbook':
                window.marketBus.emit('orderbook:updated', msg);
                break;
            case 'ticker':
                window.marketBus.emit('ticker:updated', msg);
                break;
            case 'pong':
                // Heartbeat-Antwort vom Server – alles OK
                break;
            default:
                console.warn('[WS] Unknown message type:', msg.type);
        }
    }

    _reconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('[WS] Max reconnect attempts reached');
            this._updateStatusIndicator('failed');
            this._showReconnectButton();
            return;
        }

        this.reconnectAttempts++;
        const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
        console.log(`[WS] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
        
        this._updateStatusIndicator('reconnecting');
        setTimeout(() => this.connect(), delay);
    }

    startHeartbeat() {
        // Alle 25 Sekunden Ping senden (Cloud Run Timeout = 30s)
        this.heartbeatInterval = setInterval(() => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.ws.send(JSON.stringify({ type: 'ping' }));
            }
        }, 25000);
    }

    stopHeartbeat() {
        if (this.heartbeatInterval) {
            clearInterval(this.heartbeatInterval);
            this.heartbeatInterval = null;
        }
    }

    disconnect() {
        this.isIntentionallyClosed = true;
        this.stopHeartbeat();
        if (this.ws) {
            this.ws.close(1000, 'User navigated away');
        }
    }

    _updateStatusIndicator(status) {
        const indicator = document.getElementById('ws-status');
        if (!indicator) return;

        const states = {
            connected:    { text: 'Live', class: 'status--live', icon: '🟢' },
            disconnected: { text: 'Offline', class: 'status--offline', icon: '🔴' },
            reconnecting: { text: 'Verbinde...', class: 'status--reconnecting', icon: '🟡' },
            failed:       { text: 'Keine Verbindung', class: 'status--failed', icon: '⚫' },
        };

        const s = states[status] || states.disconnected;
        indicator.textContent = `${s.icon} ${s.text}`;
        indicator.className = `ws-status ${s.class}`;
    }

    _showReconnectButton() {
        const container = document.getElementById('ws-reconnect');
        if (!container) return;
        container.innerHTML = `
            <button onclick="window.marketWs.reconnectAttempts=0; window.marketWs.connect();" 
                    class="btn btn--small btn--outline">
                🔄 Erneut verbinden
            </button>`;
        container.style.display = 'block';
    }
}
```

---

#### 3.4.4. Candlestick-Chart Integration (ApexCharts)

**Warum ApexCharts?** Für tokenisierte Immobilien (RWA) benötigen wir keine hochkomplexen Day-Trading-Werkzeuge (Fibonacci etc.), sondern eine moderne, extrem fließende und vertrauenswürdige Visualisierung. Wir verwenden **ApexCharts**, da es:
1. **100% White-Label** ist (kein Wasserzeichen wie bei TradingView).
2. Atemberaubende und flüssige Kerzen-Animationen bei Live-Updates (WebSockets) liefert.
3. Sich optisch perfekt in ein "Neobank"-Design (Trade Republic / Robinhood Vibe) integrieren lässt.

**Frontend-Initialisierung (Architektur-Konzept):**

```javascript
// marketplace-charts.js
function initApexChart(containerId, initialCandles) {
    var options = {
        series: [{ data: initialCandles }],
        chart: {
            type: 'candlestick',
            height: 400,
            background: '#0a0a0f',   // POOOL Dark Mode
            animations: {
                enabled: true,
                easing: 'easeinout',
                speed: 300
            },
            fontFamily: "'TT Norms Pro', sans-serif",
            toolbar: { show: false } // Cleanes Neobank-Design
        },
        plotOptions: {
            candlestick: {
                colors: { upward: '#22c55e', downward: '#ef4444' }
            }
        },
        xaxis: { type: 'datetime' },
        yaxis: { tooltip: { enabled: true } }
    };

    var chart = new ApexCharts(document.querySelector("#" + containerId), options);
    chart.render();

    // Realtime Updates via Event-Bus
    window.marketBus.on('trade:executed', (trade) => {
        // ... (Update logic: Kerze aktualisieren oder neu anhängen)
    });
}
```

---

#### 3.4.5. Orderbook-Rendering (Live-DOM-Updates)

**Design-Prinzip:** Das Orderbook muss sich wie eine echte Börse anfühlen – neue Orders blitzen grün/rot auf, gefüllte Orders verschwinden mit einer kurzen Fade-Animation. Kein vollständiger Re-Render bei jedem WebSocket-Update, sondern gezielte DOM-Patches.

```javascript
// marketplace-orderbook.js
// Live-Orderbook mit effizienten DOM-Updates

class MarketplaceOrderbook {
    constructor() {
        this.bidsContainer = document.getElementById('orderbook-bids');
        this.asksContainer = document.getElementById('orderbook-asks');
        this.spreadDisplay = document.getElementById('orderbook-spread');
        this.lastBids = [];
        this.lastAsks = [];
    }

    init() {
        // Initial laden via REST API
        this.loadSnapshot();

        // Live-Updates via Event-Bus
        window.marketBus.on('orderbook:updated', (data) => {
            this.renderBids(data.bids);
            this.renderAsks(data.asks);
            this.updateSpread(data.spread);
        });
    }

    async loadSnapshot() {
        try {
            const res = await fetch(
                `/api/marketplace/${window.assetId}/orderbook`
            );
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const data = await res.json();

            this.renderBids(data.bids);
            this.renderAsks(data.asks);
            this.updateSpread(data.spread);
        } catch (err) {
            console.error('[Orderbook] Failed to load:', err);
            this.bidsContainer.innerHTML = 
                '<div class="orderbook-empty">Orderbook nicht verfügbar</div>';
        }
    }

    renderBids(bids) {
        this._renderSide(this.bidsContainer, bids, this.lastBids, 'bid');
        this.lastBids = bids;
    }

    renderAsks(asks) {
        // Asks werden umgekehrt angezeigt (höchster Preis oben)
        const reversed = [...asks].reverse();
        this._renderSide(this.asksContainer, reversed, this.lastAsks, 'ask');
        this.lastAsks = reversed;
    }

    _renderSide(container, levels, previousLevels, side) {
        // Effizientes DOM-Patching: Nur geänderte Zeilen aktualisieren
        const maxRows = 15;  // Top-15 Preis-Level anzeigen

        // Sicherstellen dass der Container genügend Zeilen hat
        while (container.children.length < maxRows) {
            container.appendChild(this._createRow(side));
        }
        while (container.children.length > maxRows) {
            container.removeChild(container.lastChild);
        }

        // Maximales Volumen für die Hintergrund-Balken berechnen
        const maxQty = Math.max(...levels.map(l => l.total_qty), 1);

        for (let i = 0; i < maxRows; i++) {
            const row = container.children[i];
            const level = levels[i];

            if (!level) {
                row.style.display = 'none';
                continue;
            }

            row.style.display = '';
            const priceEl = row.querySelector('.ob-price');
            const qtyEl = row.querySelector('.ob-qty');
            const barEl = row.querySelector('.ob-bar');

            const priceFormatted = (level.price / 100).toFixed(2);
            const qtyFormatted = level.total_qty.toLocaleString();

            // Flash-Animation wenn sich der Wert geändert hat
            const prevLevel = previousLevels[i];
            if (prevLevel && prevLevel.total_qty !== level.total_qty) {
                row.classList.add('ob-flash');
                setTimeout(() => row.classList.remove('ob-flash'), 300);
            }

            priceEl.textContent = `$${priceFormatted}`;
            qtyEl.textContent = qtyFormatted;

            // Hintergrund-Balken (visualisiert das Volumen)
            const barWidth = (level.total_qty / maxQty) * 100;
            barEl.style.width = `${barWidth}%`;
        }
    }

    _createRow(side) {
        const row = document.createElement('div');
        row.className = `ob-row ob-row--${side}`;
        row.innerHTML = `
            <div class="ob-bar"></div>
            <span class="ob-price"></span>
            <span class="ob-qty"></span>
        `;
        return row;
    }

    updateSpread(spreadCents) {
        if (this.spreadDisplay) {
            this.spreadDisplay.textContent = `Spread: $${(spreadCents / 100).toFixed(2)}`;
        }
    }
}
```

**CSS für das Orderbook (Auszug):**

```css
/* marketplace-trading.css – Orderbook Styles */

.orderbook {
    display: grid;
    grid-template-rows: auto 1fr auto 1fr auto;
    gap: 0;
    background: var(--surface-dark, #0f1117);
    border-radius: 12px;
    overflow: hidden;
    font-variant-numeric: tabular-nums;  /* Monospace-Zahlen für Alignment */
}

.ob-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 4px 12px;
    position: relative;
    font-size: 13px;
    transition: background-color 0.15s ease;
}

.ob-row--bid .ob-price { color: #22c55e; }  /* Grün: Kaufangebote */
.ob-row--ask .ob-price { color: #ef4444; }  /* Rot: Verkaufsangebote */

.ob-bar {
    position: absolute;
    top: 0;
    right: 0;
    height: 100%;
    opacity: 0.08;
    transition: width 0.3s ease;
}

.ob-row--bid .ob-bar { background: #22c55e; }
.ob-row--ask .ob-bar { background: #ef4444; }

/* Flash-Animation bei Updates */
.ob-flash {
    animation: ob-flash-anim 0.3s ease;
}
@keyframes ob-flash-anim {
    0%   { background: rgba(255,255,255,0.08); }
    100% { background: transparent; }
}

.ob-qty {
    text-align: right;
    color: var(--text-secondary, #9ca3af);
}
```

---

#### 3.4.6. Buy/Sell-Formular (Validierung, Debounce, Optimistic UI)

**Die kritischste UI-Komponente:** Ein Nutzer gibt hier €2.000+ ein und klickt „Kaufen". Zwischen Klick und Bestätigung darf es kein Doppel-Submit geben, die Balance muss client-seitig validiert werden, und der Button muss sofort disabled werden.

```javascript
// In marketplace-trading.js – Buy/Sell Form Handler

class OrderForm {
    constructor() {
        this.form = document.getElementById('order-form');
        this.submitBtn = document.getElementById('order-submit-btn');
        this.priceInput = document.getElementById('order-price');
        this.qtyInput = document.getElementById('order-quantity');
        this.sideToggle = document.querySelectorAll('[name="order-side"]');
        this.totalDisplay = document.getElementById('order-total');
        this.balanceDisplay = document.getElementById('available-balance');
        
        this.isSubmitting = false;
        this.userBalance = 0;       // In Cents, wird vom Server geladen
        this.userTokens = 0;        // Gehaltene Tokens für Sell-Orders
        this.currentSide = 'buy';
    }

    init() {
        // 1. Balance laden
        this._loadBalance();

        // 2. Echtzeit-Berechnung bei Eingabe
        this.priceInput.addEventListener('input', () => this._updateTotal());
        this.qtyInput.addEventListener('input', () => this._updateTotal());

        // 3. Buy/Sell Toggle
        this.sideToggle.forEach(radio => {
            radio.addEventListener('change', (e) => {
                this.currentSide = e.target.value;
                this._updateFormStyle();
                this._updateTotal();
            });
        });

        // 4. Form-Submit mit Debounce + Doppelklick-Schutz
        this.form.addEventListener('submit', (e) => {
            e.preventDefault();
            this._handleSubmit();
        });

        // 5. Balance-Updates via Event-Bus
        window.marketBus.on('balance:changed', (data) => {
            this.userBalance = data.new_balance_cents;
            this._updateBalanceDisplay();
        });

        // 6. "Max"-Button (setzt Quantity auf Maximum)
        const maxBtn = document.getElementById('order-max-btn');
        if (maxBtn) {
            maxBtn.addEventListener('click', () => this._setMaxQuantity());
        }
    }

    _updateTotal() {
        const price = Math.round(parseFloat(this.priceInput.value || 0) * 100);
        const qty = parseInt(this.qtyInput.value || 0);
        const total = price * qty;

        this.totalDisplay.textContent = `$${(total / 100).toFixed(2)}`;

        // Validierung: Rot markieren wenn Balance überschritten
        if (this.currentSide === 'buy' && total > this.userBalance) {
            this.totalDisplay.classList.add('text-danger');
            this.submitBtn.disabled = true;
            this.submitBtn.title = 'Nicht genügend Guthaben';
        } else if (this.currentSide === 'sell' && qty > this.userTokens) {
            this.totalDisplay.classList.add('text-danger');
            this.submitBtn.disabled = true;
            this.submitBtn.title = 'Nicht genügend Tokens';
        } else {
            this.totalDisplay.classList.remove('text-danger');
            this.submitBtn.disabled = false;
            this.submitBtn.title = '';
        }
    }

    async _handleSubmit() {
        // Doppelklick-Schutz
        if (this.isSubmitting) return;
        this.isSubmitting = true;

        // UI: Button sofort disablen + Spinner zeigen
        const originalText = this.submitBtn.textContent;
        this.submitBtn.disabled = true;
        this.submitBtn.innerHTML = '<span class="spinner-small"></span> Wird gesendet...';

        const price = Math.round(parseFloat(this.priceInput.value) * 100);
        const qty = parseInt(this.qtyInput.value);
        const idempotencyKey = crypto.randomUUID();  // Verhindert Server-seitige Doppelausführung

        try {
            // Client-seitige Validierung (letzte Verteidigungslinie)
            this._validate(price, qty);

            // Optimistic UI: Toast sofort zeigen
            showToast('info', `Order wird gesendet: ${qty} Shares @ $${(price/100).toFixed(2)}...`);

            const res = await fetch('/api/marketplace/orders', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Idempotency-Key': idempotencyKey,
                },
                body: JSON.stringify({
                    asset_id: window.assetId,
                    side: this.currentSide,
                    order_type: 'limit',
                    price_cents: price,
                    quantity: qty,
                }),
            });

            if (res.status === 428) {
                // 428 = 2FA Required (Step-Up Authentication)
                this._show2FAModal(idempotencyKey);
                return;
            }

            if (!res.ok) {
                const err = await res.json();
                throw new Error(err.message || `HTTP ${res.status}`);
            }

            const order = await res.json();

            // Erfolg!
            showToast('success', `✅ Order erstellt: ${qty} Shares @ $${(price/100).toFixed(2)}`);
            window.marketBus.emit('order:submitted', order);

            // Balance optimistisch aktualisieren
            if (this.currentSide === 'buy') {
                const total = price * qty;
                this.userBalance -= total;
                window.marketBus.emit('balance:changed', { 
                    new_balance_cents: this.userBalance 
                });
            }

            // Form zurücksetzen
            this.qtyInput.value = '';
            this._updateTotal();

        } catch (err) {
            console.error('[OrderForm] Submit failed:', err);
            showToast('error', `❌ ${err.message}`);
            window.marketBus.emit('error:api', { 
                endpoint: 'orders', message: err.message 
            });
        } finally {
            // Button wiederherstellen
            this.isSubmitting = false;
            this.submitBtn.disabled = false;
            this.submitBtn.textContent = originalText;
        }
    }

    _validate(priceCents, qty) {
        if (priceCents <= 0) throw new Error('Preis muss größer als $0 sein');
        if (qty <= 0 || !Number.isInteger(qty)) throw new Error('Menge muss eine ganze Zahl > 0 sein');
        if (priceCents * qty < 1000) throw new Error('Mindestordervolumen: $10.00');

        if (this.currentSide === 'buy') {
            if (priceCents * qty > this.userBalance) {
                throw new Error('Nicht genügend Guthaben');
            }
        } else {
            if (qty > this.userTokens) {
                throw new Error(`Nicht genügend Tokens (verfügbar: ${this.userTokens})`);
            }
        }
    }

    _setMaxQuantity() {
        if (this.currentSide === 'buy') {
            const price = Math.round(parseFloat(this.priceInput.value || 0) * 100);
            if (price > 0) {
                this.qtyInput.value = Math.floor(this.userBalance / price);
            }
        } else {
            this.qtyInput.value = this.userTokens;
        }
        this._updateTotal();
    }

    _updateFormStyle() {
        const isBuy = this.currentSide === 'buy';
        this.submitBtn.textContent = isBuy ? 'Kaufen' : 'Verkaufen';
        this.submitBtn.className = isBuy 
            ? 'btn btn--primary btn--buy' 
            : 'btn btn--primary btn--sell';
    }

    async _loadBalance() {
        try {
            const res = await fetch('/api/wallet/balance');
            if (res.ok) {
                const data = await res.json();
                this.userBalance = data.balance_cents || 0;
                this._updateBalanceDisplay();
            }
        } catch (err) {
            console.error('[OrderForm] Failed to load balance:', err);
        }

        try {
            const res = await fetch(`/api/portfolio/${window.assetId}`);
            if (res.ok) {
                const data = await res.json();
                this.userTokens = data.tokens_owned || 0;
            }
        } catch (err) {
            console.error('[OrderForm] Failed to load tokens:', err);
        }
    }

    _updateBalanceDisplay() {
        if (this.balanceDisplay) {
            this.balanceDisplay.textContent = 
                `Verfügbar: $${(this.userBalance / 100).toFixed(2)}`;
        }
    }

    _show2FAModal(idempotencyKey) {
        // 2FA-Modal anzeigen (Step-Up Authentication)
        const modal = document.getElementById('twofa-modal');
        if (modal) {
            modal.dataset.idempotencyKey = idempotencyKey;
            modal.classList.add('modal--active');
            modal.querySelector('input[name="totp-code"]')?.focus();
        }
    }
}
```

---

#### 3.4.7. P2P-Offer Flow (Cap Table + Modals)

```javascript
// marketplace-p2p.js
// P2P (Peer-to-Peer) Direktangebote zwischen Nutzern

class P2POfferManager {
    constructor(assetId) {
        this.assetId = assetId;
    }

    init() {
        // "Privates Angebot senden" Buttons in der Cap Table
        document.querySelectorAll('[data-action="send-p2p-offer"]').forEach(btn => {
            btn.addEventListener('click', () => {
                const targetUserId = btn.dataset.userId;
                const targetUsername = btn.dataset.username;
                this._showOfferModal(targetUserId, targetUsername);
            });
        });

        // Eingehende Angebote via WebSocket
        window.marketBus.on('p2p:offer_received', (offer) => {
            this._showIncomingOffer(offer);
        });
    }

    _showOfferModal(targetUserId, targetUsername) {
        const modal = document.getElementById('p2p-offer-modal');
        modal.querySelector('.p2p-target-name').textContent = targetUsername;
        modal.querySelector('[name="taker_user_id"]').value = targetUserId;
        modal.classList.add('modal--active');

        // Submit-Handler
        const form = modal.querySelector('form');
        form.onsubmit = async (e) => {
            e.preventDefault();
            await this._submitOffer(form, modal);
        };
    }

    async _submitOffer(form, modal) {
        const data = new FormData(form);
        try {
            const res = await fetch('/api/marketplace/p2p/offer', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    asset_id: this.assetId,
                    taker_user_id: data.get('taker_user_id'),
                    side: data.get('side'),
                    price_cents: Math.round(parseFloat(data.get('price')) * 100),
                    quantity: parseInt(data.get('quantity')),
                    message: data.get('message') || null,
                }),
            });

            if (!res.ok) {
                const err = await res.json();
                throw new Error(err.message);
            }

            showToast('success', '✅ Angebot erfolgreich gesendet');
            modal.classList.remove('modal--active');
            form.reset();
        } catch (err) {
            showToast('error', `❌ ${err.message}`);
        }
    }

    _showIncomingOffer(offer) {
        const badge = document.getElementById('p2p-notification-badge');
        if (badge) {
            const count = parseInt(badge.textContent || '0') + 1;
            badge.textContent = count;
            badge.style.display = 'inline-flex';
        }
        
        showToast('info', 
            `📩 Neues Angebot: ${offer.quantity} Shares @ $${(offer.price_cents/100).toFixed(2)}`,
            8000  // 8 Sekunden sichtbar
        );
    }
}
```

---

#### 3.4.8. Trading-Seite Orchestrierung (`marketplace-trading.js`)

Die Hauptdatei initialisiert alle Sub-Module und verbindet sie:

```javascript
// marketplace-trading.js
// Orchestriert alle Marketplace-Komponenten auf der Asset-Trading-Seite

document.addEventListener('DOMContentLoaded', async () => {
    // Asset-ID aus dem URL-Pfad oder einem Data-Attribut
    window.assetId = document.body.dataset.assetId 
        || window.location.pathname.split('/').pop();

    // 1. Event-Bus ist bereits geladen (marketplace-event-bus.js)

    // 2. WebSocket-Verbindung herstellen
    window.marketWs = new MarketplaceWebSocket(window.assetId);
    window.marketWs.connect();

    // 3. Chart initialisieren
    window.marketChart = new MarketplaceChart('chart-container', window.assetId);
    await window.marketChart.init();

    // 4. Orderbook initialisieren
    const orderbook = new MarketplaceOrderbook();
    orderbook.init();

    // 5. Buy/Sell-Formular initialisieren
    const orderForm = new OrderForm();
    orderForm.init();

    // 6. P2P-Offers initialisieren
    const p2p = new P2POfferManager(window.assetId);
    p2p.init();

    // 7. Globales Error-Handling: Alle API-Fehler → Toast
    window.marketBus.on('error:api', (err) => {
        console.error(`[API Error] ${err.endpoint}: ${err.message}`);
    });

    // 8. Page-Visibility: WS pausieren wenn Tab inaktiv
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            window.marketWs.stopHeartbeat();
        } else {
            window.marketWs.startHeartbeat();
        }
    });

    // 9. Cleanup bei Navigation
    window.addEventListener('beforeunload', () => {
        window.marketWs.disconnect();
    });
});
```

---

#### 3.4.9. Loading-, Error- und Empty-States

**Jede Komponente muss 4 Zustände korrekt darstellen:**

| Zustand | Beschreibung | UI-Verhalten |
|---|---|---|
| **Loading** | Daten werden geladen | Skeleton-Placeholder (Shimmer-Animation) |
| **Success** | Daten da | Normale Anzeige |
| **Empty** | Keine Daten (z.B. keine Trades) | Friendly Message: „Noch keine Trades für dieses Asset" |
| **Error** | API-Fehler | Error-Box mit Retry-Button |

```javascript
// Utility: State-Management für eine Komponente
function setComponentState(containerId, state, options = {}) {
    const container = document.getElementById(containerId);
    if (!container) return;

    // Alle State-Klassen entfernen
    container.classList.remove('state--loading', 'state--empty', 'state--error');

    switch (state) {
        case 'loading':
            container.classList.add('state--loading');
            container.innerHTML = `
                <div class="skeleton-loader">
                    <div class="skeleton-line"></div>
                    <div class="skeleton-line skeleton-line--short"></div>
                    <div class="skeleton-line"></div>
                </div>`;
            break;

        case 'empty':
            container.classList.add('state--empty');
            container.innerHTML = `
                <div class="empty-state">
                    <span class="empty-state-icon">${options.icon || '📊'}</span>
                    <p class="empty-state-text">${options.message || 'Keine Daten verfügbar'}</p>
                </div>`;
            break;

        case 'error':
            container.classList.add('state--error');
            container.innerHTML = `
                <div class="error-state">
                    <span class="error-state-icon">⚠️</span>
                    <p class="error-state-text">${options.message || 'Fehler beim Laden'}</p>
                    <button class="btn btn--small btn--outline" 
                            onclick="${options.retryFn || ''}">
                        🔄 Erneut versuchen
                    </button>
                </div>`;
            break;
    }
}
```

---

#### 3.4.10. Accessibility (Barrierefreiheit)

| Anforderung | Umsetzung |
|---|---|
| **Keyboard-Navigation** | Alle interaktiven Elemente (Buttons, Inputs, Tabs) per `Tab` erreichbar |
| **ARIA-Labels** | `aria-label="Orderbook Kaufangebote"` auf den Orderbook-Containern |
| **Focus-Management** | Nach Modal-Öffnung: Focus ins erste Input. Nach Modal-Schließung: Focus zurück zum Trigger |
| **Screen-Reader** | Trade-Toasts mit `role="alert"` für sofortige Ansage |
| **Kontrast** | Grün (#22c55e) und Rot (#ef4444) auf dunklem Hintergrund = WCAG AA konform |
| **Reduced Motion** | `@media (prefers-reduced-motion: reduce)` – Flash-Animationen deaktivieren |

```css
/* Reduced Motion: Disables flashing for users who prefer less animation */
@media (prefers-reduced-motion: reduce) {
    .ob-flash { animation: none; }
    .skeleton-line { animation: none; }
    * { transition-duration: 0.01ms !important; }
}
```

---

#### 3.4.11. Sicherheits-Checkliste (vor Marketplace-Launch)

| # | Aufgabe | Status | Verantwortlich |
|---|---|---|---|
| 1 | Doppelklick-Schutz auf allen finanziellen Submit-Buttons verifiziert | ❌ | Frontend |
| 2 | Idempotency-Key wird bei jedem Order-Submit generiert und mitgesendet | ❌ | Frontend |
| 3 | Client-seitige Balance-Validierung inkl. `held_balance` | ❌ | Frontend |
| 4 | WebSocket-Reconnect funktioniert nach 30s Disconnect | ❌ | Frontend + QA |
| 5 | Alle Geldbeträge als Integer-Cents verarbeitet (kein Float im JS) | ❌ | Frontend |
| 6 | XSS-Schutz: Kein `innerHTML` mit User-generierten Daten (nur `textContent`) | ❌ | Frontend |
| 7 | CSP-Header erlaubt `wss://` für WebSocket-Verbindungen | ❌ | Frontend + Backend |
| 8 | Chart-Library (`lightweight-charts`) self-hosted (nicht per CDN) | ❌ | Frontend |
| 9 | Toast-Notifications haben `role="alert"` für Screen-Reader | ❌ | Frontend |
| 10 | Alle Inputs haben `inputmode="decimal"` für Mobile-Keyboards | ❌ | Frontend |
| 11 | Loading/Error/Empty-States für alle Komponenten implementiert | ❌ | Frontend |
| 12 | 2FA-Modal (Step-Up) zeigt sich korrekt bei $500+ Trades | ❌ | Frontend + Backend |
| 13 | Page-Visibility API: WS pausiert bei inaktivem Tab | ❌ | Frontend |
| 14 | `beforeunload`: WS-Verbindung sauber schließen | ❌ | Frontend |
| 15 | Responsive Design: Trading-Seite funktioniert auf Mobile (360px+) | ❌ | Frontend + QA |

---

#### 3.4.12. Zusammenfassung: Wochenplan für den Frontend Engineer

```
Woche 1: Event-Bus + WebSocket-Client + Connection-Status UI
         Grundlayout der Trading-Seite (HTML + CSS Grid)
         Lightweight-Charts einbinden + Candlestick-Daten laden
         
Woche 2: Orderbook-Rendering + Live-DOM-Updates via WebSocket
         Buy/Sell-Formular mit Validierung + Debounce + Doppelklick-Schutz
         Balance-Anzeige + "Max"-Button + Echtzeit-Total-Berechnung
         
Woche 3: P2P-Offer Modals + Cap Table + Notification-Badge
         2FA Step-Up Modal (für Trades >$500)
         Loading/Error/Empty-States für alle Komponenten
         
Woche 4: Responsive Design (Mobile-First: 360px → 1920px)
         Accessibility (ARIA, Keyboard, Reduced Motion)
         Cross-Browser Testing (Chrome, Safari, Firefox, Mobile Safari)
         Performance-Audit (keine >50ms DOM-Reflows bei WS-Updates)
```

> **Die goldene Regel:** Kein `innerHTML` mit User-Daten. Kein `parseFloat` für Geldbeträge. Kein Submit-Button der nach dem Klick enabled bleibt. Diese drei Fehler sind die häufigsten Ursachen für XSS, Rundungsfehler und Doppel-Trades in Trading-UIs.

---

### 3.5. Marketplace Admin Dashboard (Steuerung, Überwachung & Compliance)
*Die Kommandobrücke des Handelsplatzes. Ohne sie fliegt das System blind.*

> **Verantwortungsbereich:** Der Marketplace-Admin-Bereich ist KEINE einzelne Seite, sondern eine **komplette Sektion** im bestehenden Admin-Dashboard mit mindestens 12 Unterseiten. Er gibt Admins die Werkzeuge zur Echtzeit-Überwachung aller Trades, zur Genehmigung von Großorders, zur Fee-Verwaltung, zur Reconciliation-Prüfung, zum Fraud-Monitoring und zur OJK-Compliance-Berichterstattung. Ohne diesen Bereich ist der Marketplace blind – kein Admin könnte eingreifen, wenn ein Bug €50.000 falsch bucht.

---

#### 3.5.1. RBAC: Neue Marketplace-Permissions (Erweiterung des bestehenden Systems)

Das bestehende `admin-permission-guard.js` verwendet ein Permission-basiertes System (z.B. `users.view`, `kyc.read`, `treasury.read`). Für den Marketplace werden **3 neue Permission-Gruppen** benötigt:

**Neue Permissions:**

| Permission | Beschreibung | Wer bekommt sie? |
|---|---|---|
| `marketplace.view` | Kann Trades, Orderbook und Marktdaten einsehen (read-only) | Marketplace Manager, Compliance Officer, Super Admin |
| `marketplace.manage` | Kann Orders genehmigen/ablehnen, Fees ändern, Assets enablen/disablen | Marketplace Manager, Super Admin |
| `marketplace.compliance` | Kann Reconciliation-Reports einsehen, OJK-Berichte generieren, Travel-Rule-Daten exportieren | Compliance Officer, Super Admin |

**Erweiterung der `PAGE_PERMISSION_MAP` in `admin-permission-guard.js`:**

```javascript
// NEU: Marketplace-Sektion Permissions
"nav-marketplace-overview":    "marketplace.view",
"nav-marketplace-orders":      "marketplace.view",
"nav-marketplace-trades":      "marketplace.view",
"nav-marketplace-orderbook":   "marketplace.view",
"nav-marketplace-fees":        "marketplace.manage",
"nav-marketplace-approvals":   "marketplace.manage",
"nav-marketplace-p2p":         "marketplace.view",
"nav-marketplace-analytics":   "marketplace.view",
"nav-marketplace-alerts":      "marketplace.manage",
"nav-marketplace-reconciliation": "marketplace.compliance",
"nav-marketplace-compliance":  "marketplace.compliance",
"nav-marketplace-settings":    "marketplace.manage",
```

**Empfohlene Admin-Rollen:**

| Rolle | Bestehende Permissions | Neue Marketplace Permissions |
|---|---|---|
| **Super Admin** | `all` | Automatisch alles |
| **Marketplace Manager** (NEU) | `assets.view`, `orders.view` | `marketplace.view`, `marketplace.manage` |
| **Compliance Officer** (NEU) | `audit.read`, `kyc.read` | `marketplace.view`, `marketplace.compliance` |
| **Support Agent** | `support.read`, `users.view` | `marketplace.view` (read-only) |

---

#### 3.5.2. Sidebar-Navigation: Neue Marketplace-Sektion

Die bestehende Admin-Sidebar bekommt eine neue Sektion zwischen "Financial" und "Settings":

```
Bestehende Sidebar-Struktur:          Erweiterte Struktur:
─────────────────────────             ─────────────────────────
📊 Dashboard                          📊 Dashboard
                                      
👥 USER MANAGEMENT                    👥 USER MANAGEMENT
   Users                                 Users
   KYC                                   KYC
   Support                               Support
                                      
📋 CONTENT                            📋 CONTENT
   Submissions                           Submissions
   Assets                                Assets
                                      
💰 FINANCIAL                          💰 FINANCIAL
   Orders                                Orders
   Deposits                              Deposits
   Treasury                              Treasury
   Dividends                             Dividends
   Rewards                               Rewards
                                      
                                      📈 MARKETPLACE (NEU)
                                         Overview & Monitoring
                                         Live Orderbook
                                         Trade History
                                         Open Orders
                                         Pending Approvals
                                         Fee Management
                                         P2P Offers
                                         Analytics & Charts
                                         Alerts & Watchlist
                                         Reconciliation
                                         Compliance & OJK
                                         Marketplace Settings
                                      
⚙️ SYSTEM                            ⚙️ SYSTEM
   Admins & Roles                        Admins & Roles
   Audit Logs                            Audit Logs
   Settings                              Settings
```

---

#### 3.5.3. Die 12 Marketplace-Admin-Seiten (Übersicht)

| # | Seite | Route | Priorität | Datenquelle | Zweck |
|---|---|---|---|---|---|
| 1 | **Overview & Monitoring** | `/admin/marketplace/` | 🔴 LAUNCH | Core DB + Redis | Echtzeit-Dashboard: KPIs, Volume, Status-Ampeln |
| 2 | **Live Orderbook** | `/admin/marketplace/orderbook` | 🔴 LAUNCH | Redis + WebSocket | Admin sieht das Live-Orderbook aller Assets |
| 3 | **Trade History** | `/admin/marketplace/trades` | 🔴 LAUNCH | `trade_history` | Alle ausgeführten Trades mit Filtern + Export |
| 4 | **Open Orders** | `/admin/marketplace/orders` | 🔴 LAUNCH | `market_orders` | Alle offenen Orders aller Nutzer |
| 5 | **Pending Approvals** | `/admin/marketplace/approvals` | 🔴 LAUNCH | `market_orders` WHERE status='pending_review' | Großorder-Genehmigungen |
| 6 | **Fee Management** | `/admin/marketplace/fees` | 🟡 WEEK 2 | `fee_configurations`, `fee_promotions` | Fees erstellen, ändern, Promotions steuern |
| 7 | **P2P Offers** | `/admin/marketplace/p2p` | 🟡 WEEK 2 | `p2p_offers` | Übersicht privater Angebote |
| 8 | **Analytics & Charts** | `/admin/marketplace/analytics` | 🟡 WEEK 3 | `trade_history`, `candles_*` | Volume-Charts, Asset-Vergleiche, Top-Trader |
| 9 | **Alerts & Watchlist** | `/admin/marketplace/alerts` | 🟡 WEEK 3 | Custom metrics + Redis | Wash-Trading-Alerts, Anomalien, Watchlist |
| 10 | **Reconciliation** | `/admin/marketplace/reconciliation` | 🔴 LAUNCH | `wallets`, `trade_history`, `wallet_transactions` | Bilanz-Check: SUM(wallets) == expected |
| 11 | **Compliance & OJK** | `/admin/marketplace/compliance` | 🟡 WEEK 4 | `travel_rule_records`, `trade_history` | OJK-Reports, Travel-Rule-Daten, Steuer-Exports |
| 12 | **Marketplace Settings** | `/admin/marketplace/settings` | 🟡 WEEK 2 | Config-Tabelle + Redis | Globale Limits, Circuit-Breaker-Schwellen, Trading-Status |

---

#### 3.5.4. Seite 1: Overview & Monitoring (`/admin/marketplace/`)

**Priorität: 🔴 Zwingend zum Launch**

Das Haupt-Dashboard. Gibt dem Admin auf einen Blick den kompletten Status des Marktplatzes.

**Layout (4 Zeilen):**

```
┌─────────────────────────────────────────────────────────────────┐
│  MARKETPLACE OVERVIEW                               🟢 LIVE    │
│                                                                 │
│  ROW 1: Status-Ampeln (4 Cards)                                 │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────┐ │
│  │ Trading      │ │ Open Orders  │ │ 24h Volume   │ │ Pending│ │
│  │ 🟢 ACTIVE    │ │    142       │ │  $48,230     │ │  3     │ │
│  │              │ │ +12 today    │ │  ↑23% vs y/d │ │ Review │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └────────┘ │
│                                                                 │
│  ROW 2: Letzte 10 Trades (Live-Tabelle, via WebSocket)          │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Time     │ Asset          │ Side │ Price  │ Qty │ Total    │ │
│  │ 14:32:17 │ Berlin Apt 42  │ BUY  │ $105   │ 30  │ $3,150  │ │
│  │ 14:30:05 │ Jakarta Tower  │ SELL │ $72    │ 100 │ $7,200  │ │
│  │ ...      │ ...            │ ...  │ ...    │ ... │ ...     │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ROW 3: Top-5 Assets nach Volume (Horizontal Bar Chart)         │
│  ROW 4: System-Health (DB Latenz, Redis Memory, WS Connections) │
└─────────────────────────────────────────────────────────────────┘
```

**API-Endpoints die diese Seite konsumiert:**

| Endpoint | Methode | Daten | Refresh |
|---|---|---|---|
| `/api/admin/marketplace/stats` | GET | KPI-Cards (Volume, Orders, Trades, Pending) | Alle 30s Auto-Refresh |
| `/api/admin/marketplace/recent-trades?limit=10` | GET | Letzte 10 Trades | Via WebSocket oder 10s Polling |
| `/api/admin/marketplace/top-assets?period=24h` | GET | Top-5 Assets nach Volume | Alle 60s |
| `/api/admin/marketplace/health` | GET | DB-Latenz, Redis-Status, WS-Connections | Alle 15s |

**Datenbank-Queries (Backend):**

```sql
-- KPI: 24h Trading Volume
SELECT COALESCE(SUM(total_cents), 0) AS volume_24h,
       COUNT(*) AS trade_count_24h
FROM trade_history
WHERE executed_at > NOW() - INTERVAL '24 hours';

-- KPI: Offene Orders
SELECT COUNT(*) AS open_orders
FROM market_orders
WHERE status IN ('open', 'partially_filled');

-- KPI: Pending Approvals
SELECT COUNT(*) AS pending_approvals
FROM market_orders
WHERE status = 'pending_review';

-- Top-5 Assets nach Volume (24h)
SELECT a.title, a.id,
       COALESCE(SUM(th.total_cents), 0) AS volume_cents,
       COUNT(th.id) AS trade_count
FROM assets a
LEFT JOIN trade_history th ON a.id = th.asset_id 
    AND th.executed_at > NOW() - INTERVAL '24 hours'
GROUP BY a.id, a.title
ORDER BY volume_cents DESC
LIMIT 5;
```

---

#### 3.5.5. Seite 2: Live Orderbook (`/admin/marketplace/orderbook`)

**Priorität: 🔴 Zwingend zum Launch**

Der Admin sieht das aggregierte Orderbook **aller Assets** oder kann auf ein einzelnes Asset filtern. Im Gegensatz zur öffentlichen Frontend-Ansicht sieht der Admin auch die **User-IDs** hinter jeder Order.

**Features:**

| Feature | Beschreibung | Datenquelle |
|---|---|---|
| **Asset-Filter (Dropdown)** | Auswahl eines bestimmten Assets oder "Alle" | `assets` Tabelle |
| **Bid/Ask-Tabelle** | Top-20 Bids + Asks mit User-ID, Menge, Preis | Redis `bids:asset:*`, `asks:asset:*` |
| **User-Identifikation** | Klick auf User-ID → Link zu `/admin/users/{id}` | `users` Tabelle |
| **Spread-Anzeige** | Aktueller Spread in Cents + Prozent | Berechnet |
| **Order-Tiefe** | Summe aller Bids vs. Asks (Balanceindikator) | Redis aggregiert |
| **Manueller Rebuild** | Button: "Orderbook aus DB neu laden" (nach Redis-Crash) | `market_orders` → Redis |

**API-Endpoints:**

| Endpoint | Methode | Beschreibung |
|---|---|---|
| `/api/admin/marketplace/orderbook/{asset_id}` | GET | Orderbook mit User-IDs (Admin-Only, mehr Daten als Public) |
| `/api/admin/marketplace/orderbook/rebuild` | POST | Triggert Redis-Rebuild aus PostgreSQL (🔒 marketplace.manage) |

---

#### 3.5.6. Seite 3: Trade History (`/admin/marketplace/trades`)

**Priorität: 🔴 Zwingend zum Launch**

Vollständige, filterbare Trade-Historie mit Export-Funktion. Die kritischste Audit-Seite.

**Features:**

| Feature | Beschreibung | Priorität |
|---|---|---|
| **Server-seitige Pagination** | 50 Trades pro Seite, Cursor-basiert | 🔴 Launch |
| **Filter: Asset** | Dropdown oder Suchfeld | 🔴 Launch |
| **Filter: Zeitraum** | Date-Range-Picker (Von–Bis) | 🔴 Launch |
| **Filter: User** | Suche nach Buyer oder Seller (User-ID oder Name) | 🔴 Launch |
| **Filter: Preis-Range** | Min/Max Preis | 🟡 Week 2 |
| **Filter: On-Chain-Status** | pending / submitted / confirmed / failed | 🔴 Launch |
| **Export: CSV** | Alle gefilterten Trades als CSV herunterladen | 🔴 Launch |
| **Export: PDF** | Formatierter Report für Compliance | 🟡 Week 3 |
| **Klickbare User-IDs** | Link zu `/admin/users/{id}` (Buyer + Seller) | 🔴 Launch |
| **Klickbare Asset-Namen** | Link zu `/admin/assets/{id}` | 🔴 Launch |
| **Fee-Spalte** | Zeigt die berechnete Fee pro Trade | 🔴 Launch |
| **On-Chain TX-Link** | Link zu Basescan für bestätigte Trades | 🟡 Week 3 |

**API-Endpoint:**

```
GET /api/admin/marketplace/trades
    ?page=1
    &per_page=50
    &asset_id=550e8400-...
    &from=2026-03-01T00:00:00Z
    &to=2026-03-20T23:59:59Z
    &user_id=...
    &on_chain_status=pending
    &sort=executed_at
    &order=desc
```

**Datenbank-Query:**

```sql
SELECT th.id, th.executed_at, th.price_cents, th.quantity, th.total_cents,
       th.fee_cents, th.on_chain_status, th.on_chain_tx_hash,
       a.title AS asset_title, a.id AS asset_id,
       buyer.email AS buyer_email, buyer.id AS buyer_id,
       seller.email AS seller_email, seller.id AS seller_id
FROM trade_history th
JOIN assets a ON th.asset_id = a.id
JOIN users buyer ON th.buyer_user_id = buyer.id
JOIN users seller ON th.seller_user_id = seller.id
WHERE ($1::UUID IS NULL OR th.asset_id = $1)
  AND ($2::TIMESTAMPTZ IS NULL OR th.executed_at >= $2)
  AND ($3::TIMESTAMPTZ IS NULL OR th.executed_at <= $3)
  AND ($4::UUID IS NULL OR th.buyer_user_id = $4 OR th.seller_user_id = $4)
  AND ($5::VARCHAR IS NULL OR th.on_chain_status = $5)
ORDER BY th.executed_at DESC
LIMIT $6 OFFSET $7;
```

---

#### 3.5.7. Seite 4: Open Orders (`/admin/marketplace/orders`)

**Priorität: 🔴 Zwingend zum Launch**

Alle aktuell offenen und teilweise gefüllten Orders aller Nutzer. Der Admin kann jede Order einsehen und manuell stornieren (z.B. bei Verdacht auf Manipulation).

**Features:**

| Feature | Beschreibung |
|---|---|
| **Tabelle** | ID, User, Asset, Side, Price, Qty, Filled, Status, Created, Expires |
| **Filter: Status** | open, partially_filled, pending_review, expired |
| **Filter: Side** | buy / sell |
| **Filter: Asset** | Dropdown |
| **Admin-Cancel** | Button "Cancel Order" (mit Bestätigungs-Dialog + Grund-Eingabe) |
| **Held-Balance-Anzeige** | Zeigt wie viel Geld/Tokens der User aktuell geblockt hat |
| **Ablauf-Timer** | Zeigt verbleibende Zeit bis zum Expiry |

**Admin-Cancel-API:**

```
DELETE /api/admin/marketplace/orders/{order_id}
Body: { "reason": "Suspected wash trading" }
Auth: 🔒 marketplace.manage
```

> **Wichtig:** Beim Admin-Cancel wird die gleiche `release_hold()`-Logik aufgerufen wie beim User-Cancel. Der Unterschied: Es wird ein Audit-Log-Eintrag mit `actor=admin` und `reason` erstellt.

---

#### 3.5.8. Seite 5: Pending Approvals (`/admin/marketplace/approvals`)

**Priorität: 🔴 Zwingend zum Launch**

Großorders (>20% eines Asset-Supplies oder >$50.000 Volumen) landen hier zur manuellen Prüfung.

**Layout:**

```
┌─────────────────────────────────────────────────────────────────┐
│  PENDING APPROVALS (3 offene Anträge)                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ ⏳ ORDER #a1b2c3 – vor 2 Stunden                           │ │
│  │                                                             │ │
│  │ User:    Max Mustermann (ID: 550e8400-...)                  │ │
│  │ Asset:   Berlin Apartment 42 (BA42)                         │ │
│  │ Side:    BUY                                                │ │
│  │ Price:   $105.00/Token × 250 Tokens = $26,250.00            │ │
│  │ Supply:  250 / 1.000 Tokens = ⚠️ 25% des Gesamtangebots    │ │
│  │                                                             │ │
│  │ User-Kontext:                                               │ │
│  │ • KYC: ✅ Verifiziert (seit 12.01.2026)                     │ │
│  │ • Bisherige Trades: 14 (alle unauffällig)                   │ │
│  │ • Wallet-Balance: $32,500.00                                │ │
│  │ • Bereits gehaltene Tokens: 0 (Neuinvestor in dieses Asset) │ │
│  │                                                             │ │
│  │ [✅ Genehmigen]  [❌ Ablehnen]  [👤 User-Profil öffnen]    │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ ⏳ ORDER #d4e5f6 – vor 45 Minuten                           │ │
│  │ ...                                                         │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

**API-Endpoints:**

| Endpoint | Methode | Beschreibung |
|---|---|---|
| `/api/admin/marketplace/orders/pending` | GET | Alle Orders mit status='pending_review' |
| `/api/admin/marketplace/orders/{id}/approve` | POST | Order genehmigen → Status → 'open', Order ins Redis-Orderbook |
| `/api/admin/marketplace/orders/{id}/reject` | POST | Order ablehnen → Status → 'rejected', Balance-Hold zurückgeben |

**Beim Genehmigen passiert:**
1. `market_orders.status` → `'open'`
2. Order wird in Redis Sorted Set eingefügt (`ZADD`)
3. Audit-Log: `"Admin {admin_name} approved large order {order_id}"`
4. Notification an den User: "Deine Order wurde genehmigt und ist jetzt im Orderbook"

**Beim Ablehnen passiert:**
1. `market_orders.status` → `'rejected'`
2. Balance-Hold wird zurückgegeben (`release_hold()`)
3. Audit-Log mit Ablehnungsgrund
4. Notification an den User mit Ablehnungsgrund

---

#### 3.5.9. Seite 6: Fee Management (`/admin/marketplace/fees`)

**Priorität: 🟡 Week 2**

Zentrale Steuerung aller Gebühren (siehe auch Abschnitt 2.6). Der Admin konfiguriert hier die 4-Ebenen Fee-Hierarchie.

**Layout (3 Tabs):**

```
[Platform Defaults] [Asset-spezifisch] [Promotions]
```

**Tab 1: Platform Defaults**

| Feld | Aktueller Wert | Editierbar |
|---|---|---|
| Taker Fee | 5.00% (500 BPS) | ✅ Input mit BPS-Slider |
| Maker Fee | 0.00% (0 BPS) | ✅ Input mit BPS-Slider |
| Withdrawal Fee | $2.50 (250 Cents) | ✅ Dollar-Input |
| P2P Fee | 5.00% (500 BPS) | ✅ Input mit BPS-Slider |
| Listing Fee | $0.00 | ✅ Dollar-Input |

**Tab 2: Asset-spezifische Overrides**

Tabelle aller Assets mit individuellen Fee-Overrides:

| Asset | Taker | Maker | Override-Grund | Aktiv | Aktionen |
|---|---|---|---|---|---|
| Berlin Apartment 42 | 0.30% | 0.00% | Early-Bird Deal | ✅ | [Bearbeiten] [Löschen] |
| Jakarta Tower | (Platform Default) | (Platform Default) | – | – | [Override erstellen] |

**Tab 3: Promotions (zeitlich begrenzte Fee-Aktionen)**

| Promotion | Scope | Taker | Zeitraum | Status | Aktionen |
|---|---|---|---|---|---|
| "Launch Week" | Global | 0.00% | 01.04. – 07.04.2026 | ⏳ Geplant | [Bearbeiten] [Stornieren] |
| "Jakarta Premium" | Asset: Jakarta Tower | 0.25% | 15.03. – 15.06.2026 | 🟢 Aktiv | [Beenden] |

**API-Endpoints:**

| Endpoint | Methode | Beschreibung | Auth |
|---|---|---|---|
| `GET /api/admin/marketplace/fees` | GET | Alle Fee-Konfigurationen | `marketplace.view` |
| `PUT /api/admin/marketplace/fees/platform` | PUT | Platform-Defaults ändern | `marketplace.manage` |
| `POST /api/admin/marketplace/fees/asset` | POST | Asset-spezifischen Override erstellen | `marketplace.manage` |
| `DELETE /api/admin/marketplace/fees/asset/{id}` | DELETE | Override entfernen | `marketplace.manage` |
| `GET /api/admin/marketplace/promotions` | GET | Alle Promotions | `marketplace.view` |
| `POST /api/admin/marketplace/promotions` | POST | Neue Promotion erstellen | `marketplace.manage` |
| `PUT /api/admin/marketplace/promotions/{id}` | PUT | Promotion bearbeiten | `marketplace.manage` |
| `DELETE /api/admin/marketplace/promotions/{id}` | DELETE | Promotion stornieren | `marketplace.manage` |

---

#### 3.5.10. Seite 7: P2P Offers (`/admin/marketplace/p2p`)

**Priorität: 🟡 Week 2**

Übersicht aller privaten Peer-to-Peer-Angebote. Der Admin kann hier verdächtige P2P-Deals erkennen (z.B. weit unter Marktpreis → mögliche Geldwäsche).

**Features:**

| Feature | Beschreibung |
|---|---|
| **Tabelle** | Maker, Taker, Asset, Side, Price, Qty, Status, Created |
| **Preis-Warnung** | ⚠️ Icon wenn P2P-Preis >20% vom letzten Marktpreis abweicht |
| **Filter: Status** | pending, accepted, declined, expired, countered |
| **Admin-Cancel** | Kann verdächtige P2P-Offers manuell stornieren |
| **Counter-History** | Zeigt die Kette von Angeboten + Gegenangeboten (via `parent_offer_id`) |

**API-Endpoint:**

```
GET /api/admin/marketplace/p2p
    ?status=pending
    &asset_id=...
    &page=1&per_page=50
```

---

#### 3.5.11. Seite 8: Analytics & Charts (Embedded Metabase OSS)

**Priorität: 🟡 Week 3**

Anstatt komplexe HTML-Seiten für Analytics, Fee-Management und Berichte selbst in Vanilla JS und Chart.js zu programmieren, binden wir **Metabase OSS (Open Source)** direkt in das Admin-Panel ein. Das spart massive Entwicklungszeit und liefert extrem professionelle Reportings.

**Vorteile der Metabase Integration:**
* **100% Kostenlos:** Metabase OSS ist AGPL-lizenziert und dauerhaft kostenfrei, ohne Limits für Tabellen oder Dashboards. 
* **Self-Hosted:** Da das Backend auf Google Cloud Run läuft, stellen wir einfach einen zweiten Container daneben und verbinden ihn (read-only) mit der PostgreSQL Replica.
* **Kein Frontend-Code nötig:** Das gesamte Layout, Volumens-Charts, Tabellen und Heatmaps werden in Metabase per Drag & Drop erstellt.
* **Einbettung per Embed/Iframe:** Im POOOL-Admin-Dashboard gibt es nur einen `/admin/marketplace/analytics` Tab, der per Embed-URL sicher auf das Metabase-Dashboard verweist.

**Ersetzte Metriken / Charts (alle in Metabase visualisiert):**
* Volume Over Time (Tägliches Handelsvolumen)
* Fee-Revenue (Eingenommene Gebühren pro Woche)
* Top-Trader-Ranking
* Bid-Ask-Ratio und Anomalien-Auswertungen
* OJK Compliance und AML Reports lassen sich als CSV via Metabase auf Knopfdruck exportieren

Das führt zu einer **Zeitersparnis von ca. 2 Wochen Frontend-Entwicklung**!

---

#### 3.5.12. Seite 9: Alerts & Watchlist (`/admin/marketplace/alerts`)

**Priorität: 🟡 Week 3**

Zeigt automatisch generierte Alerts (Wash-Trading-Verdacht, Preis-Anomalien, Rate-Limit-Verstöße) und ermöglicht das manuelle Setzen von Watchlists für verdächtige Nutzer.

**Alert-Typen:**

| Alert | Trigger | Severity | Auto-Generiert? |
|---|---|---|---|
| **Wash-Trading: Same IP** | Buyer und Seller haben identische IP | 🟡 Warning | ✅ Via Matching-Engine |
| **Volume-Anomalie** | Ein User macht >50% des Tagesvolumens | 🟡 Warning | ✅ Via Daily Cron |
| **Rapid-Fire Orders** | >5 Orders/Minute auf gleiches Asset | 🟡 Warning | ✅ Via Rate-Limiter |
| **Preis-Abweichung** | Preis >15% vom letzten Gutachten-Wert | ⚠️ Alert | ✅ Via Trade-Event |
| **Balance negativ** | wallet.balance_cents < 0 (sollte unmöglich sein) | 🔴 Critical | ✅ Via Cron Check |
| **Reconciliation Mismatch** | SUM(wallets) ≠ expected | 🔴 Critical | ✅ Via Daily Reconciliation |
| **Manuell: User auf Watchlist** | Admin setzt einen User auf die Beobachtungsliste | 📌 Watchlist | ❌ Manuell |

**Datenbank-Schema für Alerts:**

```sql
-- database/054_marketplace_alerts.sql

CREATE TABLE marketplace_alerts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type   VARCHAR(50) NOT NULL,    -- 'wash_trading', 'volume_anomaly', etc.
    severity     VARCHAR(15) NOT NULL DEFAULT 'warning'
                 CHECK (severity IN ('info', 'warning', 'critical')),
    asset_id     UUID REFERENCES assets(id),
    user_id      UUID REFERENCES users(id),
    trade_id     UUID REFERENCES trade_history(id),
    message      TEXT NOT NULL,
    metadata     JSONB,                   -- Zusätzliche Daten (IPs, Volumes, etc.)
    status       VARCHAR(15) NOT NULL DEFAULT 'new'
                 CHECK (status IN ('new', 'acknowledged', 'resolved', 'false_positive')),
    resolved_by  UUID REFERENCES users(id),
    resolved_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_status ON marketplace_alerts(status) 
    WHERE status IN ('new', 'acknowledged');
CREATE INDEX idx_alerts_user ON marketplace_alerts(user_id);

-- Watchlist
CREATE TABLE marketplace_watchlist (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id),
    reason     TEXT NOT NULL,
    added_by   UUID NOT NULL REFERENCES users(id),
    is_active  BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Alert-Aktionen (Admin):**

| Aktion | Effekt |
|---|---|
| **Acknowledge** | Status → 'acknowledged', Admin hat es gesehen |
| **Resolve** | Status → 'resolved', Problem wurde behoben |
| **False Positive** | Status → 'false_positive', kein echtes Problem |
| **User auf Watchlist** | User bekommt Flag, alle zukünftigen Trades werden extra geloggt |
| **User Account Freeze** | Alle offenen Orders storniert, Withdrawals blockiert (🔴 Nur Super Admin) |

---

#### 3.5.13. Seite 10: Reconciliation (`/admin/marketplace/reconciliation`)

**Priorität: 🔴 Zwingend zum Launch**

Die kritischste Compliance-Seite. Zeigt ob das System finanziell konsistent ist.

**Layout:**

```
┌─────────────────────────────────────────────────────────────────┐
│  DAILY RECONCILIATION REPORT                    📅 2026-03-20   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  SYSTEMWEITE BILANZ                                        │ │
│  │                                                            │ │
│  │  SUM(wallet balances):    $1,234,567.89                    │ │
│  │  SUM(deposits):         + $1,500,000.00                    │ │
│  │  SUM(withdrawals):      - $  200,000.00                    │ │
│  │  SUM(purchases):        - $   65,432.11                    │ │
│  │  Expected balance:        $1,234,567.89                    │ │
│  │                                                            │ │
│  │  Delta:                   $0.00                            │ │
│  │  Status:                  ✅ BALANCED                       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  FEE-EINNAHMEN RECONCILIATION                              │ │
│  │                                                            │ │
│  │  SUM(trade_history.fee_cents):  $4,230.50                  │ │
│  │  SUM(fee wallet balance):       $4,230.50                  │ │
│  │  Delta:                         $0.00  ✅                   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  TOKEN-INTEGRITÄT (pro Asset)                              │ │
│  │                                                            │ │
│  │  Asset             │ Total Supply │ SUM(holdings) │ Delta  │ │
│  │  Berlin Apt 42     │ 1,000        │ 1,000         │ ✅ 0   │ │
│  │  Jakarta Tower     │ 500          │ 500           │ ✅ 0   │ │
│  │  Munich Villa      │ 2,000        │ 2,000         │ ✅ 0   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Historie: [📅 Letzter Check: 2026-03-20 03:00 UTC ✅]          │
│           [📅 2026-03-19 03:00 UTC ✅]                          │
│           [📅 2026-03-18 03:00 UTC ✅]                          │
│  [📥 Export Reconciliation Report (CSV)]                        │
└─────────────────────────────────────────────────────────────────┘
```

**3 Invarianten die geprüft werden:**

```sql
-- Invariante 1: Cash-Balance-Integrität
SELECT
    (SELECT COALESCE(SUM(balance_cents + held_balance_cents), 0) 
     FROM wallets WHERE wallet_type = 'cash') AS total_balances,
    (SELECT COALESCE(SUM(amount_cents), 0) FROM wallet_transactions 
     WHERE type = 'deposit' AND status = 'completed') AS total_deposits,
    (SELECT COALESCE(SUM(amount_cents), 0) FROM wallet_transactions 
     WHERE type = 'withdrawal' AND status = 'completed') AS total_withdrawals,
    (SELECT COALESCE(SUM(total_cents), 0) FROM orders 
     WHERE status = 'completed') AS total_purchases;
-- Erwartung: total_balances = total_deposits - total_withdrawals - total_purchases

-- Invariante 2: Token-Integrität (pro Asset)
SELECT a.id, a.title, a.tokens_total,
       COALESCE(SUM(i.tokens_owned + i.held_tokens), 0) AS sum_holdings,
       a.tokens_total - COALESCE(SUM(i.tokens_owned + i.held_tokens), 0) AS delta
FROM assets a
LEFT JOIN investments i ON a.id = i.asset_id
GROUP BY a.id, a.title, a.tokens_total
HAVING a.tokens_total != COALESCE(SUM(i.tokens_owned + i.held_tokens), 0);
-- Erwartung: Kein Ergebnis (= alle Deltas sind 0)

-- Invariante 3: Fee-Einnahmen-Integrität
SELECT
    (SELECT COALESCE(SUM(fee_cents), 0) FROM trade_history) AS total_fees_earned,
    (SELECT COALESCE(SUM(balance_cents), 0) FROM wallets 
     WHERE wallet_type = 'fee' OR user_id = 'SYSTEM_FEE_ACCOUNT') AS fee_balance;
-- Erwartung: total_fees_earned = fee_balance (nach Abzug ausgezahlter Fees)
```

---

#### 3.5.14. Seite 11: Compliance & OJK (`/admin/marketplace/compliance`)

**Priorität: 🟡 Week 4**

Generiert die von der OJK (Indonesische Finanzaufsicht) geforderten Reports und gibt Zugang zu Travel-Rule-Daten.

**Features:**

| Feature | Beschreibung | Export-Format |
|---|---|---|
| **OJK Quarterly Report** | Handelsvolumen, Nutzeranzahl, KYC-Rate, Incidents | PDF + XLSX |
| **Travel-Rule-Export** | Alle Trades mit Sender/Empfänger-Identität (Name, ID-Typ, ID-Nummer) | CSV |
| **User Tax Reports** | Steuer-Reports für Investoren (FIFO-Berechnung) | CSV + PDF |
| **Segregated Account Report** | SUM(wallets) vs. tatsächlicher Bankkontostand (manueller Input) | PDF |
| **AML Suspicious Activity Report** | Alle Alerts mit Status 'acknowledged' oder 'resolved' | PDF |
| **Dividend Report** | Alle ausgeschütteten Dividenden pro Asset | CSV |

**API-Endpoints:**

```
GET /api/admin/marketplace/compliance/ojk-report?quarter=2026-Q1
GET /api/admin/marketplace/compliance/travel-rule?from=...&to=...&format=csv
GET /api/admin/marketplace/compliance/tax-report?user_id=...&year=2026
GET /api/admin/marketplace/compliance/aml-report?from=...&to=...
```

---

#### 3.5.15. Seite 12: Marketplace Settings (`/admin/marketplace/settings`)

**Priorität: 🟡 Week 2**

Globale Konfiguration des Marktplatzes. Einige Settings wirken sofort (via Redis), andere erfordern einen Server-Restart.

**Konfigurierbare Parameter:**

| Setting | Default | Typ | Sofort wirksam? |
|---|---|---|---|
| **Trading Aktiv** (Kill-Switch) | `true` | Toggle | ✅ Ja (Redis Flag) |
| **Max Order Value (ohne Review)** | $50,000 | Dollar-Input | ✅ Ja |
| **Max Konzentration (%)** | 80% | Slider | ✅ Ja |
| **Großorder-Schwelle (%)** | 20% | Slider | ✅ Ja |
| **Min Order Value** | $10.00 | Dollar-Input | ✅ Ja |
| **Max offene Orders pro User/Asset** | 10 | Number-Input | ✅ Ja |
| **Limit-Order Default-Expiry** | 90 Tage | Number-Input | ✅ Ja |
| **P2P-Offer Expiry** | 48 Stunden | Number-Input | ✅ Ja |
| **Circuit Breaker: 7-Tage-Limit** | ±25% | Percentage-Input | ✅ Ja |
| **Circuit Breaker: 30-Tage-Limit** | ±40% | Percentage-Input | ✅ Ja |
| **Settlement-Batch-Intervall** | 24 Stunden | Dropdown (1h/6h/12h/24h) | ⚠️ Nächster Run |
| **Rate-Limit: Orders/Min/User** | 10 | Number-Input | ✅ Ja |
| **Rate-Limit: WebSocket Conn/IP** | 5 | Number-Input | ⚠️ Nächster Reconnect |

**Kill-Switch (Trading Notfall-Stop):**

```
┌───────────────────────────────────────────────────┐
│  ⚠️ TRADING KILL-SWITCH                           │
│                                                   │
│  Aktueller Status: 🟢 TRADING AKTIV               │
│                                                   │
│  [🔴 TRADING SOFORT STOPPEN]                       │
│                                                   │
│  Effekt: Alle neuen Orders werden abgelehnt.       │
│  Bestehende Orders bleiben im Orderbook.           │
│  Matching-Engine wird pausiert.                    │
│  Withdrawals bleiben aktiv.                        │
│                                                   │
│  ⚠️ Nur Super Admins können Trading reaktivieren.  │
└───────────────────────────────────────────────────┘
```

**Implementierung (Redis-basiert für Sofortwirkung):**

```rust
// Kill-Switch Check in der Order-Submission Route:
async fn check_trading_enabled(redis: &RedisPool) -> Result<(), AppError> {
    let enabled: Option<String> = redis.get("marketplace:trading_enabled").await?;
    if enabled.as_deref() == Some("false") {
        return Err(AppError::ServiceUnavailable(
            "Trading is currently paused by an administrator".into()
        ));
    }
    Ok(())
}

// Kill-Switch Toggle (Admin-API):
async fn toggle_trading(
    State(state): State<AppState>,
    session: AdminSession,  // Muss marketplace.manage haben
    Json(body): Json<ToggleTradingRequest>,
) -> Result<Json<Value>, AppError> {
    // NUR Super Admins dürfen Trading stoppen/starten
    if !session.is_super_admin() {
        return Err(AppError::Forbidden("Only super admins can toggle trading".into()));
    }

    let redis = &state.redis;
    redis.set("marketplace:trading_enabled", if body.enabled { "true" } else { "false" }).await?;

    // Audit-Log (KRITISCH!)
    audit_log(&state.pool, AuditEntry {
        actor_id: session.user_id,
        action: if body.enabled { "marketplace.trading.enabled" } else { "marketplace.trading.disabled" },
        details: body.reason.clone(),
        ..Default::default()
    }).await?;

    // Notification an alle Admins
    tracing::warn!(
        admin_id = %session.user_id,
        enabled = body.enabled,
        reason = %body.reason.unwrap_or_default(),
        "TRADING KILL-SWITCH TOGGLED"
    );

    Ok(Json(json!({ "trading_enabled": body.enabled })))
}
```

---

#### 3.5.16. Datenfluss-Diagramm: Admin-Seiten ↔ Backend ↔ Datenbanken

```
┌──────────────────────────────────────────────────────────────────────┐
│  ADMIN DASHBOARD DATENFLUSS                                         │
│                                                                     │
│  Browser (Admin)          Rust Backend (Axum)       Datenbanken      │
│  ─────────────────       ──────────────────       ──────────────     │
│                                                                     │
│  Overview Page ──────→ GET /api/admin/marketplace/stats              │
│                        │                                            │
│                        ├─→ SELECT FROM trade_history ──→ Core DB    │
│                        ├─→ SELECT FROM market_orders ──→ Core DB    │
│                        └─→ ZCARD asks:* / bids:* ─────→ Redis       │
│                                                                     │
│  Trade History ──────→ GET /api/admin/marketplace/trades             │
│                        └─→ SELECT FROM trade_history                │
│                            JOIN users (buyer + seller)              │
│                            JOIN assets ───────────────→ Core DB     │
│                                                                     │
│  Approvals ──────────→ POST /api/admin/marketplace/orders/{id}/     │
│                        approve                                      │
│                        ├─→ UPDATE market_orders ──────→ Core DB     │
│                        ├─→ ZADD asks:|bids: ─────────→ Redis        │
│                        └─→ INSERT audit_logs ─────────→ Core DB     │
│                                                                     │
│  Fee Management ─────→ PUT /api/admin/marketplace/fees/platform     │
│                        ├─→ UPDATE fee_configurations ─→ Core DB     │
│                        └─→ DEL fee_cache:* ──────────→ Redis        │
│                                                                     │
│  Settings ───────────→ POST /api/admin/marketplace/settings/        │
│  (Kill-Switch)         toggle-trading                               │
│                        └─→ SET marketplace:trading_enabled ─→ Redis │
│                                                                     │
│  Reconciliation ─────→ GET /api/admin/marketplace/reconciliation    │
│                        ├─→ SUM(wallets.balance_cents) ─→ Core DB   │
│                        ├─→ SUM(deposits/withdrawals) ──→ Core DB   │
│                        └─→ SUM(investments.tokens) ────→ Core DB   │
│                                                                     │
│  Compliance ─────────→ GET /api/admin/marketplace/compliance/       │
│                        travel-rule                                   │
│                        └─→ SELECT FROM travel_rule_records          │
│                            JOIN users (buyer + seller) ─→ Core DB  │
└──────────────────────────────────────────────────────────────────────┘
```

---

#### 3.5.17. Datei-Architektur der neuen Admin-Seiten

```
frontend/platform/admin/
├── marketplace/
│   ├── index.html              # Overview & Monitoring Dashboard
│   ├── orderbook.html          # Live Orderbook (Admin-Ansicht)
│   ├── trades.html             # Trade History + Export
│   ├── orders.html             # Open Orders Management
│   ├── approvals.html          # Pending Large-Order Approvals
│   ├── fees.html               # Fee Management (3 Tabs)
│   ├── p2p.html                # P2P Offers Oversight
│   ├── analytics.html          # Analytics & Charts
│   ├── alerts.html             # Alerts & Watchlist
│   ├── reconciliation.html     # Daily Balance Reconciliation
│   ├── compliance.html         # OJK Compliance & Reports
│   └── settings.html           # Marketplace Configuration
├── static/
│   ├── css/
│   │   └── admin-marketplace.css  # Shared Styles für alle 12 Seiten
│   └── js/
│       ├── admin-marketplace-overview.js
│       ├── admin-marketplace-orderbook.js
│       ├── admin-marketplace-trades.js
│       ├── admin-marketplace-orders.js
│       ├── admin-marketplace-approvals.js
│       ├── admin-marketplace-fees.js
│       ├── admin-marketplace-p2p.js
│       ├── admin-marketplace-analytics.js
│       ├── admin-marketplace-alerts.js
│       ├── admin-marketplace-reconciliation.js
│       ├── admin-marketplace-compliance.js
│       └── admin-marketplace-settings.js
```

---

#### 3.5.18. Sicherheits-Checkliste (Admin Dashboard)

| # | Aufgabe | Status | Verantwortlich |
|---|---|---|---|
| 1 | 3 neue Permissions (`marketplace.view/manage/compliance`) in Roles-System | ❌ | Backend |
| 2 | `PAGE_PERMISSION_MAP` in `admin-permission-guard.js` um 12 Einträge erweitert | ❌ | Frontend |
| 3 | Admin-Sidebar um Marketplace-Sektion erweitert | ❌ | Frontend |
| 4 | Kill-Switch nur für Super Admins zugänglich | ❌ | Backend + Frontend |
| 5 | Alle Admin-API-Endpoints prüfen Admin-Session + Permission | ❌ | Backend |
| 6 | PII-Masking auf User-Daten in Trade-History (wenn kein `pii.view`) | ❌ | Frontend |
| 7 | Audit-Logs für jede Admin-Aktion (Approve, Reject, Fee-Change, Kill-Switch) | ❌ | Backend |
| 8 | Reconciliation-Cron-Job läuft täglich und speichert Ergebnisse | ❌ | Backend + DevOps |
| 9 | Export-Endpoints haben Rate-Limits (max 5/Stunde) | ❌ | Backend |
| 10 | Keine `innerHTML` mit User-Daten in Admin-Seiten (XSS-Schutz) | ❌ | Frontend |
| 11 | Admin-Cancel von Orders: Bestätigungs-Dialog + Grund-Pflichtfeld | ❌ | Frontend |
| 12 | Compliance-Exports sind verschlüsselt und nur per Auth downloadbar | ❌ | Backend |

---

#### 3.5.19. Zusammenfassung: Wochenplan für Admin-Dashboard

```
Woche 1 (LAUNCH):  Overview Dashboard + Reconciliation
                    Trade History (mit Server-Side Pagination + Export)
                    Open Orders + Admin-Cancel
                    Pending Approvals (Approve/Reject Flow)
                    → 4 Seiten = MVP Admin-Bereich

Woche 2:            Fee Management (3 Tabs: Platform, Asset, Promotions)
                    P2P Offers Oversight
                    Marketplace Settings (Kill-Switch, Limits)
                    Live Orderbook (Admin-Ansicht mit User-IDs)
                    → 8 Seiten kumulativ

Woche 3:            Analytics & Charts (Volume, Top-Trader, Fee-Revenue)
                    Alerts & Watchlist + Alert-Schema (Migration 054)
                    → 10 Seiten kumulativ

Woche 4:            Compliance & OJK (Reports, Travel-Rule, Tax)
                    Polishing + Cross-Page Navigation
                    → 12 Seiten = Vollständiger Admin-Bereich
```

> **Die goldene Regel:** Die Reconciliation-Seite ist die wichtigste Admin-Seite im gesamten System. Wenn dort ein Delta ≠ $0.00 erscheint, hat das System einen Fehler – und dieser Fehler betrifft echtes Geld. Diese Seite muss am ersten Tag live sein, nicht in "Week 4".

## 4. Datenbank-Erweiterungen (PostgreSQL & Redis) – Vollständige Referenz

> **Kontext:** Diese Sektion dient als zentrale Referenz für alle Datenbank-Änderungen, die der Marketplace erfordert. Die Implementierungsdetails (wer, wann, wie) stehen in Sektion 3.3 (DevOps) und 3.1 (Backend). Hier steht die **vollständige Schema-Dokumentation**.

---

### 4.1. Redis: Datenstrukturen & Keys

Redis ist der **Speed-Layer** – er speichert alle Daten, die in Millisekunden gelesen werden müssen. PostgreSQL bleibt die **Truth Source** – bei Redis-Crash wird alles aus PostgreSQL rekonstruiert.

#### Vollständiger Redis-Key-Katalog

| Key-Pattern | Typ | TTL | Beschreibung | Rebuild-Quelle |
|---|---|---|---|---|
| `bids:{asset_id}` | Sorted Set | ∞ (persistent) | Kauforders: Score = `price_cents`, Member = `order_id` | `market_orders WHERE side='buy' AND status IN ('open','partially_filled')` |
| `asks:{asset_id}` | Sorted Set | ∞ (persistent) | Verkaufsorders: Score = `price_cents`, Member = `order_id` | `market_orders WHERE side='sell' AND status IN ('open','partially_filled')` |
| `order:{order_id}` | Hash | ∞ (persistent) | Order-Details: `user_id`, `qty`, `qty_filled`, `price`, `side` | `market_orders WHERE id = order_id` |
| `ticker:{asset_id}` | Hash | 300s | Letzter Preis, 24h-Änderung, 24h-Volumen | `trade_history` aggregiert |
| `idempotency:{key}` | String | 3600s | Idempotency-Lock für Order-Submissions | Nicht rekonstruierbar (ephemeral) |
| `trading_session:{user_id}` | String | 900s (15 Min) | 2FA Step-Up Session (Trading freigeschaltet) | Nicht rekonstruierbar (User muss 2FA erneut eingeben) |
| `rate_limit:orders:{user_id}` | String (Counter) | 60s | Anzahl Orders pro Minute pro User | Nicht rekonstruierbar (ephemeral) |
| `marketplace:trading_enabled` | String | ∞ (persistent) | Kill-Switch: "true" / "false" | Manuell setzen |
| `fee_cache:{asset_id}` | Hash | 300s | Cached Fee-Konfiguration für ein Asset | `fee_configurations` + `fee_promotions` |
| `marketplace:config` | Hash | ∞ (persistent) | Globale Marketplace-Settings (Limits, Schwellen) | `marketplace_settings` Tabelle oder manuell |

#### Redis Sorted-Set Operationen (Orderbook)

```
# Buy-Order einstellen (höchster Preis hat Priorität → Score = -price für DESC)
ZADD bids:{asset_id} {price_cents} {order_id}

# Sell-Order einstellen (niedrigster Preis hat Priorität → Score = price für ASC)
ZADD asks:{asset_id} {price_cents} {order_id}

# Top-20 Bids abfragen (höchster Preis zuerst)
ZREVRANGEBYSCORE bids:{asset_id} +inf -inf LIMIT 0 20 WITHSCORES

# Top-20 Asks abfragen (niedrigster Preis zuerst)
ZRANGEBYSCORE asks:{asset_id} -inf +inf LIMIT 0 20 WITHSCORES

# Bestes Bid (höchster Kaufpreis)
ZREVRANGEBYSCORE bids:{asset_id} +inf -inf LIMIT 0 1 WITHSCORES

# Bestes Ask (niedrigster Verkaufspreis)
ZRANGEBYSCORE asks:{asset_id} -inf +inf LIMIT 0 1 WITHSCORES

# Order stornieren
ZREM bids:{asset_id} {order_id}
ZREM asks:{asset_id} {order_id}
DEL order:{order_id}

# Orderbook-Tiefe (Anzahl Orders pro Seite)
ZCARD bids:{asset_id}
ZCARD asks:{asset_id}
```

#### Redis-Rebuild nach Crash

```sql
-- Dieses Query liefert alle aktiven Orders zum Rebuild des Redis-Orderbooks:
SELECT id, asset_id, side, price_cents, quantity, quantity_filled, user_id
FROM market_orders
WHERE status IN ('open', 'partially_filled')
ORDER BY created_at ASC;  -- FIFO: älteste Order zuerst (Time-Priority)
```

```rust
// Rust: Rebuild-Funktion (aufgerufen via Admin-API oder beim Server-Start)
async fn rebuild_orderbook(pool: &PgPool, redis: &RedisPool) -> Result<(), AppError> {
    let orders = sqlx::query_as!(MarketOrder,
        "SELECT * FROM market_orders WHERE status IN ('open', 'partially_filled')"
    ).fetch_all(pool).await?;

    for order in orders {
        let key = if order.side == "buy" {
            format!("bids:{}", order.asset_id)
        } else {
            format!("asks:{}", order.asset_id)
        };
        
        // ZADD: Score = price_cents, Member = order_id
        redis.zadd(&key, order.price_cents as f64, order.id.to_string()).await?;
        
        // Order-Details als Hash
        redis.hset_multiple(format!("order:{}", order.id), &[
            ("user_id", order.user_id.to_string()),
            ("qty", order.quantity.to_string()),
            ("qty_filled", order.quantity_filled.to_string()),
            ("price", order.price_cents.to_string()),
            ("side", order.side.clone()),
        ]).await?;
    }

    tracing::info!(count = orders.len(), "Orderbook rebuilt from PostgreSQL");
    Ok(())
}
```

---

### 4.2. PostgreSQL: Neue Marketplace-Tabellen

#### Migration 050: `market_orders` (Offene Orders)

```sql
-- database/050_marketplace_orders.sql
-- Alle Limit-Orders im System (offen, gefüllt, storniert)

CREATE TABLE market_orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    side            VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type      VARCHAR(10) NOT NULL DEFAULT 'limit' 
                    CHECK (order_type IN ('limit', 'market')),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    quantity_filled INTEGER NOT NULL DEFAULT 0 CHECK (quantity_filled >= 0),
    status          VARCHAR(20) NOT NULL DEFAULT 'open' 
                    CHECK (status IN (
                        'open',              -- Im Orderbook, wartet auf Match
                        'partially_filled',  -- Teilweise ausgeführt
                        'filled',            -- Vollständig ausgeführt
                        'cancelled',         -- Vom Nutzer storniert
                        'admin_cancelled',   -- Vom Admin storniert
                        'expired',           -- Abgelaufen (TTL)
                        'pending_review',    -- Großorder wartet auf Admin-Genehmigung
                        'rejected'           -- Großorder vom Admin abgelehnt
                    )),
    idempotency_key UUID UNIQUE,             -- Verhindert Doppel-Submissions
    cancel_reason   TEXT,                     -- Stornierungsgrund (Admin oder System)
    expires_at      TIMESTAMPTZ,             -- Ablaufzeitpunkt (Default: created_at + 90 Tage)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraint: filled darf nie > quantity sein
    CONSTRAINT chk_filled_lte_qty CHECK (quantity_filled <= quantity)
);

-- Performance-Indexes
CREATE INDEX idx_orders_asset_status ON market_orders(asset_id, status) 
    WHERE status IN ('open', 'partially_filled');
CREATE INDEX idx_orders_user ON market_orders(user_id, created_at DESC);
CREATE INDEX idx_orders_expiry ON market_orders(expires_at) 
    WHERE status = 'open' AND expires_at IS NOT NULL;
CREATE INDEX idx_orders_pending ON market_orders(status) 
    WHERE status = 'pending_review';
```

#### Migration 051: `trade_history` (Ausgeführte Trades)

```sql
-- database/051_trade_history.sql
-- Immutable Log aller ausgeführten Trades (wird NIEMALS aktualisiert oder gelöscht)

CREATE TABLE trade_history (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    buy_order_id    UUID NOT NULL REFERENCES market_orders(id),
    sell_order_id   UUID NOT NULL REFERENCES market_orders(id),
    buyer_user_id   UUID NOT NULL REFERENCES users(id),
    seller_user_id  UUID NOT NULL REFERENCES users(id),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    total_cents     BIGINT GENERATED ALWAYS AS (price_cents * quantity) STORED,
    fee_cents       BIGINT NOT NULL DEFAULT 0 CHECK (fee_cents >= 0),
    fee_bps         INTEGER NOT NULL DEFAULT 0,        -- Fee in Basis Points (zur Nachvollziehbarkeit)
    
    -- On-Chain Settlement Status
    on_chain_status VARCHAR(15) NOT NULL DEFAULT 'pending'
                    CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed')),
    on_chain_tx_hash VARCHAR(66),                       -- 0x + 64 hex chars
    on_chain_batch_id UUID,                             -- Referenz zur Settlement-Batch
    
    executed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance-Indexes
CREATE INDEX idx_trade_asset_time ON trade_history(asset_id, executed_at DESC);
CREATE INDEX idx_trade_buyer ON trade_history(buyer_user_id, executed_at DESC);
CREATE INDEX idx_trade_seller ON trade_history(seller_user_id, executed_at DESC);
CREATE INDEX idx_trade_onchain ON trade_history(on_chain_status) 
    WHERE on_chain_status IN ('pending', 'submitted');

-- Hinweis: Diese Tabelle wird in Sektion 4.4 zur TimescaleDB Hypertable konvertiert
```

#### Migration 052: `p2p_offers` (Private Direktangebote)

```sql
-- database/052_p2p_offers.sql
-- Peer-to-Peer (Over-the-Counter) Direktangebote zwischen Nutzern

CREATE TABLE p2p_offers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id          UUID NOT NULL REFERENCES assets(id),
    maker_user_id     UUID NOT NULL REFERENCES users(id),
    taker_user_id     UUID NOT NULL REFERENCES users(id),
    side              VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    price_cents       BIGINT NOT NULL CHECK (price_cents > 0),
    quantity          INTEGER NOT NULL CHECK (quantity > 0),
    message           TEXT,                            -- Optionale Nachricht an den Taker
    status            VARCHAR(15) NOT NULL DEFAULT 'pending'
                      CHECK (status IN (
                          'pending',      -- Wartet auf Antwort
                          'accepted',     -- Angenommen → wird ausgeführt
                          'declined',     -- Abgelehnt
                          'expired',      -- Nicht beantwortet innerhalb TTL
                          'countered',    -- Gegenangebot erstellt
                          'cancelled',    -- Vom Maker zurückgezogen
                          'admin_cancelled' -- Vom Admin storniert
                      )),
    parent_offer_id   UUID REFERENCES p2p_offers(id),  -- Verweist auf Vorgänger bei Gegenangeboten
    trade_id          UUID REFERENCES trade_history(id),-- Verweist auf den ausgeführten Trade
    expires_at        TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '48 hours'),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraint: Maker und Taker dürfen nicht gleich sein
    CONSTRAINT chk_no_self_offer CHECK (maker_user_id != taker_user_id)
);

CREATE INDEX idx_p2p_taker ON p2p_offers(taker_user_id, status) 
    WHERE status = 'pending';
CREATE INDEX idx_p2p_asset ON p2p_offers(asset_id, created_at DESC);
CREATE INDEX idx_p2p_expiry ON p2p_offers(expires_at) 
    WHERE status = 'pending';
```

#### Migration 053: `fee_configurations` & `fee_promotions` (Gebühren-System)

```sql
-- database/053_fee_configuration.sql
-- 4-Ebenen Fee-Hierarchie: Promotion > Developer Deal > Asset > Platform Default

-- Platform- und Asset-spezifische Gebühren
CREATE TABLE fee_configurations (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('platform', 'asset', 'developer')),
    asset_id      UUID REFERENCES assets(id),           -- NULL für scope='platform'
    developer_id  UUID REFERENCES users(id),             -- NULL außer für scope='developer'
    taker_fee_bps INTEGER NOT NULL DEFAULT 500           -- 500 = 5.00%
                  CHECK (taker_fee_bps >= 0 AND taker_fee_bps <= 1000),
    maker_fee_bps INTEGER NOT NULL DEFAULT 0             -- 0 = 0.00%
                  CHECK (maker_fee_bps >= 0 AND maker_fee_bps <= 1000),
    is_active     BOOLEAN NOT NULL DEFAULT true,
    reason        TEXT,                                   -- Grund für Override
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Nur ein aktiver Eintrag pro Scope+Referenz
    CONSTRAINT uq_fee_scope UNIQUE (scope, asset_id, developer_id, is_active)
);

-- Zeitlich begrenzte Promotions (höchste Priorität im Fee-Lookup)
CREATE TABLE fee_promotions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          VARCHAR(100) NOT NULL,
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('global', 'asset')),
    asset_id      UUID REFERENCES assets(id),            -- NULL für scope='global'
    taker_fee_bps INTEGER NOT NULL CHECK (taker_fee_bps >= 0),
    maker_fee_bps INTEGER NOT NULL CHECK (maker_fee_bps >= 0),
    starts_at     TIMESTAMPTZ NOT NULL,
    ends_at       TIMESTAMPTZ NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_promo_dates CHECK (ends_at > starts_at)
);

CREATE INDEX idx_promo_active ON fee_promotions(starts_at, ends_at) 
    WHERE is_active = true;
```

#### Migration 054: `marketplace_alerts` & `marketplace_watchlist` (Fraud Detection)

```sql
-- database/054_marketplace_alerts.sql
-- Automatisch und manuell generierte Alerts für verdächtige Aktivitäten

CREATE TABLE marketplace_alerts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type   VARCHAR(50) NOT NULL,
    severity     VARCHAR(15) NOT NULL DEFAULT 'warning'
                 CHECK (severity IN ('info', 'warning', 'critical')),
    asset_id     UUID REFERENCES assets(id),
    user_id      UUID REFERENCES users(id),
    trade_id     UUID REFERENCES trade_history(id),
    message      TEXT NOT NULL,
    metadata     JSONB,
    status       VARCHAR(15) NOT NULL DEFAULT 'new'
                 CHECK (status IN ('new', 'acknowledged', 'resolved', 'false_positive')),
    resolved_by  UUID REFERENCES users(id),
    resolved_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_status ON marketplace_alerts(status) 
    WHERE status IN ('new', 'acknowledged');
CREATE INDEX idx_alerts_severity ON marketplace_alerts(severity, created_at DESC)
    WHERE status = 'new';
CREATE INDEX idx_alerts_user ON marketplace_alerts(user_id);

-- Admin-Watchlist für verdächtige User
CREATE TABLE marketplace_watchlist (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id),
    reason     TEXT NOT NULL,
    added_by   UUID NOT NULL REFERENCES users(id),
    is_active  BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_watchlist_user ON marketplace_watchlist(user_id) 
    WHERE is_active = true;
```

#### Migration 055: `reconciliation_reports` (Tägliche Bilanz-Checks)

```sql
-- database/055_reconciliation_reports.sql
-- Speichert die Ergebnisse der täglichen Reconciliation-Prüfung

CREATE TABLE reconciliation_reports (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date         DATE NOT NULL UNIQUE,
    
    -- Cash-Bilanz
    total_wallet_cents  BIGINT NOT NULL,    -- SUM(wallets.balance_cents + held_balance_cents)
    total_deposits_cents BIGINT NOT NULL,   -- SUM(deposits)
    total_withdrawals_cents BIGINT NOT NULL,-- SUM(withdrawals)
    total_purchases_cents BIGINT NOT NULL,  -- SUM(primary market purchases)
    cash_delta_cents    BIGINT NOT NULL,    -- Soll - Ist (muss 0 sein!)
    
    -- Fee-Bilanz
    total_fees_earned_cents BIGINT NOT NULL,
    fee_wallet_cents    BIGINT NOT NULL,
    fee_delta_cents     BIGINT NOT NULL,    -- Soll - Ist (muss 0 sein!)
    
    -- Token-Integrität
    token_mismatches    INTEGER NOT NULL DEFAULT 0, -- Anzahl Assets mit Delta ≠ 0
    token_details       JSONB,                      -- Details pro Asset bei Mismatch
    
    -- Status
    status              VARCHAR(15) NOT NULL DEFAULT 'pass'
                        CHECK (status IN ('pass', 'warning', 'fail')),
    notes               TEXT,
    
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

### 4.3. Bestehende Tabellen: Notwendige Erweiterungen

Zwei bestehende Tabellen müssen für den Marketplace angepasst werden:

#### `wallets` – Neues Feld: `held_balance_cents`

```sql
-- database/050b_alter_wallets_held_balance.sql
ALTER TABLE wallets 
    ADD COLUMN held_balance_cents BIGINT NOT NULL DEFAULT 0 
    CHECK (held_balance_cents >= 0);

-- Constraint: Ein User kann nie mehr "halten" als er hat
ALTER TABLE wallets 
    ADD CONSTRAINT chk_held_lte_balance 
    CHECK (held_balance_cents <= balance_cents);

COMMENT ON COLUMN wallets.held_balance_cents IS 
    'Betrag der durch offene Buy-Orders geblockt ist. Wird bei Order-Placement erhöht, bei Cancel/Fill verringert.';
```

#### `investments` – Neues Feld: `held_tokens`

```sql
-- database/050c_alter_investments_held_tokens.sql
ALTER TABLE investments 
    ADD COLUMN held_tokens INTEGER NOT NULL DEFAULT 0 
    CHECK (held_tokens >= 0);

ALTER TABLE investments 
    ADD CONSTRAINT chk_held_tokens_lte_owned 
    CHECK (held_tokens <= tokens_owned);

COMMENT ON COLUMN investments.held_tokens IS 
    'Tokens die durch offene Sell-Orders geblockt sind. Wird bei Order-Placement erhöht, bei Cancel/Fill verringert.';
```

---

### 4.4. TimescaleDB: Hypertables für Candlestick-Charts

```sql
-- database/056_timescaledb_setup.sql
-- Erfordert: CREATE EXTENSION IF NOT EXISTS timescaledb;

-- trade_history in eine Hypertable konvertieren (partitioniert nach executed_at)
SELECT create_hypertable('trade_history', 'executed_at', 
    migrate_data => true, 
    chunk_time_interval => INTERVAL '7 days'
);

-- Continuous Aggregates: Automatisch berechnete Candlestick-Daten

-- 1-Minuten-Kerzen
CREATE MATERIALIZED VIEW candles_1m
WITH (timescaledb.continuous) AS
SELECT 
    asset_id,
    time_bucket('1 minute', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 minute', executed_at);

-- 1-Stunden-Kerzen
CREATE MATERIALIZED VIEW candles_1h
WITH (timescaledb.continuous) AS
SELECT 
    asset_id,
    time_bucket('1 hour', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 hour', executed_at);

-- 1-Tages-Kerzen
CREATE MATERIALIZED VIEW candles_1d
WITH (timescaledb.continuous) AS
SELECT 
    asset_id,
    time_bucket('1 day', executed_at) AS bucket,
    first(price_cents, executed_at) AS open,
    max(price_cents) AS high,
    min(price_cents) AS low,
    last(price_cents, executed_at) AS close,
    sum(quantity) AS volume,
    count(*) AS trade_count
FROM trade_history
GROUP BY asset_id, time_bucket('1 day', executed_at);

-- Refresh-Policies (automatischer Hintergrund-Refresh)
SELECT add_continuous_aggregate_policy('candles_1m',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');

SELECT add_continuous_aggregate_policy('candles_1h',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

SELECT add_continuous_aggregate_policy('candles_1d',
    start_offset => INTERVAL '30 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');
```

---

### 4.5. Entity-Relationship-Diagramm (Marketplace-Tabellen)

```
┌───────────────────────────────────────────────────────────────────────┐
│                    MARKETPLACE ER-DIAGRAMM                            │
│                                                                       │
│  ┌─────────┐         ┌────────────────┐         ┌─────────────┐      │
│  │  users   │────1:N──│ market_orders  │──N:1────│   assets    │      │
│  │─────────│         │────────────────│         │─────────────│      │
│  │ id (PK) │         │ id (PK)        │         │ id (PK)     │      │
│  │ email   │         │ user_id (FK)   │         │ title       │      │
│  │ ...     │         │ asset_id (FK)  │         │ tokens_total│      │
│  └────┬────┘         │ side           │         └──────┬──────┘      │
│       │              │ price_cents    │                │             │
│       │              │ quantity       │                │             │
│       │              │ status         │                │             │
│       │              └───────┬────────┘                │             │
│       │                      │                         │             │
│       │              ┌───────▼────────┐                │             │
│       │              │ trade_history  │                │             │
│       ├──────1:N─────│────────────────│──N:1───────────┘             │
│       │              │ id (PK)        │                              │
│       │              │ buy_order_id   │──FK──→ market_orders         │
│       │              │ sell_order_id  │──FK──→ market_orders         │
│       │              │ buyer_user_id  │──FK──→ users                 │
│       │              │ seller_user_id │──FK──→ users                 │
│       │              │ price_cents    │                              │
│       │              │ quantity       │                              │
│       │              │ total_cents    │ (GENERATED)                  │
│       │              │ fee_cents      │                              │
│       │              │ on_chain_status│                              │
│       │              │ executed_at    │ ← TimescaleDB Hypertable     │
│       │              └───────┬────────┘                              │
│       │                      │                                       │
│       │              ┌───────▼────────┐                              │
│       │              │ candles_1m/1h  │ (Materialized Views)         │
│       │              │ candles_1d     │ (Continuous Aggregates)       │
│       │              └────────────────┘                              │
│       │                                                              │
│       │              ┌────────────────┐                              │
│       ├──────1:N─────│  p2p_offers    │──N:1──→ assets               │
│       │              │────────────────│                              │
│       │              │ maker_user_id  │──FK──→ users                 │
│       │              │ taker_user_id  │──FK──→ users                 │
│       │              │ parent_offer_id│──FK──→ p2p_offers (self-ref) │
│       │              │ trade_id       │──FK──→ trade_history         │
│       │              └────────────────┘                              │
│       │                                                              │
│       │              ┌────────────────┐                              │
│       ├──────1:N─────│   wallets      │ (BESTEHEND, ERWEITERT)       │
│       │              │ + held_balance │                              │
│       │              └────────────────┘                              │
│       │                                                              │
│       │              ┌────────────────┐                              │
│       ├──────1:N─────│  investments   │ (BESTEHEND, ERWEITERT)       │
│       │              │ + held_tokens  │                              │
│       │              └────────────────┘                              │
│       │                                                              │
│       │              ┌──────────────────────┐                        │
│       ├──────1:N─────│ marketplace_alerts   │                        │
│       │              │ marketplace_watchlist │                        │
│       │              └──────────────────────┘                        │
│       │                                                              │
│       │              ┌────────────────┐                              │
│       └──────1:N─────│fee_configurations│                            │
│                      │ fee_promotions  │                             │
│                      └────────────────┘                              │
│                                                                       │
│              ┌────────────────────────┐                               │
│              │ reconciliation_reports │ (Standalone, keine FKs)       │
│              └────────────────────────┘                               │
└───────────────────────────────────────────────────────────────────────┘
```

---

### 4.6. Migrations-Reihenfolge

Migrationen müssen in exakt dieser Reihenfolge ausgeführt werden (Abhängigkeiten beachten):

```
Phase 1 (vor Marketplace-Code):
  050b_alter_wallets_held_balance.sql      ← Ändert bestehende Tabelle
  050c_alter_investments_held_tokens.sql   ← Ändert bestehende Tabelle

Phase 2 (Marketplace-Tabellen):
  050_marketplace_orders.sql               ← market_orders (keine FKs auf neue Tabellen)
  051_trade_history.sql                    ← trade_history (FK auf market_orders)
  052_p2p_offers.sql                       ← p2p_offers (FK auf trade_history)
  053_fee_configuration.sql                ← fee_configurations + fee_promotions
  054_marketplace_alerts.sql               ← marketplace_alerts + watchlist (FK auf trade_history)
  055_reconciliation_reports.sql           ← reconciliation_reports (standalone)

Phase 3 (Blockchain-Tabellen):
  050d_alter_assets_blockchain.sql         ← contract_address, spv_entity, ipfs_cid auf assets
  057_user_wallets.sql                     ← user_wallets (Custody-Wallets)
  058_onchain_balances.sql                 ← onchain_balances (Blockchain-Cache)
  059_settlement_batches.sql               ← settlement_batches (Audit-Log)
  060_dividend_distributions.sql           ← dividend_distributions + dividend_payouts

Phase 4 (TimescaleDB – nach Daten-Migration): 
  061_timescaledb_setup.sql                ← Hypertable + Continuous Aggregates
```

> **⚠️ WICHTIG:** Die TimescaleDB-Extension muss VOR Migration 061 aktiviert werden:
> `CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;`
> Dies passiert in der Cloud SQL Konfiguration (Sektion 3.3.1).

---

### 4.7. Daten-Integritäts-Invarianten (MÜSSEN zu jeder Zeit gelten)

| # | Invariante | Prüf-Query | Was passiert bei Verletzung? |
|---|---|---|---|
| 1 | **Cash-Bilanz** | `SUM(balance + held) = SUM(deposits) - SUM(withdrawals) - SUM(purchases)` | 🔴 Trading sofort stoppen, manueller Audit |
| 2 | **Token-Bilanz** | `SUM(tokens_owned + held_tokens) = asset.tokens_total` für JEDES Asset | 🔴 Trading für das Asset stoppen |
| 3 | **Held ≤ Available** | `held_balance_cents <= balance_cents` für JEDE Wallet | 🔴 Alle Orders des Users stornieren |
| 4 | **Filled ≤ Quantity** | `quantity_filled <= quantity` für JEDE Order | 🔴 Order manuell korrigieren |
| 5 | **Fee-Bilanz** | `SUM(trade_history.fee_cents) = SUM(fee_wallet.balance)` | 🟡 Warnung, kein Trading-Stop |
| 6 | **Keine Self-Trades** | `buyer_user_id != seller_user_id` in JEDEM trade_history-Eintrag | 🟡 Alert generieren, Trades untersuchen |
| 7 | **Keine neg. Balances** | `balance_cents >= 0` UND `held_balance_cents >= 0` für ALLE Wallets | 🔴 Sofort-Alarm, alle Transaktionen des Users pausieren |
| 8 | **On-Chain Sync** | `SUM(onchain_balances.balance)` pro Asset = `totalSupply()` on-chain | 🟡 Event-Indexer Replay triggern |
| 9 | **Settlement vollständig** | Keine Trades mit `on_chain_status = 'pending'` älter als 48h | 🟡 Alert + manuelles Settlement triggern |
| 10 | **Wallet-Konsistenz** | Jeder KYC-verifizierte User hat genau 1 Eintrag in `user_wallets` | 🟡 Identity-Worker erneut ausführen |

> **Reconciliation-Job:** Invarianten 1-5 werden täglich automatisch geprüft (Sektion 3.3.7). Invarianten 6-7 sind als DB-Constraints (`CHECK`) implementiert und können DB-seitig nie verletzt werden. Invarianten 8-10 werden vom Blockchain Event-Indexer (Sektion 3.2.11) und Settlement-Worker (Sektion 3.2.3) überwacht.

---

## 5. Benötigtes Entwickler-Team (Hiring Plan)

Um diesen hochkomplexen Sekundärmarkt (Trading Engine) kombiniert mit der Blockchain-Infrastruktur sicher in Produktion zu bringen, benötigt POOOL ein hochspezialisiertes Team, das exakt auf den aktuellen Tech-Stack (Rust, Vanilla JS, GCP, Postgres) zugeschnitten ist.

**Team-Größe: 5-6 Personen (4 zwingend, 1-2 optional)**

---

### 5.0. Technical Project Manager / Tech Lead (Die Steuerung)

**Die wichtigste Position, die oft vergessen wird.** Diese Person steuert den gesamten Implementierungsprozess, priorisiert Tasks, koordiniert Abhängigkeiten zwischen den Entwicklern und stellt sicher, dass der Plan in der richtigen Reihenfolge abgearbeitet wird.

*   **Fokus:** Projekt-Koordination, Sprint-Planung, technische Entscheidungen, Risiko-Management, Third-Party-Account-Management
*   **Verantwortlichkeiten:**
    *   Implementierungsplan (Kapitel 6) in Sprints aufteilen und tracken
    *   Entscheidungen treffen wenn Blocked-Issues auftreten
    *   Third-Party-Accounts rechtzeitig einrichten lassen (siehe Kapitel 6.10)
    *   Code-Reviews koordinieren und Merge-Konflikte lösen
    *   Wöchentliches Status-Meeting mit dem Team
    *   Risiken frühzeitig erkennen (z.B. "Smart Contract Audit dauert 4 Wochen – müssen wir JETZT beauftragen")
*   **Muss können:** Verständnis für alle 5 Domänen (Backend, Frontend, Blockchain, DevOps, Admin), Erfahrung mit agiler Entwicklung, klare Kommunikation
*   **Masterplan-Referenz:** Kapitel 6 (Roadmap), 6.10 (Accounts), 6.12 (Status-Template)

**Lieferbare Meilensteine:**

| Woche | Deliverable |
|---|---|
| 1 | Sprint-Board aufgesetzt, Team-Onboarding abgeschlossen |
| 1 | Alle Third-Party-Accounts beantragt (Checkliste 6.10) |
| 4 | Smart Contract Audit beauftragt (⚠️ 4-6 Wochen Vorlauf!) |
| Laufend | Wöchentlicher Status-Report (Template 6.12) |

---

### 5.1. Senior Rust / Backend Engineer (Die Trading Engine & Core API)

Das Herzstück des Systems. Diese Person baut die Orderbuch-Logik, die Matching-Engine, die Settlement-Pipeline und alle Marketplace-APIs.

*   **Fokus:** Axum (Web-Framework), Tokio (Asynchrones Rust), SQLx, Redis (`redis-rs`), WebSockets
*   **Muss können:** Memory-Management, Concurrency (nebenläufige Prozesse), Erfahrung mit algorithmischen Matching-Engines und Database-Deadlock-Vermeidung
*   **Masterplan-Referenz:** Sektion 3.1 (vollständiger Implementation Guide, ~1.500 Zeilen)

**Lieferbare Meilensteine:**

| Woche | Deliverable | Referenz |
|---|---|---|
| 2-3 | Backend-Härtung: 2FA Step-Up, Withdrawal-Limits, Circuit Breaker | 3.1.1-3.1.3 |
| 4-5 | Order-Submission API + Redis Orderbook-Modul | 3.1.4-3.1.5 |
| 5-6 | Matching-Engine (Price-Time-Priority, Partial Fills) | 3.1.6 |
| 6-7 | Settlement-Pipeline (8-Step ACID Transaction) | 3.1.7 |
| 7-8 | WebSocket-Server + Fee-Engine + Remaining APIs | 3.1.8-3.1.11 |
| 8-9 | Admin-Marketplace APIs (alle `/api/admin/marketplace/*` Endpoints) | 3.5.4-3.5.15 |

> **Kritische Eigenschaft:** Diese Person muss `SELECT ... FOR UPDATE` und `BEGIN ... COMMIT` im Schlaf beherrschen. Jeder Bug in der Settlement-Funktion = echtes Geldproblem.

---

### 5.2. Smart Contract / Web3 Security Engineer (Die Tokenisierung)

Verantwortlich für den rechtlichen und kryptografischen Beweis der Anteile auf der Base-Blockchain.

*   **Fokus:** Solidity, ERC-3643 Standard (Security Tokens), Alloy (Rust-Brücke), Foundry (Testing)
*   **Muss können:** Fuzz-Testing, tiefe Kenntnisse in EVM-Sicherheitslücken (Reentrancy, Arithmetic Overflow), Verständnis für regulierte Security Tokens
*   **Masterplan-Referenz:** Sektion 3.2 (vollständiger Implementation Guide, ~2.500 Zeilen)

**Lieferbare Meilensteine:**

| Woche | Deliverable | Referenz |
|---|---|---|
| 4-5 | Foundry-Projekt + IdentityRegistry + PooolToken (ERC-3643) | 3.2.1-3.2.4 |
| 5-6 | Compliance-Module (ManualApprovalModule, CountryRestriction) | 3.2.5-3.2.6 |
| 6-7 | Foundry Tests (Unit + Fuzz, 10.000+ Runs) | 3.2.7 |
| 7 | Base Sepolia Testnet Deploy + Verification | 3.2.8 |
| 7-8 | Alloy (Rust) Integration + Settlement-Worker-Bridge | 3.2.9 |
| 8-9 | Gas-Optimierung + Freeze/Recovery-Mechanismen | 3.2.10-3.2.11 |

> **Arbeitet PARALLEL** zum Rust-Engineer ab Woche 4. Keine Abhängigkeit bis zur Integration in Woche 8.

---

### 5.3. Database & DevOps Engineer (Daten / Backups / Infrastruktur)

Da Trading-Systeme Hochverfügbarkeit und Millisekunden-Latenzen erfordern, ist ein dedizierter DevOps Engineer keine Option sondern Pflicht.

*   **Fokus:** PostgreSQL 16, TimescaleDB, Redis (Memorystore), Google Cloud Platform (GCP), PgBouncer, Cloud Run, Monitoring
*   **Muss können:** Skalierung von relationalen Datenbanken bei hoher Schreib-Last, Point-in-Time-Recovery, Read-Replica-Routing, und Verwaltung von zwei getrennten Cloud SQL Instanzen
*   **Masterplan-Referenz:** Sektion 3.3 (vollständiger Implementation Guide, ~730 Zeilen), Sektion 4 (DB-Schemas)

**Lieferbare Meilensteine:**

| Woche | Deliverable | Referenz |
|---|---|---|
| 1 | Cloud SQL (Core + Community) + Read-Replicas provisioniert | 3.3.1 |
| 1 | Redis Memorystore + PITR aktiviert + Backup-Strategie | 3.3.2, 3.3.4 |
| 1-2 | CI/CD Pipeline (GitHub Actions → Cloud Run) | 6.2 |
| 2 | Alle Marketplace-Migrationen ausgeführt (050-056) | 4.6 |
| 2-3 | Read-Replica Routing im Rust-Code (Dual-Pool) | 3.3.3 |
| 3 | Monitoring-Alerts + Health-Check Endpoint | 3.3.7 |
| 3-4 | TimescaleDB Hypertables + Continuous Aggregates | 3.3.5, 4.4 |
| Laufend | Täglicher Reconciliation-Job prüft Invarianten | 3.3.7, 4.7 |

---

### 5.4. Frontend / UI Engineer (Vanilla Web & Data Visualization)

Da das POOOL-Frontend absichtlich aus reinem HTML, CSS und Vanilla JS (ohne React/Vue) besteht, wird ein JavaScript-Purist benötigt, der **zwei große Bereiche** abdeckt:

1. **Investor-Facing Trading UI** (Sektion 3.4)
2. **Admin Dashboard – 12 neue Marketplace-Seiten** (Sektion 3.5)

*   **Fokus:** Vanilla JavaScript, CSS Grid/Flexbox, WebSockets, Charting-Libraries (`lightweight-charts.js`, Chart.js), DOM-Manipulation
*   **Muss können:** Direkte DOM-Manipulation performant umsetzen, SSR (MiniJinja) Templates einbinden, WebSocket-Connections mit Reconnect implementieren, und **kein `innerHTML` mit User-Daten verwenden** (XSS-Prävention)
*   **Masterplan-Referenz:** Sektion 3.4 (Trading UI, ~1.150 Zeilen), Sektion 3.5 (Admin Dashboard, ~890 Zeilen)

**Lieferbare Meilensteine:**

| Woche | Deliverable | Referenz |
|---|---|---|
| 6-7 | Event-Bus + WebSocket-Client + Candlestick-Chart | 3.4.2-3.4.4 |
| 7-8 | Orderbook-UI + Buy/Sell-Formular + Validierung | 3.4.5-3.4.6 |
| 8-9 | P2P-Offer Modals + 2FA Step-Up Modal | 3.4.7 |
| 8-9 | **Admin: Overview Dashboard + Reconciliation** | 3.5.4, 3.5.13 |
| 9-10 | **Admin: Trade History + Open Orders + Approvals** | 3.5.6-3.5.8 |
| 10-11 | **Admin: Fee Management + Settings (Kill-Switch)** | 3.5.9, 3.5.15 |
| 11-12 | **Admin: Analytics + Alerts + Compliance** | 3.5.11-3.5.14 |
| 12 | Responsive Design + Accessibility + Cross-Browser | 3.4.10, 3.4.12 |

> **Kritischer Hinweis:** Diese Person baut ~20 neue HTML-Seiten (8 Trading + 12 Admin). Das ist ein massiver Umfang. Ab Woche 8 arbeitet sie parallel an Trading-UI und Admin-Dashboard. Falls das Team Budget hat, ist eine **zweite Frontend-Person** für das Admin-Dashboard empfehlenswert.

---

### 5.5. QA & Testing Engineer (Optional, aber DRINGEND empfohlen)

Bei einem System, das Echtgeld (Fiat) und Immobilien-Tokens bewegt, ist rigoroses Testing keine Option.

*   **Fokus:** End-to-End (E2E) Testing (Playwright/Python), Load & Stress-Testing, Financial Test Suite (`sqlx::test`)
*   **Muss können:** Stresstests (Simulation von 5.000 Nutzern, die blitzschnell dasselbe Asset kaufen), Überprüfung von Race Conditions und Financial Fraud Simulation
*   **Masterplan-Referenz:** Sektion 1.12 (Test-Strategie), Phase 6 (Testing)

**Lieferbare Meilensteine:**

| Woche | Deliverable |
|---|---|
| 8-9 | Financial Unit Tests + Concurrent Trade Tests |
| 9-10 | E2E Tests (Playwright): Kompletter User-Journey |
| 10-11 | Load Test (100 User, 500 Orders/Min, 30 Min) |
| 11-12 | UAT + Bug-Fix Sprint |

---

### 5.6. Team-Interaktions-Matrix (Wer blockiert wen?)

```
                    PM    Rust   Web3   DevOps  Frontend  QA
PM (Tech Lead)      ─     ←→     ←→     ←→      ←→       ←→
Rust Backend        →      ─     →(W8)  ←(W1)   →(W6)    →(W10)
Web3 Engineer       →     ←(W8)   ─      ─       ─       →(W10)
DevOps Engineer     →     →(W1)   ─      ─       ─        ─
Frontend Engineer   →     ←(W6)   ─      ─       ─       →(W10)
QA Engineer         →     ←(W10) ←(W10)  ─      ←(W10)    ─
```

**Legende:**
- `→(W6)` = "liefert Output an, ab Woche 6"
- `←(W1)` = "erhält Input von, ab Woche 1"
- `←→` = bidirektionale Kommunikation

**Kritische Abhängigkeitskette:**
```
DevOps (W1) → Rust Backend (W2) → Frontend (W6) → QA (W10)
                    ↓
              Web3 (W4, parallel) → Integration (W8) → QA (W10)
```

---

### 5.7. Kosten-Schätzung (Monatlich, Remote-Team)

| Rolle | Erfahrung | Geschätzte Kosten (Remote, EUR/Monat) | Dauer |
|---|---|---|---|
| **Tech Lead / PM** | 5+ Jahre | €4.000 - €7.000 | 3 Monate |
| **Rust Backend Engineer** | 3+ Jahre Rust | €6.000 - €10.000 | 3 Monate |
| **Web3/Solidity Engineer** | 2+ Jahre Solidity | €5.000 - €9.000 | 2 Monate (W4-W10) |
| **DevOps Engineer** | 3+ Jahre GCP/AWS | €4.000 - €7.000 | 3 Monate |
| **Frontend Engineer** | 3+ Jahre Vanilla JS | €4.000 - €7.000 | 2.5 Monate (W4-W13) |
| **QA Engineer** (optional) | 2+ Jahre | €3.000 - €5.000 | 1.5 Monate (W8-W13) |
| **Smart Contract Audit** | – | €5.000 - €30.000 (einmalig) | – |
| **Infra-Kosten (GCP)** | – | €65 - €85/Monat | Laufend |
| | | | |
| **Total (ohne Audit)** | | **€26.000 - €45.000/Monat** | 3 Monate |
| **Total Projekt** | | **€78.000 - €145.000** | inkl. Audit |

> **Hinweis:** Diese Schätzung basiert auf Remote-Entwicklern (Osteuropa, Südamerika, Südostasien). Senior-Freelancer in Westeuropa kosten 2-3x mehr. Die Ranges berücksichtigen Junior-Senior-Spreads.

---

## 6. Implementierungsplan: Schritt-für-Schritt Roadmap

> **Die richtige Reihenfolge ist NICHT Frontend → Backend → Server.** Der Industrie-Standard ist: **Infrastruktur → Backend-Core → Marketplace Engine → Smart Contracts → Frontend → Integration → Testing → Launch.** Frontend kann erst gebaut werden, wenn die APIs existieren. Smart Contracts können erst integriert werden, wenn das Settlement steht. Alles andere führt zu doppelter Arbeit.

---

### 6.1. Phasen-Übersicht

```
┌──────────────────────────────────────────────────────────────────────┐
│            IMPLEMENTIERUNGSPLAN (chronologisch)                      │
│                                                                      │
│  Woche 1-2    ┌────────────────────────────────┐                    │
│  PHASE 0      │ Infrastruktur & Accounts        │ ← DevOps + PM     │
│               │ (GCP, DBs, Redis, CI/CD)        │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 2-4    ┌──────────▼─────────────────────┐                    │
│  PHASE 1      │ Backend Core (DB, Auth, 2FA)    │ ← Rust Dev         │
│               │ (Härtung, Limits, Reconcil.)    │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 4-8    ┌──────────▼─────────────────────┐                    │
│  PHASE 2      │ Marketplace Engine              │ ← Rust Dev         │
│               │ (Redis, Matching, Settlement)   │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 4-8    ┌──────────┴─────────────────────┐ (parallel)         │
│  PHASE 3      │ Smart Contracts (ERC-3643)      │ ← Web3 Dev         │
│               │ (Deploy, Test, Audit)           │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 6-10   ┌──────────▼─────────────────────┐                    │
│  PHASE 4a     │ Frontend: Trading UI            │ ← Frontend Dev     │
│               │ (Orderbook, Charts, WebSocket)  │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 8-12   ┌──────────┴─────────────────────┐ (parallel)         │
│  PHASE 4b     │ Admin Dashboard: Marketplace    │ ← Frontend Dev     │
│               │ (12 Seiten, RBAC, Kill-Switch)  │ + Rust Dev (APIs)  │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 8-10   ┌──────────▼─────────────────────┐                    │
│  PHASE 5      │ Integration & Sicherheit        │ ← Alle Devs        │
│               │ (Backend↔SC, 2FA, Admin RBAC)   │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 10-12  ┌──────────▼─────────────────────┐                    │
│  PHASE 6      │ Testing & QA                    │ ← QA + alle        │
│               │ (Financial Tests, Fuzz, E2E)    │                    │
│               └──────────┬─────────────────────┘                    │
│                          │                                           │
│  Woche 12-14  ┌──────────▼─────────────────────┐                    │
│  PHASE 7      │ Soft Launch & Monitoring        │ ← PM + DevOps      │
│               └────────────────────────────────┘                    │
└──────────────────────────────────────────────────────────────────────┘
```

**Gesamtdauer: ~14 Wochen (3.5 Monate)** – 1 Woche mehr als ursprünglich geplant wegen des Admin-Dashboard-Umfangs (12 Seiten).

---

### 6.2. PHASE 0: Infrastruktur & Account-Setup (Woche 1-2)

**Verantwortlich:** DevOps Engineer + Project Manager
**Abhängigkeiten:** Keine (erster Schritt)

| # | Task | Beschreibung | Dauer | Benötigter Account |
|---|---|---|---|---|
| 0.1 | Google Cloud Projekt einrichten | Neues GCP-Projekt oder bestehendes konfigurieren | 1 Tag | ✅ Bereits vorhanden |
| 0.2 | Cloud SQL #1 (Core DB) provisionieren | `db-f1-micro`, PostgreSQL 16, PITR aktiviert | 1 Tag | ✅ GCP |
| 0.3 | Cloud SQL #1 Read-Replica erstellen | `db-f1-micro` Read-Replica | 30 Min | ✅ GCP |
| 0.4 | Cloud SQL #2 (Community DB) provisionieren | `db-f1-micro`, PostgreSQL 16, PITR aktiviert | 1 Tag | ✅ GCP |
| 0.5 | Cloud SQL #2 Read-Replica erstellen | `db-f1-micro` Read-Replica | 30 Min | ✅ GCP |
| 0.6 | Redis (Google Memorystore) erstellen | `basic-M1`, 1GB | 30 Min | ✅ GCP |
| 0.7 | Cloud Run konfigurieren | Container-Registry, Service-Account, Secrets | 2 Std | ✅ GCP |
| 0.8 | GCS Bucket konfigurieren | Upload-Bucket für KYC-Docs und Avatare | 30 Min | ✅ GCP |
| 0.9 | Sentry-Projekt erstellen | Error-Monitoring für Production | 30 Min | 🆕 [sentry.io](https://sentry.io) |
| 0.10 | GitHub Actions CI/CD | Pipeline: Build → Test → Deploy | 1 Tag | ✅ Bereits vorhanden |
| 0.11 | DB-Migrationen ausführen (Phase 1) | `050b_alter_wallets`, `050c_alter_investments` (Sektion 4.3) | 30 Min | – |
| 0.12 | DB-Migrationen ausführen (Phase 2) | `050-055` Marketplace-Tabellen (Sektion 4.2, 4.6) | 1 Std | – |
| 0.13 | TimescaleDB Extension aktivieren | `CREATE EXTENSION timescaledb` + Migration 056 (Sektion 4.4) | 30 Min | – |
| 0.14 | Community-DB-Migrationen | Community-DB: Neue Community-Tabellen | 2 Std | – |
| 0.15 | PgBouncer als Sidecar konfigurieren | In Dockerfile als Sidecar-Prozess | 1 Tag | – (Open Source) |
| 0.16 | Monitoring-Alerts einrichten | Cloud Monitoring: CPU >70%, Connections >80%, Latenz >500ms | 2 Std | ✅ GCP |
| 0.17 | Marketplace RBAC-Permissions anlegen | 3 neue Permissions ins Roles-System (Sektion 3.5.1) | 1 Std | – |

**Deliverable Phase 0:** Alle 4 Datenbanken laufen (inkl. 7 neue Marketplace-Tabellen), Redis läuft, CI/CD-Pipeline deployed automatisch, Monitoring ist aktiv, RBAC für Admin-Dashboard vorbereitet.

---

### 6.3. PHASE 1: Backend Core – Härtung & 2FA (Woche 2-4)

**Verantwortlich:** Rust Backend Engineer
**Abhängigkeiten:** Phase 0 muss abgeschlossen sein

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 1.1 | Zweiten DB-Pool einrichten | `community_pool` neben `core_pool` in `db.rs` | 1 Tag | Sektion 1.7 |
| 1.2 | Connection Pool erweitern | `max_connections(10)` → `max_connections(50)`, `acquire_timeout(5s)` | 2 Std | Sektion 1.9.A |
| 1.3 | Step-Up 2FA implementieren | `require_step_up_2fa()` Middleware für Withdrawals + Trades | 2 Tage | Sektion 1.11 |
| 1.4 | Trading-Session in Redis | `SET trading_session:{user_id}` mit 15-Min TTL | 1 Tag | Sektion 1.11 |
| 1.5 | Withdrawal-Limits einführen | Daily cap ($10.000), Velocity-Check, Cooldown >$1.000 (24h) | 2 Tage | Sektion 1.8 Frage 3 |
| 1.6 | Idempotency für Checkout | Idempotency-Key in `execute_checkout` | 1 Tag | Sektion 1.8 Frage 6 |
| 1.7 | Circuit Breaker (DB Health) | Health-Check-Endpoint, 503 bei DB-Ausfall | 1 Tag | Sektion 1.8 Frage 4 |
| 1.8 | Reconciliation-Job | Täglicher Job: Wallet-Summen vs. Transaction-Summen | 2 Tage | Sektion 1.8 Frage 2 |
| 1.9 | 2FA-Pflicht für hohe Balances | Redirect zu 2FA-Setup wenn Wallet > $1.000 | 1 Tag | Sektion 1.11 |

**Deliverable Phase 1:** Backend ist gehärtet – 2FA, Withdrawal-Limits, Reconciliation, Circuit Breaker. Keine neuen Features, nur Sicherheit.

---

### 6.4. PHASE 2: Marketplace Engine (Woche 4-8)

**Verantwortlich:** Rust Backend Engineer + DevOps Engineer
**Abhängigkeiten:** Phase 1 muss abgeschlossen sein

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 2.1 | Neue DB-Tabellen | `market_orders`, `trade_history`, `p2p_offers` Migrationen | 1 Tag | Sektion 4.B |
| 2.2 | Redis Orderbook-Modul | `redis-rs`, ZADD/ZREM/ZRANGEBYSCORE Helper-Functions | 3 Tage | Sektion 2.3 |
| 2.3 | Order-Submission API | `POST /api/marketplace/orders` – Validierung, Balance-Check, Redis | 3 Tage | Sektion 2.12 |
| 2.4 | Matching-Engine | Tokio-Task mit Price-Time-Priority, Partial Fills | 5 Tage | Sektion 2.4 |
| 2.5 | Settlement-Funktion | `settle_trade()` – 8-Step ACID Transaction | 3 Tage | Sektion 2.5 |
| 2.6 | Fee-Berechnung | Taker 5.0%, Maker 0%, Treasury-Wallet | 1 Tag | Sektion 2.6 |
| 2.7 | Order APIs | Cancel, Orderbook, Trades, Ticker, Candles | 4 Tage | Sektion 2.12 |
| 2.8 | WebSocket-Server | Axum WebSocket für Live-Orderbook + Trades | 3 Tage | Sektion 2.9 |
| 2.9 | ⏸️ ~~Konzentrationslimits + Großorder-Review~~ | DEFERRED – wird später geprüft | – | Sektion 2.10 |
| 2.10 | P2P/OTC Offer API | Create, Accept/Decline, Settlement | 3 Tage | Sektion 2.7 |
| 2.11 | Redis-Recovery | Orderbook aus `market_orders` WHERE status='open' rebuilden | 1 Tag | Sektion 2.3 |

**Deliverable Phase 2:** Voll funktionsfähige Trading-Engine mit allen APIs.

---

### 6.5. PHASE 3: Smart Contracts (Woche 4-8, PARALLEL zu Phase 2)

**Verantwortlich:** Smart Contract / Web3 Engineer
**Abhängigkeiten:** Unabhängig (kann parallel laufen)

| # | Task | Beschreibung | Dauer | Benötigter Account |
|---|---|---|---|---|
| 3.1 | Foundry-Projekt aufsetzen | `forge init`, OpenZeppelin, ERC-3643 T-REX | 1 Tag | – |
| 3.2 | IdentityRegistry Contract | On-Chain KYC-Registry | 3 Tage | – |
| 3.3 | PooolToken Contract (ERC-3643) | Compliance-Token mit Transfer-Restrictions | 5 Tage | – |
| 3.4 | Foundry Unit + Fuzz Tests | Compliance, Invariant Tests, 10.000+ Fuzz-Runs | 5 Tage | – |
| 3.5 | Base Sepolia Testnet Deploy | Contracts deployen und verifizieren | 1 Tag | 🆕 Base RPC (kostenlos) |
| 3.6 | Hardhat Integration Tests | Backend ↔ Smart Contract Flow Tests | 3 Tage | – |
| 3.7 | Alloy (Rust) Integration | `alloy-rs` im Backend, Contract-Calls aus Rust | 3 Tage | – |
| 3.8 | Google Cloud KMS Setup | Wallet-Key-Management für Treasury | 1 Tag | ✅ GCP |
| 3.9 | Smart Contract Audit beauftragen | ⚠️ IN WOCHE 4 BEAUFTRAGEN (4-6 Wochen Vorlauf!) | – | 🆕 Audit-Firma |

**Deliverable Phase 3:** ERC-3643-Token auf Base Testnet, Tests grün, Audit in Arbeit.

---

### 6.6. PHASE 4a: Frontend – Trading UI (Woche 6-10)

**Verantwortlich:** Frontend Engineer
**Abhängigkeiten:** Phase 2 APIs müssen verfügbar sein (mindestens 2.3-2.8)
**Referenz:** Sektion 3.4 (vollständiger Implementation Guide)

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 4a.1 | Event-Bus + WebSocket-Client | `marketplace-event-bus.js` + `marketplace-websocket.js` | 2 Tage | 3.4.2, 3.4.3 |
| 4a.2 | Marketplace-Übersichtsseite | Alle Assets mit Live-Preis, 24h-Änderung | 3 Tage | – |
| 4a.3 | TradingView Chart | `lightweight-charts.js` + Candlestick-Daten + Interval-Switcher | 3 Tage | 3.4.4 |
| 4a.4 | Asset-Detail (Orderbook) | Bid/Ask-Tabelle, Spread, Flash-Animationen, DOM-Patching | 3 Tage | 3.4.5 |
| 4a.5 | Order-Formular | Buy/Sell, Validierung, Idempotency-Key, Optimistic UI | 3 Tage | 3.4.6 |
| 4a.6 | WebSocket: Live-Updates | Orderbook, Trades, Ticker via WebSocket + Reconnect | 2 Tage | 3.4.3 |
| 4a.7 | Meine Orders + Trade-History | Offene Orders, Cancel, eigene Trade-Liste | 3 Tage | – |
| 4a.8 | P2P Offer UI + Cap Table | Direct Offer Modal, Accept/Decline, Notification | 3 Tage | 3.4.7 |
| 4a.9 | 2FA Modal (Step-Up) | TOTP-Eingabe vor Trades >$500 / Withdrawals | 2 Tage | – |
| 4a.10 | Loading/Error/Empty States | Skeleton-Loader, Retry-Buttons für alle Komponenten | 1 Tag | 3.4.9 |

**Deliverable Phase 4a:** Voll funktionsfähige Investor-seitige Marketplace-UI mit Charts, WebSocket, 2FA.

---

### 6.6b. PHASE 4b: Admin Dashboard – Marketplace-Sektion (Woche 8-12)

**Verantwortlich:** Frontend Engineer + Rust Backend Engineer (für APIs)
**Abhängigkeiten:** Phase 2 APIs + Trading UI (Phase 4a) teilweise fertig
**Referenz:** Sektion 3.5 (vollständiger Implementation Guide, 12 Seiten)

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 4b.1 | **RBAC-Erweiterung** | 3 neue Permissions + 12 `PAGE_PERMISSION_MAP`-Einträge | 1 Tag | 3.5.1 |
| 4b.2 | **Sidebar-Erweiterung** | Neue "📈 MARKETPLACE" Sektion in Admin-Sidebar | 0.5 Tage | 3.5.2 |
| 4b.3 | **Admin API-Endpoints** (Backend) | Alle `/api/admin/marketplace/*` Endpoints implementieren | 5 Tage | 3.5.4-3.5.15 |
| 4b.4 | 🔴 **Overview Dashboard** | KPI-Cards, Live-Trade-Tabelle, Top-5 Assets, Health | 2 Tage | 3.5.4 |
| 4b.5 | 🔴 **Reconciliation** | 3 Invarianten-Checks, Delta-Anzeige, Historien-Tabelle | 2 Tage | 3.5.13 |
| 4b.6 | 🔴 **Trade History** | Server-Side Pagination, 6 Filter, CSV-Export | 2 Tage | 3.5.6 |
| 4b.7 | 🔴 **Open Orders + Admin-Cancel** | Tabelle, Filter, Cancel-Dialog mit Grund | 1.5 Tage | 3.5.7 |
| 4b.8 | 🔴 **Pending Approvals** | Großorder-Review mit User-Kontext, Approve/Reject | 2 Tage | 3.5.8 |
| 4b.9 | 🟡 **Fee Management** | 3 Tabs (Platform, Asset, Promotions), CRUD | 2 Tage | 3.5.9 |
| 4b.10 | 🟡 **Marketplace Settings** | Kill-Switch, 13 konfigurierbare Parameter | 1.5 Tage | 3.5.15 |
| 4b.11 | 🟡 **Live Orderbook (Admin)** | Admin-Ansicht mit User-IDs, Rebuild-Button | 1 Tag | 3.5.5 |
| 4b.12 | 🟡 **P2P Offers Oversight** | Tabelle, Preis-Warnungen, Admin-Cancel | 1 Tag | 3.5.10 |
| 4b.13 | 🟡 **Analytics & Charts** | Volume-Charts, Top-Trader, Fee-Revenue, Heatmap | 2 Tage | 3.5.11 |
| 4b.14 | 🟡 **Alerts & Watchlist** | Alert-Tabelle, Acknowledge/Resolve, Watchlist | 2 Tage | 3.5.12 |
| 4b.15 | 🟡 **Compliance & OJK** | OJK-Report, Travel-Rule, Tax-Export, AML | 2 Tage | 3.5.14 |

**Deliverable Phase 4b:** 12 Admin-Seiten live, RBAC konfiguriert, Kill-Switch funktional. MVP (🔴-Seiten) in Woche 9 ready, voller Umfang in Woche 12.

---

### 6.7. PHASE 5: Integration & Sicherheit (Woche 8-10)

**Verantwortlich:** Alle Developer + Project Manager
**Abhängigkeiten:** Phasen 2, 3, 4a müssen abgeschlossen sein. Phase 4b läuft parallel.

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 5.1 | Backend ↔ Smart Contract | Settlement-Worker schickt Batch-Transfers an Base L2 | 3 Tage | 3.2.9 |
| 5.2 | On-Chain Settlement Cron | Täglicher Job: Trades → Merkle Root → Base L2 | 2 Tage | 3.2.10 |
| 5.3 | Frontend ↔ Blockchain | TX-Hash anzeigen, Blockchain-Explorer-Links | 1 Tag | – |
| 5.4 | Security Review | Alle Endpunkte auf Auth-Bypass, IDOR, XSS prüfen | 3 Tage | – |
| 5.5 | Rate Limiting | Redis-basiert, max 10 Orders/Minute | 1 Tag | 3.1.11 |
| 5.6 | GDPR Compliance | Anonymisierung Community-DB, Selective Deletion Core-DB | 2 Tage | – |
| 5.7 | Admin RBAC Integration | Marketplace-Permissions in Roles-API + permission-guard.js | 1 Tag | 3.5.1 |
| 5.8 | Kill-Switch E2E-Test | Trading stoppen/starten via Admin, verify Orders rejected | 1 Tag | 3.5.15 |
| 5.9 | Reconciliation-Cron aktivieren | Täglicher Job, Ergebnisse in `reconciliation_reports` speichern | 1 Tag | 3.3.7, 4.7 |

---

### 6.8. PHASE 6: Testing & QA (Woche 10-12)

**Verantwortlich:** QA Engineer + alle Developer
**Abhängigkeiten:** Phase 5 abgeschlossen

| # | Task | Beschreibung | Dauer | Referenz |
|---|---|---|---|---|
| 6.1 | Financial Unit Tests | Deposit, Withdrawal, Balance-Invarianten | 2 Tage | Sektion 1.12 |
| 6.2 | Concurrent Trade Tests | Race-Condition-Tests parallel | 2 Tage | Sektion 1.12 |
| 6.3 | Reconciliation Test | Full Lifecycle → Reconciliation = 0 | 1 Tag | Sektion 1.12 |
| 6.4 | Smart Contract Fuzz | `forge test --fuzz-runs 10000` | 1 Tag | Sektion 1.12 |
| 6.5 | E2E (Playwright) | Full User Journey im Browser | 3 Tage | Sektion 1.12 |
| 6.6 | Load Test | 100 User, 500 Orders/Min, 30 Minuten | 2 Tage | – |
| 6.7 | UAT | Interne Test-Nutzer durchlaufen alles | 3 Tage | – |
| 6.8 | Bug-Fix Sprint | Alle Bugs aus 6.1-6.7 fixen | 3 Tage | – |

---

### 6.9. PHASE 7: Soft Launch & Monitoring (Woche 12-14)

**Verantwortlich:** Project Manager + DevOps Engineer
**Abhängigkeiten:** Phase 6 Tests bestanden, Smart Contract Audit abgeschlossen, Admin Dashboard MVP live

| # | Task | Beschreibung | Dauer |
|---|---|---|---|
| 7.1 | Production Deploy | Finaler Build → Cloud Run | 1 Tag |
| 7.2 | Smart Contract Mainnet | Contracts auf Base Mainnet deployen | 1 Tag |
| 7.3 | Admin Dashboard Verify | Alle 5 Launch-kritischen Admin-Seiten (🔴) live + getestet | 1 Tag |
| 7.4 | Reconciliation Day-0 Check | Erste manuelle Reconciliation + Baseline setzen | 0.5 Tage |
| 7.5 | Soft Launch (Invite-Only) | 10-20 Beta-Tester mit echtem Geld | 1 Woche |
| 7.6 | 24/7 Monitoring | Sentry, Cloud Monitoring, Reconciliation-Cron, Alert-Dashboard | Laufend |
| 7.7 | Admin Training | Marketplace Manager + Compliance Officer einweisen | 1 Tag |
| 7.8 | Public Launch | Marketplace für alle Nutzer öffnen | 1 Tag |

---

### 6.10. Third-Party Accounts & Services (Checkliste für den PM)

> **Für den Project Manager:** Diese Liste zeigt ALLE externen Accounts, wann sie gebraucht werden, und wer sie einrichten soll.

| # | Service | Zweck | Phase | Wer | Kosten | URL |
|---|---|---|---|---|---|---|
| 1 | **Google Cloud Platform** | Hosting, DBs, Redis, Cloud Run | Phase 0 | DevOps | ~$65-85/Mo | ✅ Bereits vorhanden |
| 2 | **Sentry** | Error-Monitoring | Phase 0 | DevOps | Free Tier | [sentry.io](https://sentry.io) |
| 3 | **GitHub** | Code, CI/CD | Phase 0 | PM | ✅ Vorhanden | ✅ Bereits vorhanden |
| 4 | **Didit.me** | KYC/AML | Phase 1 | PM | Pay-per-use | ✅ Bereits vorhanden |
| 5 | **Base Sepolia RPC** | Testnet Smart Contracts | Phase 3 | Web3 Dev | Kostenlos | [docs.base.org](https://docs.base.org) |
| 6 | **Alchemy** | Production RPC für Base Mainnet | Phase 3 | Web3 Dev | Free / $49/Mo | [alchemy.com](https://alchemy.com) |
| 7 | **Basescan** | Contract Verification | Phase 3 | Web3 Dev | Kostenlos | [basescan.org](https://basescan.org) |
| 8 | **Pinata** | IPFS für Token-Metadata | Phase 3 | Web3 Dev | Free / $20/Mo | [pinata.cloud](https://pinata.cloud) |
| 9 | **SC Auditor** | Security Audit ERC-3643 | Phase 3 ⚠️ | PM | $5k-$30k | Trail of Bits, OpenZeppelin |
| 10 | **Plaid / Token.io** | Banking API (Deposit-Matching) | Phase 5 | PM | Pay-per-tx | [plaid.com](https://plaid.com) |
| 11 | **SendGrid** | E-Mails (2FA, Trades) | Phase 1 | DevOps | Free / $15/Mo | [sendgrid.com](https://sendgrid.com) |
| 12 | **Cloud Armor** | WAF / DDoS-Schutz | Phase 5 | DevOps | ~$10/Mo | ✅ GCP Add-On |

**PM Tag-1 Checkliste:**

```
☐ GCP: Billing prüfen, Budget-Alert bei $100/Mo setzen
☐ Sentry: Projekt erstellen, DSN-Key als Secret in Cloud Run
☐ GitHub: CI/CD Secrets konfigurieren (DATABASE_URL, REDIS_URL, SENTRY_DSN)
☐ Didit.me: API-Keys prüfen, Webhook-URL für Production
☐ Smart Contract Auditor: 3 Angebote einholen (JETZT, nicht in Woche 8!)
☐ Alchemy: Free Account erstellen für Base RPC
☐ Pinata: Free Account für IPFS
☐ SendGrid: API-Key, Sender-Domain verifizieren
```

---

### 6.11. Abhängigkeiten & Blocker

```
PARALLEL möglich:
• Phase 2 (Marketplace) + Phase 3 (Smart Contracts) → gleiche Wochen
• Phase 4 (Frontend, ab Woche 6) sobald Phase 2 APIs teilweise fertig

BLOCKER:
• Phase 4a KANN NICHT starten ohne Phase 2 APIs (min. Orders, Orderbook, Trades)
• Phase 4b KANN NICHT starten ohne Phase 2 APIs + Admin-Marketplace-APIs
• Phase 4b Admin-APIs werden parallel von Rust-Dev gebaut (ab Woche 8)
• Phase 5 KANN NICHT starten ohne Phase 2 + 3 + 4a alle fertig
• Phase 7 KANN NICHT starten ohne bestandene Phase 6 Tests
• Phase 7 erfordert mindestens 5 Admin-Seiten (🔴 Launch-Critical)
• ⚠️ Smart Contract Audit muss in WOCHE 4 beauftragt werden!
  (4-6 Wochen Bearbeitungszeit → Ergebnis erst in Woche 8-10)
```

---

### 6.12. Wöchentliches PM Status-Meeting (Template)

```markdown
# POOOL Marketplace – Weekly Status (Woche X)

## ✅ Abgeschlossen letzte Woche
- [ Task ]

## 🔵 In Arbeit diese Woche
- [ Task ] (Zuständig: Name, Deadline: Fr)

## 🔴 Blockiert
- [ Task ] ← Blockiert durch: [ Reason ]

## 📋 Third-Party Account Status
- [ ] Sentry: ✅ eingerichtet
- [ ] Alchemy: ⏳ wartet auf PM
- [ ] SC Auditor: 🔴 Muss DIESE Woche beauftragt werden!

## 📊 Metriken
- Tests: X bestanden / Y fehlgeschlagen
- Kosten diesen Monat: $XX (Budget: $85)
- Nächster Meilenstein: Phase X (Woche Y)
```

---

## 7. Extended Community Backlog (Post-M7 Vision)

> **Architektur-Note:** Die Community läuft in einer separaten PostgreSQL-Instanz (siehe Sektion 1.7). Die folgenden Features repräsentieren die finale Stufe des Web3/Social-Ausbaus und dürfen erst nach Abschluss von Modul 7 (Community Roadmap) angegangen werden.

### 7.1. UX & Discovery (The "Sticky" Layer)
* **Core Discovery:** Algorithmic Feed Sorting (Hot/Trending), Global Search Engine, @-Mentions & Notifications, Hashtag Architecture, Rich Link Previews (OpenGraph).
* **Retention:** Saved/Bookmarked Posts, Threaded Comment Collapse, "Recommended for You" Feeds, Offline Push/Email Digests, Direct Messaging (DMs).
* **Rich Media:** Native Polls & Surveys, Auto-Saving Drafts (localStorage), Inline GIF/Tenor Integration, Custom User Flairs, Native Dark/Light Mode.
* **Polish:** Quote Reposts, Dynamic "Top Contributor" Badges, Live Presence Indicators ("John is typing..."), Native Content Translation, "Time to Read" Estimates.

### 7.2. Web3 & Platform Integrations
* **Token-Gated Circles:** Automatische Freigabe für Investoren, die z.B. $1.000 eines spezifischen Assets halten.
* **NFT Verification & Sync:** WalletConnect-Integration für NFT-Avatare; automatische Cross-Posts zu X (Twitter) und Discord.
* **On-Chain Features:** Embedded "Buy Asset"-Widgets in Posts, DAO Treasury Display für Investment-Clubs, On-Chain Tipping, Trading PnL Leaderboards (Sync aus Trade-History!).

### 7.3. Circle Owner Analytics & Tools
* **Analytics:** Active Member Heatmaps, Top Contributor Reports, Engagement Funnel Metrics.
* **Branding:** Upload von Custom Bannern, primäre HEX-Brand-Colors für den Circle.
* **Management & Moderation:** Automated Welcome Messages, Post Scheduling, Circle-Level Shadowbanning, Keyword Alerts (Defense), Membership Questionnaires, Moderation Audit Logs.

### 7.4. Mobile Experience & Native Hooks
* **Native Feel:** PWA Install Prompts, Mobile-Optimized Bottom Nav, Swipe-to-Go-Back Gestures, Pull-to-Refresh.
* **OS Integrations:** Web Push Notifications, Critical SMS Alerts, Haptic Feedback (Vibration API), Native OS Share Sheets.

---
*Dieser Masterplan bildet das unerschütterliche technische Fundament für die Transformation von POOOL in einen echten, lebendigen Finanzmarktplatz.*

