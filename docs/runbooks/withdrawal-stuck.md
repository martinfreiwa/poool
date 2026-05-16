# Runbook: WalletWithdrawalsStuckSlaBreach

## Alert source

`infra/prometheus/alerts.yml` → `WalletWithdrawalsStuckSlaBreach`

Fires when `wallet_reconciliation_findings{finding="withdrawals_stuck"}`
> 3 for 15 min. That metric is set by the hourly reconciliation worker
(`backend/src/wallet/reconciliation.rs::flag_stuck_withdrawals`) and
counts `withdrawal_requests.status = 'pending'` rows older than the
operational SLA (default 48h).

## What this means

User funds are debited from their wallet (we hold the cash) but the
operations team hasn't pushed the wire. Every minute past SLA erodes
trust and risks a chargeback / complaint. **This is a customer-facing
incident even though nothing is "broken" in code.**

## Triage in 5 minutes

1. Open `/admin/withdrawals` (filter: status=pending, sort by created
   asc). Confirm the count matches the alert.
2. For each stuck row:
   - **Bank details valid?** Holder name matches KYC? IBAN passes
     checksum? If not → reject with reason, refund will fire
     automatically. Tell the user via email outside the system.
   - **KYC fresh?** `users.kyc_status = 'verified'` AND verified
     within compliance window? If stale → block, ask for re-KYC.
   - **Compliance flag?** Check `compliance_alerts` for the same
     `user_id`. If a sanctions/PEP hit is open, withdrawal is
     correctly blocked — comment on alert, hand to compliance.
3. If all three are clean → push the wire through your bank portal,
   then click "Confirm" on the admin page. Reconciliation worker
   clears the stuck flag on the next tick.

## Common causes

| Pattern | What to do |
|---|---|
| One specific user keeps appearing | KYC stale or bank name mismatch — likely the human-review path that's actually working as intended |
| Burst of stuck withdrawals all in the last hour | Ops team away/holiday — page on-call ops |
| Stuck count climbing while wires are clearing on the bank side | Admin "Confirm" UI broken — re-check after recent deploys |
| Same `provider_reference` keeps re-stucking | Confirm endpoint hitting an idempotency conflict; check `idempotency_keys` table for an in-flight reservation older than 24h that needs releasing |

## Escalation

> 10 stuck → page on-call ops AND post in `#payments-incidents`
> 25 stuck → wake compliance lead; we're in regulatory-reportable
territory if total stuck value crosses €/$10k per user-day.

## Recovery

After the wires are pushed and admin confirms each one, the metric
falls on the next reconciliation tick (≤1h). If it doesn't:

```sh
# Force a reconciliation pass:
curl -X POST https://api.poool.com/admin/reconciliation/run \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```
