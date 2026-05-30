# Community Asset Reviews

Purpose: Verify asset-linked community circles, asset review creation/update/delete, review listing, upvotes, ownership/eligibility checks, and moderation visibility.

Prerequisites:
- User is logged in.
- Seed data includes at least one marketplace asset with a community circle.
- Use only disposable reviews for mutation checks.

Pages and endpoints covered:
- `/community/circle/:slug`
- `/api/community/assets/:id/circle`
- `/api/community/assets/:id/reviews`
- `/api/community/reviews/:review_id/upvote`

Steps:
1. Open an asset detail page or known asset context that links to Community.
2. Resolve the asset's circle with `/api/community/assets/:id/circle` through the UI flow.
3. Open the linked circle and verify asset identity, access state, and review entry points.
4. List asset reviews and verify author, rating/sentiment if present, body, timestamps, and upvote count.
5. Create or update a disposable review with valid content.
6. Reload and verify the review persists once.
7. Edit the review and verify the update replaces the previous user review rather than duplicating it.
8. Upvote another user's review and verify count/state changes once.
9. Remove the upvote and verify count/state recovers.
10. Delete the disposable review and verify list/detail behavior after deletion.
11. Verify users without required asset access see the expected restriction state if eligibility rules apply.

Expected Result:
- Asset-to-circle lookup is stable and links to the correct circle.
- Review CRUD respects ownership and eligibility rules.
- Upvotes are idempotent and cannot be duplicated through rapid clicks.
- Deleted or hidden reviews do not leak through list views.

Edge Cases:
- Asset has no circle.
- Asset ID is invalid, missing, or unauthorized.
- User tries to review the same asset twice.
- Review body empty, too long, or contains HTML/script.
- Upvote own review.
- Upvote missing/deleted review.
- Review by blocked, muted, banned, or shadowbanned user.
