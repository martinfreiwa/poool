# E2E_COVERAGE_TRACKER.md
**Last Deep Audit:** 2026-03-23 (Full Source-of-Truth Cross-Reference)
**Framework:** pytest-playwright (Strict Native API)
**Total Tracked Flows:** 130+ (all HTML pages + all API workflows)

---

## 1. Required Environment & Extensions
```bash
pip install pytest-playwright pytest-base-url psycopg2-binary
playwright install --with-deps
```

*Detected requirements rationale:* 
- **`pytest-playwright`**: Core E2E framework requirement.
- **`pytest-base-url`**: Deduced from `os.environ.get("BASE_URL")` usage.
- **`psycopg2-binary`**: Used in `conftest.py` and `test_admin_financials.py`.

---

## 2. Core User Flows (Investor & Developer Dashboards)
All screens a regular investor or issuer can see are rigidly tracked below. Cross-referenced against every `.html` file in `frontend/platform/` and every backend route in `src/*/mod.rs`.

### 2.1 Auth, Onboarding & Recovery
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-1.01** | `/auth/signup` → `signup.html` | Account Creation | 🖥️📱 | ⚠️ Flaky | Brittle CSS. Lacks `page.route` checks. |
| **F-1.02** | `/auth/login` → `login.html` | Login (Email+PW) | 🖥️📱 | ⚠️ Flaky | Needs `expect_response` intercept. |
| **F-1.03** | `/auth/google` | Google OAuth Redirect | 🖥️📱 | ⚪ Planned | Mock `page.route` for Google consent screen. |
| **F-1.04** | `/auth/google/callback` | OAuth Callback | 🖥️📱 | ⚪ Planned | Validate redirect logic after Google auth. |
| **F-1.05** | `/auth/forgot-password` → `forgot-password.html` | Password Recovery | 🖥️📱 | ⚪ Planned | Missing email token interception tests. |
| **F-1.06** | `/auth/reset-password` → `reset-password.html` | Set New Password | 🖥️📱 | ⚪ Planned | Token validation + form submit. |
| **F-1.07** | `/auth/verify-email` → `verify-email.html` | Email Verification | 🖥️📱 | ⚪ Planned | Verify token acceptance logic. |
| **F-1.08** | `/auth/resend-verification` | Resend Email (API) | 🖥️📱 | ⚪ Planned | POST action from verify page. |
| **F-1.09** | `/auth/2fa/setup` → `auth-2fa-setup.html` | TOTP Setup | 🖥️📱 | ⚪ Planned | QR code render + first code validation. |
| **F-1.10** | `/auth/2fa` → `auth-2fa.html` | TOTP Verify | 🖥️📱 | ⚪ Planned | Code entry during login redirect. |
| **F-1.11** | `/auth/2fa/step-up` | Financial Step-Up 2FA (API) | 🖥️📱 | ⚪ Planned | JSON API for withdrawal/trade re-auth. |
| **F-1.12** | `/kyc` → `kyc.html` | KYC Initiation | 🖥️📱 | ⚪ Planned | Didit.me fallback simulation. |
| **F-1.13** | `/welcome` → `welcome.html` | Post-Signup Welcome | 🖥️📱 | ⚪ Planned | Onboarding rendering. |
| **F-1.14** | `/logout` | Session Destruction | 🖥️📱 | ⚪ Planned | Cookie cleared + redirect to login. |

### 2.2 Investor Dashboard, Portfolio & Wallet
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-2.00** | `/` → `index.html` | Dashboard Landing | 🖥️📱 | ⚪ Planned | Root widget loading for logged-in users. |
| **F-2.01** | `/portfolio` → `portfolio.html` | Master Portfolio | 🖥️📱 | ⚪ Planned | Charts, asset array, total yield render. |
| **F-2.02** | `/wallet` → `wallet.html` | Core Wallet Hub | 🖥️📱 | ⚪ Planned | Fiat vs Rewards vs Locked display. |
| **F-2.03** | `/wallet/deposit` (POST via wallet) | Fiat Deposit | 🖥️📱 | ⚪ Planned | Payment gateway simulated injections. |
| **F-2.04** | `/wallet/withdraw` (POST via wallet) | Fiat Withdrawal | 🖥️📱 | ⚪ Planned | Min balances, AML holds, 2FA step-up. |
| **F-2.05** | `/api/wallet/balance` | Balance API | 🖥️📱 | ⚪ Planned | JSON response correctness check. |
| **F-2.06** | `/api/wallet/transactions` | Tx History API | 🖥️📱 | ⚪ Planned | Pagination and filtering. |
| **F-2.07** | `/transactions` → `transactions.html` | Ledger History UI | 🖥️📱 | ⚪ Planned | Table paging and date range filters. |
| **F-2.08** | `/rewards` → `rewards.html` | Referral Program | 🖥️📱 | ⚪ Planned | Referral bonuses, loyalty point arrays. |
| **F-2.09** | `/rewards-v2` → `rewards-v2.html` | Premium Rewards UI | 🖥️📱 | ⚪ Planned | V2 layout + campaign progress bars. |
| **F-2.10** | `/rewards/:code`, `/r/:code` | Referral Landing | 🖥️📱 | ⚪ Planned | Cookie set + redirect to signup. |
| **F-2.11** | `/tier` | Tier Info Page | 🖥️📱 | ⚪ Planned | User tier level display. |
| **F-2.12** | `/my-trading` → `my-trading.html` | Open Orders Panel | 🖥️📱 | ⚪ Planned | Unfulfilled maker/taker positions. |
| **F-2.13** | `/api/marketplace/trades/mine` | Trade History API | 🖥️📱 | ⚪ Planned | Display settled user trades on My Trading. |

