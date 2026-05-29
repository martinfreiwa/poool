# POOOL Circles Product Vision and Implementation Roadmap

Datum: 2026-05-20  
Status: Product/Engineering Specification, updated with implementation status on 2026-05-21  
Related Audit: `docs/community-production-readiness-audit.md`

## Implementation Status Update 2026-05-21

The implementation roadmap below is no longer only theoretical. Phases 1-9 have been implemented as a vertical product/ops slice:

- `/community/circle/:slug` opens the Circle Feed, not Settings.
- `/community/circle/:slug/settings` remains the role-gated Manage surface.
- `/community/circles` is the canonical My-Circles/Discover page.
- Circle posts, mentions, content types, Q&A/knowledge status, announcements, AMAs/events, challenges, onboarding, asset circles, holder-only resources, resource versions, manage settings, report queues, ops alerts, and platform-admin alert actions exist in the local codebase.
- Browser-backed verification now covers Circle Settings, Resource Library upload/replacement/delivery/version restore/review, private-GCS stream delivery through a development fake root, multi-user resource access denial, My-Circles-to-Feed-to-Post-to-Comment-to-Settings journey, mobile Circle surfaces, and axe accessibility checks.
- A local-live provider preflight now exists at `scripts/local-live/community-circles-staging-preflight.sh`; it validates staging env shape, Slack/PagerDuty URL safety, fake-GCS exclusion, webhook unit tests, and focused Circles static contracts before a real staging alert is sent.
- Provider close-loop artifacts now exist at `docs/community-circles-provider-runbook.md`, with seed and receipt-check scripts for one synthetic staging alert.
- Legal/compliance launch criteria now exist at `docs/community-circles-compliance-checklist.md`.
- The remaining work before Production Live is operational: run the provider runbook with real staging Slack/PagerDuty event IDs and complete legal/compliance sign-off for investment-community language.

The authoritative post-implementation evidence is `docs/community-circles-post-implementation-audit.md`.

## 1. Core Decision

POOOL Circles werden als eigenstaendige Investment-Communities innerhalb der Plattform definiert. Ein Circle ist nicht die Settings-Seite und nicht nur ein Verwaltungsobjekt. Ein Circle ist ein eigener sozialer Investment-Space fuer Diskussion, Q&A, Updates, Events, Wissen, Mitglieder und assetbezogene Aktivitaet.

Die wichtigste UX-Regel:

> Circle click opens the Circle Feed. Settings are only a secondary Manage surface.

Zielzustand:

```text
Community
-> My Circles
-> Circle anklicken
-> Circle Feed
-> Manage/Settings nur fuer berechtigte Rollen
```

Nicht mehr:

```text
Community
-> My Circle
-> Circle anklicken
-> Circle Settings
```

## 2. Original Code Reality Check

This section preserves the pre-implementation baseline that motivated the work. It is no longer the current code state; the post-implementation audit is the source of truth for implemented evidence.

| Bereich | Aktueller Stand | Konsequenz |
|---|---|---|
| Circle Links | `frontend/platform/static/js/community-circles-discover.js` verlinkt Spotlight, Cards und Pills bereits nach `/community/circle/:slug`. | Die UI ist schon auf eine Feed-URL ausgerichtet. |
| Circle Route | `backend/src/lib.rs` leitet `/community/circle/:slug` aktuell auf `/community/circle/:slug/settings` weiter. | Der Hauptfehler liegt serverseitig im Routing/IA. |
| Settings Page | `frontend/platform/community-circle-settings.html` ist ein vollwertiger Settings-Bereich mit "View feed"-CTA. | Settings sind als Funktion wertvoll, aber als Default-Ziel falsch. |
| Circle Lookup | `/api/community/circles/by-slug/:slug` liefert Circle und `my_role`. | Basis fuer Circle Header, Rolle und Manage-Sichtbarkeit ist vorhanden. |
| My Circles API | `/api/community/me/circles` existiert. | My-Circles-Uebersicht ist technisch vorbereitet. |
| Circle Membership | Rollen `owner`, `admin`, `moderator`, `member` sind im Rework angelegt. | Rollenmodell ist nah am Ziel, braucht aber UI-/Permission-Matrix. |
| Posts | `posts` hat aktuell kein `circle_id`; `CreatePostRequest` hat kein Circle-Ziel. | Echter Circle Feed braucht Datenbank- und API-Erweiterung. |
| Token Gates | Token-gated Circles existieren als Datenmodell. | Investment-Integration hat bereits eine technische Basis. |

