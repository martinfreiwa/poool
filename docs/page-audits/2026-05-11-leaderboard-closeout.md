# Leaderboard Audit Close-Out — 2026-05-11

This document closes the leaderboard audit that began with the
`2026-04-28-leaderboard.md` page audit. It enumerates the commits, the bugs
fixed, the features added, the test coverage delivered, the deferred items,
and the commands required to confirm the shippable state.

Owner: Claude Opus 4.7
Date: 2026-05-11
Status: SHIPPABLE — all P0/P1/P2 + 3/4 P3 items closed; one P3 item deferred
with documented rationale.

---

## 1. Scope

Four leaderboards live in the platform; this audit explicitly covered the
first three. The fourth (Affiliate Volume Tiers) was scoped out by the
prompt and was not touched in code.

| Leaderboard | Surface | Owner module | Audit coverage |
|---|---|---|---|
| Investor leaderboard | `/leaderboard` | `backend/src/leaderboard/` | Full — code + tests + docs |
| Community XP leaderboard | `/community` (global + per-circle) | `backend/src/community/xp.rs`, `community/routes.rs` | Full — period filter, anonymization, regression tests |
| Circles leaderboard | `/community` (Circle tab) | `backend/src/community/circles.rs` | Full — `is_public` exposure, P1-6 fix |
| Affiliate volume tiers | `/admin/rewards`, `/affiliate-*` | `backend/src/rewards/` | Out of scope — code untouched, only mentioned in docs |

---

## 2. Commits

23 commits dedicated to leaderboard work, plus 4 closeout follow-ups.
Range: `38ce110..6585ec6`.

| SHA | Subject |
|---|---|
| `38ce110` | fix(leaderboard): restore missing UI controls (audit P0-1) |
| `6811498` | fix(admin/leaderboard): send CSRF token on XP grant (audit P0-2) |
| `8368a92` | fix(leaderboard): merge prefs on partial PUT (audit P0-3) |
| `16916b9` | fix(leaderboard): refresh endpoint is POST, not GET (audit P1-4) |
| `4182489` | fix(leaderboard): drop non-active users on refresh (audit P1-5) |
| `ceb080a` | fix(community): expose is_public on circle leaderboard (audit P1-6) |
| `92622b6` | fix(leaderboard): correct formatCompact thresholds (audit P1-7) |
| `814a96a` | fix(tests): refresh leaderboard rank assertions (audit P1-8) |
| `f90f954` | fix(leaderboard): weighted ROI bps precision via NUMERIC (audit task 1) |
| `09abc2f` | chore(db): drop unused leaderboard_snapshots table (audit task 3) |
| `a73878c` | feat(community): windowed XP leaderboard period filter (audit task 2) |
| `2a852b3` | test(leaderboard): SQL regression tests for refresh + prefs + paging (audit task 4) |
| `6798f97` | feat(community): XP leaderboard period dropdown (audit A1) |
| `e78f025` | feat(leaderboard): admin Refresh now button (audit A2) |
| `36eb820` | refactor(leaderboard): document and assert metric_columns allowlist (audit A3) |
| `af2a29e` | fix(leaderboard): derive has_more from count, not page fullness (audit B1) |
| `274c2ed` | test(leaderboard): timeframed tier filter count alignment (audit B2) |
| `2454265` | perf(leaderboard): cache last_updated in AppState to skip MAX() (audit C1) |
| `11f37e6` | fix(leaderboard): demo data dynamic import (audit C2) |
| `003b047` | test(leaderboard): e2e Playwright suite for /leaderboard (audit D2) |
| `d4a93ea` | refactor(backend): extract router for testability (audit D1) |
| `139c4ba` | perf(leaderboard): seed last_updated cache at startup (audit C1 follow-up) |
| `cad2047` | test(leaderboard): service-level has_more regression tests (audit B1 follow-up) |
| `bd213a7` | perf(leaderboard): add 0-300s jitter to refresh interval (audit P3) |
| `6402ab3` | docs(backend): document items exposed by D1 router extraction (audit P3) |
| `6585ec6` | docs(leaderboard): audit closeout — roadmap + audit doc + BROKEN_LOGICS |
| (this doc) | docs(leaderboard): final close-out summary |

---

## 3. Bugs fixed

| Priority | Count | Items |
|---|---|---|
| P0 | 3 | P0-1 missing UI controls; P0-2 admin XP grant CSRF; P0-3 prefs partial PUT clobbered other fields |
| P1 | 5 | P1-4 refresh GET → POST; P1-5 stale non-active rows; P1-6 circle `is_public` hidden; P1-7 formatCompact thresholds; P1-8 stale rank test assertions |
| P2 | 4 | weighted ROI precision (task 1); unused `leaderboard_snapshots` table (task 3); windowed XP filter behavior (task 2); SQL regression tests (task 4) |
| P3 | 3 (of 4) | jitter on refresh interval; last_updated cache (C1 + follow-up); demo data dynamic import (C2). One P3 item deferred — see Section 6. |

