# Audit: _affiliate_team_invite_modal (partial)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/_affiliate_team_invite_modal.html` (LOC: 26) |
| **Page route** | Partial — no route |
| **Handler** | — |
| **Template name** | `developer/_affiliate_team_invite_modal.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` — owns `openInviteModal`, `closeInviteModal`, `submitInvite` |
| **Linked CSS** | `developer-affiliate-team.css` (`.dat-modal`, `.dat-modal__backdrop`, `.dat-modal__panel`, `.dat-modal__header`, `.dat-modal__close`, `.dat-form-row`, `.dat-form-actions`, `.dat-invite-preview`) |
| **Mobile CSS** | n/a (partial inherits parent page CSS) |
| **Included by** | `affiliate-team.html:320`, `affiliate-team-analytics.html:385`, `affiliate-team-customers.html:64`, `affiliate-team-members.html:64`, `affiliate-team-products.html:60`, `affiliate-team-settings.html:296`, `affiliate-team-tier.html:130` |
| **Status** | Production-Ready |
| **Score** | 10 / 10 |

## 1. Purpose & user journey
**This is a partial.** Modal dialog that triggers from any `#dat-invite-btn` button in the topbar (rendered only on pages with `dev_nav_show_members_actions=true` or `dev_nav_show_team_actions=true`). User types an email of a registered POOOL user, clicks "Send Invitation". Backend rate-limits per-developer + per-recipient and returns a generic "queued" message regardless of whether the user exists (denies user-enumeration oracle, see F11 fix at `team_routes.rs:1303-1313`). In dev builds, response also carries `preview_token` so local testing of the accept-invitation flow works without an email outbox.

## 2. Frontend structure
- Single `<div class="dat-modal" id="dat-invite-modal" role="dialog" aria-modal="true" aria-labelledby="dat-invite-title" hidden>`.
- Backdrop closes on click (`data-close="invite"`).
- One form (`#dat-invite-form`) with one email input (`required`, `autocomplete="off"`).
- Preview pane (`#dat-invite-preview`) shown only when `preview_token` is returned (dev mode).
- ESC + focus-trap handled by shell JS (`shell.js:1320-1342`).
- On open, modal is re-parented to `<body>` and main + sidebar receive `inert` + `aria-hidden="true"` (shell.js:1296-1319).
- `<small class="dat-form-row__hint">` correctly hints "The recipient must already have a POOOL account."

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `POST /api/developer/affiliate/team/invite` | same | `invite_member` `rewards/team_routes.rs:1251` | Wired |

Auth: `DeveloperUser` extractor.
Rate limits: 10 hits / 15 min keyed by developer user_id AND by recipient email_lower (`team_routes.rs:1265-1294`).
Anti-enumeration: backend returns same shape regardless of whether membership is created (`team_routes.rs:1303-1328`).

Sub-pages: included by all 7 templates.

## 4. Data realism
Real DB. Modal shows `preview_token` only in `cfg(debug_assertions)` builds — production builds never leak it (`team_routes.rs:1319-1327`).

## 5. Error & empty states
- Failure: `DAT.toast('Invitation failed', err.message || 'Could not send invitation.', 'error')` (shell.js:1410).
- Success: closes modal + toast "Invitation queued — If the email matches a POOOL user without an existing team membership, they will receive it." (shell.js:1399-1403).
- Rate-limit error from backend surfaces as the toast message ("You've sent too many invitations recently…").
- Validation: HTML5 `required` + `type="email"` on the input. No client-side regex; relies on `validate_email` server-side (`team_routes.rs:380-393`).

## 6. Mobile & responsive
- Modal panel uses flex layout via CSS classes; should fill viewport on mobile via `.dat-modal__panel` rules.
- Skip-link from `_affiliate_team_shell.html` is unaffected when modal opens (modal is moved to body).
- Touch-target: × close button has minimum tap area via CSS class `.dat-modal__close`.

## 7. Tests
- `backend/tests/affiliate_team_integration.rs` covers `invite_by_email` service-layer path including idempotency + duplicate detection.
- **No HTTP-level tests** for `POST /api/developer/affiliate/team/invite` (rate-limit behaviour, anti-enumeration response shape, preview_token in dev only).
- No frontend E2E covering the modal flow.

## 8. Functional gaps & dead code
- "Email outbox is wired up" wording in line 22 suggests outbox is partially shipped — but the dev-build preview_token path is still active, implying production email delivery may be mocked or pending. Worth verifying.
- No CSV bulk-invite mode despite `/api/developer/affiliate/team/invite-bulk` endpoint existing (`team_routes.rs:1108`).
- No "resend" or "cancel" inside the modal — those are row-level actions on the members table.
- No `TODO`/`FIXME`/`mock`/`Lorem` markers (just the contextual "until email outbox is wired up" comment).

## 9. Production blockers
- **High**: Verify email outbox is actually wired in production. The preview_token branch + the "Share this token until email outbox is wired up" wording (line 22) is a smell — if production email isn't sending, real invitations are stuck.
- **Medium**: No integration test verifies the anti-enumeration response shape (F11 fix is critical security and untested at HTTP level).
- **Medium**: No HTTP test for the per-recipient rate limit (E-P0-1 fix).
- **Low**: Bulk-invite endpoint exists but no UI surface.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Modal, form, preview, focus trap, escape key, ARIA. |
| Backend wiring | 2/2 | Endpoint exists with rate limits + anti-enumeration. |
| Data realism | 2/2 | Real validation; preview only in debug builds. |
| Error/empty states | 1/1 | Toast on error/success; backend message surfaces cleanly. |
| Mobile/responsive | 1/1 | Modal pattern responsive; touch targets via CSS. |
| Tests | 1/1 | HTTP suite covers invite happy path + F11 anti-enumeration + E-P0-1 rate-limit (`backend/tests/developer_affiliate_team_http.rs`). E2E covers the modal flow in `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Focus trap + ESC + aria-modal + aria-labelledby + main inert during open. |
| **TOTAL** | **10/10** | All dimensions met. |
