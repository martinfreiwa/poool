#!/usr/bin/env bash
# End-to-end DM test between two community members.
#
# Creates two fresh test users (Alice + Bob), mints sessions, then:
#   1. Alice → Bob: opens a thread + posts "Hey Bob 👋"
#   2. Bob → Alice: replies in the same thread
#   3. Verifies both messages are visible to both users
#   4. Verifies blocked-user case returns 403 (after Bob blocks Alice)
#   5. Cleans up users, threads, messages
#
# Usage:
#   ./scripts/test_dm_e2e.sh
#
# Requirements: backend running on :8888, psql + curl + jq + python3.

set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8888}"
DB_CORE="${DATABASE_URL:-postgres://martin@localhost/poool}"
DB_COMM="${COMMUNITY_DATABASE_URL:-postgres://martin@localhost/poool_community}"
PASS=0
FAIL=0

step() { printf "\n\033[1;34m── %s\033[0m\n" "$*"; }
ok()   { printf "  \033[0;32m✓\033[0m %s\n" "$*"; PASS=$((PASS+1)); }
bad()  { printf "  \033[0;31m✗\033[0m %s\n" "$*"; FAIL=$((FAIL+1)); }
note() { printf "    \033[0;90m%s\033[0m\n" "$*"; }

# ── Setup: 2 fresh users + community profiles + sessions ────────────
ALICE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
BOB_ID=$(uuidgen   | tr '[:upper:]' '[:lower:]')
ALICE_TOKEN=$(python3 -c "import secrets; print(secrets.token_urlsafe(48))")
BOB_TOKEN=$(python3 -c "import secrets; print(secrets.token_urlsafe(48))")
ALICE_EMAIL="dm-test-alice-$(date +%s)@poool.test"
BOB_EMAIL="dm-test-bob-$(date +%s)@poool.test"

cleanup() {
  psql "$DB_COMM" -q -c "
    DELETE FROM dm_messages WHERE sender_id IN ('$ALICE_ID','$BOB_ID');
    DELETE FROM dm_threads  WHERE participant_a_id IN ('$ALICE_ID','$BOB_ID')
                               OR participant_b_id IN ('$ALICE_ID','$BOB_ID');
    DELETE FROM block_relationships WHERE blocker_id IN ('$ALICE_ID','$BOB_ID')
                                       OR blocked_id IN ('$ALICE_ID','$BOB_ID');
    DELETE FROM community_profiles WHERE user_id IN ('$ALICE_ID','$BOB_ID');
  " > /dev/null 2>&1 || true
  psql "$DB_CORE" -q -c "
    DELETE FROM user_sessions WHERE user_id IN ('$ALICE_ID','$BOB_ID');
    DELETE FROM users         WHERE id      IN ('$ALICE_ID','$BOB_ID');
  " > /dev/null 2>&1 || true
}
trap cleanup EXIT

step "Seed: 2 fresh users + sessions + community profiles"

psql "$DB_CORE" -q -c "
  INSERT INTO users (id, email, password_hash, status, email_verified) VALUES
    ('$ALICE_ID', '$ALICE_EMAIL', 'x', 'active', TRUE),
    ('$BOB_ID',   '$BOB_EMAIL',   'x', 'active', TRUE);
  INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified) VALUES
    ('$ALICE_ID', '$ALICE_TOKEN', NOW() + INTERVAL '1 hour', TRUE),
    ('$BOB_ID',   '$BOB_TOKEN',   NOW() + INTERVAL '1 hour', TRUE);
" > /dev/null

psql "$DB_COMM" -q -c "
  INSERT INTO community_profiles (user_id) VALUES ('$ALICE_ID'), ('$BOB_ID')
  ON CONFLICT (user_id) DO NOTHING;
" > /dev/null

ok "Alice = ${ALICE_ID:0:8}…"
ok "Bob   = ${BOB_ID:0:8}…"

