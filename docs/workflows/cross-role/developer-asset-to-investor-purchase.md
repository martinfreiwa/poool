# Developer Asset To Investor Purchase

Purpose: Verify the full asset lifecycle from developer submission through admin review/publication to investor purchase and post-purchase visibility.

Roles: Developer, Admin, Investor.

Primary pages:
- Developer: `/developer/dashboard`, `/developer/add-asset`, `/developer/application-form`, `/developer/document-upload-step3`, `/developer/property-content`, `/developer/submission-success`, `/developer/submissions`, `/developer/asset-detail`, `/developer/assets`, `/developer/ranking`
- Admin: `/admin/developer-submissions`, `/admin/developer-submission-review`, `/admin/assets`, `/admin/asset-details`, `/admin/asset-tokenize`, `/admin/audit-logs`
- Investor: `/marketplace`, `/property/:slug`, `/cart`, `/checkout`, `/payment-in-progress`, `/payment-success`, `/portfolio`, `/transactions`, `/leaderboard`

Prerequisites:
- Local backend and PostgreSQL are running.
- Developer account can create assets.
- Admin account can review developer submissions and manage assets.
- Investor account is email verified, KYC approved, and has enough wallet balance or a supported bank-transfer checkout path.
- Test upload files exist: one valid image, one valid PDF, and one invalid file for validation.
- Use a unique asset name such as `Workflow Test Asset YYYY-MM-DD HHMM`.

Steps:
1. Log in as Developer and open `/developer/dashboard`.
2. Verify dashboard KPIs, review banner state, assets fragment, and navigation links to `/developer/assets`, `/developer/submissions`, and `/developer/add-asset`.
3. Open `/developer/add-asset`.
4. Select the supported real-estate asset type and verify unsupported cards stay disabled or informational.
5. Continue to `/developer/application-form`.
6. Fill required property and financial fields with test-safe values:
   - Property name using the unique workflow prefix.
   - Property type, area, address, city, country, lease type, lease term.
   - Land/building size, bedrooms, bathrooms, status, year built/renovated.
   - Purchase price and minimum share price in integer-compatible values.
7. Try one invalid currency/number value and verify validation blocks save/next without corrupting existing fields.
8. Click `Save & Exit`, reload `/developer/submissions`, and verify the draft appears with the expected status.
9. Reopen the draft and continue to `/developer/document-upload-step3`.
10. Upload all required document categories with valid files.
11. Try one invalid upload and verify the error identifies type/size validation and does not attach the file.
12. Delete one uploaded document and upload it again.
13. Continue to `/developer/property-content`.
14. Fill public listing content, location description, media/gallery, video URL if supported, rental yield, capital appreciation, occupancy, investor profit share, and expected return.
15. Save progress, reload, and verify all fields and uploaded media persist.
16. Submit/tokenize the asset.
17. Verify `/developer/submission-success` explains the review state and links back to dashboard/submissions/support.
18. Open `/developer/submissions` and verify the asset is `submitted` or `in review`.
19. Log out or switch session. Log in as Admin.
20. Open `/admin/developer-submissions` and find the workflow asset by name.
21. Use search/filter/sort controls and verify the asset remains discoverable.
22. Open `/admin/developer-submission-review` for that asset.
23. Verify every submitted field, media item, document, financial metric, and developer identity detail is visible to the admin.
24. Download/open each uploaded document and verify the link is scoped to the admin session.
25. Add admin review notes and, where the UI supports it, upload admin-side supporting documents or images.
26. Test `Request changes` with a disposable note, then verify the Developer sees the requested-change state in `/developer/submissions` and `/developer/asset-detail`.
27. As Developer, apply the requested change and resubmit.
28. As Admin, reopen the review, verify the changed field, then approve the submission.
29. If tokenization is a separate action, open `/admin/asset-tokenize` or the tokenization section and complete the safe local tokenization path.
30. Open `/admin/assets` and `/admin/asset-details` and verify the asset is published/available, has correct token totals, funding status, documents, media, and public slug.
31. Verify `/admin/audit-logs` records submission review, request-changes, resubmit, approve, publish/tokenize actions with the admin/developer actors.
32. Log in as Investor.
33. Open `/marketplace` and search/filter until the workflow asset appears.
34. Open `/property/:slug`.
35. Verify public investor-facing title, gallery, documents, developer links, risk/compliance content, financial tabs, calculator, and amount controls match the approved admin data.
36. Enter an invalid amount below minimum and verify inline validation.
37. Enter a valid amount and add the asset to cart.
38. Open `/cart`, update the amount once, then remove and re-add the asset to verify cart mutations.
39. Proceed to `/checkout`.
40. Accept required disclosures, select wallet or bank-transfer path, and submit payment using disposable local data.
41. Verify the order reaches `/payment-in-progress` or `/payment-success` according to payment method.
42. If bank transfer is pending, log in as Admin and approve the order/deposit in `/admin/orders` or `/admin/deposits`, then return as Investor.
43. Verify `/payment-success`, `/portfolio`, `/transactions`, and `/property/:slug` reflect the purchase.
44. Verify Developer `/developer/dashboard`, `/developer/assets`, and `/developer/asset-detail` show updated funding/investor metrics.
45. Verify Admin `/admin/orders`, `/admin/assets`, and `/admin/treasury` reflect the completed purchase.
46. Run cleanup if this was a disposable local asset: reverse order/holding state or delete fixture rows according to the local cleanup script/runbook.

Expected Result:
- Developer can create, save, upload, submit, revise, and see final approval state.
- Admin can review complete data, request changes, approve, publish/tokenize, and audit the action chain.
- Investor can discover, inspect, add to cart, purchase, and see the asset in portfolio/transactions.
- Developer and admin dashboards update after investor purchase.
- All monetary values are stored and displayed from integer cents without float drift.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | Asset moves draft -> submitted -> review -> approved/published -> purchased. |
| Missing required field | Developer cannot advance and field-level errors are visible. |
| Missing required document | Developer cannot submit or admin sees a clear incomplete state. |
| Invalid upload | File is rejected and no attachment record remains. |
| Request changes | Developer sees admin note, can resubmit, and old review state is not treated as approved. |
| Admin rejection | Asset does not appear in marketplace and developer sees rejected state. |
| Unauthorized developer | Non-owner cannot edit or resubmit the asset. |
| Unauthorized admin | Admin without review permission gets `403` for review APIs/pages. |
| Investor invalid amount | Cart/checkout blocks below-minimum, above-available, sold-out, and annual-limit cases. |
| Duplicate checkout submit | One order/holding is created; repeated submit is idempotent or blocked. |

Cleanup:
- Delete or archive the workflow asset only if no retained test order depends on it.
- Remove uploaded files from local storage/GCS test bucket when supported.
- Revert investor wallet/portfolio/order rows created during the test.
- Keep audit logs unless the local test strategy explicitly truncates disposable audit data.

Backend/API surfaces:
- See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for the complete route-to-workflow mapping.
- Mutating APIs used by this workflow must be verified for authorization, validation, idempotency where applicable, and reload/readback across roles.


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
