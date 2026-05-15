# Community Profile Page + Design Pass + Loose-End Fixes — Implementation Prompt

> Drop this into a fresh Claude Code session. Self-contained — no prior context required.

---

## Mission

Three workstreams, executed in order:

1. **Loose-end cleanup** — close the carry-overs from the previous community refactor (nested comment replies, follower pagination, verification auto-flag for future posts, hashtag pagination, mobile-tab UX, notify_user adoption sweep, end-to-end smoke verification with a logged-in session).
2. **Full design pass** — every community page (community.html + 9 partials + 14 admin pages) gets a consistent visual language: zero inline styles, design-system classes only, mobile-responsive, dark-mode parity, empty states / skeletons / toasts unified.
3. **New User Community Profile page** — a dedicated `/community/me` (own) and `/community/u/:user_id` (others) full-page experience showing bio, avatar, stats, posts, comments, followers, following, circle, badges, analytics, media gallery, activity timeline. The page is the user's home in the community: every detail about them, plus a settings tab on the own variant.

Each workstream commits independently. Don't bundle.

---

## Codebase architecture you must know

**Stack**
- Backend: Rust + Axum, SQLx + Postgres, mounted in `backend/src/community/` and `backend/src/main.rs`.
- Frontend: Jinja2 (MiniJinja) + HTMX + Alpine.js + vanilla JS. No build step.
- Design system: `ds-*` class prefix. CSS lives in `frontend/platform/static/css/` (community.css, ds-*.css, dashboard-tokens.css). Theme vars: `--btn-primary-bg`, `--card-bg`, `--card-border-color`, `--page-title-color`, `--body-color`, `--text-secondary`, `--ds-surface-hover`, `--warning-bg`, `--warning-fg`, `--danger-bg`, `--danger-fg`.
- Migrations: `database/community/NNN_xxx.sql`. Next available number is **033**.
- Tests: pytest, E2E at `tests/e2e/test_community*.py`, static contracts at `tests/test_community_*.py`.

**Key existing files**

Backend:
- `backend/src/community/routes.rs` — ~7000 lines. Handler fns + `pub fn router() -> Router<AppState>` near line 2300.
- `backend/src/community/service.rs`, `models.rs`, `notifications.rs`, `xp.rs`, `audit.rs`, `validation.rs`, `user_bridge.rs`, `circles.rs`, `amas.rs`, `challenges.rs`, `reviews.rs`, `moderation.rs`, `background.rs`.
- `backend/src/main.rs` — page routes (~line 985 onwards): `/community`, `/community/post/:id`, `/community/hashtag/:tag`, plus HTMX partial handlers `community_feed_list_htmx`, `community_announcements_list_htmx`, `community_htmx_partial`.
- `backend/src/templates.rs` — MiniJinja env. Templates DO NOT auto-reload — restart server to pick up template changes.
- `backend/src/storage/routes.rs` — `upload_avatar` (`/api/upload/avatar`) and `upload_post_image` (`/api/upload/post-image`).
- `backend/src/settings/mod.rs` — existing `/settings` page (1476 lines, account-level only; not community-specific).
- `backend/src/admin/mod.rs` — admin page routes.
- `backend/src/auth/routes.rs:2268` — `page_profile` for `/profile` (legacy).

Frontend templates (Jinja, in `frontend/platform/`):
- `community.html` — main community SPA shell with tabs + modals.
- `community-hashtag.html` — SSR hashtag landing page.
- `partials/community_feed.html` — feed tab (composer + sidebar).
- `partials/community_circle.html` — My Circle tab.
- `partials/community_challenges.html` — Challenges tab.
- `partials/community_announcements.html` + `community_announcements_list.html`.
- `partials/community_ama.html` — AMA tab.
- `partials/community_post_card.html` — single post card (server-rendered).
- `partials/community_post_list.html` — feed list wrapper + infinite-scroll sentinel.
- `partials/community_disabled.html` — banned-user fallback (13 lines).

