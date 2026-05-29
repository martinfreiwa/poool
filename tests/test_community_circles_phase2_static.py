from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase2_discover_filter_chips_are_available_and_accessible():
    partial = read("frontend/platform/partials/community_circle.html")

    for filter_value in [
        "all",
        "featured",
        "trending",
        "new",
        "asset",
        "private",
        "public",
        "official",
        "holder-only",
        "kyc-gated",
    ]:
        assert f'data-cc-filter="{filter_value}"' in partial

    assert 'data-cc-filter="all" aria-pressed="true"' in partial
    assert 'data-cc-filter="private" aria-pressed="false"' in partial
    assert "Asset Circles" in partial
    assert "Holder-only" in partial
    assert "KYC-gated" in partial


def test_phase2_discover_frontend_models_access_states_and_actions():
    js = read("frontend/platform/static/js/community-circles-discover.js")
    css = read("frontend/platform/static/css/community.css")

    assert "function getFilterTags" in js
    assert "function hasFilterTag" in js
    assert "function primaryAction" in js
    assert "function actionMenu" in js
    assert "data-cc-filter-tags" in js
    assert "data-cc-open" in js
    assert "cc-card--clickable" in js
    assert "role=\"link\"" in js
    assert "tagItems(data.private, 'private')" in js
    assert "tagItems(data.public, 'public')" in js
    assert "tagItems(data.asset, 'asset')" in js
    assert "tagItems(data.holder_only, 'holder-only')" in js
    assert "tagItems(data.kyc_gated, 'kyc-gated')" in js
    assert "Request Access" in js
    assert "data-cc-request" in js
    assert "Locked" in js
    assert "data-cc-copy" in js
    assert "data-cc-leave" in js
    assert "/api/community/circles/leave" in js
    assert "/request" in js
    assert "New Circle · Be the first to post" not in js
    assert "1 post this week" not in js
    assert "actionMenu(c, role, 'card')" not in js
    assert ".cc-card--clickable" in css
    assert ".cc-card:focus-visible" in css
    assert "@media (max-width: 640px)" in css
    assert "grid-template-columns: 1fr" in css


def test_phase2_discover_backend_payload_exposes_category_groups():
    circles = read("backend/src/community/circles.rs")
    routes = read("backend/src/community/routes.rs")

    for field in [
        "pub public: Vec<CircleCardRow>",
        "pub private: Vec<CircleCardRow>",
        "pub asset: Vec<CircleCardRow>",
        "pub holder_only: Vec<CircleCardRow>",
        "pub official: Vec<CircleCardRow>",
        "pub kyc_gated: Vec<CircleCardRow>",
        "pub token_gate_asset_id: Option<Uuid>",
        "pub token_gate_asset_name: Option<String>",
    ]:
        assert field in circles

    assert "WHERE (is_public = FALSE OR visibility = 'private')" in circles
    assert "WHERE (token_gate_asset_id IS NOT NULL OR circle_type = 'asset')" in circles
    assert "WHERE (is_official = TRUE OR circle_type = 'official')" in circles
    assert "WHERE (kyc_required = TRUE OR join_policy = 'kyc_required')" in circles
    assert "visibility <> 'hidden'" in circles

    for payload_field in [
        '"public"',
        '"private"',
        '"asset"',
        '"holder_only"',
        '"official"',
        '"kyc_gated"',
    ]:
        assert payload_field in routes

    assert "payload.holder_only.len()" in routes
    assert ".chain(payload.kyc_gated.iter())" in routes
