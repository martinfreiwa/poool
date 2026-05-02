from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_add_bank_modal_does_not_render_transfer_system_badge():
    wallet_html = (ROOT / "frontend/platform/wallet.html").read_text()
    wallet_css = (ROOT / "frontend/platform/static/css/wallet.css").read_text()

    assert "bank-dynamic-fields" in wallet_html
    assert "ACH Transfer" not in wallet_html
    assert "ds-badge ds-badge--info" not in wallet_html
    assert "systemLabel" not in wallet_html

    assert "/static/images/logos/poool-icon.svg" in wallet_html
    assert "POOOL withdrawals" in wallet_html
    assert "wallet-bank-modal__trust-row" in wallet_html
    assert "wallet-bank-form__section" in wallet_html

    for selector in (
        ".wallet-bank-modal",
        ".wallet-bank-modal__logo",
        ".wallet-bank-form__section",
        ".wallet-bank-form__actions",
    ):
        assert selector in wallet_css
