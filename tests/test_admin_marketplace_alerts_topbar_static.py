from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_marketplace_alerts_topbar_uses_svg_icons_not_emoji():
    html = (ROOT / "frontend/platform/admin/marketplace/alerts.html").read_text()

    topbar_start = html.index('class="admin-topbar-right"')
    topbar_end = html.index("</header>", topbar_start)
    topbar = html[topbar_start:topbar_end]

    assert 'class="alerts-fraud-link"' in topbar
    assert 'class="alerts-topbar-icon"' in topbar
    assert 'class="alerts-fraud-count"' in topbar
    assert 'class="alerts-volume-control"' in topbar
    assert 'aria-label="Alert sound volume"' in topbar
    assert 'id="alerts-btn-notify"' in topbar

    for emoji in ("⚠", "🔊", "🔔"):
        assert emoji not in topbar

    assert topbar.count("<svg") >= 4
