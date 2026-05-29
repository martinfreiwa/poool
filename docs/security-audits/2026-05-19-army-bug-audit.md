# Army Bug Audit — 2026-05-19

Eight parallel report-only audit agents ran across backend Rust + frontend HTML/JS/CSS. Findings consolidated below by severity, then by domain.

**Bands:** auth+admin / money paths / developer+assets / community+comms / infra+core (backend); developer pages / auth+money pages / marketing+rest (frontend).

**Totals:** ~28 Critical · ~55 High · ~70 Medium · ~25 Low.

## Verification pass (2026-05-19)

Seven verification agents re-checked every Critical + High finding against current on-disk code. Eighth agent identified cross-cutting fix bundles.

**Aggregate: 106 CONFIRMED · 4 FALSE POSITIVES · 2 NEEDS_REPRO · 4 DOWNGRADED.**

### False positives (do NOT fix)

| Finding | Reason |
|---|---|
| `backend/src/wallet/routes.rs:1408-1428` cancel_withdrawal idempotency | `SELECT … FOR UPDATE` + status guard is inside the same tx as the refund UPDATE; double-refund race not possible. |
| `backend/src/blockchain/service.rs:215-225` settlement worker concurrency cap | Sequential `loop { process; sleep }`, not `tokio::time::interval` — self-serializes. No overlapping batches possible. |
| `frontend/platform/property.html:153` `\| safe` long_description | `PropertyDisplayData::from_asset` (`backend/src/marketplace/models.rs:319-333`) calls `escape_html()` before wrapping in `<p>`. Template `\| safe` marks already-escaped HTML. (commodity.html is still real — `CommodityDisplayData::from_asset` skips the escape.) |
| `frontend/platform/static/js/inbox-bell.js:108-122` link_url innerHTML | `n.link_url` passes through `escapeText()` before reaching `href=`. Scheme-validation gap is real but it's not an innerHTML-XSS sink. |
| `frontend/platform/components/head.html:213` Alpine "floating" claim | head.html pins `alpinejs@3.14.9`; only `frontend/platform/admin/*.html` (20 files) use floating `@3.x.x`. SRI half of the finding is still confirmed. |

### Downgraded (real but DB CHECK constraints catch the failure mode → loud error not silent loss)

| Finding | Old → New |
|---|---|
| `backend/src/marketplace/p2p.rs:450-469` buyer wallet | Crit → High (DB `044`/`050b` enforce `balance_cents >= 0` and `held_balance_cents <= balance_cents`) |
| `backend/src/marketplace/p2p.rs:472-482` seller tokens | High → Medium (`094_fix_investments_tokens_owned_check.sql` enforces `tokens_owned >= 0`) |
| `backend/src/payments/service.rs:874-878` wallet debit | High → Low (DB CHECK + in-tx FOR UPDATE provide safety net) |

### Needs repro

- `backend/src/leaderboard/service.rs:343-459` — TRUNCATE+INSERT empty-window depends on whether sqlx wraps in implicit tx. Worth a quick experiment.
- `frontend/platform/wallet.html:1844-2371` modal openers double-wired — agent saw single wiring at 2360-2371; the "duplicate" inline `onclick` claim needs broader file scan.

## Cross-cutting fix bundles

8 bundles where one fix shape closes many findings. Sequenced PR plan:

