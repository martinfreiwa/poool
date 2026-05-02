from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LEGAL_ENHANCEMENTS = ROOT / "frontend/platform/static/js/legal-enhancements.js"


def test_legal_enhancements_does_not_inject_terms_reaccept_banner():
    source = LEGAL_ENHANCEMENTS.read_text()

    removed_banner_markers = [
        "terms-reaccept-banner",
        "terms-accept-btn",
        "Our Terms have been updated",
        "/api/user/legal-status",
        "/api/user/legal-accept",
    ]

    for marker in removed_banner_markers:
        assert marker not in source


def test_legal_enhancements_keeps_legal_footer_and_cookie_consent():
    source = LEGAL_ENHANCEMENTS.read_text()

    assert "injectPlatformFooter();" in source
    assert "showCookieConsent();" in source
    assert "Terms &amp; Conditions" in source
