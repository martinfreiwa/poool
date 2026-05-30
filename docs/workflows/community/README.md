# Community Workflows

This folder stores repeatable workflows for the POOOL Community area.

Keep these workflows separate from account, settings, marketplace, and developer workflows so the Community test surface stays easy to scan.

Do not store passwords, API keys, session cookies, downloaded personal data, or other secrets in this folder.

## Community Surface Map

Primary page:
- `/community`

Topbar tabs:
- Feed: `/community?tab=feed`, partial `/community/partials/feed`
- Announcements: `/community?tab=announcements`, partial `/community/partials/announcements`
- My Circles: `/community?tab=circle`, partial `/community/partials/circle`
- Challenges: `/community?tab=challenges`, partial `/community/partials/challenges`
- Expert AMAs: `/community?tab=ama`, partial `/community/partials/ama`

Client-side panels:
- Search: `/community?tab=search`
- Messages: `/community?tab=dms`
- Notifications: `/community?tab=notifications`
- Saved posts: `/community?tab=saved`

Related pages:
- `/community/post/:id`
- `/community/u/:user_id`
- `/community/me`
- `/community/me/edit`
- `/community/circle/:slug`
- `/community/circles`
- `/community/circle/:slug/settings`
- `/community/badge/:id`
- `/community/hashtag/:tag`

Admin pages:
- `/admin/community/index.html`
- `/admin/community/posts.html`
- `/admin/community/post-detail.html`
- `/admin/community/comments.html`
- `/admin/community/reports.html`
- `/admin/community/users.html`
- `/admin/community/user-detail.html`
- `/admin/community/circles.html`
- `/admin/community/circle-detail.html`
- `/admin/community/challenges.html`
- `/admin/community/announcements.html`
- `/admin/community/amas.html`
- `/admin/community/badges.html`
- `/admin/community/leaderboard.html`
- `/admin/community/appeals.html`
- `/admin/community/settings.html`
- `/admin/community/verified-owner-requests.html`

## Workflow Index

- [Community Shell and Tab Navigation](./shell-and-tabs.md)
- [Community Route and Link Integrity](./community-route-and-link-integrity.md)
- [Community Feed](./feed.md)
- [Community Autocomplete and Composer Suggestions](./community-autocomplete-and-composer-suggestions.md)
- [Community Trending and Discovery Widgets](./community-trending-discovery.md)
- [Community Posts, Comments, Reactions, and Reports](./posts-comments-reactions-and-reports.md)
- [Community Comment Ownership and Moderation](./community-comment-ownership-and-moderation.md)
- [Community Post Q&A and Official Answers](./community-post-qa-and-official-answers.md)
- [Community Saved Posts and Bookmarks](./community-saved-bookmarks.md)
- [Community Blocks, Mutes, and Safety Controls](./community-blocks-mutes-and-safety.md)
- [Community Challenges](./challenges.md)
- [Community Announcements](./announcements.md)
- [Community Circles](./circles.md)
- [Community Circle Space, Settings, Resources, and Ops](./circle-space-settings-resources-ops.md)
- [Inner Circle Complete Operations](./inner-circle-complete-operations.md)
- [Community Circle Governance](./community-circle-governance.md)
- [Community Circle Engagement Sidebar](./community-circle-engagement-sidebar.md)
- [Community Circle Resources and Versioning](./community-circle-resources-versioning.md)
- [Community Expert AMAs](./expert-amas.md)
- [Community Search, Messages, and Notifications](./search-messages-notifications.md)
- [Community Notification Settings Page](./community-notification-settings-page.md)
- [Community Profiles, Member Directory, and Profile Edit](./profiles-member-directory-and-edit.md)
- [Community XP and Leaderboards](./community-xp-and-leaderboards.md)
- [Community Asset Reviews](./community-asset-reviews.md)
- [Community Hashtags and Badges](./hashtags-and-badges.md)
- [Community Ban Appeals](./community-ban-appeals.md)
- [Community Disabled State](./community-disabled-state.md)
- [Admin Community Operations](./admin-community-operations.md)
- [Admin Community Audit Log](./admin-community-audit-log.md)
- [Admin Community Settings](./admin-community-settings.md)
- [Admin Community Badges and Grants](./admin-community-badges-and-grants.md)
- [Admin Community Leaderboard and XP Awards](./admin-community-leaderboard-and-xp.md)
- [Community Edge-Case Matrix](./edge-case-matrix.md)
- [Community Workflow Coverage Audit](./coverage-audit.md)
- [Community Browser Workflow Run — 2026-05-29](./browser-run-2026-05-29.md)
- [Community Workflow Run - 2026-05-29](./community-workflow-run-2026-05-29.md)

## Recommended Pass Order

Run the Community workflows in this order when doing a full page/product pass:

1. `shell-and-tabs.md` — proves the shell, auth, tab routing, and HTMX/client-panel contract.
2. `community-route-and-link-integrity.md` — proves internal URLs, generated links, and tab query states resolve to canonical routes.
3. `feed.md` — proves the primary user surface and composer.
4. `community-autocomplete-and-composer-suggestions.md` — proves mention, hashtag, asset, and circle suggestion behavior.
5. `community-trending-discovery.md` — proves trending widgets and discovery links.
6. `posts-comments-reactions-and-reports.md` — proves post detail, engagement, reports, owner actions, and safety states.
7. `community-comment-ownership-and-moderation.md` — proves own-comment actions and admin comment moderation.
8. `community-post-qa-and-official-answers.md` — proves Q&A status and official answer flows.
9. `community-saved-bookmarks.md` — proves Saved posts and bookmark state.
10. `profiles-member-directory-and-edit.md` — proves identity, follow graph, own-profile edit, privacy, moderation history, and verified-owner request.
11. `community-blocks-mutes-and-safety.md` — proves self-service safety relationships.
12. `circles.md` — proves My Circles, discovery, create/join/request/invite flows.
13. `circle-space-settings-resources-ops.md` — proves Circle detail, settings, resources, reports, members, requests, bans, and ops alerts.
14. `inner-circle-complete-operations.md` — proves the full Inner Circle owner/admin workflow.
15. `community-circle-governance.md` — proves role, ownership, privacy, token-gate, kick, ban, and delete flows.
16. `community-circle-engagement-sidebar.md` — proves Circle announcements, events, resources, challenges, and onboarding modules.
17. `community-circle-resources-versioning.md` — proves resource metadata, lifecycle, access, versions, review, and restore flows.
18. `announcements.md`, `challenges.md`, `expert-amas.md` — proves secondary content modules.
19. `search-messages-notifications.md` — proves client-side panels and notification preferences.
20. `community-notification-settings-page.md` — proves the dedicated Community notification settings page.
21. `community-xp-and-leaderboards.md` — proves user XP, XP history, and user/circle leaderboards.
22. `community-asset-reviews.md` — proves asset-linked reviews and review upvotes.
23. `hashtags-and-badges.md` — proves linked discovery/detail pages.
24. `community-ban-appeals.md` — proves banned-user appeal submission and admin review.
25. `community-disabled-state.md` — proves disabled/unavailable Community behavior.
26. `admin-community-operations.md` — proves admin moderation and platform operations overview.
27. `admin-community-settings.md`, `admin-community-badges-and-grants.md`, `admin-community-leaderboard-and-xp.md`, and `admin-community-audit-log.md` — prove specialized admin subflows.
28. `edge-case-matrix.md` — applies cross-cutting auth, safety, upload, privacy, destructive-action, and a11y cases across the full surface.
29. `coverage-audit.md` — reruns the route/API/link inventory checks before changing Community workflows.
30. `browser-run-2026-05-29.md` — records the latest non-mutating local browser execution and observed defects.

## Standard Verification Rules

Apply these checks to every Community workflow:

1. Start from a logged-in test account on `http://localhost:8888`.
2. Verify the page does not show console errors after the tab or page finishes loading.
3. Verify unauthenticated access redirects to `/auth/login` for protected Community pages and partials.
4. Verify visible user-generated text is rendered as text, not executable HTML.
5. Verify keyboard focus remains usable after tab switches, modal opens, modal closes, HTMX swaps, and client-side panel switches.
6. Verify desktop and mobile layouts do not overlap, truncate important actions, or hide required controls.
7. Verify every state-changing action has an expected success, validation-error, authorization-error, and network-error behavior.
8. Verify direct deep links keep URL state, active state, and visible panel state aligned.
9. Verify owner/admin-only actions are hidden or blocked for ordinary users and return explicit authorization errors if called directly.
10. Verify destructive actions are only executed against disposable test data and leave an audit trail where expected.

## Coverage Checklist

| Surface | Workflow |
|---------|----------|
| Main shell, tabs, direct tab URLs, HTMX partials | Shell and Tab Navigation |
| Internal route/link integrity and generated URLs | Route and Link Integrity |
| Feed, composer, pagination, feed filters | Feed |
| Composer autocomplete and suggestions | Autocomplete and Composer Suggestions |
| Trending posts, assets, and hashtags | Trending and Discovery Widgets |
| Post detail, comments, reactions, polls, bookmarks, reports | Posts, Comments, Reactions, and Reports |
| Own-comment edit/delete/reactions and admin comment moderation | Comment Ownership and Moderation |
| Q&A status and official answers | Post Q&A and Official Answers |
| Saved posts and bookmark status | Saved Posts and Bookmarks |
| Profiles, member directory, follow graph, profile edit, verified-owner request | Profiles, Member Directory, and Profile Edit |
| Blocks, mutes, safety relationships | Blocks, Mutes, and Safety Controls |
| My Circles, discovery, create/join/request/invite/delete | Circles |
| Circle detail, settings, resources, roles, reports, ops alerts | Circle Space, Settings, Resources, and Ops |
| Full Inner Circle owner/admin settings and operations | Inner Circle Complete Operations |
| Circle roles, moderators, ownership, privacy, token gates, bans | Circle Governance |
| Circle sidebar modules and onboarding | Circle Engagement Sidebar |
| Circle resources, access, versions, review, restore | Circle Resources and Versioning |
| Announcements categories and partial list refresh | Announcements |
| Challenges, submissions, and voting | Challenges |
| Admin AMA creation, status lifecycle, questions, answers/comments, upvotes/likes, featured questions | Expert AMAs |
| Search, DMs, notification list/preferences | Search, Messages, and Notifications |
| Dedicated Community notification settings | Notification Settings Page |
| XP, XP history, global/circle leaderboards | XP and Leaderboards |
| Asset-linked circle/review flows | Asset Reviews |
| Hashtag and badge detail/discovery pages | Hashtags and Badges |
| Ban appeal submission and admin review | Ban Appeals |
| Disabled/unavailable Community mode | Disabled State |
| Admin moderation and Community configuration overview | Admin Community Operations |
| Admin audit log and CSV export | Admin Community Audit Log |
| Admin settings | Admin Community Settings |
| Admin badge CRUD and grants | Admin Community Badges and Grants |
| Admin leaderboard and XP awards | Admin Community Leaderboard and XP Awards |
| Cross-cutting unusual states | Edge-Case Matrix |
| Workflow route/API/link coverage validation | Workflow Coverage Audit |
| Browser execution evidence and latest observed defects | Browser Workflow Run 2026-05-29; Community Workflow Run 2026-05-29 |