| # | Bundle | Count | Complexity | Why |
|---|---|---|---|---|
| 1 | `AdminUser` without `require_permission` | ~45 handlers / 13 files | Moderate (mechanical, needs slug picks) | Split into 3 PRs. PR1 = 6 critical (blockchain pause/unpause, storage retention, treasury, reports, dashboard search, pin_metadata). |
| 2 | `field.bytes().await` before size check | 6 callsites | Trivial (helper exists in KYC) | Extract `read_field_capped` helper into `backend/src/storage/upload_helpers.rs`. |
| 3 | innerHTML with user-controlled data | ~40 sinks / 15 files | Large (case-by-case) | Split per page-band. Toast/inbox/portfolio/mp-trading/p2p first. |
| 4 | Multiple `escapeHtml`/`escapeAttr` variants | 27 distinct impls | Moderate | Consolidate into `frontend/platform/static/js/util-escape.js`. Prerequisite for #3. |
| 5 | Float-money math in JS | 10 sites / 6 files | Trivial | Extract `parseAmountCents` from `wallet.html:1792` into shared util. |
| 6 | Hardcoded operator emails | ~25 refs / 13 files | Moderate (needs `platform_settings` keys) | Two PRs: backend keys, then template/JS defaults. |
| 7 | `#[allow(dead_code)]` on live functions | 4 false-positives + 4 real-dead | Trivial | One small PR to clear false-positives. |
| 8 | `target="_blank"` without `rel="noopener noreferrer"` | 13 sites / 9 files | Trivial | Single PR with eslint-equivalent rule to prevent regression. |

**Top 5 to attack first:** Bundle 1 PR1 → Bundle 2 → Bundle 8 → Bundle 4 → Bundle 3 PR1.

---

---

## Critical (drop everything)

### Money + financial logic

| File:Line | Issue |
|---|---|
| `backend/src/dividends/service.rs:245` | Per-holder payout divides by `total_tokens` (full supply) but `calculate_eligible_total` returns the **full pot** — the last-iterated eligible holder gets the entire unallocated remainder. Catastrophic over-payout, slips past the `total_credited > total_amount_cents` guard. |
| `backend/src/payments/service.rs:713` | Cart `subtotal_cents += asset_price * tokens_qty` is unchecked `i64` arithmetic. An admin-set max-price asset overflows silently, bypassing the investment-limit gate. |
| `backend/src/marketplace/p2p.rs:450-469` | `accept_offer` buyer wallet deduction lacks `AND balance_cents >= $1`; doesn't account for `held_balance_cents`. With open buy orders, a P2P accept can drive balance below held → effective double-spend. |
| `backend/src/dividends/service.rs:643-651` | `execute_distribution` wallet credit has no `FOR UPDATE` on target row. Combined with the per-holder bug above, repeat runs compound the over-payout. |
| `backend/src/blockchain/service.rs:643` (`reserve_nonce`) | Worker crash between broadcast and DB commit leaves permanent nonce gap; `release_nonce` only rolls back the trivial case. Every subsequent settlement freezes until manual ops. |
| `backend/src/wallet/routes.rs:1633-1640` | Withdrawal `wallet_transactions` row excludes `fee_cents`; user statements run wrong by the fee on every withdrawal. |

### Auth + admin authorization

| File:Line | Issue |
|---|---|
| `backend/src/auth/routes.rs:253-254` and `:1556-1557` | Hardcoded `is_2fa_verified=true` at login (and Google OAuth callback). Login-time TOTP challenge is disabled. Password compromise → full session, 2FA bypass. |
| `backend/src/admin/blockchain.rs:1311` (`api_admin_blockchain_pause`) and `:1389` (unpause) | No `require_blockchain_control_permission` — any admin role (support, finance, etc.) can pause/unpause global token transfers. |
| `backend/src/admin/blockchain.rs:1319-1335` (also `:1397`, `:387`) | `CHAIN_SETTLEMENT_PRIVATE_KEY` passed as `cast send --private-key <KEY>` CLI arg. Visible in `ps`, `/proc/*/cmdline`, any process listing. |
| `backend/src/kyc/routes.rs:474-490` (`wallet_bind`) | UPDATE has `(addr IS NULL OR addr = $1)` filter but no `rows_affected()` check. User with one wallet bound gets 200-OK rebind that silently does nothing — UI thinks success, on-chain settlement still goes to original. |
| `backend/src/admin/storage.rs:303` (retention_run) and `:341` (retention_arm) | Bare `AdminUser`, no permission check. Any admin can fire real GCS+DB delete pass against KYC documents (regulatory-reportable destructive op). |
| `backend/src/lib.rs:1722` | `/uploads` mounted unauthenticated via `ServeDir::new("../uploads")`. KYC documents, deposit proofs, developer files are world-readable to anyone with the filename. |