Wichtig: Phase 1 ist nicht nur ein Redirect-Fix. Ohne `posts.circle_id` waere ein Circle Feed nur eine leere Shell oder ein globaler Feed mit falschem Kontext.

## 3. Product Positioning

Interne Definition:

> POOOL Circles are dedicated community spaces where members, investors and experts discuss assets, markets, opportunities and platform topics in public, private or ownership-gated groups.

Deutsch:

> POOOL Circles sind eigenstaendige Community-Spaces innerhalb der Plattform, in denen Nutzer, Investoren und Experten ueber Assets, Maerkte, Chancen und Plattformthemen diskutieren - oeffentlich, privat oder nur fuer berechtigte Mitglieder.

POOOL sollte keine generische Social-App bauen. Der Fokus ist:

```text
Community around assets, investors, knowledge and ownership.
```

Das heisst:

- weniger zufaellige Social Posts,
- mehr strukturierte Diskussion,
- mehr Asset- und Due-Diligence-Bezug,
- mehr Expertenantworten,
- mehr offizielle Updates,
- mehr Vertrauen und Moderation,
- mehr Verbindung zu Portfolio, Marketplace und Ownership.

## 4. Competitive Pattern Mapping

Diese Tabelle uebernimmt nicht blind Fremdprodukte, sondern extrahiert Muster, die fuer POOOL relevant sind.

| Quelle | Relevantes Muster | POOOL-Interpretation |
|---|---|---|
| Facebook Groups | Gruppen unterscheiden public/private; Content und Mitglieder stehen vor Admin-Verwaltung. Quelle: https://www.facebook.com/help/220336891328465?locale=en_GB | Circle Default ist Feed/Header/Members, nicht Settings. Privacy muss klar sichtbar sein. |
| Reddit Post Flair | Post Flair kategorisiert Content-Typen und Subtopics; Flair kann als Navigation/Filter dienen. Quelle: https://support.reddithelp.com/hc/en-us/articles/15484545678996-Post-Flair | POOOL braucht Post Types und Circle-Flairs wie Question, Market Insight, Risk, Due Diligence, Property Update. |
| Reddit User Flair | User Flair macht Rollen, Vertrauen oder Expertise sichtbar. Quelle: https://support.reddithelp.com/hc/en-us/articles/15484503095060-User-Flair | POOOL braucht sichtbare Rollen wie Verified Investor, Asset Holder, Expert, Moderator, Official. |
| Discord Forum Channels | Forum Channels strukturieren laengere Diskussionen und nutzen Tags/Filter. Quelle: https://support.discord.com/hc/en-us/articles/6208479917079-Forum-Channels-FAQ | Circle Q&A und Due-Diligence-Threads duerfen nicht im Feed verschwinden. |
| Discord Stage Channels | Stage Channels sind fuer AMAs, Fireside Chats und Townhalls gedacht. Quelle: https://support.discord.com/hc/en-us/articles/1500005513722-Stage-Channels-FAQ | Globale und circle-spezifische AMAs sollten eigene Event-Kontexte bekommen. |
| Circle.so | Community, Events, Content, Access Control, Gamification und Analytics sind in einem System verbunden. Quelle: https://circle.so/platform | POOOL Circles sollen branded Spaces mit Events, Members, Stats und Automations werden. |
| Mighty Networks | Spaces koennen Feed, Events, Members, Pages und Welcome Checklists kombinieren. Quelle: https://www.mightynetworks.com/community | Circle Onboarding, Resources und Events sind keine Nice-to-haves, sondern Aktivierungsmechanik. |
| Discourse | Diskussionen werden langfristig suchbares Wissen; Trust/Moderation sind Kernbestandteile. Quelle: https://www.discourse.org/about | POOOL Circle Content muss auffindbar, filterbar und als Wissen nutzbar sein. |
| eToro | Social Investing verbindet Feed, Profile, Reputation und Strategie. Quelle: https://www.etoro.com/trading/social/ | POOOL sollte Reputation sichtbar machen, aber kein Copy-Trading oder Advice-Mechanik bauen. |
| Stocktwits Rooms | Investoren bleiben fuer thematische Raeume auf der Plattform statt in externe Gruppen auszuweichen. Quelle: https://www.prnewswire.com/news-releases/stocktwits-launches-rooms-so-investors-can-find-their-tribe-300690893.html | Asset- und Topic-Circles halten Investment-Diskussion im POOOL-Oekosystem. |
| Republic Europe / Seedrs | Investor Q&A und Discussion Tabs sind Teil der Investment-Erfahrung. Quelle: https://europe.republic.com/investors-site/guides/what-to-expect-as-a-startup-seedrs-investor/ | Asset Circles sollten Due-Diligence-Fragen, Updates und Dokumentkontext enthalten. |