# CSRF tokens: the platform sets a per-session `csrf_token` cookie on the
# first authenticated GET. Mint a token for each user by hitting /api/me.
mint_csrf() {
  local tok="$1"
  local jar; jar=$(mktemp)
  curl -s -c "$jar" -b "poool_session=$tok" "$BASE_URL/api/me" > /dev/null
  awk '$6 == "csrf_token" {print $7}' "$jar"
  rm -f "$jar"
}
ALICE_CSRF=$(mint_csrf "$ALICE_TOKEN")
BOB_CSRF=$(mint_csrf "$BOB_TOKEN")
[ -n "$ALICE_CSRF" ] && ok "Alice CSRF token minted (${ALICE_CSRF:0:8}…)" || bad "Alice CSRF empty"
[ -n "$BOB_CSRF" ]   && ok "Bob CSRF token minted (${BOB_CSRF:0:8}…)"   || bad "Bob CSRF empty"

# ── Test 1: Alice opens DM with Bob ─────────────────────────────────
step "1. Alice opens DM thread with Bob"

CREATE_RES=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/community/dms/threads" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d "$(printf '{"recipient_user_id":"%s","content":"Hey Bob 👋 first DM"}' "$BOB_ID")")
STATUS=$(echo "$CREATE_RES" | tail -1)
BODY=$(echo "$CREATE_RES" | sed '$d')

if [ "$STATUS" = "200" ]; then
  ok "POST /api/community/dms/threads → 200"
  THREAD_ID=$(echo "$BODY" | jq -r '.thread_id')
  FIRST_MSG_ID=$(echo "$BODY" | jq -r '.message_id')
  note "thread_id = ${THREAD_ID:0:8}…"
  note "msg_id    = ${FIRST_MSG_ID:0:8}…"
else
  bad "Expected 200, got $STATUS"
  note "body: $BODY"
  exit 1
fi

# ── Test 2: Bob replies in same thread ──────────────────────────────
step "2. Bob replies in same thread"

REPLY_RES=$(curl -s -w "\n%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$BOB_TOKEN; csrf_token=$BOB_CSRF" \
  -H "X-CSRF-Token: $BOB_CSRF" \
  -d '{"content":"Hi Alice — got your message!"}')
STATUS=$(echo "$REPLY_RES" | tail -1)
BODY=$(echo "$REPLY_RES" | sed '$d')

if [ "$STATUS" = "200" ]; then
  ok "POST /api/community/dms/threads/:id/messages → 200"
  REPLY_MSG_ID=$(echo "$BODY" | jq -r '.message_id // .id')
  note "reply msg_id = ${REPLY_MSG_ID:0:8}…"
else
  bad "Expected 200, got $STATUS"
  note "body: $BODY"
fi

# ── Test 3: Both users see both messages ────────────────────────────
step "3. Both users see both messages in the thread"

for who in alice bob; do
  if [ "$who" = "alice" ]; then TOK="$ALICE_TOKEN"; else TOK="$BOB_TOKEN"; fi
  LIST_RES=$(curl -s -w "\n%{http_code}" \
    "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
    -H "Cookie: poool_session=$TOK")
  STATUS=$(echo "$LIST_RES" | tail -1)
  BODY=$(echo "$LIST_RES" | sed '$d')
  if [ "$STATUS" = "200" ]; then
    COUNT=$(echo "$BODY" | jq '.messages | length // (. | length)')
    if [ "$COUNT" -ge 2 ]; then
      ok "$who sees $COUNT messages"
    else
      bad "$who sees only $COUNT messages (expected ≥2)"
      note "body: $BODY"
    fi
  else
    bad "GET messages as $who → $STATUS"
    note "body: $BODY"
  fi
done

# ── Test 4: Thread list shows the new thread for both ──────────────
step "4. Thread list endpoint shows the conversation for both users"

for who in alice bob; do
  if [ "$who" = "alice" ]; then TOK="$ALICE_TOKEN"; else TOK="$BOB_TOKEN"; fi
  THREADS=$(curl -s "$BASE_URL/api/community/dms/threads" \
    -H "Cookie: poool_session=$TOK")
  if echo "$THREADS" | jq -e --arg t "$THREAD_ID" '
        (.threads // .) | map(select(.id == $t or .thread_id == $t)) | length > 0
      ' > /dev/null; then
    ok "$who's thread list contains the new thread"
  else
    bad "$who's thread list missing the thread"
    note "first 200 chars: ${THREADS:0:200}"
  fi
done

# ── Test 5: Validation — empty message rejected ─────────────────────
step "5. Empty message rejected (400)"

EMPTY_RES=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d '{"content":"   "}')
if [ "$EMPTY_RES" = "400" ]; then
  ok "Empty content rejected with 400"
else
  bad "Expected 400 for empty content, got $EMPTY_RES"
