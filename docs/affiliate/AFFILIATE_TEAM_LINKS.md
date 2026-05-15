# Developer-Team-Affiliate System

> Phase 1–6 abgeschlossen. Stand: 2026-05-15.

## Was ist das

Erweiterung des bestehenden Affiliate-Systems um ein **Team-Modell**:

- **Personal-Affiliate** (Bestand) — User hat eigenen Code, kassiert Provisionen selbst.
- **Developer-Team-Affiliate** (neu) — Developer hat ein Team mit N Mitarbeitern. Jeder Mitarbeiter bekommt einen *Business*-Affiliate-Link, dessen Provisionen + Auszahlungen an den Developer fließen. Mitarbeiter behält parallel seinen privaten Affiliate-Link.

Trennung: ein Mitarbeiter kann beide Kontexte haben — Personal-Daten + Business-Daten werden strikt voneinander getrennt gehalten, mit eigenem Mode-Switcher im Dashboard.

## Architektur-Kern

Drei Konzepte, die das System trägt:

1. **Link-Ownership** (`payout_user_id`) — wem gehört der Link wirtschaftlich
2. **Link-Attribution** (`attribution_user_id`) — wer hat ihn ausgeführt
3. **Link-Type** — `personal` (Owner = Attribution) oder `team_business` (Owner ≠ Attribution, Team-Bezug zwingend)

Diese sind in `affiliate_links` als first-class Spalten modelliert, mit DB-CHECK-Constraint der die Shape-Invariants garantiert.

## Datenbank — neue/erweiterte Tabellen

### `developer_teams` (Migration 156)
```
id, developer_user_id, display_name, public_slug, is_default,
status (active/paused/terminated), created_at, updated_at,
terminated_at, terminated_reason
```
- `one_default_team_per_developer` partial-unique (heute genau 1 Default-Team pro Developer; Multi-Team-vorbereitet)
- `developer_teams_public_slug_uniq` (case-insensitive, für `/affiliate/join/<slug>`)
- RLS-Policy: nur Owner liest eigene Teams

### `developer_team_memberships` (Migration 156)
```
id, team_id, user_id, role (member/manager),
status (invited/pending_developer_approval/active/removed),
invitation_token_hash, invitation_expires_at,
invited_by_user_id, invited_at, joined_at, removed_at,
removed_reason, removed_by_user_id, ...
```
- `one_active_membership_per_user` partial-unique → ein User kann nur in **einem** nicht-removed Team gleichzeitig sein

### `affiliate_links` (Migration 157)
```
id, code (UNIQUE), link_type ('personal'|'team_business'),
attribution_user_id, payout_user_id, team_id (nullable),
status (active/inactive/suspended), ...
```
- CHECK `affiliate_links_shape_check`:
  - Personal → `team_id IS NULL AND attribution = payout`
  - Team-Business → `team_id NOT NULL AND attribution ≠ payout`
- Indizes auf `attribution_user_id`, `payout_user_id`, `team_id`, `(link_type, status)` — alle partial-WHERE `status='active'`

### Erweiterungen
- `affiliate_referrals` → +`link_id`, `attribution_user_id`, `payout_user_id` (NOT NULL nach Backfill)
- `affiliate_commissions` → dieselbe Erweiterung
- `referral_clicks` → `+link_id` (NULLABLE für Legacy-Rows), monatlich partitioniert (Migration 160)

### Performance-Tables (Migration 159)
- `affiliate_daily_rollups` (rollup_date × link_id) — pre-aggregiert Clicks/Signups/Qualified/Revenue/Commission. Worker `run_affiliate_rollup_worker` läuft alle 15 Min und UPSERTet `days_back=2` (idempotent).
- `affiliate_live_counters` (PK payout_user_id) — O(1) Counter-Tile für Dashboard. Auto-gepflegt via DB-Trigger `trg_affiliate_commissions_counter_sync` (Migration 161).

### Partitionierung (Migration 160)
- `referral_clicks` ist jetzt RANGE-partitioniert by `created_at` (monatlich)
- Helper-Funktionen: `referral_clicks_ensure_future_partitions(months_ahead)` und `referral_clicks_drop_old_partitions(retain_months)`
- Worker `run_referral_clicks_partition_maint_worker` läuft täglich

## Backend — Service Layer