Frontend JS (in `frontend/platform/static/js/`):
- `community-feed.js` (~1700 lines) — wrapped in `window.initCommunityFeed = function() {...}`, invoked on DOMContentLoaded. Holds: feed, comments, post edit/delete, follow, polls, bookmarks, hashtags, lightbox, drag-drop, profile modal, relationship list, edit-profile modal, autocomplete delegation.
- `community-circles.js` (~735 lines) — My Circle tab.
- `community-amas.js` (~321 lines) — AMA tab.
- `community-challenges.js` — Challenges tab with action deeplinks.
- `community-announcements.js` — no-op shim now.
- `community-autocomplete.js` — composer autocomplete.
- `community-ban-appeal.js` — banner + modal handler.
- `toast.js` — exposes both `window.showPooolToast(title, message, type)` and `window.showToast(message, type)` (alias).
- `user-data.js` — publishes `window.__POOOL_USER` asynchronously after `/api/me`.
- `admin-community-*.js` — admin page handlers.

Frontend admin (in `frontend/platform/admin/community/`): index, amas, announcements, appeals, badges, challenges, circles, circle-detail, comments, leaderboard, post-detail, posts, reports, user-detail, users.

**Conventions you MUST follow** (carryover from previous workstream)
1. **Never inline `style="..."` in HTML or in JS-built DOM.** All styles via `ds-*` classes or new classes added to `community.css`. This is the single biggest cleanup target in workstream 2.
2. **Never use `textContent` for content that should render markup** — but for user input use `textContent`, never `innerHTML`. For server-sanitized HTML use `| safe` in Jinja or `innerHTML` after explicit sanitization.
3. **Every write endpoint:** `require_csrf_header` → auth → `check_user_not_banned` → validate → DB → audit (admin only) → response.
4. **Every new fetch call from JS:** `credentials: 'same-origin'`, `csrfHeaders()` on writes, surface errors via `window.showToast(message, 'error')` — never `alert()`.
5. **New JS modules:** wrap in IIFE (or inside the `initCommunityFeed` wrapper if it lives in the feed scope). Register in `community.html` extra_js or a dedicated page's `extra_js` list.
6. **New DB columns / tables:** add a migration in `database/community/NNN_xxx.sql`. Use `IF NOT EXISTS`. Never modify existing migrations.
7. **Backend handlers:** add to `router()` next to logically related routes; do not append at end.
8. **Tests:** every new endpoint gets an entry in `tests/e2e/test_community.py`; every new tab/page gets coverage in `tests/test_community_tab_contract_static.py`.

**Verification protocol** (after each commit)
1. `cargo check --manifest-path backend/Cargo.toml` — must be clean.
2. Use `mcp__Claude_Preview__preview_start` (server name `backend` from `.claude/launch.json`) — restart after Rust changes since the running binary is stale.
3. Wait 30 seconds for compile + boot.
4. Log in: hit `/auth/login`, use the test admin from earlier sessions (`e2e-admin-*`), or use the dev login flow described in `start_local.sh`.
5. Navigate to the affected page. Use `preview_eval`, `preview_console_logs`, `preview_screenshot`, `preview_snapshot`. **Never ask the user to test manually.**
6. Run `pytest tests/e2e/test_community.py -x` and admin siblings if applicable.
7. Each new endpoint: `curl` it once with the dev cookie to confirm 2xx and JSON shape.