### Infra + data integrity

| File:Line | Issue |
|---|---|
| `backend/src/common/routes_helper.rs:55-59` (also `:122-125`, `:178-182`) | 500 HTML body interpolates raw MiniJinja error + cause chain → leaks file paths, raw context substrings, DB error text to unauthenticated users. |
| `backend/src/lib.rs:4243-4374` (`run_migrations`) | Per-file failures `continue`-d, schema half-applied, server keeps serving. Silent data corruption mode. |
| `backend/src/lib.rs:1736-1789` (CORS) | Prod allow-origin derived from `BASE_URL` via `replace("platform.", "")` with no anchoring → sister domains silently permitted on misconfig. |
| `backend/src/lib.rs:1577-1587` | No `with_graceful_shutdown`. SIGTERM hard-kills in-flight DB transactions / settlement / outbox / email → partial-state inconsistency. |
| `backend/src/lib.rs:259`, `db.rs:87,151,183`, `config.rs:88,140` | Startup `panic!` paths kill the binary before Sentry buffers. Bad env on fresh Cloud Run revision = silent boot loop, no root-cause capture. |

### Developer + asset uploads

| File:Line | Issue |
|---|---|
| `backend/src/developer/villa_operations.rs:740-756` (period docs) and `:914-953` (annual docs) | Upload trusts client `Content-Type` only — no magic-byte sniff, no SVG-payload rejection. Authenticated developer can store XSS-laced SVG disguised as PDF, served back via `download_asset_document` with the stored content-type. |
| `backend/src/developer/change_requests.rs:958-1032` (`apply_changes_to_asset`) | Each field UPDATE is separate, unwrapped, `.ok()`-swallowed, no transaction. Partial failure leaves asset inconsistent AND returns `{message: "Changes saved successfully"}`. |

### Community + comms

| File:Line | Issue |
|---|---|
| `backend/src/community/routes.rs:229-263` (`parse_and_store_opengraph`) | SSRF: reqwest with 3s timeout, no scheme allowlist, no private-IP block, no redirect cap. User posts `http://169.254.169.254/...` → server fetches inside prod VPC. Cloud metadata theft. |
| `backend/src/community/routes.rs:4336-4394`, `:4461-4516` (DM endpoints) | Ignore `allow_dms_from_strangers` profile flag; no rate limit. Anyone with target UUID can flood DMs to anyone. |
| `backend/src/community/routes.rs:1803-1840`, `:1918-1933`, `:1935-2066` | Admin community endpoints accept bare `AdminUser`, no `require_community_view_or_manage`. Any admin role reads all posts (incl. hidden) and rewrites `content_tags`. |
| `backend/src/common/notifications.rs:32-36` | `&title[..180]` byte-slice on `&str` panics on multi-byte UTF-8 (emoji, CJK, accents). Any caller passing user-controlled title >180 bytes panics the task. |

### Frontend XSS + CSRF

