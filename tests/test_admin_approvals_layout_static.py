from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_approvals_new_request_form_is_compact():
    html = (ROOT / "frontend/platform/admin/approvals.html").read_text()

    card_start = html.index('class="new-request-card"')
    tabs_start = html.index("<!-- Filter Tabs -->", card_start)
    card = html[card_start:tabs_start]

    assert 'id="new-request-form"' in card
    assert 'class="new-request-grid"' in card
    assert 'class="new-request-payload-row"' in card
    assert 'id="req-action-type"' in card
    assert 'id="req-entity-type"' in card
    assert 'id="req-entity-id"' in card
    assert 'id="req-payload" rows="1"' in card
    assert "Submit for Approval" in card
    assert 'style="margin-bottom: 12px"' not in card
    assert 'style="' not in card

    for css in (
        ".new-request-payload-row",
        "grid-template-columns: minmax(0, 1fr) auto",
        "height: 38px",
        "white-space: nowrap",
    ):
        assert css in html
