# Community — Finalization & Test Hardening Prompt

> Drop this into a fresh Claude Code session. Self-contained — no prior context required.

This wraps up the multi-prompt community workstream. Previous prompts shipped:

- **Phase 1–4** (`docs/COMMUNITY_FIX_PROMPT.md`): 38 tasks fixing bugs + adding features (post edit/delete, ban appeals, saved tab HTMX, AMA admin actions, audit log, appeals UI, profile-pic upload, followers list, block/mute, challenges, announcements detail, hashtag page, leaderboard, comment edit/replies, reactions, autocomplete, multi-type reactions, shadowban indicator, notification prefs, verified-owner request, lightbox, infinite scroll, Ctrl+Enter, drag-drop).
- **WS1–3** (`docs/COMMUNITY_PROFILE_AND_DESIGN_PROMPT.md`): nested replies, pagination, verification denormalization, mobile tab UX, design-system rails, **full `/community/me` + `/community/u/:id` profile page** with 9 tabs (posts, comments, followers, following, media, circle, activity, analytics, settings).

**What's left** is the boring-but-important last mile: close out the deferred items, run the full test sweep clean, document the surface, and prepare for production push.

---

## Mission

Bring the community module to a production-ready state. Three workstreams, run in order:

1. **Test green-up** — every community-related test passes locally and the suite is reliable.
2. **Design pass close-out** — finish the WS2.6 admin polish across the remaining 13 pages, run a real axe-core a11y check, capture visual regression baselines.
3. **Production readiness** — verify migrations apply cleanly to a fresh DB, audit perf on the new profile endpoints, update `docs/CODEBASE_ARCHITECTURE.md`, deploy.

Each workstream commits independently. Use TodoWrite to track 15 tasks.

---

## Codebase architecture you must know

**Stack**
- Backend: Rust + Axum, SQLx + Postgres. **Important**: the entry point at `backend/src/main.rs` is now a thin shim that calls `poool_backend::run()`. All wiring lives in `backend/src/lib.rs` — the route mount is `pub fn build_platform_router(state: AppState) -> Router` near line 1121, and page handlers (`page_community_my_profile`, `render_community_profile`, etc.) start around line 3067.
- Frontend: Jinja2 (MiniJinja, **no `tojson` filter**) + HTMX + Alpine.js + vanilla JS.
- Design system: `ds-*` for user pages, `admin-*` for admin pages. Shared CSS lives in `frontend/platform/static/css/`.
- DB migrations: `database/community/NNN_xxx.sql`. **Next available number: 036.**
- Tests: pytest, E2E at `tests/e2e/test_community*.py`, static contracts at `tests/test_community_*.py`. Pytest config has `-x` in `addopts`; override with `-o "addopts=-v --tb=line"` for a full sweep.
- Auth fixture: `tests/e2e/conftest.py:authenticated_user_page` creates a user, attaches `poool_session` cookie, yields `(page, tracker, user)` where `user["user_id"]` is the canonical key (not `user["id"]`).

**Templates DO NOT auto-reload** — restart `backend` preview server after every template edit.

**Mobile topbar quirk**: `leaderboard.css` hides `.lb-topbar-tabs` at `<=768px` and is loaded AFTER `community.css`. To keep community tabs visible on mobile, override with the more-specific `.lb-topbar-tabs.community-topbar-tabs` selector (already done in WS1.5 follow-up).

**Conventions** (carry over from prior prompts):
1. Never inline `style="..."` in HTML or JS-built DOM. Use `ds-*` / `admin-*` / `community-*` classes.
2. User content → `textContent`. Server-sanitized HTML → `| safe` filter. Never `innerHTML` raw user input.
3. Write endpoints: CSRF → auth → ban check → validate → DB → audit (admin only).
4. Frontend fetches: `credentials: 'same-origin'`, `csrfHeaders()` helper, errors via `window.showToast`.
5. Migrations sequential and `IF NOT EXISTS`. Never modify an existing migration.
6. One logical concern per commit. Conventional commits format. Scope `community` for user-facing, `admin` for admin pages.

