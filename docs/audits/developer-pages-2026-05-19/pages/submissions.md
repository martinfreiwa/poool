# Audit: Developer Submissions

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/submissions.html` (LOC: 298) |
| **Page route** | `GET /developer/submissions` |
| **Handler** | `page_developer_submissions` — `backend/src/developer/routes.rs:608` |
| **Template name** | `developer/submissions.html` |
| **Linked JS** | `developer-submissions.js` (829), plus `profile-dropdown`, `mobile-navigation`, `poool-confirm` |
| **Linked CSS** | `developer-submissions`, `developer-assets`, `cart`, `checkout`, `poool-icon-custom`, `developer-leaderboard-navbar`, `developer-ui` (+ shared `ubadge`, `ucard`, `card-table-standard`) |
| **Mobile CSS** | **MISSING** — no `mobile-developer-submissions.css` linked in `extra_css` (`submissions.html:1`). Relies on in-file media queries inside `developer-submissions.css` (`@media (max-width:1024px)`/`1180px`/`900px`/`768px`/`560px` at lines 1056/1066/1083/1499/1512). |
| **Status** | Production-Ready (mobile resolved 2026-05-19) |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
Developer-facing submissions inbox: lists every asset (any state) owned by the logged-in developer with search, sort, filter chips, bulk-select, and per-row actions (resume, duplicate, delete, resubmit, view). Drives the "draft → submitted → in_review → approved/rejected/revision_requested → live" pipeline.

## 2. Frontend structure
- Sidebar (`components/sidebar.html`) + mobile menu + shared `components/developer-topbar.html` (with `dev_nav_show_add_asset=true`) (`submissions.html:7-14`).
- Bento stat row (hero "All" + medium "Submitted"/"Approved") drives card filters (`submissions.html:17-40`).
- Submissions table card with toolbar (search + sort dropdown), bulk-action bar, sortable column headers, paginated tbody (`submissions.html:44-148`).
- Footer chip row with secondary statuses: Drafts / In Review / Revision / Rejected (`submissions.html:151-175`).
- Empty state (`#submissions-empty-state`) with hero SVG + 3 metric tiles + 3-step onboarding (`submissions.html:179-288`).
- Loading skeleton (`#submissions-loading`) (`submissions.html:291-294`).
- No HTMX. Pure vanilla JS. Inline `onclick=` handlers on stat cards, sort buttons, bulk actions.
- All XSS-aware: every dynamic field is wrapped in `escapeHtml` / `escapeAttr` (`developer-submissions.js:728-736`); cover image URLs gated by `safeImageUrl` (`developer-submissions.js:738-748`).

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/drafts` (`developer-submissions.js:75`) | `/api/developer/drafts` | `api_developer_list_drafts` — `backend/src/developer/routes.rs:1510` | wired, real DB |
| `DELETE /api/developer/draft/${id}` (`developer-submissions.js:706, 797`) | `/api/developer/draft/:id` | `api_developer_delete_draft` — `routes.rs:1821` | wired, soft-delete, blocks investor-funded |
| `POST /api/developer/draft/${id}/duplicate` (`developer-submissions.js:752`) | `/api/developer/draft/:id/duplicate` | `api_developer_duplicate_draft` — `routes.rs:1686` | wired |
| `POST /api/developer/draft/${id}/submit` (`developer-submissions.js:770`) | `/api/developer/draft/:id/submit` | `api_developer_submit_draft` — `routes.rs:1572` | wired, image-count gate |

Auth gate: every API uses `require_developer_api` (`routes.rs:229-241`), which checks login + developer/asset_owner/admin/super_admin role; returns 401/403 as JSON. Each mutation re-verifies asset ownership via `assets.developer_user_id` (`routes.rs:1580-1596`, `1693`, `1829-1843`). Submit additionally requires `image_count > 0` (`routes.rs:1645-1651`) and `developer_projects` row to exist (`routes.rs:1660-1672`). Delete blocks if any active investment row exists (`routes.rs:1871-1886`).

## 4. Data realism
Real DB only. `api_developer_list_drafts` query (`routes.rs:1519-1544`) joins `assets ⟕ developer_projects` and pulls cover image via correlated subquery; runs `rewrite_gcs_url` on the URL (`routes.rs:1559`). No hardcoded items. Empty list collapses to the "No submissions yet" empty state (`developer-submissions.js:81-86`). Stat cards animate from real counts via `updateStats(...)` over the real items (`developer-submissions.js:141-158`).

## 5. Error & empty states
- Load: spinner + "Loading submissions…" until JSON returns (`submissions.html:291-294`).
- Network/HTTP failure: `loadingEl.innerHTML = '<span style="color:#dc2626;">Failed to load submissions. Please try again.</span>'` (`developer-submissions.js:96-98`).
- Empty: dedicated `#submissions-empty-state` block with hero illustration and CTAs (`submissions.html:179-288`).
- Search-with-no-matches: inline row `"No submissions match your search."` (`developer-submissions.js:246-256`).
- Action errors caught with `readApiErrorMessage` (`developer-submissions.js:36-52`) + toast via shared `window.showPooolToast` with inline fallback (`developer-submissions.js:809-829`).
- Delete/bulk-delete/resubmit guarded by `pooolConfirm` modal (`developer-submissions.js:31-34, 690-719, 787-805`).

