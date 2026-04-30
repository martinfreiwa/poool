# Admin User Details Fix Pass

Date: 2026-04-28
Page: `/admin/user-details`
Tracker ID: `admin.user-details`
Status: fixed, needs authenticated E2E recheck

## Fixed Issues

- PAGE-ISSUE-0288: Sensitive user detail reads and mutations now enforce granular permissions. User list/detail reads require `users.view` and `pii.view` and write `admin.pii_access` audit entries. Profile/status/session/password-reset mutations require `users.edit`; balance adjustments require `treasury.write`; role changes require `roles.edit` and super-admin checks.
- PAGE-ISSUE-0289: Profile updates, optional tier override, and audit writes now run inside one database transaction.

## Evidence

- Backend: `backend/src/admin/users.rs`
- Static coverage: `tests/admin/test_admin_user_details_static.py`

## Verification

- `python3 -m pytest tests/admin/test_admin_user_details_static.py -q`
- `node --check frontend/platform/static/js/admin-user-details.js`
- `rustfmt --edition 2021 --check backend/src/admin/users.rs`

## Remaining Recheck

Authenticated admin browser/API E2E still needs to cover user detail load, PII access auditing, profile update, balance adjustment, status change, session revocation, force password reset, role denial/allowance, and visible UI error states with real admin-role fixtures.
