# Community Trending and Discovery Widgets

Purpose: Verify trending posts, trending assets, trending hashtags, discover widgets, empty states, and link integrity across Community.

Prerequisites:
- User is logged in.
- Seed data includes enough posts, hashtags, and assets to produce non-empty trending results when possible.

Pages and endpoints covered:
- `/community?tab=feed`
- `/community/hashtag/:tag`
- `/api/community/trending`
- `/api/community/trending-assets`
- `/api/community/hashtags/trending`
- `/api/community/search`

Steps:
1. Open Community Feed and locate trending widgets.
2. Verify trending posts load with title/body excerpt, count signals, and links to `/community/post/:id`.
3. Verify trending assets load with asset identity and links to the correct marketplace/property route.
4. Verify trending hashtags load and link to `/community/hashtag/:tag`.
5. Click each widget type and verify destination content matches the clicked item.
6. Compare one widget result with `/api/community/search` or direct detail API to ensure IDs and labels align.
7. Verify empty trending responses show non-blocking empty states.
8. Verify hidden/deleted/private content does not appear in trending results for unauthorized users.

Expected Result:
- Trending widgets are supplemental and never block the primary feed.
- Links are correct, encoded, and safe.
- Trending output respects visibility, moderation, and privacy rules.

Edge Cases:
- No trending posts/assets/hashtags.
- Hashtag with special characters or URL encoding.
- Trending post deleted after widget renders.
- Trending asset unavailable or sold out.
- User is blocked/muted by an author in trending results.
