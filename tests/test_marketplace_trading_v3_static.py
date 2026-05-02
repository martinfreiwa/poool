from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TRADING_V3_JS = ROOT / "frontend/platform/static/js/marketplace-trading-v3.js"
TRADING_V3_HTML = ROOT / "frontend/platform/marketplace-trading-v3.html"
TRADING_V3_CSS = ROOT / "frontend/platform/static/css/marketplace-trading-v3.css"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_trading_lightbox_reuses_property_detail_lightbox_classes():
    source = read(TRADING_V3_JS)

    assert "overlay.className = 'lightbox-modal lightbox-opening'" in source
    assert "topBar.className = 'lightbox-top-bar'" in source
    assert "counter.className = 'lightbox-counter'" in source
    assert "imageWrapper.className = 'lightbox-image-wrapper'" in source
    assert "img.className = 'lightbox-content'" in source
    assert "thumbnails.className = 'lightbox-thumbnails'" in source
    assert "thumb.className = 'lightbox-thumb'" in source
    assert "'lightbox-prev'" in source
    assert "'lightbox-next'" in source


def test_trading_lightbox_has_property_detail_feature_parity():
    source = read(TRADING_V3_JS)

    assert "document.body.classList.add('lightbox-open')" in source
    assert "document.body.classList.remove('lightbox-open')" in source
    assert "e.key === 'Escape'" in source
    assert "e.key === 'ArrowLeft'" in source
    assert "e.key === 'ArrowRight'" in source
    assert "touchstart" in source
    assert "touchmove" in source
    assert "touchend" in source
    assert "scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' })" in source


def test_trading_page_loads_shared_property_detail_lightbox_css_before_v3_css():
    template = read(TRADING_V3_HTML)

    assert "{% set extra_css=['property-detail', 'marketplace-trading-v3', 'marketplace-orderbook'] %}" in template


def test_trading_page_contains_legacy_property_detail_width_overrides():
    css = read(TRADING_V3_CSS)

    assert "V3 PROPERTY DETAIL CONTAINMENT" in css
    assert ".tv3-main-card .info-badge-text {\n    flex: 1 1 auto;" in css
    assert ".tv3-main-card .funding-step-base:nth-child(2) .funding-step-text" in css
    assert ".tv3-main-card .funding-step-base.last .funding-step-text" in css
    assert ".tv3-main-card .leasing-item-header h3" in css
    assert ".tv3-main-card .tv3-calc-statistics-card" in css
    assert "grid-template-columns: repeat(3, minmax(0, 1fr));" in css
    assert ".tv3-main-card .faq-item {\n    min-width: 0;" in css
    assert "overflow-wrap: anywhere;" in css


def test_trading_page_uses_inline_document_download_icons():
    template = read(TRADING_V3_HTML)

    assert "download-cloud-02.svg" not in template
    assert template.count('class="tv3-doc-dl" type="button"') == 3
    assert template.count('class="tv3-doc-dl-icon"') == 3
    assert 'aria-label="Download Smart Contract"' in template
    assert 'aria-label="Download Token Registration"' in template
    assert 'aria-label="Download Articles of Organization"' in template


def test_trading_page_normalizes_secondary_location_without_duplicate_country():
    source = read(TRADING_V3_JS)

    assert "function normalizeSecondaryLocation(rawLocation, rawCountry)" in source
    assert "const locationParts = normalizeSecondaryLocation(rawAsset.location, rawAsset.country);" in source
    assert "location: locationParts.displayLocation" in source
    assert "country: locationParts.country" in source
    assert "city: locationParts.city" in source
    assert "location: rawAsset.location + (rawAsset.country ? ', ' + rawAsset.country : '')" not in source


def test_trading_hero_title_is_larger():
    css = read(TRADING_V3_CSS)

    assert ".tv3-hero-title {\n    font-size: 28px;" in css