### 2.3 Marketplace & Asset Discovery
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-3.01** | `/marketplace` → `marketplace.html` | Launchpad (RE) | 🖥️📱 | ✅ Solid | Available/Funded tab switching tested. |
| **F-3.02** | `/marketplace-secondary` → `marketplace-secondary.html` | P2P Secondary Market | 🖥️📱 | ⚪ Planned | Pre-owned fractional shares. |
| **F-3.03** | `/commodities-marketplace` → `commodities-marketplace.html` | Commodities Catalog | 🖥️📱 | ⚪ Planned | Gold/Art sorting, filtering. |
| **F-3.04** | `/marketplace-trading-v2` → `marketplace-trading-v2.html` | Trading Pro v2 | 🖥️📱 | ⚪ Planned | Candlestick charts + orderbook. |
| **F-3.05** | `/marketplace-trading-v3` → `marketplace-trading-v3.html` | Trading Pro v3 | 🖥️📱 | ⚪ Planned | Advanced WebSocket live feed. |
| **F-3.06** | `/property/{slug}` → `property.html` | Asset Detail (RE) | 🖥️📱 | ⚠️ Flaky | 5 tabs (Overview/Financials/Docs/Location/Reviews). |
| **F-3.07** | `/commodity/{slug}` → `commodity.html` | Asset Detail (Alt) | 🖥️📱 | ⚪ Planned | Non-real-estate overrides. |
| **F-3.08** | `/cart` → `cart.html` | Shopping Basket | 🖥️📱 | ⚠️ Flaky | Missing cart expiration logic. |
| **F-3.09** | `/checkout` → `checkout.html` | Checkout Engine | 🖥️📱 | ⚪ Planned | Platform fee calculation, wallet deduction. |
| **F-3.10** | `/payment-in-progress` → `payment-in-progress.html` | Pending Gate | 🖥️📱 | ⚪ Planned | 3D-Secure / bank transfer wait state. |
| **F-3.11** | `/payment-success` → `payment-success.html` | Post-Purchase | 🖥️📱 | ⚪ Planned | Redirect + success render. |
| **F-3.12** | `/ws/market/:asset_id` | WebSocket Feed | 🖥️ | ⚪ Planned | Real-time orderbook/trade stream. |
| **F-3.13** | `/api/marketplace/:id/orderbook` | Orderbook API | 🖥️ | ⚪ Planned | Bid/ask array correctness. |
| **F-3.14** | `/api/marketplace/:id/candles` | Candlestick API | 🖥️ | ⚪ Planned | OHLCV data validation. |
| **F-3.15** | `/api/marketplace/orders` (POST) | Submit Order | 🖥️ | ⚪ Planned | Limit/market order placement. |
| **F-3.16** | `/api/marketplace/p2p/offers` | P2P Offers (CRUD) | 🖥️ | ⚪ Planned | Create/cancel/respond to P2P offers. |
| **F-3.17** | `/api/assets/search` | Global Asset Search | 🖥️📱 | ⚪ Planned | Sidebar auto-complete filtering exclusively for assets. |
| **F-3.18** | `/property/{slug}` Gallery | Secure Image Galleries| 🖥️📱 | ⚪ Planned | Validate GCS signed URL generation and carousel indicators. |

### 2.4 Developer (Issuer) Dashboard
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-4.01** | `/developer` → redirect → `/developer/dashboard` → `developer/dashboard.html` | Issuer Hub | 🖥️ | ⚪ Planned | Stats, KPIs, HTMX fragment charts. |
| **F-4.02** | `/developer/assets` → `developer/assets.html` | Live Campaigns | 🖥️ | ⚪ Planned | Funded campaigns listing. |
| **F-4.03** | `/developer/asset-detail` → `developer/asset-detail.html` | Asset Perf | 🖥️ | ⚪ Planned | Issuer-specific insights. |
| **F-4.04** | `/developer/submissions` → `developer/submissions.html` | Draft Status | 🖥️ | ⚪ Planned | Accept/deny tracking. |
| **F-4.05** | `/developer/add-asset` → `developer/add-asset.html` | Application Init | 🖥️ | ⚪ Planned | Start fresh draft. |
| **F-4.06** | `/developer/application-form` → `developer/application-form.html` | Meta + Financials | 🖥️ | ⚪ Planned | Multi-field form submission. |
| **F-4.07** | `/developer/property-content` → `developer/property-content.html` | Rich Content | 🖥️ | ⚪ Planned | WYSIWYG editor + image gallery. |
| **F-4.08** | `/developer/document-upload-step3` → `developer/document-upload-step3.html` | Legal Docs | 🖥️ | ⚪ Planned | PDF/Doc upload via signed GCS. |
| **F-4.09** | `/developer/submission-success` → `developer/submission-success.html` | Completion | 🖥️ | ⚪ Planned | Confirmation + redirect. |
| **F-4.10** | `/developer/settings` → `developer/settings.html` | Issuer Org Setup | 🖥️ | ⚪ Planned | Org profile, payout bank nodes. |
| **F-4.11** | `/api/developer/draft` (CRUD) | Draft API | 🖥️ | ⚪ Planned | Create/update/delete/submit/duplicate. |
| **F-4.12** | `/api/developer/assets/:id` (PUT) | Change Request | 🖥️ | ⚪ Planned | Edit live asset → admin review. |