## 5. Information Architecture

### 5.1 Global Community

Route:

```text
/community
```

Tabs:

- Feed
- Announcements
- My Circles
- Challenges
- Expert AMAs

Funktion:

- allgemeiner Austausch,
- globale Marktgedanken,
- Plattform-News,
- globale AMAs,
- globale Challenges,
- Einstieg in Circle Discovery.

### 5.2 My Circles

Target Route:

```text
/community/circles
```

Kurzfristig kann das weiterhin als Tab in `/community?tab=circle` leben, aber langfristig sollte es eine eigene Seite werden.

Sektionen:

- Recommended for you,
- My Circles,
- Discover,
- Create Circle.

Naming:

- "My Circle" wird zu "My Circles".

Card-Regel:

- `Open` fuehrt immer nach `/community/circle/:slug`.
- Drei-Punkte-Menue fuehrt optional zu Manage, Leave, Share.
- Settings darf nicht der primaere Klickpfad sein.

### 5.3 Circle Feed

Route:

```text
/community/circle/:slug
```

Funktion:

- Circle Header,
- role-aware actions,
- Circle Tabs,
- Circle-specific Feed,
- Composer mit Zielkontext,
- Sidebar mit Rules, Mods, Related Assets, Events und Top Contributors.

### 5.4 Circle Settings

Route:

```text
/community/circle/:slug/settings
```

Funktion:

- Basic Info,
- Privacy,
- Membership,
- Roles,
- Content Settings,
- Moderation,
- Rules,
- Analytics.

Sichtbarkeit:

- Owner,
- Admin,
- Moderator mit Berechtigung,
- Platform Admin.

## 6. Circle Detail Page Specification

### 6.1 Header

Pflichtinhalte:

- Banner,
- Icon/Avatar,
- Circle Name,
- Privacy Status: Public, Private, Hidden, Invite-only, Token-gated, KYC-gated,
- eigene Rolle: Visitor, Member, Moderator, Owner, Expert,
- Mitgliederanzahl,
- Aktivitaetsstatus,
- Primary Action: Post oder Join/Request Access,
- Secondary Actions: Invite, Notifications,
- Manage Action nur bei Berechtigung.

Beispiel:

```text
Founder Circle
Member · Public · 2 / 50 members · 4 posts this week
[Post] [Invite] [Notifications] [...]
```

Aktivitaetscopy:

- `4 posts this week`
- `2 new discussions`
- `1 upcoming AMA`
- `3 members active`
- `New Circle`
- `Trending`
- `Official`
- `Invite-only`
- `Holder-only`
- bei Inaktivitaet: `New Circle · Be the first to post`

Nicht verwenden:

- `Quiet this week` als Default, weil es Inaktivitaet negativ framed.

### 6.2 Tabs

MVP:

- Feed
- About
- Members

Phase 2/3:

- Announcements
- Q&A
- Events / AMAs
- Challenges
- Resources

Asset Circle Extension:

- Official Updates
- Documents
- Yield / Reports
- Risk Discussion
- About Asset

### 6.3 Feed

Regeln:

- Zeigt nur Posts mit `circle_id = current_circle.id`.
- Public Circle: Read kann fuer Nicht-Mitglieder erlaubt sein, Write nur fuer Mitglieder.
- Private/Hidden/Token/KYC Circle: Read nur fuer berechtigte Nutzer.
- Shadowbanned/hidden/moderation logic muss wie im globalen Feed gelten.
- Pinned Posts und Announcements stehen oben.

### 6.4 Composer

Im globalen Feed:

```text
Post to: Global Community
```

Optional:

- Founder Circle,
- Bali Real Estate,
- Uluwatu Luxury Retreat Investors,
- Official Platform Updates.

Im Circle Feed:

```text
Post to: Founder Circle
```

Regeln:

