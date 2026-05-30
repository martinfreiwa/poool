from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase7_schema_scopes_engagement_to_circles():
    migration = read("database/community/053_circle_engagement_onboarding.sql")

    for token in [
        "ADD COLUMN IF NOT EXISTS announcement_comments_enabled BOOLEAN NOT NULL DEFAULT TRUE",
        "ADD COLUMN IF NOT EXISTS onboarding_enabled BOOLEAN NOT NULL DEFAULT TRUE",
        "ADD COLUMN IF NOT EXISTS circle_id UUID NULL REFERENCES circles(id) ON DELETE CASCADE",
        "ADD COLUMN IF NOT EXISTS asset_id UUID NULL",
        "ADD COLUMN IF NOT EXISTS rsvp_enabled BOOLEAN NOT NULL DEFAULT FALSE",
        "CREATE TABLE IF NOT EXISTS circle_event_rsvps",
        "ADD COLUMN IF NOT EXISTS challenge_scope VARCHAR(32) NOT NULL DEFAULT 'global'",
        "CREATE TABLE IF NOT EXISTS circle_challenge_progress",
        "CREATE TABLE IF NOT EXISTS circle_onboarding_progress",
        "UNIQUE (circle_id, user_id, challenge_id)",
        "UNIQUE (circle_id, user_id)",
        "'circle_due_diligence_question'",
        "'circle_market_insight'",
        "'circle_comment'",
    ]:
        assert token in migration


def test_phase7_backend_keeps_global_and_circle_engagement_separate():
    amas = read("backend/src/community/amas.rs")
    challenges = read("backend/src/community/challenges.rs")
    routes = read("backend/src/community/routes.rs")

    for token in [
        "pub circle_id: Option<Uuid>",
        "pub asset_id: Option<Uuid>",
        "pub rsvp_enabled: bool",
        "pub async fn list_circle_amas",
        "AND circle_id IS NULL",
        "AND circle_id = $1",
    ]:
        assert token in amas

    for token in [
        "pub async fn list_circle_challenges_for_user",
        "pub async fn increment_circle_progress",
        "circle_challenge_progress",
        "challenge_scope = 'circle'",
        "COALESCE(c.challenge_scope, 'global') = 'global'",
        "Progress is keyed by",
    ]:
        assert token in challenges

    for token in [
        '"/api/community/circles/:id/announcements"',
        '"/api/community/circles/:id/events"',
        '"/api/community/circles/:id/challenges"',
        '"/api/community/circles/:id/onboarding"',
        '"/api/community/circles/:id/onboarding/:step"',
        "async fn get_circle_announcements",
        "async fn get_circle_events",
        "async fn get_circle_challenges",
        "async fn get_circle_onboarding",
        "async fn update_circle_onboarding_step",
        "async fn ensure_ama_read_access",
        "WHERE p.circle_id = $1",
        "AND p.post_type IN ('announcement', 'official_update')",
        "notifications_feature_flagged",
    ]:
        assert token in routes


def test_phase7_backend_records_circle_activity_only_for_circle_progress():
    routes = read("backend/src/community/routes.rs")

    for token in [
        "record_circle_post_engagement",
        "first_question_posted",
        "circle_due_diligence_question",
        "circle_market_insight",
        "circle_comment",
        "mark_circle_onboarding_step",
        "ensure_circle_write_access(&state, &c_pool, circle_id, user.id)",
        "SELECT circle_id FROM posts WHERE id = $1",
        "SELECT EXISTS(SELECT 1 FROM ama_questions WHERE id = $1 AND ama_id = $2)",
    ]:
        assert token in routes


def test_phase7_frontend_surfaces_circle_engagement_without_settings_regression():
    page = read("frontend/platform/community-circle.html")
    js = read("frontend/platform/static/js/community-feed.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        "circle-onboarding-panel",
        "circle-announcements-list",
        "circle-events-list",
        "circle-challenges-list",
        "Events / AMAs",
        "Circle progress",
        "Manage",
    ]:
        assert token in page

    for token in [
        "window.loadCircleEngagement",
        "window.markCircleOnboardingStep",
        "/api/community/circles/${id}/announcements",
        "/api/community/circles/${id}/events",
        "/api/community/circles/${id}/challenges",
        "/api/community/circles/${id}/onboarding",
        "textContent = announcement.content",
        "replaceChildren",
        "loadCircleEngagement();",
    ]:
        assert token in js

    for token in [
        ".circle-engagement-card",
        ".circle-engagement-item",
        ".circle-engagement-challenge",
        ".circle-onboarding-step",
        ".circle-onboarding-step__button",
    ]:
        assert token in css

    onboarding_status = css[
        css.index(".circle-onboarding-step__status")
        : css.index(".circle-onboarding-step__label")
    ]
    onboarding_button = css[
        css.index(".circle-onboarding-step__button")
        : css.index(".circle-onboarding-step__button:hover")
    ]

    assert "color: var(--brand-greeny-green, #03FF88);" in onboarding_status
    assert "background: var(--btn-primary-bg, #0000FF);" in onboarding_status
    assert "border-color: var(--btn-primary-bg, #0000FF);" in onboarding_status
    assert "background: var(--btn-primary-bg, #0000FF);" in onboarding_button
    assert "color: var(--brand-greeny-green, #03FF88);" in onboarding_button
