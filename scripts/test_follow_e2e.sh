#!/usr/bin/env bash
# E2E: Follow / unfollow + follower/following lists.
source "$(dirname "$0")/lib/community_test_helpers.sh"
trap cleanup_users EXIT

step "Seed Alice + Bob"
mk_user alice e2e-follow-a
mk_user bob   e2e-follow-b
A=$(_u_id alice); B=$(_u_id bob)

step "1. Alice follows Bob"
RES=$(http_post alice "/api/community/follow/$B" "")
assert_status 200 "$RES" "POST /follow/:id"

step "2. Bob's followers list contains Alice"
RES=$(http_get bob "/api/community/profile/$B/followers")
assert_status 200 "$RES" "GET /profile/:id/followers"
if body_of "$RES" | jq -e --arg u "$A" '(.followers? // .users? // .) | map(.user_id // .id) | index($u)' > /dev/null; then
  ok "Followers list includes Alice"
else
  bad "Alice not in Bob's followers list"
  note "body: $(body_of "$RES" | head -c 200)"
fi

step "3. Alice's following list contains Bob"
RES=$(http_get alice "/api/community/profile/$A/following")
assert_status 200 "$RES" "GET /profile/:id/following"
if body_of "$RES" | jq -e --arg u "$B" '(.following? // .users? // .) | map(.user_id // .id) | index($u)' > /dev/null; then
  ok "Following list includes Bob"
else
  bad "Bob not in Alice's following list"
fi

step "4. Follower count on Bob's profile increased"
RES=$(http_get alice "/api/community/profile/$B")
CNT=$(body_of "$RES" | jq -r '.follower_count // 0')
[ "$CNT" -ge 1 ] && ok "Bob.follower_count = $CNT" || bad "expected ≥1, got $CNT"

step "5. Alice unfollows Bob"
RES=$(http_del alice "/api/community/follow/$B")
ST=$(status_of "$RES")
if [ "$ST" = "200" ] || [ "$ST" = "204" ]; then
  ok "DELETE /follow/:id → $ST"
else
  bad "expected 200/204, got $ST"
fi

step "6. After unfollow, Alice no longer in Bob's followers"
RES=$(http_get bob "/api/community/profile/$B/followers")
if body_of "$RES" | jq -e --arg u "$A" '(.followers? // .users? // .) | map(.user_id // .id) | index($u)' > /dev/null; then
  bad "Alice still listed after unfollow"
else
  ok "Followers list no longer contains Alice"
fi

step "7. Cannot follow yourself"
RES=$(http_post alice "/api/community/follow/$A" "")
ST=$(status_of "$RES")
[ "$ST" = "400" ] && ok "Self-follow → 400" || bad "expected 400, got $ST"

step "8. Anonymous follow → 401"
ANON=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/community/follow/$B")
[ "$ANON" = "401" ] || [ "$ANON" = "403" ] && ok "Anonymous → $ANON" || bad "expected 401/403, got $ANON"

summary
