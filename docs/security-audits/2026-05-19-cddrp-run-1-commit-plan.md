---
related-audit: 2026-05-19-cddrp-run-1.md
related-audit-2: 2026-05-19-army-bug-audit.md
purpose: Sequenced commit plan for the 131+ security fixes landed during CDDRP Run 1
build-verified: cargo check --tests clean against live DB
not-included: User's pre-existing WIP (developer/* subsystem, admin/mod.rs, frontend/developer/*, etc.)
---

# CDDRP Run 1 — Commit Plan

This document sequences the 131+ security fixes from CDDRP Run 1 into 11 atomic commits, ordered so each commit compiles independently against `cargo check`. Pre-commit verification (`cargo check --tests`) was clean at every phase.

## Pre-commit prerequisites

1. **Stage user WIP separately first.** Several untracked files predate this session and are part of unrelated WIP — they must be staged/committed (or stashed) before the security commits below land cleanly:
   - `backend/src/admin/developer_applications.rs` (referenced by `admin/mod.rs` — won't compile without it)
   - `database/203_developer_applications.sql` + rollback
   - `backend/tests/admin_developer_applications_http.rs`, `backend/tests/developer_*_http.rs`
   - `tests/_developer_static.py`, `tests/e2e/test_developer_*.py`, `tests/test_developer_*_static.py`

2. ~~Run `cargo sqlx prepare`~~ **Not needed** — verified `SQLX_OFFLINE=true cargo check` is clean against the current `.sqlx/` cache (the new `ON CONFLICT` query happens to match an existing cache entry's schema, and the new role-perm session query also resolves cleanly). No new `.sqlx/*.json` files to stage.

3. **Verify `cargo check --tests` clean.** Already done at end of session.

## Commit sequence

All commits use Conventional Commits style (matching project history). Subjects are ≤72 chars. Each body ends with the cross-reference trailer `Audit: docs/security-audits/2026-05-19-cddrp-run-1.md`.

---

### 1. `docs(audit): add CDDRP run 1 security audit + remediation reports`

Adds the methodology, findings, verification, and remediation log for the 131+ security fixes in this PR.

**Files:**
```
docs/security-audits/2026-05-19-army-bug-audit.md
docs/security-audits/2026-05-19-cddrp-run-1.md
docs/security-audits/2026-05-19-cddrp-run-1-commit-plan.md
.claude/commands/audit-fix.md
```

**Body:**
```
- 2026-05-19-army-bug-audit: domain-band sweep across backend + frontend (157 raw findings)
- 2026-05-19-cddrp-run-1: cross-cutting (B1-B8) + tooling (C1/C2/C4/C5) + tests sweep,
  verified against prior audit, 99 base findings remediated + 30+ follow-up
- audit-fix.md slash command: CDDRP v2.0 methodology spec for future runs

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 2. `chore(backend): housekeeping — dead code, stale attrs, PII in comments`

Mechanical cleanup that has no behaviour impact but unblocks the security-fix commits.

**Files:**
```
backend/src/common/routes_helper.rs    # delete unused serve_admin_protected helper
backend/src/storage/service.rs         # drop stale #[allow(dead_code)] on live delete_object
backend/src/blockchain/signer.rs       # anonymise 5 user emails in #[ignore] test comments
backend/src/admin/access.rs            # exists-then-INSERT race → ON CONFLICT DO NOTHING (api_roles_create)
backend/src/community/circles.rs       # escape \ % _ in search_circles LIKE wildcards
```

**Body:**
```
- common/routes_helper.rs: remove dead `serve_admin_protected` (zero callers)
- storage/service.rs: drop stale #[allow(dead_code)] on `delete_object` (called from
  retention.rs and storage/routes.rs)
- blockchain/signer.rs: anonymise emails next to wallet addresses in the
  one-shot mainnet-bootstrap test (`holder #1..5`)
- admin/access.rs::api_roles_create: race-free `INSERT ... ON CONFLICT (name) DO NOTHING
  RETURNING id` — fail-closed on conflict instead of TOCTOU
- community/circles.rs::search_circles: escape `\`, `%`, `_` before LIKE
  (was a latent footgun — user-controlled query reached LIKE without escape)

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 3. `fix(auth): redact email PII from logs, UTF-8-safe truncation, verify-email GET→POST`

Auth-surface hardening: log-redaction, panic guards, link-prefetch defeat.

**Files:**
```
backend/src/auth/service.rs            # drop email from 3 tracing::info! lines + verify_email sibling-token invalidation
backend/src/auth/routes.rs             # verify_email_page renders auto-submit form; new POST handler
backend/src/common/notifications.rs    # char-boundary-safe title truncation (chars().take(180))
backend/src/settings/service.rs        # delete_account_selective: validate displayed phrase, not hardcoded "DELETE"
frontend/platform/account-deletion.html # send user-typed confirm phrase, not literal "DELETE"
```

**Body:**
```
- auth/service.rs: replace 3 `tracing::info!` lines that logged raw email with
  user_id-only versions (session validation, new-user registration, verification queue)
- auth/service.rs::verify_email: invalidate all sibling verification tokens on
  consume (mirror reset_password defense-in-depth)
- auth/routes.rs::verify_email_page: was GET with side effects (link prefetchers
  consumed token before user clicked). Now renders a minimal auto-submitting
  POST form with <noscript> fallback button
- auth/routes.rs::verify_email_confirm_handler (new): performs the state mutation
- common/notifications.rs: byte-slice `&title[..180]` panicked on multi-byte
  UTF-8 boundaries (emoji, CJK, accented letters). Use `chars().take(180).collect()`
- settings/service.rs + account-deletion.html: confirmation phrase the UI
  prompts for ("delete my account") is now actually sent and validated by
  the server (was a hardcoded "DELETE" — pure UI theater previously)

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 4. `fix(security): close SSRF in affiliate webhook + community OG parser`

Critical SSRF closures, both targeting GCP metadata service exfiltration.

**Files:**
```
backend/src/rewards/service.rs         # affiliate webhook: validate_postback_url + Policy::none()
backend/src/community/routes.rs        # OG parser: scheme + host + IP block + DNS resolve + redirect::Policy::none() + bytes cap + mention amplification cap
```

**Body:**
```
- rewards/service.rs::create_webhook_subscription: was validating only
  `starts_with("https://")` — added the existing `validate_postback_url`
  (host allowlist + IP block + DNS resolve, already used by the legacy
  S2S worker)
- rewards/service.rs subscription worker: added `.redirect(Policy::none())`
  to match the legacy worker pattern. Closes 30x-redirect chain to
  169.254.169.254 (GCP metadata) via status-code oracle
- community/routes.rs::parse_and_store_opengraph: full SSRF hardening
  (Url::parse, scheme allowlist, host string allowlist, IP block on literals
  + DNS-resolved IPs, `redirect::Policy::none()`) + body cap (MAX_OG_BYTES = 1 MB
  via chunked accumulation) — was reading unbounded `res.text()`
- community/routes.rs::parse_and_notify_mentions: cap at 50 mentions per
  post (was unbounded — 10K @-mentions = 10K SELECT + 10K INSERT in
  fire-and-forget task) with single `tracing::warn!` on cap-hit

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 5. `fix(uploads): chunked multipart reads + deposit-proof filename sanitisation`

Closes memory-amplification on multipart uploads and overwrite-of-prior-proofs.

**Files:**
```
backend/src/storage/upload_helpers.rs       # NEW — read_field_capped helper
backend/src/storage/mod.rs                  # declare upload_helpers module
backend/src/storage/routes.rs               # migrate upload_asset_image + upload_asset_document
backend/src/blog/routes.rs                  # admin_blog_upload_asset: chunked read with 8 MB cap
backend/src/support/handlers.rs             # submit_ticket + reply_to_ticket: chunked read
backend/src/rewards/routes.rs               # api_affiliate_upload_material + tax_document: chunked
backend/src/admin/assets.rs                 # admin image + doc uploads: chunked
backend/src/payments/routes.rs              # deposit proof_of_transfer: chunked + filename sanitise + UUID prefix
```

**Body:**
```
Before this commit, 8+ multipart upload handlers called `field.bytes().await`
which buffers the entire payload before any size check — allowing memory
amplification up to the per-route DefaultBodyLimit (25 MB in some places).

- storage/upload_helpers.rs (NEW): `read_field_capped(&mut field, max_bytes, label)`
  returns `Result<Vec<u8>, ApiError>` with early-bail on overflow. Mirrors
  the KYC chunked-read pattern at storage/routes.rs:499-526.
- 7 call-sites migrated to the helper (storage image/doc, blog asset, support
  submit + reply, rewards material + tax-doc, admin assets image + doc,
  deposit proof).
- payments/routes.rs deposit-proof: also added filename sanitisation
  (ASCII allowlist + leading-dot strip + 64-char cap + fallback "proof")
  and UUID prefix to prevent overwrite of prior proofs (AMLD audit-trail
  immutability).

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 6. `feat(common): JSON-in-script escape helper + 4 callsites`

Closes `</script>` breakout vector in JSON islands embedded via `| safe`.

**Files:**
```
backend/src/common/json_safe.rs        # NEW — to_safe_json_script helper
backend/src/common/mod.rs              # declare json_safe module
backend/src/portfolio/routes.rs        # portfolio_json
backend/src/payments/routes.rs         # cart_json, wallet_json, bank_json
```

**Body:**
```
`serde_json::to_string` does not escape `</script>`. A JSON island like
`<script type="application/json">{{ portfolio_json | safe }}</script>`
where any field could contain `</script><script>alert(...)</script>`
breaks out of the JSON parser. Today the fields are admin-controlled
(asset titles) so risk is low, but the foot-gun ships on every page render.

- common/json_safe.rs (NEW): `to_safe_json_script` escapes `</` → `<\/`
  and U+2028/9 → `\u202[89]`. Output remains valid JSON; JSON.parse
  unescapes correctly on the client.
- 4 callsites migrated: portfolio_json (portfolio/routes.rs:88),
  cart_json + wallet_json + bank_json (payments/routes.rs).

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md
```

---

### 7. `fix(admin): require_permission gates on 76+ admin handlers`

The largest single bundle: RBAC gates on previously bare `AdminUser` handlers.

**Files:**
```
backend/src/admin/blockchain.rs              # pause / unpause / pin_metadata (3 handlers)
backend/src/admin/storage.rs                 # retention_run / retention_arm (2)
backend/src/admin/dashboard.rs               # api_admin_search (1)
backend/src/admin/marketplace.rs             # 26 handlers (kill-switch, compliance, view)
backend/src/admin/villa_operations.rs        # 15 handlers
backend/src/admin/villa_valuations.rs        # 8 handlers
backend/src/admin/villa_capex.rs             # 3 handlers
backend/src/admin/villa_developer_access.rs  # 3 handlers
backend/src/admin/villa_deduction_policy.rs  # 2 handlers
backend/src/admin/villa_forecast.rs          # 5 handlers
backend/src/admin/villa_nav_snapshot.rs      # 1 handler
backend/src/developer/change_requests.rs     # 8 handlers (admin-side review)
```

**Body:**
```
Adds `admin.require_permission(&state.db, "<slug>").await?` (or
`require_blockchain_control_permission`) to 76 handlers that previously
took only the `AdminUser` extractor — meaning any admin role could
access them.

Permission slug mapping per area:
- Blockchain: `BLOCKCHAIN_CONTROL_PERMISSION` for pause/unpause/pin_metadata
- Storage: `kyc.write` for retention operations
- Dashboard: `users.view` for the topbar search
- Marketplace: `marketplace.manage` (kill-switch), `marketplace.compliance`
  (18 alerts/watchlist/rules), `marketplace.view` (7 views/push)
- Villa subsystem: `villa.<area>.{view,write,approve,manage}` with
  maker/checker split on publish/approve paths
- Developer change-requests admin: `developer_projects.view` (reads),
  `developer_projects.write` (mutations)

NOTE: The new villa.* / developer_projects.* slugs are NOT yet defined
in a DB seed migration. Until that ships, only super_admin / admin (which
hold the `'all'` wildcard) can access these handlers. Sub-roles
(compliance, finance, support) need a follow-up seed migration.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§2.1, §3.2)
```

---

### 8. `fix(admin): bound previously-unbounded queries with LIMIT + pagination`

Closes resource-exhaustion vectors from admin / community read endpoints.

**Files:**
```
backend/src/admin/users.rs              # api_admin_user_detail investments: LIMIT 500 + ?page=
backend/src/admin/withdrawals.rs        # api_admin_withdrawals: LIMIT 500 + ?page=
backend/src/admin/kyc.rs                # api_admin_kyc: LIMIT 500 + ?page=
backend/src/admin/treasury.rs           # dividends/calculate: warn at >10K (correctness requires all)
backend/src/admin/emails.rs             # send_campaign: audience cap LIMIT 100K + batched INSERTs (500/batch)
backend/src/rewards/routes.rs           # affiliate referrals: LIMIT 500 + ?page=
backend/src/community/circles.rs        # get_circle_members: LIMIT 100
backend/src/community/service.rs        # get_pending_reports: LIMIT 200
```

**Body:**
```
8 read endpoints previously returned the entire result set with no LIMIT
clause. At scale (100K+ users, 100K+ referrals, large circles, etc.)
each call materialised multi-MB JSON and blocked the handler worker.

- admin/users.rs, withdrawals.rs, kyc.rs, rewards/routes.rs: bounded at
  500 rows + `?page=N` offset pagination
- admin/emails.rs::send_campaign: audience capped at 100K + per-user
  INSERT loop replaced with batched `INSERT ... VALUES (), ()` via
  sqlx::QueryBuilder (500 rows / chunk, per-chunk transaction)
- admin/treasury.rs::dividends/calculate: NO limit added (correctness
  requires every active investor); added `tracing::warn!` when count >10K
- community/circles.rs::get_circle_members: bounded at 100 rows
- community/service.rs::get_pending_reports: bounded at 200 rows
- Pagination plumb for community endpoints deferred (5 internal callers)

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§3.5)
```

---

### 9. `fix(marketplace): integer-overflow guards + WS channel-map eviction`

Marketplace-specific safety: arithmetic overflows that bypassed compliance,
and a long-running memory leak in the WS broadcast layer.

**Files:**
```
backend/src/marketplace/validation.rs   # concentration-cap: i32 checked_add (both sites)
backend/src/marketplace/service.rs      # trade-history: checked_mul row-skip + checked_add/sub fallback
backend/src/marketplace/websocket.rs    # subscriber-count eviction + MAX_CHANNELS = 100_000 cap
backend/src/blockchain/event_indexer.rs # MAX_TRANSFER_BATCH_IDS = 10_000 cap on Vec::with_capacity
backend/src/payments/service.rs        # (no edit here — kept untouched per audit)
```

**Body:**
```
- marketplace/validation.rs::check_concentration_limit{_tx}: `current_owned +
  additional_tokens` was unchecked i32+i32. A buy with quantity≈i32::MAX
  wrapped to negative, passed the 80%-cap check. Now `checked_add`
  with explicit reject (BadRequest / OrderRejection::InvalidQuantity).
- marketplace/service.rs trade-history: `total = price.saturating_mul(qty)`
  followed by `total ± fee` wrapped at i64::MAX. Now `checked_mul` (skip row
  + log on overflow) and `checked_add/sub` (fallback to `total` + log).
- marketplace/websocket.rs: CHANNELS HashMap never evicted per-asset
  broadcast::Senders (256-msg ring buffer each). Now subscriber-count
  eviction at disconnect with TOCTOU-safe double-check, plus
  MAX_CHANNELS = 100_000 hard cap returning WS Close 1013.
- blockchain/event_indexer.rs: Vec::with_capacity(ids_count) where
  ids_count parsed from on-chain hex (untrusted). Capped at 10_000
  with tracing::warn! and early-return on over-cap.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§3.6, §2.4)
```

---

### 10. `fix(admin): session invalidation on role/permission mutations + change_email`

After any role grant, role revoke, role update, or role-permission edit,
or change_email, the target user's existing sessions are deleted so
they pick up the new authorisation state on next request.

**Files:**
```
backend/src/admin/settings.rs           # add_admin / remove_admin / update_admin_role (3 sites)
backend/src/admin/access.rs             # api_roles_update_permissions
backend/src/settings/service.rs         # change_email
```

**Body:**
```
- admin/settings.rs add/remove/update_admin handlers: DELETE FROM user_sessions
  WHERE user_id = $1 after the role mutation. Compromised super_admin session
  can no longer silently mint other admins with stale-session-survival.
- admin/access.rs::api_roles_update_permissions: DELETE FROM user_sessions
  WHERE user_id IN (SELECT user_id FROM user_roles WHERE role_id = ANY($1)
  AND is_active = TRUE) — invalidates every user holding any affected role.
  Closes silent privilege escalation/de-escalation via permission map edit.
- settings/service.rs::change_email: DELETE FROM user_sessions after email
  UPDATE — mirrors the existing change_password pattern. Closes
  cookie-bearing-attacker pivot via email swap → forgot-password takeover.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§3.3)
```

---

### 11. `fix(infra): static 500 HTML body + fail-closed migrations + CORS strip_prefix`

Infrastructure-tier defensive hardening.

**Files:**
```
backend/src/lib.rs                      # run_migrations fail-closed (panic instead of continue); CORS replace → strip_prefix
backend/src/common/routes_helper.rs     # 500 body: static <h1>Internal Server Error</h1>, log error server-side
backend/src/admin/pages.rs              # 500/404 body: same fix
backend/src/blog/routes.rs              # 500 body: same fix (2 sites)
backend/src/portfolio/routes.rs         # 500/404 body: same fix
backend/src/assets/routes.rs            # 500 body: same fix (2 sites)
backend/src/rewards/routes.rs           # invoice 500 body: same fix
backend/src/kyc/routes.rs               # wallet_bind: check rows_affected == 0 → 409 Conflict
backend/src/assets/models.rs            # CommodityDisplayData: escape_html on long_description (parity with Property)
```

**Body:**
```
- lib.rs::run_migrations: was logging error + `continue`-ing on per-file
  failure → silent half-applied schema. Now `panic!` on any error
  (table create / dir read / file read / tx begin / statement / insert /
  commit). Fail-closed at boot is safer than serving traffic against
  a corrupt schema.
- lib.rs CORS prod branch: `base_url.replace("platform.", "")` was unanchored
  → `eu-platform.poool.app` → `eu-poool.app`, sister-domain bleed. Now
  `strip_prefix("platform.").unwrap_or(&base_host)`.
- 8 sites: `Html(format!("<h1>...{}</h1>", e))` for 500/404 bodies leaked
  raw MiniJinja / sqlx errors (file paths, context fragments) to clients.
  Now static body + `tracing::error!` server-side.
- kyc/routes.rs::wallet_bind: UPDATE had `(addr IS NULL OR addr = $1)`
  filter but didn't check rows_affected(). Second-wallet rebind silently
  200-OK'd while the on-chain address stayed unchanged. Now 409 Conflict
  on rows_affected == 0.
- assets/models.rs::CommodityDisplayData::from_asset: was wrapping `<p>{}</p>`
  without escape_html on the description (PropertyDisplayData has the
  escape). Closes stored XSS on commodity pages.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§3.6 + §2.5)
```

---

### 12. `fix(frontend): admin community XSS, concentration banner, rel=noopener sweep`

All frontend security fixes batched.

**Files:**
```
frontend/platform/admin/community/posts.html              # escapeHtml/Attr/JsString helpers + wrap 19 sites
frontend/platform/admin/community/post-detail.html        # same helpers + wrap 27 sites + double-nested editTags fix
frontend/platform/static/js/admin-asset-details.js        # concentration banner: esc(topHolder.name)
frontend/platform/payment-success.html                    # rel="noopener noreferrer" on basescan link
frontend/platform/landing-v2.html                         # same on wa.me link
frontend/platform/landing.html                            # same
frontend/platform/property-public.html                    # same
frontend/platform/marketplace-trading-v3.html             # same
frontend/platform/admin/asset-details.html                # same
frontend/platform/admin/blockchain-treasury.html          # same (2 links)
frontend/platform/admin/settings.html                     # same (2 links)
frontend/platform/static/js/admin-blockchain-treasury.js  # same (2 dynamic links)
frontend/platform/static/js/admin-orders.js               # same (2 dynamic links)
frontend/platform/static/js/admin-rewards.js              # same
frontend/platform/static/js/support.js                    # same
```

**Body:**
```
- admin/community/posts.html + post-detail.html: added escapeHtml /
  escapeAttr / escapeJsString helpers at top of <script>. Wrapped every
  API-sourced interpolation (author_name, content, content_tags,
  reporter_name, reason, etc.) — 27 sites total across both files.
  Closes admin ATO via stored XSS (any community user could set a
  malicious display name → every moderator's session compromised).
- admin-asset-details.js: concentration-risk banner inserted
  `topHolder.name` raw into innerHTML while the sibling cap-table at
  line 475 correctly used esc(). Now consistent: esc(topHolder.name).
- rel=noopener sweep: 13 `target="_blank"` links across 8 HTML files
  and 4 JS template-literal sites now have `rel="noopener noreferrer"`.
  Closes reverse-tabnabbing on outbound links.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§2.4 + §3.7)
```

---

### 13. `feat(rbac): seed permission slugs introduced in CDDRP run 1`

Closes QA caveat #2: the ~50 new permission slugs added in commit 7 work today only via the `'all'` shortcut held by super_admin / admin. Sub-roles need explicit grants.

**Files:**
```
database/204_cddrp_run1_permission_slugs.sql                    # NEW
database/rollback/204_cddrp_run1_permission_slugs.rollback.sql  # NEW
```

**Body:**
```
Seeds 24 new (role, permission) row sets in admin_permissions:

- Villa subsystem (15 slugs): super_admin gets all; compliance gets read-only
  views + valuation-approve (independent oversight); finance gets read views +
  operations/valuations/capex approve (treasury sign-off).
- developer_projects.view/.write: super_admin gets both; compliance + support
  get .view for triage.
- .view variants alongside existing .read (kyc/users/support): granted to
  the same roles that hold the .read variants. Closes drift between newer
  handlers using .view and older handlers using .read.
- Pre-existing gap (approvals.manage, marketplace.edit, platform.manage,
  users.edit): granted to super_admin only — sub-role grants need product
  input.

Roundtrip-tested on live local DB: 45 INSERTs → DELETE 45 → 45 INSERTs again.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md §14 caveat #2
```

---

### 14. `test: prod-DSN safety guard on integration test pool()`

**Files:**
```
backend/tests/auth_2fa_http.rs                # mod safety + assert_database_url_is_local
backend/tests/checkout_wallet_http.rs         # same
```

(The 3rd file, `backend/tests/developer_affiliate_team_http.rs`, is part of user WIP — handled in their commit.)

**Body:**
```
Three integration tests unconditionally `std::env::set_var(
"TOTP_SECRET_ENCRYPTION_KEY", "0123…ef")` or `BANK_IBAN_ENCRYPTION_KEY` to
literal test values, without verifying DATABASE_URL points at a non-prod
host. If a developer ever exported a prod-shaped DSN and ran
`cargo test --ignored`, real secrets would be re-encrypted under public
test keys.

Added `mod safety { fn assert_database_url_is_local() }` which panics if
DATABASE_URL is non-local (anything other than @localhost / @127.0.0.1 /
@[::1] / @/cloudsql/ unix socket / @martin@). Wired into `pool()` and
into the `install_test_totp_key()` helper so the guard runs before any
`set_var` for an encryption key.

Audit: docs/security-audits/2026-05-19-cddrp-run-1.md (§2.3)
```

---

## PR description draft

Title: `security: CDDRP Run 1 — 131 fixes across backend + frontend (audit-driven)`

Body:

```markdown
# CDDRP Run 1 — Security audit + remediation

This PR lands 131+ security fixes discovered by a 40-agent CDDRP v2.0 protocol
run (audit → verification → bundle → remediation, with quality-review verdict
of READY-TO-COMMIT).

## Summary

| Severity | Found | Confirmed | Fixed |
|---|---|---|---|
| Critical | ~18 | 14 (4 downgraded or FP) | 14 |
| High | ~44 | 38 | 37 |
| Medium | ~70 | n/a (not all verified) | ~50 |
| Low | ~25 | n/a | 30+ |

Build: `cargo check --tests` against live DB — clean at every phase.

## Highlights

### Critical fixes
- **SSRF via affiliate webhook** (`rewards/service.rs`) — added
  `validate_postback_url` call + `redirect::Policy::none()`. Closes
  status-code oracle to GCP cloud metadata service.
- **SSRF via community OG parser** (`community/routes.rs`) — scheme +
  host + IP block + DNS resolve + redirect-disable + 1 MB body cap.
- **76 admin RBAC gates** — `require_permission` added to villa subsystem
  (38 handlers), marketplace compliance (26), dev change_requests (8),
  blockchain control (3), storage retention (2), dashboard (1).
- **Stored XSS in admin community moderation surfaces** — added
  escapeHtml/escapeAttr/escapeJsString helpers and wrapped 27 API-sourced
  interpolations across posts.html + post-detail.html. Closes admin ATO
  vector from crafted display name.

### Resource-exhaustion guards
- **WS channel-map memory leak** — subscriber-count eviction + 100K cap
  on `marketplace/websocket.rs::CHANNELS`.
- **8 unbounded admin queries** — LIMIT 500 + `?page=` pagination on
  users, withdrawals, KYC, affiliate referrals, plus circles/reports
  bounded at 100/200.
- **Mention amplification** — 50-mention cap on community posts (was
  unbounded → 10K mentions = 10K DB queries).
- **OG body unbounded** — 1 MB chunked-read cap.
- **Email-campaign mass-insert** — audience cap 100K + batched
  `INSERT ... VALUES (), ()` (500 rows/chunk).
- **Multipart uploads** — `read_field_capped` helper + migrated 7
  upload handlers from `field.bytes()` to chunked-read.

### Auth-flow hardening
- **`verify_email_page` GET → POST** — closes link-prefetcher token
  consumption. Sibling tokens also invalidated for defense-in-depth.
- **Session invalidation** on role grant/revoke/update + role-permission
  edit + change_email.
- **Test prod-DSN safety guard** — integration tests with hardcoded
  TOTP/IBAN encryption keys now refuse to run against non-local DATABASE_URL.

### Integer overflow guards
- **Concentration-cap i32 wrap** (`marketplace/validation.rs`) — `checked_add`
  closes 80% per-holder compliance-rule bypass.
- **Trade-history net field** (`marketplace/service.rs`) — `checked_mul` row-skip
  + `checked_add/sub` fallback.
- **ERC-1155 event Vec bomb** (`blockchain/event_indexer.rs`) — 10K cap
  on `ids_count` parsed from on-chain hex.

### Infrastructure
- **Fail-closed migrations** — `run_migrations` now `panic!`s on any error
  (was logging + continuing → silent half-applied schema).
- **CORS strip_prefix anchoring** — closes sister-domain bleed footgun.
- **Static 500/404 HTML bodies** — 8 sites no longer leak MiniJinja /
  sqlx error detail to clients.

## Caveats

1. **New permission slugs need a follow-up DB seed migration.** ~50 new
   slugs (`villa.*`, `developer_projects.*`, etc.) work today only via the
   `'all'` shortcut held by `super_admin` / `admin`. For sub-roles to gain
   access, a separate migration must INSERT the slug→role rows.
2. **`/uploads` static-file mount still unauthenticated** (`lib.rs:1722`).
   Deferred pending design decision (signed URLs vs auth-gated `ServeDir`).
3. **2FA login-time bypass not restored** (`auth/routes.rs:253`, `:1556`).
   Comment says "temporarily disabled" — needs explicit go-ahead.
4. **Dividends per-holder math, settlement private key CLI arg, DM
   rate-limit** — all deferred (need design or regression test).

## Documentation

- `docs/security-audits/2026-05-19-army-bug-audit.md` — domain-band audit (157 findings)
- `docs/security-audits/2026-05-19-cddrp-run-1.md` — methodology, findings, verifications, remediation log
- `docs/security-audits/2026-05-19-cddrp-run-1-commit-plan.md` — this commit sequence
- `.claude/commands/audit-fix.md` — CDDRP v2.0 methodology spec
```

---

## Excluded from this commit plan

The following changes appear in `git diff HEAD` but are **user WIP from before this session** — handle in your own commits, separate from the security fixes:

- `AGENTS.md`, `docs/DESIGN.md`, `docs/IMPLEMENTATION_ROADMAP.md` (docs WIP)
- `backend/src/admin/mod.rs` (developer-applications wiring)
- `backend/src/admin/developer_applications.rs` (NEW — untracked)
- `backend/src/developer/{mod,models,routes,service,villa_operations}.rs` (developer subsystem rework)
- `backend/tests/e2e/reports/report.html`
- `backend/tests/admin_developer_applications_http.rs` + `backend/tests/developer_*_http.rs` (NEW)
- `database/202_villa_operations_custom_expenses.sql` + rollback (NEW)
- `database/203_developer_applications.sql` + rollback (NEW)
- `frontend/platform/components/{developer-assets,developer-topbar,head}.html`
- `frontend/platform/developer/*.html` (many)
- `frontend/platform/developer/affiliate-team-analytics.html` (DELETED — user removed)
- `frontend/platform/developer/dashboard.html.bak` (DELETED — user removed)
- `frontend/platform/static/css/developer-*.css`, `my-trading.css`, `ucard.css`, `card-table-standard.css`, mobile-developer-*.css
- `frontend/platform/static/js/developer-*.js`, `leaderboard.js`
- `frontend/platform/static/js/developer-submission-success.js` (DELETED)
- `tests/test_developer_assets_static.py` + new `tests/_developer_static.py` + `tests/e2e/test_developer_*.py` + `tests/test_developer_*_static.py`
- `tests/developer_affiliate_team_http.rs` — has the prod-DSN safety guard from this session merged in, but the file itself is user WIP. Commit the file in user's PR; the safety guard hunks will fold in naturally.

Recommended approach: `git stash` user WIP first → land security commits 1-13 in order → restore user WIP → land WIP commits.
