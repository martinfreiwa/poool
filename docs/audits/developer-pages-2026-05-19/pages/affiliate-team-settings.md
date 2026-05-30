# Audit: Affiliate Team Settings

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-settings.html` (LOC: 299) |
| **Page route** | `GET /developer/affiliate-team/settings` |
| **Handler** | `page_developer_affiliate_team_settings` — `backend/src/developer/routes.rs:367` |
| **Template name** | `developer/affiliate-team-settings.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-settings.js` (430) |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Owner-only team configuration. 4 cards in a `<form id="dat-settings-form" novalidate>`:
1. **Team Identity** — display_name (req, 1-120) + public_slug (3-40 lower-kebab regex `/^[a-z0-9](?:[a-z0-9-]{1,38}[a-z0-9])?$/`).
2. **Public Join Page** — live URL preview based on slug, copy + open-in-new-tab.
3. **Brand Customization** — logo URL (https), accent color (hex), email sender display name + live email-CTA-button preview.
4. **Payouts & Banking** — next-payout read-only tiles + account holder, IBAN (req if any bank field provided), BIC, bank name, country.
5. **Team Overview** — lifetime read-only tiles (members, revenue, commission, created).
Sticky action footer with dirty-state detection enables Save/Discard; `beforeunload` warns on unsaved changes.

## 2. Frontend structure
- Standard sidebar/topbar/mobile-menu chrome (no team-actions / no date-range topbar).
- Forms use `ds-input` / `ds-btn` design-system classes.
- A11y: every input has `aria-describedby` to its hint; required fields marked with `*` + `aria-required`-style markup; live preview is `role="img"` (line 159).
- Slug field has `pattern` HTML5 validator + JS mirror (`SLUG_RE` at settings.js:15).
- IBAN field shows masked form returned by server (`**** **** **** {last4}`); dirty-detector compares masked form so re-typing the mask reads as "clean" (`settings.js:39-41, 49`).
- Hard-coded inline `<style>` attributes on Branding card (lines 136-146, 161-167) — should be moved to CSS.
- Bank IBAN: server requires 2FA step-up to PATCH bank fields (`team_routes.rs:499-509`).

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` | same | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `PATCH /api/developer/affiliate/team` | same | `update_team` `rewards/team_routes.rs:487` | Wired (2FA step-up for bank edits) |
| `GET /api/developer/affiliate/team/analytics/overview` (read-only payout tiles) | same | `analytics_overview` | Wired |
| `POST /api/developer/affiliate/team/invite` (invite modal) | same | `invite_member` | Wired |

Auth: `require_developer_page` (page) + `DeveloperUser` (API).
Server validates: display_name (1-120 trimmed), slug (`validate_slug` + uniqueness check across `developer_teams` excluding `status='terminated'`), bank fields (`normalize_bank_text`, `validate_iban`, `validate_bic`, `validate_country2`). Slug uniqueness query at `team_routes.rs:532-547`.
Bank IBAN write path encrypts to `bank_iban_encrypted` + stores `bank_iban_last4` (legacy plaintext column dual-written for migration period).

## 4. Data realism
Real DB. IBAN never round-trips plaintext to client (`team_routes.rs:421-432`). Branding fields persisted to `developer_teams.logo_url / accent_color / email_from_display`. Lifetime counters from `affiliate_live_counters` PK lookup.

## 5. Error & empty states
- Status banner in footer (`#dat-settings-status`) shows Saving… / Saved ✓ / Save failed.
- Toast on save success/failure.
- Public link card has empty state with helpful prompt ("Set a public slug above to publish your team's join page.").
- Empty state for unconfigured bank: "Not configured" pill.
- Invalid slug pattern marks input `aria-invalid="true"` and shows error hint (settings.js:104-119).
- Loading toast if `apiGet /team` fails: "Could not load team data. Try refreshing." (settings.js:404).

## 6. Mobile & responsive
- Form cards use `dat-settings-grid` (no horizontal scroll).
- `@media (max-width: 980px)` and `(max-width: 720px)` rules at `developer-affiliate-team.css:1249, 1252`.
- Bank input rows stack on mobile via grid behaviour.
- Inline `style="display:flex;gap:16px;..."` (line 136) won't respond to viewport — minor.

## 7. Tests
- No HTTP integration tests for the PATCH path.
- No frontend E2E tests for /developer/affiliate-team/settings.
- `validate_iban` / `validate_bic` / `validate_slug` are not unit-tested in the audit results.
- Step-up 2FA enforcement IS tested in `auth_2fa_http.rs` (separate test file).

## 8. Functional gaps & dead code
- Team status pill at line 32 always defaults to "Active" — only flipped if the backend returns status (which it does).
- "Discard changes" reloads only the values, not the underlying server state — if the server changed externally, the user gets stale values.
- No explicit "Delete team" / "Terminate team" UI — terminated status is checked in slug uniqueness but no user-facing trigger.
- No `TODO`/`FIXME`/`mock`/`Lorem` markers.
- Inline `<style>` attributes (e.g. lines 136-146, 161-167) bypass the design system.

## 9. Production blockers
- **High**: ~~No integration test covers PATCH `/api/developer/affiliate/team`~~ — **RESOLVED 2026-05-19** via `backend/tests/developer_affiliate_team_http.rs` (`patch_team_updates_display_name_and_slug`, `patch_team_iban_is_encrypted_at_rest` for B-P0-1, `patch_team_bank_fields_require_2fa_step_up` for the 2FA gate, `patch_team_rejects_taken_slug`).
- **Medium**: No "delete/terminate team" path despite schema supporting it.
- **Low**: Inline `<style>` on Branding card should move to CSS.
- **Low**: `accent_color` validation pattern is on the `<input pattern>` only; server-side validation not visible in audit slice — re-verify.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | 4 cards covering identity, public-page, branding, banking, overview + sticky footer + dirty detection + beforeunload. |
| Backend wiring | 2/2 | Single PATCH endpoint with per-field validation + 2FA step-up for bank changes. |
| Data realism | 2/2 | Real DB + encrypted IBAN dual-path. |
| Error/empty states | 1/1 | Status banner, toasts, aria-invalid, empty-state cards. |
| Mobile/responsive | 0.5/1 | Form stacks reasonably; some inline-style breakpoints lossy. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_affiliate_team_http.rs`: `patch_team_updates_display_name_and_slug` + `patch_team_rejects_taken_slug` + `patch_team_iban_is_encrypted_at_rest` (B-P0-1) + `patch_team_bank_fields_require_2fa_step_up`. E2E IBAN step-up case in `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | aria-describedby, role=img preview, required markers, focus management. |
| **TOTAL** | **9.5/10** | Remaining gap: mobile inline-style row + missing terminate-team UI. |
