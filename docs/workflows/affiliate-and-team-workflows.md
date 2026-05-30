# Affiliate And Team Workflows

Purpose: Verify investor affiliate onboarding, legal/compliance pages, referral attribution, payout settings, partner dashboard, admin affiliate operations, and developer affiliate-team lifecycle.

Roles: Public Visitor, Investor, Affiliate, Developer Team Owner, Team Member, Admin.

Primary pages:
- `/affiliate`, `/affiliate/onboarding`, `/affiliate/dashboard`, `/affiliate/referrals`, `/affiliate/materials`, `/affiliate/settings`
- `/affiliate/terms`, `/affiliate/code-of-conduct`, `/affiliate/marketing-materials`, `/affiliate/qualified-referral-payout`, `/affiliate/tax`, `/affiliate/privacy-notice`, `/affiliate/complaints`
- Legacy template aliases: `/affiliate-dashboard`, `/affiliate-onboarding`, `/affiliate-referrals`, `/affiliate-materials`, `/affiliate-marketing-materials`, `/affiliate-settings`, `/affiliate-promo`, `/affiliate-code-of-conduct`, `/affiliate-qualified-referral-payout`, `/affiliate-tax`, `/affiliate-privacy-notice`, `/affiliate-complaints`
- `/rewards`, `/rewards-v2`, `/rewards/:code`, `/r/:code`, `/developer/affiliate-team*`, `/admin/affiliate-applications`, `/admin/affiliate-finance`, `/admin/affiliate-fraud`, `/admin/admin-affiliate-fraud`, `/admin/affiliate-teams`

Backend/API surfaces:
- `backend/src/rewards/**` for referral cookies, affiliate onboarding, payout methods, commissions, materials, and partner dashboards.
- `backend/src/developer/affiliate_team*` and `backend/src/admin/affiliate_teams.rs` for developer team attribution and admin lifecycle.
- `backend/src/admin/rewards.rs` for finance/payout review.

Prerequisites:
- Disposable affiliate applicant, referred investor, developer team owner, and team member accounts.
- KYC/tax state is controlled per test case.
- Bank/tax/payout data uses non-real test identifiers.
- Commissionable checkout fixture is available through cross-role referral workflow.

Steps:
1. As Public Visitor, open affiliate landing/legal pages and referral routes; verify cookies, signup/login CTAs, legal links, and redirect targets.
2. As Investor, complete affiliate onboarding wizard: traffic source, audience, URL, phone, KYC/tax fields, policy acknowledgements, quiz answers, save/back/continue, and submit.
3. As Admin, review affiliate application, approve/reject/request changes, inspect fraud signals, and verify applicant dashboard readback after reload.
4. As Affiliate, use dashboard/referrals/materials/settings: copy/share links, filter/export referrals, download/copy approved materials, configure payout/tax/postback settings, and request payout when eligible.
5. As Public Visitor/Referred Investor, follow referral link, sign up, complete checkout, and verify attribution/commission/payout through `affiliate-referral-checkout-payout.md`.
6. As Developer Team Owner, run team overview/members/customers/products/settings/analytics/tier flows; invite member, approve/remove, export, edit slug/bank data, and verify attribution to team/member.
7. As Admin, inspect developer affiliate teams and finance board; suspend/resume/terminate/move/remove only on disposable teams, then reload team and finance pages.

Expected Result:
- Referral and team attribution survive signup/checkout and are visible to affiliate, developer, and admin finance views.
- Payout/tax/bank mutations validate input, audit changes, and never expose sensitive identifiers beyond masked display.
- Legal/compliance pages remain read-only references and onboarding requires policy acceptance.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Referral cookie | Code persists through signup without cross-user leakage. |
| Affiliate onboarding | Required compliance/tax/KYC steps block incomplete submit. |
| Admin application review | Approval/rejection changes applicant dashboard after reload and audits admin. |
| Partner dashboard/materials | Copy/download/export controls work and respect approval status. |
| Payout settings/request | Bank/tax/postback validation persists safe values and masks sensitive data. |
| Developer team lifecycle | Member attribution, exports, analytics, tier, and admin finance views reconcile. |
| Fraud/legal pages | Read-only pages and fraud visualizer routes are classified without mutating data. |

Negative Cases:
- Invalid referral code, expired cookie, self-referral, duplicate attribution, or referral overwrite.
- Affiliate without approved state opens materials/payout pages.
- Invalid bank/tax/postback URL or tax document upload.
- Team slug collision, unauthorized member approval/removal, moving member across teams without admin permission.
- Payout request below threshold or duplicate payout approval.

Audit / DB / Financial Checks:
- Commission, payable, paid, payout request, and order attribution amounts are integer cents.
- Admin affiliate application, payout, fraud, and team lifecycle actions write audit logs.
- Tax document uploads validate type/size and store private links.
- Attribution rows connect referral code, referred user, order, affiliate/team/member, commission, and payout state.

Cleanup:
- Remove or mark disposable applications, teams, invitations, referral cookies, commissions, payout requests, and uploads.
- Revert payout settings/postbacks and team bank data.
- Retain finance/audit rows unless local policy allows truncation.