### `backend/src/rewards/team_links.rs`
- `find_active_by_code(code)` — Cookie/URL-Code-Auflösung
- `create_personal_link(user_id)` — idempotent, beim Onboarding-Approve
- `create_team_business_link(team_id, member_id, actor_id)` — validiert active membership
- `deactivate_link(link_id, actor, reason)` — soft-delete
- `deactivate_all_team_business_for_member(team_id, member_id, actor, reason)` — off-boarding cascade
- `ensure_developer_has_affiliate_row(developer_id)` — auto-seedet Developer affiliate-Profil als Payout-Recipient

### `backend/src/rewards/team_members.rs`
- `invite_by_email(team_id, email, inviter_id)` → SHA-256-Token, 14-Tage TTL, triggert Email `team_invitation_received`
- `accept_invitation(user_id, token)` → flippt auf active, erzeugt Business-Link, sendet `team_member_approved`
- `self_request_join(user_id, developer_slug)` → status `pending_developer_approval`, sendet Email an Developer
- `approve_pending(membership_id, developer_id)` → active + Business-Link
- `remove_member(membership_id, actor_id, reason)` → status removed, cascade deactivate links, sendet Email an Member

### `backend/src/rewards/team_reports.rs`
- `team_period_summary` / `team_period_by_member` / `team_customers` / `team_products` — alle lesen `affiliate_daily_rollups` (O(31) statt O(N))

### `backend/src/rewards/team_routes.rs`
9 Developer-Endpoints unter `/api/developer/affiliate/team/...` (DeveloperUser-gated) + 3 Member-Self-Service Endpoints unter `/api/affiliate/team/...`

### `backend/src/admin/affiliate_teams.rs`
6 Admin-Endpoints unter `/api/admin/affiliate-teams/...` gegated über `affiliates.team_manage`. Inkl. **Move-Member** für Cross-Team-Wechsel.

## Frontend — Pages

### Developer
- `/developer/affiliate-team` (Tabs: Members, Customers, Products, Settings) — 5 KPI-Tiles + Invite-Modal

### Member
- `/affiliate/dashboard` (Bestand erweitert): **Mode-Switcher** Personal/Business (URL-Param `?ctx=`), **Team-Banner** mit Business-Link + Copy-Button

### Admin
- `/admin/affiliate-teams` — Listing mit Status-Filter + Detail-Drawer (Counters, Members mit Move/Remove, Audit-Stream)

## Email-Templates (Migration ./email.rs)
| Event | Empfänger | Trigger |
|---|---|---|
| `team_invitation_received` | invited user | `invite_by_email()` |
| `team_member_approved` | member | `accept_invitation()` / `approve_pending()` |
| `team_member_removed` | member | `remove_member()` |
| `team_self_request_received` | developer | `self_request_join()` |

Alle gehen durch `transactional_email_outbox` mit durable retry.

## Permissions

| Permission | Rolle | Wofür |
|---|---|---|
| `affiliates.manage` (Bestand) | admin, super_admin | Affiliate-Onboarding, Payouts, Clawback, Fraud |
| `affiliates.team_manage` (neu, Mig 158) | admin, super_admin | Team-Verwaltung: suspend/resume/terminate, member-move |
| `rewards.manage` (Mig 151) | admin, super_admin | User-Tiers + Rewards-Balance |

Developer-Endpoints werden nicht über `admin_permissions` gegated — sie nutzen `DeveloperUser`-Extractor + inline Team-Ownership-Check via `require_team_owner`.

## Skalierung

| Hot-Path | Lösung |
|---|---|
| Dashboard KPI-Tile | `affiliate_live_counters` PK-Lookup, O(1) |
| 30-Tage Team-Report | `affiliate_daily_rollups` index-only-scan, O(31 × n_links) |
| Click-Insert | Append-only auf monatliche Partition |
| Click-Retention | Drop alter Partition statt DELETE |
| Holdback-Worker | `FOR UPDATE SKIP LOCKED` + batched |

## Self-Referral Guards

Drei Ebenen:
1. **DB-CHECK** in `affiliate_referrals`: referred ≠ attribution UND referred ≠ payout
2. **App-Logik** in `attribute_affiliate_referral`: bei Team-Business — referred darf nicht selbst Mitglied desselben Teams sein
3. **IP-Overlap** (Bestand) — Fraud-Matrix F.1 gegen `payout_user_id`

## Edge-Cases (gelöst)

