from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OVERVIEW_HTML = ROOT / "frontend/platform/admin/marketplace/index.html"
OVERVIEW_JS = ROOT / "frontend/platform/static/js/mp-index.js"


def test_marketplace_overview_halt_trading_uses_stop_sign_icon():
    source = OVERVIEW_HTML.read_text(encoding="utf-8")

    halt_start = source.index('id="mp-halt-btn"')
    halt_end = source.index("</button>", halt_start)
    halt_button = source[halt_start:halt_end]

    assert "Halt Trading" in halt_button
    assert "<svg" in halt_button
    assert 'aria-hidden="true"' in halt_button
    assert 'd="M8.1 3h7.8L21 8.1v7.8L15.9 21H8.1L3 15.9V8.1L8.1 3z"' in halt_button
    assert 'd="M8 12h8"' in halt_button
    assert '<rect x="6" y="6" width="12" height="12" rx="1"/>' not in halt_button


def test_marketplace_overview_removes_large_system_health_card_but_keeps_topbar_dots():
    html = OVERVIEW_HTML.read_text(encoding="utf-8")
    js = OVERVIEW_JS.read_text(encoding="utf-8")

    assert 'aria-label="System health"' in html
    assert 'id="health-dot-matching"' in html
    assert 'id="health-dot-db"' in html
    assert 'id="health-dot-ws"' in html
    assert "System Health" not in html
    assert 'id="mp-health-summary"' not in html
    assert 'id="health-grid"' not in html
    assert 'id="health-tier-critical"' not in html
    assert 'id="health-tier-info"' not in html
    assert "function updateTopbarHealth" in js
    assert "function renderHealth(health)" in js
    assert "function renderHealthMessage" not in js
