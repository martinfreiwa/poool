from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIDEBAR = ROOT / "frontend/platform/components/sidebar.html"
PROFILE_DROPDOWN = ROOT / "frontend/platform/static/js/profile-dropdown.js"
ROUTES_HELPER = ROOT / "backend/src/common/routes_helper.rs"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_sidebar_account_card_renders_user_before_client_hydration():
    sidebar = read(SIDEBAR)
    routes_helper = read(ROUTES_HELPER)

    assert 'class="sidebar__account-name">{{ user_display_name }}</span>' in sidebar
    assert 'class="sidebar__account-email">{{ user.email }}</span>' in sidebar
    assert 'class="profile-account-name">{{ user_display_name }}</span>' in sidebar
    assert 'user_display_name => user_display_name' in routes_helper
    assert '"user_display_name".to_string()' in routes_helper


def test_profile_dropdown_does_not_paint_profile_type_into_main_email():
    script = read(PROFILE_DROPDOWN)
    load_saved_profile = script.split("function loadSavedProfile()", 1)[1]

    assert 'document.querySelector("#account-email")' not in load_saved_profile
    assert "mainEmail.textContent = type.textContent" not in load_saved_profile
    assert "name.textContent.trim()" in load_saved_profile
