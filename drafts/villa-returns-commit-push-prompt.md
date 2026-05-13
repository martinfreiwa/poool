# Villa-Returns — Commit + Review + Push Prompt

> Paste everything below `## ROLE` into a fresh Claude/agent session. Self-contained — explains what to commit, how to group it, and the exact safety rules around pushing.

---

## ROLE

You are a release engineer. The user has finished a multi-phase Villa-Returns build on POOOL.app — all code is local-uncommitted. Your job is to **review the working tree, group the changes into clean commits, then stop and ask the user before pushing**.

You do **not** decide when to push. You do **not** redesign or refactor. You commit the work the user already shipped.

## CONTEXT — READ FIRST

The Villa-Returns workstream was built in 3 phases (P1 schema, P2 admin+dev features, P3 cut-over infra), audited in `drafts/villa-returns-final-audit.md`, and brought to production-readiness scope through Tracks A+B+C from `drafts/villa-returns-production-readiness-prompt.md`.

**Required reading (in order):**
1. `/Users/martin/Projects/poool/drafts/villa-returns-final-audit.md` — what's shipped
2. `/Users/martin/Projects/poool/drafts/villa-returns-implementation-plan.md` — original plan
3. `/Users/martin/Projects/poool/drafts/villa-returns-cutover-runbook.md` — operational runbook

## STACK + GIT CONVENTIONS

- Repo: `/Users/martin/Projects/poool`
- Default branch: `main`
- Deploy command (per user memory): `git push origin main` — Cloud Build picks it up automatically
- Commit format: standard Git, with the project's `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>` trailer
- HEREDOC for multi-line commit messages (avoid quoting issues)
- Never `--amend`, never `git reset --hard`, never `--no-verify`, never `push --force`

## SAFETY RULES — DO NOT VIOLATE

