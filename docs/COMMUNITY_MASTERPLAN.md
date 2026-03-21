# POOOL Community System — Masterplan

> **Kritische Warnung:** Eine Community-Plattform für eine Finanz-Investmentplattform ist KEIN einfaches Social-Media-Feature. Jeder Post, jede Reaction und jede DM kann regulatorische Konsequenzen haben (z.B. Anlageberatung ohne Lizenz, Marktmanipulation, Betrug). Dieser Plan geht tief ins technische Detail und berücksichtigt die Wechselwirkungen mit dem bestehenden Trading-System.

> **Architektur-Kontext:** Wie in `MASTERPLAN.md` §1.7 entschieden, lebt die Community in einer **physisch getrennten PostgreSQL-Datenbank** (`pool_community`). Die Community darf NIEMALS die Core-DB (Trades, Wallets, Investments) beeinflussen. Die einzige Verbindung ist die `user_id` — alles andere ist lose gekoppelt.

---

## Inhaltsverzeichnis

1. [Analyse des Ist-Zustands](#1-analyse-des-ist-zustands)
2. [Feature-Architektur & Subsysteme](#2-feature-architektur--subsysteme)
    - [2.1. Social Feed (Posts, Comments, Reactions)](#21-social-feed)
    - [2.2. Follow-System & My Circle](#22-follow-system--my-circle)
    - [2.3. Badge- & Gamification-System](#23-badge---gamification-system)
    - [2.4. Expert AMAs (Ask Me Anything)](#24-expert-amas)
    - [2.5. Asset Reviews & Ratings](#25-asset-reviews--ratings)
    - [2.6. Direct Messages (DMs)](#26-direct-messages)
    - [2.7. Announcements & Notifications](#27-announcements--notifications)
    - [2.8. Community Profiles](#28-community-profiles)
    - [2.9. Content Moderation & Compliance](#29-content-moderation--compliance)
    - [2.10. Leaderboard & Challenges](#210-leaderboard--challenges)
    - [2.11. Experience Points (XP) — Erfahrungspunkte-System](#211-experience-points-xp--erfahrungspunkte-system)
3. [Datenbank-Schema (Community-DB)](#3-datenbank-schema-community-db)
4. [API-Endpunkte](#4-api-endpunkte)
5. [Frontend-Architektur](#5-frontend-architektur)
6. [Entwickler-Perspektiven](#6-entwickler-perspektiven)
7. [Sicherheit & Compliance](#7-sicherheit--compliance)
8. [Infrastruktur & Performance](#8-infrastruktur--performance)

---

## 0. Stakeholder-Entscheidungen (2026-03-21)

### E1. Separate Datenbank (Bestätigt in Marketplace-Masterplan §1.7)

**Entscheidung:** Die Community bekommt eine **eigene Cloud SQL Instanz** (`poool-community-db`), physisch getrennt von der Core-DB.

**Begründung:**
- Community-Traffic (Likes, Comments, Follows) erzeugt 10-50x mehr Writes/Tag als der Marketplace
- Ein viraler Post mit 500 Likes in 30 Sekunden darf NIEMALS einen €500-Trade blockieren
- Community-Migrationen (`ALTER TABLE`) dürfen nie die Trading-DB locken
- Security: XSS in Community-Posts kompromittiert nur Community-Daten, nie Wallets

### E2. User-Daten-Bridging

**Entscheidung:** Die Community-DB speichert NUR die `user_id` (UUID). Alle User-Profildaten (Name, Avatar, Tier, KYC-Status) werden **gecacht** und via Batch-Lookup aus der Core-DB geholt.

**Mechanismus:**
1. Community-Post hat `user_id` als Fremdschlüssel (ohne FK Constraint — da andere DB)
2. Rust-Backend sammelt alle `user_id`s eines Feed-Requests
3. Batch-Query an Core-DB: `SELECT id, display_name, avatar_url, tier FROM users WHERE id IN ($1, $2, ...)`
4. Join im Rust-Code (nicht in SQL)
5. Caching in Redis: `community:user:{user_id}` mit 5-Minuten TTL

### E3. Content-Policy für Finanzplattform

**Entscheidung:** Posts dürfen KEINE konkreten Anlageempfehlungen enthalten. Ein Disclaimer wird automatisch an Investment-bezogene Posts angehängt.

**Regeln:**
- ✅ Erlaubt: "Ich habe in die Bali Cocoa Farm investiert und bin zufrieden"
- ✅ Erlaubt: "Der Cocoa-Preis ist um 8% gestiegen"
- ❌ Verboten: "Kauft JETZT die Bali Cocoa Farm, der Preis wird steigen!"
- ❌ Verboten: "Dieses Investment garantiert 28% Rendite"
- Automatischer Disclaimer: "Hinweis: Dies ist keine Anlageberatung. Investitionen sind mit Risiken verbunden."

### E4. Gamification-Strategie

**Entscheidung:** Die Community nutzt ein **Badge-System** mit Investment-verknüpften Badges (nicht käuflich, nur durch echte Aktionen verdienbar).

**Badge-Kategorien:**
| Kategorie | Beispiele | Wie verdient |
|---|---|---|
| **Investment** | First Investor, Diversified (5+ Assets), Diamond Hands (1yr Hold) | Automatisch via Core-DB |
| **Community** | Connector (10 Followers), Influencer (50+ Likes auf 1 Post), Mentor (10 hilfreiche Antworten) | Automatisch via Community-DB |
| **Verified** | KYC Verified, Dividend Received, Exited Successfully | Automatisch via Core-DB |
| **Tier** | Silver, Gold, Platinum, Diamond | Aus Rewards-System (Core-DB) |

---

## 1. Analyse des Ist-Zustands

### 1.1. Was existiert

| Komponente | Status | Details |
|---|---|---|
| **Frontend** | ✅ Statische Demo | `community.html` mit 5 Tabs (Feed, Announcements, My Circle, Expert AMAs, Reviews) — **rein statisch, keine Backend-Anbindung** |
| **CSS** | ✅ Vorhanden | `static/css/community.css` + `community-card.css` |
| **Backend** | ❌ Nicht vorhanden | Kein `src/community/` Modul, keine Routes, keine Models |
| **Datenbank** | ❌ Nicht vorhanden | Keine Community-Tabellen, keine Migrationen |
| **API** | ❌ Nicht vorhanden | Keine Community-API-Endpunkte |

### 1.2. Bestehendes Frontend (Demo-Analyse)

Die `community.html` enthält bereits ein ausgereiftes UI-Mockup mit 5 Tabs:

| Tab | Features im Mockup | Backend-Anforderung |
|---|---|---|
| **Feed** | Create Post, Announcements, Milestones, Tips, Farm Updates, Reactions, Comments | Posts CRUD, Reactions, Comments, Feed-Algorithmus |
| **Announcements** | Filter (All/New/Dividends/Platform/Market), Pinned Posts, Read More | Announcements CRUD, Kategorien, Pinning |
| **My Circle** | Circle Stats, Member List, Challenges, Leaderboard, Invite Friends | Follow-System, Referral-Integration, Challenges |
| **Expert AMAs** | Upcoming AMA Hero, Past AMAs Archive, Question Submission, Reminders | AMA Scheduling, Q&A System, Live-Events |
| **Reviews** | Star Ratings, Breakdown Chart, Filter by Commodity, Helpful Votes | Review CRUD, Rating Aggregation, Verificated Reviews |

> **Wichtige Erkenntnis:** Das Frontend-Mockup ist bereits sehr detailliert. Das Backend muss genau diese Strukturen mit APIs bedienen. Die Frontend-Arbeit besteht hauptsächlich darin, die statischen Demo-Daten durch `fetch()`-Calls zu ersetzen.

### 1.3. Wechselwirkungen mit anderen Subsystemen

```
┌──────────────────────────────────────────────────────────────────┐
│                 COMMUNITY SYSTEM DEPENDENCIES                    │
│                                                                  │
│  ┌──────────────┐         ┌──────────────────────────────────┐  │
│  │ CORE DB      │         │ COMMUNITY DB                     │  │
│  │              │◄───────▶│                                  │  │
│  │ • users      │ user_id │ • posts        • messages        │  │
│  │ • investments│ Batch   │ • comments     • conversations   │  │
│  │ • wallets    │ Lookup  │ • reactions    • reports         │  │
│  │ • assets     │         │ • follows      • announcements   │  │
│  │ • kyc_records│         │ • badges       • amas            │  │
│  │ • rewards    │         │ • reviews      • ama_questions   │  │
│  └──────────────┘         │ • user_badges  • challenges      │  │
│         │                 └──────────────────────────────────┘  │
│         │                              │                        │
│         ▼                              ▼                        │
│  ┌──────────────┐         ┌──────────────────────────────────┐  │
│  │ Redis        │         │ GCS (Cloud Storage)              │  │
│  │              │         │                                  │  │
│  │ • user cache │         │ • post_images/                   │  │
│  │ • feed cache │         │ • community_avatars/             │  │
│  │ • rate limits│         │                                  │  │
│  └──────────────┘         └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

**Cross-DB Queries die nötig sind (alle über Rust-Code, nie via SQL JOIN):**

| Community braucht von Core-DB | Häufigkeit | Caching |
|---|---|---|
| Username + Avatar für Posts/Comments | Jeder Feed-Request | Redis 5min TTL |
| User-Tier (Silver/Gold/Platinum/Diamond) | Badge-Berechnung | Redis 1h TTL |
| KYC-Status (Verified Badge) | Profil-Anzeige | Redis 1h TTL |
| Investment-Daten (für "Verified Investor" Badge) | Badge-Worker (Batch) | Redis 24h TTL |
| Asset-Namen (für Asset-bezogene Posts/Reviews) | Feed-Rendering | Redis 1h TTL |

---

## 2. Feature-Architektur & Subsysteme

### 2.1. Social Feed

Der Feed ist das Herzstück der Community. Er zeigt eine chronologische Timeline von Posts verschiedener Typen.

#### Post-Typen

| Typ | Ersteller | Besonderheiten |
|---|---|---|
| **general** | Jeder User | Freitext, optional Bild |
| **market_insight** | Jeder User | Investment-bezogen, automatischer Disclaimer |
| **milestone** | System (automatisch) | "Sarah hat ihr 5. Investment getätigt!" |
| **farm_update** | Asset-Operator (Developer) | Offizielles Update zu einem Asset |
| **announcement** | Admin/POOOL Team | Plattform-News, Pinnable |
| **review** | Verified Investor | Bewertung eines Assets (verlinkt zu Reviews-Tab) |

#### Feed-Algorithmus (Einfach, keine KI)

**Phase 1 (Launch):** Rein chronologisch — neueste Posts zuerst.

**Phase 2 (ab ~500 Nutzer):** Gewichteter Feed mit Scoring:

```
Score = base_time_score
      + (is_following_author ? 50 : 0)
      + (post_type == 'announcement' ? 100 : 0)
      + (post_type == 'farm_update' ? 75 : 0)
      + (is_pinned ? 200 : 0)
      + log2(reaction_count + 1) * 10
      + log2(comment_count + 1) * 15
```

> **Warum kein ML-Algorithmus?** Bei <1.000 Nutzern ist ein ML-Feed Overkill. Chronologisch + Boost für Announcements und gefolgte Autoren reicht völlig. Ein ML-System kann in Phase 4 (>5.000 Nutzer) evaluiert werden.

#### Reactions System

Statt nur "Like" unterstützen wir **Emoji-Reactions** (wie Slack/Discord):

| Reaction | Emoji | Kontext |
|---|---|---|
| Fire | 🔥 | "Das ist gut!" |
| Insightful | 💡 | "Guter Insight" |
| Clap | 👏 | "Gratulation!" |
| Green | 🌱 | "Nachhaltig!" |

**Technisch:** Ein User kann pro Post maximal 1 Reaction pro Typ abgeben. Toggle-Logik (erneut klicken = entfernen).

#### Comments

- Flache Kommentare (keine verschachtelten Threads — zu komplex für V1)
- Maximal 2.000 Zeichen pro Kommentar
- Ammonia HTML-Sanitizer für XSS-Schutz
- "Helpful"-Button auf Kommentare (für Sortiering)
- Pagination: 10 Kommentare initial, "Load More" Button

---

### 2.2. Follow-System & My Circle

#### Follow-Mechanik

- **Unidirektional:** User A folgt User B (User B muss nicht zurückfolgen)
- **Kein Approval nötig** (öffentliche Profile)
- Followers/Following-Count wird im Profil angezeigt
- Feed wird durch Follows beeinflusst (Posts von gefolgten Usern bekommen Score-Boost)

#### "My Circle" — Vollständiges Konzept

> **Kernidee:** Ein "Circle" ist die persönliche Investoren-Community eines Users. Jeder User hat **automatisch** einen eigenen Circle — er wird beim Account-Erstellung implizit "gegründet". Der Circle besteht aus allen Usern, die über deinen **Referral-Link** beigetreten sind, PLUS Usern die du manuell eingeladen hast.

##### Was ist ein Circle?

Ein Circle ist **KEIN separater Raum oder Chatgroup** — es ist eine **Beziehungsliste** (ähnlich einer Freundesliste), die:
1. Automatisch alle Referrals enthält (Core-DB `referral_tracking`)
2. Manuell durch "Invite to Circle" erweiterbar ist (Community-DB `circle_members`)
3. Gemeinsame Stats anzeigt (Total Invested, Aktive Members, Ranking)
4. Challenges als Gruppe annehmen kann
5. Im Leaderboard als Team gerankt wird

##### Wie wird ein Circle gegründet?

```
┌──────────────────────────────────────────────────────────────────┐
│  CIRCLE LIFECYCLE                                                │
│                                                                  │
│  1. User erstellt Account → Circle wird AUTOMATISCH erstellt     │
│     (circle_id = user_id, Name = "{User}'s Circle")              │
│                                                                  │
│  2. User teilt seinen Referral-Link (bereits existierend!)       │
│     https://poool.app/signup?ref=POOOL001                        │
│                                                                  │
│  3. Neuer User meldet sich über den Link an                      │
│     → Core-DB: referral_tracking(referrer_id, referred_id)       │
│     → Community-DB: circle_members(circle_id, user_id) AUTO-JOIN │
│                                                                  │
│  4. Circle-Owner sieht neues Mitglied im "My Circle" Tab         │
│     → Notification: "🎉 Sarah ist deinem Circle beigetreten!"   │
│                                                                  │
│  5. Alternativ: Circle-Owner schickt manuelle Einladung           │
│     → In-App Invite: "Tritt meinem Circle bei!"                  │
│     → Empfänger kann annehmen oder ablehnen                      │
└──────────────────────────────────────────────────────────────────┘
```

##### Kostet ein Circle etwas?

**Nein — Circles sind komplett kostenlos.** Der Circle ist das Herzstück des Referral-Systems und soll Wachstum fördern. Kostenbarrieren wären kontraproduktiv.

| Aspekt | Kosten | Begründung |
|---|---|---|
| Circle erstellen | **Kostenlos** (automatisch) | Jeder User hat einen Circle |
| Circle beitreten | **Kostenlos** | Wachstum fördern |
| Challenges teilnehmen | **Kostenlos** | Engagement fördern |
| Circle verlassen | **Jederzeit möglich** | Keine Lock-in-Effekte |

##### Referral-Link → Automatischer Circle-Beitritt

**Das ist die eleganteste Lösung:** Der bestehende Referral-Link (`/signup?ref=POOOL001`) wird zur Circle-Einladung.

**Technischer Flow:**

```
User B klickt auf User A's Referral-Link
    → /signup?ref=POOOL001
    → User B registriert sich
    → Core-DB: INSERT INTO referral_tracking (referrer_id=A, referred_id=B)
    → Rust-Backend (nach Signup): Auto-Join Circle
        → Community-DB: INSERT INTO circle_members (circle_id=A, user_id=B, joined_via='referral')
    → Notification an A: "🎉 B ist über deinen Link beigetreten!"
    → Notification an B: "👋 Willkommen in A's Circle!"
```

> **Wichtig:** Dies erfordert einen Hook im bestehenden Signup-Flow (`backend/src/auth/`). Nach erfolgreichem Signup mit `?ref=` Parameter muss ein **asynchroner Call** an die Community-DB gemacht werden. Falls die Community-DB down ist, wird der Auto-Join über einen **Retry-Worker** nachgeholt (lose Kopplung!).

##### Kann man einem Circle beitreten OHNE Referral-Link?

**Ja — über manuelle Einladung:**

| Methode | Wie | Bedingung |
|---|---|---|
| **Referral-Link** (automatisch) | Signup über `?ref=` Link | Keine — passiert automatisch |
| **Manuelle Einladung** | Circle-Owner klickt "Invite" auf einem User-Profil | Empfänger muss die Einladung akzeptieren |
| **Öffentliche Circles** (Phase 2) | User sucht nach interessanten Circles und tritt bei | Circle-Owner muss "Public" aktiviert haben |
| **Kein Beitritt nötig** | User kann auch ohne Circle die Community nutzen | Posts, Reactions, Reviews — alles ohne Circle möglich |

> **Design-Entscheidung:** Ein User kann nur in **einem Circle** gleichzeitig sein (wie eine Guild in einem MMO). Das verhindert Gamification-Missbrauch (User tritt 50 Circles bei um überall XP zu sammeln). Wenn ein User manuell einem neuen Circle beitritt, verlässt er den alten automatisch.

##### Braucht man bestimmte Voraussetzungen für einen Circle?

**Nein — aber es gibt Level-basierte Vorteile:**

| Circle-Feature | Voraussetzung | Details |
|---|---|---|
| **Circle beitreten** | Keine | Jeder registrierte User |
| **Circle anzeigen** | Keine | Sichtbar für alle |
| **Circle-Chat** (Phase 2) | KYC Verified | Schutz vor Spam-Accounts |
| **Challenge starten** | Circle Level ≥ 3 | Mindestens 5 Members + 500 XP |
| **Public Circle** (Phase 2) | Circle Level ≥ 5 | Mindestens 10 Members + 2.000 XP |
| **Eigene Circle-Seite** (Phase 3) | Circle Level ≥ 7 | Custom Banner, Description, etc. |

##### Circle-Levels (basierend auf XP)

Circles haben ein eigenes Level-System, das sich aus der **kollektiven Aktivität** aller Members berechnet:

| Circle Level | XP benötigt | Vorteile |
|---|---|---|
| **1** | 0 | Grundfunktionen: Members-Liste, Stats |
| **2** | 250 | Circle-Badge sichtbar auf Profilen der Members |
| **3** | 500 | Challenges annehmen, Circle-Leaderboard sichtbar |
| **4** | 1.000 | Custom Circle-Name/Beschreibung |
| **5** | 2.000 | Öffentlich sichtbar, andere können beitreten |
| **6** | 5.000 | Priority-Platzierung im Leaderboard |
| **7** | 10.000 | Custom Circle-Banner, eigene Seite |
| **8** | 25.000 | Exklusiver "Elite Circle" Badge für alle Members |
| **9** | 50.000 | Zugang zu Beta-Features, eigene Challenge erstellen |
| **10** | 100.000 | Permanente Leaderboard-Krone, VIP-Einladungen |

**Circle-XP = Summe der individuellen XP aller Members** (siehe §2.11 XP-System)

##### Circle-Daten & Cross-DB Bridging

| Feature | Datenquelle | Details |
|---|---|---|
| Circle Members | **Community-DB** `circle_members` + **Core-DB** `referral_tracking` | Merged in Rust |
| Circle Total Invested | Core-DB `investments` SUM | Summe aller Investments der Circle-Members |
| Circle XP | Community-DB `xp_ledger` SUM | Aggregiert pro Circle |
| Circle Level | Community-DB `circles.level` | Berechnet aus Circle-XP |
| Circle Rank | Community-DB Leaderboard-Query | ORDER BY circle_xp DESC |
| Referral Earnings | Core-DB `referral_commissions` | Deine persönlichen Provisionen |

> **Cross-DB Bridging:** Circle-Membership lebt in der Community-DB (`circle_members`), aber die ursprüngliche Referral-Beziehung kommt aus der Core-DB (`referral_tracking`). Beim Signup mit Referral-Code schreibt der Auth-Service in die Core-DB, und ein **Event** (über einen Background-Worker oder sogar synchron im selben Request) schreibt den Auto-Join in die Community-DB. Die Community-DB ist die **Source of Truth** für Circle-Membership — die Core-DB `referral_tracking` ist nur der Trigger.

##### Circle-Governance

| Regel | Details |
|---|---|
| **Nur 1 Circle pro User** | Verhindert XP-Farming über mehrere Circles |
| **Circle-Owner ist immer der Referrer** | Der User mit dem Referral-Code ist automatisch Owner |
| **Owner kann Members entfernen** | Kick-Funktion (Member bekommt Notification) |
| **Members können jederzeit gehen** | Leave-Button, sofort wirksam |
| **Circle wird NIE gelöscht** | Auch bei 0 Members bleibt der Circle bestehen |
| **Ban übertragen?** | Community-Ban = auch Circle-Ausschluss |

---

### 2.11. Experience Points (XP) — Erfahrungspunkte-System

> **Kernidee:** XP belohnen **echte Aktivität** auf der Plattform — sowohl Investment-Aktivitäten als auch Community-Engagement. XP sind NICHT käuflich und können NICHT getauscht werden. Sie dienen ausschließlich dazu, den "Level" eines Users und seines Circles zu bestimmen, was bestimmte Features freischaltet.

#### Wie verdient man XP?

| Aktion | XP | Kategorie | Max/Tag | Datenquelle |
|---|---|---|---|---|
| **Account erstellen** | 50 | Onboarding | Einmalig | Core-DB |
| **KYC verifizieren** | 100 | Onboarding | Einmalig | Core-DB |
| **Erstes Investment** | 200 | Investment | Einmalig | Core-DB |
| **Investment tätigen** | 50 | Investment | 5x/Tag (250) | Core-DB |
| **Dividend erhalten** | 75 | Investment | Unbegrenzt | Core-DB |
| **Post erstellen** | 10 | Community | 5x/Tag (50) | Community-DB |
| **Kommentar schreiben** | 5 | Community | 20x/Tag (100) | Community-DB |
| **Reaction geben** | 2 | Community | 50x/Tag (100) | Community-DB |
| **Follower gewinnen** | 15 | Community | Unbegrenzt | Community-DB |
| **"Helpful" Vote erhalten** | 10 | Community | Unbegrenzt | Community-DB |
| **Review schreiben** | 25 | Community | 1x/Asset | Community-DB |
| **AMA-Frage einreichen** | 10 | Community | 3x/AMA | Community-DB |
| **AMA-Frage beantwortet** | 50 | Community | Unbegrenzt | Community-DB |
| **Jemand tritt deinem Circle bei** | 30 | Social | Unbegrenzt | Community-DB |
| **Challenge abschließen** | 100-500 | Challenges | Pro Challenge | Community-DB |
| **Täglicher Login** | 5 | Engagement | 1x/Tag | Core-DB |
| **7-Tage Streak** | 50 | Engagement | 1x/Woche | Community-DB |
| **30-Tage Streak** | 200 | Engagement | 1x/Monat | Community-DB |

> **Anti-Gaming-Regeln:**
> - **Daily Caps** verhindern Spam (max 50 XP durch Reactions/Tag, max 50 XP durch Posts/Tag)
> - **Selbst-Reactions zählen nicht** (du kannst deinen eigenen Post nicht liken)
> - **Gelöschte Posts verlieren XP retroaktiv** (XP-Abzug bei Deletion/Moderation)
> - **Banned Users verlieren XP für Ban-Periode** (Motiviert regelkonformes Verhalten)

#### User-Levels (basierend auf persönlichen XP)

| Level | Name | XP benötigt | Freigeschaltete Features |
|---|---|---|---|
| **1** | Newcomer | 0 | Basis-Features: Posts, Reactions, Feed lesen |
| **2** | Explorer | 100 | Kommentare schreiben, Profil-Bio bearbeiten |
| **3** | Contributor | 300 | Reviews schreiben (wenn Investor), AMA-Fragen |
| **4** | Active Member | 750 | Bilder in Posts hochladen (max 2) |
| **5** | Established | 1.500 | "Established" Badge, bis zu 4 Bilder pro Post |
| **6** | Influencer | 3.000 | Kann manuell in Circle einladen |
| **7** | Expert | 7.500 | "Expert" Badge, Posts im Feed priorisiert |
| **8** | Leader | 15.000 | "Leader" Badge, kann Circle-Challenges vorschlagen |
| **9** | Ambassador | 30.000 | "Ambassador" Badge, Zugang zu exklusiven AMAs |
| **10** | Legend | 75.000 | "Legend" Badge, permanente Custom-Profilelemente |

> **Warum Level-Gates für Features?** Das verhindert, dass ein frisch registrierter Spam-Account sofort 100 Posts mit Bildern erstellen kann. Du musst erst **beweisen**, dass du ein echtes Community-Mitglied bist, bevor du alle Features nutzen kannst. Das ist ein bewährtes Pattern von Reddit (Karma), Stack Overflow (Reputation) und Discord (Server-Levels).

#### XP-Berechnung — Technisch

```
┌──────────────────────────────────────────────────────────────────┐
│  XP LEDGER (Append-Only Log — NIEMALS mutiert)                   │
│                                                                  │
│  Jede XP-Änderung wird als Eintrag im xp_ledger gespeichert:    │
│                                                                  │
│  | id | user_id | amount | reason      | source_id | created_at │
│  |----|---------|--------|-------------|-----------|------------|│
│  | 1  | alice   | +50    | signup      | NULL      | 2026-03-01 │
│  | 2  | alice   | +10    | post_create | post_123  | 2026-03-02 │
│  | 3  | alice   | +2     | reaction    | react_456 | 2026-03-02 │
│  | 4  | alice   | -10    | post_delete | post_123  | 2026-03-03 │
│  | 5  | alice   | +200   | first_invest| inv_789   | 2026-03-05 │
│                                                                  │
│  Total XP = SUM(amount) WHERE user_id = 'alice' → 252           │
│  Level = lookup(252) → Level 3 (Contributor)                     │
│                                                                  │
│  Denormalisiert in community_profiles.xp_total + .level          │
│  (updated via Trigger oder Worker alle 5 Minuten)                │
└──────────────────────────────────────────────────────────────────┘
```

**Warum Append-Only?**
- Vollständige Audit-Trail — man kann nachvollziehen WOHER jeder XP-Punkt kam
- Einfache Korrektur bei Missbrauch (negative Einträge statt UPDATE)
- Kein Race-Condition-Risiko bei parallelen XP-Vergaben

#### XP-Worker (Background-Task)

Der XP-Worker läuft alle **5 Minuten** und:
1. Summiert `xp_ledger` pro User → `community_profiles.xp_total`
2. Berechnet Level aus XP → `community_profiles.level`
3. Summiert XP aller Circle-Members → `circles.xp_total`, `circles.level`
4. Prüft: Hat ein User gerade ein neues Level erreicht? → Notification + Badge
5. Prüft: Hat ein Circle gerade ein neues Level erreicht? → Notification an Owner

#### Zusammenspiel: XP → Badges → Circle-Level → Challenges

```
User-Aktionen (Post, React, Invest, Follow)
    ↓
XP verdient (xp_ledger INSERT)
    ↓
User-Level steigt (community_profiles.level)
    ↓
Neue Features freigeschaltet (z.B. Level 4 → Bilder posten)
    ↓
User verdient Badge (z.B. Level 5 → "Established" Badge)
    ↓
Circle-XP steigt (SUM aller Member-XP)
    ↓
Circle-Level steigt (circles.level)
    ↓
Circle schaltet Features frei (z.B. Level 3 → Challenges)
    ↓
Circle nimmt an Challenge teil
    ↓
Challenge abgeschlossen → Bonus-XP für alle Members
    ↓
Feedback-Loop: Mehr XP → Höheres Level → Mehr Features → Mehr Engagement
```

---

### 2.3. Badge- & Gamification-System

Badges sind **nicht käuflich** — sie werden ausschließlich durch echte Aktionen auf der Plattform verdient.

#### Badge-Definitionen

| Badge ID | Name | Icon | Kriterium | Datenquelle |
|---|---|---|---|---|
| `first_investor` | First Investment | 🎯 | 1+ Investment | Core-DB `investments` |
| `diversified` | Diversified Investor | 🌍 | 5+ verschiedene Assets | Core-DB `investments` |
| `diamond_hands` | Diamond Hands | 💎 | 1+ Investment seit >12 Monaten gehalten | Core-DB `investments` |
| `dividend_earner` | Dividend Earner | 💰 | 1+ Dividend erhalten | Core-DB `dividend_payouts` |
| `successful_exit` | Successful Exit | 🏆 | 1+ Exit mit Gewinn abgeschlossen | Core-DB `investments` |
| `kyc_verified` | KYC Verified | ✅ | KYC Status = approved | Core-DB `kyc_records` |
| `connector` | Connector | 🤝 | 10+ Followers | Community-DB `follows` |
| `influencer` | Influencer | ⭐ | 50+ Reactions auf einen Post | Community-DB `reactions` |
| `mentor` | Mentor | 🎓 | 10+ "Helpful" Votes auf Kommentare | Community-DB `comments` |
| `reviewer` | Top Reviewer | 📝 | 5+ verifizierte Reviews | Community-DB `reviews` |
| `ama_participant` | AMA Participant | 🎤 | 1+ AMA Frage beantwortet (vom Experten) | Community-DB `ama_questions` |
| `early_adopter` | Early Adopter | 🚀 | Account erstellt vor Community-Launch | Core-DB `users.created_at` |
| `tier_silver` | Silver Tier | 🥈 | Rewards-Tier = Silver | Core-DB `users.tier` |
| `tier_gold` | Gold Tier | 🥇 | Rewards-Tier = Gold | Core-DB `users.tier` |
| `tier_platinum` | Platinum Tier | 💠 | Rewards-Tier = Platinum | Core-DB `users.tier` |
| `tier_diamond` | Diamond Tier | 💎 | Rewards-Tier = Diamond | Core-DB `users.tier` |

#### Badge-Worker (Background-Task)

```
┌─────────────────────────────────────────────────────┐
│  BADGE CALCULATION WORKER (runs every 6 hours)       │
│                                                     │
│  1. Query Core-DB: Get all users with their         │
│     investment counts, tiers, KYC status            │
│  2. Query Community-DB: Get follower counts,        │
│     reaction counts, review counts                  │
│  3. For each user: Calculate earned badges          │
│  4. Compare with existing badges in community_db    │
│  5. INSERT new badges (never DELETE earned badges)   │
│  6. Emit notification for newly earned badges       │
└─────────────────────────────────────────────────────┘
```

---

### 2.4. Expert AMAs (Ask Me Anything)

AMAs sind **zeitlich begrenzte Live-Events** wo ein Experte (Farmer, Analyst, POOOL-Team) Fragen der Community beantwortet.

#### AMA-Lifecycle

```
DRAFT → SCHEDULED → ACCEPTING_QUESTIONS → LIVE → CLOSED → ARCHIVED
  ↑         ↑              ↑                 ↑       ↑         ↑
Admin    Admin setzt    Community          Admin   Admin    Auto nach
erstellt  Datum/Zeit    reicht Fragen      startet  beendet  7 Tagen
                        ein & votet
```

#### AMA-Features

| Feature | Details |
|---|---|
| **Question Submission** | User können Fragen vorab einreichen (max 500 chars) |
| **Question Upvotes** | Community votet auf Fragen → Experte beantwortet Top-Fragen zuerst |
| **Live Answers** | Experte beantwortet Fragen in Echtzeit (Admin-Interface) |
| **Transcript** | Nach dem AMA: Alle Q&As werden als lesbares Archiv gespeichert |
| **Reminder** | User können einen Reminder setzen → In-App Notification |
| **Max Questions** | 100 Fragen pro AMA (verhindert Spam) |

---

### 2.5. Asset Reviews & Ratings

Reviews sind **nur für verifizierte Investoren** sichtbar — du kannst nur ein Asset bewerten, in das du tatsächlich investiert hast.

#### Review-Regeln

| Regel | Begründung |
|---|---|
| Nur KYC-verifizierte User | Spam-Prävention |
| Nur für eigene Investments | Fake-Reviews verhindern |
| 1 Review pro User pro Asset | Keine Mehrfachbewertungen |
| 1-5 Sterne + Freitext (50-2000 Chars) | Qualitätssicherung |
| "Verified Investor" Badge automatisch | Vertrauen |
| "Received Dividend" Badge falls zutreffend | Extra-Vertrauen |
| Admin kann Reviews entfernen (mit Begründung) | Moderation |
| Durchschnittsbewertung auf Asset-Seite anzeigen | Social Proof |

---

### 2.6. Direct Messages (DMs)

**Phase 2 Feature** — Nicht im Launch enthalten. DMs erhöhen das Missbrauchsrisiko (Spam, Scam, unerlaubte Anlageberatung) erheblich.

#### DM-Regeln (wenn implementiert)

- Nur zwischen Usern die sich gegenseitig folgen (keine Cold-DMs)
- Oder: Nur zwischen Usern die in dasselbe Asset investiert haben
- Rate Limit: Max 20 DMs pro Stunde
- Automatische Scam-Keyword-Erkennung
- Report-Button pro Nachricht

---

### 2.7. Announcements & Notifications

#### Announcement-System

Announcements werden von **Admins** oder **Asset-Operatoren** erstellt.

| Kategorie | Ersteller | Beispiel |
|---|---|---|
| `new_commodity` | Admin | "Neues Asset: Kalimantan Timber!" |
| `dividend` | Admin/System | "Q1 Dividend für Bali Cocoa verteilt" |
| `platform_update` | Admin | "Community Feature ist live!" |
| `market_news` | Admin | "Cocoa-Preise erreichen 5-Jahres-Hoch" |
| `farm_update` | Developer | "Month 6 Harvest Report" |

#### Notification-System (In-App)

| Event | Notification |
|---|---|
| Neuer Follower | "Sarah F. folgt dir jetzt" |
| Reaction auf deinen Post | "Alex K. hat 🔥 auf deinen Post reagiert" |
| Kommentar auf deinen Post | "Emma W. hat deinen Post kommentiert" |
| Neues Badge verdient | "🏆 Du hast 'Diversified Investor' verdient!" |
| AMA Reminder | "AMA 'Cocoa Supply Chains' startet in 1 Stunde" |
| Neues Announcement | "📢 Neues Asset: Kalimantan Timber" |
| Review hilfreich markiert | "12 Personen fanden dein Review hilfreich" |

> **Wichtig:** Notifications werden in der **Core-DB** gespeichert (bestehende `notifications`-Tabelle), da sie cross-system sind (Badge von Community, aber Notification-Delivery über Core).

---

### 2.8. Community Profiles

Jeder User hat ein Community-Profil, das aus **Core-DB + Community-DB** zusammengesetzt wird.

#### Profil-Daten

| Feld | Quelle | Editierbar |
|---|---|---|
| Display Name | Core-DB `users.display_name` | Ja (Settings) |
| Avatar | Core-DB/GCS | Ja (Settings) |
| Bio | Community-DB `community_profiles.bio` | Ja (Community Settings) |
| Tier Badge | Core-DB `users.tier` | Nein (automatisch) |
| KYC Badge | Core-DB `kyc_records.status` | Nein (automatisch) |
| Earned Badges | Community-DB `user_badges` | Nein (automatisch) |
| Posts Count | Community-DB COUNT(`posts`) | Nein |
| Followers Count | Community-DB COUNT(`follows`) | Nein |
| Following Count | Community-DB COUNT(`follows`) | Nein |
| Member Since | Core-DB `users.created_at` | Nein |

---

### 2.9. Content Moderation & Compliance

#### Automatische Moderation

| Check | Aktion | Details |
|---|---|---|
| **HTML-Sanitization** | Immer | Ammonia v4.1 stripped alle unsicheren Tags |
| **Spam-Detection** | Auto-Flag | >5 Posts in 10 Minuten → Rate-Limited |
| **Keyword-Filter** | Auto-Flag | Wörter wie "guaranteed returns", "risk-free", "buy now" → Review-Queue |
| **URL-Filter** | Auto-Flag | External URLs → Admin-Review (Phishing-Schutz) |
| **Duplicate-Detection** | Auto-Block | Identischer Post-Inhalt innerhalb 1 Stunde |
| **Investment-Disclaimer** | Auto-Append | Posts mit Investment-Keywords → Disclaimer angehängt |

#### Manuelle Moderation

| Aktion | Wer | Ergebnis |
|---|---|---|
| **Report** | Jeder User | Post/Comment wird in Moderation-Queue geschoben |
| **Hide** | Moderator/Admin | Post wird unsichtbar, User wird benachrichtigt |
| **Delete** | Admin | Post wird gelöscht (Audit-Log Eintrag) |
| **Warn** | Admin | User bekommt Warnung (3 Warnings → Temp-Ban) |
| **Temp-Ban** | Admin | User kann 7 Tage nicht posten |
| **Perm-Ban** | Super-Admin | User wird permanent aus Community ausgeschlossen |

#### Compliance-Disclaimer

Jeder Post der Investment-Keywords enthält bekommt automatisch:

```html
<div class="community-disclaimer">
  ⚠️ This is a user opinion, not financial advice.
  Investments involve risk, including potential loss of capital.
  Always do your own research before investing.
</div>
```

---

### 2.10. Leaderboard & Challenges

**Challenges** sind zeitlich begrenzte Gruppenziele für Circles:

| Challenge | Ziel | Belohnung |
|---|---|---|
| March Growth Sprint | 3 neue Circle-Members investieren | 2% Bonus-Cashback für den Circle |
| Review Champion | Circle schreibt 5 Reviews in einem Monat | Exklusives "Reviewer" Badge |
| Community Builder | Circle erreicht zusammen 50 Follower | Leaderboard-Boost für 1 Woche |

---

## 3. Datenbank-Schema (Community-DB)

> **Alle Tabellen leben in der separaten Community-DB (`pool_community`).** Keine Foreign Keys zur Core-DB — nur `user_id` als UUID-Referenz.

### Migration-Reihenfolge

```
community_001_posts.sql
community_002_comments.sql
community_003_reactions.sql
community_004_follows.sql
community_005_badges.sql
community_006_reviews.sql
community_007_amas.sql
community_008_announcements.sql
community_009_reports.sql
community_010_messages.sql
community_011_profiles.sql
community_012_challenges.sql
community_013_circles.sql          ← NEU: Circles + Circle Members
community_014_xp_ledger.sql        ← NEU: XP-System (Append-Only Ledger)
community_015_circle_invites.sql   ← NEU: Manuelle Circle-Einladungen
community_016_indexes.sql
```

### 3.1. Kern-Tabellen

```sql
-- community_001_posts.sql
CREATE TABLE posts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,  -- Referenz auf Core-DB users.id (kein FK!)
    post_type       VARCHAR(20) NOT NULL DEFAULT 'general'
                    CHECK (post_type IN ('general', 'market_insight', 'milestone',
                                         'farm_update', 'announcement', 'review')),
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 5000),
    content_sanitized TEXT,          -- Ammonia-bereinigter Content
    asset_id        UUID,            -- Optional: Referenz auf Core-DB assets.id
    image_urls      TEXT[],          -- GCS-Pfade zu Post-Bildern (max 4)
    is_pinned       BOOLEAN NOT NULL DEFAULT false,
    is_hidden       BOOLEAN NOT NULL DEFAULT false,
    hidden_reason   TEXT,
    disclaimer_shown BOOLEAN NOT NULL DEFAULT false,
    reaction_count  INTEGER NOT NULL DEFAULT 0,   -- Denormalisiert für Performance
    comment_count   INTEGER NOT NULL DEFAULT 0,   -- Denormalisiert für Performance
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_created_at ON posts(created_at DESC);
CREATE INDEX idx_posts_post_type ON posts(post_type);
CREATE INDEX idx_posts_asset_id ON posts(asset_id) WHERE asset_id IS NOT NULL;

-- community_002_comments.sql
CREATE TABLE comments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 2000),
    content_sanitized TEXT,
    helpful_count   INTEGER NOT NULL DEFAULT 0,
    is_hidden       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);

-- community_003_reactions.sql
CREATE TABLE reactions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    reaction_type   VARCHAR(20) NOT NULL
                    CHECK (reaction_type IN ('fire', 'insightful', 'clap', 'green')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, user_id, reaction_type)  -- 1 Reaction pro Typ pro User
);

CREATE INDEX idx_reactions_post_id ON reactions(post_id);
CREATE INDEX idx_reactions_user_id ON reactions(user_id);

-- Trigger: Update denormalized count on posts
CREATE OR REPLACE FUNCTION update_reaction_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE posts SET reaction_count = reaction_count + 1 WHERE id = NEW.post_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE posts SET reaction_count = reaction_count - 1 WHERE id = OLD.post_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_reaction_count
AFTER INSERT OR DELETE ON reactions
FOR EACH ROW EXECUTE FUNCTION update_reaction_count();

-- community_004_follows.sql
CREATE TABLE follows (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_id     UUID NOT NULL,  -- Wer folgt
    following_id    UUID NOT NULL,  -- Wem gefolgt wird
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(follower_id, following_id),
    CHECK(follower_id != following_id)  -- Kann sich nicht selbst folgen
);

CREATE INDEX idx_follows_follower ON follows(follower_id);
CREATE INDEX idx_follows_following ON follows(following_id);

-- community_005_badges.sql
CREATE TABLE badges (
    id              VARCHAR(50) PRIMARY KEY,  -- z.B. 'first_investor', 'connector'
    name            VARCHAR(100) NOT NULL,
    description     TEXT NOT NULL,
    icon            VARCHAR(10) NOT NULL,     -- Emoji
    category        VARCHAR(20) NOT NULL
                    CHECK (category IN ('investment', 'community', 'verified', 'tier')),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE user_badges (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    badge_id        VARCHAR(50) NOT NULL REFERENCES badges(id),
    earned_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, badge_id)
);

CREATE INDEX idx_user_badges_user ON user_badges(user_id);

-- community_006_reviews.sql
CREATE TABLE reviews (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    asset_id        UUID NOT NULL,       -- Core-DB assets.id
    rating          SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    title           VARCHAR(200),
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 50 AND 2000),
    content_sanitized TEXT,
    is_verified_investor BOOLEAN NOT NULL DEFAULT false,
    has_received_dividend BOOLEAN NOT NULL DEFAULT false,
    helpful_count   INTEGER NOT NULL DEFAULT 0,
    is_hidden       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, asset_id)            -- 1 Review pro User pro Asset
);

CREATE INDEX idx_reviews_asset ON reviews(asset_id);
CREATE INDEX idx_reviews_user ON reviews(user_id);

-- community_007_amas.sql
CREATE TABLE amas (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title           VARCHAR(300) NOT NULL,
    description     TEXT,
    expert_name     VARCHAR(200) NOT NULL,
    expert_title    VARCHAR(300),
    expert_avatar_url TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'scheduled', 'accepting_questions',
                                      'live', 'closed', 'archived')),
    scheduled_at    TIMESTAMPTZ,
    started_at      TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    max_questions   INTEGER NOT NULL DEFAULT 100,
    created_by      UUID NOT NULL,       -- Admin user_id
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE ama_questions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ama_id          UUID NOT NULL REFERENCES amas(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    question        TEXT NOT NULL CHECK (char_length(question) BETWEEN 10 AND 500),
    answer          TEXT,                -- Filled in by expert during AMA
    answered_by     UUID,               -- Expert/Admin user_id
    answered_at     TIMESTAMPTZ,
    upvote_count    INTEGER NOT NULL DEFAULT 0,
    is_featured     BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE ama_question_upvotes (
    question_id     UUID NOT NULL REFERENCES ama_questions(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(question_id, user_id)
);

-- community_008_announcements.sql
-- (Posts mit post_type = 'announcement' reichen hier, aber wir brauchen Kategorien)
CREATE TABLE announcement_categories (
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    category        VARCHAR(30) NOT NULL
                    CHECK (category IN ('new_commodity', 'dividend', 'platform_update',
                                        'market_news', 'farm_update')),
    PRIMARY KEY(post_id)
);

-- community_009_reports.sql
CREATE TABLE content_reports (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id     UUID NOT NULL,
    content_type    VARCHAR(20) NOT NULL CHECK (content_type IN ('post', 'comment', 'review', 'message')),
    content_id      UUID NOT NULL,
    reason          VARCHAR(50) NOT NULL
                    CHECK (reason IN ('spam', 'harassment', 'misinformation',
                                      'financial_advice', 'scam', 'inappropriate', 'other')),
    description     TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'reviewed', 'action_taken', 'dismissed')),
    reviewed_by     UUID,
    reviewed_at     TIMESTAMPTZ,
    action_taken    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reports_status ON content_reports(status) WHERE status = 'pending';

-- community_011_profiles.sql
CREATE TABLE community_profiles (
    user_id         UUID PRIMARY KEY,    -- 1:1 mit Core-DB users.id
    bio             TEXT CHECK (char_length(bio) <= 300),
    circle_id       UUID,                -- Welchem Circle gehört dieser User an?
    xp_total        INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert aus xp_ledger
    level           SMALLINT NOT NULL DEFAULT 1,     -- Berechnet aus xp_total (1-10)
    level_name      VARCHAR(30) NOT NULL DEFAULT 'Newcomer',
    is_community_banned BOOLEAN NOT NULL DEFAULT false,
    ban_reason      TEXT,
    ban_expires_at  TIMESTAMPTZ,
    warning_count   INTEGER NOT NULL DEFAULT 0,
    post_count      INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    follower_count  INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    following_count INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    login_streak    INTEGER NOT NULL DEFAULT 0,      -- Aktuelle Login-Streak in Tagen
    last_login_date DATE,                            -- Letzter Login-Tag (für Streak)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_profiles_circle ON community_profiles(circle_id) WHERE circle_id IS NOT NULL;
CREATE INDEX idx_profiles_xp ON community_profiles(xp_total DESC);
CREATE INDEX idx_profiles_level ON community_profiles(level);

-- community_012_challenges.sql
CREATE TABLE challenges (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(200) NOT NULL,
    description     TEXT NOT NULL,
    challenge_type  VARCHAR(30) NOT NULL
                    CHECK (challenge_type IN ('circle_growth', 'review_count',
                                              'community_activity', 'custom')),
    target_value    INTEGER NOT NULL,    -- z.B. "3 neue Members"
    reward_description TEXT NOT NULL,
    reward_xp       INTEGER NOT NULL DEFAULT 100,   -- XP-Belohnung bei Abschluss
    min_circle_level SMALLINT NOT NULL DEFAULT 1,   -- Mindest-Circle-Level
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE challenge_progress (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id    UUID NOT NULL REFERENCES challenges(id),
    circle_id       UUID NOT NULL REFERENCES circles(id),  -- Circle, nicht User!
    current_value   INTEGER NOT NULL DEFAULT 0,
    is_completed    BOOLEAN NOT NULL DEFAULT false,
    completed_at    TIMESTAMPTZ,
    UNIQUE(challenge_id, circle_id)
);

-- community_013_circles.sql
CREATE TABLE circles (
    id              UUID PRIMARY KEY,    -- = owner_user_id (jeder User = 1 Circle)
    owner_id        UUID NOT NULL,       -- Core-DB users.id (= id)
    name            VARCHAR(100) NOT NULL DEFAULT '',  -- Automatisch: "{User}'s Circle"
    description     TEXT CHECK (char_length(description) <= 500),
    banner_url      TEXT,                -- GCS-Pfad für Custom-Banner (ab Level 7)
    is_public       BOOLEAN NOT NULL DEFAULT false,  -- Andere können beitreten (ab Level 5)
    xp_total        INTEGER NOT NULL DEFAULT 0,      -- SUM aller Member-XP (denormalisiert)
    level           SMALLINT NOT NULL DEFAULT 1,     -- Berechnet aus xp_total (1-10)
    member_count    INTEGER NOT NULL DEFAULT 0,      -- Denormalisiert
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_circles_xp ON circles(xp_total DESC);
CREATE INDEX idx_circles_level ON circles(level);
CREATE INDEX idx_circles_public ON circles(is_public) WHERE is_public = true;

CREATE TABLE circle_members (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id       UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,       -- Core-DB users.id
    joined_via      VARCHAR(20) NOT NULL DEFAULT 'referral'
                    CHECK (joined_via IN ('referral', 'invite', 'public_join')),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)                      -- 1 User = 1 Circle (IMPORTANT!)
);

CREATE INDEX idx_circle_members_circle ON circle_members(circle_id);
CREATE INDEX idx_circle_members_user ON circle_members(user_id);

-- Trigger: Update denormalized member_count
CREATE OR REPLACE FUNCTION update_circle_member_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE circles SET member_count = member_count + 1, updated_at = NOW() WHERE id = NEW.circle_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE circles SET member_count = member_count - 1, updated_at = NOW() WHERE id = OLD.circle_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_circle_member_count
AFTER INSERT OR DELETE ON circle_members
FOR EACH ROW EXECUTE FUNCTION update_circle_member_count();

-- community_014_xp_ledger.sql
-- APPEND-ONLY: Niemals UPDATE oder DELETE auf dieser Tabelle!
CREATE TABLE xp_ledger (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,       -- Core-DB users.id
    amount          INTEGER NOT NULL,    -- Positiv = verdient, Negativ = Abzug
    reason          VARCHAR(30) NOT NULL
                    CHECK (reason IN (
                        'signup', 'kyc_verify', 'first_invest', 'invest',
                        'dividend', 'post_create', 'post_delete',
                        'comment_create', 'comment_delete',
                        'reaction_give', 'follower_gain', 'follower_lose',
                        'helpful_receive', 'review_create', 'review_delete',
                        'ama_question', 'ama_answered',
                        'circle_join', 'circle_leave',
                        'challenge_complete', 'daily_login',
                        'streak_7day', 'streak_30day',
                        'admin_adjust', 'ban_penalty'
                    )),
    source_id       UUID,                -- Optional: ID des Posts/Comments/etc.
    source_type     VARCHAR(20),         -- Optional: 'post', 'comment', 'reaction', etc.
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_xp_user ON xp_ledger(user_id);
CREATE INDEX idx_xp_created ON xp_ledger(created_at DESC);
CREATE INDEX idx_xp_user_reason_date ON xp_ledger(user_id, reason, created_at DESC);
-- Für Daily-Cap-Check: Wie viele XP hat User X heute durch Reason Y verdient?

-- community_015_circle_invites.sql
CREATE TABLE circle_invites (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id       UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    inviter_id      UUID NOT NULL,       -- Circle-Owner
    invitee_id      UUID NOT NULL,       -- Eingeladener User
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'accepted', 'declined', 'expired')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    responded_at    TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days'),
    UNIQUE(circle_id, invitee_id)        -- Keine Doppel-Einladungen
);

CREATE INDEX idx_invites_invitee ON circle_invites(invitee_id) WHERE status = 'pending';
```

**Gesamtzahl Tabellen: 21** (posts, comments, reactions, follows, badges, user_badges, reviews, amas, ama_questions, ama_question_upvotes, announcement_categories, content_reports, community_profiles, challenges, challenge_progress, circles, circle_members, xp_ledger, circle_invites + ggf. helpful_votes, notification_preferences)

---

## 4. API-Endpunkte

### 4.1. Übersicht aller Community-APIs (~43 Endpunkte)

#### Feed & Posts (12 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/feed` | Paginated Feed (eigene + gefolgte + announcements) | ✅ |
| `GET` | `/api/community/feed/global` | Globaler Feed (alle Posts) | ✅ |
| `POST` | `/api/community/posts` | Neuen Post erstellen | ✅ + KYC |
| `GET` | `/api/community/posts/{id}` | Einzelnen Post laden | ✅ |
| `PUT` | `/api/community/posts/{id}` | Post bearbeiten (nur eigene, in 15min) | ✅ |
| `DELETE` | `/api/community/posts/{id}` | Post löschen (nur eigene) | ✅ |
| `POST` | `/api/community/posts/{id}/reactions` | Reaction hinzufügen/entfernen (toggle) | ✅ |
| `GET` | `/api/community/posts/{id}/comments` | Kommentare zu einem Post | ✅ |
| `POST` | `/api/community/posts/{id}/comments` | Kommentar erstellen | ✅ |
| `DELETE` | `/api/community/comments/{id}` | Kommentar löschen (eigene) | ✅ |
| `POST` | `/api/community/comments/{id}/helpful` | "Helpful" markieren (toggle) | ✅ |
| `POST` | `/api/community/posts/{id}/report` | Post melden | ✅ |

#### Follows (4 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `POST` | `/api/community/follow/{user_id}` | User folgen | ✅ |
| `DELETE` | `/api/community/follow/{user_id}` | Entfolgen | ✅ |
| `GET` | `/api/community/users/{id}/followers` | Follower-Liste | ✅ |
| `GET` | `/api/community/users/{id}/following` | Following-Liste | ✅ |

#### Profiles (3 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/profile/{user_id}` | Community-Profil (public) | ✅ |
| `PUT` | `/api/community/profile` | Eigenes Profil bearbeiten (Bio) | ✅ |
| `GET` | `/api/community/profile/{user_id}/badges` | Badges eines Users | ✅ |

#### Reviews (5 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/reviews?asset_id=` | Reviews für ein Asset | ✅ |
| `POST` | `/api/community/reviews` | Review erstellen (nur verified investors) | ✅ + KYC + Investment |
| `PUT` | `/api/community/reviews/{id}` | Review bearbeiten (nur eigene) | ✅ |
| `DELETE` | `/api/community/reviews/{id}` | Review löschen (nur eigene) | ✅ |
| `POST` | `/api/community/reviews/{id}/helpful` | "Helpful" markieren | ✅ |

#### AMAs (7 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/amas` | Alle AMAs (upcoming + past) | ✅ |
| `GET` | `/api/community/amas/{id}` | Einzelnes AMA mit Fragen | ✅ |
| `POST` | `/api/community/amas/{id}/questions` | Frage einreichen | ✅ |
| `POST` | `/api/community/amas/{id}/questions/{qid}/upvote` | Frage upvoten | ✅ |
| `POST` | `/api/community/amas/{id}/remind` | Reminder setzen | ✅ |
| `POST` | `/api/admin/community/amas` | AMA erstellen (Admin) | ✅ Admin |
| `POST` | `/api/admin/community/amas/{id}/answer/{qid}` | Frage beantworten (Admin) | ✅ Admin |

#### Announcements (3 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/announcements` | Announcements mit Filter | ✅ |
| `POST` | `/api/admin/community/announcements` | Announcement erstellen | ✅ Admin |
| `PUT` | `/api/admin/community/announcements/{id}/pin` | Announcement pinnen/unpinnen | ✅ Admin |

#### Moderation/Admin (6 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/admin/community/reports` | Pending Reports | ✅ Admin |
| `POST` | `/api/admin/community/reports/{id}/action` | Report bearbeten (hide/delete/warn/ban) | ✅ Admin |
| `GET` | `/api/admin/community/stats` | Community-Statistiken | ✅ Admin |
| `POST` | `/api/admin/community/users/{id}/ban` | User bannen | ✅ Admin |
| `POST` | `/api/admin/community/users/{id}/unban` | User entbannen | ✅ Admin |
| `POST` | `/api/admin/community/posts/{id}/hide` | Post verstecken | ✅ Admin |

#### Circles (9 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/circle` | My Circle (eigener Circle: Members, Stats, XP, Level) | ✅ |
| `GET` | `/api/community/circles/{id}` | Anderer Circle (öffentlich) | ✅ |
| `PUT` | `/api/community/circle` | Circle bearbeiten (Name, Description — ab Level 4) | ✅ |
| `GET` | `/api/community/circle/members` | Members meines Circles | ✅ |
| `POST` | `/api/community/circle/invite/{user_id}` | User in Circle einladen (ab Level 6) | ✅ |
| `POST` | `/api/community/circle/invites/{id}/respond` | Einladung annehmen/ablehnen | ✅ |
| `GET` | `/api/community/circle/invites` | Meine offenen Einladungen | ✅ |
| `POST` | `/api/community/circle/leave` | Circle verlassen | ✅ |
| `DELETE` | `/api/community/circle/members/{user_id}` | Member kicken (nur Owner) | ✅ |

#### XP & Levels (3 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/xp` | Eigene XP-Übersicht (Total, Level, History) | ✅ |
| `GET` | `/api/community/xp/history` | XP-Ledger (letzte 50 Einträge) | ✅ |
| `GET` | `/api/community/xp/leaderboard` | User-XP-Ranking (Top 50) | ✅ |

#### Challenges & Leaderboard (4 Endpunkte)

| Method | Path | Beschreibung | Auth |
|---|---|---|---|
| `GET` | `/api/community/challenges` | Aktive Challenges (gefiltert nach Circle-Level) | ✅ |
| `POST` | `/api/community/challenges/{id}/join` | Challenge für meinen Circle annehmen | ✅ |
| `GET` | `/api/community/leaderboard` | Circle-Leaderboard (nach XP gerankt) | ✅ |
| `GET` | `/api/community/leaderboard/weekly` | Wöchentliches Circle-Ranking | ✅ |

**Gesamt: ~56 API-Endpunkte** (43 User-facing + 13 Admin/System)

---

## 5. Frontend-Architektur

### 5.1. Datei-Struktur

```
frontend/platform/
├── community.html                    # Bestehendes HTML (wird zu MiniJinja-Template)
├── static/
│   ├── css/
│   │   ├── community.css             # ✅ Existiert — erweitern
│   │   └── community-card.css        # ✅ Existiert
│   └── js/
│       ├── community-feed.js         # Feed laden, Posts, Reactions, Comments
│       ├── community-announcements.js # Announcements Tab
│       ├── community-circle.js       # My Circle Tab + Leaderboard
│       ├── community-amas.js         # Expert AMAs Tab
│       ├── community-reviews.js      # Reviews Tab
│       └── community-profile.js      # User Profile Modal
```

### 5.2. Umstellung von statisch auf dynamisch

Die bestehende `community.html` enthält hart-kodierte Demo-Daten. Die Umstellung erfolgt in 3 Schritten:

1. **HTML-Struktur beibehalten** — das Markup ist gut, nur die Daten werden ausgetauscht
2. **JavaScript-Module erstellen** — pro Tab ein JS-Modul das per `fetch()` die API aufruft
3. **DOM-Rendering** — Templates im JS, kein `innerHTML` mit User-Daten (XSS-Schutz!)

---

## 6. Entwickler-Perspektiven

### 6.1. Backend-Engineer (Rust/Axum)

**Neues Modul:** `backend/src/community/`

```
community/
├── mod.rs              # Modul-Registrierung, Router
├── models.rs           # Alle Community-Structs (Post, Comment, Reaction, etc.)
├── routes.rs           # HTTP-Handler (thin — delegieren an service)
├── service.rs          # Business-Logik (Feed-Algorithmus, Badge-Berechnung, etc.)
├── validation.rs       # Input-Validierung (Content-Länge, Rate-Limits, etc.)
├── moderation.rs       # Content-Moderation (Keyword-Filter, Spam-Detection)
├── user_bridge.rs      # Cross-DB User-Lookup mit Batch + Redis-Cache
└── background.rs       # Badge-Worker, Feed-Cache-Worker
```

**Kritische Patterns:**   
- **User-Bridge:** Alle User-Daten kommen aus Core-DB via Batch-Lookup. NIEMALS direkte JOINs
- **Sanitization:** JEDER User-Input durch Ammonia HTML-Sanitizer
- **Rate-Limiting:** Redis-basiert — max 5 Posts/Stunde, max 30 Reactions/Minute
- **Dual Pool:** `pool_community` für alle Community-Queries, `pool_core` nur für User-Lookups

### 6.2. Frontend-Engineer (Vanilla JS)

- Bestehende CSS/HTML-Struktur beibehalten und erweitern
- Pro Tab ein separates JS-Modul — kein globaler State
- `textContent` für User-Daten, DOM-Construction für komplexe Elemente
- Optimistic UI für Reactions (sofort visuelles Feedback, Server-Bestätigung asynchron)
- Infinite-Scroll für Feed mit Intersection Observer

### 6.3. DevOps-Engineer

- Cloud SQL Community-DB aufsetzen (Phase 0.2 im Hauptroadmap)
- Migrations in separatem Verzeichnis: `database/community/`
- Monitoring: Post-Throughput, Report-Queue-Länge, DB-Latenz
- GCS-Bucket für Community-Uploads konfigurieren

---

## 7. Sicherheit & Compliance

### 7.1. Regulatorische Risiken

| Risiko | Beschreibung | Mitigierung |
|---|---|---|
| **Unerlaubte Anlageberatung** | User gibt konkreten Kauf-Rat | Automatischer Disclaimer + Keyword-Filter |
| **Marktmanipulation** | Koordinierter "Pump" eines Assets via Community | Monitoring: Wenn 10+ Posts zum selben Asset in 1h → Admin-Alert |
| **Fake Reviews** | User gibt 5-Sterne Review ohne Investment | Nur verifizierte Investoren können Reviews schreiben |
| **Privacy/GDPR** | User-Daten in Community sichtbar | Opt-in Community-Profile, Anonymisierung bei Account-Löschung |
| **Scam/Phishing** | User sendet Phishing-Links in Posts/DMs | URL-Filter, externe Links markiert, Report-System |

### 7.2. GDPR-Konformität

Bei Account-Löschung in Core-DB:
1. Community-Profil anonymisieren: `display_name → "Deleted User"`, Bio löschen
2. Posts behalten aber anonymisieren (für Thread-Kohärenz)
3. DMs: Beidseitig löschen
4. Reviews: Anonymisieren (Rating bleibt für Durchschnitt)
5. Follows: Auflösen
6. Badges: Löschen

---

## 8. Infrastruktur & Performance

### 8.1. Lastprofil

| Kennzahl | Launch (50 User) | 6 Monate (500 User) | 12 Monate (2.000 User) |
|---|---|---|---|
| Posts/Tag | ~20 | ~200 | ~1.000 |
| Comments/Tag | ~50 | ~500 | ~2.500 |
| Reactions/Tag | ~200 | ~2.000 | ~10.000 |
| Feed-Requests/Tag | ~500 | ~5.000 | ~25.000 |
| DB-Writes/Tag (gesamt) | ~300 | ~3.000 | ~15.000 |

### 8.2. Caching-Strategie

| Cache-Key | TTL | Inhalt |
|---|---|---|
| `community:user:{id}` | 5 min | Display Name, Avatar, Tier, KYC |
| `community:feed:global:page:{n}` | 30 sec | Global Feed Page |
| `community:feed:user:{id}:page:{n}` | 30 sec | Personal Feed Page |
| `community:post:{id}:reactions` | 1 min | Reaction Counts pro Post |
| `community:asset:{id}:rating` | 10 min | Average Rating |
| `community:leaderboard` | 5 min | Top 20 Circles |
| `community:ratelimit:post:{user_id}` | 1 hour | Post-Counter für Rate-Limit |

### 8.3. Kosten (inkrementell zum Hauptsystem)

| Phase | DB-Tier | Kosten/Monat |
|---|---|---|
| Launch (0-50 User) | `db-f1-micro` | ~$10 |
| Growth (50-500 User) | `db-g1-small` | ~$25 |
| Scale (500-2.000 User) | `db-custom-2-4096` | ~$60 |

> **Fazit:** Die Community-Infrastruktur ist im Vergleich zum Trading-System billig. Die größte Investition ist die **Entwicklungszeit** (~4-6 Wochen für einen Full-Stack-Engineer), nicht die Cloud-Kosten.

---

*Dieses Dokument ist die Grundlage für `COMMUNITY_ROADMAP.md`. Letzte Aktualisierung: 2026-03-21.*
