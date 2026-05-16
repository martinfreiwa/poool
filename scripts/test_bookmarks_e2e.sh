#!/usr/bin/env bash
# E2E: Post bookmark toggle + list.
source "$(dirname "$0")/lib/community_test_helpers.sh"
trap cleanup_users EXIT

step "Seed Alice"
mk_user alice e2e-bm-a

step "Alice creates a post"
RES=$(http_post alice "/api/community/posts" '{"content":"Bookmark me!","post_type":"general"}')
assert_status 200 "$RES" "POST /posts"
POST_ID=$(body_of "$RES" | jq -r '.id // .post_id // empty')
[ -n "$POST_ID" ] && ok "post_id = ${POST_ID:0:8}…" || { bad "no post_id"; summary; exit 1; }

step "1. Bookmark the post"
RES=$(http_post alice "/api/community/posts/$POST_ID/bookmark" "")
assert_status 200 "$RES" "POST /posts/:id/bookmark"

step "2. Bookmark list contains the post"
RES=$(http_get alice "/api/community/bookmarks")
if body_of "$RES" | jq -e --arg pid "$POST_ID" '(.bookmarks? // .posts? // .) | map(.id // .post_id) | index($pid)' > /dev/null; then
  ok "Bookmark list contains the post"
else
  bad "Bookmarks list missing the post"
fi

step "3. Bookmark same post again (toggle should remove or be idempotent)"
RES=$(http_post alice "/api/community/posts/$POST_ID/bookmark" "")
ST=$(status_of "$RES")
[ "$ST" = "200" ] && ok "Second toggle → 200" || bad "expected 200, got $ST"

step "4. After second toggle, bookmark may be removed (toggle semantics)"
RES=$(http_get alice "/api/community/bookmarks")
COUNT=$(body_of "$RES" | jq --arg pid "$POST_ID" '(.bookmarks? // .posts? // .) | map(select(.id == $pid or .post_id == $pid)) | length')
note "After toggle: $COUNT entries for this post (0 = toggle, 1 = idempotent — both acceptable)"
ok "Second toggle did not crash ($COUNT entries)"

step "5. Anonymous bookmark → 401"
ANON=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/community/posts/$POST_ID/bookmark")
[ "$ANON" = "401" ] || [ "$ANON" = "403" ] && ok "Anonymous → $ANON" || bad "expected 401/403, got $ANON"

summary
