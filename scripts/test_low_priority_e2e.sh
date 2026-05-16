#!/usr/bin/env bash
# E2E: Low-priority community features.
#   - Challenges (list + progress)
#   - Expert AMAs (list + detail)
#   - Badges (read)
#   - Verified-owner requests (submit + list + duplicate-reject)
#   - Polls (create post-with-poll + vote + results)
source "$(dirname "$0")/lib/community_test_helpers.sh"
trap cleanup_users EXIT

step "Seed Alice + Bob"
mk_user alice e2e-lowprio-a
mk_user bob   e2e-lowprio-b

# ── Helper: pick any published asset (verified-owner needs one) ──
ASSET_ID=$(psql "$DB_CORE" -tA -c "SELECT id FROM assets WHERE published = TRUE LIMIT 1" 2>/dev/null | tr -d '[:space:]')
[ -n "$ASSET_ID" ] && ok "test asset = ${ASSET_ID:0:8}…" || { bad "no published asset in DB"; summary; exit 1; }

# ══════════════════════════════════════════════════════════════════
# CHALLENGES
# ══════════════════════════════════════════════════════════════════
step "1. List challenges (any authed user)"
RES=$(http_get alice "/api/community/challenges")
assert_status 200 "$RES" "GET /api/community/challenges"
COUNT=$(body_of "$RES" | jq '(.challenges? // .) | length' 2>/dev/null)
note "challenges returned: $COUNT"
[ "${COUNT:-0}" -ge 0 ] && ok "Challenges endpoint responsive" || bad "invalid response shape"

step "2. Anonymous → 401"
ANON=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/community/challenges")
[ "$ANON" = "401" ] || [ "$ANON" = "403" ] && ok "Anonymous → $ANON" || bad "expected 401/403, got $ANON"

# ══════════════════════════════════════════════════════════════════
# EXPERT AMAs
# ══════════════════════════════════════════════════════════════════
step "3. List AMAs"
RES=$(http_get alice "/api/community/amas")
assert_status 200 "$RES" "GET /api/community/amas"
COUNT=$(body_of "$RES" | jq '(.amas? // .) | length' 2>/dev/null)
note "AMAs returned: $COUNT"

step "4. AMA detail — try with a real id if any exist"
AMA_ID=$(body_of "$RES" | jq -r '(.amas? // .) | .[0].id // empty')
if [ -n "$AMA_ID" ]; then
  DETAIL=$(http_get alice "/api/community/amas/$AMA_ID")
  DST=$(status_of "$DETAIL")
  if [ "$DST" = "200" ]; then
    ok "GET /api/community/amas/:id → 200"
  else
    bad "expected 200, got $DST"
  fi
else
  # Fallback: random UUID must 404 (proves route is mounted + not 500).
  RAND=$(uuidgen | tr '[:upper:]' '[:lower:]')
  RES=$(http_get alice "/api/community/amas/$RAND")
  ST=$(status_of "$RES")
  [ "$ST" = "404" ] && ok "Unknown AMA id → 404 (route mounted, no data)" \
                    || bad "expected 404 for unknown id, got $ST"
fi

# ══════════════════════════════════════════════════════════════════
# BADGES
# ══════════════════════════════════════════════════════════════════
step "5. Badge detail — random UUID returns 404, real ID returns 200"
BADGE_ID=$(psql "$DB_COMM" -tA -c "SELECT id FROM badges LIMIT 1" 2>/dev/null | tr -d '[:space:]')
if [ -n "$BADGE_ID" ]; then
  RES=$(http_get alice "/api/community/badges/$BADGE_ID")
  ST=$(status_of "$RES")
  if [ "$ST" = "200" ]; then
    ok "Real badge → 200"
  else
    bad "expected 200 for known badge, got $ST"
  fi
else
  note "no badges seeded — testing 404 path only"
  RAND=$(uuidgen | tr '[:upper:]' '[:lower:]')
  RES=$(http_get alice "/api/community/badges/$RAND")
  ST=$(status_of "$RES")
  [ "$ST" = "404" ] && ok "Unknown badge → 404" || bad "expected 404, got $ST"
fi

