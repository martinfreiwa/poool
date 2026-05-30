# Secondary Trading To Admin Marketplace Settlement

Purpose: Verify investor secondary trading, order management, admin marketplace oversight, and settlement/readback.

Roles: Buyer Investor, Seller Investor, Admin.

Primary pages:
- `/portfolio`
- `/marketplace-secondary`
- `/marketplace-trading-v2`
- `/marketplace-trading-v3`
- `/my-trading`
- `/trade-success`
- `/transactions`
- `/admin/marketplace/orders`
- `/admin/marketplace/orderbook`
- `/admin/marketplace/trades`
- `/admin/marketplace/p2p`
- `/admin/marketplace/primary-escrow`
- `/admin/marketplace/reconciliation`
- `/admin/pending-settlements`

Prerequisites:
- Seller investor owns tokens/holdings for a tradable asset.
- Buyer investor has wallet balance.
- Admin can manage marketplace operations.

Steps:
1. As Seller, open `/portfolio` and verify holding is eligible for secondary trading.
2. Open `/marketplace-trading-v3`.
3. Select sell mode, enter invalid price/amount, and verify validation.
4. Submit a valid sell order.
5. Verify `/my-trading` shows the open order.
6. As Buyer, open `/marketplace-secondary` and find the listing.
7. Open `/marketplace-trading-v3`, inspect orderbook/chart/trade history, select buy mode, and submit a matching buy order.
8. Verify `/trade-success` and Buyer `/my-trading` state.
9. As Admin, open `/admin/marketplace/orderbook` and verify orderbook reflects the match/fill.
10. Open `/admin/marketplace/orders` and verify order statuses, grouped user view, filters, pagination, and export controls.
11. Open `/admin/marketplace/trades` and verify trade row, fee cents, buyer/seller, asset, and timestamp.
12. Run or inspect `/admin/marketplace/reconciliation`.
13. Resolve/retry any controlled local mismatch only if using disposable fixtures.
14. Open `/admin/pending-settlements` or `/admin/marketplace/primary-escrow` and run safe local settlement if required.
15. Buyer and Seller reload `/portfolio`, `/transactions`, and `/my-trading`.
16. Verify token/cash balances, held balances, fees, and transaction rows.
17. Cancel a second open order as Seller and verify admin/orderbook state.
18. Verify audit logs for order submit/cancel/admin settlement actions.

Expected Result:
- Orders validate, match, settle, and reconcile without negative balances or self-trades.
- Admin marketplace pages agree with investor pages.

Backend/API surfaces:
- See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for the complete route-to-workflow mapping.
- Mutating APIs used by this workflow must be verified for authorization, validation, idempotency where applicable, and reload/readback across roles.


Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | The workflow reaches the final cross-role state and every role sees the expected state after reload. |
| Authorization boundary | Non-owner or wrong-role direct page/API access returns login redirect, `401`, or explicit `403`. |
| Validation failure | Missing, malformed, stale, duplicate, or out-of-state input is rejected without partial persistence. |
| Audit/readback | Mutating action writes expected audit/DB rows and the next role sees the update only after reload. |
| Cleanup | Disposable `Workflow Test` data can be reverted, archived, or intentionally retained with a note. |

Negative Cases:
- Unauthorized direct page/API access by each non-owner role.
- Missing required fields, invalid state transition, duplicate submit, stale record, and network failure.
- For uploads, invalid file type, oversize file, missing storage object, and inaccessible download link.
- For financial flows, malformed amount, insufficient balance, duplicate approval/settlement, and cents mismatch.


Audit / DB / Financial Checks:
- Verify every admin action writes an audit row with actor, action, target, timestamp, prior/new state where available, and redacted sensitive values.
- Verify all monetary values are stored as integer cents (`BIGINT`/`i64`) and any percentage values use basis points where modeled that way.
- Verify multi-table financial writes are transactional and duplicate submits are idempotent or explicitly blocked.
- Verify uploaded files record MIME type, size, owner/target, storage key/link, access scope, success state, and failed-upload cleanup.
- After every cross-role transition, reload the new role's page and verify the visible state from the database/API, not stale client state.


Cleanup:
- Revert or archive every `Workflow Test` record created by this workflow using approved local cleanup paths.
- Remove temporary uploaded files and downloaded artifacts where policy allows.
- Restore account, wallet, role, feature-flag, notification, and content settings changed during the run.
- Retain audit logs unless the environment is fully disposable and the cleanup runbook explicitly truncates them.
