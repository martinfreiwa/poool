# Community Section — Fix & Implementation Prompt

> Drop this into a fresh Claude Code session. Self-contained — no prior context required.

---

## Mission

Fix all known bugs and implement all missing features in the POOOL community section, front-end and back-end. Work through phases in order. After each phase, run the verification protocol before moving on.

This is a **large, multi-session task**. Use TodoWrite to track phases. Do **not** batch unrelated changes into single commits — one logical concern per commit.

---

## Codebase architecture (read this before touching anything)

**Stack**
- Backend: Rust + Axum, SQLx + Postgres, mounted in `backend/src/community/`
- Frontend: Jinja2 (MiniJinja server-side) + HTMX + Alpine.js + vanilla JS (no React/Vue)
- Design system: `ds-*` class prefix (`ds-card`, `ds-btn`, `ds-input`, `ds-modal`, `ds-form-group`, `ds-text-*`, `ds-badge`, `ds-progress`, `ds-spinner`, `ds-helper-text`, `ds-sr-only`)
- DB migrations: `database/community/NNN_name.sql` (sequential — next available number is 027)
- Tests: Python pytest, E2E in `tests/e2e/test_community*.py`, static contracts in `tests/test_community_*.py`

**Key files / patterns**

Backend:
- Module entry: `backend/src/community/mod.rs` (re-exports `routes::router`)
- All HTTP handlers + router mount: `backend/src/community/routes.rs` (~5500 lines — handler fns, then `pub fn router() -> Router<AppState>` at line ~2144)
- Service / business logic: `backend/src/community/service.rs`
- Per-feature modules: `circles.rs`, `amas.rs`, `challenges.rs`, `reviews.rs`, `moderation.rs`, `notifications.rs`, `xp.rs`, `audit.rs`, `validation.rs`, `user_bridge.rs`, `background.rs`
- Error type: `AppError::{BadRequest, Forbidden, Unauthorized, NotFound, Internal, Database}` — import from `crate::error::AppError`
- DB pool: `let c_pool = get_community_pool(&state)?;`
- CSRF guard: `require_csrf_header(&headers, &jar)?;` — call **first** on any state-changing handler (POST/PUT/DELETE)
- Ban guard: `check_user_not_banned(&c_pool, user.id).await?;` — call before any write
- Audit log: existing pattern using `crate::community::audit::log_action(...)` for admin actions
- Upload routes already exist: `POST /api/upload/post-image`, `POST /api/upload/avatar` in `backend/src/storage/routes.rs` — reuse them, don't reimplement
- HTMX partial routes: `backend/src/main.rs:2682` `community_htmx_partial` and the explicit `/community/partials/feed/list`, `/community/partials/announcements/list`

Frontend:
- Main page: `frontend/platform/community.html` (tabs, modals, search panel, notifications panel, saved panel, scripts)
- Partials (Jinja, served via HTMX swap into `#community-content-area`): `frontend/platform/partials/community_feed.html`, `community_ama.html`, `community_circle.html`, `community_challenges.html`, `community_announcements.html`, `community_announcements_list.html`, `community_post_card.html`, `community_post_list.html`
- JS bundles (loaded via `extra_js=[...]` in head.html include):
  - `static/js/community-feed.js` (1519 lines) — feed, comments, profile modal, follow, post composer, polls, bookmarks, hashtags, trending sidebar, AMA sidebar
  - `static/js/community-circles.js` (735 lines)
  - `static/js/community-amas.js` (321 lines)
  - `static/js/community-challenges.js` (74 lines — stub)
  - `static/js/community-announcements.js` (21 lines — stub)
- CSS: `static/css/community.css` (uses `ds-*` tokens and theme CSS vars `--btn-primary-bg`, `--card-bg`, `--card-border-color`, `--page-title-color`, `--body-color`)
- Helpers already available:
  - `window.openCommunityModal(modalId, triggerEl)` and `window.closeCommunityModal(modalId)` — includes focus trap, ESC handler, focus restore (defined in `community.html` ~line 521)
  - `window.switchCommunityTab(btn)` and `window.activateCommunityTab(tabName)`
  - `csrfHeaders()` local helper inside community-feed.js IIFE — reads `csrf_token` cookie, sets `X-CSRF-Token` header
  - `window.showPooolToast(title, message, type)` — defined in `static/js/toast.js`. **Code currently calls non-existent `window.showToast` — alias it.**
  - `__POOOL_USER` populated by `user-data.js` (name, etc.)

Topbar tabs (HTMX-driven) defined in `frontend/platform/components/investor-topbar.html` lines 28-37. Tab names: `feed`, `announcements`, `circle`, `challenges`, `ama`, plus client-tabs `saved`, `search`, `notifications`.