**Stop-and-ask triggers** (don't drift, surface and wait):
- Migration would drop or rewrite data.
- Route signature change that would break an existing client.
- Architectural contradiction (two sources of truth for the same field).

Everything else: keep going. Update TodoWrite per task.

---

## Workstream 1 — Loose-end cleanup

Commit each task separately. Conventional commits, scope `community`.

### 1.1 — Nested comment replies

Migration `database/community/033_comment_replies.sql`:
```sql
ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS parent_comment_id UUID NULL REFERENCES comments(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_comment_id) WHERE parent_comment_id IS NOT NULL;
```

Backend (`backend/src/community/routes.rs`):
- Extend `CreateCommentReq` with `parent_comment_id: Option<Uuid>`.
- In `create_comment`: if parent_comment_id is set, validate it exists, belongs to the same post, and the parent itself isn't a reply (cap depth at 1 to keep UI manageable).
- `Comment` struct in `models.rs`: add `pub parent_comment_id: Option<Uuid>`.
- `get_comments`: return parent_comment_id on each row.
- Update the existing comment serialization at routes.rs:846 to include `parent_comment_id`.

Frontend (`community-feed.js` `loadComments`):
- Group comments client-side: top-level comments (no parent), then nested replies indented under parent.
- Add a "Reply" button next to Edit on every comment row. Clicking opens an inline textarea below that comment. Submit posts with `parent_comment_id` set to the parent.
- Render replies with a left border + reduced avatar size + `community-comment-row--reply` class.

CSS (`community.css`):
```css
.community-comment-row--reply {
    margin-left: 32px;
    padding-left: 12px;
    border-left: 2px solid var(--card-border-color, #EAECF0);
}
.community-comment-row--reply .community-comment-row__avatar {
    width: 28px;
    height: 28px;
}
```

Notification: when a reply is posted, call `notify_user(parent_author_id, "post_comment", ...)` so the parent author gets a reply notification. Skip if parent author == replier.

### 1.2 — Follower / Following pagination

The current `list_relationship` helper caps at 50 rows with no offset. Add pagination:
- Accept `?page=N` (default 1, size 30).
- Return `{ users: [...], has_more: bool }`.
- Frontend `openRelationshipList` adds a "Load more" button at the bottom that fires when has_more is true. Track current page in modal state.

### 1.3 — Verification auto-flag on future posts

Right now `admin_review_verification_request` (`approve` branch) bulk-updates existing `posts.verified_owner` but new posts don't inherit. Two options — pick (1):

**(1) Denormalize on community_profiles** — add `is_verified_owner BOOLEAN NOT NULL DEFAULT FALSE` to `community_profiles` in migration `034_verified_owner_profile.sql`. Approval sets it to true. `create_user_post` reads it to set `verified_owner` on the new row. Cleaner long-term.

**(2) Compute at read** — drop the `verified_owner` column on posts entirely, and JOIN `community_profiles.is_verified_owner` at every read. More queries but no denormalization drift.

Go with (1). Migration also backfills existing rows: `UPDATE community_profiles cp SET is_verified_owner = TRUE WHERE EXISTS (SELECT 1 FROM verification_requests WHERE user_id = cp.user_id AND status = 'approved')`.

### 1.4 — Hashtag page pagination

`get_hashtag_feed_data` already accepts `page` but the `/community/hashtag/:tag` SSR page never passes it. Add `?page=N` query handling in `page_community_hashtag`, and emit a "Load more" button or HTMX sentinel at the bottom of `community-hashtag.html` similar to `community_post_list.html`.

### 1.5 — Mobile tab UX

The community topbar tabs overflow on narrow viewports (< 640px). Add to `community.css`:
```css
@media (max-width: 640px) {
    .community-topbar-tabs {
        overflow-x: auto;
        scroll-snap-type: x mandatory;
        -webkit-overflow-scrolling: touch;
        white-space: nowrap;
        gap: 4px;
    }
    .community-tab-btn {
        scroll-snap-align: start;
        flex-shrink: 0;
        padding: 8px 12px;
        font-size: 13px;
    }
}
```

Also: bind `touchstart`/`touchend` on `.lb-container` to detect a swipe gesture (>50px horizontal, <30px vertical) and switch to the next/prev visible tab in `community-tab-btn` order. Use Pointer Events when available. New file `static/js/community-mobile-tabs.js`, load via `extra_js`. Skip the swipe handler when an input is focused.

### 1.6 — notify_user adoption sweep

Find every direct `INSERT INTO notifications` and replace with `notify_user(...)` so notification preferences (task 31) are honoured. Likely culprits:
- `xp.rs` (level-up notifications)
- `badges` background worker
- `background.rs` (digest emails — separate, skip)

Grep: `grep -rn "INSERT INTO notifications" backend/src/community/`. Replace each with the helper.

### 1.7 — Smoke verification with login

Use the test admin user from `start_local.sh` (search `start_local.sh` for `E2E_ADMIN_EMAIL` or similar). Use `preview_eval` to POST to `/auth/login` with the test creds in the same browser session, then re-run the end-to-end smoke from the previous workstream:
- `window.openEditPostModal`, `window.deleteOwnPost`, `window.openRelationshipList`, `window.uploadProfileAvatar`, `window.uploadVerifyProof`, `window.handleComposerDrop`, `window.submitVerifyRequest` should all be functions.
- Click the kebab menu on a post → Edit → save → confirm `reload-feed` fires and the post text updates.
- Click followers count → modal opens → list renders → click Follow on a row → button flips to Unfollow.
- Drop an image onto the composer → upload completes → preview appears.

Document the test admin credentials in the commit message so they're reproducible.

---

## Workstream 2 — Design pass

Scope: every community-related page + admin page, every JS-built DOM, every partial. Goal: a single coherent visual system, zero inline styles, mobile + dark mode parity.

Each step below is a commit. Run after each: `cargo check`, restart server, visual diff on `preview_screenshot`.

### 2.1 — Inline-style inventory & elimination

Grep for inline styles:
```
grep -rnE "style=\"" frontend/platform/community.html frontend/platform/partials/community_*.html frontend/platform/community-hashtag.html
grep -rnE "style\.cssText|\.style\." frontend/platform/static/js/community-*.js | head -200
```

For each match:
- HTML attributes → move to a class in `community.css`. Class name follows the `community-<block>__<element>--<modifier>` BEM pattern already in use.
- JS `el.style.X = ...` → toggle a class instead. Build a small `applyState(el, stateName)` helper if a pattern repeats more than twice.
- The shimmer/skeleton block in `renderSkeleton` (~50 lines of inline styles) — extract into `static/css/community-skeleton.css` and load via `extra_css`.

Expected diff: 30-50 inline `style=""` removals plus ~100 lines new CSS.

### 2.2 — Consistent empty / loading / error states

Today different tabs use different empty-state visual languages (icon + title + desc on some, plain text on others). Standardize on a single block:
```html
<div class="community-state community-state--empty">
    <div class="community-state__icon" aria-hidden="true">...</div>
    <h3 class="community-state__title">...</h3>
    <p class="community-state__desc">...</p>
    <button class="ds-btn ds-btn--primary community-state__cta">...</button>
</div>
```
With `--loading`, `--error`, `--empty` modifiers. Replace every ad-hoc empty/error renderer in:
- `community-feed.js` (skeleton, error states for hashtag/saved/comments)
- `community-circles.js` (no-circle state)
- `community-amas.js` (empty AMA)
- `community-challenges.js` (empty challenge list, error)
- `community-announcements_list.html` (no announcements)

### 2.3 — Dark mode parity

Audit every community-specific colour. Hard-coded hex codes that should use vars:
- `#101828`, `#181D27`, `#344054`, `#667085`, `#98A2B3`, `#FFFFFF`, `#F2F4F7`, `#FEF3F2`, `#FFFAEB`, `#EAECF0`, `#D0D5DD`, `#0000FF`, `#03FF88`, `#B42318`, `#039855`, `#067647`, `#F79009`, `#D92D20`.

Map each to a CSS variable from `dashboard-tokens.css` or add new ones if missing. Each variable should have a `[data-theme="dark"]` override defined in one place (likely `dashboard-tokens.css`). Sample audit pass: for one feed-post card, walk every property and verify the dark-mode variant doesn't break contrast (WCAG AA 4.5:1 for body text).

`toast.js` injects raw hex into inline style; refactor toast styles into `static/css/toast.css` and load via head.html so dark mode works there too.

### 2.4 — Topbar + sidebar coherence

The community topbar tabs and the global sidebar nav use different active-state styles. Pick the sidebar's pattern (rounded background pill + icon tint) for both. Update `community-topbar-tabs` + tab CSS accordingly.

Mobile: collapse the right-side topbar utilities (search input + notifications bell) into a single ds-icon-button that opens a popover with both. Add ARIA labels.

### 2.5 — Post card density + hierarchy

Today the post card mixes 14px and 16px body text, has inconsistent vertical rhythm (margins range 8-20px), and the engagement bar wraps awkwardly on narrow viewports. Define a single `.feed-post` density token block at the top of community.css:
```css
.feed-post {
    --feed-post-pad: 16px;
    --feed-post-pad-x-mobile: 12px;
    --feed-post-gap: 12px;
    --feed-post-radius: 14px;
}
```
Apply consistently. On `@media (max-width: 640px)` use `--feed-post-pad-x-mobile` for left/right padding, collapse the engagement bar to a vertical stack if it would wrap.

### 2.6 — Admin page polish

Admin community pages (`frontend/platform/admin/community/*.html`) contain heavy inline styling, especially in the JS-built table rows. Sweep them the same way:
- Replace `style="font-weight: 500; color: #181D27;"` with `<strong class="admin-table__cell-primary">`.
- The action button colour-tones (warning/danger) — move to `.admin-btn--warning`, `.admin-btn--danger` modifiers in `admin.css`.

Keep the visual behavior identical; this is purely a class refactor.

### 2.7 — Accessibility audit

Manual + automated pass per page:
- Every interactive element has a label (aria-label or visible text).
- Every modal has `role="dialog"`, `aria-modal="true"`, `aria-labelledby`, and an explicit Close button.
- Tab order is logical (no tabindex > 0 except 0/-1).
- Run `axe-core` programmatically via `preview_eval` (CDN-load it temporarily) and capture violations per page.
- Colour contrast: pick 3 sample text/background pairs per page and verify with `preview_inspect` + a contrast calculator.

Output a `qa-reports/community-design-pass-axe.md` listing each violation by page + severity + the commit that fixed it.

### 2.8 — Final visual snapshot pass

Take `preview_screenshot` of each page at 1440px, 768px (tablet), 375px (mobile). Save under `qa-reports/community-design-pass-screens/`. Compare with the same set on `main` before the design pass started; bundle the comparison set as a single commit `chore(community): visual regression baseline post-design-pass`.

---

## Workstream 3 — User Community Profile page

The big one. A separate full-page experience that aggregates everything a user can know about themselves (own page) or another user (public page).

### 3.1 — Backend: per-user data endpoints

All in `backend/src/community/routes.rs`. Public unless noted "(own only)".

```
GET  /api/community/profile/:id/posts?page=N        — paginated PostDisplay array, viewer's reactions populated
GET  /api/community/profile/:id/comments?page=N     — paginated comments with their parent post snippet
GET  /api/community/profile/:id/media?page=N        — every image_url from this user's posts, newest first
GET  /api/community/profile/:id/badges              — list of badges earned (already exists; verify)
GET  /api/community/profile/:id/activity?page=N     — chronological feed of XP events, badges earned, posts, comments
GET  /api/community/profile/me/analytics            — own only. Aggregates last 30 days: posts_count, reactions_received, comments_received, profile_views, top_post_id, top_post_reactions, xp_earned, login_streak, days_active
GET  /api/community/profile/me/bookmarks?page=N     — own only (already exists, paginate)
GET  /api/community/profile/me/blocked-users        — own only
GET  /api/community/profile/me/muted-users          — own only
```

Reuse helpers:
- `map_to_post_display` for posts.
- `user_bridge::get_users_info_batch` for author names.
- `service::get_badges_batch` for badges per row.

Migration `database/community/035_profile_views.sql`:
```sql
CREATE TABLE IF NOT EXISTS profile_views (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_user_id UUID NOT NULL,
    viewer_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_profile_views_profile ON profile_views(profile_user_id, created_at DESC);
```
- Add `record_profile_view(profile_user_id, viewer_user_id)` helper.
- `page_community_user_profile` (added below) calls it on every page render.
- Analytics endpoint counts `SELECT count(*) FROM profile_views WHERE profile_user_id = $1 AND created_at > NOW() - INTERVAL '30 days'`.

### 3.2 — Backend: page routes

`backend/src/main.rs`:
```rust
.route("/community/me", get(page_community_my_profile))
.route("/community/u/:user_id", get(page_community_user_profile))
```

`page_community_my_profile` redirects to `/community/u/<current_user_id>?own=1` or renders the same template with `is_own = true`. Simpler: render the same template both paths, with context `is_own: bool`.

Template name: `frontend/platform/community-profile.html`. It's a full page (sidebar + topbar + content), not a partial.

Context to pass:
```rust
#[derive(serde::Serialize)]
struct Context {
    is_own: bool,
    target_user_id: Uuid,
    profile: serde_json::Value,    // bio, display_name, avatar_url, badges, follower_count, following_count, post_count, level, level_name, xp_total, login_streak, is_verified_owner, is_shadowbanned (own only)
    circle: Option<serde_json::Value>,  // current circle name + member_count + level
    base_url: String,
    asset_version: ...,
}
```

`profile` is fetched via `service::get_user_profile` + the bridge user info + community_profiles row (`xp_total`, `level`, `level_name`, `login_streak`, `is_verified_owner`).

### 3.3 — Frontend: page template

`frontend/platform/community-profile.html` — full-page template. Structure:

```
[sidebar]
[topbar with breadcrumb: Community › <Name>]
[hero header card]
  - left: avatar (128px) with online indicator dot
  - center: display name + verified pill + level badge + bio
  - right: follow button OR (if own) "Edit profile" + "Settings" buttons
[stat strip: XP | Level | Followers | Following | Posts | Login streak | Joined]
[tab navigation: Posts | Comments | Followers | Following | Badges | Media | Circle | Activity | (Analytics if own) | (Settings if own)]
[tab content panel]
```

Tabs are URL-routable (`?tab=posts` etc) using the same `switchCommunityTab` pattern from `community.html`.

Each tab content is lazy-loaded on activation via fetch to the relevant `/api/community/profile/:id/...` endpoint. Infinite scroll via HTMX sentinels where applicable.

Sample tab markups (in `community-profile.html`):

**Posts tab** — reuses the existing `feed-post` card. Loaded via fetch (not HTMX since these are user-specific and don't share the global feed list endpoint). Build cards client-side using a new shared helper `window.renderPostCard(p)` extracted from `community-feed.js`'s `buildPostCard` but with all the bells (reactions, comments, bookmark, kebab if own).

**Comments tab** — chronological list. Each row shows:
```
[comment text snippet]
on "Post title or first 80 chars" · timeAgo · X reactions
```
Click row → navigate to `/community/post/<post_id>#comment-<id>`.

**Followers / Following tabs** — paginated list with row = avatar + name + Follow/Unfollow button. Same row markup as the existing relationship-list modal — extract `buildRelationshipRow` into a shared helper.

**Badges tab** — grid of badge cards (4 per row desktop, 2 on mobile). Each card shows icon, name, description, earned_at. Add a tooltip on hover showing the criteria.

**Media tab** — masonry grid of all images this user has posted. Click → lightbox (reuse existing `community-lightbox` element). Each tile has a small caption showing the post date and a link to the source post.

**Circle tab** — if the user is in a circle:
```
[Circle name + emoji]
[circle stats: member count, level, total XP]
[circle leaderboard top 5]
[link "View full circle ›" → /community?tab=circle (own) or N/A (other)]
```
If not in any circle, show empty state.

**Activity tab** — vertical timeline. Pulls from `/profile/:id/activity` which aggregates:
- Posts: "Posted '<snippet>' (X reactions)"
- Comments: "Commented on '<post>'"
- Reactions received: "Their post got X reactions"
- Badges earned
- XP gains (`xp_ledger` rows)
- Level-ups
Each row has an icon, action label, timestamp, and (when relevant) a link to the source.

**Analytics tab (own only)** — three cards:
1. **Last 30 days**: posts_count, reactions_received, comments_received, xp_earned, profile_views. Each as big number + small label.
2. **Engagement chart**: SVG line chart (sparkline-style, hand-rolled, no library) showing reactions/day over 30 days. Use the existing chart approach if there is one; otherwise build a small `<svg viewBox>` with polyline.
3. **Top post card**: the user's most-reacted post in the last 30 days, rendered as a full feed post card.

**Settings tab (own only)** — three subsections:
- **Profile**: bio textarea, avatar upload (reuse `uploadProfileAvatar`), submit → PUT /api/community/profile.
- **Privacy & moderation**: notification preferences toggles (port from notification tab), blocked-users list with unblock button, muted-users list with unmute button.
- **Account actions**: verification request panel (already exists in edit-profile modal; promote into this tab and remove from the modal), "Download my data" link (out of scope today, just a placeholder), "Delete community profile" (out of scope, placeholder with confirm dialog leading to /support).

### 3.4 — Frontend: JS

New module `frontend/platform/static/js/community-profile.js` (IIFE). Responsibilities:
- Tab routing (URL ?tab= → activate panel).
- Lazy-load each tab on first activation (cache the response in a per-tab map).
- Build helpers: `renderPostCard`, `renderCommentRow`, `renderRelationshipRow`, `renderBadgeCard`, `renderMediaTile`, `renderActivityRow`, `renderAnalyticsCard`.
- Reuse `window.openCommunityModal`, `window.openUserProfile`, `window.toggleFollow`, `window.toggleBlock`, `window.toggleMute` from community-feed.js.
- The page loads `community-feed.js` first so all those globals are available.

Extra_js for the profile page:
```
extra_js=['htmx-init', 'profile-dropdown', 'community-feed', 'community-autocomplete', 'community-profile']
```

### 3.5 — Frontend: CSS

New file `static/css/community-profile.css`:
- Hero header card layout (CSS grid, 3-column on desktop, single-column on mobile)
- Stat strip with 7 cells, scrollable horizontally on mobile
- Tab navigation pills, sticky on scroll
- Per-tab content layouts (posts list, comments list, followers grid, badges grid, media masonry, activity timeline, analytics cards)
- Loading skeleton per tab type
- Dark mode parity

Add to community.html / community-profile.html: `extra_css=['community', 'community-profile', 'leaderboard']`.

### 3.6 — Cross-linking

- Every existing reference to "user profile modal" (in community-feed.js `openUserProfile`) should ALSO surface a "View full profile" link inside the modal that goes to `/community/u/:user_id`.
- The sidebar profile card in `community_feed.html` (lines ~133-152) — add a "View my profile" link below the Edit button → `/community/me`.
- Every `@mention` click currently opens the profile modal (via `openProfileByHandle`). Add an option "View full profile" inside the modal too.

### 3.7 — Tests

E2E in `tests/e2e/test_community_profile.py`:
- Load `/community/me` as authed user → expect 200, page title contains display name.
- Each tab activates without console errors.
- API responses: `/api/community/profile/:id/posts` returns array of PostDisplay with `author_id`, `content`, `reaction_count`.
- Analytics tab only loads for the own user (anonymous → 401 on `/profile/me/analytics`).

Static contract test in `tests/test_community_profile_static.py`:
- All required Jinja blocks present (hero, stat strip, tab nav, every tab panel).
- All required script tags loaded.
- Required ARIA labels present.

### 3.8 — Migration to-do list

- `033_comment_replies.sql` (workstream 1.1)
- `034_verified_owner_profile.sql` (workstream 1.3)
- `035_profile_views.sql` (workstream 3.1)

Apply automatically on backend boot (`start_local.sh` does this).

---

## Out of scope

- Direct messages
- WebSocket / SSE realtime updates
- Full SPA rewrite — stay on HTMX + Alpine
- Stripe / billing settings (those live on existing /settings)
- Email notification delivery (we just store the preferences)
- Achievement / badge earning rules (those are runtime decisions in `background.rs`)
- Verifying badges grant the actual badge (out — verification request flow only)

---

## Output expectations

For each workstream + each task:
- One-line intent statement before starting.
- Make the change.
- Show the diff or summarize.
- Verify per protocol.
- Conventional commit. Scope `community`. Example: `feat(community-profile): hero header + stat strip`.

Open with a brief plan for the whole prompt, then start workstream 1 task 1. Use TodoWrite to track 18 tasks (7 in WS1, 8 in WS2, 8 in WS3).

Stop and ask if:
- A migration would drop existing data.
- The user's existing `/profile` route at `auth/routes.rs:2268` should be merged with the new `/community/u/:user_id` or kept separate.
- The Activity tab's data shape requires denormalizing across multiple tables (you may suggest a materialized view).

Otherwise keep going.

---

## Definition of done

- All 18 tasks committed.
- `cargo check` clean.
- Backend smoke (curl + preview_eval) returns 2xx on every new endpoint.
- `/community/me` and `/community/u/<id>` load without console errors, both as own and as anonymous viewer (anonymous views public sections only).
- Visual regression screenshots captured at 3 viewport widths and stored under `qa-reports/`.
- pytest suite passes: `pytest tests/e2e/test_community*.py -x`.
- Commit log reads as a clean narrative — one logical concern per commit, with workstream prefix in the body if helpful.
