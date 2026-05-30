# Community Production Readiness Audit

Datum: 2026-05-20  
Repository: `/Users/martin/Projects/poool`  
Scope: Community Backend, Community Frontend, Admin-Community, Datenbankmigrationen, Community-Tests und produktionsrelevante Querschnittsthemen.

## 1. Executive Summary

Die Community ist funktional weit fortgeschritten, aber noch nicht produktionsreif fuer einen regulierten Investor-/Finanzkontext. Der Code enthaelt bereits eine breite Feature-Basis: Feed, Posts, Kommentare, Reactions, Bookmarks, Hashtags, Profile, Follows, Blocks/Mutes, Direct Messages, Circles, Challenges, AMAs, Badges, XP, Admin-Moderation, Audit-Logik und mehrere Background Worker. Sicherheitsgrundlagen wie Session-Auth, CSRF-Schutz, serverseitige Authorisierung fuer viele Mutationen, SQLx, Transaktionen und HTML-Sanitization sind vorhanden.

Die produktionskritischen Luecken liegen nicht in einem einzelnen fehlenden Feature, sondern in der Haerte der Plattform: API-Sichtbarkeit, Content-Rendering/XSS-Defense-in-Depth, CSP-Haertung, Abuse-Schutz ohne Redis, Datenschutz/Retention fuer Community- und DM-Daten, Trust-&-Safety-Prozesse, Observability, Test-Drift und Release-Gates. Der aktuelle Stand ist damit eher "funktionales Beta-/Pilot-System" als "Production Live".

Gesamtbewertung: 61/100 Produktionsreife.

Interpretation: Die Community kann als eingeschraenkter interner Pilot oder moderierter Beta-Test betrieben werden, aber nicht als oeffentliches, produktives Community-System mit regulatorischem Anspruch, ohne die P0/P1-Punkte in diesem Audit zu schliessen.

## 2. Bewertungsmethodik

Die Bewertung kombiniert:

1. statische Code-Inspektion der Community-Dateien,
2. Abgleich mit Projektstandards aus `AGENTS.md`, `docs/AGENT_DEVELOPMENT_PROMPT.md`, `docs/DESIGN.md` und `docs/IMPLEMENTATION_ROADMAP.md`,
3. Abgleich mit aktuellen Industrie- und Sicherheitsstandards,
4. fokussierte lokale Testausfuehrung,
5. Produktionsrisiko-Bewertung nach Impact, Eintrittswahrscheinlichkeit und regulatorischem Kontext.

Skala:

| Score | Bedeutung |
|---:|---|
| 0-39 | Nicht produktionsfaehig; Kernfunktionen oder Schutzmechanismen fehlen. |
| 40-59 | Teilweise implementiert; nur intern oder in isolierten Tests nutzbar. |
| 60-74 | Beta-/Pilot-reif; mehrere produktionskritische Gates offen. |
| 75-89 | Nahe Produktion; gezielte Security/QA/Ops-Gaps offen. |
| 90-100 | Produktionsreif nach definierten Akzeptanzkriterien. |

## 3. Standards-Baseline

Die Community wurde gegen folgende Standards und Best Practices gemappt:

| Bereich | Referenz |
|---|---|
| Web Application Security | OWASP ASVS 5.0.0, insbesondere AuthN/AuthZ, Session, Input Validation, Output Encoding, Logging, API Controls: https://owasp.org/www-project-application-security-verification-standard/ |
| API Security | OWASP API Security Top 10 2023, insbesondere BOLA, Broken Authentication, Broken Object Property Authorization, Unrestricted Resource Consumption, SSRF und Sensitive Business Flows: https://owasp.org/www-project-api-security/ |
| XSS | OWASP XSS Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html |
| CSRF | OWASP CSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html |
| Logging/Monitoring | OWASP Logging Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html |
| Secure SDLC | NIST SP 800-218 SSDF 1.1: https://csrc.nist.gov/pubs/sp/800/218/final |
| Accessibility | WCAG 2.2 W3C Recommendation: https://www.w3.org/WAI/news/2023-10-05/wcag22rec/ |
| Datenschutz | GDPR Art. 17 / Right to Erasure als Datenschutz-Mindestsignal: https://www.edpb.europa.eu/gdpr-articles/article-17-right-erasure-right-be-forgotten_es |