| File:Line | Issue |
|---|---|
| `frontend/platform/commodity.html:95`, `frontend/platform/property.html:153` | `{{ asset.long_description \| safe }}` renders developer-controlled HTML unsanitized. Stored XSS against every investor viewing the marketplace page. |
| `frontend/platform/static/js/portfolio-data.js:384`, `:399` | `escHtml` doesn't escape `'`. Title/cover interpolated into `onclick="..., '${...}',"`. Property title with `'` breaks JS context → arbitrary JS for every viewer of the portfolio. |
| `frontend/platform/components/head.html:194-208` | Form-action CSRF rewriter appends token to **every** POST form including cross-origin ones. CSRF token leaks to third-party endpoints. |
| `frontend/platform/static/js/toast.js:139-150` | `showPooolToast(title, msg)` uses `innerHTML`. Wide caller list passes `err.message` / `data.error` directly. Any backend error that ever reflects user input = XSS. |
| `frontend/platform/static/js/inbox-bell.js:108-122` | `n.link_url` dropped into `href=` with no scheme validation. `javascript:` notification link = XSS. |
| `frontend/platform/static/js/marketplace-trading-v3.js:74` | `toast.innerHTML = message` with `result.error` from order API. Same shape as toast.js. |
| `frontend/platform/static/js/mp-p2p.js:467` | Search-highlight `innerHTML` with regex-matched substring from `td code`. Stored XSS via P2P trader handle viewable by any admin. |
| `frontend/platform/signup.html:61` (and forgot-password / reset-password / verify-email / auth-2fa) | State-changing htmx POSTs with no CSRF hidden input; `auth-head.html` snapshots cookie once at script eval. If CSRF cookie set by the same response that returns the page (common first visit), POST has empty token. |
| `frontend/platform/checkout.html:104` + `:382` + `:94` | Confirm button outside form, two paths (onsubmit + click) to `handleCheckout`. `checkoutInProgress` guard only on click path. Real-money double-charge on Enter-in-input + button race. |
| `frontend/platform/wallet.html:2237` | `parseFloat(rawVal) * 100 > cashCents` float drift; doesn't include withdraw fee. Edge-case over-balance pass-through + UX bugs. |
| `frontend/platform/account-deletion.html:189-193` | UI requires user to type `"delete my account"`; POST sends hardcoded `{"confirm":"DELETE"}`. Confirmation phrase never reaches server. |

---

## High (fix this sprint)

### Money + ledger

| File:Line | Issue |
|---|---|
| `backend/src/payments/service.rs:989-995` (and inverse at `:1391-1393`) | Wallet checkout bumps `invested_12m_cents` on every order including pending bank-transfer; `cleanup_expired_orders` never reverses. Bank-transfer expiry permanently inflates user's invested-12m. Conversely, admin `approve_order` for bank-transfer never increments → limit silently bypassed. |
| `backend/src/payments/service.rs:1465-1469` | `approve_order` uses `grand_total_cents` (subtotal+fee) for affiliate commission tracker → commissions inflated by `platform_fee_pct`. |
| `backend/src/marketplace/p2p.rs:472-482` | Seller token deduction lacks `AND tokens_owned >= $1` invariant. Concurrent settlement can drive `tokens_owned` negative. |
| `backend/src/marketplace/p2p.rs:507-516` vs `marketplace/settlement.rs:557-566` | Column-name divergence: `purchase_price_cents` vs `purchase_value_cents`. One path errors at runtime on first-buyer-of-asset. |
| `backend/src/wallet/routes.rs:1408-1428` (`api_cancel_withdrawal`) | Refund non-idempotent; idempotency-key retry can double-refund during status race. |
| `backend/src/dividends/service.rs:272` | Ineligible holders inserted with `amount_cents=1` placeholder; any future caller summing without `eligible=true` filter is off by N cents. |
| `backend/src/payments/service.rs:874-878` | Wallet debit UPDATE lacks `AND balance_cents >= $1` invariant; relies entirely on FOR UPDATE inside tx. Cheap defensive fix missing. |
| `backend/src/payments/routes.rs:1573-1588` | Admin RBAC is per-handler `if !is_admin` inline, not middleware. Single mis-mount = unauthenticated admin endpoint. |
| `backend/src/blockchain/service.rs:215-225` | Settlement worker `interval` has no concurrency cap; slow chain causes overlapping batches → FD exhaustion. |
| `backend/src/wallet/routes.rs:343` (`parse_dollars_to_cents`) | Silently truncates 3+ decimals (`"19.999"` → 1999c). Inconsistent with `payments/routes.rs:118` which rejects scale>2. |

### Auth + admin (granular permission gaps)

