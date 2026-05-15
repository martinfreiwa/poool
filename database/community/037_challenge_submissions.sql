-- 14.8.11 follow-up: vote-based / submission challenges.
--
-- Adds two tables so a challenge with `requirement_type = 'submission'` can
-- collect one entry per user and let other community members upvote them.
-- Progress on submission-type challenges is the number of votes received,
-- not the number of actions taken; admins decide reward thresholds.

CREATE TABLE IF NOT EXISTS challenge_submissions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL,
    content      TEXT NOT NULL,
    vote_count   INTEGER NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- One submission per user per challenge. Re-submitting overwrites.
    UNIQUE (challenge_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_challenge_submissions_challenge
    ON challenge_submissions(challenge_id, vote_count DESC);

CREATE INDEX IF NOT EXISTS idx_challenge_submissions_user
    ON challenge_submissions(user_id);

CREATE TABLE IF NOT EXISTS challenge_submission_votes (
    submission_id UUID NOT NULL REFERENCES challenge_submissions(id) ON DELETE CASCADE,
    voter_id      UUID NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (submission_id, voter_id)
);

CREATE INDEX IF NOT EXISTS idx_challenge_submission_votes_voter
    ON challenge_submission_votes(voter_id);

-- Keep submissions.vote_count in sync via trigger so the leaderboard read is O(1).
CREATE OR REPLACE FUNCTION challenge_submission_vote_count_trigger()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE challenge_submissions
        SET vote_count = vote_count + 1
        WHERE id = NEW.submission_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE challenge_submissions
        SET vote_count = GREATEST(vote_count - 1, 0)
        WHERE id = OLD.submission_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS challenge_submission_vote_count
    ON challenge_submission_votes;

CREATE TRIGGER challenge_submission_vote_count
    AFTER INSERT OR DELETE ON challenge_submission_votes
    FOR EACH ROW EXECUTE FUNCTION challenge_submission_vote_count_trigger();
