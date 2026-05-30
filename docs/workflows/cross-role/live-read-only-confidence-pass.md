# Live Read-Only Confidence Pass

Purpose: Verify live/staging routes and cross-role read-only surfaces without mutating production data.

Roles: Public Visitor, Investor, Developer, Admin.

Primary pages:
- Public: `/`, `/id/`, `/blog`, `/blog/:slug`, `/p/:slug`, legal pages
- Investor: `/marketplace`, `/wallet`, `/portfolio`, `/transactions`, `/leaderboard`, `/settings`, `/support`, `/community`
- Developer: `/developer/dashboard`, `/developer/assets`, `/developer/submissions`, `/developer/operations`, `/developer/affiliate-team`
- Admin: `/admin/`, `/admin/users`, `/admin/kyc`, `/admin/orders`, `/admin/deposits`, `/admin/marketplace/`, `/admin/community/`, `/admin/audit-logs`

Prerequisites:
- User explicitly approves the target environment.
- Test accounts are approved for live/staging read-only use.
- No form submit, upload, approval, rejection, financial, destructive, or outbound-message action will be executed.
- Optional unauthenticated smoke harness: run `LIVE_READ_ONLY_BASE_URL=https://platform.poool.app python3 -m pytest tests/test_live_read_only_smoke.py -q` to verify public pages and unauthenticated protection boundaries with GET requests only. The harness is skipped unless `LIVE_READ_ONLY_BASE_URL` is set.

Steps:
1. Record environment, date, commit/deploy version if visible, and account roles.
2. As Public Visitor, open public pages and verify HTTP `200`, no obvious broken layout, and no unintended auth leakage.
   - Legal/public pages must not render the 404 template under HTTP `200`.
   - Root legal routes and published aliases such as `/privacy`, `/legal/terms`, `/legal/privacy`, and `/legal/imprint` must resolve because email and blog links can point to them long after deployment.
3. Verify protected pages redirect unauthenticated users to login.
4. As Investor, log in and open investor pages listed above.
5. Verify data loads, navigation works, and controls are visible, but do not submit mutations.
6. As Developer, open developer pages and verify dashboard/assets/submissions/operations/affiliate-team read-only data loads.
7. As Admin, open admin pages and verify queues/tables/filters render.
8. Confirm admin mutating controls are present only for authorized admin roles, but do not click final submit/approve/reject/send buttons.
9. Check browser console and network for visible `4xx/5xx` failures on page load.
10. Log out and verify session ends.

Expected Result:
- Live/staging surfaces are reachable and role-gated.
- Read-only data renders for each role.
- No production data is mutated.

## Execution Note - 2026-05-30 Live Read-only

Target environments: `https://www.poool.app` and `https://platform.poool.app`.

Scope actually executed:
- Public GET checks for landing, auth, blog, and legal pages.
- Unauthenticated protected-route boundary checks.
- Authenticated read-only GET checks with an approved admin-capable account.
- Investor, Developer, Community, and Admin canonical page loads.

No upload, save, approve, reject, delete, financial action, outbound message, resource mutation, settings mutation, or content mutation was executed.

Results:
- Canonical Investor pages loaded: `/marketplace`, `/commodities-marketplace`, `/marketplace-secondary`, `/my-trading`, `/wallet`, `/portfolio`, `/rewards`, `/cart`, `/leaderboard`, `/settings`, `/support`.
- Canonical Developer pages loaded: `/developer/dashboard`, `/developer/assets`, `/developer/operations`, `/developer/support`.
- Canonical Community pages loaded: `/community`, `/community?tab=circle`, `/community?tab=ama`, `/community?tab=challenges`, `/community/circles` plus `circle`, `ama`, `resources`, `members`, `join-requests`, `banned`, and `danger-zone` tabs.
- Canonical Admin pages loaded: `/admin/`, `/admin/users`, `/admin/kyc`, `/admin/orders`, `/admin/deposits`, `/admin/marketplace/`, `/admin/marketplace/orders`, `/admin/reports`, `/admin/settings`, `/admin/system`, `/admin/storage`, `/admin/audit-logs`, and the main Admin Community pages.

Findings:
- Live still renders 404 templates under HTTP `200` for `/imprint`, `/aml-kyc-policy`, `/gdpr-data-request`, and `/legal/terms`; local routes/tests already cover these, so this is deployment parity until the live revision is updated.
- `/admin/dashboard` renders a 404 template on live; canonical admin dashboard is `/admin/`. Local code now redirects `/admin/dashboard` to `/admin/`.
- `/admin/community/moderation` and `/admin/community/audit-log` render 404 templates on live; canonical surfaces are `/admin/community/reports`, `/admin/community/comments`, user-detail moderation, and `/api/admin/community/audit-log`. Local code now redirects both aliases to canonical read-only surfaces.
- `/community/circle/poool-founder` returned `404`, while `/community/circle/poool-founder/settings` returned `200`; the live circle experience is currently reachable through the `/community/circles` tabbed shell/API-backed surfaces.

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
