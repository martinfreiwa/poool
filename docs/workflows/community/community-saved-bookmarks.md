# Community Saved Posts and Bookmarks

Purpose: Verify saved-post discovery, bookmark status, bookmark toggles, pagination, empty states, and stale/deleted post handling.

Prerequisites:
- User is logged in.
- Seed data includes at least two bookmarkable posts, one owned by the user and one by another user.
- Use disposable posts for delete/stale-state checks.

Pages and endpoints covered:
- `/community?tab=saved`
- `/api/community/bookmarks`
- `/api/community/posts/:id/bookmark`
- `/api/community/posts/:id/bookmark/status`

Steps:
1. Open `/community?tab=saved` from the profile edit shortcut and by direct URL.
2. Verify the Saved tab/panel is visible, active, and not competing with the Feed panel.
3. Bookmark a normal feed post and verify the action state updates once.
4. Open `/api/community/posts/:id/bookmark/status` through the UI state or a test request and verify it matches the visible bookmark state.
5. Reload `/community?tab=saved` and verify the bookmarked post appears.
6. Unbookmark the same post and verify it disappears from Saved after refresh or list update.
7. Bookmark multiple posts and verify ordering, pagination/load-more behavior, and post detail links.
8. Delete or hide a disposable bookmarked post from another tab/admin path and verify Saved handles the stale item cleanly.
9. Verify logged-out users cannot list bookmarks or mutate bookmark state.

Expected Result:
- Bookmark state is consistent across feed cards, post detail, status API, and Saved.
- Saved empty state is explicit and has a safe route back to Community.
- Deleted, hidden, locked, or inaccessible posts do not leak sensitive content.
- Duplicate bookmark clicks do not create duplicate Saved rows.

Edge Cases:
- Saved list with zero posts.
- Bookmarking an already bookmarked post.
- Unbookmarking a post that is not bookmarked.
- Bookmark status for missing, hidden, deleted, or unauthorized post.
- Direct `?tab=saved` access after session expiry.
- Rapid double-click or network retry during toggle.
