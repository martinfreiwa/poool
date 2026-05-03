from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SETTINGS_HTML = ROOT / "frontend/platform/admin/marketplace/settings.html"
ORDERS_HTML = ROOT / "frontend/platform/admin/marketplace/orders.html"
ADMIN_MP_CSS = ROOT / "frontend/platform/static/css/admin-marketplace.css"


def test_settings_helper_text_not_bound_to_fullscreen_overlay_class():
    settings = SETTINGS_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")

    assert 'id="help-batch"' in settings
    assert 'class="mp-help"' in settings

    overlay_rule_start = css.index("/* Help overlay */")
    overlay_rules = css[overlay_rule_start:css.index("/* Audit diff table */", overlay_rule_start)]

    assert ".mp-help-overlay" in overlay_rules
    assert ".mp-help-overlay[hidden]" in overlay_rules
    assert ".mp-help { position: fixed" not in overlay_rules
    assert ".mp-help[hidden]" not in overlay_rules


def test_orders_keyboard_help_uses_overlay_class_explicitly():
    orders = ORDERS_HTML.read_text(encoding="utf-8")

    assert 'id="orders-help" class="mp-help mp-help-overlay"' in orders
    assert 'class="mp-help-backdrop"' in orders
    assert 'class="mp-help-panel"' in orders