**Verification protocol** (after each commit):
1. `cargo check --manifest-path backend/Cargo.toml` clean.
2. Restart backend preview server (`mcp__Claude_Preview__preview_stop` then `preview_start name=backend`). Wait 30-45s for compile.
3. Re-run only the test files you touched, or the full sweep:
   ```
   python3 -m pytest tests/e2e/test_community.py tests/e2e/test_community_profile.py tests/test_community*.py -o "addopts=-v --tb=line"
   ```
4. `preview_eval` + `preview_console_logs` to verify no new client-side errors.

**Stop-and-ask triggers**: migration that drops or rewrites data; route signature change that breaks an existing client; two sources of truth for the same field.

---

## Workstream 1 — Test green-up

### 1.1 — `test_community_partial_requires_auth` async-loop error

Currently the test suite ends with `1 passed, 1 error` because this test (around `tests/e2e/test_community.py:170`) is wrapped to use Playwright's sync API from inside an asyncio loop — Playwright rejects this.

Either:
- Switch the test to use Playwright's **async** API (preferred — convert the function to `async def` and use `await page.goto(...)`).
- Or strip it of the asyncio context (less likely to be the right fix).

The test exercises the auth wall on `/community/partials/feed` and similar HTMX partial endpoints. Convert + commit as `fix(community): unwrap test_community_partial_requires_auth from asyncio context`.

### 1.2 — Restore full `-x` discipline

`pyproject.toml` `addopts` includes `-x`. The full community sweep currently needs `-o "addopts=-v --tb=line"` to see all failures. After 1.1 lands, the sweep should pass cleanly with the default config. Verify and commit a single change (or none) confirming the baseline.

### 1.3 — Backend integration tests

`backend/tests/` (Rust integration tests) should also build. Run:
```
cargo test --manifest-path backend/Cargo.toml --no-run
```
Fix any community-related compile errors. Add one new integration test exercising `GET /api/community/profile/:id/posts` against a seeded user.

### 1.4 — Migration cleanliness on fresh DB

Drop and recreate `poool_community` locally:
```
dropdb poool_community && createdb poool_community
```
Restart backend — it should apply all migrations (`001` through `035`) without error. Capture the log of any migration that fails and either fix the migration or add a `BEGIN; … COMMIT;` wrapper if it's a transient ordering issue.

Migrations to verify run cleanly:
- `033_comment_replies.sql`
- `034_verified_owner_profile.sql` (includes backfill from `verification_requests`)
- `035_profile_views.sql`

If you find an ordering issue, **do not modify the existing file**; add a `036_fix_xxx.sql` correcting it.

---

## Workstream 2 — Design-pass close-out

### 2.1 — Admin community pages: inline-style sweep

13 admin pages still have ~30-44 inline `style=""` attributes each. The rails are laid in `admin.css` from the previous prompt:
- `.admin-btn--warning` / `.admin-btn--danger`
- `.admin-table__cell-{primary,secondary,muted,text,actions}`
- `.admin-badge--status-{pending,approved,rejected,live,draft,archived,closed,accepting,scheduled}`
- `.admin-modal-overlay`, `.admin-modal{,--md,--lg}`
- `.admin-table__message{,--error}`

`admin-community-appeals.js` is the template — it's already migrated. Apply the same pattern to:

| File | Inline-style count (approx) |
|---|---|
| `frontend/platform/admin/community/badges.html` | 44 |
| `frontend/platform/admin/community/circle-detail.html` | 42 |
| `frontend/platform/admin/community/challenges.html` | 39 |
| `frontend/platform/admin/community/amas.html` | 39 |
| `frontend/platform/admin/community/announcements.html` | 36 |
| `frontend/platform/admin/community/post-detail.html` | 35 |
| `frontend/platform/admin/community/posts.html` | 19 |
| `frontend/platform/admin/community/reports.html` | 17 |
| `frontend/platform/admin/community/comments.html` | 17 |
| `frontend/platform/admin/community/leaderboard.html` | ~ |
| `frontend/platform/admin/community/user-detail.html` | ~ |
| `frontend/platform/admin/community/users.html` | ~ |
| `frontend/platform/admin/community/index.html` | ~ |