## 4. Scope und Code-Oberflaeche

Die Community ist gross genug, dass sie als eigenstaendiges Produktmodul behandelt werden muss.

| Zone | Umfang |
|---|---:|
| `backend/src/community/*.rs` | 15.667 Zeilen |
| `frontend/platform/static/js/community*.js` | 6.919 Zeilen |
| `frontend/platform/static/css/community*.css` | 8.968 Zeilen |
| Community User Templates | 2.435 Zeilen |
| Admin Community Templates | 5.092 Zeilen |
| Gesamt untersuchter Community-Code | ca. 39.091 Zeilen |

Zentrale Dateien:

- Backend: `backend/src/community/routes.rs`, `service.rs`, `circles.rs`, `background.rs`, `validation.rs`, `moderation.rs`, `xp.rs`, `amas.rs`, `challenges.rs`, `reviews.rs`, `notifications.rs`, `audit.rs`, `user_bridge.rs`.
- Frontend: `frontend/platform/community.html`, `community-profile.html`, `community-profile-edit.html`, `community-circle-settings.html`, `community-hashtag.html`, `community-badge.html`, `static/js/community-*.js`, `static/css/community*.css`, `partials/community_*.html`, `partials/community_post_card.html`.
- Admin: `frontend/platform/admin/community/*.html`.
- Datenbank: `database/community/*.sql`, `database/069_apply_missing_community_schema.sql`, `database/082_community_badge_permissions.sql`, `database/083_admin_community_permissions.sql`, `database/084_community_xp_nonnegative.sql`.

## 5. Prozentuale Reife nach Kategorie

| Kategorie | Score | Produktionsurteil |
|---|---:|---|
| Backend/API-Funktionalitaet | 72% | Breite Feature-Abdeckung, aber API-Scope, Validierung und Fehlersemantik brauchen Haertung. |
| Security/AppSec | 63% | Gute Basis mit CSRF, SQLx und Sanitization; XSS/CSP/API-Exposure/Rate-Limits sind noch produktionskritisch. |
| Privacy/Compliance | 49% | GDPR-Anonymisierung existiert, deckt aber Retention, DMs, Reports, Cache und Nachholfaelle nicht ausreichend ab. |
| Trust & Safety/Moderation | 56% | Admin-Moderation ist vorhanden; Policy-Taxonomie, DM-Moderation, Automod-Qualitaet und SLA-Prozesse fehlen. |
| Frontend/UX | 68% | Viele Oberflaechen existieren; es gibt Design-/Copy-/Dialog- und Contract-Drift. |
| Accessibility | 64% | Tabs/ARIA/Fokus wurden verbessert; WCAG-2.2-AA-Nachweis fehlt, native Prompts und dynamische Statusmeldungen bleiben offen. |
| Data/Performance/Scalability | 67% | Separate Community-DB, Indizes und Batch-Hydration sind vorhanden; Suche, Feed-Schutz, Body-Limits und Multi-Instance-Abuse sind offen. |
| Observability/Ops | 52% | Globale Logs/Metrics existieren; Community-spezifische SLOs, Alerts, Dashboards und Runbooks fehlen. |
| Tests/QA/Release Gates | 54% | Viele Tests existieren, aber Static-Contract-Drift und `cargo check`-Schemafehler blockieren Release-Vertrauen. |
| Dokumentation/Governance | 48% | Roadmap- und Page-Audit-Spuren existieren; aktuelle Feature-Spezifikation, Moderationspolicy und Production Runbook fehlen. |

Gesamt: 61%.

## 6. Was aktuell fertig ist

### 6.1 Backend und API

Vorhanden:

- Community-Router mit User- und Admin-API in `backend/src/community/routes.rs`.
- Feed und Post-Detail inklusive optionalem Viewer-Kontext.
- Post-Erstellung mit Transaktion, Profil-Erstellung, Moderation, Hashtag-Extraktion, Poll-Unterstuetzung und XP-Vergabe.
- Kommentare, Comment-Reactions, Post-Reactions, Bookmarks, Reports.
- Profile, Follower/Following, Blocks, Mutes und Profileinstellungen.
- Circles inklusive Rollen, Invites, Ban/Unban, Moderation und Retry-Worker.
- Challenges, AMAs, Badges, Leaderboard und XP-System.
- Direct Messages mit Thread-/Message-Modell.
- Admin-Community: Reports, Posts, Kommentare, User-Detail, Ban/Mute/Shadowban/Warn, Mod Notes, Challenges, AMAs, Badges, Circles, XP, Audit-Export, Verified Owner Requests und Settings.
- Background Worker fuer Asset Velocity, Gamification, XP Aggregation, Invite Expiry, Circle Retry, Trending Refresh, GDPR-Anonymisierung und Weekly Digest.

