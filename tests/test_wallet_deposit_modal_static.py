from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_deposit_instruction_modal_uses_brand_logo_and_structured_warning():
    wallet_js = (ROOT / "frontend/platform/static/js/wallet.js").read_text()
    wallet_css = (ROOT / "frontend/platform/static/css/wallet.css").read_text()

    assert "/static/images/logos/poool-icon.svg" in wallet_js
    assert 'class="dim-warning" role="note"' in wallet_js
    assert "Reference required." in wallet_js
    assert "⚠️ Include the reference number" not in wallet_js

    for selector in (
        ".dim-modal",
        ".dim-header",
        ".dim-logo-mark",
        ".dim-warning__icon",
    ):
        assert selector in wallet_css
