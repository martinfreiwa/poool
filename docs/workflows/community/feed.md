# Community Feed

Purpose: Verify Community feed loading, filtering, post composer behavior, reactions, comments, pagination, and safety around user-generated content.

Prerequisites:
- User is logged in.
- Test account can create Community posts.
- Seed data includes at least one post, one comment, and one author different from the test account when possible.

Pages and endpoints covered:
- `/community?tab=feed`
- `/community/partials/feed`
- `/community/partials/feed/list`
- `/api/community/feed`
- `/api/community/posts`
- `/api/community/trending`
- `/api/community/trending-assets`
- `/api/community/mentions/suggest`
- `/api/community/hashtags/suggest`
- `/api/community/assets/suggest`

Steps:
1. Open `/community?tab=feed`.
2. Verify feed composer controls render: post input, post type selector, upload action, poll/tag controls if present, and submit action.
3. Verify feed filters render and can switch between all/following and fresh/hot modes.
4. Create a text-only test post with a unique marker.
5. Verify the new post appears after submit without a full page break.
6. Reload `/community?tab=feed` and verify the post persists.
7. Create a post with image upload if local storage is configured; verify upload status, preview, and persisted image display.
8. Create a poll post with 2-4 options and expiry; verify poll renders after reload.
9. Trigger `@` mention autocomplete and hashtag autocomplete; verify suggestions are escaped and selectable.
10. Add a comment to the test post.
11. React to the post, then undo or toggle the reaction if supported.
12. Open a post detail route if the card links to one and verify comments/reactions are still visible.
13. Scroll/load more feed items and verify pagination keeps current filters.
14. Try invalid composer input such as empty content and over-limit content.
15. Verify investment-related text triggers the required disclaimer state.
16. Verify banned/shadowbanned account banners if the test account state requires them.
17. Verify feed widgets for trending posts, trending assets, and trending hashtags if visible.
18. Switch between direct feed URL, hashtag-filtered feed, and circle-filtered feed if available.

Expected Result:
- Feed loads and filters update results correctly.
- Valid posts and comments persist after reload.
- Invalid input is blocked with a clear validation state.
- Reactions update counts without duplicate submissions.
- User-generated content is escaped and cannot inject HTML/script.
- Composer uploads, poll creation, autocomplete, and disclaimer states either work or fail with clear recoverable errors.

Edge Cases:
- Following feed with no followed users.
- Feed page after the last page of results.
- Feed request with invalid `page`, `feed_mode`, `sort_by`, `source`, `category`, or `circle_id`.
- Composer double-submit.
- Stale feed reload after deleting the post from another tab.
- Trending widgets return empty, malformed, or slow responses.
- Circle-filtered feed for a circle the user cannot access.
