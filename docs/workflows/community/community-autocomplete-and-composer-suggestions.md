# Community Autocomplete and Composer Suggestions

Purpose: Verify composer autocomplete for mentions, hashtags, assets, circle mentions, keyboard behavior, escaping, and stale suggestion handling.

Prerequisites:
- User is logged in.
- Seed data includes searchable users, hashtags, assets, and circles where supported.

Pages and endpoints covered:
- `/community?tab=feed`
- `/community/circle/:slug`
- `/api/community/mentions/suggest`
- `/api/community/hashtags/suggest`
- `/api/community/assets/suggest`
- `/api/community/circles/search`

Steps:
1. Open the feed composer.
2. Type `@` and a known user prefix; verify mention suggestions load.
3. Select a mention with mouse and keyboard and verify inserted text is correct.
4. Type `#` and a known hashtag prefix; verify suggestions load and insert safely.
5. Type `$` or the configured asset trigger; verify asset suggestions load and link/insert as expected.
6. In circle-aware composer flows, type a circle mention prefix and verify only visible/authorized circles are suggested.
7. Verify arrow keys, Enter, Escape, blur, and click-out behavior.
8. Submit a post with selected suggestions and verify rendered links go to the correct route.
9. Verify suggestion labels containing HTML/script render as text only.
10. Verify slow, empty, failed, and stale responses do not insert the wrong suggestion.

Expected Result:
- Autocomplete is keyboard-accessible and does not break normal typing.
- Suggestions are scoped to user visibility and privacy rules.
- Inserted mentions/hashtags/assets link to the correct pages after posting.
- User-generated suggestion labels cannot execute HTML/script.

Edge Cases:
- Empty query.
- Query with spaces, emoji, punctuation, and very long text.
- Suggestion deleted between fetch and selection.
- Private/hidden circle suggestion for unauthorized user.
- Duplicate display names.
- Network failure after menu opens.

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
