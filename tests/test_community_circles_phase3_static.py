from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase3_schema_adds_circle_types_roles_and_reputation_flairs():
    migration = read("database/community/050_circle_types_reputation.sql")

    for column in [
        "circle_type VARCHAR(32) NOT NULL DEFAULT 'social'",
        "visibility VARCHAR(32) NOT NULL DEFAULT 'public'",
        "join_policy VARCHAR(32) NOT NULL DEFAULT 'open'",
        "is_official BOOLEAN NOT NULL DEFAULT FALSE",
        "kyc_required BOOLEAN NOT NULL DEFAULT FALSE",
        "private_investor_club BOOLEAN NOT NULL DEFAULT FALSE",
        "allow_cross_post BOOLEAN NOT NULL DEFAULT TRUE",
    ]:
        assert column in migration

    for value in [
        "'social'",
        "'asset'",
        "'topic'",
        "'expert'",
        "'private_investor'",
        "'official'",
        "'public'",
        "'private'",
        "'hidden'",
        "'holder_only'",
        "'kyc_required'",
        "'verified_expert'",
    ]:
        assert value in migration

    assert "CREATE TABLE IF NOT EXISTS community_reputation_flair_grants" in migration
    for flair in [
        "verified_investor",
        "asset_holder",
        "helpful_contributor",
        "founder_member",
        "long_term_member",
        "ama_speaker",
        "official_poool",
        "real_estate_analyst",
        "commodity_expert",
    ]:
        assert flair in migration

    assert "never investment advice or performance claims" in migration
    assert "do not store raw KYC documents or sensitive PII" in migration


def test_phase3_backend_models_and_discovery_expose_access_metadata():
    circles = read("backend/src/community/circles.rs")
    lib = read("backend/src/lib.rs")
    admin_routes = read("backend/src/community/routes.rs")

    for field in [
        "pub circle_type: String",
        "pub visibility: String",
        "pub join_policy: String",
        "pub is_official: bool",
        "pub kyc_required: bool",
        "pub private_investor_club: bool",
        "pub allow_cross_post: bool",
    ]:
        assert field in circles

    assert 'visibility <> \'hidden\'' in circles
    assert "WHERE (is_official = TRUE OR circle_type = 'official')" in circles
    assert "WHERE (kyc_required = TRUE OR join_policy = 'kyc_required')" in circles
    assert 'visibility = \'public\'' in circles
    assert "admin_get_circles(" in circles
    assert "visibility: Option<&str>" in circles
    assert 'Some("hidden") => Some("hidden")' in admin_routes

    assert "let is_gated = circle.token_gate_asset_id.is_some()" in lib
    assert '\"holder_only\" | \"kyc_required\"' in lib
    assert "circle.is_public && circle.visibility == \"public\" && !is_gated" in lib


def test_phase3_server_side_access_rules_cover_hidden_invites_and_gates():
    circles = read("backend/src/community/circles.rs")
    routes = read("backend/src/community/routes.rs")

    assert "pub async fn has_pending_invite" in circles
    assert "visibility == \"hidden\" && !has_invite" in circles
    assert "visibility == \"hidden\" && role.is_none() && !invited" in circles
    assert "join_policy == \"invite_only\" && !has_invite" in circles
    assert "join_policy == \"request\" && !has_invite" in circles
    assert "member_count >= max_members" not in circles
    assert "This circle is full." not in circles
    assert "Cannot approve: circle is now full." not in circles
    assert "visibility == \"hidden\"" in routes
    assert "has_pending_invite(pool, circle_id, user_id)" in routes

    assert "pub async fn check_token_gate" in circles
    assert "let min_cents = gate_min_cents.unwrap_or(0).max(1)" in circles
    assert "tokens_owned::BIGINT * a.token_price_cents" in circles
    assert "format_cents" in circles
    assert "pub async fn check_kyc_gate" in circles
    assert "FROM kyc_records" in circles
    assert "check_token_gate(&c_pool, &state.db, user.id, circle_id)" in routes
    assert "check_kyc_gate(&c_pool, &state.db, user.id, circle_id)" in routes


def test_phase3_reputation_flairs_are_admin_system_granted_not_user_spoofed():
    models = read("backend/src/community/models.rs")
    service = read("backend/src/community/service.rs")
    routes = read("backend/src/community/routes.rs")
    post_card = read("frontend/platform/partials/community_post_card.html")

    assert "pub struct ReputationFlairDisplay" in models
    assert "pub author_reputation_flairs: Vec<ReputationFlairDisplay>" in models
    assert "pub async fn get_reputation_flairs_batch" in service
    assert "pub async fn grant_reputation_flair" in service
    assert "intentionally not called from `update_user_profile`" in service

    update_profile_req = routes[
        routes.index("pub struct UpdateProfileReq") : routes.index("async fn get_profile_me")
    ]
    for forbidden_field in [
        "is_official",
        "verified_expert",
        "official_poool",
        "reputation_flairs",
    ]:
        assert forbidden_field not in update_profile_req

    assert "service::get_reputation_flairs_batch(c_pool, &user_ids)" in routes
    assert "p.author_reputation_flairs" in post_card
    assert "official_poool" in post_card
    assert "'POOOL' in p.author_name" not in post_card


def test_phase3_frontend_and_admin_ops_surface_types_without_content_first_regression():
    discover_js = read("frontend/platform/static/js/community-circles-discover.js")
    admin_detail = read("frontend/platform/admin/community/circle-detail.html")
    admin_list = read("frontend/platform/static/js/admin-community-circles.js")
    circle_page = read("frontend/platform/community-circle.html")

    for token in [
        "circle.join_policy === 'holder_only'",
        "circle.join_policy === 'kyc_required'",
        "function visibilityOf",
        "private_investor_club",
        "is_official",
    ]:
        assert token in discover_js

    for token in [
        "edit-circle-type",
        "edit-join-policy",
        "edit-is-official",
        "edit-kyc-required",
        "edit-private-investor-club",
        "allow_cross_post",
        "<option value=\"hidden\"",
    ]:
        assert token in admin_detail

    assert "function createTypeCell" in admin_list
    assert "circle.circle_type" in admin_list
    assert "circle.visibility" in admin_list
    assert "Holder-only" in admin_list
    assert "KYC" in admin_list
    assert "circle.member_count || 0} / ${circle.max_members || 0" not in admin_list
    assert "Number(c.member_count || 0)} / ${Number(c.max_members || 0)" not in admin_detail

    assert "circle-space-hero__badges" in circle_page
    assert "Private Investor Club" in circle_page
    assert "KYC-gated" in circle_page
    assert "href=\"/community/circle/{{ circle_slug }}/settings\"" in circle_page
