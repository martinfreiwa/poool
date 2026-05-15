-- 160: Partition referral_clicks BY RANGE(created_at)
--
-- Bei erwartetem Click-Volume (100M+ Rows in wenigen Jahren) ist eine flache
-- Tabelle nicht haltbar. Monatliche Partitionierung erlaubt:
--   * Query-Pruning auf Hot-Partitions (letzte 30 Tage)
--   * günstigen DROP alter Partitionen (Retention) statt teurem DELETE
--   * BRIN-Index nur auf Parent → automatisch propagiert
--
-- Strategie (User-Wahl: Variante B = keine Daten verlieren):
--   1. Rename current referral_clicks → referral_clicks_legacy
--   2. Create new referral_clicks PARTITION BY RANGE(created_at) mit
--      identischer Spalten-Definition
--   3. Erzeuge Partitionen für jeden Monat in dem Legacy-Daten liegen +
--      die kommenden 12 Monate
--   4. COPY data legacy → new (Routing erfolgt automatisch)
--   5. Drop legacy
--   6. Helper-Funktion für rolling partition maintenance

BEGIN;

-- 1) Move bestehende Tabelle zur Seite
ALTER TABLE referral_clicks RENAME TO referral_clicks_legacy;
ALTER INDEX referral_clicks_pkey         RENAME TO referral_clicks_legacy_pkey;
ALTER INDEX idx_referral_clicks_code     RENAME TO idx_referral_clicks_legacy_code;
ALTER INDEX idx_referral_clicks_subid    RENAME TO idx_referral_clicks_legacy_subid;
ALTER INDEX idx_referral_clicks_link     RENAME TO idx_referral_clicks_legacy_link;

-- 2) Neue Tabelle als Partition-Parent
-- PK MUSS die Partition-Key-Spalte (created_at) enthalten.
CREATE TABLE referral_clicks (
    id          UUID        NOT NULL DEFAULT gen_random_uuid(),
    code        VARCHAR(32) NOT NULL,
    link_id     UUID        REFERENCES affiliate_links(id),
    ip_address  INET,
    user_agent  TEXT,
    subid       VARCHAR(255),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE INDEX referral_clicks_code_idx ON referral_clicks (code);
CREATE INDEX referral_clicks_subid_idx ON referral_clicks (subid) WHERE subid IS NOT NULL;
CREATE INDEX referral_clicks_link_idx ON referral_clicks (link_id, created_at DESC)
    WHERE link_id IS NOT NULL;
CREATE INDEX referral_clicks_created_brin ON referral_clicks
    USING BRIN (created_at) WITH (pages_per_range = 32);

-- 3) Partitionen für jeden Monat in dem Legacy-Daten existieren erzeugen,
-- plus current + nächste 12 Monate (rolling window manual seed).
--
-- Wir nutzen ein anonymes DO-Block, damit auch leere Legacy-Tabelle korrekt
-- behandelt wird.
DO $$
DECLARE
    partition_month DATE;
    last_seed_month DATE;
BEGIN
    -- Älteste Click-Date oder heute-relativ rückwärts 1 Monat als Start.
    SELECT COALESCE(date_trunc('month', MIN(created_at)), date_trunc('month', NOW()) - INTERVAL '1 month')
    INTO partition_month
    FROM referral_clicks_legacy;

    -- Schleife bis heute + 12 Monate
    last_seed_month := date_trunc('month', NOW() + INTERVAL '12 months');

    WHILE partition_month <= last_seed_month LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS referral_clicks_%s
                PARTITION OF referral_clicks
                FOR VALUES FROM (%L) TO (%L)',
            to_char(partition_month, 'YYYY_MM'),
            partition_month,
            partition_month + INTERVAL '1 month'
        );
        partition_month := partition_month + INTERVAL '1 month';
    END LOOP;
END $$;

-- 4) COPY data über Parent (Postgres dispatcht auf Partition)
INSERT INTO referral_clicks (id, code, link_id, ip_address, user_agent, subid, created_at)
SELECT id, code, link_id, ip_address, user_agent, subid, created_at
FROM referral_clicks_legacy;

-- 5) Drop legacy
DROP TABLE referral_clicks_legacy;

-- 6) Helper für rolling-window-maintenance.
-- Erzeugt fehlende Partitionen für [today, today + months_ahead).
-- Aufrufer (Worker oder Cron) ruft das z.B. täglich auf.
CREATE OR REPLACE FUNCTION referral_clicks_ensure_future_partitions(months_ahead INT DEFAULT 3)
RETURNS INT
LANGUAGE plpgsql AS $$
DECLARE
    partition_month DATE;
    end_month DATE;
    created_count INT := 0;
BEGIN
    partition_month := date_trunc('month', NOW());
    end_month       := date_trunc('month', NOW() + (months_ahead || ' months')::INTERVAL);

    WHILE partition_month <= end_month LOOP
        BEGIN
            EXECUTE format(
                'CREATE TABLE referral_clicks_%s
                    PARTITION OF referral_clicks
                    FOR VALUES FROM (%L) TO (%L)',
                to_char(partition_month, 'YYYY_MM'),
                partition_month,
                partition_month + INTERVAL '1 month'
            );
            created_count := created_count + 1;
        EXCEPTION WHEN duplicate_table THEN
            -- Partition existiert bereits, weiter.
            NULL;
        END;
        partition_month := partition_month + INTERVAL '1 month';
    END LOOP;

    RETURN created_count;
END $$;

-- Helper für Retention: dropt Partitionen älter als N Monate. Aufrufer ist
-- ein Maintenance-Worker, Default ist konservativ (13 Monate).
CREATE OR REPLACE FUNCTION referral_clicks_drop_old_partitions(retain_months INT DEFAULT 13)
RETURNS INT
LANGUAGE plpgsql AS $$
DECLARE
    cutoff_month DATE;
    rec RECORD;
    dropped INT := 0;
BEGIN
    cutoff_month := date_trunc('month', NOW() - (retain_months || ' months')::INTERVAL);

    FOR rec IN
        SELECT child.relname AS partition_name
        FROM pg_inherits
        JOIN pg_class parent ON parent.oid = pg_inherits.inhparent
        JOIN pg_class child  ON child.oid  = pg_inherits.inhrelid
        WHERE parent.relname = 'referral_clicks'
          AND child.relname ~ '^referral_clicks_\d{4}_\d{2}$'
    LOOP
        IF to_date(substring(rec.partition_name FROM 'referral_clicks_(\d{4}_\d{2})$'),
                   'YYYY_MM') < cutoff_month THEN
            EXECUTE format('DROP TABLE IF EXISTS %I', rec.partition_name);
            dropped := dropped + 1;
        END IF;
    END LOOP;

    RETURN dropped;
END $$;

-- RLS: keine pro-User-Policy nötig (Click-Logs sind Admin-Sicht), aber
-- aktiviere RLS damit non-owner Roles (Future-Anon-Role, Read-Replicas)
-- nichts sehen. Owner-Bypass solange keine FORCE.
ALTER TABLE referral_clicks ENABLE ROW LEVEL SECURITY;

COMMIT;
