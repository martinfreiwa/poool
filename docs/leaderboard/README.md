# Leaderboard Subsystem — Documentation Index

The investor & developer leaderboard at `/leaderboard` and
`/developer/ranking`. Both pages share the same backend, data model, and
preferences row — only the surrounding shell differs.

## Documents in this folder

| Document | Audience | When to read |
|---|---|---|
| [API Reference](./api-reference.md) | Backend / frontend devs | Building against `/api/leaderboard/*` |
| [Runbook](./runbook.md) | On-call / ops | Refresh failing, stale data, scoring issue |
| [Visibility Semantics](./visibility-semantics.md) | Product / privacy / support | Reasoning about who sees what |
| [Migration Guide — Visibility Filter](./migration-guide.md) | Anyone deploying this branch | One-time deploy procedure for the 2026-05-16 visibility-filter change |

## At a glance

- **Stack**: axum (Rust) + sqlx + Postgres + optional Redis. No third-party leaderboard service.
- **Refresh**: background worker every 15 min, plus admin-triggered POST `/api/leaderboard/refresh`.
- **Privacy**: opt-in. Users only appear publicly when `leaderboard_preferences.visible = true`. The viewer themselves never appears in the public listing — they see their rank in the "Your Standing" card.
- **Performance**: precomputed scores + in-process `last_updated` cache + ETag/304 short-circuit + 30s `Cache-Control: private`.
- **Rate-limit**: 60 req/min per user (Redis-backed when available, else in-memory).
- **Observability**: tracing spans on all handlers + Sentry capture on every failure path + audit-log row for every preference change.

## Key source files

| File | What lives here |
|---|---|
| `backend/src/leaderboard/models.rs` | API response types (`LeaderboardEntry`, `LeaderboardMetrics`, `AssetMixSlice`, `LeaderboardPreferences`) |
| `backend/src/leaderboard/service.rs` | `refresh_all_scores`, `get_rankings`, `update_preferences`, asset-mix enrichment |
| `backend/src/leaderboard/routes.rs` | HTTP handlers, rate-limit/ETag/audit-log wiring |
| `backend/src/leaderboard/mod.rs` | Route registration |
| `database/023_leaderboard.sql` | `leaderboard_scores`, `leaderboard_preferences`, `leaderboard_snapshots` schema |
| `frontend/platform/leaderboard.html` | Investor page shell |
| `frontend/platform/developer/ranking.html` | Developer page shell (same content, dev sidebar) |
| `frontend/platform/static/js/leaderboard.js` | All client-side rendering + interactions |
| `frontend/platform/static/css/leaderboard.css` | All leaderboard visual styling |

## Tests

```sh
DATABASE_URL=postgres://martin@localhost/poool \
  cargo test --test leaderboard_integration \
             --test leaderboard_http \
             --test leaderboard_roi_precision \
             --test leaderboard_production_audit \
  -- --ignored
```

**Current status: 36 tests, all green.** See [Runbook → Test execution](./runbook.md#test-execution) for what each suite covers.

## Last major change

**2026-05-16** — Visibility filter fix. Hidden users (`lp.visible = false`) are now filtered out of `/api/leaderboard` entirely instead of being anonymized in-place. See [Migration Guide](./migration-guide.md) for context and rollout steps.