| File:Line | Issue |
|---|---|
| `backend/src/admin/legal.rs:12`, `:73` | Any admin role bumps `legal_terms_version` / `legal_privacy_version` → forces re-accept on every user (DoS) + rewrites audit baseline. |
| `backend/src/admin/blockchain.rs:2047` | Any admin pins arbitrary metadata to IPFS, overwrites `assets.chain_metadata_uri`. On-chain token metadata pointable to attacker content. |
| `backend/src/admin/treasury.rs:13` | Any admin sees platform-wide deposit/withdraw totals + last-100 transactions. Should gate `treasury.read`. |
| `backend/src/admin/reports.rs:55`, `:95` | Any admin reads/generates per-user tax reports (email, fiscal year, dividends, capital gains). PII enumeration. |
| `backend/src/admin/dashboard.rs:298` (`api_admin_search`) | Any admin ILIKE-searches users by email/name, no `pii.view` gate. |
| `backend/src/admin/submissions.rs:14`, `:74`, `:127`, `:163` | API endpoints have no permission check while page-level guards require `submissions.review`. Defense-in-depth mismatch. |
| `backend/src/admin/storage.rs:251` | `api_admin_storage_reconcile` no permission; DOS + bucket-wide metadata read. |
| `backend/src/admin/storage.rs:16` | `api_admin_storage` returns user emails alongside KYC document IDs, no audit row. |
| `backend/src/admin/blockchain.rs:1341`, `:1419`, `:403` | Raw `cast` stderr embedded in `ApiError::Internal` → routed to Sentry. If `Internal` ever surfaces detail (one IntoResponse change away), RPC URL + signed message leak. |

### Infra + observability

| File:Line | Issue |
|---|---|
| `backend/src/lib.rs:1896-1921` | `sentry_user_context` middleware does per-request DB lookup on top of each handler's own `get_current_user` → doubles DB read load on most endpoints. |
| `backend/src/lib.rs:1724` (`/metrics`) | Prometheus endpoint unauthenticated; on public Cloud Run revisions internal counters scrapable by anyone. |
| `backend/src/error.rs:148-160` | Non-Internal `AppError` variants echo caller-supplied message verbatim; callers interpolate DB error strings (e.g. `common/notifications.rs:51`). Future error string can leak constraint name/value via 400. |
| `backend/src/cache.rs:1-21` | Dead alternate `init_pool` without TLS-enforcement / redaction. Reintroduces the plaintext-Redis loophole if revived. |
| `backend/src/lib.rs:633-640` | Only `auth_rate_limiter.cleanup()` runs; `leaderboard/community/storage` rate limiter in-memory backends grow unbounded → OOM in Redis-less deploys. |
| `backend/src/lib.rs:639-940` | Long-running tokio tasks have no shutdown-signal `select!`; daily reconciliation worker kill mid-INSERT can leave no row for the day even though checks ran. |
| `backend/src/common/routes_helper.rs:33-39`, `:94-100`, `:158-164` | Every `serve_protected*` runs an extra `affiliates` lookup per page render → 3 DB reads per page incl. session + sentry middleware. |
| `backend/src/leaderboard/service.rs:343-459` | `TRUNCATE` + re-INSERT in same tx; concurrent reads see empty leaderboard during refresh. Use swap-table or serializable tx. |
| `backend/src/lib.rs:667-779` | Custom advisory-lock pattern lifts/drops lock across pool connections; pg session-scoped lock auto-releases only on idle-timeout (120s) → blocks next replica for that window. |
| `backend/src/portfolio/service.rs:6-48` | `get_portfolio` no LIMIT/pagination; correlated subquery per row. Power user can blow up response. |
| `backend/src/lib.rs:970-1268` | 5-check reconciliation block uses shared accumulators; check 1 failing silently leaves `recon_*_delta = 0` and persists "pass" report → masks drift. |

### Developer + asset uploads

