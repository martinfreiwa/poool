# Community Report To Admin Moderation And Appeal

Purpose: Verify a user-generated community action flows through report, admin moderation, user restriction, appeal, and recovery.

Roles: Reporting User, Reported User, Admin.

Primary pages:
- `/community`
- `/community/post/:id`
- `/community/u/:user_id`
- `/settings/notifications/community`
- `/admin/community/reports`
- `/admin/community/posts`
- `/admin/community/comments`
- `/admin/community/users`
- `/admin/community/user-detail`
- `/admin/community/appeals`
- `/admin/audit-logs`

Prerequisites:
- Community is enabled.
- Two user accounts exist.
- Admin can moderate community content.

Steps:
1. As Reported User, create a disposable community post.
2. As Reporting User, open `/community/post/:id`, report the post with a reason, and verify confirmation.
3. As Admin, open `/admin/community/reports` and find the report.
4. Inspect reported content, author, reporter, reason, history, and links to post/user detail.
5. Apply a moderation action such as hide post, warn, mute, or ban using disposable fixtures.
6. Verify `/admin/community/posts`, `/admin/community/users`, and `/admin/community/user-detail` show the updated state.
7. As Reported User, reload `/community` and verify the restriction state is clear.
8. Submit a ban appeal if banned.
9. As Admin, open `/admin/community/appeals`, review, approve or reject with notes.
10. Verify user-facing recovery/rejection state.
11. Verify notification preferences and moderation-history visibility for affected users.
12. Verify audit logs include report, moderation action, appeal, and appeal decision.

Expected Result:
- Reports are actionable by admins.
- Moderation affects user/community surfaces consistently.
- Appeals can restore or confirm restrictions with audit trail.

Backend/API surfaces:
- See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for the complete route-to-workflow mapping.
- Mutating APIs used by this workflow must be verified for authorization, validation, idempotency where applicable, and reload/readback across roles.


Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | The workflow reaches the final cross-role state and every role sees the expected state after reload. |
| Authorization boundary | Non-owner or wrong-role direct page/API access returns login redirect, `401`, or explicit `403`. |
| Validation failure | Missing, malformed, stale, duplicate, or out-of-state input is rejected without partial persistence. |
| Audit/readback | Mutating action writes expected audit/DB rows and the next role sees the update only after reload. |
| Cleanup | Disposable `Workflow Test` data can be reverted, archived, or intentionally retained with a note. |

Negative Cases:
- Unauthorized direct page/API access by each non-owner role.
- Missing required fields, invalid state transition, duplicate submit, stale record, and network failure.
- For uploads, invalid file type, oversize file, missing storage object, and inaccessible download link.
- For financial flows, malformed amount, insufficient balance, duplicate approval/settlement, and cents mismatch.


Audit / DB / Financial Checks:
- Verify every admin action writes an audit row with actor, action, target, timestamp, prior/new state where available, and redacted sensitive values.
- Verify all monetary values are stored as integer cents (`BIGINT`/`i64`) and any percentage values use basis points where modeled that way.
- Verify multi-table financial writes are transactional and duplicate submits are idempotent or explicitly blocked.
- Verify uploaded files record MIME type, size, owner/target, storage key/link, access scope, success state, and failed-upload cleanup.
- After every cross-role transition, reload the new role's page and verify the visible state from the database/API, not stale client state.


Cleanup:
- Revert or archive every `Workflow Test` record created by this workflow using approved local cleanup paths.
- Remove temporary uploaded files and downloaded artifacts where policy allows.
- Restore account, wallet, role, feature-flag, notification, and content settings changed during the run.
- Retain audit logs unless the environment is fully disposable and the cleanup runbook explicitly truncates them.
