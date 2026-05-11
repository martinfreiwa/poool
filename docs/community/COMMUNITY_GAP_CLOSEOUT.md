# Community Gap Closeout — Implementation Brief

**Phase:** 14.8 (extends Phase 14 Community System Modular Rollout)
**Created:** 2026-05-11
**Prerequisite:** Phase 14.1 – 14.6 (✅ DONE) and the 17-commit Community redesign (lands on `main` 2026-05-11).
**Out of scope this brief:** Phase 14.7 "Module 6: Advanced Engagement" (Spaces, Ideation Boards, Rich Embeds). DMs are scoped here as `14.8.20` because they are a frequently-requested user-facing primitive, but the rest of Module 6 stays deferred.

This file is the source-of-truth brief for the next agent / engineer. It mirrors the prompt issued on 2026-05-11 and is referenced from the Phase 14.8 rows in `IMPLEMENTATION_ROADMAP.md`.

---

## Stack reality (do not assume otherwise)

- Backend: Rust + Axum + SQLx (compile-time-checked macros) + MiniJinja SSR templates. Money values are `BIGINT` cents. All financial ops wrap in DB transactions.
- Frontend platform: **vanilla HTML + CSS + JS + HTMX — no framework, no bundler.**
- Community runs on a **separate Postgres pool** (`get_community_pool`) and may be unconfigured in some envs — features must render the `partials/community_disabled.html` card gracefully, not 500.
- Auth: session-cookie `poool_session`; CSRF token via shared fetch interceptor in `components/head.html`.
- Admin pages live under `frontend/platform/admin/community/`; investor surfaces under `frontend/platform/community.html` + `partials/community_*.html`.

## Authoritative inputs (read in this order)

1. `AGENTS.md` — repo rules, especially "BEFORE WRITING ANY CODE".
2. `docs/AGENT_DEVELOPMENT_PROMPT.md` — zero-defect standards. Pay attention to money rules and transaction discipline.
3. `docs/DESIGN.md` — apply to every new UI you add. Especially §9 Components, §10 States, §11 Accessibility, §17 Investor Dashboard Consistency Checklist.
4. `docs/IMPLEMENTATION_ROADMAP.md` — claim your tasks; coordinate via Live Agent Logs.
5. `docs/DATABASE_SCHEMA.md` — current schema.
6. **The 17 recent community redesign commits** (`git log --oneline | grep "community"`). Don't undo their work; build on top:
   - Shared primitives: `.ds-card`, `.community-modal__*`, `.community-comment-row__*`, `.community-loading-state`, `.community-empty-state__*`, `.community-disabled`, `.community-modal__switch`, `.community-xp-row__*`, `.circle-lb-item__*`, `.community-invite-row__*`, `.community-request-row__*`, `.community-ama-question__*`, `.community-hashtag-banner__*`, `.community-toast`
   - Existing HTMX tab contract (data-tab values + endpoints) — never change.
7. Existing community page audits in `docs/page-audits/2026-04-25-*.md` and `2026-04-28-*.md` — historical defects + fix history.
8. Test patterns: `tests/e2e/test_community.py`, `tests/e2e/test_admin_community_*.py`, `tests/test_community_tab_contract_static.py`, `tests/e2e/conftest.py` (especially `create_e2e_user`).
9. Database migrations: `database/community/` (sequential numbered files). New tables/columns ship as new numbered migration files.
10. Backend routes: `backend/src/community/{routes.rs, service.rs, models.rs, mod.rs, amas.rs, circles.rs, challenges.rs, audit.rs, moderation.rs, notifications.rs, reviews.rs}`. All routes registered in `backend/src/main.rs`.
11. Existing JS: `frontend/platform/static/js/community-{feed,circles,amas,challenges,announcements}.js` and `htmx-init.js`.

## Database policy

- New tables/columns → new migration file `database/community/0XX_<feature>.sql` (next sequence number after `026`).
- Never edit a shipped migration. Schema changes only via new migrations.
- All foreign keys to user IDs reference `community_profiles` or core `users` per existing convention — match the column the related table already uses.
- Counters maintained by Rust transactions, not by triggers, unless an existing trigger pattern exists for that table.
- Indexes for any new lookup pattern. Soft-delete via `deleted_at TIMESTAMPTZ NULL` where the data is user-visible content.

## Scope — 24 feature gaps grouped by priority

Take features in priority order. **Within each priority, ship vertical slices: migration → service → route → frontend wire-up → e2e test → commit.** Do not stack incomplete features.

