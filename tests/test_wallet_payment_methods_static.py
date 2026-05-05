from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_wallet_payment_method_cards_use_compact_neutral_actions():
    html = read("frontend/platform/wallet.html")
    css = read("frontend/platform/static/css/wallet.css")

    assert '<div class="wallet-payment-card-header">\n                  <h3 id="wallet-payment-cards-title" class="wallet-payment-title">Cards</h3>' in html
    assert '<div class="wallet-payment-card-header">\n                  <h3 id="wallet-payment-banks-title" class="wallet-payment-title">Banks</h3>' in html
    assert "margin-top: 0 !important;" in css
    assert "min-height: 176px;" in css
    assert ".wallet-payment-card-header" in css
    assert "font-size: 20px;" in css
    assert "margin: 0;" in css
    assert "#wallet-payment-cards-add-button,\n#wallet-payment-banks-add-button" in css
    assert "background: #FFFFFF !important;" in css
    assert "border: 1px solid #D5D7DA !important;" in css
    assert "color: #414651;" in css
    assert "box-shadow: none !important;" in css
    assert "color: #0000FF;" in css


def test_wallet_payment_empty_states_are_not_oversized():
    css = read("frontend/platform/static/css/wallet.css")

    assert ".wallet-payment-empty-state {\n  padding: 18px 16px 12px;" in css
    assert "min-height: 72px;" in css
    assert "font-size: 13px;" in css
    assert ".wallet-payment-empty-state svg {\n  width: 22px;\n  height: 22px;" in css


def test_wallet_transactions_title_lives_inside_table_card():
    html = read("frontend/platform/wallet.html")
    css = read("frontend/platform/static/css/wallet.css")

    table_start = html.index('<div id="transactions-table" class="ds-table-container table__wrapper--wallet">')
    title_start = html.index('id="wallet-transactions-title-wrapper"', table_start)
    flex_start = html.index('<div id="wallet-transactions-container" class="ds-table-flex">', table_start)

    assert table_start < title_start < flex_start
    assert ".wallet-transactions-header {\n  display: flex;" in css
    assert "padding: 20px 24px 16px;" in css
    assert "border-bottom: 1px solid var(--table-border-color);" in css
    assert "#transactions-table .ds-table-flex__head {\n  border-radius: 0;" in css


def test_wallet_cards_use_shared_blue_green_accent_strip():
    css = read("frontend/platform/static/css/wallet.css")

    assert ".wallet-balance-card::before,\n#transactions-table::before,\n.wallet-payment-card::before" in css
    assert "height: 3px;" in css
    assert "background: linear-gradient(90deg, #0000FF 0%, #03FF88 100%);" in css
    assert "#transactions-table {\n  position: relative;" in css
    assert ".wallet-balance-card" in css and "position: relative;" in css
    assert ".wallet-payment-card" in css and "overflow: hidden;" in css
