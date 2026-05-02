from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TRADE_SUCCESS_HTML = ROOT / "frontend/platform/trade-success.html"
PAYMENT_SUCCESS_CSS = ROOT / "frontend/platform/static/css/payment-success.css"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_trade_success_has_branding_and_no_confetti_animation():
    template = read(TRADE_SUCCESS_HTML)

    assert "trade-success-body" in template
    assert "trade-success-brand" in template
    assert "/static/images/logos/Logo Pool.svg" in template
    assert "Resale market order" in template
    assert "trade-success-status-strip" in template
    assert "Order accepted" in template
    assert "No charge until matched" in template
    assert "confetti-container" not in template
    assert "confetti-fall" not in template
    assert "appendChild(particle)" not in template


def test_trade_success_uses_css_classes_instead_of_inline_notice_and_copy_styles():
    template = read(TRADE_SUCCESS_HTML)

    assert "payment-success-info-notice" in template
    assert "payment-success-order__copy-group" in template
    assert "payment-success-order__value--mono" in template
    assert "payment-success-order__copy" in template
    assert 'style="display:flex; align-items:center; gap:6px;"' not in template
    assert "background:#F0F9FF" not in template


def test_trade_success_css_matches_dashboard_design_language():
    css = read(PAYMENT_SUCCESS_CSS)

    assert ".trade-success-card {" in css
    assert "border-radius: 12px;" in css
    assert "box-shadow: var(--card-shadow, 0 1px 2px rgba(10, 13, 18, 0.05));" in css
    assert ".trade-success-card::before" in css
    assert "background: linear-gradient(90deg, var(--primary-color), #62F7A4);" in css
    assert ".payment-success-btn--primary {\n    background: var(--primary-color);\n    color: #98FB96;" in css
    assert ".payment-success-btn--secondary:hover {\n    background: white;" in css
    assert ".payment-success-info-notice" in css
    assert ".confetti-container" in css