### P0 — Safety & compliance (do first)

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.1 | Ban appeal submission flow (user UI) | Backend exists at `routes.rs:4844 submit_ban_appeal`. Build banner on `/community` shown when `is_community_banned = true`; banner opens `ds-modal` with textarea + submit. Confirm with `community-toast`. |
| 14.8.2 | Block / mute another user (self-service) | Backend missing. Add `POST /api/community/users/:id/block`, `DELETE /api/community/users/:id/block`, `GET /api/community/blocks`. Same shape for mute. Migrations: `block_relationships`, `mute_relationships` (unique actor_id/target_id). Enforce on feed query + notification creation. Frontend: user profile modal action menu + new `/community/blocks` settings sub-page. |

### P1 — Own-content management

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.3 | Post edit (frontend wire-up) | Backend exists: `routes.rs:924 update_user_post PUT`. Kebab menu on post-card visible only when `p.author_id == current_user_id`; opens composer in modal with prefilled content + image. Pass `current_user_id` through `Context` in `community_feed_list_htmx`. |
| 14.8.4 | Post delete (own) | Wire backend `routes.rs:956 delete_user_post`. `window.confirm` per existing `handleDeleteCircle` pattern. |
| 14.8.5 | Comment edit | Backend missing: add `PUT /api/community/comments/:id`. Migration: add `edited_at TIMESTAMPTZ NULL`, `original_content TEXT NULL` to `comments`. Frontend: inline edit on `.community-comment-row` with Save/Cancel and "Edited" indicator. |
| 14.8.6 | Comment reactions | Migration: extend `reactions` table polymorphic (target_type + target_id) OR new `comment_reactions`. Document choice in migration comment. Backend: `POST /api/community/comments/:id/reactions` mirroring post reaction logic; same allowed values. Frontend: single `fire` reaction button per comment-row. |
| 14.8.7 | Profile picture upload | Backend exists for post images; add `/api/upload/avatar` writing `community_profiles.avatar_url`. Frontend: upload affordance in `edit-profile-modal` (already in `community.html`). |

### P1 — Discoverability

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.8 | Hashtag browse page | New SSR route `GET /community/hashtag/:tag` + HTMX partial. Reuse `routes.rs:5398 get_posts_by_hashtag`. Reuse `community_post_list.html` + new thin `community_hashtag_header.html`. Use `.community-hashtag-banner` class. |
| 14.8.9 | Global leaderboard view | Backend missing: `GET /api/community/leaderboard/global?metric=xp&timeframe=alltime|weekly|monthly`. Frontend: new partial `community_global_leaderboard.html`; new tab `data-tab="community-leaderboard-tab"` or sub-section of My Circle. Reuse `.circle-lb-item__*`. |
| 14.8.10 | Announcement detail page | New route `GET /community/announcement/:id`. Reuse `community_post_card.html`. Backlink to `/community?tab=announcements`. |
| 14.8.11 | Challenge participation flow | Confirm/add `POST /api/community/challenges/:id/join` and `/submit` + any voting endpoint. Migration: extend `challenge_progress` if join state isn't represented; add `challenge_submissions` if needed. Frontend: expand `community-challenges.js` (currently 74 lines, read-only) with Join button, submission modal, vote/upvote button. |

### P2 — Engagement depth

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.12 | Nested comment replies | Migration: `parent_comment_id UUID NULL` self-FK on `comments`. Service: thread depth cap of 2 (no replies to replies). Frontend: indent replies under parent inside existing comments section. |
| 14.8.13 | Achievement/badge detail page + earning rules | Confirm/add `GET /api/community/badges/:id`. New page `/community/badge/:id` using `.community-panel` / `.community-profile-badge` primitives. Link from `profile-modal-badges`. |
| 14.8.14 | Asset reviews surface inside community | Backend exists (`routes.rs:2309`). Cross-link: latest community reviews on asset detail; "Reviews" filter pill on Feed. |
| 14.8.15 | Notification preferences page | Confirm/add `GET/PUT /api/community/notification-prefs`. Sub-page `/settings/notifications/community` with checkbox grid using `ds-form-group`. |
| 14.8.16 | Verified-owner request flow | Migration: `verified_owner_requests` (user_id, asset_id, status, evidence_url, reviewed_at, reviewer_id). Backend: user `POST` + admin `GET/PATCH`. Frontend: user submission form (asset picker from portfolio). Admin review UI is task 14.8.23. |
| 14.8.17 | Poll result visualization polish | Replace stub viz with bar-per-option using `.ds-progress`. Show user's selected option marked. |
| 14.8.18 | Mentions / hashtags autocomplete | Backend: `GET /api/community/users/autocomplete?q=<prefix>` (top 8 by display_name prefix) + same for hashtags. Frontend: floating list below cursor in composer textarea on `@`/`#` + 2 chars. 150ms debounce. |
| 14.8.19 | Search filters | Extend `GET /api/community/search` with `?date_from=`, `?date_to=`, `?author_id=`, `?min_engagement=`. Frontend: filter row on Search tab using `feed-toggle-btn` chip pattern. |

