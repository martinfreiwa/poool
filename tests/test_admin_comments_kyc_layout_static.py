from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def test_admin_comments_empty_state_is_page_scoped_and_centered():
    html = read("frontend/platform/admin/community/comments.html")

    assert "comments-table-container" in html
    assert ".comments-table-container .admin-table-empty" in html
    assert "align-items: center;" in html
    assert "justify-content: center;" in html
    assert ".comments-table-container .admin-table-empty::before" in html
    assert "content: none !important;" in html
    assert "New community comments will appear here for moderation." in html


def test_admin_kyc_topbar_search_stays_compact_on_desktop():
    html = read("frontend/platform/admin/kyc.html")

    assert 'class="admin-body admin-kyc-page dom-ready"' in html
    assert ".admin-kyc-page .admin-topbar" in html
    assert "flex-wrap: nowrap;" in html
    assert ".admin-kyc-page .admin-topbar-right .admin-search" in html
    assert "width: clamp(220px, 26vw, 320px);" in html
    assert "max-width: 320px;" in html
    assert "@media (max-width: 900px)" in html