### 2.5 Settings, Privacy & General Utility
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-5.01** | `/settings` → `settings.html` (redirects to V2) | Profile Core | 🖥️📱 | ⚠️ Flaky | Name update. Needs `expect_response`. |
| **F-5.02** | `settings-2.html` | Extended Prefs | 🖥️📱 | ⚪ Planned | Locale, currency, timezone. |
| **F-5.03** | `/api/settings/profile` (POST) | Profile Update API | 🖥️📱 | ⚪ Planned | First/last name, avatar. |
| **F-5.04** | `/api/settings/password` (POST) | Password Change | 🖥️📱 | ⚪ Planned | Old vs new collision check. |
| **F-5.05** | `/api/settings/email` (POST) | Email Change | 🖥️📱 | ⚪ Planned | Re-verification trigger. |
| **F-5.06** | `/api/settings/phone` (POST) | Phone Change | 🖥️📱 | ⚪ Planned | Phone number update. |
| **F-5.07** | `/api/settings/preferences` (POST) | Timezone/Language | 🖥️📱 | ⚪ Planned | Locale switch, UI refresh. |
| **F-5.08** | `/api/settings/notifications` (POST) | Alert Prefs | 🖥️📱 | ⚪ Planned | Email/push notification toggles. |
| **F-5.09** | `/api/settings/leaderboard` (POST) | Privacy Rules | 🖥️📱 | ✅ Solid | Anonymization tested. |
| **F-5.10** | `/api/settings/2fa/disable` (POST) | 2FA Removal | 🖥️📱 | ⚪ Planned | Disable TOTP flow. |
| **F-5.11** | `/api/settings/export-data` (GET) | GDPR Export | 🖥️📱 | ⚪ Planned | JSON data bundle download. |
| **F-5.12** | `/api/settings/delete-account` (POST) | GDPR Delete | 🖥️📱 | ⚪ Planned | Full account purge. |
| **F-5.13** | `/account-deletion` → `account-deletion.html` | Deletion Page | 🖥️📱 | ⚪ Planned | Confirmation UI. |
| **F-5.14** | `/gdpr-data-request` → `gdpr-data-request.html` | Data Request | 🖥️📱 | ⚪ Planned | Request form render. |
| **F-5.15** | `/support` → `support.html` | Ticket System | 🖥️📱 | ⚪ Planned | Create/list user tickets. |
| **F-5.16** | `/support/ticket` → `support-ticket.html` (admin serves) | Chat View | 🖥️📱 | ⚪ Planned | Thread reply functionality. |
| **F-5.17** | `/api/payment-methods` (CRUD) | Bank/Card Mgmt | 🖥️📱 | ⚪ Planned | Add/remove/default payment methods. |
| **F-5.18** | `403.html`, `404.html`, `500.html` | Error Pages | 🖥️📱 | ⚪ Planned | Stylized error state snapshots. |
| **F-5.19** | `maintenance.html` | Maintenance Mode | 🖥️📱 | ⚪ Planned | System-offline block check. |
| **F-5.20** | `/profile` | Profile Redirect | 🖥️📱 | ⚪ Planned | Redirect to KYC/settings. |