| Case | Verhalten |
|---|---|
| Member verlässt Team | Membership `removed`, alle Business-Links deaktiviert, historische Commissions bleiben |
| Member wechselt zu anderem Developer | Admin-Override Move: alte Membership removed → neue active in Ziel-Team; alte Commissions bleiben bei altem Developer |
| Member ist Personal-Affiliate UND Team-Member | Erlaubt; getrennte `affiliate_links`-Rows, getrennte Counter, Mode-Switcher in UI |
| Customer klickt zwei Links | First-touch (Cookie-Wert wird nicht überschrieben) — Bestandsverhalten |
| Developer-Account gesperrt mit offenen Holdbacks | Commissions bleiben `frozen` (manuelle Admin-Entscheidung) |
| Team terminated | Cascade-deactivate alle Business-Links; historische Commissions bleiben unverändert |

## Rollout-Plan

### Voraussetzungen
- Migrationen 156–161 applied (`SELECT filename FROM _schema_migrations WHERE filename LIKE '15%_%' OR filename = '161_%'`)
- Backend mit `affiliate_team_*` Modulen
- Frontend-Assets in `static/{css,js}` deployed

### Schritte
1. **Migrate** — alle 6 Migrationen in production einspielen
2. **Verify** — `SELECT * FROM affiliate_links WHERE link_type='personal'` muss N=count(affiliates) liefern (Backfill-Check)
3. **Boot** — Backend mit neuen Workern starten; im Log nach `📊 Affiliate rollup worker armed` und `📦 referral_clicks partition maintainer armed` suchen
4. **Smoke** — `GET /api/developer/affiliate/team` als Developer-User; `GET /api/affiliate/team/my-membership` als regulärer User (→ status: "none" für non-members)
5. **Backfill-Rollups** — einmaliger Admin-Trigger: `SELECT recompute_rollups_for_recent_days(...)` (Rust-side, oder `cargo run --bin rollup_backfill` falls separater Binary erwünscht — heute nicht enthalten)
6. **Enable** in Production durch Sidebar-Nav-Item für Developers + Admin-Sidebar-Item für Admins; keine Feature-Flag heute (Tabelle-Bestand schon backfilled bedeutet Bestandsaffiliates unbeeinflusst)

### Rollback (falls nötig)
- Code-Rollback: keine Datenmigration nötig — bestehende Bestandstabellen (`affiliates`, `affiliate_referrals`, `affiliate_commissions`) bleiben durch Backfill-Spalten konsistent. `affiliate_id` weiter gleich `payout_user_id`; alte Read-Pfade funktionieren.
- DB-Rollback: nicht trivial — `DROP TABLE affiliate_links CASCADE` würde `affiliate_referrals.link_id` brechen. Stattdessen: leave-as-is + Code zurückrollen.

## Testing

`backend/tests/affiliate_team_integration.rs` — **11 Tests, alle PASS**:
1. Shape-Constraint blockt personal+team
2. Shape-Constraint blockt team_business mit attribution=payout
3. Partial-Unique blockt zweite aktive Membership
4. `create_personal_link` idempotent
5. `create_team_business_link` idempotent + erzwingt active membership
6. Personal-Attribution schreibt self als payout
7. Team-Business-Attribution splittet attribution vom payout
8. Self-Referral wird geblockt
9. Same-Team-Member kann nicht in eigenes Team referen
10. Off-Boarding deaktiviert alle Business-Links + Membership status removed
11. Dashboard-Context-Filter (Personal vs Business) funktioniert

Run mit:
```sh
DATABASE_URL=postgres://martin@localhost/poool \
  cargo test --test affiliate_team_integration -- --ignored --test-threads=1
```

## Bekannte offene Punkte

- **Admin: Conflict-Resolution-Worker** für Cross-Team-Attribution-Konflikte (heute manuell via Audit-Stream)
- **W-9/W-8BEN Tax-Forms** — bewusst aus Scope rausgenommen (User-Entscheidung Q3)
- **Tax-Status read-only-Display** im Business-Mode UX — KPI-Tiles werden nur als "informational" gelabelt
- **Multi-Team pro Developer** — Datenmodell vorbereitet (`is_default` Spalte), UI/Service unterstützt heute nur Default-Team
- **Email-Outbox SMTP-Konfiguration** für Production — Templates vorhanden, SMTP-Test in Dev steht aus

## Migration-Inventar

| # | Datei | Phase |
|---|---|---|
| 156 | `developer_teams.sql` | 1 |
| 157 | `affiliate_links.sql` | 1 |
| 158 | `affiliate_team_manage_permission.sql` | 1 |
| 159 | `affiliate_rollups.sql` | 1 |
| 160 | `referral_clicks_partition.sql` | 1 |
| 161 | `affiliate_live_counters_trigger.sql` | 2 |
