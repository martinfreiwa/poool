from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_settings_topbar_search_is_compact():
    html = (ROOT / "frontend/platform/admin/settings.html").read_text(encoding="utf-8")
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text(encoding="utf-8")

    assert 'class="admin-body admin-settings-page dom-ready"' in html
    assert ".admin-settings-page .admin-topbar-right" in css
    assert ".admin-settings-page .admin-topbar-right .admin-search" in css
    assert ".admin-settings-page .admin-notification-btn" in css

    settings_css_start = css.index(".admin-settings-page .admin-topbar-right")
    settings_css_end = css.index("/* ===========================", settings_css_start)
    settings_css = css[settings_css_start:settings_css_end]

    assert "height: 32px" in settings_css
    assert "width: 320px" in settings_css
    assert "max-width: 320px" in settings_css
    assert "flex-wrap: nowrap" in settings_css
