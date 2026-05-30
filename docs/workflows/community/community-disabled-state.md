# Community Disabled State

Purpose: Verify Community disabled/unavailable rendering, protected access, partial fallback behavior, and recovery when Community is re-enabled.

Prerequisites:
- Environment or seed state can simulate Community disabled/unavailable mode.
- User is logged in; also test logged-out access.

Pages and endpoints covered:
- `/community`
- `/community/partials/:tab`
- `frontend/platform/partials/community_disabled.html`
- `/api/admin/community/settings`

Steps:
1. Disable Community through the supported admin setting or test fixture.
2. Open `/community` as a logged-in user.
3. Verify disabled copy, iconography, and available navigation actions render without broken tab controls.
4. Open each Community partial route and verify it returns a safe disabled state or protected redirect.
5. Verify Community APIs that should be blocked return explicit disabled/authorization responses.
6. Verify pages outside Community still work.
7. Re-enable Community and reload `/community`.
8. Verify normal shell, tabs, and API calls recover without stale disabled cache.
9. Test logged-out direct access while disabled and verify login/protection behavior remains correct.

Expected Result:
- Disabled state is explicit and does not expose partial Community content.
- Re-enable restores normal Community behavior without requiring a browser cache clear.
- The disabled state does not break global navigation.

Edge Cases:
- Disable while user is composing a post.
- Disable while HTMX partial is loading.
- Disable while notifications/messages panel is open.
- Re-enable with stale browser tab.
- Admin setting update fails halfway.

Required Workflow Fields Appendix:

Roles: Community User; Admin moderator only for ownership, moderation, or operational escalation branches.

Primary pages: Community pages and endpoints listed above; admin community pages only where the workflow explicitly includes moderation or operations.

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
