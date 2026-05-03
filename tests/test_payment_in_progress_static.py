from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PAYMENT_JS = ROOT / "frontend/platform/static/js/payment-in-progress.js"
PAYMENT_CSS = ROOT / "frontend/platform/static/css/payment-in-progress.css"
PAYMENTS_ROUTES = ROOT / "backend/src/payments/routes.rs"


def test_payment_progress_removes_header_badge_and_support_card():
    js = PAYMENT_JS.read_text(encoding="utf-8")

    assert 'let headerHTML = "";' in js
    assert "pip-support-card" not in js
    assert "Questions or Problems?" not in js
    assert "Support Portal" not in js


def test_payment_progress_keeps_status_in_payment_details():
    js = PAYMENT_JS.read_text(encoding="utf-8")

    assert js.count("pip-status-badge pip-status-badge--${sc.cssClass}") == 2
    assert js.count("pip-status-badge__dot") == 2


def test_payment_progress_shows_purchased_item_card():
    js = PAYMENT_JS.read_text(encoding="utf-8")
    css = PAYMENT_CSS.read_text(encoding="utf-8")

    assert "renderPurchasedItemCard" in js
    assert "pip-purchased-section" in js
    assert "pip-purchased-card__stats" in js
    assert "Order Items" not in js
    assert ".pip-purchased-card" in css


def test_latest_order_payload_includes_asset_card_metadata():
    routes = PAYMENTS_ROUTES.read_text(encoding="utf-8")

    assert "a.slug" in routes
    assert "a.location_city" in routes
    assert "a.asset_type" in routes
    assert "cover_image_url" in routes
    assert "rewrite_gcs_url" in routes
