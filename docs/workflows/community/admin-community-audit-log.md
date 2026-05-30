# Admin Community Audit Log

Purpose: Verify admin Community audit log listing, filtering, detail integrity, CSV export, authorization, and sensitive-data handling.

Prerequisites:
- Admin account has Community audit permissions.
- Disposable Community mutations exist or can be created.

Pages and endpoints covered:
- `/admin/community/index.html`
- `/api/admin/community/audit-log`
- `/api/admin/community/audit-log.csv`

Steps:
1. Perform a disposable Community admin mutation, such as hide/unhide a test post.
2. Open the admin Community overview or audit log entry point.
3. Load audit log rows and verify actor, action, target type, target ID, timestamp, and summary.
4. Filter by actor, action, target type, status/date where available.
5. Verify pagination and sorting.
6. Export CSV and verify headers, row count, escaping, and no secrets/session data.
7. Verify normal users and read-only admins without export permission cannot access restricted audit data.
8. Verify audit rows are append-only from the UI perspective.

Expected Result:
- Every tested admin mutation creates an audit row.
- CSV export matches filtered data and is safe to open in spreadsheet tools.
- Audit log does not expose passwords, session tokens, private notes beyond intended admin scope, or raw PII unnecessarily.

Edge Cases:
- Zero audit rows.
- Many audit rows.
- Filter returns no rows.
- CSV values beginning with `=`, `+`, `-`, or `@`.
- Target deleted after audit row was created.
- Export during session expiry.