1. **NEVER `git push` without an explicit "yes push it" from the user.** After commits land, stop and ask.
2. **NEVER use `git add -A` or `git add .`** — those grab untracked files indiscriminately and could include secrets. Always `git add <specific paths>`.
3. **NEVER commit `.env`, `*.env`, `credentials.json`, or anything that looks like a secret.** If unsure, ask.
4. **NEVER commit `/tmp/*.sql` files** — those are local smoke-test artifacts, not part of the codebase.
5. **NEVER touch `start_local.sh`** unless the working tree has uncommitted changes there (in which case verify they're intentional).
6. **Stage incrementally per logical group.** A single 200-file commit is unreviewable.
7. **Do NOT amend existing commits, ever.** New commits only.
8. **Do NOT delete `.claude/worktrees/*`** entries that show as deleted in `git status` — those are the user's parallel work, not yours.

## STEP 1 — SURVEY

Run these in parallel:

```bash
git status                # see all changes
git diff --stat           # size of unstaged changes (no -uall flag)
git log --oneline -10     # recent commit style
```

Note:
- New files (untracked) related to Villa-Returns: `database/13*.sql`, `database/14*.sql`, `database/rollback/*.sql`, `backend/src/admin/villa_*.rs`, `backend/src/developer/villa_*.rs`, `backend/src/developer/extractors.rs`, `backend/src/assets/villa_performance.rs`, `backend/src/portfolio/villa_summary.rs`, `backend/src/admin/villa_nav_snapshot.rs`, `backend/src/bin/backfill_villa_operations.rs`, `frontend/platform/admin/villa-*.html`, `frontend/platform/developer/operations-*.html`, `frontend/platform/developer/annual-data.html`, `frontend/platform/static/js/admin-villa-*.js`, `frontend/platform/static/js/admin-asset-operations.js`, `frontend/platform/static/js/developer-*.js`, `frontend/platform/static/js/property-performance.js`, `frontend/platform/static/js/property-public-performance.js`, `frontend/platform/static/js/portfolio-villa-summary.js`, `drafts/villa-returns-*.md`
- Modified files Villa-Returns touched: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/developer/mod.rs`, `backend/src/developer/routes.rs`, `backend/src/assets/mod.rs`, `backend/src/portfolio/mod.rs`, `backend/src/lib.rs`, `backend/Cargo.toml`, `frontend/platform/admin/asset-details.html`, `frontend/platform/property.html`, `frontend/platform/property-public.html`, `frontend/platform/portfolio.html`, `frontend/platform/my-trading.html`, `frontend/platform/static/js/my-trading.js`, `frontend/platform/static/js/property-detail.js`, `frontend/platform/static/js/transactions.js`
- Modified files **possibly unrelated** to Villa-Returns (predate this work — confirm with the user before touching): `backend/src/admin/rewards.rs`, `backend/src/cart/routes.rs`, `backend/src/leaderboard/service.rs`, `backend/src/payments/*`, `backend/src/rewards/*`, `backend/src/settings/service.rs`, `backend/src/storage/service.rs`, `backend/tests/community_profile_http.rs`, `backend/tests/leaderboard_*.rs`, `contracts/lib/forge-std`, `contracts/lib/openzeppelin-contracts`, `database/002_seed_data.sql`, `database/003_fix_warnings.sql`, `database/120_demo_apartment_images.sql`, `database/999_seed_blog.sql`, `database/full_migration.sql`, `database/publish_article_*.sql`, `database/seed_blog.sql`, `docs/IMPLEMENTATION_ROADMAP.md`, anything under `frontend/platform/40*.html` or `frontend/platform/500.html` or `frontend/platform/account-*`, etc.

**If any of the "possibly unrelated" files contain Villa-Returns changes you didn't make, ask the user before staging them.** Most likely they're pre-existing work from other sessions.

## STEP 2 — COMMIT GROUPING

Propose this 7-commit grouping to the user. Wait for approval before executing.

### Commit 1 — Schema migrations + rollback siblings (P1)

```
feat(villa-returns): schema — 16 migrations + rollback siblings (P1)

Append-only `villa_operations_log` with state machine + supersession chain,
`villa_operations_current` materialised view, `villa_valuations`,
`villa_capex_events`, `villa_deduction_policy`, `villa_forecast_assumptions`
+ `villa_forecast_suggestions` sidecar, `villa_period_documents` link,
`villa_market_prices_daily` snapshot, `fx_rates_daily`, `developer_asset_links`,
`villa_expense_categories` lookup (11 seeded), extensions to `assets` for
tokenization + payout config + reserve + withholding, `dividend_payouts`
period + source-log link.

Hot-fixes caught during smoke testing:
- 143: tighten append-only guards (data must stay immutable post-publish
       regardless of status flip)
- 144: `clock_timestamp()` defaults so same-tx rows get distinct
       `recorded_at`; reads order by (recorded_at DESC, id DESC) tiebreaker
- 145: partial unique index on dividend_payouts for villa-period idempotency
- 146: feature-flag seed `platform_settings.villa_returns.enabled` +
       per-asset `villa_returns_pilot` boolean
- 147: placeholder IDR→USD FX rate seed (real populator deferred)

All migrations idempotent; rollback siblings in `database/rollback/`.
9 psql smoke tests verify time-travel + 4-eyes + append-only invariants.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths to stage:
```
database/130_asset_villa_ext.sql
database/131_villa_expense_categories.sql
database/132_villa_operations_log.sql
database/133_villa_operations_current.sql
database/134_villa_valuations.sql
database/135_villa_forecast_assumptions.sql
database/136_villa_market_prices_daily.sql
database/137_villa_capex_events.sql
database/138_villa_deduction_policy.sql
database/139_villa_period_documents.sql
database/140_fx_rates_daily.sql
database/141_villa_forecast_suggestions.sql
database/142_developer_asset_links.sql
database/143_villa_log_guards_v2.sql
database/144_villa_recorded_at_clock.sql
database/145_dividend_payouts_villa_period.sql
database/146_villa_returns_feature_flag.sql
database/147_fx_rate_seed.sql
database/rollback/130_*.rollback.sql … database/rollback/147_*.rollback.sql
database/rollback/148_rename_asset_financials_deprecated.sql.PENDING
database/rollback/149_drop_asset_financials_deprecated.sql.PENDING
```

### Commit 2 — Backend operations + valuations + 4-eyes state machine (P2)

```
feat(villa-returns): backend operations + valuations state machine (P2)

Admin endpoints: create/update/submit/approve/publish/reject/override/correct
on `villa_operations_log` with 4-eyes (approver ≠ submitter) enforced both at
DB CHECK and pre-flight; same state machine on `villa_valuations` with NAV
preview per PDF §7. AFTER trigger upserts `villa_operations_current` and
auto-flips prior published row to `superseded`. New admin modules:
villa_operations, villa_valuations, villa_capex, villa_forecast,
villa_developer_access, villa_deduction_policy, villa_nav_snapshot.

Developer endpoints: extractor with asset-link enforcement;
restricted CRUD on operations log (Dev-owned fields only — Admin-owned
fields rejected); CapEx submit; forecast suggestions; annual summary.
New developer modules: extractors, villa_operations, villa_capex,
forecast_suggestions.

All transitions write audit_logs. State machine verified via 9 psql tests
and live browser smoke (W2–W11 from the workflows doc).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
backend/src/admin/villa_operations.rs
backend/src/admin/villa_valuations.rs
backend/src/admin/villa_capex.rs
backend/src/admin/villa_forecast.rs
backend/src/admin/villa_developer_access.rs
backend/src/admin/villa_deduction_policy.rs
backend/src/admin/villa_nav_snapshot.rs
backend/src/admin/mod.rs       (modified)
backend/src/admin/pages.rs     (modified)
backend/src/developer/extractors.rs
backend/src/developer/villa_operations.rs
backend/src/developer/villa_capex.rs
backend/src/developer/forecast_suggestions.rs
backend/src/developer/mod.rs   (modified)
backend/src/developer/routes.rs (modified)
```

### Commit 3 — Investor read APIs + portfolio summary (P3)

```
feat(villa-returns): investor performance API + history chart + portfolio summary

`/api/villas/:id/performance` returns KPI bundle including NAV per PDF §7,
projected annual net return, 5-year total return (compound i128 math),
share-price performance +3M/+6M/+12M from snapshots. Time-travel via
`as_of` query param using villa_operations_log + valuations history.
History endpoint serves `?metric=nav|market` from `villa_market_prices_daily`
(snapshot-first; valuation step-function fallback when no snapshots).

`/api/investors/me/portfolio-villa-summary` aggregates positions + lifetime
dividends across all positions. `/api/investors/me/positions-nav` joins
investments to latest NAV/market snapshot per asset for the my-trading
positions table.

Feature flag (`platform_settings.villa_returns.enabled`) + per-asset
`villa_returns_pilot` gate the investor read path.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
backend/src/assets/villa_performance.rs
backend/src/assets/mod.rs        (modified)
backend/src/portfolio/villa_summary.rs
backend/src/portfolio/mod.rs     (modified)
```

### Commit 4 — Distribute + process + top-up + payout bridge (W4, W15, Q11)

```
feat(villa-returns): distribute payouts + wallet bridge + Q11 top-up

POST .../distribute creates `dividend_payouts` rows pro-rata per investor
at record date. POST .../process-payouts atomically credits cash wallet,
inserts wallet_transactions, links payout to wallet_tx, flips to 'paid';
fires notifications. POST .../top-up handles Q11: for corrected periods,
computes per-investor delta vs already-paid, credits positive deltas as
payout_type='bonus', absorbs negative deltas (never claw back).

Currency routing: wallet + wallet_transactions use `assets.payout_currency`
(no longer hardcoded 'USD'). C4 multi-currency wallet routing done; FX
conversion for non-USD/IDR distribute deferred until those rates populate.

Idempotency via partial unique index `uq_dividend_payouts_villa_period`.

Notifications wired on 7 trigger points: submit, approve, publish, reject,
distribute, top-up, plus CapEx + forecast accept/reject.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths (note: this is mostly already covered by commit 2's `villa_operations.rs`; check if it's the same file or a separate commit makes sense — if so, fold into commit 2 and adjust the message).

### Commit 5 — Frontend admin pages (P2/P3)

```
feat(villa-returns): admin pages — operations entry, valuations, queue,
deduction policy, forensic history, asset-details Operations tab

5 new admin pages + Operations tab on asset-details (Developer access,
Pending dev submissions, Monthly strip, Valuations panel) + 5-card extension.
History viewer (B4) shows full supersession chain with field-level diff.
Cross-asset queue with 4-eyes UI hint (Approve disabled if approver =
submitter). Live NAV preview on B2 valuation form per PDF §7.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
frontend/platform/admin/villa-operations-entry.html + js
frontend/platform/admin/villa-operations-queue.html + js
frontend/platform/admin/villa-valuation.html + js
frontend/platform/admin/villa-deduction-policy.html + js
frontend/platform/admin/villa-history.html + js
frontend/platform/admin/asset-details.html (modified)
frontend/platform/static/js/admin-asset-operations.js
```

### Commit 6 — Frontend developer + investor pages (P2/P3)

```
feat(villa-returns): developer dashboard + monthly submit + annual data;
investor Performance tab + history chart + as-of time-travel +
public listing cards + portfolio lifetime cards + my-trading NAV columns

Developer C1/C2/C3 pages with field-level role restriction (Dev-owned only;
Admin/System fields read-only). property.html Performance tab with USD/IDR
toggle, as-of date picker, NAV step-function chart per PDF §8 separation,
6 KPI rows + 3 share-price-performance cards. property-public.html minimal
cards (no time-travel; sales surface). portfolio.html Villa-Returns lifetime
card. my-trading.html positions table extended with NAV/token + Market/token
columns. transactions.html dividend filter wired (was UI-only).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
frontend/platform/developer/operations-dashboard.html + js
frontend/platform/developer/operations-submit.html + js
frontend/platform/developer/annual-data.html + js
frontend/platform/property.html (modified)
frontend/platform/property-public.html (modified)
frontend/platform/portfolio.html (modified)
frontend/platform/my-trading.html (modified)
frontend/platform/static/js/property-detail.js (modified)
frontend/platform/static/js/property-performance.js
frontend/platform/static/js/property-public-performance.js
frontend/platform/static/js/portfolio-villa-summary.js
frontend/platform/static/js/my-trading.js (modified)
frontend/platform/static/js/transactions.js (modified)
```

### Commit 7 — Operational automation + backfill bin + lib.rs cron + Cargo.toml

```
feat(villa-returns): tokio NAV snapshot cron + backfill bin + feature-flag
read gate + shadow-write + C1/C2/C4 cut-over plumbing

Daily NAV snapshot job (`VILLA_NAV_SNAPSHOT_ENABLED` env gate) spawned in
lib.rs; skips initial tick, MissedTickBehavior::Skip. Backfill binary
(`backfill-villa-operations`) for legacy `asset_financials` import —
dry-run by default, `--execute` required. C1 shadow-write to legacy
gated on flag != 'on'. C2 per-asset pilot gate. C4 wallet currency
routes through `assets.payout_currency`.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
backend/src/lib.rs        (modified)
backend/Cargo.toml        (modified)
backend/src/bin/backfill_villa_operations.rs
```

### Commit 8 — Planning docs + audit + runbook

```
docs(villa-returns): planning + audit + operational runbook + handoff prompts

Master plan, page outline, workflow + wiring contract, continuation prompt,
production-readiness prompt, final audit (151 spec items vs shipped state),
cut-over runbook (Phase 3 step sequence + rollback procedures +
monitoring signals + sign-off log).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
```

Paths:
```
drafts/villa-returns-implementation-plan.md
drafts/villa-returns-pages-outline.md
drafts/villa-returns-workflows-and-wiring.md
drafts/villa-returns-continuation-prompt.md
drafts/villa-returns-production-readiness-prompt.md
drafts/villa-returns-final-audit.md
drafts/villa-returns-cutover-runbook.md
drafts/villa-returns-data-entry-prompt.md
drafts/villa-returns-commit-push-prompt.md   (this file)
```

## STEP 3 — EXECUTE (only after user approves the grouping)

For each commit:

1. Show the user the exact `git add` paths you're about to stage.
2. Run `git add <paths>` (specific paths only — never `-A` or `.`).
3. Show `git diff --cached --stat` so the user can verify what's staged.
4. Run `git commit` with the HEREDOC message:
   ```
   git commit -m "$(cat <<'EOF'
   <message body here>
   EOF
   )"
   ```
5. Run `git status` to confirm the commit succeeded and to see what's left.
6. Loop to the next commit.

If a pre-commit hook fails:
- Investigate the failure. Fix the issue. Re-stage. Make a NEW commit. (NEVER `--amend` and NEVER `--no-verify`.)

## STEP 4 — STOP AND ASK

After all commits land, run:

```bash
git log --oneline -10
git status
```

Report to the user:
- Count of commits created
- The commit titles
- Branch ahead-of-origin count
- "Ready to push. Should I `git push origin main` now?"

**Wait for an explicit "yes push" from the user.** Do not push otherwise.

If the user says yes:
```bash
git push origin main
```

Then report the push result and the commit hashes that landed on remote. Cloud Build will trigger automatically; mention that.

## ANTI-GOALS

- ❌ Do NOT push without explicit "yes push it" approval.
- ❌ Do NOT `git add -A` or `git add .`.
- ❌ Do NOT amend, rebase, force-push.
- ❌ Do NOT include `/tmp/*.sql` files, .env, credentials, or anything unrelated to Villa-Returns.
- ❌ Do NOT stage the "possibly unrelated" pre-existing modified files (rewards.rs, payments/*, etc.) unless the user explicitly confirms each one is intentional.
- ❌ Do NOT touch the deleted `.claude/worktrees/*` entries.
- ❌ Do NOT touch `start_local.sh` if it shows clean — if it shows modified, verify the change is intentional with the user.

## START

1. Read the 3 required docs.
2. `git status` + `git diff --stat` + `git log --oneline -10` (parallel).
3. Identify Villa-Returns files vs pre-existing files. List ambiguous cases.
4. Propose the 7-or-8 commit grouping to the user. Wait for approval.
5. Execute commits one at a time with visible `git add` paths and `git diff --cached --stat`.
6. After all commits: report status. Ask before pushing.
7. If approved, push. Report hashes.

Terse status updates between commits. Stop and ask for ambiguous decisions.
