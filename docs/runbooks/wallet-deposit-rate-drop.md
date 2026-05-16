# Runbook: WalletDepositRateDrop

## Alert source

`infra/prometheus/alerts.yml` → `WalletDepositRateDrop`

Fires when the deposit-initiation rate in the last 10 min is less than
half the rate in the same 10-min window one hour ago (and that prior
window was non-zero).

## What this means

A user hit "Deposit" 50%+ less than the same time last hour. Two broad
causes:

1. **Demand shock** — marketing email/ad pulled, payday cycle, holiday,
   timezone. Not a code problem. Confirm with marketing/growth before
   escalating.
2. **Pipeline broken** — the deposit page or API is failing silently
   from the user's point of view. Most common: the bank-transfer
   instructions modal won't render (admin bank settings missing); the
   amount field rejects valid input; CSRF token expired and the POST
   silently 403s on the JS layer.

## Triage in 5 minutes

1. Open the Grafana **POOOL Wallet** dashboard → "Deposits initiated /
   hour" panel. Confirm the drop is real (not a single missed scrape).
2. Tail backend logs for `/api/wallet/deposits/initiate` 4xx/5xx:
   ```
   grep 'deposits/initiate' /var/log/backend.log | tail -50
   ```
   - Spike of 401/403 → session/CSRF regression.
   - Spike of 400 → input validation regressed (recent deploy?).
   - Spike of 500 → DB or GCS outage.
3. Hit `/wallet` in an incognito browser as a fresh user — does the
   page render? Does the deposit modal open? Can you submit?

## Common causes & fixes

| Symptom | Root cause | Fix |
|---|---|---|
| Modal opens, amount accepted, "Submit" silently no-ops | CSRF token mismatch (cookie cleared mid-session) | Hard refresh; investigate `/auth/csrf` issuing logic |
| Modal won't open | Frontend JS error (broken import) | Check recent deploy; rollback or hotfix |
| Submit returns 503 with "Bank settings missing" | Admin bank settings row deleted/empty | Refill via `/admin/payments/bank-settings` |
| 500s with "GCS upload failed" | Service account creds expired or bucket renamed | Rotate creds; verify `GCS_BUCKET` env |

## Escalation

If pipeline is broken and a fix isn't obvious in 15 min → page
@platform-oncall and post in `#payments-incidents`. Tag
`security: ticket` becomes `severity: page` if combined with the
`WalletDepositSubmitGap` alert (proof-upload pipeline is also down).
