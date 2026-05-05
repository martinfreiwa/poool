from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_cart_empty_step_badges_match_submission_card_design():
    html = read("frontend/platform/cart.html")
    css = read("frontend/platform/static/css/cart.css")

    assert 'class="cart-empty__step-num">01</span>' in html
    assert 'class="cart-empty__step-num">02</span>' in html
    assert 'class="cart-empty__step-num">03</span>' in html

    start = css.index(".cart-empty__step-num {")
    end = css.index(".cart-empty__step-body", start)
    block = css[start:end]

    assert "display: inline-flex;" in block
    assert "align-items: center;" in block
    assert "justify-content: center;" in block
    assert "width: 30px;" in block
    assert "height: 30px;" in block
    assert "font-size: 11px;" in block
    assert "font-weight: 800;" in block
    assert "color: #03FF88;" in block
    assert "background: linear-gradient(135deg, #0000FF 0%, #3344FF 100%);" in block
    assert "border-radius: 8px;" in block
    assert "letter-spacing: 0.04em;" in block
    assert "var(--badge-info-bg" not in block
