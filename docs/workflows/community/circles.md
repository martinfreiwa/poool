# Community Circles

Purpose: Verify My Circles, circle discovery, join/request flows, circle creation, circle settings, and destructive-action safeguards.

Prerequisites:
- User is logged in.
- Test data includes at least one public circle and one private circle when possible.
- Use only test circles for create/update/delete checks.

Pages and endpoints covered:
- `/community?tab=circle`
- `/community/circles`
- `/community/circle/:slug`
- `/community/circle/:slug/settings`
- `/api/community/circles/discover`
- `/api/community/circles/search`
- `/api/community/me/circles`
- `/api/community/circles/me`
- `/api/community/circles`
- `/api/community/circles/:id`
- `/api/community/circles/:id/join`
- `/api/community/circles/:id/request`
- `/api/community/circles/:id/requests`
- `/api/community/circles/requests/:req_id/approve`
- `/api/community/circles/requests/:req_id/decline`
- `/api/community/circles/requests/mine`
- `/api/community/circles/leave`
- `/api/community/circles/by-slug/:slug`
- `/api/community/invites`
- `/api/community/invites/:id/accept`
- `/api/community/invites/:id/decline`

Steps:
1. Open `/community?tab=circle`.
2. Verify My Circles content loads through `/community/partials/circle`.
3. Verify current circle status, member list, leaderboard/discovery rows, and empty states.
4. Verify pending invites and pending join requests sections show correctly when data exists.
5. Search for a known Circle and verify search results replace discovery without losing clear/reset controls.
6. Apply discovery filters: featured, trending, new, asset, private, public, official, holder-only, and KYC-gated.
7. Create a test circle with a unique name and description.
8. Verify the created circle appears in My Circles or discovery.
9. Open the created Circle detail page and verify feed/member privacy state.
10. Edit circle name, description, privacy, tags, rules, and disclaimers when the test account has owner/admin permissions.
11. Verify settings persist after reload.
12. Request to join a private circle using a non-owner test account if available.
13. Approve or decline the join request as owner/admin and verify requester state.
14. Join and leave a public circle if available.
15. Send, accept, and decline a Circle invite using test users if available.
16. Verify unauthorized users cannot edit/delete circles they do not own.
17. Verify delete controls are present only where allowed, and only delete test circles.

Expected Result:
- Circle lists and details load without unsafe HTML injection.
- Create/update/join/request actions return clear success or authorization states.
- Circle settings persist after reload.
- Destructive actions are gated and excluded unless explicitly running a delete test on test data.
- Search, discovery filters, invites, requests, join, and leave states stay synchronized across tab, detail page, and settings page.
