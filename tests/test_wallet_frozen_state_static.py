from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_wallet_renders_frozen_account_review_banner_and_disabled_actions():
    html = read("frontend/platform/wallet.html")
    css = read("frontend/platform/static/css/wallet.css")

    assert 'id="wallet-frozen-banner"' in html
    assert 'data-account-frozen="{% if account_frozen %}true{% else %}false{% endif %}"' in html
    assert 'id="wallet-unfreeze-note"' in html
    assert 'id="wallet-unfreeze-request-btn"' in html
    assert "Request review" in html
    assert "Review already requested" in html
    assert "deposits, withdrawals, cards, and bank changes are disabled" in html

    assert 'onclick="openDepositModal()"{% if account_frozen %} disabled aria-disabled="true"{% endif %}' in html
    assert 'onclick="openWithdrawModal()"{% if account_frozen %} disabled aria-disabled="true"{% endif %}' in html
    assert 'onclick="openCardModal()"{% if account_frozen %} disabled aria-disabled="true"{% endif %}' in html
    assert 'onclick="openBankModal()"{% if account_frozen %} disabled aria-disabled="true"{% endif %}' in html

    assert ".wallet-frozen-banner {" in css
    assert "border: 1px solid #F2C94C;" in css
    assert ".wallet-account-frozen button:disabled" in css


def test_wallet_frozen_state_javascript_blocks_mutations_and_posts_review_request():
    html = read("frontend/platform/wallet.html")

    assert "window.__WALLET_ACCOUNT_FROZEN" in html
    assert "function guardWalletFrozenAction(event)" in html
    assert "Wallet actions are paused. Request compliance review to restore account access." in html
    assert "async function requestWalletUnfreezeReview()" in html
    assert 'fetch("/api/wallet/unfreeze-request"' in html
    assert 'body: JSON.stringify({ note: note ? note.value : "" })' in html
    assert "if (guardWalletFrozenAction()) return;" in html
    assert "unfreezeBtn.addEventListener(\"click\", requestWalletUnfreezeReview)" in html