- Circle-Kontext ist sichtbar und fest voreingestellt.
- Optionales `Also share to Global Community` nur fuer Public Circles und nur mit expliziter Permission.
- Composer muss Post Type und optional Tags/Flairs unterstuetzen.

### 6.5 Sidebar

MVP:

- About this Circle,
- Rules,
- Admins/Moderators,
- Members preview.

Phase 2/3:

- Upcoming AMA,
- Top Contributors,
- Related Assets,
- Trending Tags,
- Invite Members,
- Resources.

## 7. Circle Types

| Type | Sichtbarkeit | Join-Regel | Beispiel |
|---|---|---|---|
| Public Circle | sichtbar fuer alle | jeder kann beitreten | Bali Real Estate |
| Private Circle | sichtbar, Content geschuetzt | Beitrittsanfrage | Founder Circle |
| Hidden Circle | nur per Einladung sichtbar | Invite only | Internal Beta Investor Group |
| Token-gated Circle | sichtbar oder private, je nach Config | Asset-/Token-Holding erforderlich | Uluwatu Luxury Retreat Investors |
| KYC-gated Circle | sichtbar, aber Zutritt nur verifiziert | KYC erforderlich | Verified Investors Club |
| Official Circle | von POOOL erstellt | je nach Config | Platform Updates, Governance |

## 8. Circle Roles

| Role | Capabilities |
|---|---|
| Owner | Settings, Members, Moderators, Roles, Rules, Archive/Delete, Privacy, Token/KYC gates. |
| Moderator | Posts moderieren, Kommentare entfernen, Mitglieder verwarnen, pinnen, Announcements erstellen wenn erlaubt. |
| Member | lesen, posten, kommentieren, reagieren, melden, an Events teilnehmen. |
| Visitor | Public Content lesen, Join/Request Access, keine privaten Inhalte. |
| Verified Expert | AMA beantworten, Expert Badge, Antworten hervorheben, keine automatische Admin-Berechtigung. |
| Platform Admin | globale Governance, Audit, Safety Overrides, Official Announcements. |

## 9. Content Model

### 9.1 Post Scope

Aktuell fehlt die wichtigste Relation:

```sql
posts.circle_id UUID NULL REFERENCES circles(id)
```

Semantik:

- `circle_id IS NULL`: globaler Community Post.
- `circle_id IS NOT NULL`: Circle Post.
- Optional spaeter: `share_to_global BOOLEAN`, `source_circle_id`, `visibility`.

### 9.2 Post Types

Empfohlene Typen:

- Discussion,
- Question,
- Market Insight,
- Property Update,
- Due Diligence,
- Poll,
- Announcement,
- AMA Question,
- Resource.

Wichtig: DB und API erlauben aktuell nur begrenzte Werte. Eine Migration muss `post_type` erweitern oder Post Types in eine eigene Tabelle auslagern.

### 9.3 Tags / Flairs

Empfohlene Tags:

- Market Insight,
- Question,
- Risk,
- Yield,
- Real Estate,
- Commodity,
- Bali,
- Cocoa,
- Tokenization,
- Property Update,
- Beginner,
- Advanced,
- Official,
- Answered,
- Featured,
- Due Diligence,
- Legal,
- Tax,
- Liquidity.

Empfohlenes Datenmodell:

```sql
circle_flairs (
  id UUID PRIMARY KEY,
  circle_id UUID NULL,
  name TEXT NOT NULL,
  color TEXT,
  kind TEXT CHECK (kind IN ('post_type', 'topic', 'status')),
  is_required BOOLEAN NOT NULL DEFAULT false,
  mod_only BOOLEAN NOT NULL DEFAULT false
)

post_flairs (
  post_id UUID NOT NULL,
  flair_id UUID NOT NULL,
  PRIMARY KEY (post_id, flair_id)
)
```

### 9.4 Q&A Status

Fuer Questions und Due Diligence:

- Open,
- Answered,
- Official Answer,
- Needs Clarification,
- Archived.

Empfohlen:

```sql
posts.qa_status TEXT NULL
posts.official_answer_comment_id UUID NULL
```

## 10. Investment-Specific Requirements

POOOL Circles muessen sicherer sein als normale Social Communities.

Pflicht:

- Disclaimer unter Investment-/Market-Posts,
- klare Kennzeichnung: User Opinion vs Official POOOL Statement,
- keine prominente Verstaerkung ungepruefter Renditeversprechen,
- Meldefunktion an jedem Post/Kommentar,
- Moderation Queue,
- Expert/Official Badges nur serverseitig vergeben,
- Risk Discussion als eigener Bereich fuer Asset Circles,
- Due-Diligence-Fragen auffindbar und beantwortbar,
- keine Copy-Trading-Mechanik,
- keine Darstellung von User-Reputation als Investment Advice.

## 11. Asset Circle Integration

Asset Circles sind der strategische POOOL-Vorteil.

### Property Page

Beispiel:

```text
Join the investor discussion
128 members · 14 posts this week
[Open Asset Circle]
```

### Portfolio

Nach Investment:

```text
You now have access to: Uluwatu Luxury Retreat Investors Circle
```

### Wallet / Ownership

Bei Token-/Share-Holding:

```text
Holder-only Circle unlocked
```

### Asset Circle Tabs

- Feed,
- Official Updates,
- Q&A,
- Documents,
- Yield / Reports,
- Members,
- Risk Discussion,
- About Asset.

## 12. Access Control Matrix

| Circle Type | Visitor Read | Visitor Join | Member Read | Member Post | Manage |
|---|---|---|---|---|---|
| Public | ja | direct join | ja | ja | owner/mod/admin |
| Private | limited preview | request | ja | ja | owner/mod/admin |
| Hidden | nein | invite only | ja | ja | owner/mod/admin |
| Token-gated | preview optional | if holding asset | ja | ja | owner/mod/admin |
| KYC-gated | preview optional | if KYC verified | ja | ja | owner/mod/admin |
| Official | ja oder protected | depends | ja | usually restricted | platform admin |

Jede API muss diese Matrix serverseitig durchsetzen. UI-Gating reicht nicht.

## 13. MVP Implementation Plan

### Phase 1: Circle Feed as Default Destination

Ziel:

Der Klick auf einen Circle fuehrt zum Circle Feed, nicht zu Settings.

Backend:

- Neue Page Handler Route fuer `/community/circle/:slug`.
- Redirect nach Settings entfernen.
- Neue Template-Datei, z. B. `frontend/platform/community-circle.html`.
- API fuer Circle Feed:
  - Option A: `/api/community/feed?circle_id=:id`
  - Option B: `/api/community/circles/:id/posts`
- DB Migration:
  - `posts.circle_id UUID NULL`
  - Index `(circle_id, created_at DESC) WHERE is_hidden = false`
  - Backward compatibility: globale Posts bleiben `circle_id IS NULL`.
- `CreatePostRequest` um `circle_id` erweitern.
- `create_user_post` prueft Membership/Write-Permission vor Insert.
- Feed Query filtert global vs circle sauber.
- `recent_post_count` wird aus Circle Posts aktualisiert.

Frontend:

- Neue Circle Detail Page mit Header, Tabs, Feed, Composer, Sidebar.
- Existing links in `community-circles-discover.js` bleiben korrekt.
- Settings-Link nur fuer Owner/Admin/Moderator.
- Settings Page behaelt `View feed`, ist aber nicht mehr Default.
- "My Circle" zu "My Circles" umbenennen.
- Composer zeigt `Post to: Circle Name`.

Tests:

- Static: Circle card href bleibt `/community/circle/:slug`.
- Route: `/community/circle/:slug` rendert Feed Page.
- Route: `/community/circle/:slug/settings` rendert Settings.
- API: global feed gibt nur `circle_id IS NULL`.
- API: circle feed gibt nur Posts aus diesem Circle.
- Auth: private circle feed nur fuer Mitglieder.
- Write: Nicht-Mitglied kann nicht in private Circle posten.

Exit Criteria:

- User kann aus My Circles in den Feed eines Circles navigieren.
- User kann im Circle posten.
- Der Post erscheint im Circle Feed, nicht global.
- Settings sind nur ueber Manage erreichbar.

### Phase 2: Structure and Roles

Umfang:

- Memberliste,
- role-aware Manage Menu,
- public/private/hidden visible states,
- join/request/leave/invite flows,
- Circle About,
- Circle Rules,
- bessere Settings-Struktur.

Exit Criteria:

- Rollenmatrix ist in API und UI konsistent.
- Private Circle zeigt geschuetzten Zustand fuer Nicht-Mitglieder.
- Owner/Moderator kann Mitglieder verwalten.

### Phase 3: Content Types and Flairs

Umfang:

- Post Types,
- Circle Flairs,
- Filter im Feed,
- pinned posts,
- Circle Announcements,
- Q&A Status,
- Official Answer.