### 2.6 Community & Social Platform
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-6.01** | `/community` → `community.html` | Main Feed | 🖥️📱 | ⚪ Planned | Post creation, hashtag filtering. |
| **F-6.02** | `/community/post/:id` | Post Thread SSR | 🖥️📱 | ⚪ Planned | Comment thread, reactions, report. |
| **F-6.03** | `/api/community/feed` | Feed API | 🖥️📱 | ⚪ Planned | Pagination, filter by circle/asset. |
| **F-6.04** | `/api/community/posts` (CRUD) | Post CRUD | 🖥️📱 | ⚪ Planned | Create/update/delete user posts. |
| **F-6.05** | `/api/community/posts/:id/reactions` | Toggle Reactions | 🖥️📱 | ⚪ Planned | Like/fire/rocket toggles. |
| **F-6.06** | `/api/community/posts/:id/comments` | Comments | 🖥️📱 | ⚪ Planned | Thread create + get. |
| **F-6.07** | `/api/community/posts/:id/report` | Report Content | 🖥️📱 | ⚪ Planned | Flag post to admin. |
| **F-6.08** | `/api/community/posts/:id/bookmark` | Bookmarks (UX.6) | 🖥️📱 | ⚪ Planned | Saved posts toggle. |
| **F-6.09** | `/api/community/posts/:id/poll/vote` | Polls (UX.11) | 🖥️📱 | ⚪ Planned | Native poll voting. |
| **F-6.10** | `/api/community/hashtags/trending` | Hashtags (UX.4) | 🖥️📱 | ⚪ Planned | Trending tag array. |
| **F-6.11** | `/api/community/search` | Community Search | 🖥️📱 | ⚪ Planned | User + post search. |
| **F-6.12** | `/api/community/profile/*` | Social Profiles | 🖥️📱 | ⚪ Planned | Bio, follower count, badges. |
| **F-6.13** | `/api/community/follow/:id` | Follow/Unfollow | 🖥️📱 | ⚪ Planned | Social graph mutations. |
| **F-6.14** | `/api/community/circles` (CRUD) | Circles System | 🖥️📱 | ⚪ Planned | Create/join/leave/invite/kick. |
| **F-6.15** | `/api/community/circles/:id/token-gate` | Token-Gated (W3.1) | 🖥️📱 | ⚪ Planned | Holdings-based access check. |
| **F-6.16** | `/api/community/invites` | Circle Invites | 🖥️📱 | ⚪ Planned | Accept/decline invitations. |
| **F-6.17** | `/api/community/xp` | XP System (M4) | 🖥️📱 | ⚪ Planned | Level, total XP, history. |
| **F-6.18** | `/api/community/notifications` | In-App Alerts | 🖥️📱 | ⚪ Planned | Unread count + mark-all-read. |
| **F-6.19** | `/api/community/challenges` | Challenges (M5) | 🖥️📱 | ⚪ Planned | Active challenge listing. |
| **F-6.20** | `/api/community/amas` | Expert AMAs (M5) | 🖥️📱 | ⚪ Planned | AMA events + Q&A. |
| **F-6.21** | `/api/community/assets/:id/reviews` | Property Reviews | 🖥️📱 | ⚪ Planned | Star rating + text review. |
| **F-6.22** | `/api/community/appeals` | Ban Appeals | 🖥️📱 | ⚪ Planned | User submits appeal to admin. |
| **F-6.23** | `/leaderboard` → `leaderboard.html` | Public Ranks | 🖥️📱 | ⚪ Planned | Anonymized users = "Hidden". |
| **F-6.24** | `/api/community/profile/me` (Auto) | Auto-Onboarding | 🖥️📱 | ⚪ Planned | Ensure profile creation on first hit. |
| **F-6.25** | `community.html` Sidebar Card | Dynamic Profile Card | 🖥️📱 | ⚪ Planned | Verify name/bio/stats injection. |

### 2.7 Blog
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-7.01** | `/blog` → `blog/index.html` | Blog Index | 🖥️📱 | ⚪ Planned | Article listing render. |
| **F-7.02** | `/blog/:slug` → `blog/article.html` | Article Detail | 🖥️📱 | ⚪ Planned | Rich content rendering. |
| **F-7.03** | `/blog/category/:slug` | Category Filter | 🖥️📱 | ⚪ Planned | Filtered article list. |

### 2.8 Legal & Regulatory Pages
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **F-8.01** | `/terms` → `terms.html` | T&C | 🖥️📱 | ⚪ Planned | Snapshot test. |
| **F-8.02** | `/privacy-policy` → `privacy-policy.html` | Privacy Statement | 🖥️📱 | ⚪ Planned | Snapshot test. |
| **F-8.03** | `/imprint` → `imprint.html` | Imprint | 🖥️📱 | ⚪ Planned | Snapshot test. |
| **F-8.04** | `/currency-policy` → `currency-policy.html` | FX Rate | 🖥️📱 | ⚪ Planned | Snapshot test. |
| **F-8.05** | `/aml-kyc-policy` → `aml-kyc-policy.html` | AML/KYC | 🖥️📱 | ⚪ Planned | Snapshot test. |
| **F-8.06** | `/cookies` → `cookies.html` | Cookie Consent | 🖥️📱 | ⚪ Planned | Consent trigger fires. |
| **F-8.07** | `/api/user/legal-status` | Legal Acceptance | 🖥️📱 | ⚪ Planned | T&C version check. |

---

## 3. Admin & Operational Flows (Full Coverage Map)
Cross-referenced against **61 admin HTML files** and **100+ admin API endpoints** from `backend/src/admin/mod.rs`.