### 6.2 Security-Basis

Vorhanden:

- Globaler CSRF-Middleware fuer mutierende Requests in `backend/src/auth/csrf.rs`.
- Session-basierte Authentifizierung mit HTTP-only Cookie laut Projektarchitektur.
- SQLx und parameterisierte Queries.
- Mehrere Transaktionen fuer finanz-/zustandsrelevante Community-Schreibpfade.
- Basic HTML Sanitization ueber Ammonia in `backend/src/community/validation.rs`.
- Security Headers inklusive HSTS, X-Frame-Options, Referrer-Policy und CSP in `backend/src/lib.rs`.
- Redis-basierte Rate-Limits fuer einige sensible Pfade und globale Rate-Limiter-Fallbacks.
- SSRF-Grundschutz beim OpenGraph-Fetching: private IPs, localhost und Redirects werden teilweise abgewehrt.

### 6.3 Frontend

Vorhanden:

- Hauptseite `/community` mit Tabs/Feed/Composer.
- Profile, Profile Edit, Hashtag, Badge und Circle Settings.
- Community-spezifische JS-Controller fuer Feed, Profile, AMAs, Challenges, DMs, Circles, Ban Appeals und Sync.
- Admin-Community-Seiten fuer die wichtigsten Moderations- und Operations-Aufgaben.
- Mehrere gezielte Static- und E2E-Testdateien im `tests/`-Baum.

## 7. Produktionsblocker und Befunde

### P0.1 API-Sichtbarkeit ist nicht eindeutig produktionssicher

Evidence:

- `backend/src/community/routes.rs:780` ruft fuer `/api/community/feed` nur optional `get_current_user` auf und liefert Feed-Daten auch ohne Auth.
- `backend/src/community/routes.rs:790` erlaubt Post-Detail bewusst oeffentlich.
- `backend/src/community/routes.rs:4825` macht `/api/community/search` ebenfalls optional-auth und durchsucht User-/Post-Daten.

Bewertung:

Das kann korrekt sein, wenn die Community bewusst oeffentlich ist. Das aktuelle Produktbild wirkt aber wie eine eingeloggte Investor-Community. Fuer eine Investor-/Finanzplattform ist eine unklare Public/Private-Grenze ein P0-Designrisiko: Profildaten, Posts, Bio-Suche, Social Graph und Engagement koennen ungewollt oeffentlich werden.

Erforderlich:

- Produktentscheidung dokumentieren: public community, semi-public marketing surface oder logged-in-only community.
- Default: Community APIs authenticated-by-default; explizite Allowlist fuer Share-/Preview-/SEO-Endpunkte.
- Tests fuer unauthenticated access pro Endpoint.
- Threat Model fuer Social Graph, Profile, Mentions, Circles, DMs und Reports.

### P0.2 XSS-Defense-in-Depth ist noch nicht ausreichend

Evidence:

- `backend/src/community/routes.rs:476-518` nimmt `content_sanitized` oder `content` und fuegt danach per Regex HTML fuer Hashtags, Asset-Tags und Mentions ein.
- `frontend/platform/partials/community_post_card.html:46` rendert `{{ p.rendered_content | safe }}`.
- `frontend/platform/static/js/community-feed.js` baut an mehreren Stellen dynamisches HTML und setzt fuer Post-Body HTML auf der Client-Seite.

Bewertung:

Die Sanitization-Basis ist gut, aber das Muster "HTML sanitizen, danach wieder HTML zusammensetzen, dann `safe` rendern" ist fragil. OWASP empfiehlt kontextspezifisches Output Encoding und vorsichtigen Umgang mit HTML-Sanitization. Fuer eine Finanzplattform sollte der Renderer strukturiert sein: User-Text bleibt Text, Linkification erzeugt typisierte Tokens/Nodes, und finaler Output wird nicht durch freie HTML-Strings zusammengesetzt.

Erforderlich:

- Structured Content Renderer fuer Post-Text: Tokens fuer Text, Hashtag, Mention, Asset, Link.
- Keine Regex-Rewrites ueber bereits sanitiztes HTML.
- Client-seitig keine `innerHTML`-Pfadabhaengigkeit fuer User-Content ohne erneute Sanitization.
- XSS-Fixtures fuer HTML body, attributes, URLs, CSS, DOM sinks und mixed Unicode.
- Sentry/Alert fuer geblockte Sanitization-/Validation-Events.

### P0.3 CSP bleibt zu permissiv fuer Production Live

Evidence:

- `backend/src/lib.rs:1870-1878` setzt `script-src 'unsafe-inline' 'unsafe-eval'`.
- Community Templates enthalten inline `onclick`, inline `onkeydown`, inline Styles und Inline-Scripts, z. B. `frontend/platform/partials/community_post_card.html:4`, `:52-54`, `:63-83`.

Bewertung:

CSRF ist vorhanden, aber OWASP weist zu Recht darauf hin, dass XSS CSRF-Schutz aushebeln kann. Mit `unsafe-inline` und `unsafe-eval` ist CSP nur begrenzt als letzte Verteidigung nutzbar. Alpine-/Inline-Handler duerfen nicht dauerhaft die Sicherheitsarchitektur fuer Community-Content diktieren.

Erforderlich:

- Nonce- oder hash-basierte CSP-Rollout-Strategie.
- Inline-Event-Handler in Templates entfernen.
- Alpine/Inline-JS aus produktionskritischen Community-Pfaden eliminieren oder auf CSP-kompatible Patterns migrieren.
- `script-src` ohne `unsafe-inline` und mittelfristig ohne `unsafe-eval`.

### P0.4 Abuse-Schutz und Payload-Validierung sind nicht robust genug

Evidence:

- `backend/src/community/service.rs:429-462` begrenzt Posts nur, wenn Redis verfuegbar und erreichbar ist.
- `backend/src/community/routes.rs:1389-1467` prueft Automod und Ban, aber keine explizite Post-Laenge vor Sanitization/DB.
- `backend/src/community/service.rs:528-581` verlaesst sich fuer mehrere Poll-Grenzen stark auf Datenbankconstraints bzw. Minimalchecks.

Bewertung:

Ein Bot kann ohne Redis-Ausfall-Strategie oder lokale/DB-basierte Fallback-Limits deutlich mehr Last erzeugen. Der globale Body-Limit ist fuer Community-Text viel zu grob. Datenbankconstraints sind wichtig, aber kein Ersatz fuer fruehe, nutzerfreundliche und lastschonende Application Validation.

Erforderlich:

- Application-Level-Validation fuer Post-Laenge, Post-Type, Poll-Frage, Poll-Optionen, Link-Metadaten und Image-URLs.
- Redis-Fehler duerfen Rate-Limits nicht still deaktivieren. Fallback: DB-Zaehlung, In-Memory Circuit Breaker oder fail-closed fuer Hochrisikoaktionen.
- Separate Abuse-Budgets fuer Post, Comment, DM, Report, Follow, Search, Circle Invites, AMA Questions.
- IP/User/Session/device-keyed Limits plus Bot-/automation detection fuer sensitive flows.

### P0.5 Datenschutz, Retention und Right-to-Erasure sind unvollstaendig

Evidence:

- `backend/src/community/background.rs:308-310` anonymisiert nur User mit `status='deleted'` und `updated_at >= NOW() - INTERVAL '1 day'`.
- `backend/src/community/background.rs:324-348` anonymisiert Community-Profil und loescht Appeals/Circle Memberships, aber nicht umfassend Posts, Kommentare, DMs, Reports, Mentions, Notifications, XP, Audit-Exports oder Cache.
- Direct Messages sind in `database/community/034_dms.sql` und `backend/src/community/routes.rs` implementiert, aber ohne erkennbare Retention-/Report-/Erasure-Policy.

Bewertung:

Ein stuendlicher Worker mit 1-Tages-Fenster ist nicht auditfest: verpasste Jobs oder alte Delete-Events bleiben liegen. Fuer eine Community mit personenbezogenen Daten und DMs braucht es eine formale Datenklassifikation, Retention Matrix, DSAR/Erasure-Workflow, Cache-Invalidation und klare Regeln, welche Inhalte anonymisiert statt geloescht werden duerfen.