Exit Criteria:

- Feed ist nach Typ/Tag filterbar.
- Questions koennen als Answered/Official Answer markiert werden.
- Announcements sind global und circle-spezifisch getrennt.

### Phase 4: Engagement and Onboarding

Umfang:

- Circle Challenges,
- Top Contributors,
- Notifications,
- Weekly Digest,
- New Member Welcome Checklist,
- Suggested Circles.

Exit Criteria:

- Neue Mitglieder bekommen einen klaren ersten Schritt.
- Circle Aktivitaet ist messbar und sichtbar.
- Engagement-Mechaniken sind nicht nur global, sondern circle-spezifisch.

### Phase 5: Investment Integration

Umfang:

- Asset Circles,
- Holder-only Circles,
- portfolio-basierte Empfehlungen,
- Property Page -> Circle,
- Official Asset Updates,
- Document/Report Tab,
- Expert AMAs pro Asset.

Exit Criteria:

- Asset Ownership kann Circle Access freischalten.
- Asset Pages zeigen relevante Circle-Aktivitaet.
- Official Updates und Due-Diligence-Fragen sind getrennt von normalen Social Posts.

## 14. Settings Page Restructure

Die bestehende Settings-Seite bleibt, wird aber als Manage-Bereich neu gegliedert.

Sektionen:

1. Basic Info
   - Name,
   - Description,
   - Slug,
   - Banner Image,
   - Circle Icon,
   - Category,
   - Language,
   - optional Location/Topic.

2. Privacy
   - Public,
   - Private,
   - Hidden,
   - Invite-only,
   - Token-gated,
   - KYC-gated.

3. Membership
   - Join approval required,
   - auto-approve verified investors,
   - max members,
   - invite links,
   - join questions.

4. Roles & Permissions
   - Owner,
   - Moderator,
   - Member,
   - Expert,
   - Viewer.

5. Content Settings
   - allowed post types,
   - required tags,
   - media uploads,
   - polls,
   - anonymous posting,
   - link posting.

6. Moderation
   - auto-moderation,
   - report queue,
   - blocked words,
   - investment-risk keywords,
   - approval for first post,
   - slow mode.

7. Rules
   - Circle rules,
   - disclaimer,
   - investment-risk notice,
   - netiquette.

8. Announcements
   - who can post,
   - comment settings,
   - pinning rules.

9. Analytics
   - member growth,
   - active members,
   - posts per week,
   - top tags,
   - top contributors,
   - reported content.

## 15. Data Model Backlog

Minimum:

```sql
ALTER TABLE posts ADD COLUMN circle_id UUID NULL REFERENCES circles(id) ON DELETE CASCADE;
CREATE INDEX idx_posts_circle_created ON posts(circle_id, created_at DESC) WHERE is_hidden = false;
```

Next:

```sql
ALTER TABLE circles ADD COLUMN circle_type TEXT NOT NULL DEFAULT 'public';
ALTER TABLE circles ADD COLUMN is_hidden BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE circles ADD COLUMN kyc_required BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE circles ADD COLUMN official BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE circles ADD COLUMN rules TEXT;
ALTER TABLE circles ADD COLUMN category TEXT;
```

Later:

- `circle_flairs`,
- `post_flairs`,
- `circle_resources`,
- `circle_events`,
- `circle_onboarding_steps`,
- `circle_related_assets`,
- `circle_analytics_daily`,
- `circle_moderation_settings`,
- `circle_join_questions`.

## 16. API Backlog

MVP:

```text
GET  /community/circle/:slug
GET  /api/community/circles/by-slug/:slug
GET  /api/community/circles/:id/posts
POST /api/community/circles/:id/posts
GET  /api/community/circles/:id/members
POST /api/community/circles/:id/join
POST /api/community/circles/:id/request
```

Phase 2/3:

```text
GET  /api/community/circles/:id/announcements
GET  /api/community/circles/:id/questions
POST /api/community/circles/:id/questions/:post_id/official-answer
GET  /api/community/circles/:id/flairs
PUT  /api/community/circles/:id/flairs
GET  /api/community/circles/:id/resources
GET  /api/community/circles/:id/events
GET  /api/community/circles/:id/analytics
```

## 17. Security and Compliance Constraints

Diese Spezifikation darf nicht losgeloest vom Production Readiness Audit umgesetzt werden.