### 3.1 Admin Core & System
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-1.01** | `/admin/` → `admin/index.html` | Dashboard Hub | 🖥️ | ⚪ Planned | Stats overview widgets. |
| **A-1.02** | `/admin/admins.html` | Staff Management | 🖥️ | ⚪ Planned | Invite, block, remove admins. |
| **A-1.03** | `/admin/roles.html` | RBAC Roles | 🖥️ | ⚪ Planned | Create roles, assign permissions. |
| **A-1.04** | `/admin/audit-logs.html` | Audit Trails | 🖥️ | ⚪ Planned | Filter by IP, UUID, date. |
| **A-1.05** | `/admin/system.html` | System Health | 🖥️ | ⚪ Planned | Env config render. |
| **A-1.06** | `/admin/reports.html` | Reports & Export | 🖥️ | ⚪ Planned | CSV/PDF downloads. |
| **A-1.07** | `/admin/settings.html` | Global Config | 🖥️ | ⚪ Planned | Fee limits, maintenance toggle. |
| **A-1.08** | `/admin/storage.html` | GCS Buckets | 🖥️ | ⚪ Planned | Storage analytics. |

### 3.2 Users, KYC & Financials
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-2.01** | `/admin/users.html` | User CRM | 🖥️ | ⚪ Planned | Sort, paginate, search. |
| **A-2.02** | `/admin/user-details.html` | Deep CRM View | 🖥️ | ⚠️ Flaky | `wait_for_timeout` violation. |
| **A-2.03** | `/admin/kyc.html` | KYC Ops | 🖥️ | ⚪ Planned | Approve/Reject docs. |
| **A-2.04** | `/admin/deposits.html` | Deposits | 🖥️ | ⚠️ Flaky | Timeout issue. |
| **A-2.05** | `/admin/orders.html` | Orders Grid | 🖥️ | ⚪ Planned | Order detail modal. |
| **A-2.06** | `/admin/pending-settlements.html` | Unsettled Queue | 🖥️ | ⚪ Planned | Settlement status checks. |
| **A-2.07** | `/admin/treasury.html` | System Treasury | 🖥️ | ⚪ Planned | Global balance overview. |
| **A-2.08** | `/admin/rewards.html` | Rewards Pool | 🖥️ | ⚪ Planned | Point generation/deduction. |
| **A-2.09** | API: `/api/admin/withdrawals` | Withdrawals | 🖥️ | ⚪ Planned | Approve/reject workflows. |
| **A-2.10** | API: `/api/admin/investments` | Investments | 🖥️ | ⚪ Planned | Cross-reference check. |
| **A-2.11** | API: `/api/admin/disputes` | Disputes Mgmt | 🖥️ | ⚪ Planned | Status update logic. |
| **A-2.12** | API: `/api/admin/tax-reports` | Tax & Fiscal | 🖥️ | ⚪ Planned | Report generation. |

### 3.3 Blockchain, Web3 & Tokenization
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-3.01** | `/admin/asset-tokenize.html` | Tokenization | 🖥️ | ⚪ Planned | Smart contract deploy. |
| **A-3.02** | `/admin/blockchain-contracts.html` | Contracts List | 🖥️ | ⚪ Planned | Deployed instances. |
| **A-3.03** | `/admin/blockchain-contract-detail.html` | Contract Ops | 🖥️ | ⚪ Planned | Pause/unpause/force transfer. |
| **A-3.04** | `/admin/blockchain-sync.html` | Indexer State | 🖥️ | ⚪ Planned | Sync status + force KYC sync. |
| **A-3.05** | `/admin/blockchain-treasury.html` | On-Chain Wallets | 🖥️ | ⚪ Planned | Cold storage mapping. |
| **A-3.06** | API: `/api/admin/blockchain/pin-metadata` | IPFS Pin | 🖥️ | ⚪ Planned | Pinata metadata pinning. |

### 3.4 Asset Publishing & Issuer Submissions
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-4.01** | `/admin/assets.html` | Catalog Mgmt | 🖥️ | ⚪ Planned | Toggle featured, publish. |
| **A-4.02** | `/admin/asset-details.html` | Asset Deep View | 🖥️ | ⚪ Planned | Specs, cap table. |
| **A-4.03** | `/admin/developer-submissions.html` | Submission Inbox | 🖥️ | ⚪ Planned | Review queue. |
| **A-4.04** | `/admin/developer-submission-review.html` | Review Logic | 🖥️ | ⚪ Planned | Approve/reject + notes + checklist. |
| **A-4.05** | `/admin/asset-change-requests.html` | Edit Queue | 🖥️ | ⚪ Planned | Issuer change requests. |
| **A-4.06** | `/admin/asset-change-review.html` | Change Review | 🖥️ | ⚪ Planned | Diff comparison. |
| **A-4.07** | `/admin/dividends.html` | Dividend Payouts | 🖥️ | ⚪ Planned | Create/approve/execute distributions. |
| **A-4.08** | `/admin/approvals.html` | Unified Approvals | 🖥️ | ⚪ Planned | Central approval queue. |