fi

# ── Test 6: Self-DM rejected ────────────────────────────────────────
step "6. Cannot DM yourself (400)"

SELF_RES=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d "$(printf '{"recipient_user_id":"%s","content":"talking to myself"}' "$ALICE_ID")")
if [ "$SELF_RES" = "400" ]; then
  ok "Self-DM rejected with 400"
else
  bad "Expected 400 for self-DM, got $SELF_RES"
fi

# ── Test 7: Anonymous request rejected (401) ────────────────────────
step "7. Anonymous request rejected (401)"

ANON_RES=$(curl -s -o /dev/null -w "%{http_code}" \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages")
if [ "$ANON_RES" = "401" ]; then
  ok "Anonymous GET rejected with 401"
else
  bad "Expected 401, got $ANON_RES"
fi

# ── Test 8: Idempotent thread create ───────────────────────────────
step "8. Opening the same DM twice reuses the existing thread"

DUP_RES=$(curl -s -X POST "$BASE_URL/api/community/dms/threads" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d "$(printf '{"recipient_user_id":"%s","content":"second open"}' "$BOB_ID")")
DUP_THREAD=$(echo "$DUP_RES" | jq -r '.thread_id // empty')
if [ "$DUP_THREAD" = "$THREAD_ID" ]; then
  ok "Second open returned same thread_id (idempotent via UNIQUE pair)"
else
  bad "Got different thread_id ($DUP_THREAD vs $THREAD_ID) — duplicate thread row!"
fi

# Verify DB really only has 1 row for the pair (sorted-pair).
PAIR_COUNT=$(psql "$DB_COMM" -tA -c "
  SELECT COUNT(*) FROM dm_threads
  WHERE (participant_a_id, participant_b_id) IN
        (('$ALICE_ID','$BOB_ID'), ('$BOB_ID','$ALICE_ID'))")
if [ "$PAIR_COUNT" = "1" ]; then
  ok "Exactly 1 thread row in DB for Alice↔Bob pair"
else
  bad "Found $PAIR_COUNT thread rows for the pair (expected 1)"
fi

# ── Test 9: Message ordering — chronological asc ───────────────────
step "9. Message ordering is chronological (oldest first)"

# Send 3 more messages with explicit waits so created_at differs reliably.
for i in 1 2 3; do
  curl -s -o /dev/null -X POST \
    "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
    -H "Content-Type: application/json" \
    -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
    -H "X-CSRF-Token: $ALICE_CSRF" \
    -d "$(printf '{"content":"ordered-%d"}' "$i")"
  sleep 0.05
done
MSG_CONTENT=$(curl -s "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Cookie: poool_session=$ALICE_TOKEN" \
  | jq -r '.messages // . | map(.content) | join("|")')

if [[ "$MSG_CONTENT" == *"Hey Bob"*"Hi Alice"*"ordered-1"*"ordered-2"*"ordered-3"* ]]; then
  ok "Messages returned in chronological order"
else
  bad "Order mismatch"
  note "got: $MSG_CONTENT"
fi

# ── Test 10: Read receipts — recipient GET marks messages read ─────
step "10. Recipient GET updates read_at_recipient timestamps"

UNREAD_BEFORE=$(psql "$DB_COMM" -tA -c "
  SELECT COUNT(*) FROM dm_messages
  WHERE thread_id = '$THREAD_ID' AND sender_id = '$ALICE_ID'
    AND read_at_recipient IS NULL")
note "Before Bob reads: $UNREAD_BEFORE unread messages from Alice"

# Bob lists the thread → handler updates read_at_recipient for non-self msgs.
curl -s -o /dev/null "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Cookie: poool_session=$BOB_TOKEN"

UNREAD_AFTER=$(psql "$DB_COMM" -tA -c "
  SELECT COUNT(*) FROM dm_messages
  WHERE thread_id = '$THREAD_ID' AND sender_id = '$ALICE_ID'
    AND read_at_recipient IS NULL")
if [ "$UNREAD_AFTER" = "0" ]; then
  ok "Bob reading thread cleared all unread flags on Alice's messages"
else
  bad "Expected 0 unread after Bob's GET, still $UNREAD_AFTER"
fi

# ── Test 11: 4000-char content limit ───────────────────────────────
step "11. Message >4000 chars rejected (400)"

LONG_CONTENT=$(printf 'a%.0s' {1..4001})
LONG_RES=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d "$(printf '{"content":"%s"}' "$LONG_CONTENT")")
if [ "$LONG_RES" = "400" ]; then
  ok "4001-char message rejected with 400"
else
  bad "Expected 400 for oversized message, got $LONG_RES"
fi

# ── Test 12: Cross-thread isolation — third user can't read ────────
step "12. Third user (Charlie) cannot read Alice↔Bob messages"

CHARLIE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
CHARLIE_TOKEN=$(python3 -c "import secrets; print(secrets.token_urlsafe(48))")
CHARLIE_EMAIL="dm-test-charlie-$(date +%s)@poool.test"
psql "$DB_CORE" -q -c "
  INSERT INTO users (id, email, password_hash, status, email_verified) VALUES
    ('$CHARLIE_ID', '$CHARLIE_EMAIL', 'x', 'active', TRUE);
  INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified) VALUES
    ('$CHARLIE_ID', '$CHARLIE_TOKEN', NOW() + INTERVAL '1 hour', TRUE);" > /dev/null
psql "$DB_COMM" -q -c "
  INSERT INTO community_profiles (user_id) VALUES ('$CHARLIE_ID')
  ON CONFLICT (user_id) DO NOTHING;" > /dev/null

CHARLIE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Cookie: poool_session=$CHARLIE_TOKEN")
if [ "$CHARLIE_STATUS" = "403" ] || [ "$CHARLIE_STATUS" = "404" ]; then
  ok "Non-participant Charlie blocked with $CHARLIE_STATUS"
else
  bad "Expected 403/404 for non-participant, got $CHARLIE_STATUS — privacy leak!"
fi

# ── Test 13: Block prevents new thread creation ────────────────────
step "13. After Bob blocks Alice, Alice cannot start a new thread"

CHARLIE_CSRF=$(mint_csrf "$CHARLIE_TOKEN")

# Bob blocks Alice
curl -s -o /dev/null -X POST "$BASE_URL/api/community/users/$ALICE_ID/block" \
  -H "Cookie: poool_session=$BOB_TOKEN; csrf_token=$BOB_CSRF" \
  -H "X-CSRF-Token: $BOB_CSRF"

# Alice tries to start a fresh thread → must be 403
BLOCK_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d "$(printf '{"recipient_user_id":"%s","content":"can you hear me?"}' "$BOB_ID")")
if [ "$BLOCK_STATUS" = "403" ]; then
  ok "Blocked user (Alice) gets 403 on new thread create"
else
  bad "Expected 403 after block, got $BLOCK_STATUS"
fi

# ── Test 14: Block prevents posting in existing thread ─────────────
step "14. Block also stops posting in pre-existing thread"

BLOCK_POST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/community/dms/threads/$THREAD_ID/messages" \
  -H "Content-Type: application/json" \
  -H "Cookie: poool_session=$ALICE_TOKEN; csrf_token=$ALICE_CSRF" \
  -H "X-CSRF-Token: $ALICE_CSRF" \
  -d '{"content":"sneaking through old thread"}')
if [ "$BLOCK_POST_STATUS" = "403" ]; then
  ok "Block stops Alice posting in pre-existing thread (403)"
else
  bad "Expected 403, got $BLOCK_POST_STATUS — block bypass via existing thread!"
fi

# Cleanup Charlie + block before EXIT trap fires.
psql "$DB_COMM" -q -c "
  DELETE FROM block_relationships
  WHERE actor_user_id IN ('$ALICE_ID','$BOB_ID','$CHARLIE_ID')
     OR target_user_id IN ('$ALICE_ID','$BOB_ID','$CHARLIE_ID');
  DELETE FROM community_profiles WHERE user_id = '$CHARLIE_ID';" > /dev/null 2>&1 || true
psql "$DB_CORE" -q -c "
  DELETE FROM user_sessions WHERE user_id = '$CHARLIE_ID';
  DELETE FROM users WHERE id = '$CHARLIE_ID';" > /dev/null 2>&1 || true

# ── Summary ─────────────────────────────────────────────────────────
echo
printf "\033[1m─── Summary ───\033[0m\n"
printf "  \033[0;32mpassed:\033[0m %d\n" "$PASS"
printf "  \033[0;31mfailed:\033[0m %d\n" "$FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