Pflicht vor Production Live:

- API-Grenzen public/private/serverseitig durchsetzen.
- Circle Mentions duerfen private Circle-Namen nicht an Nicht-Mitglieder leaken.
- XSS-sicherer Content Renderer statt freier HTML-Strings.
- CSP-Haertung und Entfernung inline Event Handler.
- Rate Limits fuer Circle Posts, Comments, Invites, Join Requests, Search.
- Moderation Queue und Report Reasons fuer Circle-Kontext.
- Retention/Erasure fuer Circle Content, Membership, DMs, Reports.
- Audit Logs fuer Owner/Admin/Moderator-Aktionen.

## 18. Historical Implementation Order

Diese Reihenfolge beschreibt die urspruenglich empfohlene Implementierung. Der lokale Code hat diese Punkte inzwischen als Phasen-1-bis-9-Slice umgesetzt; offene Punkte sind im Post-Implementation-Audit dokumentiert.

1. Route-IA korrigieren: `/community/circle/:slug` rendert eigene Feed-Page.
2. DB: `posts.circle_id` und Index hinzufuegen.
3. API: Circle posts lesen und schreiben.
4. AuthZ: Circle read/write matrix serverseitig durchsetzen.
5. Frontend: Circle Detail Page MVP mit Header, Feed, Composer, Sidebar.
6. Settings: Manage Button role-gated und Settings nicht mehr Default.
7. Copy: "My Circle" zu "My Circles".
8. Tests: route, API, auth, composer, DOM static, E2E.
9. Product polish: activity labels, member preview, about/rules.
10. Phase 2 Backlog ausrollen.

## 19. Acceptance Criteria for Phase 1

Phase 1 ist fertig, wenn:

- Klick auf Recommended Circle, Discover Circle und My Circles Pill oeffnet `/community/circle/:slug`.
- Diese Route zeigt nicht die Settings-Seite.
- Circle Header zeigt Name, Privacy, Rolle, Mitglieder, Aktivitaet.
- Composer zeigt eindeutig `Post to: <Circle Name>`.
- Neue Circle Posts werden mit `circle_id` gespeichert.
- Global Feed zeigt Circle Posts nicht versehentlich an.
- Circle Feed zeigt keine fremden Circle Posts.
- Nicht-Mitglieder koennen nicht in geschuetzte Circles posten.
- Settings sind nur ueber Manage sichtbar.
- Focus/keyboard navigation fuer Header Actions und Tabs funktioniert.
- Static und E2E Tests fuer den Flow sind gruen.

## 20. Non-Goals

Nicht in Phase 1:

- Copy Trading,
- Live Audio/Video,
- vollstaendige Course-Plattform,
- public SEO fuer private Circles,
- komplexe AI Moderation,
- monetarisierte Memberships,
- DMs als Circle-Kernfunktion.

## 21. Open Product Decisions

Diese Entscheidungen sollten vor Phase 2 finalisiert werden:

- Sollen Public Circle Posts im globalen Feed cross-postbar sein?
- Sind Public Circle Inhalte fuer ausgeloggte Besucher sichtbar?
- Sind Hidden Circle Namen in Mentions fuer Nicht-Mitglieder sichtbar oder komplett redacted?
- Welche Circle-Typen duerfen Nutzer selbst erstellen?
- Wer darf Official Circles erstellen?
- Sind Asset Circles automatisch pro Asset oder kuratiert durch Admins?
- Darf ein User mehrere Rollen in einem Circle haben?
- Welche Post Types sind fuer MVP Pflicht?
- Sind DMs innerhalb eines Circles erlaubt oder bleiben sie global user-to-user?

## 22. Summary

Die vorliegende Produktvision ist richtig: Der Feed ist das Produkt, Settings sind Verwaltung. Der Code zeigt sogar genau den aktuellen Bruch: Die UI verlinkt bereits auf eine Circle-Feed-URL, aber der Server leitet diese URL noch auf Settings um. Der naechste sinnvolle Engineering-Schritt ist deshalb kein grosses Redesign, sondern ein sauberer Phase-1-Schnitt:

```text
Circle Feed als echte Default-Seite bauen,
Posts an Circle-Kontext binden,
Settings in ein role-gated Manage-Menue verschieben.
```

Danach koennen Tags, Q&A, Announcements, Events, Resources, Challenges und Asset-/Holder-only-Integrationen schrittweise folgen.
