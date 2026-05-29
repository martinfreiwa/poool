# POOOL Community Circles Pre-Implementation Audit

Date: 2026-05-20  
Scope: Community Circles implementation baseline before the Circle Feed default destination work.

## Executive Summary

The current Community implementation already contains a global Community page, a My Circles tab, Circle discovery UI, Circle settings, and several backend Circle APIs. The core product gap is that a Circle is treated as a settings/admin object instead of a community space: `/community/circle/:slug` redirects to `/community/circle/:slug/settings`, while no dedicated Circle Feed page exists.

This audit establishes the baseline before implementation of Phase 1 from the optimized Circles plan. The baseline confirms three reproducible blockers:

- `/community/circle/:slug` currently redirects to the Circle settings page.
- `posts.circle_id` does not exist, so Circle posts cannot be modeled separately from global posts.
- Circle Feed and Circle-specific composer are missing.

## Baseline Against Product Vision

Primary references:

- `docs/community-production-readiness-audit.md`
- `docs/community-circles-product-vision-roadmap.md`
- User-approved Circles implementation plan, dated 2026-05-20

Product target:

- Global Community remains `/community`.
- My Circles and Discover become `/community/circles`.
- Circle detail becomes `/community/circle/:slug`.
- Circle settings remain `/community/circle/:slug/settings`.
- Global posts remain `circle_id IS NULL`.
- Circle posts use `posts.circle_id`.
- Settings are secondary and role-gated.

## Current State Findings

### Routing and Information Architecture

Status: Not production-ready.

Findings:

- `backend/src/lib.rs` registers `/community/circle/:slug`.
- The bare Circle route currently redirects to `/community/circle/:slug/settings`.
- `/community/circle/:slug/settings` exists and renders `community-circle-settings.html`.
- `/community/circles` is not a canonical server-rendered page.
- The My Circles UI exists as a tab/partial under `/community?tab=circle`.

Impact:

- Users clicking a Circle land in an admin/settings context.
- Product semantics are inverted: Circle Feed is not the primary destination.
- The current routing prevents Circle Spaces from behaving like investment communities.

### My Circles and Discovery

Status: Partially implemented.

Findings:

- `frontend/platform/partials/community_circle.html` contains Recommended, My Circles-like sections, Discover filters, and Circle creation UI.
- `frontend/platform/static/js/community-circles-discover.js` already links Circle cards to `/community/circle/:slug`.
- Because the backend redirects that URL to settings, frontend links look correct but lead to the wrong experience.
- Some naming still uses singular "My Circle" in comments or legacy tab language.

Impact:

- The discovery UX is directionally correct, but backend routing breaks the expected content-first flow.

### Feed and Composer

Status: Missing for Circles.

Findings:

- Global feed is rendered through `frontend/platform/partials/community_feed.html`.
- The global composer posts to `/api/community/posts`.
- There is no Circle Feed page template.
- There is no Circle-specific post composer.
- The composer does not show "Post to: <Circle Name>".

Impact:

- Users cannot create Circle-scoped posts.
- Circle pages cannot become living community spaces.

### Data Model

Status: Insufficient for Circle posts.

Findings:

- `backend/src/community/models.rs` `Post` does not contain `circle_id`.
- `CreatePostRequest` does not contain `circle_id`.
- The database migrations under `database/community/` do not add `posts.circle_id`.
- Existing global posts cannot be distinguished from Circle posts by schema.

Impact:

- Backend cannot enforce Circle feed isolation.
- Global feed cannot reliably exclude Circle posts.
- Circle-specific authorization cannot be tied to post creation.

### Backend APIs and Authorization

Status: Partial.

Findings:

- Existing APIs support Circle listing, joining, leaving, invitations, recommendations, and settings.
- No `GET /api/community/circles/:id/posts` endpoint exists.
- No `POST /api/community/circles/:id/posts` endpoint exists.
- Existing post creation API is global and does not validate Circle membership or Circle write permissions.
- Existing settings route is auth-gated, but the Feed route does not exist.

Impact:

