-- 14.8.20 — Direct messages.
--
-- Two-party DM threads. participant_a_id and participant_b_id are stored
-- in lexicographic order via the CHECK so (alice, bob) and (bob, alice)
-- normalise to the same row and the UNIQUE constraint deduplicates.
--
-- Block / mute enforcement happens at the application layer when creating
-- threads or posting messages — these tables don't reference
-- block_relationships directly.

CREATE TABLE IF NOT EXISTS dm_threads (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    participant_a_id  UUID NOT NULL,
    participant_b_id  UUID NOT NULL,
    last_message_at   TIMESTAMPTZ,
    deleted_at_a      TIMESTAMPTZ,
    deleted_at_b      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT dm_threads_ordered_participants CHECK (participant_a_id < participant_b_id),
    CONSTRAINT dm_threads_unique_pair UNIQUE (participant_a_id, participant_b_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_threads_a ON dm_threads (participant_a_id, last_message_at DESC);
CREATE INDEX IF NOT EXISTS idx_dm_threads_b ON dm_threads (participant_b_id, last_message_at DESC);

CREATE TABLE IF NOT EXISTS dm_messages (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id           UUID NOT NULL REFERENCES dm_threads(id) ON DELETE CASCADE,
    sender_id           UUID NOT NULL,
    content             TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 4000),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at_recipient   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_dm_messages_thread ON dm_messages (thread_id, created_at);