| File:Line | Issue |
|---|---|
| `backend/src/storage/routes.rs:1534-1583`, `:1479-1529` | Delete-asset-image / -document only delete DB row, leaks GCS object. Same for villa period docs. |
| `backend/src/developer/villa_operations.rs:282-465` (`compute_totals`) | Developer-supplied IDR cents / nights / expenses bound to UPDATE with no range validation. `i64::MIN/MAX` corrupts admin dashboards; downstream f64 cast can panic. |
| `backend/src/storage/routes.rs:1228-1313`, `:1004-1029` | `field.bytes().await` BEFORE size-check → 25MB Vec per concurrent upload allowed before endpoint cap. KYC handler uses chunked reads; image/doc paths regressed. |
| `backend/src/storage/routes.rs:244-332` (`upload_developer_logo`) | No role gate. Any authenticated user uploads to GCS bucket; ownership check happens AFTER upload. Bucket-prefix DOS. |
| `backend/src/developer/change_requests.rs:79-176` (`submit_edit`) | No `validate_text_len` — developer can push multi-MB description into assets or `proposed_values` JSONB. |
| `backend/src/storage/routes.rs:920`, `:1331-1343` | `is_svg_payload` defense not applied in `read_multipart_file` path used by avatar / developer-logo / post-image. Three of five image endpoints skip SVG check. |
| `backend/src/developer/villa_operations.rs:837-865` | `upload_villa_document` GCS-failure fallback path can leave orphan GCS objects on partial failure (no compensating delete like KYC). |
| `backend/src/developer/routes.rs:976-979` (`api_developer_asset_detail`) | `user_roles` membership check omits `is_active = TRUE`; other handlers in same file include it. Deactivated developer role passes this gate. |
| `backend/src/developer/extractors.rs:41-55` | `DeveloperUser` accepts `asset_owner` role not in `routes.rs` `user_has_developer_access`. Page-gate and API-gate role lists drift. |

### Community + comms

| File:Line | Issue |
|---|---|
| `backend/src/community/routes.rs:1809` (`admin_get_posts`) and `:1943-1973` (`admin_get_post_detail`) | `SELECT *` with no LIMIT/pagination. Single post with 10k+ comments OOMs handler. |
| `backend/src/support/handlers.rs:127`, `:272` | Attachment bytes read into memory before 5MB check; reply path has no size cap at all. |
| `backend/src/support/service.rs:258-321` (`reply_to_ticket`) | Skips `attachment_signature_matches` and 5MB cap that `submit_ticket` enforces. Open upload hole. |
| `backend/src/community/service.rs:434-460` | Post-rate-limit is check-then-set (GET, compare, INCR). Concurrent requests both pass before either increments → cap bypass. Same in comments. |
| `backend/src/community/amas.rs:206-232` | `submit_question` count-then-insert, no row lock, no unique constraint. |
| `backend/src/community/routes.rs:182-225` | Mention/reaction notifications persist unsanitized author display_name into `notifications.content`. Stored XSS if any consumer renders as HTML. |
| `backend/src/blog/sanity.rs:281-283` | `reqwest::Client::new()` with no timeout, allocated per request. Slow Sanity API hangs handler. |
| `backend/src/community/routes.rs:7600-7649` | `get_ban_appeals` N+1 cross-DB lookup. `get_users_info_batch` exists; use it. |

### Frontend (dev pages + general)

