from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_blockchain_treasury_uses_custom_svg_icons():
    html = (ROOT / "frontend/platform/admin/blockchain-treasury.html").read_text()

    assert "📋" not in html
    assert ".bt-icon" in html
    assert ".bt-stat-grid" in html
    assert ".bt-kpi-card-title" in html
    assert "class=\"copy-btn\"" in html

    assert html.count('class="bt-icon') >= 8
    assert html.count("<svg") >= 9

    for label in (
        "Settlement Wallet (SETTLEMENT_ROLE)",
        "Quick Stats",
        "Whitelisted Users",
        "Settlement Batches",
        "Pending Trades",
        "Confirmed On-Chain",
        "Tokenized Assets",
        "Total Token Supply",
        "Tokens Sold",
    ):
        assert label in html


def test_blockchain_treasury_icon_classes_are_scoped_to_page():
    html = (ROOT / "frontend/platform/admin/blockchain-treasury.html").read_text()

    style_start = html.index("<style>")
    style_end = html.index("</style>", style_start)
    style = html[style_start:style_end]

    for selector in (
        ".bt-section-title",
        ".bt-icon",
        ".bt-icon--green",
        ".bt-icon--amber",
        ".bt-stat",
        ".bt-stat-label",
        ".bt-kpi-card-title",
    ):
        assert selector in style
