from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
LEADERBOARD_TEMPLATE = REPO_ROOT / "frontend/platform/leaderboard.html"
LEADERBOARD_CSS = REPO_ROOT / "frontend/platform/static/css/leaderboard.css"


def test_investor_leaderboard_empty_state_uses_logo_not_icon():
    html = LEADERBOARD_TEMPLATE.read_text(encoding="utf-8")
    css = LEADERBOARD_CSS.read_text(encoding="utf-8")
    empty_state = html.split('id="lb-empty-layer"', 1)[1].split("<!-- 4. CONTENT STATE -->", 1)[0]

    assert "lb-empty-logo" in empty_state
    assert "/static/images/logos/Logo%20Pool.svg" in empty_state
    assert 'class="empty-icon"' not in empty_state
    assert ".lb-empty-layer .lb-empty-logo" in css
    assert "width: 112px;" in css
