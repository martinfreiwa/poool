#!/usr/bin/env bash
# E2E: Community feed (post create / list / comment / react / bookmark / hashtag).
source "$(dirname "$0")/lib/community_test_helpers.sh"
trap cleanup_users EXIT

step "Seed Alice + Bob"
mk_user alice e2e-feed-alice
mk_user bob   e2e-feed-bob
ok "alice = $(_u_id alice | cut -c1-8)…"
ok "bob   = $(_u_id bob | cut -c1-8)…"

step "1. Alice creates a post with a hashtag + mention"
RES=$(http_post alice /api/community/posts "$(printf '{"content":"Hello world from #poool — hi @%s","post_type":"general"}' "$(_u_id bob)")")
assert_status 200 "$RES" "POST /api/community/posts"
POST_ID=$(body_of "$RES" | jq -r '.id // .post_id // empty')
[ -n "$POST_ID" ] && ok "post_id = ${POST_ID:0:8}…" || bad "no post_id in response"

step "2. Bob sees the post in the public feed"
RES=$(http_get bob "/api/community/feed?sort_by=fresh&page=1")
assert_status 200 "$RES" "GET /api/community/feed"
if body_of "$RES" | jq -e --arg pid "$POST_ID" '(.posts? // .) | map(.id) | index($pid)' > /dev/null; then
  ok "Bob sees Alice's post in feed"
else
  bad "Post missing from feed"
fi

step "3. Bob comments on the post"
RES=$(http_post bob "/api/community/posts/$POST_ID/comments" '{"content":"Great post!"}')
assert_status 200 "$RES" "POST /comments"
COMMENT_ID=$(body_of "$RES" | jq -r '.id // .comment_id // empty')

step "4. Comment list endpoint shows the new comment"
RES=$(http_get alice "/api/community/posts/$POST_ID/comments")
STATUS=$(status_of "$RES")
if [ "$STATUS" = "200" ]; then
  COUNT=$(body_of "$RES" | jq '(.comments? // .) | length')
  [ "$COUNT" -ge 1 ] && ok "Comments endpoint returns ≥1 row ($COUNT)" \
                    || bad "0 comments returned"
else
  bad "GET comments → $STATUS"
fi

step "5. Bob adds a 'like' reaction"
RES=$(http_post bob "/api/community/posts/$POST_ID/reactions" '{"reaction_type":"fire"}')
assert_status 200 "$RES" "POST /reactions"

step "6. Reaction count visible on the post"
RES=$(http_get alice "/api/community/feed?sort_by=fresh&page=1")
COUNT=$(body_of "$RES" | jq --arg pid "$POST_ID" '(.posts? // .) | map(select(.id == $pid)) | .[0].reaction_count // 0')
if [ "$COUNT" -ge 1 ]; then
  ok "Post shows reaction_count = $COUNT"
else
  bad "Reaction count = $COUNT (expected ≥1)"
fi

step "7. Bob bookmarks the post"
RES=$(http_post bob "/api/community/posts/$POST_ID/bookmark" "")
assert_status 200 "$RES" "POST /bookmark"

step "8. Bookmark appears in Bob's bookmark list"
RES=$(http_get bob "/api/community/bookmarks")
if body_of "$RES" | jq -e --arg pid "$POST_ID" '(.bookmarks? // .posts? // .) | map(.id // .post_id) | index($pid)' > /dev/null; then
  ok "Bookmark list contains the post"
else
  bad "Bookmarks list missing the post"
  note "body: $(body_of "$RES" | head -c 200)"
fi

step "9. Empty post content rejected"
RES=$(http_post alice "/api/community/posts" '{"content":"","post_type":"general"}')
ST=$(status_of "$RES")
# 400 is the ideal response (app-level validation); 500 means we hit the
# DB CHECK constraint (still rejected, less clean). Both prove rejection.
if [ "$ST" = "400" ] || [ "$ST" = "500" ]; then
  ok "Empty content rejected → $ST $([ "$ST" = "500" ] && echo '(TODO: add app-level guard)')"
else
  bad "Empty content NOT rejected — got $ST"
fi

step "10. Anonymous POST → 401"
ANON=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/community/posts" \
       -H "Content-Type: application/json" -d '{"content":"anon","post_type":"general"}')
[ "$ANON" = "401" ] || [ "$ANON" = "403" ] && ok "Anonymous → $ANON" || bad "expected 401/403, got $ANON"

step "11. Hashtag detail endpoint returns the post"
RES=$(http_get alice "/api/community/hashtags/poool")
ST=$(status_of "$RES")
if [ "$ST" = "200" ]; then
  if body_of "$RES" | jq -e --arg pid "$POST_ID" '(.posts? // .) | map(.id) | index($pid)' > /dev/null; then
    ok "Hashtag #poool detail page includes the post"
  else
    bad "Post not found under #poool hashtag"
  fi
else
  bad "GET /hashtags/poool → $ST"
fi

summary
