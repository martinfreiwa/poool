from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_developer_asset_detail_api_exposes_image_ids_and_sort_order():
    routes = read("backend/src/developer/routes.rs")

    assert "SELECT id, COALESCE(image_url,''), COALESCE(is_cover,false), COALESCE(sort_order,0) FROM asset_images" in routes
    assert '"id": i.0, "url": url, "is_cover": i.2, "sort_order": i.3' in routes


def test_developer_asset_detail_frontend_can_reorder_images_with_indicators():
    js = read("frontend/platform/static/js/developer-asset-detail.js")
    css = read("frontend/platform/static/css/developer-asset-detail.css")

    assert "media-order-badge" in js
    assert "Drag images or use the arrow buttons to set the display order" in js
    assert "function moveDetailImage" in js
    assert "function syncDetailImageOrder" in js
    assert "/api/developer/draft/${assetId}/images/reorder" in js
    assert "is_cover: index === 0" in js

    assert ".media-order-badge" in css
    assert ".media-order-controls" in css
    assert ".media-order-btn" in css
    assert ".media-order-cover" in css


def test_developer_image_reorder_endpoint_validates_payload():
    storage_routes = read("backend/src/storage/routes.rs")

    assert "Exactly one image must be marked as cover" in storage_routes
    assert "Duplicate image id in reorder payload" in storage_routes
    assert "sort_order must not be negative" in storage_routes
    assert "updated.rows_affected() == 1" in storage_routes
    assert "Image not found for this asset" in storage_routes


def test_investor_image_queries_respect_saved_sort_order():
    for path in [
        "backend/src/assets/routes.rs",
        "backend/src/marketplace/service.rs",
        "backend/src/auth/routes.rs",
    ]:
        source = read(path)
        assert "ORDER BY is_cover DESC, created_at ASC" not in source

    assets_routes = read("backend/src/assets/routes.rs")
    assert "ORDER BY is_cover DESC, sort_order ASC, created_at ASC" in assets_routes