# ══════════════════════════════════════════════════════════════════
# VERIFIED-OWNER REQUESTS
# ══════════════════════════════════════════════════════════════════
step "6. Submit verified-owner request"
RES=$(http_post alice "/api/community/verified-owner-requests" \
  "$(printf '{"asset_id":"%s","evidence_url":"https://example.com/proof.pdf","note":"E2E test"}' "$ASSET_ID")")
ST=$(status_of "$RES")
if [ "$ST" = "200" ]; then
  REQ_ID=$(body_of "$RES" | jq -r '.id // empty')
  ok "POST /verified-owner-requests → 200 (id=${REQ_ID:0:8}…)"
else
  bad "expected 200, got $ST"
  note "body: $(body_of "$RES" | head -c 200)"
fi

step "7. Duplicate request rejected with 409"
RES=$(http_post alice "/api/community/verified-owner-requests" \
  "$(printf '{"asset_id":"%s","evidence_url":"x"}' "$ASSET_ID")")
ST=$(status_of "$RES")
[ "$ST" = "409" ] && ok "Duplicate → 409 Conflict" || bad "expected 409, got $ST"

step "8. List my verified-owner requests"
RES=$(http_get alice "/api/community/verified-owner-requests")
assert_status 200 "$RES" "GET /verified-owner-requests"
COUNT=$(body_of "$RES" | jq '(.requests? // .) | length')
[ "$COUNT" -ge 1 ] && ok "My request list contains ≥1 entry ($COUNT)" \
                  || bad "expected ≥1, got $COUNT"

# Cleanup the verified-owner row before EXIT trap (FK chain).
psql "$DB_COMM" -q -c "DELETE FROM verified_owner_requests WHERE user_id = '$(_u_id alice)'" > /dev/null 2>&1 || true

# ══════════════════════════════════════════════════════════════════
# POLLS
# ══════════════════════════════════════════════════════════════════
step "9. Alice creates a post with an attached poll"
POLL_POST=$(http_post alice "/api/community/posts" '{
  "content":"What is best?",
  "post_type":"general",
  "poll_question":"Best asset class?",
  "poll_options":["Real Estate","Commodities","Coffee"],
  "poll_expires_hours":72
}')
assert_status 200 "$POLL_POST" "POST /posts (with poll)"
POLL_POST_ID=$(body_of "$POLL_POST" | jq -r '.id // .post_id // empty')

step "10. Get poll results for the post"
if [ -n "$POLL_POST_ID" ]; then
  RES=$(http_get alice "/api/community/posts/$POLL_POST_ID/poll")
  ST=$(status_of "$RES")
  if [ "$ST" = "200" ]; then
    OPT_COUNT=$(body_of "$RES" | jq '(.options? // []) | length')
    if [ "$OPT_COUNT" -ge 3 ]; then
      ok "Poll has $OPT_COUNT options"
      OPTION_ID=$(body_of "$RES" | jq -r '(.options? // []) | .[0].id // empty')
    else
      bad "expected ≥3 options, got $OPT_COUNT"
    fi
  else
    bad "GET /poll → $ST"
    note "body: $(body_of "$RES" | head -c 200)"
  fi
else
  bad "no post_id returned"
fi

step "11. Bob votes on the poll"
if [ -n "${OPTION_ID:-}" ]; then
  RES=$(http_post bob "/api/community/posts/$POLL_POST_ID/poll/vote" \
    "$(printf '{"option_id":"%s"}' "$OPTION_ID")")
  ST=$(status_of "$RES")
  if [ "$ST" = "200" ]; then
    ok "POST /poll/vote → 200"
  else
    bad "expected 200, got $ST"
    note "body: $(body_of "$RES" | head -c 200)"
  fi
else
  bad "no option_id captured — cannot vote"
fi

step "12. Vote count reflects the vote"
RES=$(http_get alice "/api/community/posts/$POLL_POST_ID/poll")
TOTAL=$(body_of "$RES" | jq '(.options? // []) | map(.vote_count // 0) | add')
[ "${TOTAL:-0}" -ge 1 ] && ok "Total votes = $TOTAL (≥1)" \
                       || bad "expected ≥1 total votes, got $TOTAL"

step "13. Invalid option_id rejected (400)"
RAND=$(uuidgen | tr '[:upper:]' '[:lower:]')
RES=$(http_post bob "/api/community/posts/$POLL_POST_ID/poll/vote" \
  "$(printf '{"option_id":"%s"}' "$RAND")")
ST=$(status_of "$RES")
[ "$ST" = "400" ] && ok "Bogus option_id → 400" || bad "expected 400, got $ST"

step "14. Poll vote on non-poll post → 404"
NONPOLL=$(http_post alice "/api/community/posts" '{"content":"plain","post_type":"general"}')
NONPOLL_ID=$(body_of "$NONPOLL" | jq -r '.id // empty')
RES=$(http_post bob "/api/community/posts/$NONPOLL_ID/poll/vote" \
  "$(printf '{"option_id":"%s"}' "$RAND")")
ST=$(status_of "$RES")
[ "$ST" = "404" ] && ok "Vote on non-poll post → 404" || bad "expected 404, got $ST"

summary