Erforderlich:

- Retention Matrix fuer Profile, Posts, Kommentare, Reports, DMs, Notifications, XP, Audit Logs, Moderation Notes und OpenGraph-Daten.
- Idempotenter Erasure Job ohne 1-Tages-Fenster oder mit persistentem Erasure Ledger.
- Cache-Invalidierung fuer `user_bridge` und Redis.
- DM-Export/Deletion/Anonymization-Regeln.
- Datenschutz-/Legal-Review vor Production Live.

## 8. Weitere P1-Befunde

### P1.1 Moderation ist funktional, aber nicht Trust-&-Safety-reif

Evidence:

- `backend/src/community/validation.rs:22-45` enthaelt eine simple Wort-/Linkliste fuer Automod.
- Reports sind vorhanden, aber Reason-Taxonomie und Severity wirken nicht serverseitig standardisiert.
- Admin-Aktionen fuer Ban, Mute, Shadowban, Warning und Mod Notes sind vorhanden und teils auditiert.

Risiko:

Eine Investment-Community braucht Moderation fuer Spam, Pump-and-Dump, Market Manipulation, Impersonation, Scam Links, harassment, private data leakage, regulated advice, referral abuse und coordinated inauthentic behavior. Eine einfache Profanity-Liste deckt das nicht ab.

Empfehlung:

- Moderation Policy und Reason Taxonomy als Produkt-/Legal-Artefakt.
- Server-Allowlist fuer Report-Reasons.
- Severity, SLA, Queue-Priorisierung und Moderator Assignment.
- Automod-Anbieter oder ML/Rules-Pipeline mit Audit Trail.
- Separate Regeln fuer Finanz-/Investment-Claims und riskante Promises.

### P1.2 Direct Messages sind nicht production-ready

Evidence:

- DM-Threads und Messages existieren.
- Keine erkennbare DM-Report-Funktion, keine Moderationsqueue fuer DMs, keine Retention-Matrix, keine Verschluesselungs-/Abuse-Policy.

Risiko:

DMs sind der hoechste Missbrauchskanal fuer Scam, Impersonation, Off-platform Payment, Phishing und private Daten. In einer Finanzplattform sollten DMs entweder deutlich eingeschraenkt, moderierbar/reportbar oder vor Launch deaktiviert sein.

Empfehlung:

- Feature Flag fuer DMs.
- Report/block flow direkt in DMs.
- Link/attachment restrictions.
- Message rate limits und new-account limits.
- Retention/DSAR-Konzept.
- Moderator tooling und auditierte Zugriffskontrollen.

### P1.3 Temporäre Bans sind im Schema angelegt, aber nicht sauber durchgesetzt

Evidence:

- `community_profiles` enthaelt `ban_expires_at`.
- `check_user_not_banned` prueft nur `is_community_banned` und `muted_until`, nicht `ban_expires_at`.
- Admin-Ban-Flow setzt nach sichtbarem Code nur `is_community_banned` und `ban_reason`.

Risiko:

Temp-Bans koennen fachlich inkonsistent werden. Nutzer koennen dauerhaft blockiert bleiben oder ein abgelaufener Ban wird nicht sauber aufgeloest.

Empfehlung:

- Ban-State-Machine definieren: active, expired, lifted, permanent.
- `ban_expires_at` in Write-Gates und Read-Views beruecksichtigen.
- Scheduled unban worker oder lazy unban mit Audit.
- Tests fuer mute vs ban vs shadowban.

### P1.4 Weekly Digest ist nur Stub

Evidence:

- `backend/src/community/background.rs:368-413` loggt "Would send digest email", sendet aber nicht.

Risiko:

Produktseitig kann das als implementiertes Feature erscheinen, operativ passiert aber nichts. Falls Nutzer Preferences sehen, ist das ein Trust-/Compliance-Problem.

Empfehlung:

- Entweder Feature verstecken/deaktivieren oder echte Email-Outbox mit Opt-in, Preferences, Unsubscribe und Bounce Handling integrieren.

### P1.5 Community Observability ist zu schwach

Evidence:

- Es gibt globale Logs/Metrics und Community-Audit-Logs, aber kein sichtbares Community-SLO-/Alert-Set.

Risiko:

Ohne Metriken fuer Report Backlog, Moderation SLA, Post/Comment/DM Rate, Rate-Limit-Rejections, Search Latency, Feed Latency, OpenGraph Fetch Failures, Worker Lag und DB Pool Saturation wird Produktion blind.

Empfehlung:

- Community Dashboard: API p95/p99, error rate, queue depth, reports by severity, moderation latency, active users, abuse counters.
- Alerts: report backlog, CSRF failures spike, sanitizer blocks spike, DM spam, search scrape pattern, worker failures.
- Runbooks fuer moderation incident, XSS incident, privacy request, community DB outage und Redis outage.

### P1.6 Tests und Release Gates sind nicht stabil

Lokale Testausfuehrung am 2026-05-20:

```text
python3 -m pytest tests/test_community_tab_contract_static.py tests/test_community_profile_static.py tests/admin/test_admin_community_user_detail_static.py tests/admin/test_admin_community_users_static.py -q
```

Ergebnis:

- `tests/test_community_tab_contract_static.py`: failed, `id="ama-expert-avatar"` fehlt im AMA-Partial.
- `tests/test_community_profile_static.py`: failed, erwartet alte Klasse `community-profile-hero ds-card`, waehrend Template auf `cp-hero` umgestellt ist.
- `tests/admin/test_admin_community_user_detail_static.py` und `tests/admin/test_admin_community_users_static.py`: 6 passed.

Zusatz:

```text
cd backend && cargo check
```

Ergebnis:

- failed wegen SQLx-Compile-Time-Schemafehlern:
  - `src/admin/rewards.rs`: Spalte `tiers.referral_bonus` fehlt in lokaler DB.
  - `src/community/routes.rs`: Relation `community_profiles` fehlt in lokaler DB.

Bewertung:

Der Code mag in einer anders migrierten DB funktionieren, aber das lokale Release-Gate ist aktuell nicht reproduzierbar gruen. Fuer Production Live muss das Schema in Dev/CI/Local eindeutig und automatisiert herstellbar sein.

Erforderlich:

- Community-Migrationen in Standard-Setup/CI erzwingen oder SQLx offline metadata sauber pflegen.
- Static-Tests an aktuelles DOM anpassen oder DOM wieder an Testvertrag angleichen.
- Full Community E2E gegen authentifizierten Seed-User.
- XSS fixture test suite erneut laufen lassen.
- API authorization matrix tests fuer public/private endpoints.

## 9. P2-Befunde und Produktpolitur

| Bereich | Befund | Empfehlung |
|---|---|---|
| Fehlersemantik | `get_community_pool` gibt `AppError::Internal` bei fehlender Community DB zurueck. | Fuer API `503 Service Unavailable`, fuer UI klares Disabled-State-Pattern. |
| UI Dialoge | Circle Settings nutzt native `confirm()`/`prompt()`-Flows. | Design-system-konforme Modals mit Fokusmanagement und Audit-konformen Reason-Feldern. |
| Profile Copy | Profile Settings enthalten teils beschreibende Feature-Texte und alte Modal-Sprache. | Copy auf produktiven Workflow reduzieren. |
| Search | Search nutzt ILIKE und optional auth. | Full-text/Trigram-Index, scraping-safe paging, auth decision, query length/rate limits. |
| Inline Styles | Community partials enthalten viele inline styles. | In CSS auslagern, CSP-Haertung erleichtern. |
| OpenGraph Fetch | SSRF-Basis gut, aber DNS-Rebinding/TOCTOU nicht voll ausgeschlossen. | Resolver/connector-level IP allowlist nach finaler Verbindung pruefen oder Fetch-Proxy isolieren. |

## 10. Architektur- und Feature-Reife

### Backend/API

Stark:

- Viele Community-Domains sind implementiert.
- Mutationen nutzen haeufig Auth, Transaktionen und Audit Logs.
- Admin-Bereich ist umfangreich.
- Batch-Hydration fuer User/Badges reduziert offensichtliche N+1-Risiken.

Schwach:

- Public/private API-Grenze ist nicht formalisiert.
- Validation sitzt nicht konsequent vor DB und teuren Operationen.
- Abuse-Controls sind Redis-abhaengig und nicht flaechendeckend.
- Community DB optionality ist fuer UI besser geloest als fuer API.

### Frontend

Stark:

- Viele produktnahe Views existieren.
- Profil-JS enthaelt bereits Fix fuer spaet gesetzte `PROFILE_ID`.
- Admin-User-Community Static Tests sind aktuell gruen.

Schwach:

- DOM-Contracts zwischen Tests und Templates driften.
- Inline Event Handler blockieren CSP-Haertung.
- Native Dialoge und dynamische Inhalte brauchen WCAG-2.2-AA-Nachweis.
- Mehrere Client-HTML-Pfade bleiben XSS-sensitiv, auch wenn Daten teilweise escaped werden.

### Datenbank

Stark:

- Umfassende Community-Migrationen.
- Separate Community-DB-Unterstuetzung.
- Constraints fuer zentrale Tabellen, z. B. Post-Laenge und XP Non-Negative.

Schwach:

- Lokale DB ist nicht auf dem erwarteten SQLx-Schema.
- Retention/Erasure ist nicht als Datenmodell vollstaendig.
- DM- und Moderationsdaten brauchen klare Lifecycle-Regeln.

## 11. Roadmap zu 100% Produktionsreife

### Phase 0: Produktgrenzen und Release-Gates fixieren

Ziel: Keine Security- oder QA-Entscheidung bleibt implizit.

Aufgaben:

- Community Public/Private-Entscheidung schriftlich festlegen.
- Feature-Flags definieren: DMs, Weekly Digest, Public Profiles, Public Post Detail, Circles, AMAs.
- Akzeptanzmatrix fuer Production Live erstellen.
- CI/Local DB Setup reparieren, sodass `cargo check` reproduzierbar gruen ist.

Exit Criteria:

- Endpoint Authorization Matrix ist dokumentiert und getestet.
- `cargo check` laeuft in sauberer lokaler DB und CI gruen.
- P0-Liste aus diesem Audit ist als Tickets/Roadmap erfasst.

### Phase 1: AppSec-Haertung

Ziel: Community-Content darf keinen Account-, Session- oder Finanzkontext kompromittieren.

Aufgaben:

- API-Auth-Gates anhand der Endpoint Matrix implementieren.
- Structured Content Renderer einfuehren.
- `| safe` fuer User-Content entfernen oder auf gepruefte Renderer-Ausgabe begrenzen.
- Inline Event Handler aus Community Templates entfernen.
- CSP Nonce/Hash-Rollout starten.
- Post/Poll/Comment/DM validation serverseitig vervollstaendigen.
- Abuse rate limits fuer alle write-heavy und scrape-faehigen Endpoints ergaenzen.
- SSRF-Fetching fuer OpenGraph weiter isolieren.

Exit Criteria:

- XSS Fixture Suite gruen.
- Unauthenticated API tests gruen.
- CSP Report-Only ohne kritische Violations, danach Enforce fuer Community-Pfade.
- Redis-Ausfalltest fuer Abuse-Limits bestanden.

### Phase 2: Trust & Safety

Ziel: Moderation ist ein steuerbarer, auditierbarer Betriebsvorgang.

Aufgaben:

- Moderation Taxonomy fuer Spam, Scam, Harassment, Market Manipulation, Advice, Impersonation, Privacy und Illegal Content.
- Server-Validierung fuer Report Reasons und Admin Actions.
- Report Queue mit Severity, SLA, Assignment und Escalation.
- DM Report/Block/Rate-Limit/Retention.
- Ban-State-Machine inklusive `ban_expires_at`.
- Moderator Audit Views und Export-Controls.
- Automod Pipeline erweitern und false-positive handling definieren.

Exit Criteria:

- Moderation Runbook vorhanden.
- Testfaelle fuer alle Moderation States.
- DM Abuse Playbook vorhanden oder DMs per Feature Flag aus.

### Phase 3: Datenschutz und Compliance

Ziel: Personenbezogene Community-Daten sind lifecycle-faehig.

Aufgaben:

- Datenklassifikation und Retention Matrix.
- Idempotenter Erasure/Anonymization Worker ohne 1-Tages-Luecke.
- Cache invalidation fuer User Bridge und Redis.
- DSAR Export fuer Community-Daten.
- Regeln fuer Posts/Kommentare: Loeschen, Anonymisieren oder gesetzlich/fachlich begruendet behalten.
- DM Lifecycle und Zugriffskontrollen.
- Privacy Copy/Settings fuer Digest/Notifications.

