from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TRADING_V3_HTML = ROOT / "frontend/platform/marketplace-trading-v3.html"


def test_trading_v3_does_not_render_p2p_offer_panel():
    html = TRADING_V3_HTML.read_text()

    assert "marketplace-p2p" not in html
    assert "tv3-p2p-panel" not in html
    assert "tv3-p2p-container" not in html
    assert "MarketP2P.init" not in html

