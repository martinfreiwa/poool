#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-compose.local-live.yml}"
BASE_HTTP="${BASE_HTTP:-http://localhost:8080}"

curl -fsS "$BASE_HTTP/live" >/dev/null
curl -fsS "$BASE_HTTP/ready" >/dev/null
curl -fsS "$BASE_HTTP/health" >/dev/null
curl -fsS "$BASE_HTTP/metrics" >/dev/null

docker compose -f "$COMPOSE_FILE" exec -T redis \
  redis-cli -s /var/run/redis/redis.sock PING | grep -q PONG

docker compose -f "$COMPOSE_FILE" exec -T backend \
  sh -lc "grep -q '^pool_mode = session$' /tmp/pgbouncer/pgbouncer.ini"

docker compose -f "$COMPOSE_FILE" exec -T postgres \
  psql -U poool -d poool -Atc "select count(*) from _schema_migrations" >/dev/null

docker compose -f "$COMPOSE_FILE" exec -T postgres \
  psql -U poool -d poool_community -Atc "select count(*) from _schema_migrations" >/dev/null

printf 'local-live smoke passed.\n'
