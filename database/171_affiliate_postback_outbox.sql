-- 171_affiliate_postback_outbox.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F19 fix: S2S postback outbox with retry + persistence.
--
-- Until now `trigger_s2s_postback` spawned a fire-and-forget Tokio task that
-- did a single HTTP GET. Problems:
--   * graceful shutdown drops in-flight tasks → postback never fires
--   * no retry on transient failure (5xx, network blip)
--   * no audit trail of what was sent / when / response code
--   * affiliate's tracking platform can't tell partial-success from drop
--
-- New shape: enqueue rows into `affiliate_postback_outbox` inside the same
-- transaction that creates the commission/conversion event. A background
-- worker (see Rust patch) drains the queue with exponential backoff. Rows
-- survive restart, retries are bounded, attempts are logged.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

CREATE TABLE IF NOT EXISTS affiliate_postback_outbox (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id    UUID         NOT NULL REFERENCES affiliates(user_id),
    event           VARCHAR(64)  NOT NULL,                          -- 'registered' | 'qualified' | 'first_investment_done' | …
    subid           VARCHAR(255),
    payout_cents    BIGINT       NOT NULL DEFAULT 0,
    url             TEXT         NOT NULL,                          -- the resolved fully-templated URL we'll GET
    status          VARCHAR(20)  NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued', 'sent', 'failed_giveup')),
    attempts        INT          NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_error      TEXT,
    last_response_status INT,                                       -- HTTP status of last attempt
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    sent_at         TIMESTAMPTZ
);

-- Hot-path: worker scan for ready rows.
CREATE INDEX IF NOT EXISTS idx_postback_outbox_ready
    ON affiliate_postback_outbox (next_attempt_at)
 WHERE status = 'queued';

-- Per-affiliate history for support tickets ("what did we send to your URL?").
CREATE INDEX IF NOT EXISTS idx_postback_outbox_affiliate_time
    ON affiliate_postback_outbox (affiliate_id, created_at DESC);

COMMENT ON TABLE affiliate_postback_outbox IS
    'Durable outbox for S2S affiliate postbacks. Replaces the fire-and-forget tokio::spawn pattern: each conversion enqueues a row inside the commission tx, a background worker fires HTTP GETs with exponential backoff and updates status. Survives restarts.';

COMMIT;