### 3.5 Marketplace Admin Hub
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-5.01** | `/admin/marketplace/index.html` | Mkpt Dashboard | 🖥️ | ⚪ Planned | Stats overview. |
| **A-5.02** | `/admin/marketplace/orders.html` | Order List | 🖥️ | ⚪ Planned | Cancel orders. |
| **A-5.03** | `/admin/marketplace/orderbook.html` | Orderbook View | 🖥️ | ⚪ Planned | Bid/ask rendering + rebuild. |
| **A-5.04** | `/admin/marketplace/p2p.html` | P2P Offers | 🖥️ | ⚪ Planned | OTC activity monitoring. |
| **A-5.05** | `/admin/marketplace/trades.html` | Settled Trades | 🖥️ | ⚪ Planned | Executed match history. |
| **A-5.06** | `/admin/marketplace/primary-escrow.html` | Escrow Tracking | 🖥️ | ⚪ Planned | Held fund status. |
| **A-5.07** | `/admin/marketplace/reconciliation.html` | Ledger Recon | 🖥️ | ⚪ Planned | DB vs chain mismatch. |
| **A-5.08** | `/admin/marketplace/alerts.html` | Risk Alerts | 🖥️ | ⚪ Planned | Wash trade / price flags. |
| **A-5.09** | `/admin/marketplace/analytics.html` | Liquidity Stats | 🖥️ | ⚪ Planned | Volume charts. |
| **A-5.10** | `/admin/marketplace/compliance.html` | Compliance | 🖥️ | ⚪ Planned | OJK reports, travel rule, tax. |
| **A-5.11** | `/admin/marketplace/fees.html` | Fee Config | 🖥️ | ⚪ Planned | Maker/taker ratios. |
| **A-5.12** | `/admin/marketplace/approvals.html` | Trade Approvals | 🖥️ | ⚪ Planned | Manual order approval/reject. |
| **A-5.13** | `/admin/marketplace/settings.html` | Mkpt Settings | 🖥️ | ⚪ Planned | Toggle trading, kill switch. |
| **A-5.14** | API: `/api/admin/marketplace/health` | Engine Health | 🖥️ | ⚪ Planned | Matching engine status. |
| **A-5.15** | API: `/api/admin/marketplace/watchlist` | Watchlist | 🖥️ | ⚪ Planned | Add/view suspicious accounts. |

### 3.6 Community Admin
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-6.01** | `/admin/community/index.html` | Community Hub | 🖥️ | ⚪ Planned | Activity overview. |
| **A-6.02** | `/admin/community/users.html` | User Moderation | 🖥️ | ⚪ Planned | Ban/mute/shadowban/warn. |
| **A-6.03** | `/admin/community/user-detail.html` | User Deep View | 🖥️ | ⚪ Planned | XP history, posts, badges. |
| **A-6.04** | `/admin/community/circles.html` | Circles Array | 🖥️ | ⚪ Planned | List + admin delete. |
| **A-6.05** | `/admin/community/circle-detail.html` | Circle Ops | 🖥️ | ⚪ Planned | Transfer, kick, update. |
| **A-6.06** | `/admin/community/posts.html` | Post Moderation | 🖥️ | ⚪ Planned | Search + hide/lock posts. |
| **A-6.07** | `/admin/community/post-detail.html` | Post Deep View | 🖥️ | ⚪ Planned | Comments, reactions, reports. |
| **A-6.08** | `/admin/community/comments.html` | Comment Mod | 🖥️ | ⚪ Planned | Delete/hide/pin comments. |
| **A-6.09** | `/admin/community/reports.html` | Flag Queue | 🖥️ | ⚪ Planned | Process user-submitted reports. |
| **A-6.10** | `/admin/community/leaderboard.html` | XP/Rank Admin | 🖥️ | ⚪ Planned | Award/deduct XP manually. |
| **A-6.11** | `/admin/community/challenges.html` | Challenge Mgmt | 🖥️ | ⚪ Planned | Create/toggle challenges. |
| **A-6.12** | `/admin/community/badges.html` | Badge System | 🖥️ | ⚪ Planned | Create/grant/revoke badges. |
| **A-6.13** | `/admin/community/amas.html` | AMA Events | 🖥️ | ⚪ Planned | Create AMAs, answer Qs. |
| **A-6.14** | `/admin/community/announcements.html` | Broadcasts | 🖥️ | ⚪ Planned | Site-wide banner pushes. |
| **A-6.15** | API: `/api/admin/community/appeals` | Ban Appeals | 🖥️ | ⚪ Planned | Review/approve ban appeals. |
| **A-6.16** | API: `/api/admin/community/audit-log` | Community Audit | 🖥️ | ⚪ Planned | Moderation action trail. |

### 3.7 Support & Communication
| Flow ID | Target Route / HTML File | Feature Scope | Viewports | Status | Action Items |
|---|---|---|---|---|---|
| **A-7.01** | `/admin/support.html` | Ticket Inbox | 🖥️ | ⚪ Planned | Assign, prioritize, bulk ops. |
| **A-7.02** | `/admin/support-ticket.html` | Chat Thread | 🖥️ | ✅ Solid | Reply, sync UI, update status. |
| **A-7.03** | `/admin/notifications.html` | Push to Users | 🖥️ | ⚪ Planned | Broadcast notifications. |
| **A-7.04** | `/admin/email-marketing.html` | Email Campaigns | 🖥️ | ⚪ Planned | Template CRUD + campaign send. |

