# Community Gap Closeout — Phase 2 (remaining 12 features)

**Companion to:** `docs/community/COMMUNITY_GAP_CLOSEOUT.md` (the original full brief)
**Created:** 2026-05-11
**Predecessor session shipped:** 12 of 24 features (see Phase 14.8 rows + Live Agent Logs in `IMPLEMENTATION_ROADMAP.md`).

You are a senior full-stack engineer (Rust + Axum + SQLx + MiniJinja backend; vanilla HTML + CSS + JS + HTMX frontend) finishing Phase 14.8 Community Gap Closeout on the POOOL platform. Twelve of the 24 features in the original brief already landed on `main`. **Read the existing brief end to end before writing anything; that file is the source of truth for stack, conventions, and per-feature contracts.**

## What's already done (do NOT redo)

Confirmed via `git log` and `docs/IMPLEMENTATION_ROADMAP.md` Phase 14.8 rows:

- ✅ 14.8.1 Ban-appeal submission UI (commit `a578897`)
- ✅ 14.8.2 Block / mute self-service (commits `9c4e2c1` + `d114559`; migration `027_block_mute.sql`)
- ✅ 14.8.3 Post edit own (commit `9c4e2c1`)
- ✅ 14.8.4 Post delete own (commit `9c4e2c1`)
- ✅ 14.8.5 Comment edit (commit `eedffff`; migration `028_comment_edits.sql`)
- ✅ 14.8.6 Comment reactions (commit `7e88df2`; migration `029_comment_reactions.sql`)
- ✅ 14.8.7 Profile picture upload (commit `204b427`)
- ✅ 14.8.8 Hashtag browse page (commit `26f7b4e`)
- ✅ 14.8.10 Announcement detail page (commit `0409ab4`)
- ✅ 14.8.11 Challenge participation — **partial** (commit `9181912` ships progress + action deeplink; join / submit / vote endpoints + admin curation still pending)
- ✅ 14.8.21 Admin ban-appeals review UI (commit `b5c3568`)
- ✅ 14.8.22 Admin user audit log viewer (commit `e4c31a0`)
- ✅ Bonus: Followers/Following list view (commit `b43b8bd`)

Topbar already has the `community-leaderboard-tab` button wired as a client-only tab (`data-client-tab="leaderboard"`) — the partial + endpoint still need to land.

## Remaining 12 features (your scope)

The numbering matches the original brief at `docs/community/COMMUNITY_GAP_CLOSEOUT.md`. **Read that file** — it has the per-feature contract; this companion is just the snapshot.

### Critical safety / discovery

1. **14.8.9 Global leaderboard view** — tab button exists; the partial + `GET /api/community/leaderboard/global?metric=xp&timeframe=alltime|weekly|monthly` endpoint do not. Reuse `.circle-lb-item__*` primitives. Migration likely not needed (read from `community_profiles.total_xp` for alltime; weekly/monthly is a follow-up if expensive).
2. **14.8.11 Challenge participation completion** — add the missing pieces on top of commit `9181912`: `POST /api/community/challenges/:id/join`, `POST /api/community/challenges/:id/submit`, vote/upvote endpoint, and the JS submission modal. Migration may add `challenge_submissions` (vote-based challenges).
3. **14.8.19 Search filters** — extend the existing `search_community` route at `routes.rs:2619` (which currently accepts `q/type/page` only) with `date_from`, `date_to`, `author_id`, `min_engagement`. Wire `feed-toggle-btn` chips on the search tab.

### Engagement depth

