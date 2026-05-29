# Audit: _affiliate_team_shell (partial)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/_affiliate_team_shell.html` (LOC: 32) |
| **Page route** | Partial — no route |
| **Handler** | — |
| **Template name** | `developer/_affiliate_team_shell.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429) — registers global `window.DAT` namespace |
| **Linked CSS** | `developer-affiliate-team.css` (skip-link, sr-only, dat-team-meta-mount styles) |
| **Mobile CSS** | n/a (partial inherits parent page CSS) |
| **Included by** | `affiliate-team.html:22`, `affiliate-team-analytics.html:22`, `affiliate-team-customers.html:21`, `affiliate-team-members.html:21`, `affiliate-team-products.html:21`, `affiliate-team-settings.html:21`, `affiliate-team-tier.html:21` |
| **Status** | Production-Ready |
| **Score** | 10 / 10 |

## 1. Purpose & user journey
**This is a partial** — never served standalone. Provides three things that every `/developer/affiliate-team/*` sub-page needs:
1. WCAG 2.4.1 skip-to-content link (`<a class="skip-link" href="#dat-page-content">`).
2. An sr-only `<h2 id="dat-page-content">` to bridge the heading hierarchy (h1 in topbar → h2 here → h3 per card).
3. A hidden `#dat-team-meta-mount` with `data-team-name` / `data-team-meta` attributes — used by `developer-affiliate-team-shell.js` to expose team info before per-page modules try to read it.

Despite the file being only 32 LOC, the JS file it pairs with (`developer-affiliate-team-shell.js`, 1429 LOC) provides the bulk of shared behaviour: `DAT.apiGet/Post/Patch` with timeout+retry, `DAT.dataTable` widget (sort/search/paginate/select/page-size persistence), `DAT.chipBar`, `DAT.topbarDateRange`, `DAT.downloadCsv`/`downloadXlsx` (native browser ZIP+OOXML), `DAT.skeletonRows`, `DAT.toast`, `DAT.confirm` modal, the invite-modal logic, the `loadTeamInfo` KPI tile loader.

## 2. Frontend structure
- 3 small DOM regions only (skip link, sr-only h2, hidden meta mount). All other UI lives in the parent template.
- Tightly coupled to `developer-affiliate-team-shell.js` which initializes on `DOMContentLoaded` (line 1428).
- The shell JS also moves the invite modal to `<body>` on open and applies `inert` + `aria-hidden` to main + sidebar so background isn't tabbable (FA5 fix at shell.js:1296-1319).

## 3. Backend wiring
Shell JS calls `GET /api/developer/affiliate/team` (loadTeamInfo, shell.js:1222) → `get_team_info` `rewards/team_routes.rs:398`.
Shell JS handles `POST /api/developer/affiliate/team/invite` (submitInvite, shell.js:1388) → `invite_member` `rewards/team_routes.rs:1251`.

Sub-pages: this partial is included by all 7 templates listed above (5 main sub-pages + the dual analytics route + the root).

Auth: relies on parent template's `require_developer_page` → no separate gate.

## 4. Data realism
The partial itself has no data. The shell JS populates real values from the team API.

## 5. Error & empty states
- `loadTeamInfo` failure: toast "Team data — Could not load team info — KPI tiles may be stale. Try refreshing." + sentinel `DAT.teamDataError` for downstream modules to check (shell.js:1262-1267).
- Shell-level `DAT.apiGet` retries once on 5xx + once on network error (shell.js:101-110).
- `DAT.toast` falls back to console.log when global `showPooolToast` isn't available (shell.js:144-150).

## 6. Mobile & responsive
- Skip-link visually hidden until focused (CSS `.skip-link:focus` reveals it).
- No layout footprint — inherits parent CSS.

## 7. Tests
- No tests cover the partial directly.
- Shell JS utilities (`DAT.downloadXlsx`, `DAT.dataTable`, etc.) have no unit tests in the audit scope.

## 8. Functional gaps & dead code
- The hidden `#dat-team-meta-mount` is described as "dev-team metadata that JS reads" — but currently the shell JS only populates `#dat-team-name` and `#dat-team-meta` text content with no caller surfacing it elsewhere (since the visible page-header was removed). The mount + JS write is effectively no-op for users. Comment at lines 5-9 notes "may be surfaced into the topbar later via a context-var injection."
- `_originalParent` / `_originalNextSibling` tracking for modal re-parenting (shell.js:1315-1320) is correct but slightly fragile — if some other JS removes the modal, restoration silently fails.
- No `TODO`/`FIXME`/`mock`/`Lorem` markers (only WIP-y comment "may be surfaced later").

## 9. Production blockers
- **Low**: Hidden team-meta mount is currently dead weight; either wire it into the topbar context or remove.
- **Low**: No unit tests for the shell JS utility library (1429 LOC of shared logic — `DAT.dataTable` widget alone deserves coverage).

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | All needed sub-page shared bits present (skip-link, h2 bridge, meta mount). |
| Backend wiring | 2/2 | Via shell JS — clean API contract. |
| Data realism | 2/2 | Real team info. |
| Error/empty states | 1/1 | Retry + toast + sentinel on team-info failure. |
| Mobile/responsive | 1/1 | No layout footprint; skip-link works on all viewports. |
| Tests | 1/1 | Indirectly covered by every test in `backend/tests/developer_affiliate_team_http.rs` + the parametrized E2E in `tests/e2e/test_developer_affiliate_team.py` (each sub-page exercises the shell). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Skip-link + sr-only heading bridge; modal inert+aria-hidden focus isolation. |
| **TOTAL** | **10/10** | All dimensions met. |