### P3 — New surface

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.20 | Direct messages (DMs) | Migration: `dm_threads`, `dm_messages` (see brief). Backend: thread + messages endpoints. Enforce block/mute. Frontend: new tab `data-tab="community-dms-tab"` with split-pane (threads list left, conversation right desktop; stacked mobile). No typing indicators/presence/push this phase. |

### Admin track (parallel-safe)

| Task ID | Feature | Notes |
|---|---|---|
| 14.8.21 | Ban appeals review UI | Backend exists. New `frontend/platform/admin/community/appeals.html`. Table of pending appeals + detail modal with approve/deny. |
| 14.8.22 | User audit log viewer | Backend exists (`/api/admin/community/users/:id/audit-log`). New panel in `frontend/platform/admin/community/user-detail.html` using `ds-table`. |
| 14.8.23 | AMA status / answer / feature management | Wire existing endpoints into `frontend/platform/admin/community/amas.html`: status select, answer textarea, feature toggle. |
| 14.8.24 | Community settings page | Migration: `community_settings` key/value. Backend: `GET/PUT /api/admin/community/settings`. New admin page `admin/community/settings.html`. |

## Working method

1. **Inventory before coding.** Open `IMPLEMENTATION_ROADMAP.md`, claim the specific 14.8.X row, and verify no parallel agent is on it.
2. **Slice per feature:**
   1. Migration (if needed) — write, run via `sqlx migrate run`, commit.
   2. Backend route + service + model — `cargo check`, write a unit test in `community/tests.rs` for any non-trivial service logic, `cargo test community::tests`.
   3. Frontend partial + JS wire-up — use only the existing class primitives unless a new one is justified; document any new class in the commit message.
   4. E2E test under `tests/e2e/test_community*.py` covering happy path + at least one auth/permission failure + at least one empty/error state.
   5. Verify via `preview_*` tools at 375 / 768 / 1440. Keyboard walkthrough mandatory.
   6. Commit with conventional-commits style: `feat(community): <feature>` or `feat(community-admin): <feature>`.
3. **Commit cadence:** one commit per feature where possible. Three is the maximum split (migration / backend / frontend) for the bigger items like DMs.
4. **No `unwrap()` in production paths.** Use `AppError`. Propagate, don't mask.
5. **Disabled-pool guard:** every new HTMX partial handler that touches `state.community_db` must check `is_none()` and render `partials/community_disabled.html`.
6. **CSRF on all mutating fetches** — use the existing helper in `community-feed.js`.
7. **Permissions:** every new admin route checks `roles` contains `admin`. Audit log entries via `community/audit.rs` for moderation-class actions.
8. **Deploy:** `git push origin main` (CI/CD handles the rest).

## Constraints (do not violate)

- HTMX tab contract: existing `data-tab` values stay stable. New tabs add new values; never reuse.
- No backwards-compatibility shims. If a class or function gets renamed, update all callers in the same commit.
- No new frontend frameworks. Plain vanilla JS + HTMX + Alpine where Alpine is already in use.
- Do not modify `backend/src/db.rs` or `pgbouncer/entrypoint.sh` (AGENTS.md hard rule).
- Money rule: any monetary value is `BIGINT` cents server-side. Format only at display time.
- Reuse the 17 redesign primitives. New class only when truly needed (justify in commit message).

## Definition of done per feature

- Migration applied cleanly on a fresh DB.
- `cargo check` and `cargo clippy` clean.
- E2E test added and passing locally.
- DESIGN.md §17 checklist passes for the new UI.
- Keyboard-only walkthrough works.
- Empty / loading / error / disabled-pool states render.
- Mobile (375), tablet (768), desktop (1440) all verified via `preview_*`.
- Commit message describes the feature, links any new class, and notes follow-ups.

## First response (from the executing agent)

Produce a **prioritized execution plan** (no code yet):
1. Confirm or revise the priority order with rationale tied to user safety + business risk.
2. For each priority bucket (P0 / P1-content / P1-discovery / P2 / P3 / Admin), output:
   - Feature list in proposed commit order
   - Pre-flight checks (verify by grep — does the backend endpoint actually exist? does the migration table already exist?)
   - Estimated commit count
   - Migration count
3. Flag any conflicts: features whose migrations or routes would step on each other.
4. Identify the longest-pole feature (likely DMs) and whether to defer or start it first to unblock dependencies.
5. List test fixtures needed (e.g., seeded ban appeal, blocked relationship) — flag any that need new helpers in `tests/e2e/conftest.py`.

After the plan is approved, start at 14.8.1 and ship one feature at a time.
