from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase6_schema_adds_qa_status_official_answers_and_audit_log():
    migration = read("database/community/052_qa_knowledge_layer.sql")

    for token in [
        "ADD COLUMN IF NOT EXISTS qa_status VARCHAR(32) NOT NULL DEFAULT 'open'",
        "ADD COLUMN IF NOT EXISTS official_answer_comment_id UUID NULL REFERENCES comments(id)",
        "ADD COLUMN IF NOT EXISTS faq_candidate BOOLEAN NOT NULL DEFAULT FALSE",
        "ADD COLUMN IF NOT EXISTS featured_question BOOLEAN NOT NULL DEFAULT FALSE",
        "ADD COLUMN IF NOT EXISTS related_resource_url TEXT",
        "ADD COLUMN IF NOT EXISTS related_asset_id UUID",
        "is_official_answer BOOLEAN NOT NULL DEFAULT FALSE",
        "is_verified_answer BOOLEAN NOT NULL DEFAULT FALSE",
        "answer_marked_by UUID",
        "answer_marked_at TIMESTAMPTZ",
        "CREATE TABLE IF NOT EXISTS community_answer_audit_log",
        "idx_posts_qa_status_created",
        "idx_comments_official_answers",
    ]:
        assert token in migration

    for status in [
        "open",
        "answered",
        "official_answer",
        "needs_clarification",
        "archived",
    ]:
        assert f"'{status}'" in migration


def test_phase6_backend_models_expose_qa_and_answer_fields():
    models = read("backend/src/community/models.rs")

    for field in [
        "pub qa_status: String",
        "pub official_answer_comment_id: Option<Uuid>",
        "pub faq_candidate: bool",
        "pub featured_question: bool",
        "pub related_resource_url: Option<String>",
        "pub related_asset_id: Option<Uuid>",
        "pub is_official_answer: bool",
        "pub is_verified_answer: bool",
        "pub answer_marked_by: Option<Uuid>",
        "pub answer_marked_at: Option<DateTime<Utc>>",
    ]:
        assert field in models


def test_phase6_backend_gates_qa_status_and_official_answers():
    routes = read("backend/src/community/routes.rs")

    for token in [
        "const QA_POST_TYPES",
        "const QA_STATUSES",
        "fn normalize_qa_status",
        "fn is_qa_post_type",
        "fn is_circle_qa_responder_role",
        "async fn user_can_manage_qa_post",
        "verified_expert",
        "async fn update_post_qa_status",
        "async fn mark_official_answer",
        "Only Circle moderators, verified experts, or platform admins can update Q&A status",
        "Only Circle moderators, verified experts, or platform admins can mark official answers",
        "Q&A status can only be set on Question or Due Diligence posts",
        "Official answers can only be set on Question or Due Diligence posts",
        "community_answer_audit_log",
        "qa.status.update",
        "qa.official_answer.mark",
        "is_locked = CASE WHEN $1 = 'archived' THEN TRUE ELSE is_locked END",
    ]:
        assert token in routes

    assert '"/api/community/posts/:id/qa-status"' in routes
    assert '"/api/community/comments/:id/official-answer"' in routes


def test_phase6_comments_and_search_surface_knowledge_signals():
    routes = read("backend/src/community/routes.rs")

    for token in [
        '"is_official_answer": c.is_official_answer',
        '"is_verified_answer": c.is_verified_answer',
        '"answer_marked_by": c.answer_marked_by',
        '"answer_marked_at": c.answer_marked_at',
        '"can_mark_official_answer": can_mark_official_answer',
        "WHEN p.qa_status = 'official_answer' THEN 0",
        "WHEN p.qa_status = 'answered' THEN 1",
        "WHEN p.post_type IN ('question', 'due_diligence', 'resource') THEN 2",
    ]:
        assert token in routes


def test_phase6_frontend_renders_qa_status_and_official_answer_controls():
    circle_page = read("frontend/platform/community-circle.html")
    post_card = read("frontend/platform/partials/community_post_card.html")
    js = read("frontend/platform/static/js/community-feed.js")
    css = read("frontend/platform/static/css/community.css")

    assert "openCircleQaTab(event)" in circle_page

    for token in [
        "feed-post-qa",
        "data-qa-status",
        "Official Answer",
        "Needs Clarification",
        "FAQ Candidate",
        "Featured Question",
        "View official answer",
        "related_resource_url",
    ]:
        assert token in post_card

    for token in [
        "window.openCircleQaTab",
        "window.markOfficialAnswer",
        "/api/community/comments/${encodeURIComponent(commentId)}/official-answer",
        "is_official_answer",
        "is_verified_answer",
        "community-comment-row__answer-badges",
        "community-comment-row__answer-btn",
        "Mark official answer",
    ]:
        assert token in js

    for token in [
        ".feed-post-qa",
        ".feed-post-qa__status--official_answer",
        ".feed-post-qa__status--answered",
        ".community-comment-row__answer-badge--official",
        ".community-comment-row__answer-badge--verified",
        ".community-comment-row__answer-btn",
    ]:
        assert token in css
