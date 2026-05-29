# Community Hashtags and Badges

Purpose: Verify Community discovery/detail pages for hashtags and badges, including empty states, deep links, and safety around generated links.

Prerequisites:
- User is logged in.
- Seed data includes posts with hashtags and at least one Community badge when possible.

Pages and endpoints covered:
- `/community/hashtag/:tag`
- `/community/badge/:id`
- `/api/community/hashtags/:tag`
- `/api/community/hashtags/trending`
- `/api/community/hashtags/suggest`
- `/api/community/badges/:id`

Steps:
1. Open a hashtag from a feed post.
2. Verify the hashtag page banner, post count, feed list, and back navigation.
3. Open a direct hashtag URL with the same tag and verify the same state.
4. Search or type a hashtag in the composer and verify suggestions.
5. Verify trending hashtags load where surfaced in the Community UI.
6. Open a badge detail page from a profile or badge list.
7. Verify badge icon, name, code, description, holder count, recent holders, and holder profile links.
8. Open a badge page directly and verify protected access behavior.
9. Verify empty states for hashtags with no posts and badges with no holders.

Expected Result:
- Hashtag and badge detail pages load directly and from links.
- Hashtag filters return the expected posts and do not break feed pagination.
- Badge holder lists link to the correct profiles.
- Empty and not-found states are explicit.
- Tags, badge names, and holder names render safely as text.

Edge Cases:
- Unknown hashtag.
- Unknown badge ID.
- Hashtag with uppercase, spaces, emoji, URL encoding, punctuation, and very long text.
- Hashtag from hidden/deleted posts.
- Badge with no holders.
- Badge granted then revoked.
- Duplicate badge code in admin seed data if codes are displayed.
- Logged-out direct access.
- Malicious hashtag or badge display text containing HTML/script.
