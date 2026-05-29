from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def css_block(css: str, selector: str, after: int = 0) -> str:
    start = css.index(selector, after)
    end = css.index("}", start)
    return css[start:end]


def test_my_trading_summary_and_table_match_hero_width():
    css = read("frontend/platform/static/css/my-trading.css")

    shared_width_block = css_block(css, ".myt-hero,")
    assert ".myt-stats-row.sub-stats" in shared_width_block
    assert ".myt-table-card.submissions-table-container" in shared_width_block
    assert "max-width: 100% !important;" in shared_width_block
    assert "width: 100% !important;" in shared_width_block

    stats_grid_block = css_block(
        css,
        ".myt-stats-row.sub-stats",
        css.index("/* === Summary Cards"),
    )
    assert "grid-template-columns: repeat(4, minmax(0, 1fr));" in stats_grid_block
    assert "width: 100% !important;" in stats_grid_block

    table_block = css_block(css, ".myt-table-card.submissions-table-container", css.index("/* === Card Header"))
    assert "width: 100% !important;" in table_block
    assert "max-width: 100% !important;" in table_block


def test_my_trading_summary_cards_use_hero_card_density():
    css = read("frontend/platform/static/css/my-trading.css")

    stat_card_block = css_block(css, ".myt-stats-row .sub-stat {")
    assert "min-height: 88px !important;" in stat_card_block
    assert "padding: 18px 20px !important;" in stat_card_block
    assert "justify-content: center !important;" in stat_card_block


def test_my_trading_lower_cards_do_not_render_top_accent():
    css = read("frontend/platform/static/css/my-trading.css")

    assert ".myt-table-card.submissions-table-container::before {\n    content: \"\";" not in css

    stat_no_accent_block = css_block(
        css,
        "body.my-trading-body .myt-stats-row .sub-stat::before",
    )
    assert "body.my-trading-body .myt-stats-row .sub-stat.active::before" in stat_no_accent_block
    assert "body.my-trading-body .myt-table-card.submissions-table-container::before" in stat_no_accent_block
    assert "content: none !important;" in stat_no_accent_block
    assert "display: none !important;" in stat_no_accent_block
    assert "height: 0 !important;" in stat_no_accent_block
    assert "background: none !important;" in stat_no_accent_block