| File:Line | Issue |
|---|---|
| `frontend/platform/developer/dashboard.html:40-42` | Missing `dev_nav_show_period_tabs=true` in `{% with %}` → topbar tabs the JS queries don't render. Period selector silently absent. |
| `frontend/platform/developer/document-upload-step3.html` (lines 96, 129, 172, 206, 248, 282, 324, 363, 406, 441, 484) | Hardcoded `file1-1`..`file6-1` sample rows call `removeFile(id)` with single arg; handler's `if (assetId && documentId)` is false → silently removes DOM, no API call. Misleading delete control. |
| `frontend/platform/static/js/developer-asset-detail.js:763`, `:767` | `inv.tokens_owned.toLocaleString()` no null-guard; `inv.status` not escaped before innerHTML. Single malformed row crashes cap table. |
| `frontend/platform/static/js/portfolio-data.js` (escHtml mismatch) | Codebase has 6 distinct `escapeHtml` impls with subtle differences — only `user-data.js` escapes `'`. Maintenance landmine. |
| `frontend/platform/components/auth-head.html:53-67` | CSRF token read ONCE at script load → stale after cookie rotation on login. No native-fetch interceptor on auth pages. |
| `frontend/platform/admin/*.html` (20 files) + `components/head.html:211,213` | Alpine.js loaded from `alpinejs@3.x.x` (floating tag), no SRI. htmx@1.9.10 also no SRI. Supply-chain risk on every admin and authenticated page. |
| `frontend/platform/static/js/admin-sidebar-loader.js:348-368` | Sidebar links to `/statistics-template.html` etc. which now live in `_archive/` and are served via `serve_protected("_archive/...")`. Source-of-truth confusion for next maintainer. |
| `frontend/platform/static/js/community-feed.js:182-189` (and `:1498-1505`) | `escapeHtml` doesn't escape quotes; `escapeAttr` HTML-escapes `'` to `&#39;` which the browser decodes BEFORE JS sees it → still breaks JS string context. |
| `frontend/platform/static/js/user-data.js:242`, `:282` | `<img src="${avatarSrc}">` with `avatar_url` not escaped. `"x" onerror="alert(1)"` injects event handler shown on every user who opens profile switcher. |
| `frontend/platform/static/js/mobile-navigation.js:521-527` | `item.url` and `item.image` raw-interpolated into href/src. |
| `frontend/platform/static/js/admin-permission-guard.js:16-42` | Stacks fetch-wrapper on top of head.html's; load-order dependent. Refactor will silently break CSRF on admin POSTs. |
| `frontend/platform/leaderboard.html:161-162` | `legal-enhancements.js` and `mobile-navigation.js` loaded without `?v={{ asset_version }}` cache-buster. Stale JS for returning users. |
| `frontend/platform/static/js/marketplace-trading.js` | Not referenced by any page; v2 and v3 are the live ones. Dead but kept. |
| `frontend/platform/admin/admin-affiliate-fraud.html`, `admin/asset-change-requests.html`, `admin/asset-change-review.html` | Don't load `admin-permission-guard.js`. No sidebar permission-hiding; rely on head.html's fetch-wrapper for CSRF. |
| `frontend/platform/auth-2fa-step-up.html:115-116` | `return_to` from server `tojson`'d then `window.location.href = returnTo`. Open-redirect if server doesn't allowlist same-origin. |
| `frontend/platform/checkout.html:953` | `serverErrorEl.innerHTML = html` from non-JSON error responses. Reflects to DOM if server ever embeds user input. |
| `frontend/platform/payment-success.html:140` | `target="_blank"` to basescan.org without `rel="noopener noreferrer"`. Reverse-tabnabbing post-payment. |
| `frontend/platform/wallet.html:1844-2371` | Modal openers wired via both inline `onclick` AND `addEventListener` → fire twice. Fast double-open can overwrite deposit idempotency key mid-submit. |
| `frontend/platform/wallet.html:1238`, `:1265` | Stripe card-create has no `submitting` mutex; rapid second click after error → two PaymentMethod tokens. |
| `frontend/platform/wallet.html:2233` | `var cashCents = {{ cash_cents }};` unquoted. Non-numeric value from backend → JS syntax error → entire inline script (defining all `open*Modal`) fails. Wallet UI bricked. |

---

## Medium

Backlog. Includes: missing PII redaction in audit logs, broken/dead handlers, race conditions in non-money paths, dead code (developer/chart.rs, developer/models.rs DraftListItem, cache.rs::init_pool, db.rs helpers), inconsistent cookie-secure logic across auth.rs / csrf.rs, hardcoded admin emails for fee wallet, `format!` building SQL with currently-safe inputs (compliance/routes.rs:60-77), dead frontend pages (maintenance.html, 403.html, 500.html — no backend route), dual file-pairs (admin/affiliate-fraud.html + admin/admin-affiliate-fraud.html), 7-day `expires_at` jobs not wired, fragile `e.contains("not found")` error-string routing, `format!()`-into-SQL with hardcoded i64 in leaderboard, `__templ_*` code-gen residue in property-content.html / document-upload-step3.html, missing edit-window on comment edits, profanity filter substring matches (`ass` → `assassin`), email helper not escaping `'`.