For each: replace inline styles with classes. Where a pattern isn't in `admin.css` yet, add it there rather than creating a one-off class. Commit per file (or per logical group of 2–3 small ones).

### 2.2 — Full axe-core a11y run

Run axe-core programmatically on every community-facing page via the preview tools. Sequence:

1. `preview_start` backend.
2. Log in via the e2e fixture's `attach_session_cookie` flow.
3. For each URL in:
   - `/community`
   - `/community/me`
   - `/community/u/<test-user-id>`
   - `/community/post/<seeded-post-id>`
   - `/community/hashtag/test`
   - `/admin/community/`
   - `/admin/community/users.html`
   - `/admin/community/appeals.html`
   - `/admin/community/circles.html`

   Inject axe-core from CDN via `preview_eval`:
   ```js
   const s = document.createElement('script');
   s.src = 'https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.10.0/axe.min.js';
   document.head.appendChild(s);
   await new Promise(r => s.onload = r);
   const results = await axe.run(document);
   return results.violations.map(v => ({id: v.id, impact: v.impact, nodes: v.nodes.length}));
   ```

4. Fix every `critical` and `serious` violation. Log `moderate` and `minor` in `qa-reports/community-a11y-audit.md` with a "deferred" disposition if non-trivial.

5. Commit per page once green: `fix(community): a11y violations on /community/me (axe-core)`.

### 2.3 — Visual regression baseline

For each viewport (1440px / 768px / 375px), capture `preview_screenshot` of every community page in light AND dark theme. Save under `qa-reports/community-design-pass-screens/<page>/<theme>-<width>.jpg`. The dark theme is toggled by setting `document.documentElement.dataset.theme = 'dark'` via `preview_eval` before the screenshot.

Bundle the screenshot set as one commit: `chore(community): visual regression baseline (light + dark, 3 viewports)`. This is the reference; future changes can diff against these images.

### 2.4 — Dark mode token finalization

While capturing screenshots, audit for visible failures:
- Hashtag banner (`#EEF4FF` / `#D1E0FF` hard-coded) should remap to design tokens.
- Profile-hero level + verified pills.
- Comment row reaction button.
- Admin status badges.

Add new tokens to `dashboard-tokens.css` if any colour can't be expressed with the existing vars. Verify with `preview_inspect` that computed contrast ratios meet WCAG AA (4.5:1 for body text, 3:1 for large text).

---

## Workstream 3 — Production readiness

### 3.1 — Performance audit on new profile endpoints

The five endpoints from `WS3.1` (`/api/community/profile/:id/posts|comments|media|activity` + `/api/community/profile/me/analytics`) each issue 3-5 queries. Worth verifying no N+1 surprise:

1. Add `tracing::instrument` and `tracing::debug!` for query timing in `list_user_posts`, `list_user_comments`, `list_user_media`, `list_user_activity`, `get_my_analytics`.
2. Run with `RUST_LOG=debug` and hit each endpoint against a user with **50+** posts, comments, and media. Confirm each endpoint completes in < 200ms.
3. Check `EXPLAIN ANALYZE` for the `list_user_activity` UNION query — that's the heaviest. Add indexes if missing:
   - `idx_posts_user_created (user_id, created_at DESC)` — verify exists
   - `idx_comments_user_created (user_id, created_at DESC)` — verify exists
   - `idx_xp_ledger_user_created (user_id, created_at DESC)` — verify exists

   If any is missing, add migration `036_profile_perf_indexes.sql`.

4. Commit: `perf(community): profile endpoint query timing + missing indexes`.

### 3.2 — Sentry error tracking on new code paths

Verify Sentry captures errors from:
- `submit_verification_request` (length validation, pending-already)
- `notify_user` when prefs blob has malformed JSON
- `record_profile_view` when `community_db` is offline

These should already work via the global `AppError` → Sentry path, but the new `record_profile_view` swallows errors with `.ok()` — that's fine for a fire-and-forget telemetry call but worth adding a `tracing::warn!` so DB outages still surface in logs.

Commit: `chore(community): trace profile_view DB failures`.

