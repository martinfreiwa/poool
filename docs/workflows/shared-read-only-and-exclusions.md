# Shared Read-Only And Exclusion Workflow

Purpose: Classify shared shell, read-only route, component, partial, archive, PDF-template, HTMX fragment, and diagnostic surfaces so they are covered without duplicating role workflows.

Roles: Public Visitor, Investor, Developer, Admin.

Primary pages:
- Shared shell and aliases: `/profile`, `/fonts-template.html`, `/forms-template.html`, `/statistics-template.html`, `/table-template.html`, `/overlays-template.html`.
- HTMX/read-only fragments and API readback routes listed in `WORKFLOW_COVERAGE_MATRIX.md`.
- Non-standalone templates under `frontend/platform/components/**`, `frontend/platform/partials/**`, `frontend/platform/templates/**`, and `frontend/platform/_archive/**`.
- Health/read-only API surfaces such as `/health`, `/api/me`, search/list/detail endpoints, report downloads, and marketplace read models.

Backend/API surfaces:
- `backend/src/lib.rs` shared page/API routes.
- Any `GET` route in `backend/src/**/*.rs` that is not already owned by a role workflow.
- Component and partial templates imported by role pages.

Prerequisites:
- Local backend is running with disposable authenticated sessions for each role.
- No live/staging mutations are run during this workflow.
- Browser viewport includes desktop and mobile widths for shell/components.

Steps:
1. Open each shared shell page or read-only route directly and through the role page that uses it.
2. Verify components/partials render inside their parent workflow and are not treated as standalone destinations.
3. For GET APIs, verify authorization, pagination/filter query handling, empty result shape, and error shape.
4. For archive/PDF/template files, verify they are excluded from standalone page coverage with a reason and source path.
5. For diagnostic/admin read-only pages, verify they do not expose secrets, raw storage paths, or cross-tenant data.
6. On every role switch, reload the destination page and verify the visible state belongs to the current role.

Expected Result:
- Every non-mutating route or template is classified as role workflow coverage, read-only matrix coverage, or explicit exclusion.
- No read-only check mutates data.
- Component and partial coverage is inherited from parent pages and not double-counted as product pages.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Shared shell | Sidebar/topbar/profile/search/cart/notification/mobile controls render in role workflows. |
| GET API | Authorized calls return expected shape; unauthorized calls redirect/401/403. |
| HTMX fragment | Fragment is covered by parent page and not a standalone destination. |
| Component/partial | Parent workflow proves it renders; matrix marks it `excluded-component`. |
| Archive/PDF template | Matrix marks it `excluded-non-standalone` with source path. |
| Health/read-only | No secrets or mutations; response is stable enough for live confidence pass. |

Negative Cases:
- Logged-out access to protected GET routes.
- Cross-role direct access to another user's read model.
- Missing query parameter for context-dependent detail page.
- Fragment opened without parent context.
- Archive/component path incorrectly linked as a real route.

Audit / DB / Financial Checks:
- Read-only routes must not insert/update/delete rows.
- Export/report downloads should log audit only where product policy requires it; otherwise no data mutation is expected.
- Financial read models show cents-derived display values and never recompute authoritative balances client-side.
- Storage links are scoped and do not expose private bucket internals.

Cleanup:
- Clear any downloaded temporary reports/screenshots.
- No DB cleanup is expected for read-only checks.
