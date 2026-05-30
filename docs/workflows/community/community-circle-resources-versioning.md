# Community Circle Resources and Versioning

Purpose: Verify Circle resource library management, file uploads, metadata edits, lifecycle states, version creation, version access, review, and restore.

Prerequisites:
- User is logged in as circle owner/admin.
- A disposable circle exists.
- Small valid and invalid test files are available.

Pages and endpoints covered:
- `/community/circle/:slug/settings`
- `/api/community/circles/:id/resources`
- `/api/community/circles/:id/resources/manage`
- `/api/community/circles/:id/resources/upload`
- `/api/community/circles/:id/resources/:resource_id/manage`
- `/api/community/circles/:id/resources/:resource_id/lifecycle`
- `/api/community/circles/:id/resources/:resource_id/versions`
- `/api/community/circles/:id/resources/:resource_id/versions/upload`
- `/api/community/circles/:id/resources/:resource_id/versions/:version_id/access`
- `/api/community/circles/:id/resources/:resource_id/versions/:version_id/review`
- `/api/community/circles/:id/resources/:resource_id/versions/:version_id/restore`
- `/api/community/circles/:id/resources/:resource_id/access`

Steps:
1. Open Circle settings and navigate to Resource Library.
2. Verify current resources load with empty/loading/error states.
3. Upload a valid resource file and create metadata: title, description, category, visibility, and lifecycle state.
4. Reload settings and circle detail; verify the resource appears only where access rules allow.
5. Edit resource metadata and verify persistence.
6. Upload a new version for the resource.
7. Open version history and verify both versions show correct metadata and active/current state.
8. Check resource access and version access as owner, member, non-member, and logged-out user where applicable.
9. Review/approve/reject a version if the workflow exposes review states.
10. Restore an older version and verify active content changes while history remains intact.
11. Change lifecycle state and verify archived/draft/published visibility.
12. Attempt invalid uploads: unsupported type, empty file, oversized file, and duplicate version marker.

Expected Result:
- Resource metadata, lifecycle state, files, and versions persist after reload.
- Access checks match circle privacy and resource visibility.
- Version review/restore actions are auditable and role-protected.
- Invalid uploads fail with clear recoverable errors and no stale preview.

Edge Cases:
- Non-owner tries manage endpoints.
- Resource deleted/archived while a version upload is in progress.
- Restore a stale version after newer approved version exists.
- Version access for private/token-gated circle.
- Duplicate title/version label.
- Network failure after file upload but before metadata save.

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