4. **14.8.12 Nested comment replies** — migration: `parent_comment_id UUID NULL` self-FK on `comments`; service depth cap of 2 (no replies to replies). FE indents replies inside the existing comments section. Counter stays a flat total.
5. **14.8.13 Achievement/badge detail page** — `GET /api/community/badges/:id` user-facing endpoint (admin badge CRUD already exists). New `/community/badge/:id` page using `.community-panel` / `.community-profile-badge` primitives. Link from `profile-modal-badges`.
6. **14.8.14 Asset reviews surface inside community** — backend CRUD exists at `routes.rs:2309`. FE-only: cross-link the latest community reviews on `frontend/platform/asset-detail.html` (and the reverse — a "Reviews" filter pill on Feed).
7. **14.8.15 Notification preferences page** — migration: `notification_prefs` key/value (or per-channel boolean columns on `community_profiles`). `GET/PUT /api/community/notification-prefs`. New sub-page `/settings/notifications/community` with `ds-form-group` + checkbox grid.
8. **14.8.16 Verified-owner request flow** — migration: `verified_owner_requests` (user_id, asset_id, status, evidence_url, reviewed_at, reviewer_id, created_at). User `POST /api/community/verified-owner-requests` + admin `GET/PATCH /api/admin/community/verified-owner-requests/:id`. Frontend: user submission form with asset picker from portfolio; admin review UI mirrors the appeals-review pattern in `admin/community/appeals.html`.
9. **14.8.17 Poll result visualisation polish** — pure frontend. Replace the stub poll viz with a bar-per-option layout using `.ds-progress` for proportional fills, mark the user's selected option, show "X% · N votes" per row.
10. **14.8.18 Mentions / hashtag autocomplete** — new endpoints `GET /api/community/users/autocomplete?q=` (top 8 by display_name prefix) and `GET /api/community/hashtags/autocomplete?q=`. Frontend: floating list below cursor in the composer textarea on `@`/`#` + 2 chars. 150ms debounce. Selection inserts the resolved `@display_name` or `#tag`.

### New surface

11. **14.8.20 Direct messages (DMs)** — biggest single item, treat as its own 3-commit slice. Migration: `dm_threads` (id, participant_a_id, participant_b_id, last_message_at, deleted_at_a, deleted_at_b) and `dm_messages` (id, thread_id, sender_id, content, created_at, read_at_recipient). Endpoints: `GET /api/community/dms/threads`, `GET /api/community/dms/threads/:id/messages`, `POST /api/community/dms/threads/:id/messages`, `POST /api/community/dms/threads` (create with first message). **Enforce block + mute rules** from 14.8.2 — refuse to create/send a thread between blocked users. Frontend: new tab `data-tab="community-dms-tab"` with split-pane layout (threads list left, conversation right at desktop; stacked at mobile). HTMX polling is **out of scope this phase**; just refresh on tab open + after each send. Out of scope: typing indicators, presence, push.

### Admin tail

12. **14.8.23 Admin AMA status / answer / feature management** — endpoints already exist at `routes.rs:2345-2358` (status, answer, feature). Wire them into `frontend/platform/admin/community/amas.html`: status select per AMA, answer textarea per question, "Feature this question" toggle.
13. **14.8.24 Admin community settings page** — migration: `community_settings` key/value table. `GET/PUT /api/admin/community/settings`. New admin page `frontend/platform/admin/community/settings.html`.

## Stack reality (do not assume otherwise)

- Backend: Rust + Axum + SQLx (compile-time-checked macros where applicable). Community DB is a **separate Postgres pool** (`get_community_pool`); routes must render `partials/community_disabled.html` gracefully when the pool is `None`.
- Frontend platform: **vanilla HTML + CSS + JS + HTMX — no framework, no bundler.** Alpine is in use where existing pages already use it; do not introduce it on new pages.
- Auth: session-cookie `poool_session`; CSRF token via the shared fetch interceptor in `components/head.html`. JS must call the existing `csrfHeaders()` helper (in `community-feed.js`) on all mutating fetches.
- Money rule: `BIGINT` cents on the server, format only at display time.

## Authoritative inputs (read in this order before code)

1. `AGENTS.md` — "BEFORE WRITING ANY CODE" block.
2. `docs/AGENT_DEVELOPMENT_PROMPT.md` — zero-defect standards.
3. `docs/DESIGN.md` — §9 Components, §10 States, §11 Accessibility, §17 Investor Dashboard Consistency Checklist.
4. `docs/IMPLEMENTATION_ROADMAP.md` — claim your task ID in the Live Agent Logs and the Phase 14.8 rows.
5. `docs/community/COMMUNITY_GAP_CLOSEOUT.md` — full per-feature contract; this companion is the short version.
6. The Phase 14.8 commits listed above (`git show <sha>` to learn the pattern; e.g., the ban-appeal banner pattern from `a578897` is the cleanest template for surfacing a new feature in `community.html`).
7. `tests/e2e/test_community.py` — six 14.8.* tests already pass; copy their fixture helpers (`_csrf_headers`, `_seed_community_post`, `_make_e2e_user_via_db`) instead of reinventing.

## Constraints (do not violate)

