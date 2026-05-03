from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_marketplace_page_passes_sidebar_user_context():
    routes = (ROOT / "backend/src/assets/routes.rs").read_text()

    marketplace_start = routes.index("pub async fn page_marketplace")
    property_start = routes.index("pub async fn page_property", marketplace_start)
    marketplace_handler = routes[marketplace_start:property_start]

    assert "get_current_user(&jar, &state.db).await" in marketplace_handler
    assert "sidebar_user_display_name(&user.email)" in marketplace_handler
    assert "user => user" in marketplace_handler
    assert "user_display_name => user_display_name" in marketplace_handler
    assert "is_developer => false" in marketplace_handler
    assert "is_authenticated(&jar, &state.db)" not in marketplace_handler


def test_commodities_page_passes_sidebar_user_context():
    routes = (ROOT / "backend/src/assets/routes.rs").read_text()

    commodities_start = routes.index("pub async fn page_commodities_marketplace")
    tab_start = routes.index("pub async fn api_commodities_tab", commodities_start)
    commodities_handler = routes[commodities_start:tab_start]

    assert "sidebar_user_display_name(&user.email)" in commodities_handler
    assert "user => user" in commodities_handler
    assert "user_display_name => user_display_name" in commodities_handler
    assert "is_developer => false" in commodities_handler