Total: 15 numbered bug-fix items closed.

---

## 4. New features

5 user/admin-visible features.

1. **XP leaderboard period dropdown** (`6798f97`, A1) — `?period=week|month|alltime` selector on the community XP leaderboard.
2. **Admin "Refresh now" button** (`e78f025`, A2) — admin-only manual `POST /api/leaderboard/refresh` action surfaced as a button on `/leaderboard`.
3. **Windowed XP period filter** (`a73878c`) — backend `LeaderboardPeriod` enum + windowed `xp_ledger` aggregation in `community::xp::get_user_leaderboard_for_period`.
4. **Hidden-user anonymization on global XP leaderboard** (bundled with `a73878c`) — respects `leaderboard_preferences.visible=false`.
5. **Refresh-interval jitter** (`bd213a7`, closeout P3) — 0-300s random offset per instance + startup log line for ops visibility.

---

## 5. Test coverage added

| Tier | Count | Files |
|---|---|---|
| Backend integration (DB-bound, `--ignored`) | 9 | `backend/tests/leaderboard_integration.rs` |
| HTTP integration (router + tower::oneshot, `--ignored`) | 7 | `backend/tests/leaderboard_http.rs` |
| ROI precision (NUMERIC bps, `--ignored`) | 1 | `backend/tests/leaderboard_roi_precision.rs` |
| Service-level unit (sync, no DB) | varies | `backend/src/leaderboard/service.rs` regression block from `cad2047` |
| e2e Playwright | 7 | `tests/e2e/test_leaderboard.py` |

Verification snapshot (2026-05-11):

- `cargo test --test leaderboard_integration --test leaderboard_http --test leaderboard_roi_precision -- --ignored --test-threads=1` → 17/17 pass.
- `uv run pytest tests/e2e/test_leaderboard.py` → 7/7 pass.
- `cargo test` (unit, non-ignored) → 233/233 pass on the `lib` target.

---

## 6. Known open items (explicitly deferred)

| Item | Reason for deferral | Owner / Tracking |
|---|---|---|
| Live Sentry dashboard + alert on `refresh_all_scores` failure rate (last P3 item) | Out of scope — requires platform observability work beyond the leaderboard module. The error path already logs at `tracing::error!` level; wiring a dashboard belongs to a Sentry/ops follow-up, not this audit. | Tracked in `docs/page-audits/2026-04-28-leaderboard.md` closeout note. |
| Affiliate volume-tier leaderboard | Scoped out by the closeout prompt — no code changes attempted, only docs. | Separate audit if/when needed. |

Nothing else is open. No half-finished commits, no `TODO(leaderboard)` left in code.

---

## 7. Verification commands

Run from `/Users/martin/Projects/poool`.

```bash
# 1. Build is clean with deny-level missing_docs lint.
cd backend && cargo check --all-targets

# 2. Clippy is clean (zero warnings) on the backend lib + test targets.
cd backend && cargo clippy --package poool-backend --all-targets -- -W clippy::all

# 3. Backend integration suite — 16/16 leaderboard + 1 ROI precision.
DATABASE_URL=postgres://martin@localhost/poool \
COMMUNITY_DATABASE_URL=postgres://martin@localhost/poool_community \
  cargo test \
    --test leaderboard_integration \
    --test leaderboard_http \
    --test leaderboard_roi_precision \
    -- --ignored --test-threads=1

# 4. End-to-end Playwright leaderboard suite — 7/7.
uv run pytest tests/e2e/test_leaderboard.py -o addopts="" --timeout=60

# 5. Confirm commit count in the audit range (this doc, lower bound inclusive).
git log --oneline 38ce110~1..HEAD | wc -l
```

Expected results on a clean checkout:

- Step 1: `Finished` with zero warnings.
- Step 2: `Finished` with no `warning:` lines printed.
- Step 3: `test result: ok. 9 passed`, `test result: ok. 7 passed`, `test result: ok. 1 passed`.
- Step 4: `7 passed`.
- Step 5: ≥ 65 (audit + adjacent community work).

---

## 8. Files of record

- `docs/page-audits/2026-04-28-leaderboard.md` — original audit, with closing note appended.
- `docs/page-audits/2026-05-11-leaderboard-closeout.md` — this document.
- `docs/IMPLEMENTATION_ROADMAP.md` — Live Agent Log entry dated 2026-05-11 12:00.
- `docs/issue-tracking/BROKEN_LOGICS.md` — PAGE-ISSUE-0005 marked resolved with commit SHA `11f37e6`.

---

## 9. Sign-off

The leaderboard surfaces (investor, community XP, circles) ship in a
shippable, documented, fully-tested state. No further leaderboard work is
required before pushing to `main`.
