# Community Announcements

Purpose: Verify announcements loading, category filters, list refresh behavior, and announcement detail links.

Prerequisites:
- User is logged in.
- Seed data includes announcements in at least one category when possible.

Pages and endpoints covered:
- `/community?tab=announcements`
- `/community/partials/announcements`
- `/community/partials/announcements/list`
- `/api/admin/community/announcements`

Steps:
1. Open `/community?tab=announcements`.
2. Verify the Announcements tab is active and the list loads.
3. Click category filters: `All`, `New Commodities`, `Dividends`, `Platform Updates`, `Market News`, and `Farm Updates`.
4. Verify each category request updates only the announcements list area.
5. Open an announcement detail or linked post if available.
6. Return to Announcements and verify the previous layout is still stable.
7. Verify empty category behavior when no results exist.

Expected Result:
- Announcement categories load through `/community/partials/announcements/list`.
- Filtering does not reload the whole Community shell.
- Empty categories show a stable empty state.
- Links route to the expected announcement/post target.
