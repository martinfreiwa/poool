#!/usr/bin/env bash
# E2E: Block + mute relationships.
source "$(dirname "$0")/lib/community_test_helpers.sh"
trap cleanup_users EXIT

step "Seed Alice + Bob"
mk_user alice e2e-block-a
mk_user bob   e2e-block-b
A=$(_u_id alice); B=$(_u_id bob)

step "1. Alice blocks Bob"
RES=$(http_post alice "/api/community/users/$B/block" "")
assert_status 200 "$RES" "POST /users/:id/block"

step "2. Alice's block list contains Bob"
RES=$(http_get alice "/api/community/blocks")
if body_of "$RES" | jq -e --arg u "$B" '(.blocks? // .blocked? // .) | map(.target_user_id // .user_id // .id) | index($u)' > /dev/null; then
  ok "Block list contains Bob"
else
  bad "Bob missing from block list"
  note "body: $(body_of "$RES" | head -c 200)"
fi

step "3. Cannot block yourself (400)"
RES=$(http_post alice "/api/community/users/$A/block" "")
ST=$(status_of "$RES")
[ "$ST" = "400" ] && ok "Self-block → 400" || bad "expected 400, got $ST"

step "4. Alice unblocks Bob"
RES=$(http_del alice "/api/community/users/$B/block")
ST=$(status_of "$RES")
if [ "$ST" = "200" ] || [ "$ST" = "204" ]; then
  ok "DELETE /users/:id/block → $ST"
else
  bad "expected 200/204, got $ST"
fi

step "5. After unblock, Bob not in block list"
RES=$(http_get alice "/api/community/blocks")
if body_of "$RES" | jq -e --arg u "$B" '(.blocks? // .blocked? // .) | map(.target_user_id // .user_id // .id) | index($u)' > /dev/null; then
  bad "Bob still in block list after unblock"
else
  ok "Block list no longer contains Bob"
fi

step "6. Alice mutes Bob"
RES=$(http_post alice "/api/community/users/$B/mute" "")
assert_status 200 "$RES" "POST /users/:id/mute"

step "7. Mute list contains Bob"
RES=$(http_get alice "/api/community/mutes")
if body_of "$RES" | jq -e --arg u "$B" '(.mutes? // .muted? // .) | map(.target_user_id // .user_id // .id) | index($u)' > /dev/null; then
  ok "Mute list contains Bob"
else
  bad "Bob missing from mute list"
fi

step "8. Alice unmutes Bob"
RES=$(http_del alice "/api/community/users/$B/mute")
ST=$(status_of "$RES")
if [ "$ST" = "200" ] || [ "$ST" = "204" ]; then
  ok "DELETE /users/:id/mute → $ST"
else
  bad "expected 200/204, got $ST"
fi

step "9. Anonymous block → 401"
ANON=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/community/users/$B/block")
[ "$ANON" = "401" ] || [ "$ANON" = "403" ] && ok "Anonymous → $ANON" || bad "expected 401/403, got $ANON"

summary