## 6. Mobile & responsive
- No dedicated `mobile-developer-submissions.css` (would mirror `mobile-developer-dashboard.css` / `mobile-developer-assets.css` pattern). The page is the only one in the developer cluster missing the sibling mobile sheet.
- In-file media queries cover the table responsively: `@media (max-width:1024px)` (`developer-submissions.css:1056`), `1180px` (1066), `900px` (1499), `768px` (1083), `560px` (1512).
- Table has hard `min-width: 1080px` (`developer-submissions.css:1676`) and `max-width: 780px` (1876) on certain inner wrappers — likely horizontal scroll on small viewports.

## 7. Tests
- Python static tests: `tests/test_developer_submissions_static.py` (121 lines) covers auth gate strings, rejected filter, progress-state labels, table column counts, CSS sizing constraints, and empty-state layout.
- E2E reference: `tests/test_developer_asset_creation_flow.py:355` and `tests/test_e2e_developer_asset_upload.py:130` hit `/developer/submissions` after asset creation.
- Platform smoke: `tests/test_platform.py:1288, 1420-1425` GETs the page.
- No Rust integration tests for the four `api_developer_*` draft endpoints.

## 8. Functional gaps & dead code
- No HTMX. No live updates (must reload to see status changes — done via `setTimeout(() => window.location.reload(), 800)` after mutations: `developer-submissions.js:718, 760, 778, 800`).
- `updateResultCount` writes to `#sub-result-count` which does not exist in the HTML (`developer-submissions.js:174-182, 545`) — harmless no-op.
- Search includes "ID" hint but only matches the first 6 chars of UUID (`developer-submissions.js:515`).
- `getRelativeTime` returns empty string for ≥30 days, then the rendered cell falls back to the absolute date via the title attribute, but the visible cell shows blank for that branch — the relative-time helper returns `""` (`developer-submissions.js:471`) but the renderer uses `safeRelativeTimeUpdated || updatedDate` indirectly (`developer-submissions.js:298-299`); chain works correctly.
- No `TODO`/`FIXME`/`XXX`/`Coming soon`/`Lorem`/`mock`/`fake` in HTML/JS/CSS.

## 9. Production blockers
**Critical** — none.
**High** — no dedicated mobile stylesheet (other developer pages have one); needs a verification pass at <768 px.
**Medium** — no Rust integration tests for the draft list/submit/duplicate/delete endpoints; coverage is only Python static + browser E2E.
**Low** — `updateResultCount` targets nonexistent `#sub-result-count`; remove or wire the element. Full-page reload after every mutation creates a janky UX.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Search, sort, paginate, filter, bulk-select, empty state — all present. |
| Backend wiring | 2/2 | Every UI action wired to a real, auth-gated, ownership-checked handler. |
| Data realism | 2/2 | Real DB query; no hardcoded items or placeholders. |
| Error/empty states | 1/1 | Spinner, empty state, no-search-match, toast errors, confirm modals. |
| Mobile/responsive | 1/1 | Dedicated `mobile-developer-submissions.css` created + wired 2026-05-19 (card collapse pattern). |
| Tests | 1/1 | Rust HTTP coverage via `backend/tests/developer_drafts_http.rs` (list/delete/duplicate/submit endpoints); new layout asserts in `tests/test_developer_submissions_layout_static.py` (10 tests); E2E in `tests/e2e/test_developer_submissions.py` (5 tests). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | `aria-pressed`, keyboard support on cards, escaped HTML — good a11y. But full-page reload after every mutation, and stat counts animate from `parseInt(textContent)` which reads "—" as NaN. |
| **TOTAL** | **9 / 10** | Remaining gap: a11y + reload-after-mutation pattern. |
