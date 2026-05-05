from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_rewards_referral_input_matches_copy_button_height():
    css = read("frontend/platform/static/css/rewards.css")

    group_start = css.index(".refer-input-group {")
    group_end = css.index(".refer-input-wrapper", group_start)
    group_block = css[group_start:group_end]
    wrapper_start = css.index(".refer-input-wrapper {")
    wrapper_end = css.index(".refer-input-wrapper:focus-within", wrapper_start)
    wrapper_block = css[wrapper_start:wrapper_end]
    button_start = css.index(".copy-link-btn {")
    button_end = css.index(".copy-link-btn:hover", button_start)
    button_block = css[button_start:button_end]
    scoped_button_start = css.index("body#rewards-body .copy-link-btn,")
    scoped_button_end = css.index("body#rewards-body .copy-link-btn:hover", scoped_button_start)
    scoped_button_block = css[scoped_button_start:scoped_button_end]
    scoped_input_start = css.index("body#rewards-body .refer-input-wrapper,")
    scoped_input_end = css.index("body#rewards-body .refer-input-wrapper:focus-within", scoped_input_start)
    scoped_input_block = css[scoped_input_start:scoped_input_end]

    assert "align-items: center;" in group_block
    assert "min-height: 40px;" in wrapper_block
    assert "padding: 0 14px;" in wrapper_block
    assert "min-height: 40px;" in button_block
    assert "padding: 0 16px;" in button_block
    assert "min-height: 40px;" in scoped_button_block
    assert "min-height: 40px;" in scoped_input_block


def test_rewards_marketing_tier_icons_card_removed():
    html = read("frontend/platform/rewards.html")
    marketing_start = html.index('id="marketing-tab"')
    marketing_end = html.index("<!-- Tab 5:", marketing_start)
    marketing_html = html[marketing_start:marketing_end]

    assert "Tier Icons" not in marketing_html
    assert "Official investor tier badge icons" not in marketing_html
    assert "Glassmorphism" not in marketing_html
    assert "tier-icon-var" not in marketing_html
    assert "Social Media Templates" in marketing_html


def test_rewards_commissions_cards_follow_design_card_contract():
    html = read("frontend/platform/rewards.html")
    css = read("frontend/platform/static/css/rewards.css")

    commissions_start = html.index('id="commissions-tab"')
    commissions_end = html.index("<!-- end commissions-tab", commissions_start)
    commissions_html = html[commissions_start:commissions_end]
    card_surface_start = css.index("body#rewards-body .tier-progress-card,")
    card_surface_end = css.index("body#rewards-body .tier-card--active", card_surface_start)
    card_surface_block = css[card_surface_start:card_surface_end]
    accent_start = css.index("body#rewards-body .tier-card--active::before,")
    accent_end = css.index("body#rewards-body .tier-progress-card,", accent_start)
    accent_block = css[accent_start:accent_end]

    assert 'class="commissions-summary-grid"' in commissions_html
    assert 'class="quick-insights-card commissions-summary-card"' in commissions_html
    assert 'class="commissions-card commissions-card--form"' in commissions_html
    assert 'class="commissions-card commissions-card--table"' in commissions_html
    assert "<h3>Payout settings</h3>" in commissions_html
    assert "<h3>Commissions</h3>" in commissions_html
    assert 'style="display:grid; grid-template-columns' not in commissions_html
    assert '<h2 class="section-title">Payout settings</h2>' not in commissions_html
    assert '<h2 class="section-title" style="margin-top: 32px;">Commissions</h2>' not in commissions_html

    assert ".commissions-summary-grid {" in css
    assert "grid-template-columns: repeat(3, minmax(0, 1fr));" in css
    assert "gap: var(--section-gap, 24px);" in css
    assert "body#rewards-body .commissions-summary-card," in card_surface_block
    assert "body#rewards-body .commissions-card," in card_surface_block
    assert "border-radius: 12px !important;" in card_surface_block
    assert "box-shadow: var(--rewards-card-shadow) !important;" in card_surface_block
    assert "body#rewards-body .commissions-summary-card::before," in accent_block
    assert "body#rewards-body .commissions-card::before" in accent_block
    assert "height: 3px;" in accent_block
    assert "linear-gradient(90deg, var(--primary-color, #0000FF), var(--progress-fill-bg, #98FB96))" in accent_block
