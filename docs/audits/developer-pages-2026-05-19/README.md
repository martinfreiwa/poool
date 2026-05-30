# Developer Pages — Production Readiness Audit

**Date:** 2026-05-19
**Scope:** All HTML files in `frontend/platform/developer/` (23 pages + 1 stale `.bak`)
**Method:** Static analysis. Each page audited against backend routes (`backend/src/developer/routes.rs`, `mod.rs`, `service.rs`, `villa_*.rs`, `change_requests.rs`), linked JS (`frontend/platform/static/js/developer-*.js`), CSS (`frontend/platform/static/css/developer-*.css`), and shared components (`frontend/platform/components/`).

---

## Scoring Rubric (0–10)

Each page scored across 7 weighted dimensions:

| Dimension | Max | Definition |
| --- | --- | --- |
| Frontend completeness | 2 | All sections present, no `Lorem ipsum`, no commented-out blocks, no `TODO` text in DOM. |
| Backend wiring | 2 | All fetch URLs / `hx-get` / `hx-post` resolve to a real handler in `backend/src/`. Auth-gated. |
| Data realism | 2 | Real DB queries via `service.rs` — not hardcoded JSON, not `Math.random()`, not faker data. |
| Error / empty states | 1 | Renders gracefully when API returns 4xx/5xx, when list is empty, when user has no data. |
| Mobile / responsive | 1 | Dedicated `mobile-*.css` exists OR layout uses fluid grid + tested at <768 px. |
| Tests | 1 | Has Rust integration test (`backend/tests/`) or E2E spec (`tests-e2e/`). |
| Polish (a11y, i18n, perf) | 1 | Semantic HTML, alt-text on images, no console errors, no >2 MB asset load. |

### Status bands

| Score | Status | Meaning |
| --- | --- | --- |
| 9–10 | **Production-Ready** | Ship as-is. |
| 7–8 | **Beta** | Functional but rough edges — ship to limited cohort. |
| 5–6 | **Alpha** | Core path works, multiple gaps. Internal demo only. |
| 3–4 | **Stub** | Skeleton + hardcoded data, no real backend. |
| 0–2 | **Stale / Broken** | Dead, orphaned, or broken; delete or rebuild. |

---

## Production-Readiness Matrix

> Filled in after per-page audits complete. See `_OVERVIEW.md` for the live matrix and prioritized blocker queue.

---

## Auth & Access Model (context for every audit)

- Page routes (`GET /developer/...`) registered in [backend/src/developer/mod.rs](../../../backend/src/developer/mod.rs).
- Pages render via `axum::response::Html` from `MiniJinja` templates served via [backend/src/templates.rs](../../../backend/src/templates.rs).
- Auth gating on pages uses session cookie + `middleware::get_current_user` then a role check via `user_has_developer_access` in `routes.rs` — roles `developer | asset_owner | admin | super_admin`. Non-developers redirected to onboarding / dashboard.
- JSON API routes (`/api/developer/...`) use the `DeveloperUser` extractor in [backend/src/developer/extractors.rs](../../../backend/src/developer/extractors.rs). Returns 401 if anonymous, 403 if not a developer.
- Per-villa writes additionally call `DeveloperUser::require_asset_link(asset_id)` to enforce `developer_asset_links` table (active = `effective_until IS NULL`).

---

## File Index

Per-page audits live in `./pages/`:

- Core: [dashboard](./pages/dashboard.md), [dashboard.html.bak](./pages/dashboard-bak.md), [annual-data](./pages/annual-data.md), [ranking](./pages/ranking.md)
- Assets: [assets](./pages/assets.md), [asset-detail](./pages/asset-detail.md), [add-asset](./pages/add-asset.md), [property-content](./pages/property-content.md)
- Submissions / Operations: [submissions](./pages/submissions.md), [submission-success](./pages/submission-success.md), [operations-dashboard](./pages/operations-dashboard.md), [operations-submit](./pages/operations-submit.md)
- Affiliate Team: [affiliate-team](./pages/affiliate-team.md), [affiliate-team-analytics](./pages/affiliate-team-analytics.md), [affiliate-team-customers](./pages/affiliate-team-customers.md), [affiliate-team-members](./pages/affiliate-team-members.md), [affiliate-team-products](./pages/affiliate-team-products.md), [affiliate-team-settings](./pages/affiliate-team-settings.md), [affiliate-team-tier](./pages/affiliate-team-tier.md)
- Affiliate Team Partials: [_affiliate_team_shell](./pages/_affiliate_team_shell.md), [_affiliate_team_invite_modal](./pages/_affiliate_team_invite_modal.md)
- Onboarding / Application: [developer-onboarding](./pages/developer-onboarding.md), [application-form](./pages/application-form.md), [document-upload-step3](./pages/document-upload-step3.md)

Overview matrix: [_OVERVIEW.md](./_OVERVIEW.md)
