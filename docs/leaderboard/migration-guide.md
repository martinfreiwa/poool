# Migration Guide — Visibility Filter (2026-05-16)

Operational guide for deploying the visibility-filter behavior change.
Read this in full before shipping.

---

## What changed

**Before**: Every user with a `leaderboard_scores` row appeared in the
public listing. Users who hadn't opted in (`visible = false`) were shown
with a pseudonymized display name: `'Investor #' || substring(user_id,
1, 6)`.

**After**: Hidden users (`visible = false`) are filtered out of the
public listing entirely. The viewer also never sees themselves in their
own listing (they see their rank in the "Your Standing" card). Visible
users are re-ranked sequentially via `ROW_NUMBER()` so there are no
rank gaps from filtered-out users.

---

## Why

The previous behavior was a privacy gap reported via the UX audit:

- UI label: *"Hidden from public rankings"*
- Actual behavior: user still appeared with all metric values, just under a pseudonym.

User opt-out → no public listing presence. Anything less is a trust-eroding semantic mismatch.

---

## User-visible impact

| Population | What they'll notice |
|---|---|
| Users with `visible = true` | No change. They still appear, with their real display name. |
| Users with `visible = false` AND a `leaderboard_scores` row (most users — default is `false`) | They DISAPPEAR from the public listing. Their own "Your Standing" still shows their rank. |
| New users post-deploy | No change to defaults. `visible` is still opt-in (`false`). |

**Effect on listing size**: in dev, the listing went from 11 visible (mostly-anonymized) users → 4 visible users after the filter took effect, because only 4 had opted in. Expect a similar 50-80% drop in production listing size if your opt-in rate is around 10-20%.

---

## Database-level changes

**None.** The fix is purely in the read query. No migration, no schema change, no data backfill.

The `leaderboard_preferences.visible` column already defaults to `FALSE`
(opt-in) — see `database/023_leaderboard.sql:42`.

---

## Code-level changes

| File | Change |
|---|---|
| `backend/src/leaderboard/service.rs` | Both `get_rankings_alltime` and `get_rankings_timeframed` now wrap their CTEs in a `visible` filter (`COALESCE(lp.visible, false) = true AND user_id <> $current_user`). Pseudonym CASE removed. `ROW_NUMBER()` re-ranks within the visible set. Tier filter treats NULL `user_tiers.tier_id` as Intro (1). |
| Count queries | Same predicates as the listing — they must agree exactly or pagination breaks. |
| `LeaderboardEntry.asset_mix` | New field on the JSON response. Empty array for entries 4+. |

---

## Pre-deploy checklist

- [ ] **Run the full test suite** — 36 tests must be green:
  ```sh
  DATABASE_URL=… cargo test --test leaderboard_integration \
                            --test leaderboard_http \
                            --test leaderboard_roi_precision \
                            --test leaderboard_production_audit \
                            -- --ignored --test-threads=1
  ```
- [ ] **Audit communication plan**: if your opt-in rate is low, the listing will visibly shrink. Decide whether to send a heads-up email to users currently appearing under a pseudonym so they can opt in if desired.
- [ ] **Sentry alert rule** for `message:"leaderboard"` is active and routes to on-call.
- [ ] **Manual smoke test** plan: opt one test user in, deploy, verify they appear and the pseudonymized users do not.

---

## Deploy procedure

1. **Push** to main:
   ```sh
   git push origin main
   ```

2. **Run the deploy workflow** (manual — not auto-triggered by push):
   ```sh
   gh workflow run deploy.yml
   ```

3. **Watch the rollout**:
   ```sh
   gh run watch
   ```

4. **Post-deploy verification** (within 5 min of deploy completing):
   ```sh
   # Visible-user count, before vs after
   psql $PROD_DB -c "SELECT COUNT(*) AS visible_users FROM leaderboard_preferences WHERE visible = true;"
   psql $PROD_DB -c "SELECT COUNT(*) AS scored_users  FROM leaderboard_scores;"

   # API response — should only show visible users, ranks 1..N sequential
   curl -s -H "Cookie: poool_session=<test-admin>" \
        https://api.poool.example/api/leaderboard?per_page=20 | jq '.rankings | map(.rank)'
   ```

5. **Monitor** Sentry + tracing for ~30 min:
   - `Leaderboard listing failed:` → unexpected query-time error
   - `Background leaderboard refresh failed:` → unrelated to this deploy, but watch anyway
   - 429 rate-limit spikes from the same user → could be a frontend retry loop

---

## Rollback procedure

The change is purely a read-path query refactor — no schema change to undo. To roll back:

1. Revert the deploy:
   ```sh
   gh workflow run deploy.yml -f ref=<previous-good-sha>
   ```

2. Verify the previous behavior is back:
   ```sh
   curl -s -H "Cookie: …" /api/leaderboard?per_page=5 | jq '.rankings[].display_name'
   # → expect to see some "Investor #abcdef" placeholders again
   ```

3. **No data restoration needed** — the visibility flag was never modified by the migration.

---

## Long-term cleanup (post-stabilization)

After the change has been live for a sprint and no rollback has happened:

- [ ] Update help docs / FAQ that previously described the pseudonym behavior.
- [ ] Audit user-facing copy on `/settings → Leaderboard` to confirm "Show on Leaderboard" semantically reads as "appear in the public listing", which is now what it does.
- [ ] Consider an opt-in nudge in onboarding: "Want to compete on the leaderboard? Enable Show on Leaderboard in Settings."
- [ ] (Optional) Drop the `'Investor #' || substring(...)` pattern from the community leaderboard too, if you want unified semantics. The community endpoint still uses the old pattern — see `backend/src/community/routes.rs:5575`.

---

## Why these specific tests are the rollback signal

If you've rolled back, the following tests will FAIL on the rolled-back binary against the prod-state DB:

| Test | What it would prove |
|---|---|
| `visibility_filter_excludes_hidden_users_and_reranks` | Listing returns 4 rows (visible only) with ranks 1-4 |
| `visible_viewer_excluded_from_own_listing` | Listing for a visible viewer doesn't contain themselves |
| `pseudonym_pattern_never_appears_in_response` | No `'Investor #'` strings in response |

If you see these tests failing post-rollback, that's the EXPECTED state of the rolled-back code. If they fail on the NEW code, you have a regression.

---

## FAQ

**Q: A user complains they're no longer on the leaderboard but they thought they were public.**
A: Check `leaderboard_preferences.visible` for their user_id. If `false`, they were never opted in — under the old behavior they were pseudonymized, which they may have read as "private but appearing." Send them to `/settings → Leaderboard → Show on Leaderboard`. Mention this in the change announcement.

**Q: The listing is empty after deploy.**
A: Confirm at least some users have `visible = true`. In dev we seed 4 visible users manually for screenshots — production may need a similar nudge if opt-in rate is near zero.

**Q: My rank in "Your Standing" is wrong (says #5 but the public listing only shows 4 users).**
A: This is correct behavior. `my_rank` is computed from `leaderboard_scores.rank_invested` (counts ALL users) — your "real" rank among everyone who could appear. The listing only shows OPTED-IN users, so it can be much shorter. The discrepancy is intentional: opt-out shouldn't help you climb relative-rank.

**Q: Can I get back to the pseudonym behavior temporarily?**
A: Yes — git revert the visibility filter changes in `service.rs`. The change is isolated to that file (plus tests). The pseudonym CASE statement is in conversation history if you need to reconstruct it.
