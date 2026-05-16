# Security Audit: Admin + Auth Surfaces (Follow-up to Wallet Audit)

Date: 2026-05-16

Status: **5 of 10 fixed (2026-05-17); 5 deferred** pending larger-
refactor scope. None of the items below are exploited in the wild —
they're latent risks discovered by mirroring the 2026-05 wallet audit
pattern onto the next-most-sensitive surfaces.

| # | Sev | Status | Commit |
|---|-----|--------|--------|
| 1 | H | ✅ fixed 2026-05-17 | `9539e31` (idempotency + outbox) |
| 2 | H | deferred | needs idempotency on treasury endpoint |
| 3 | H | ✅ fixed 2026-05-17 | `07af8ab` (role guard) |
| 4 | H | deferred | landing with the user's step_up enum WIP |
| 5 | M | ✅ fixed 2026-05-17 | `9539e31` (sync outbox) |
| 6 | M | deferred | login audit refactor |
| 7 | M | ✅ fixed 2026-05-17 | `875c7ac` (fail-closed) |
| 8 | M | deferred | needs new `pii.view_sensitive` permission |
| 9 | M | deferred | dual rate-limit atomicity refactor |
| 10 | L | ✅ fixed 2026-05-17 | `9d824fd` (log redaction) |

## Scope

Files reviewed:

- `backend/src/admin/deposits.rs`
- `backend/src/admin/users.rs`
- `backend/src/auth/routes.rs`
- `backend/src/auth/step_up.rs`

Lens applied: the same 10 issue classes that surfaced in the prior wallet
audit (C-1 mandatory-upload bypass; H-1 missing permission gate; H-2/H-3
idempotency scope + release; H-4 atomic audit log; M-1 CSV injection;
M-2 SELECT-then-UPDATE race; M-3 PII in log lines; C-2 webhook replay
window; M-4 defense-in-depth invariants).

## Findings

| # | Sev | File:line | Issue | Fix |
|---|---|---|---|---|
| 1 | H | `admin/deposits.rs:185+` `api_admin_deposit_confirm` | Mutating admin endpoint has no `Idempotency-Key`. Double-click/retry fires `deposit_confirmed` email twice; the second status check is the only line of defence. Same class as wallet H-2. | Accept `Idempotency-Key`, dedupe via `idempotency_keys` table per `(key, admin_id)`. |
| 2 | H | `admin/users.rs:814+` `api_admin_user_update_balance` | Treasury credit/debit endpoint has no idempotency. Retried request silently double-credits a user. Same class as #1. | Same fix as #1; mandatory header for money-moving admin calls. |
| 3 | H | `admin/users.rs:1033+` `api_admin_user_update_roles` | Step 1 `DELETE FROM user_roles` succeeds even if no role names in step 2 exist (`role_id` lookup silently skipped). A typo'd role demotes the user to **zero roles** without erroring. | Pre-validate every name in `payload.roles` resolves to a `roles.id`; or assert `rows_inserted == payload.roles.len()` and rollback otherwise. |
| 4 | H | `auth/routes.rs:768+` `step_up_verify` | Action parser drops `affiliate_bank` and `affiliate_payout` from the `FinancialAction` enum (only 4 of 6 branches). Endpoints permanently reject with `TwoFactorRequired` even after a successful TOTP. Defeats the "always 2FA" intent at `step_up.rs:46+`. | Add `"affiliate_bank" => AffiliateBankEdit, "affiliate_payout" => AffiliatePayoutRequest` arms. (User WIP already adds these enum variants — fix lands as part of that branch.) |
| 5 | M | `admin/deposits.rs:225+` confirm email | Fire-and-forget `tokio::spawn` for `deposit_confirmed` email runs *after* the DB commit. Pod restart between commit and spawn loses the user's notification. Same class as wallet H-4. | Replace `tokio::spawn` with a synchronous `trigger_transactional_email().await` — the function's first step is the durable `transactional_email_outbox` INSERT, so the email survives restart. |
| 6 | M | `auth/routes.rs:289+` `spawn_login_side_effects` | `user.login` audit log is fire-and-forget via `tokio::spawn` with 2s timeout — pod shutdown loses the audit row. | Inline the audit INSERT into the request handler (durable), or move to a dedicated outbox table for batching. |
| 7 | M | `admin/users.rs:11+` `api_admin_users` | PII-access audit is fire-and-forget (`let _ = sqlx::query(...).await`). Error swallowed → "who looked at every user" forensics silently drops rows under DB pressure. | `.map_err(ApiError::from)?` so the read fails if audit fails (fail-closed), or persist to outbox. |
| 8 | M | `admin/users.rs:188+, 510+` user-detail | Admin user-detail returns `tax_id`, `holder_name`, `address`, `payment_methods.last_four` in JSON with **no rate-limit or download-burst guard**, and the per-row audit covers only the top-level fetch. Over-broad PII scrape vector for a compromised admin session. | Per-detail audit row per child entity, **and** gate `payment_methods`/`tax_id` behind a separate `pii.view_sensitive` permission so the default admin role can't enumerate sensitive fields. |
| 9 | M | `auth/routes.rs:222+, 429+` login + 2FA rate-limit | Two sequential `RateLimiter::check()` calls (IP bucket, then email/user bucket) are **non-atomic**. An attacker bursting between the two checks consumes one slot but bypasses the second tier on parallel requests. Same class as wallet M-2. | Combine into one Redis Lua/`MULTI` script that increments both keys atomically, or check the stricter (user) bucket first and only debit IP on success. |
| 10 | L | `auth/step_up.rs:122+, 138+` | `tracing::info!` includes `amount_cents` and the `FinancialAction` enum (`Withdrawal`/`AffiliatePayoutRequest`) at user-id resolution. Log aggregators see a per-user "wants to withdraw $X" stream. Low because no email/name is leaked. | Drop `amount_cents` from the log line; emit a generic `step_up_required` event with only `user_id`. |

## Not flagged (verified clean during audit)

- `admin/deposits.rs` cancel / extend-expiry / update-status / force-password-reset all wrap UPDATE + audit in one transaction (H-4 ✓).
- `admin/deposits.rs` bulk-cancel uses `cancel_one_deposit_with_audit` helper (H-4 ✓ — landed in the 2026-05 wallet batch).
- Per-handler permission checks (`require_*_permission`) are present on every admin entry point (H-1 ✓).
- No CSV exports in scope (M-1 N/A).
- No webhook handlers in scope (C-2 N/A).

## Why fixes were deferred

The four target files all have substantial uncommitted in-flight WIP
(storage reconciler, step_up enum additions, frozen-user metadata,
storage rate-limiter wiring) that introduces cross-file symbol
dependencies. A targeted security commit on any of these files would
either (a) sweep WIP into a security commit under a misleading message,
or (b) leave HEAD in a non-buildable state because the WIP isn't a
unit.

Recommended path: ship the WIP first as feature commits, then layer the
fixes above as a second batch following the same per-finding commit
pattern used in the prior wallet audit (see commits `7639522`,
`685ad50`, `613cfca`).

## Monitoring added in parallel

`infra/prometheus/alerts.yml` extended with security-class alerts (see
the same-day commit) covering: webhook signature failures, admin
permission-denied spikes, idempotency conflict storms. These give early
warning even before the fixes above land.