**Conventions you must follow**
1. Never inline `style="..."` in new HTML or JS-built DOM. Use existing `ds-*` classes or add classes to `community.css`.
2. Render user content with `textContent` in JS, or `| safe` in templates **only after server-side sanitization** (see existing `rendered_content` pattern in routes.rs).
3. Every write endpoint: CSRF check → auth → ban check → validate → DB → audit (admin only) → response.
4. Every new frontend fetch: `credentials: 'same-origin'`, use `csrfHeaders()` on writes, surface errors via `showToast` (not `alert()`).
5. New JS files: load via `extra_js=[...]` in the page's Jinja include header. Wrap in IIFE.
6. New DB columns / tables: add a migration in `database/community/NNN_xxx.sql`. Include both up SQL and any indexes. **Never modify an existing migration.**
7. Backend handlers: add to the `router()` block in `routes.rs` next to logically related routes (don't append at end).
8. Tests: every new endpoint gets a row in `tests/e2e/test_community.py` (or sibling). Every new tab UI element gets coverage in `tests/test_community_tab_contract_static.py`.

---

## Phase 1 — Trivial fixes (no schema, no new endpoints)

Do these first. Each is a one-to-few-line change. Commit each separately.

1. **Global `showToast` alias.** In `static/js/toast.js`, after the existing `showPooolToast` definition, add:
   ```js
   window.showToast = function(message, type) {
     return window.showPooolToast(null, message, type || 'info');
   };
   ```
   Verify all current callers (`community-feed.js:1496`, `community-amas.js:57`, `community_post_card.html:96`) work.

2. **Search "Follow" button no-op.** `frontend/platform/community.html:170` — replace `@click.stop=""` with `@click.stop="window.toggleFollow(u.user_id, u.is_following, $event.target)"`. Make sure the search backend (`/api/community/search`) returns `is_following` on each user — check `search_community` handler in `routes.rs`, add field if missing.

3. **Sidebar "Edit Profile" bypasses modal.** `frontend/platform/partials/community_feed.html:151` — change `onclick="window.location.href='/settings'"` to `onclick="window.openProfileEditModal()"`. The modal + handler already exist in `community.html:646` and `community-feed.js:1452`.

4. **Delete orphan onboarding code.** In `community-feed.js`, remove the `checkOnboarding()` function and the `closeOnboardingModal` declaration — their target DOM (`onboarding-modal`, `ob-bio`, `ob-post`) no longer exists. Also remove the `localStorage` flag read/write for `poool_community_onboarding_dismissed`. Keep the `updateMyProfileCard` call — extract its fetch into a new `loadMyProfile()` and call it on init.

5. **Delete orphan `create-post-modal` reference.** `community-feed.js:576-577` — remove the `const modal = document.getElementById('create-post-modal'); if (modal) modal.style.display = 'none';` lines (composer is inline now).

6. **Hardcoded placeholder name/bio in feed sidebar.** `frontend/platform/partials/community_feed.html:137-138` — replace `Martin F.` and `Sustainable investor • Long-term holder 🌱` with empty `<span>` (let JS populate) or with `{{ user.display_name }}` / `{{ user.bio | default('') }}` if the partial receives a user context. Check `community_htmx_partial` in `main.rs` to confirm what context it passes.

7. **`{{ base_url }}` undefined in post card share.** `frontend/platform/partials/community_post_card.html:96` — verify `base_url` is set in the MiniJinja context for `/community/partials/feed/list` handler in `main.rs`. If not, add it (read from `state.config.base_url` or `state.public_origin`).

8. **Render all post images, not just `[0]`.** `frontend/platform/partials/community_post_card.html:51-54` — replace the single-image markup with a loop. For 1 image, full-width. For 2-4 images, CSS grid. Add `.feed-post-image-grid` styles to `community.css`. Add lightbox click handler (Phase 2 may upgrade this — for now, `onclick="window.open(this.src, '_blank')"` is acceptable).

9. **Image + link preview should both render.** Same file:51-67 — change `{% elif p.link_preview %}` to `{% endif %}{% if p.link_preview %}` so both blocks can show.

10. **Mention click should go to profile, not search.** `community-feed.js:895` — change `window.location.href = '/community?search=' + encodeURIComponent(mention)` to open the user profile modal: resolve `mention` (display_name or username) to `user_id` via new endpoint `GET /api/community/users/by-handle/:handle` (Phase 2) — for now, fall back to `openUserProfile` if you can pass the user_id from the server-rendered mention tag (modify `routes.rs:310` `mention-tag` span to include `data-user-id`).

11. **AMA "View full AMA" inline click.** `frontend/platform/partials/community_feed.html:210` — replace inline DOM query with `onclick="window.activateCommunityTab('ama')"`.

---

## Phase 2 — Missing wiring (backend exists, frontend doesn't use it)

12. **Post edit (own post).** Backend route already exists: `PUT /api/community/posts/:id` → `update_user_post` (`routes.rs:924`). Add to frontend:
    - Add an "Edit" button to `community_post_card.html`, visible only when `p.author_id == current_user_id`. Need to pass `current_user_id` into the template context.
    - Build an edit modal in `community.html` (mirror the structure of `report-post-modal`). Textarea pre-filled with current content.
    - Add `window.openEditPostModal(postId, currentContent)` and `window.submitEditPost()` to `community-feed.js`.
    - On success, dispatch `reload-feed` event.

13. **Post delete (own post).** Backend: `DELETE /api/community/posts/:id` → `delete_user_post` (`routes.rs:956`).
    - Add a kebab menu next to the report button on `community_post_card.html` (own-author only). Use existing `ds-btn--icon ds-btn--ghost` style.
    - Menu items: Edit, Delete.
    - Delete uses native `confirm('Delete this post?')` then `fetch` with `method: 'DELETE'`, CSRF header.
    - On success, fade out the post element (existing animation in `celebration-effects.js` if applicable, otherwise CSS class).

14. **Ban appeal user submission form.** Backend: `POST /api/community/appeals` → `submit_ban_appeal` (`routes.rs:4844`). Validates 10-2000 char body.
    - Create new page `frontend/platform/community-banned.html` (Jinja). When a banned user hits `/community`, the backend should redirect them here.
    - Check `community_disabled.html` partial — currently a 13-line stub at `partials/community_disabled.html`. Repurpose or extend it.
    - Add textarea + submit button + character counter. Use `ds-input`, `ds-form-group`.
    - On success, show toast + redirect to `/`.
    - Add new JS file `static/js/community-banned.js`, load via `extra_js`.

15. **Saved tab — wire bookmark/report/reactions/comments.** Currently `loadSavedPosts` uses `buildPostCard` which is a stripped-down builder. Replace with fetch to the HTMX partial endpoint:
    - Add backend route `GET /community/partials/feed/list?source=bookmarks` that filters the existing list query by the current user's bookmarks. Or simpler: have the saved tab use `hx-get="/community/partials/feed/list?source=bookmarks"` like the main feed does.
    - Remove client-side `buildPostCard` from saved tab path. (Keep it only for hashtag filter — see Phase 3 task 24 to replace that too.)

16. **AMA admin actions (status, answer, feature questions).** Backend routes exist but admin page inline script doesn't call them:
    - `POST /api/admin/community/amas/:id/status` — change status (upcoming → live → completed)
    - `POST /api/admin/community/amas/:id/questions/:qid/answer` — expert posts answer
    - `POST /api/admin/community/amas/:id/questions/:qid/feature` — pin a question
    - Find these route mounts in `routes.rs` and verify the exact paths. Update `frontend/platform/admin/community/amas.html` inline script to call them with proper CSRF.

17. **Admin user audit log viewer.** Backend: `GET /api/admin/community/users/:id/audit-log` exists.
    - Add a tab/section to `frontend/platform/admin/community/user-detail.html`.
    - Render as table: timestamp, admin user, action, target, details (JSON).
    - Inline script fetches on tab activation.

18. **Admin ban appeals review UI.** Backend: `GET /api/admin/community/appeals`, `POST /api/admin/community/appeals/:id/review`.
    - Create new page `frontend/platform/admin/community/appeals.html` modeled on `reports.html`.
    - Create new JS `static/js/admin-community-appeals.js` modeled on `admin-community-reports.js` (list, filters, approve/deny action with reason).
    - Add link to admin community sidebar (`static/js/sidebar-community.js` or wherever).

---

## Phase 3 — New features (need backend + frontend + migration + tests)

For each: write the migration first, then backend handler + router entry, then frontend, then test. Run `cargo check` after backend changes, run server + verify partial in browser after frontend changes.

19. **Profile picture upload in edit-profile modal.**
    - Migration `database/community/027_profile_avatar.sql`: add `avatar_url TEXT` to `community_profiles` if not already there. (Check — `update_oauth_profile` in `auth/service.rs` may already store avatar at user level. Decide: store at community level or reuse user-level avatar.)
    - Reuse `POST /api/upload/avatar` from `storage/routes.rs`. Add `PUT /api/community/profile` payload field `avatar_url` (already exists per `update_profile` handler — verify it accepts the field, extend if not).
    - Edit profile modal (`community.html:646`): add `<input type="file">` + preview, upload on file pick, then save with bio.
    - Update `loadMyProfile`, `openUserProfile`, post avatar rendering to use the new field if present.

20. **Followers / Following list view.**
    - Backend: add `GET /api/community/profile/:id/followers` and `GET /api/community/profile/:id/following`. Each returns paginated `[{user_id, display_name, avatar_url, is_following}]`.
    - Frontend: in `user-profile-modal`, make the followers/following stat values clickable. Open a secondary modal with a scrollable list. Each row has a Follow/Unfollow button using existing `toggleFollow`.

21. **Block / mute another user (self-service).**
    - Migration `database/community/028_user_blocks.sql`: `CREATE TABLE user_blocks (blocker_id UUID, blocked_id UUID, reason TEXT, created_at TIMESTAMPTZ, PRIMARY KEY(blocker_id, blocked_id))`. Add indexes on both columns.
    - Backend: `POST /api/community/users/:id/block`, `DELETE /api/community/users/:id/block`, `GET /api/community/blocks` (own list).
    - Filter feed/comments query to exclude posts from blocked users (`WHERE NOT EXISTS (SELECT 1 FROM user_blocks WHERE blocker_id = $current_user AND blocked_id = p.user_id)`).
    - Frontend: in user-profile-modal action row, add Block button (with confirm). Block → close modal → reload feed.

22. **Challenges participation (join / submit / track).**
    - Backend: `POST /api/community/challenges/:id/join`, `POST /api/community/challenges/:id/submit` (payload: optional proof_url, optional content). Track progress via existing `challenges.rs` module if it supports user assignments (check first — comment in earlier audit said it does).
    - Frontend: rewrite `static/js/community-challenges.js` (currently 74-line read-only). Add Join/Submit/View Progress buttons to each card in `partials/community_challenges.html`. Build a submit modal.
    - Award XP on completion via existing `xp.rs::award_xp(...)`.

23. **Announcements detail page + working "View in Feed".**
    - Backend: `GET /community/announcement/:id` returns a full-page Jinja render of one announcement plus its comments.
    - Frontend: new template `frontend/platform/announcement-detail.html`. Update `community_announcements_list.html:67` "View in Feed →" button to link to that page (or to scroll to the specific post if announcements are posts with a category).
    - Update `static/js/community-announcements.js` to drop the tab-switch hack.

24. **Dedicated hashtag page.**
    - Backend: `GET /community/hashtag/:tag` server-renders a page with `community_post_list.html` partial filtered by hashtag.
    - Update `community-feed.js` `filterByHashtag` and the rendered mention/hashtag span (`routes.rs:304`) to link to this URL (HTMX boost or normal nav).
    - Keep the in-feed filter banner as a quick-filter option but add a "Open hashtag page" link.

25. **Global XP leaderboard.**
    - Backend: `GET /api/community/leaderboard?scope=global&period=week` returning top N users with XP, level, badges.
    - Frontend: new tab "Leaderboard" in topbar (`investor-topbar.html`), HTMX-loaded partial `community_leaderboard.html`. Rank rows, podium for top 3, period filter (week / month / all-time). Reuse `circle-leaderboard` CSS classes from `community.css` where possible.

26. **Comment edit / delete / nested replies.**
    - Migration `database/community/029_comment_replies.sql`: add `parent_comment_id UUID NULL REFERENCES comments(id)` to `comments` table. Add index.
    - Backend: `PUT /api/community/comments/:id` (own only), `DELETE /api/community/comments/:id` (own or admin), update `GET /api/community/posts/:id/comments` to return tree structure or flat list with `parent_comment_id` for client-side nesting.
    - Update `create_comment` handler in `routes.rs:610` to accept optional `parent_comment_id`.
    - Frontend (`community-feed.js` `loadComments`): render replies indented under parent. Reply button on each comment opens an inline reply textarea. Edit/Delete buttons on own comments (own-author check needs server-provided `is_own` flag or comparison to `__POOOL_USER.id`).

27. **Comment reactions.**
    - Migration `database/community/030_comment_reactions.sql`: `CREATE TABLE comment_reactions (comment_id UUID, user_id UUID, reaction_type TEXT, created_at TIMESTAMPTZ, PRIMARY KEY(comment_id, user_id, reaction_type))`. Index on `comment_id`.
    - Backend: `POST /api/community/comments/:id/reactions` (toggle). Update comment serialization to include `reaction_count` and `current_user_reacted`.
    - Frontend: add a small react button on each comment row matching the post reaction style. Reuse XP award logic from posts.

28. **Mention / hashtag autocomplete.**
    - Backend: `GET /api/community/mentions/suggest?q=foo` returns up to 10 users matching prefix. `GET /api/community/hashtags/suggest?q=foo` returns 10 hashtag matches.
    - Frontend: build a generic autocomplete attached to `#post-content-input`. Watch for `@` or `#` or `$`, fetch on debounce (250ms), show popover below cursor, arrow-key navigation, enter to select. No external library — vanilla JS. Add new module `static/js/community-autocomplete.js`, load before `community-feed.js`.

29. **Multi-type reactions on posts.**
    - Backend `toggle_reaction` (`routes.rs`) already accepts a type parameter — confirm. Currently UI hardcodes `'fire'`.
    - Frontend: replace the single fire button on `community_post_card.html` with a hover/long-press reaction picker showing fire/heart/clap/insightful/laugh emoji. Reuse Facebook-style hover pattern. Add CSS to `community.css`.

30. **Shadowban indicator + own moderation history.**
    - Frontend: when user's profile fetch returns `is_shadowbanned: true`, show a subtle banner at top of feed: "Your posts have limited visibility. See moderation history."
    - Add `GET /api/community/profile/me/moderation-log` backend endpoint returning warnings, mutes, ban history with timestamps. Show in profile edit modal in a collapsed section.

31. **Notification preferences.**
    - Migration `database/community/031_notification_preferences.sql`: `CREATE TABLE notification_preferences (user_id UUID PRIMARY KEY, prefs JSONB NOT NULL DEFAULT '{}')`.
    - Backend: `GET /api/community/notifications/preferences`, `PUT /api/community/notifications/preferences`. Wire into `notifications.rs` so notifications respect prefs.
    - Frontend: add settings panel in the notifications tab header — toggle per type (mention, follow, post_like, post_comment, announcement, reward).

32. **Verified-owner badge request flow.**
    - Migration: add `verification_requests` table.
    - Backend: `POST /api/community/profile/verify-request` with proof (asset ownership proof image upload + statement), admin review endpoint.
    - Frontend: "Request verification" button in edit profile modal. Admin page already partially supports this — extend admin user-detail if needed.
    - Out of scope details: leave the exact compliance flow for the admin to spec; expose hooks only.

---

## Phase 4 — UX polish

33. Image lightbox on post image click (modal with zoom).
34. Infinite scroll on feed (HTMX `hx-trigger="revealed"` on sentinel div + cursor pagination).
35. Ctrl+Enter to submit post / comment.
36. Drag-drop on post image upload.
37. Reaction picker keyboard accessibility.
38. Mobile swipe between tabs.

---

## Verification protocol (run after each phase)

1. `cargo check --manifest-path backend/Cargo.toml` — backend must compile clean.
2. `cargo test --manifest-path backend/Cargo.toml community::` — backend unit tests pass.
3. Start dev server: check `docs/CODEBASE_ARCHITECTURE.md` or `package.json`-like manifest for the right command.
4. Use the Preview tools (`preview_start`, `preview_snapshot`, `preview_console_logs`, `preview_network`) to verify the changed page in the browser. Never ask the user to test manually.
5. Run the related Python tests: `pytest tests/e2e/test_community.py -x` and any sibling admin tests touched.
6. Confirm no new `console.error` in `preview_console_logs`.
7. For each new endpoint, manually curl it once with the dev cookie to confirm 200/201 and shape of JSON response.

---

## Out of scope (do not do)

- Don't rebuild the entire community section as a SPA. Stay with HTMX + Alpine.
- Don't introduce new JS frameworks or build tools.
- Don't touch the worktrees in `.claude/worktrees/` (those are deleted).
- Don't modify existing migrations — only add new ones with incremented numbers.
- Don't add direct messages — separate epic, requires E2E encryption discussion.
- Don't add WebSocket / SSE — out of current scope.

---

## Output expectations

For each phase:
- Open with a one-line plan ("Phase 1 task 3: wiring sidebar Edit Profile button to existing modal").
- Make the change.
- Show the diff (or summarize if large).
- Verify per protocol.
- Commit: conventional commits format, scope `community`, e.g. `fix(community): wire sidebar edit profile button to existing modal`.

Stop and ask the user only if:
- A migration would drop or rewrite data.
- A backend route signature change would break an existing client.
- You discover an architectural contradiction (e.g. two sources of truth for the same field).

Otherwise: keep going through the list. Update TodoWrite as you complete each numbered item.
