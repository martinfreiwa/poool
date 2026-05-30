from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def css_block(css: str, selector: str, after: int = 0) -> str:
    start = css.index(selector, after)
    end = css.index("}", start)
    return css[start:end]


def test_rewards_referral_share_is_compact_inline_control():
    html = read("frontend/platform/rewards.html")
    css = read("frontend/platform/static/css/rewards.css")
    js = read("frontend/platform/static/js/rewards.js")

    refer_start = html.index('<div id="rewards-refer-card"')
    refer_end = html.index("</div>\n          </div> <!-- end rewards-tab panel -->", refer_start)
    refer_html = html[refer_start:refer_end]
    share_start = refer_html.index('<div class="refer-share-form"')
    share_html = refer_html[share_start:]

    share_block = css_block(css, ".refer-share-form {")
    share_row_block = css_block(css, ".refer-share-row {")
    link_block = css_block(css, ".refer-link-value {")
    icon_block = css_block(css, ".refer-copy-link-icon {")
    icon_hover_block = css_block(css, ".refer-copy-link-icon:hover {")
    standard_card_block = css_block(css, "body#rewards-body .rewards-overview-card,")
    refer_action_override = css_block(css, "body#rewards-body .rewards-overview-card .refer-action-area {")
    flow_block = css_block(css, ".refer-flow-list {")
    flow_connector_block = css_block(css, ".refer-flow-step__connector {")
    flow_number_block = css_block(css, ".refer-flow-step__number {")

    assert '<span id="rewards-referral-input" class="refer-link-value"' in share_html
    assert '<input id="rewards-referral-input"' not in share_html
    assert 'class="refer-input-wrapper"' not in share_html
    assert 'class="refer-share-row"' in share_html
    assert 'class="refer-copy-link-icon"' in share_html
    assert 'aria-label="Copy referral link"' in share_html
    assert "Copy link" not in share_html
    assert 'class="refer-share-actions"' not in share_html
    assert 'class="refer-card-main rewards-overview-card__body"' in refer_html
    assert 'class="refer-action-area rewards-overview-card__footer"' in refer_html
    assert 'class="refer-flow-list refer-flow-progress"' in refer_html
    assert 'class="refer-flow-step__connector"' in refer_html
    assert 'id="rewards-copy-message-btn"' not in share_html
    assert "Copy message" not in share_html
    assert 'id="rewards-email-invite-link"' not in share_html
    assert "Email" not in share_html
    assert 'class="refer-reward-tiles"' not in refer_html
    assert "refer-reward-tile" not in css

    assert "display: flex !important;" in standard_card_block
    assert "flex-direction: column !important;" in standard_card_block
    assert "padding: 0 !important;" in standard_card_block
    assert "border-left: 0 !important;" in refer_action_override
    assert "display: flex;" in flow_block
    assert "background: linear-gradient(90deg, var(--primary-color, #0000FF), var(--brand-greeny-green, #03FF88));" in flow_connector_block
    assert "background: #0000FF;" in flow_number_block
    assert "color: #03FF88;" in flow_number_block
    assert "display: flex;" in share_block
    assert "flex-direction: column;" in share_block
    assert "width: 100%;" in share_block
    assert "padding: 0;" in share_block
    assert "border: 0;" in share_block
    assert "background: transparent;" in share_block
    assert "gap: 8px;" in share_row_block
    assert "text-overflow: ellipsis;" in link_block
    assert "white-space: nowrap;" in link_block
    assert "width: 28px;" in icon_block
    assert "height: 28px;" in icon_block
    assert "color: #667085;" in icon_block
    assert "background: transparent;" in icon_block
    assert "border: 0;" in icon_block
    assert "color: #0000FF;" in icon_hover_block
    assert "background: transparent;" in icon_hover_block
    assert "refer-secondary-action" not in css
    assert "body#rewards-body .refer-share-form .copy-link-btn" not in css

    assert "input.dataset.copyValue = baseReferralLink;" in js
    assert "input.textContent = baseReferralLink || \"No link generated\";" in js
    assert "function getCopyValue(element)" in js
    assert "element.dataset.copyValue" in js
    assert "function copyReferralMessage" not in js
    assert "rewards-copy-message-btn" not in js
    assert "rewards-email-invite-link" not in js
    assert "function buildReferralMessage" not in js
    assert "function updateReferralEmailLink" not in js


def test_rewards_overview_cards_share_standard_card_structure():
    html = read("frontend/platform/rewards.html")
    css = read("frontend/platform/static/css/rewards.css")

    rewards_start = html.index('id="rewards-tab"')
    rewards_end = html.index("<!-- Tab 3: Tier -->", rewards_start)
    rewards_html = html[rewards_start:rewards_end]

    card_block = css_block(css, "body#rewards-body .rewards-overview-card,")
    header_block = css_block(css, "body#rewards-body .rewards-overview-card__header {")
    body_block = css_block(css, "body#rewards-body .rewards-overview-card__body {")
    footer_block = css_block(css, "body#rewards-body .rewards-overview-card__footer {")
    icon_block = css_block(css, "body#rewards-body .rewards-overview-card__icon {")
    title_block = css_block(css, "body#rewards-body .rewards-overview-card__title {")

    assert 'class="tier-progress-card rewards-overview-card"' in rewards_html
    assert 'class="rewards-summary-card rewards-overview-card"' in rewards_html
    assert 'class="refer-earn-card refer-earn-card--action rewards-overview-card"' in rewards_html
    assert rewards_html.count("rewards-overview-card__header") == 3
    assert rewards_html.count("rewards-overview-card__body") == 3
    assert rewards_html.count("rewards-overview-card__footer") == 3
    assert rewards_html.count("rewards-overview-card__icon") == 3
    assert "summary-card-body rewards-overview-card__body" in rewards_html
    assert "tp-hint rewards-overview-card__footer" in rewards_html

    assert "padding: 0 !important;" in card_block
    assert "background: var(--rewards-card-bg) !important;" in card_block
    assert "border-bottom: 1px solid var(--rewards-card-border) !important;" in header_block
    assert "padding: 20px 24px !important;" in header_block
    assert "padding: 18px 24px !important;" in body_block
    assert "border-top: 1px solid var(--rewards-card-border) !important;" in footer_block
    assert "padding: 14px 24px 16px !important;" in footer_block
    assert "background: linear-gradient(135deg, #0000FF 0%, #1E40AF 100%) !important;" in icon_block
    assert "color: #03FF88 !important;" in icon_block
    assert "font-size: 18px !important;" in title_block


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
