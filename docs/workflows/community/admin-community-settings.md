# Admin Community Settings

Purpose: Verify admin-managed Community settings, disabled-state controls, validation, persistence, and user-facing effects.

Prerequisites:
- Admin account has Community settings permissions.
- Record starting settings and restore them after the pass.

Pages and endpoints covered:
- `/admin/community/settings.html`
- `/api/admin/community/settings`
- `/community`
- `/community/partials/:tab`

Steps:
1. Open `/admin/community/settings.html`.
2. Verify all settings load from `/api/admin/community/settings`.
3. Change one test-safe setting and save.
4. Reload admin settings and verify persistence.
5. Verify the corresponding user-facing Community behavior changes after reload.
6. Toggle Community disabled/unavailable mode only in a controlled local/test environment.
7. Verify `/community` and partials reflect disabled state, then restore the original setting.
8. Try invalid values for booleans, numbers, URLs, text limits, and unknown keys.
9. Verify read-only admin and normal user access is blocked.

Expected Result:
- Settings save atomically and persist after reload.
- User-facing effects match setting names and do not require manual cache clearing.
- Invalid or unknown settings are rejected without overwriting valid settings.
- Original state is restored at the end.

Edge Cases:
- Concurrent admin saves.
- Partial settings payload.
- Missing CSRF token.
- Setting references deleted announcement/challenge/AMA.
- Disable Community while a user is on a client-side tab.

Required Workflow Fields Appendix:

Roles: Admin, Community User for readback where the admin action changes visible community state.

Primary pages: Admin Community pages listed above, plus affected public/community readback pages when applicable.

Backend/API surfaces: Community routes and services under `backend/src/community/**`; admin community routes under `backend/src/admin/**` where this workflow includes moderation, grants, settings, reports, or audit review. See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for exact route-to-workflow mappings.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | The workflow reaches the visible final state and persists after page reload. |
| Authorization boundary | Logged-out, wrong-role, non-owner, banned, or muted actors are redirected, blocked, or receive `401`/`403` without partial writes. |
| Validation failure | Missing, malformed, duplicate, stale, or out-of-state input is rejected with recoverable UI feedback. |
| Reload/readback | The affected community/admin page is reloaded after mutation and reflects database/API state, not stale client state. |
| Cleanup | Disposable `Workflow Test` content, uploads, grants, reports, or moderation state can be removed, reverted, or intentionally retained with a note. |

Negative Cases: Use the edge cases above plus unauthorized direct API access, duplicate submit, stale record, hidden/deleted content access, network failure, and unsafe user-generated content. Upload branches must reject invalid file type, oversize files, missing storage objects, and inaccessible storage links.

Audit / DB / Financial Checks: Admin moderation, grants, settings, reports, appeals, and destructive actions must write community/admin audit rows with actor, action, target, timestamp, prior/new state where available, and redacted sensitive values. Community XP, badges, reports, notifications, and saved/bookmark rows must persist once and remain idempotent on duplicate requests. Community workflows do not move money; if an asset/investment reference is shown, verify it remains read-only here and any monetary values stay integer cents in the owning investor/admin workflow.

Cleanup: Delete or hide disposable posts/comments/uploads where policy allows, undo test reactions/bookmarks/follows/mutes/blocks, revert badge/grant/settings/moderation changes, remove temporary files, and retain audit logs unless the environment is disposable and the cleanup runbook explicitly truncates them.
