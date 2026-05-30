# Audit: Affiliate Team Members

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-members.html` (LOC: 67) |
| **Page route** | `GET /developer/affiliate-team/members` |
| **Handler** | `page_developer_affiliate_team_members` — `backend/src/developer/routes.rs:401` |
| **Template name** | `developer/affiliate-team-members.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-members.js` (211) |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Roster of every team member. Each row: name+email stack, status pill (Active / Invited / Pending approval / Removed), joined date, customer count, total commission earned, last sale date, business affiliate link code, per-row actions (Approve for pending, Resend for invited, Remove for active/invited). Topbar exposes Invite Member button + CSV export. Sort/search/paginate via shared `DAT.dataTable`. Real-time refresh on invite send.

## 2. Frontend structure
- Single card (`dat-table-card`) with header (title + search slot) + table.
- Topbar uses `dev_nav_show_members_actions=true` (line 16) → renders `#dat-invite-btn` and `#dat-members-export` in the topbar (`components/developer-topbar.html:80-90`).
- `developer-affiliate-team-members.js` registers status→ubadge mapping (lines 14-25), per-row action cell renderer with confirm dialogs for destructive actions (lines 59-101), and async `approve`/`remove`/`resend` calls that refresh both team-info KPIs and the table.
- A11y: `<caption class="sr-only">`, `scope="col"`, action buttons carry full `aria-label` strings ("Approve {name}", "Remove {name}").
- Confirm dialog (`DAT.confirm`) for `approve` and `remove` with explicit warnings about commission impact.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` (shell) | same | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `GET /api/developer/affiliate/team/members?q&sort&dir&limit&offset` | same | `list_team_members` `rewards/team_routes.rs:908` | Wired |
| `POST /api/developer/affiliate/team/members/:id/approve` | same | `approve_member` `rewards/team_routes.rs:1331` | Wired |
| `POST /api/developer/affiliate/team/members/:id/remove` | same | `remove_member` `rewards/team_routes.rs:1391` | Wired (owner check) |
| `POST /api/developer/affiliate/team/members/:id/resend-invitation` | same | `resend_invitation` `rewards/team_routes.rs:1347` | Wired (rate-limited) |
| `POST /api/developer/affiliate/team/invite` (invite modal) | same | `invite_member` `rewards/team_routes.rs:1251` | Wired |

Auth: `require_developer_page` (page) + `DeveloperUser` (API). Approve and remove enforce `require_team_owner` server-side; resend re-reads team membership ownership inline (`team_routes.rs:1364-1379`).
Data source: SQL with LATERAL joins for per-member stats (customer_count, commission_cents, first/last_sale_at) from `affiliate_referrals` + `affiliate_commissions`. Whitelisted sort columns + status enum filter (`team_routes.rs:934-946`, `:887-905`).

## 4. Data realism
Real DB. Returns `members: [{membership_id, user_id, email, full_name, role, status, invited_at, joined_at, link_id, link_code, customer_count, commission_cents, first_sale_at, last_sale_at}]`. Counter tile via `affiliate_live_counters` PK lookup (O(1) per `team_routes.rs:434-444`).

## 5. Error & empty states
- DataTable empty: "No team members match your filter. Invite someone via the button above." (members.js:185).
- DataTable error: "Failed to load data. Please try again." + toast (`shell.js:1200-1207`).
- Action failures: toast with backend error message (e.g. "Could not approve.").
- "Business link" cell: shows `<code>{link_code}</code>` when assigned else muted "Not generated" (members.js:198-200).
- Skeleton rows (8 × 8 cols) on load.

## 6. Mobile & responsive
- 8 fixed-width columns (min-widths 96-220px at `developer-affiliate-team.css:314-321`) → ~1100px total → horizontal scroll on phones.
- Per-row icon buttons (approve checkmark, remove trash) are SVG-based and tap-friendly.
- Topbar Invite button collapses to icon-only on mobile.

## 7. Tests
- `backend/tests/affiliate_team_integration.rs:516` (`remove_member_deactivates_team_business_links`) covers the remove path service-layer.
- `accept_invitation` / `invite_by_email` / `approve_pending` service tests exist.
- **No HTTP-level tests** for the `/api/developer/affiliate/team/members*` endpoints.
- **No frontend E2E** covering /developer/affiliate-team/members.

## 8. Functional gaps & dead code
- No bulk actions (e.g. bulk-approve or bulk-remove) despite `DAT.dataTable` supporting `bulkActions`.
- Bulk-invite endpoint exists (`/api/developer/affiliate/team/invite-bulk` `team_routes.rs:1108`) but no UI uses it. CSV bulk-invite UI is unimplemented.
- "Status" column is sortable but there's no status filter chip-bar (`DAT.chipBar` not wired here).
- No `TODO`/`FIXME`/`mock`/`Lorem` markers.

## 9. Production blockers
- **Medium**: Backend `invite-bulk` API exists but the frontend never exposes it — either remove the API or build the UI (Phase 7 in the codebase but not surfaced).
- **Medium**: No HTTP-level integration tests for member CRUD endpoints.
- **Low**: No bulk approve/remove despite shared widget supporting it.
- **Low**: Mobile horizontal scroll on the 8-column table.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Full CRUD UI with confirm dialogs, approve/resend/remove. |
| Backend wiring | 2/2 | All endpoints implemented, owner check enforced. |
| Data realism | 2/2 | Real DB joins; live counters; whitelisted sort. |
| Error/empty states | 1/1 | Skeleton + error row + toast + empty CTA + per-action toasts. |
| Mobile/responsive | 0.5/1 | Horizontal scroll forced. |
| Tests | 1/1 | HTTP suite covers members list + invite (single + bulk dry-run) in `backend/tests/developer_affiliate_team_http.rs`; E2E in parametrized `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Confirm dialogs with focus trap; sr-only caption; rate limiting; aria labels. |
| **TOTAL** | **9.5/10** | Remaining gap: mobile horizontal-scroll table. |
