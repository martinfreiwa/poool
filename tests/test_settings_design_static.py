from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SETTINGS_HTML = ROOT / "frontend/platform/settings.html"
SETTINGS_CSS = ROOT / "frontend/platform/static/css/settings.css"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_settings_save_buttons_use_design_system_primary_button():
    template = read(SETTINGS_HTML)

    for button_id in (
        "btn-save-social",
        "btn-save-developer-profile",
        "btn-save-developer-links",
    ):
        assert f'id="{button_id}" class="ds-btn ds-btn--primary"' in template

    assert 'id="btn-save-social" class="ds-btn ds-btn--primary settings-btn--primary"' not in template
    assert 'id="btn-save-developer-profile" class="ds-btn ds-btn--primary settings-btn--primary"' not in template
    assert 'id="btn-save-developer-links" class="ds-btn ds-btn--primary settings-btn--primary"' not in template


def test_developer_settings_does_not_override_design_system_primary_buttons():
    css = read(SETTINGS_CSS)

    assert ".developer-settings-main .settings-edit-footer .ds-btn--primary" not in css
    assert "#form-developer-identity .settings-edit-footer .settings-btn--primary" not in css
    assert "#form-developer-links .settings-edit-footer .settings-btn--primary" not in css


def test_info_learning_action_cards_center_content():
    css = read(SETTINGS_CSS)

    assert ".developer-settings-main .action-grid-card {\n    display: flex !important;" in css
    assert "align-items: center !important;" in css
    assert "justify-content: center !important;" in css
    assert "text-align: center !important;" in css
    assert ".developer-settings-main .action-grid-card .action-text {\n    display: flex;" in css
    assert "width: 100%;" in css