- HTMX tab contract — `data-tab` values stay stable. New tabs add new values; never reuse.
- Shared primitive reuse — the 17 redesign + 6 new 14.8.* primitive families on `community.css` cover most needs. New CSS class only when truly justified; document the new class in the commit body.
- Disabled-pool guard — every new HTMX partial handler that touches `state.community_db` must check `is_none()` and render `partials/community_disabled.html`.
- Permissions — every new admin route checks `admin` role (existing pattern in `backend/src/admin/`). Audit-log moderation-class actions via `community/audit.rs`.
- Do not modify `backend/src/db.rs` or `pgbouncer/entrypoint.sh` (AGENTS.md hard rule).
- No new frontend frameworks.

## Working method (per feature)

1. **Claim the row** in `docs/IMPLEMENTATION_ROADMAP.md` Phase 14.8 + add a Live Agent Log entry with status `🔄 IN PROGRESS`.
2. **Vertical slice**:
   1. Migration if needed (next sequence number after the highest existing `0XX_*.sql` in `database/community/`; today that's `029`, so start at `030`).
   2. Backend service + route. `cargo check` from `backend/`.
   3. Frontend partial + JS wire-up using existing class primitives.
   4. E2E test in `tests/e2e/test_community.py` (or `test_admin_community_*.py` for admin features). Cover happy path + auth/permission failure + empty/error state. Use the existing helpers.
   5. Verify with `preview_*` tools at 375 / 768 / 1440. Keyboard walkthrough mandatory.
   6. Commit with conventional format: `feat(community): <feature> (14.8.X)` or `feat(community-admin): …`.
3. **Roadmap update**: flip status to `✅ DONE`, fill Assignee + commit ref, add Live Agent Log check-out entry. Single commit `docs(community): mark 14.8.X DONE`.
4. **Maximum one feature per commit pair (code + docs)**. The largest item (14.8.20 DMs) is the only one allowed up to 3 code commits (migration / backend / frontend).

## Recommended commit order

P1-discovery → P2 cheap → P3 → Admin tail. Specifically:

1. **14.8.17 Poll viz polish** (no backend, no migration) — warm-up, ships in one commit.
2. **14.8.13 Badge detail page** (1 new user endpoint + 1 new page).
3. **14.8.18 Autocomplete** (2 new endpoints + composer JS).
4. **14.8.19 Search filters** (extend existing route + 4 chips on search tab).
5. **14.8.14 Asset reviews surface** (FE-only on asset detail + Feed pill).
6. **14.8.9 Global leaderboard** (new endpoint + new partial; tab button already exists).
7. **14.8.15 Notification prefs** (migration + endpoints + new settings sub-page).
8. **14.8.16 Verified-owner request flow** (migration + 2 endpoints + 2 UIs, can split admin to last).
9. **14.8.12 Nested comment replies** (migration + service signature change + FE indent).
10. **14.8.11 Challenge participation completion** (3 endpoints + submission modal).
11. **14.8.23 Admin AMA wiring** (FE only on existing admin page).
12. **14.8.24 Admin community settings page** (migration + 2 endpoints + new admin page).
13. **14.8.20 DMs last** — largest, isolated, doesn't unblock anything else and would lock high-traffic files for an extended period if started earlier.

## Definition of done per feature

- Migration applies cleanly on a fresh DB.
- `cargo check` and `cargo clippy` clean.
- E2E test added and passing locally with the full `test_community.py` suite still passing.
- DESIGN.md §17 checklist passes for the new UI.
- Keyboard-only walkthrough works.
- Empty / loading / error / disabled-pool states render.
- Mobile (375), tablet (768), desktop (1440) all verified via `preview_*`.
- Commit message describes the feature, links any new class or token, notes follow-ups.
- Roadmap row flipped to `✅ DONE` with commit ref; Live Agent Log entry recorded.

## First response (from the executing agent)

Produce a **prioritized execution plan** (no code yet):
1. For each remaining feature, do a grep-verified pre-flight: does the endpoint already exist? does the migration table already exist? does the partial already render?
2. Flag any drift between this prompt and what's currently on `main` (parallel agents have been busy — assume some adjacent work may have landed between this brief being written and your start).
3. List any new test fixtures you'll need in `tests/e2e/conftest.py`.
4. Confirm or revise the commit order above with rationale.
5. Identify any feature that would require a `ds-*` cross-page primitive — flag it; do not promote without explicit approval.

After plan approval, start at the first remaining item and ship one feature at a time.
