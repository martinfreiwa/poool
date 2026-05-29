# Admin Community Badges and Grants

Purpose: Verify badge CRUD, user badge grants/revocations, badge detail page effects, duplicate handling, and auditability.

Prerequisites:
- Admin account has Community badge permissions.
- Disposable badge and test user are available.

Pages and endpoints covered:
- `/admin/community/badges.html`
- `/community/badge/:id`
- `/community/u/:user_id`
- `/api/admin/community/badges`
- `/api/admin/community/badges/:id`
- `/api/admin/community/users/:id/badge`
- `/api/admin/community/users/:id/badge/:badge_id`
- `/api/community/badges/:id`

Steps:
1. Open admin badges page and verify existing badges load.
2. Create a disposable badge with valid name, code, icon, and description.
3. Edit the badge and verify changes persist.
4. Open the public badge detail page and verify public fields and holder list.
5. Grant the badge to a test user.
6. Verify the user's profile shows the badge and the badge holder list includes the user.
7. Revoke the badge and verify profile/detail pages update.
8. Try duplicate code, missing fields, oversized text, invalid icon, and malformed color/icon values.
9. Verify normal users cannot call admin badge endpoints.

Expected Result:
- Badge create/update/grant/revoke operations persist and are reflected on public pages.
- Duplicate grants are idempotent or rejected clearly.
- Badge text/icon fields render safely.
- Admin actions are audited.

Edge Cases:
- Grant badge to nonexistent, banned, or deleted user.
- Revoke badge user does not have.
- Delete/update badge with existing holders if supported.
- Badge with zero holders.
- Very long badge name/code.
