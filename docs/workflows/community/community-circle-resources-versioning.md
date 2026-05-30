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