### 3.8 Admin API-Only Workflows (No Dedicated HTML Page)
| Flow ID | API Route | Feature Scope | Status | Action Items |
|---|---|---|---|---|
| **A-8.01** | `/api/admin/maintenance/clear-cache` | Cache Bust | ⚪ Planned | Redis flush. |
| **A-8.02** | `/api/admin/maintenance/rotate-logs` | Log Rotation | ⚪ Planned | Ops call. |
| **A-8.03** | `/api/admin/debug/seed` | Debug Seed | ⚪ Planned | Test data generation. |
| **A-8.04** | `/api/admin/legal/version` | T&C Versioning | ⚪ Planned | Bump legal version. |
| **A-8.05** | `/api/admin/users/:id/force-password-reset` | Force PW Reset | ⚪ Planned | Admin-triggered. |
| **A-8.06** | `/api/admin/users/:id/sessions` (DELETE) | Revoke Sessions | ⚪ Planned | Kill user sessions. |
| **A-8.07** | `/api/admin/users/:id/roles` (POST) | Role Assignment | ⚪ Planned | Assign RBAC roles. |
| **A-8.08** | `/api/blog/admin/articles` (CRUD) | Blog CMS | ⚪ Planned | Create/edit/publish/delete articles. |
| **A-8.09** | `/api/admin/primary-escrow` | Escrow List API | ⚪ Planned | Primary offering funds. |

---

## 4. Playwright Strictness Audit
| Violation Type | Location | Description |
|---|---|---|
| `wait_for_timeout` | `test_admin_financials.py` | Hardcoded `2000ms` waits. Replace with `expect()`. |
| `networkidle` | All test files | Anti-pattern. Use `expect_response` or `wait_for_url`. |
| Python `assert` | `test_admin_financials.py` | Use Playwright `expect().to_have_text()` instead. |
| Brittle selectors | All test files | `page.locator("#id")` → use `page.get_by_role()` / `page.get_by_text()`. |
| Missing `page.route()` | All test files | No network interception for validation. |
| Missing mobile | Most tests | Only desktop 1280x800. Need 375x812 parameterization. |

---

## 5. Coverage Summary

| Section | Total Flows | ✅ Solid | ⚠️ Flaky | ⚪ Planned |
|---|---|---|---|---|
| 2.1 Auth & Recovery | 14 | 0 | 2 | 12 |
| 2.2 Portfolio & Wallet | 14 | 0 | 0 | 14 |
| 2.3 Marketplace | 18 | 1 | 2 | 15 |
| 2.4 Developer Dashboard | 12 | 0 | 0 | 12 |
| 2.5 Settings & Utility | 20 | 1 | 1 | 18 |
| 2.6 Community | 25 | 0 | 0 | 25 |
| 2.7 Blog | 3 | 0 | 0 | 3 |
| 2.8 Legal Pages | 7 | 0 | 0 | 7 |
| 3.1 Admin Core | 8 | 0 | 0 | 8 |
| 3.2 Admin Users/Finance | 12 | 0 | 1 | 11 |
| 3.3 Admin Blockchain | 6 | 0 | 0 | 6 |
| 3.4 Admin Assets | 8 | 0 | 0 | 8 |
| 3.5 Admin Marketplace | 15 | 0 | 0 | 15 |
| 3.6 Admin Community | 16 | 0 | 0 | 16 |
| 3.7 Admin Support | 4 | 1 | 0 | 3 |
| 3.8 Admin API-Only | 9 | 0 | 0 | 9 |
| **TOTAL** | **191** | **3** | **6** | **182** |

---

## 6. Automatic Quality Checks (Built Into Every Test)
Every test fixture (`quality_page`, `authenticated_user_page`, `admin_page`, `mobile_page`, `tablet_page`) automatically includes the `PageQualityTracker` and post-test hooks.

### 6.1 What Gets Monitored Automatically
| # | Check | How It Works | Auto-Fail? |
|---|---|---|---|
| 1 | **Console Errors** | `page.on("console")` captures ALL `console.error` | ✅ via `.assert_no_critical_errors()` |
| 2 | **Uncaught JS Exceptions** | `page.on("pageerror")` catches unhandled throws | ✅ Always critical |
| 3 | **Console Warnings** | `page.on("console")` captures `console.warn` | ❌ Logged only |
| 4 | **HTTP 4xx/5xx** | `page.on("response")` tracks every response | ✅ via `.assert_no_network_failures()` |
| 5 | **Resource Load Failures** | `page.on("requestfailed")` — DNS/CORS/network | ❌ Logged for review |
| 6 | **Server 500 on Navigate** | `tracker.navigate_and_check()` checks status | ✅ Immediate fail |
| 7 | **Unexpected Login Redirect** | Detects auth-redirect to `/auth/login` | ✅ Immediate fail |
| 8 | **Blank Page Detection** | `assert_page_loaded()` — body has ≥ 10 chars | ✅ Immediate fail |
| 9 | **Broken Images** | `assert_no_broken_images()` — `naturalWidth === 0` | ✅ On demand |
| 10 | **Basic A11y** | `assert_basic_a11y()` — title, lang, h1, duplicate IDs | ✅ On demand |
| 11 | **Page Load Time** | `time.time()` delta per URL | ❌ Logged in report |
| 12 | **Screenshot on Failure** | `pytest_runtest_makereport` hook | Auto-saved `FAIL_*.png` |
| 13 | **Playwright Trace** | `context.tracing.start()` — screenshots+snapshots+sources | Auto-saved `FAIL_*.zip` |
| 14 | **JSON Quality Report** | Full report per test | Auto-saved `*.json` |