### 3.3 — Update docs/CODEBASE_ARCHITECTURE.md

Add a new "Community module" section documenting:

- All page routes (`/community`, `/community/me`, `/community/u/:id`, `/community/post/:id`, `/community/hashtag/:tag`, `/community/badge/:id`).
- The API surface, grouped by feature area (posts, comments, profile, circles, AMAs, challenges, leaderboard, notifications, moderation, search, hashtags, bookmarks, polls, badges, verification).
- The migration ledger (001-035, with a one-line description of each).
- The JS module map (community-feed, community-circles, community-amas, community-challenges, community-announcements, community-autocomplete, community-ban-appeal, community-mobile-tabs, community-profile).
- The CSS file map (community.css, community-profile.css, leaderboard.css interaction).
- Pre-existing quirks future maintainers need: MiniJinja lacks `tojson`, templates don't auto-reload, leaderboard.css hides tabs at 768px.

Commit: `docs(community): full module reference in CODEBASE_ARCHITECTURE.md`.

### 3.4 — Final regression sweep

Run the full pytest suite (not just community):
```
python3 -m pytest tests/ -o "addopts=-v --tb=line" --no-header
```

Triage any new community-related regression. Pre-existing non-community failures are out of scope.

### 3.5 — Deploy

Per the user's preference (`memory/feedback_deploy.md`): deploy = `git push origin main`. CI/CD handles the rest. No manual gcloud commands. Confirm with the user before pushing.

After push, monitor:
- CI build status via `gh run watch`
- Production logs for migration application
- Sentry for any new exception classes from the community module
- Production `/community/me` smoke-test from a logged-in account

Commit final note in a `qa-reports/community-deploy-log.md` capturing the commit SHA pushed, CI run URL, and any post-deploy observations.

---

## Out of scope (do NOT do)

- Don't add direct messages — separate epic.
- Don't add WebSocket / SSE for realtime updates.
- Don't migrate community-feed.js's modal `style="display:none"` toggles — they're functional state and break if removed.
- Don't touch `auth/routes.rs:page_profile` (the legacy `/profile` route) — separate concern.
- Don't rewrite tests in async pytest unless test 1.1 specifically requires it.

---

## Definition of done

- **Tests**: `python3 -m pytest tests/e2e/test_community.py tests/e2e/test_community_profile.py tests/test_community*.py` returns 0 failures, 0 errors with the default `addopts=-x` configuration.
- **Backend**: `cargo check` clean, `cargo test community::` passes, fresh DB migrations apply without error.
- **Design**: every admin community page passes the inline-style grep:
  ```
  grep -cE 'style="' frontend/platform/admin/community/*.html | grep -v ":0"
  ```
  (empty result = clean).
- **A11y**: 0 critical or serious axe-core violations on `/community`, `/community/me`, `/community/u/:id`, `/community/post/:id`, `/community/hashtag/:tag`, plus the four most-trafficked admin community pages.
- **Visual regression**: `qa-reports/community-design-pass-screens/` populated with the full matrix (light + dark, 3 viewports, 9 pages).
- **Docs**: `CODEBASE_ARCHITECTURE.md` has a "Community" section under 600 lines covering routes, APIs, migrations, JS map, CSS map, quirks.
- **Performance**: every profile endpoint < 200ms p99 against a seeded user with 50+ rows.
- **Deploy**: latest commit pushed to `main`, CI green, production smoke-test verified by the user.

---

## Output expectations

For each task:
- One-line plan before starting.
- Make the change, show the diff or summarize.
- Verify per protocol (cargo check + restart + pytest where applicable).
- Commit conventional: `fix(community): …` / `feat(community): …` / `refactor(admin): …` / `perf(community): …` / `docs(community): …` / `test(community): …` / `chore(community): …`.
- Update TodoWrite as you complete each numbered item.

Stop and ask only if:
- A migration would drop or rewrite existing data.
- The user must confirm before `git push origin main`.
- You discover a security issue worth flagging before the final merge.

Otherwise: keep going through the 15 tasks. Total expected session length: 1.5-3 hours of agent time depending on how many a11y violations surface.