- Circle content cannot be created or fetched through a dedicated API.
- Server-side enforcement for Circle write access is absent.

### Security and Compliance

Status: Partial, not complete for Circle scope.

Findings:

- Community service uses server-side post sanitization and automoderation for global posts.
- Circle membership APIs exist.
- There is no Circle-scoped post authorization matrix.
- There is no server-side separation between global and Circle posts.
- Compliance disclaimers exist for some global post types, but not in a Circle-specific UX.

Impact:

- Circle posting would be unsafe if implemented only in frontend.
- Private and protected Circle semantics cannot be trusted until enforced server-side.

### Accessibility

Status: Partial.

Findings:

- The global Community UI includes tab and form controls.
- No Circle Detail page exists, so Circle-specific tab navigation, header actions, and composer accessibility cannot be assessed yet.

Impact:

- Circle Feed must be implemented with keyboard-accessible tabs/actions and clear labels from the start.

### Tests

Status: Incomplete and currently blocked by unrelated baseline failures.

Executed baseline checks:

```bash
python3 -m pytest tests/test_community_tab_contract_static.py tests/test_community_profile_static.py tests/admin/test_admin_community_user_detail_static.py tests/admin/test_admin_community_users_static.py -q
```

Result:

- Failed in `tests/test_community_tab_contract_static.py::test_ama_fragment_and_controller_share_dom_contract`.
- Missing DOM contract: expected `id="ama-expert-avatar"` in `frontend/platform/partials/community_ama.html`.
- This appears unrelated to Circle Feed routing.

```bash
cd backend && cargo check
```

Result:

- Failed due local SQLx/database baseline issues:
  - `backend/src/admin/rewards.rs`: column `tiers.referral_bonus` does not exist.
  - `backend/src/community/routes.rs`: relation `community_profiles` does not exist.

Impact:

- The repository baseline is not fully green before this implementation.
- Post-implementation verification must separate existing environment/schema blockers from newly introduced Circle changes.

### Route and Template Inspection

Inspected areas:

- `backend/src/lib.rs`
- `backend/src/community/routes.rs`
- `backend/src/community/service.rs`
- `backend/src/community/models.rs`
- `backend/src/community/circles.rs`
- `frontend/platform/community.html`
- `frontend/platform/partials/community_circle.html`
- `frontend/platform/partials/community_feed.html`
- `frontend/platform/static/js/community-feed.js`
- `frontend/platform/static/js/community-circles-discover.js`
- `frontend/platform/community-circle-settings.html`

Key reproducible finding:

- Frontend discovery links already target `/community/circle/:slug`; backend redirects that route to settings.

## Pre-Implementation Scores

| Category | Score | Rationale |
|---|---:|---|
| UX / Information Architecture | 42% | Discovery exists, but Circle click opens settings instead of community space. |
| Backend/API | 45% | Circle APIs exist for membership/settings, but Circle posts endpoints are absent. |
| Data Model | 35% | No `posts.circle_id`; global and Circle content cannot be separated. |
| Security/AuthZ | 48% | Auth and membership primitives exist, but Circle posting authorization is absent. |
| Frontend | 50% | Global Community and Circle discovery exist; Circle detail page is missing. |
| Accessibility | 45% | Base controls exist; Circle page accessibility cannot be validated yet. |
| Tests | 38% | Static tests exist, but baseline failures and Circle-specific coverage gaps remain. |
| Documentation | 55% | Product vision exists; implementation-specific audit and test matrix were missing. |
| Production Readiness | 41% | Critical Circle Feed, schema, authz, and route semantics are incomplete. |

Overall baseline: 44%.

## Implementation Gate

Phase 1 may proceed if the implementation:

- Removes the bare Circle route redirect to settings.
- Adds a Circle Feed SSR page.
- Adds `posts.circle_id`.
- Adds Circle post read/write APIs with server-side authorization.
- Keeps global feed scoped to `circle_id IS NULL`.
- Adds focused tests for route, template, API contract, and migration expectations.
- Produces a matching post-implementation audit with test results and residual risks.