### 6.2 Pre-Flight Checks
| Check | What It Does |
|---|---|
| **Backend Health Gate** | Session-scoped. Hits `/health` 3x before ANY test runs. If backend is down → `pytest.exit()` immediately. |
| **Directory Setup** | Auto-creates `screenshots/`, `reports/`, `traces/` dirs. |

### 6.3 Available Fixtures
| Fixture | Viewport | Auth | Yields |
|---|---|---|---|
| `quality_page` | 1280×800 Desktop | ❌ Anonymous | `(page, tracker)` |
| `mobile_page` | 375×812 iPhone | ❌ Anonymous | `(page, tracker)` |
| `tablet_page` | 768×1024 iPad | ❌ Anonymous | `(page, tracker)` |
| `authenticated_user_page` | 1280×800 Desktop | ✅ Fresh user (KYC+$10k) | `(page, tracker, user_ctx)` |
| `admin_page` | 1280×800 Desktop | ✅ Admin login | `(page, tracker)` |
| `admin_mobile_page` | 375×812 iPhone | ✅ Admin login | `(page, tracker)` |

### 6.4 Context Configuration
| Setting | Value | Why |
|---|---|---|
| `locale` | `en-US` | Consistent date/number formatting |
| `timezone_id` | `Asia/Jakarta` | POOOL is Indonesia-based |
| `ignore_https_errors` | `true` | Local/staging certs |
| `has_touch` | `true` (mobile/tablet) | Touch events on mobile |
| `is_mobile` | `true` (mobile) | Mobile CSS media queries |
| `user_agent` | iPhone/iPad string | Realistic device emulation |
| Tracing | screenshots+snapshots+sources | Full trace on failure |

### 6.5 Reusable Helper Functions
```python
# Available in any test via: from conftest import ...
wait_for_api_response(page, "**/api/wallet/balance", action_fn=lambda: page.click("#refresh"))
take_named_screenshot(page, "marketplace_loaded")
check_toast_message(page, "Settings saved")
fill_and_submit_form(page, {"#first-name": "John", "#email": "john@test.com"}, "#btn-save")
intercept_api_call(page, "**/api/wallet/balance", {"balance_cents": 5000})
verify_table_has_data(page, "#users-table", min_rows=3)
verify_modal_opens_and_closes(page, "#btn-details", ".modal")
```

### 6.6 Noise Filtering
The tracker filters known noise patterns from asserting failures:
- `favicon.ico` 404s
- Ad blocker `ERR_BLOCKED_BY_CLIENT`
- Chrome `ResizeObserver loop` warnings
- Third-party: Sentry, Google Analytics, GTM, Hotjar
- `Failed to decode downloaded font`
- `third-party cookie` warnings
- DevTools messages, HMR updates

### 6.7 Installed Packages
| Package | Purpose |
|---|---|
| `pytest>=8.0` | Test runner |
| `pytest-playwright>=0.5.0` | Browser automation |
| `pytest-base-url>=2.0` | Configurable base URL |
| `pytest-html>=4.0` | HTML test report |
| `pytest-xdist>=3.5` | Parallel test execution |
| `pytest-rerunfailures>=14.0` | Auto-retry flaky tests |
| `pytest-timeout>=2.3` | Per-test timeout (60s default) |
| `psycopg2-binary>=2.9` | Database assertions |

### 6.8 Test Markers
```bash
pytest -m smoke       # Quick page-load health (< 5s each)
pytest -m admin       # Admin dashboard flows
pytest -m marketplace # Trading and asset flows
pytest -m financial   # Monetary value tests (CRITICAL)
pytest -m mobile      # Mobile viewport tests
pytest -m "not slow"  # Skip tests > 15s
```

### 6.9 Running Tests
```bash
# Activate venv first
source .venv/bin/activate

# All E2E tests (headless)
pytest tests/e2e/ -v

# All tests with HTML report
pytest tests/e2e/ --html=tests/e2e/reports/report.html

# Only smoke tests
pytest tests/e2e/ -m smoke -v

# With visible browser (debugging)
HEADED=1 pytest tests/e2e/test_marketplace.py -v

# Slow motion debugging (500ms per action)
HEADED=1 SLOWMO=500 pytest tests/e2e/test_settings.py -v

# Parallel execution (4 workers)
pytest tests/e2e/ -n 4

# Retry flaky tests
pytest tests/e2e/ --reruns=2 --reruns-delay=1

# Against staging
BASE_URL=https://platform.poool.app pytest tests/e2e/ -m smoke

# View Playwright traces (after failure)
npx playwright show-trace tests/e2e/traces/FAIL_*.zip
```

