# POOOL Live Site User Audit Workflow

Date: 2026-05-11  
Timezone: Europe/Berlin, CEST +0200  
Auditor: Codex in-app browser and HTTP checks  
Primary target: https://www.poool.app and https://platform.poool.app  
Mode: Read-only QA. No account creation, form submissions, orders, withdrawals, or financial actions were performed.

## Scope

The original request was to log in as a user, find things that are not working, and create a list. Initial work started against the local development server because the browser was on `localhost`. The user then clarified that the audit should run on the live website, so local work was stopped and the target switched to production.

## Timeline

| Time CEST | Workflow | Target | Result |
|---|---|---|---|
| 01:07 | Repo orientation and credential discovery | Local repo and local DB | Found seeded local user account for test use. |
| 01:07 | Local server check | http://127.0.0.1:8888 | Backend was not running. |
| 01:07-01:08 | Started local backend | Local Rust backend | Server came up on port 8888. Startup logged repeated migration-already-exists errors, Redis disabled warning, and later fund-conservation drift warnings. |
| 01:08 | Local authenticated session setup | Local platform | Browser automation could not type into the email input due a browser-control issue with `input[type=email]`. Used a real server-issued local session for the seeded user to continue read-only local testing. |
| 01:08-01:10 | Local authenticated route scan | Local platform user pages | Main routes returned 200. Local-only broken references found: `/static/images/ID.webp` and `/static/images/icons/trash-icon.svg`. Direct `/marketplace-trading-v3` showed an asset-not-found page with placeholder trading data. |
| 01:11 | User clarified target | Live website | Stopped local backend and switched to live site. |
| 01:12 | Live public landing inspection | https://www.poool.app | Landing page loaded successfully. No browser console errors observed. |
| 01:12 | Live public link/reference scan | Landing, login, signup | 57 internal live references checked. No broken references found. |
| 01:12 | Live unauthenticated protected-route scan | https://platform.poool.app | Protected user routes correctly returned 303 redirects to `/auth/login`. |
| 01:13 | Live authenticated browser audit | Existing live browser session | Browser was already authenticated on `platform.poool.app`, so no credentials were requested or transmitted. |
| 01:13-01:14 | Live authenticated page sweep | Marketplace, commodities, portfolio, wallet, rewards, leaderboard, settings, support, cart, community, KYC, secondary market, my trading, trading v3 | No console errors were observed across the swept pages. One production issue was confirmed on direct trading-v3 access, plus accessibility/UX risks listed below. |

## Live Findings

### 1. Direct Trading V3 route shows stale placeholder trading data

Severity: Medium  
URL: https://platform.poool.app/marketplace-trading-v3  
Status: Reproduced on live authenticated session.

Steps:
1. Navigate directly to `/marketplace-trading-v3` while authenticated.
2. Observe the page title and main content.

Observed:
- Page heading shows `Asset Not Found`.
- The same page still shows placeholder/default market data such as `$105.00 / share`, `Property Value $850,000`, `Available 342 / 1,000`, yield metrics, and trade UI content.
- Multiple `Loading...` / `Loading...` accessibility labels remain in the page.

Expected:
- Missing asset context should show a clean error state or redirect back to the secondary market.
- It should not display default trading data or trade-adjacent controls when no asset is loaded.

Risk:
- A user can land on a misleading asset-not-found page that still looks partially tradable and financially specific.

### 2. Secondary market cards navigate by clicking text, but are not exposed as links

Severity: Low to Medium  
URL: https://platform.poool.app/marketplace-secondary  
Status: Reproduced on live authenticated session.

Steps:
1. Open `/marketplace-secondary`.
2. Inspect the accessible DOM.
3. Click the first asset title, `The Grand Pavilion - Ultra-Luxury Estate`.

Observed:
- Clicking the title navigated to `/marketplace-trading-v3?asset=grand-pavilion-ubud-estate`.
- The accessible DOM did not expose the asset cards/titles as links or buttons; only image carousel buttons were exposed for the cards.

Expected:
- Asset cards or titles should be real anchors or accessible buttons with clear labels.

Risk:
- Keyboard and screen-reader users may not discover how to open a secondary-market asset detail page.

### 3. Valid Trading V3 detail pages keep loader images in the accessibility tree

Severity: Low  
URL tested: https://platform.poool.app/marketplace-trading-v3?asset=grand-pavilion-ubud-estate  
Status: Reproduced on live authenticated session.

Observed:
- The valid asset detail loaded with no console errors.
- The accessibility tree still included repeated images labelled `Loading...` before the actual property images.

Expected:
- Loader graphics should be hidden from assistive tech after content is loaded, or use empty alt text if decorative.

Risk:
- Screen-reader output is noisy and may imply content is still loading when the page is already populated.

## Live Checks That Passed

- `https://www.poool.app/` loaded and rendered public landing content.
- `https://platform.poool.app/auth/login` and `/auth/signup` returned 200 in the public scan.
- Protected routes including `/marketplace`, `/portfolio`, `/wallet`, `/settings`, `/support`, `/cart`, `/community`, `/kyc`, `/marketplace-secondary`, `/my-trading`, and `/marketplace-trading-v3` redirected unauthenticated requests to `/auth/login`.
- Authenticated live page sweep showed no JavaScript console errors for the tested pages.
- Public live reference scan found no broken internal links/assets among the checked landing/login/signup references.

## Local-Only Notes

These were discovered before the user clarified that the target should be live. They may still be useful for local cleanup but are not production findings unless reproduced live.

- Local startup logged multiple migration failures caused by already-existing objects.
- Local startup logged Redis not configured, meaning marketplace matching was disabled locally.
- Local startup later logged a fund conservation invariant violation.
- Local page reference scan found missing `/static/images/ID.webp` and `/static/images/icons/trash-icon.svg`.
- Local direct `/marketplace-trading-v3` had the same asset-not-found plus placeholder-data issue later confirmed on live.

## Limitations

- No destructive or write actions were performed.
- Checkout, buy/sell order submission, payment method changes, support ticket creation, profile edits, KYC submission, withdrawals, and account changes were intentionally not submitted.
- The live authenticated audit used the already-authenticated in-app browser session. No production password was requested or entered.