(Full medium list in source agent reports — too long to inline. Approx 70 items across all bands.)

---

## Low

Cleanup. Includes: PII in source comments (signer.rs), TODO comments on financial constants ($30 friend reward), crate-level `#![allow(dead_code)]` suppressing the safety net, hardcoded MAX_DEPOSIT_IDR tied to stale FX assumption, sidebar emoji prefixes inconsistent with log parsers, real-looking TODO/FIXME in rewards/service.rs, `bin/mj_test.rs` test scratchpad shipped as bin target.

---

## Top 10 to fix this week

1. `backend/src/auth/routes.rs:253-254`, `:1556-1557` — remove hardcoded `is_2fa_verified=true`; restore login-time TOTP challenge.
2. `backend/src/dividends/service.rs:245` — switch divisor to `eligible_tokens`; fix `calculate_eligible_total` to stop returning the full pot.
3. `backend/src/admin/blockchain.rs:1311`, `:1389` — gate `pause`/`unpause` on `BLOCKCHAIN_CONTROL_PERMISSION`.
4. `backend/src/lib.rs:1722` — auth-gate or remove the unauthenticated `/uploads` mount serving KYC docs + deposit proofs.
5. `backend/src/admin/blockchain.rs:1319-1335` (and `:1397`, `:387`) — pipe `CHAIN_SETTLEMENT_PRIVATE_KEY` via env or stdin, not CLI arg.
6. `backend/src/payments/service.rs:713` — `checked_mul`/`checked_add` on the cart subtotal loop.
7. `backend/src/marketplace/p2p.rs:450-469` — add `AND balance_cents >= $1` + `held_balance_cents` accounting on buyer wallet deduction.
8. `backend/src/common/routes_helper.rs:55-59`, `:122-125`, `:178-182` — render static 500; log error server-side only.
9. `backend/src/community/routes.rs:229-263` — private-IP/loopback block, scheme allowlist, redirect cap on `parse_and_store_opengraph`.
10. `frontend/platform/commodity.html:95`, `frontend/platform/property.html:153` — sanitize `asset.long_description` server-side (HTML allowlist) or drop `| safe`.

## Top 10 — frontend / browser-side

1. `frontend/platform/components/head.html:194-208` — gate form-action CSRF rewrite on same-origin check.
2. `frontend/platform/static/js/toast.js:139-150` — `textContent` instead of `innerHTML` (kills the wide-blast XSS surface).
3. `frontend/platform/static/js/portfolio-data.js:384`, `:399` — replace `escHtml`-into-JS-string with `addEventListener`.
4. `frontend/platform/account-deletion.html:189-193` — send the user-typed `confirm` value, not hardcoded `"DELETE"`.
5. `frontend/platform/wallet.html:2237` — use `parseAmountCents` + add withdraw fee to balance comparison.
6. `frontend/platform/wallet.html:2233` — `{{ cash_cents | default(0) }}`; consider quoting.
7. `frontend/platform/signup.html:61` (+ forgot/reset/verify/2fa) — add CSRF hidden input; switch to per-request snapshot.
8. `frontend/platform/checkout.html:104`/`:382` — drop the form's dead `onsubmit`; gate click-handler on `checkoutInProgress` before any DOM read.
9. `frontend/platform/static/js/inbox-bell.js:108-122` — scheme allowlist on `link_url`.
10. `frontend/platform/developer/dashboard.html:40-42` — add `dev_nav_show_period_tabs=true` to the period-tabs `{% with %}`.

---

## Pre-existing audit deduplication

The auth+admin band cross-referenced `docs/security-audits/2026-05-16-admin-auth-followup.md` and excluded its 10 findings. Findings above are new or out-of-scope of that audit.

## Notes on heavy WIP

Backend `developer/` module and frontend `developer/` pages are mid-edit per `git status`. Findings reflect on-disk state. Critical and High items there (silent partial writes in `apply_changes_to_asset`, SVG MIME spoofing in villa docs, missing period-tabs flag on dashboard) all stem from the in-progress changes.
