# Admin Content Publish To Public And Investor Readback

Purpose: Verify admin-managed content becomes visible on public or authenticated pages and can be reverted.

Roles: Admin, Public Visitor, Investor.

Primary pages:
- `/admin/blog`
- `/admin/blog-editor`
- `/admin/blog-persona`
- `/admin/blog-strategy`
- `/admin/email-marketing`
- `/admin/notifications`
- `/blog`
- `/blog/:slug`
- `/blog/category/:slug`
- `/`
- `/notifications` if available through shell

Prerequisites:
- Admin can manage blog/content/notifications.
- Test image exists for blog asset upload.
- Outbound email/test send must use local/staging sink only.

Steps:
1. As Admin, open `/admin/blog`.
2. Create a draft article in `/admin/blog-editor` with workflow-prefixed title, category, excerpt, body, and image.
3. Save draft and verify it remains unpublished on `/blog`.
4. Publish the article.
5. As Public Visitor, open `/blog`, `/blog/:slug`, and `/blog/category/:slug`.
6. Verify title, author/category, image, body, footer/social/legal links, mobile menu, and sign-in CTA.
7. As Admin, archive/unpublish the article.
8. Verify public pages no longer show it or show the intended archived state.
9. As Admin, open `/admin/notifications` and create a disposable in-app notification targeted to a test investor.
10. As Investor, verify notification appears in shell/notification center if implemented.
11. As Admin, open `/admin/email-marketing`, create a draft/test campaign, and send only to local/staging sink.
12. Verify no real outbound email is sent in local workflow.
13. Verify audit logs for publish/archive/notification/email actions.

Expected Result:
- Admin content publishing is visible to public/investor surfaces only after publish.
- Archive/unpublish removes or clearly marks content.
- Notification/email actions are scoped and auditable.

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