Exit Criteria:

- Datenschutz-Review abgeschlossen.
- Erasure Integration Test gruen.
- Retention Job Observability vorhanden.

### Phase 4: Frontend, Accessibility und Design

Ziel: Community ist nutzbar, testbar, zugänglich und Design-System-konform.

Aufgaben:

- Static DOM Contracts aktualisieren.
- Native `confirm()`/`prompt()` ersetzen.
- Keyboard- und Screenreader-Flows fuer Feed, Composer, Tabs, Modals, DMs und Admin Queue pruefen.
- WCAG 2.2 AA Checkliste fuer Community.
- Mobile Viewports und long-content states testen.
- Inline Styles abbauen.

Exit Criteria:

- Static Tests gruen.
- Playwright E2E fuer wichtigste Community-Flows gruen.
- Accessibility-Audit ohne kritische Blocker.

### Phase 5: Observability und Betrieb

Ziel: Produktion kann betrieben und verteidigt werden.

Aufgaben:

- Community SLOs definieren: availability, p95 latency, report SLA, worker freshness.
- Metrics fuer Feed, Search, Post, Comment, DM, Report, Moderation, Rate-Limit, Sanitizer, OpenGraph.
- Alerts und Dashboards.
- Runbooks: DB outage, Redis outage, XSS, spam wave, moderation backlog, privacy request.
- Load Tests fuer Feed/Search/Post/DM.

Exit Criteria:

- Dashboards und Alerts live.
- Lasttest bestanden.
- Rollback- und feature-flag playbook dokumentiert.

### Phase 6: Release Candidate und Pilot

Ziel: Kontrollierter Launch statt Big Bang.

Aufgaben:

- DMs/Weekly Digest/Public Profiles nur aktivieren, wenn Gate bestanden.
- Interner Pilot mit Moderator Coverage.
- Beta mit begrenzter Nutzergruppe und aktivem Monitoring.
- Post-launch review nach 48 Stunden, 7 Tagen und 30 Tagen.

Exit Criteria:

- Keine offenen P0/P1.
- Alle Release Gates gruen.
- Moderator/Support/Incident-Rollen besetzt.

## 12. Production-Live-Akzeptanzkriterien

Community darf als production-live gelten, wenn alle folgenden Bedingungen erfuellt sind:

1. P0- und P1-Befunde dieses Audits sind geschlossen oder per dokumentierter Risikoakzeptanz entschieden.
2. `cargo check`, `cargo test`, fokussierte Community Unit/Integration Tests und Community E2E laufen in CI gruen.
3. Authenticated/unauthenticated Endpoint Matrix ist getestet.
4. XSS/CSRF/SSRF/Rate-Limit-Testfaelle sind gruen.
5. CSP ist fuer Community-Pfade mindestens in Report-Only sauber, Zielzustand ohne `unsafe-inline`.
6. Datenschutz-/Retention-Konzept ist implementiert und reviewed.
7. DM-Funktion ist entweder production-ready oder per Feature Flag deaktiviert.
8. Moderation Runbook, Report Queue, Admin Audit und SLA-Monitoring sind live.
9. Observability Dashboard und Alerts sind aktiv.
10. Rollback-/Feature-Flag-Plan ist dokumentiert und geprobt.

## 13. Empfohlene naechste Dokumente

Auf Basis dieses Audits sollten als naechstes erstellt werden:

1. `docs/community-production-readiness-roadmap.md` mit Tickets, Ownern, Aufwand und Reihenfolge.
2. `docs/community-endpoint-authorization-matrix.md`.
3. `docs/community-trust-safety-policy.md`.
4. `docs/community-data-retention-matrix.md`.
5. `docs/community-release-gates.md`.

## 14. Kurzfazit

Die Community ist nicht "unfertig" im Sinne fehlender Oberflaechen oder leerer Module. Sie ist funktional breit, aber noch nicht ausreichend hart fuer Production Live. Die naechste Arbeit sollte nicht primär Feature-Ausbau sein, sondern Production Hardening: API-Grenzen, Content-Security, Abuse-Schutz, Datenschutz, Moderationsbetrieb, Observability und reproduzierbare Tests. Erst danach ist eine sinnvolle Erweiterung der Community-Futures und des Gesamtfunktionsumfangs belastbar planbar.
