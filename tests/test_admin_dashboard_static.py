from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_admin_dashboard_links_support_queue_to_registered_route():
    html = read("frontend/platform/admin/index.html")
    js = read("frontend/platform/static/js/admin-dashboard.js")
    routes = read("backend/src/admin/mod.rs")

    assert 'href="/admin/support.html"' in html
    assert 'href="/admin/support-tickets.html"' not in html
    assert 'href: "/admin/support.html"' in js
    assert 'support-tickets.html' not in js
    assert '.route("/admin/support.html", get(page_admin_generic))' in routes


def test_admin_dashboard_pending_deposits_include_ids_for_bulk_actions():
    rust = read("backend/src/admin/dashboard.rs")
    js = read("frontend/platform/static/js/admin-dashboard.js")

    assert "SELECT d.id::text, u.email, d.amount_cents" in rust
    assert '"id": id' in rust
    assert "if (deposit.id)" in js
    assert "/api/admin/deposits/${encodeURIComponent(id)}/${kind}" in js


def test_admin_dashboard_preserves_deposit_range_badge_on_refresh():
    html = read("frontend/platform/admin/index.html")
    js = read("frontend/platform/static/js/admin-dashboard.js")

    assert 'id="kpi-deposits-label">Deposits <span class="admin-kpi-range-tag" id="kpi-deposits-range-tag">30d</span>' in html
    assert 'setTextById("kpi-deposits-range-tag", rangeTag)' in js
    assert 'setTextById("kpi-deposits-label"' not in js


def test_admin_dashboard_pending_deposits_loading_row_spans_all_columns():
    html = read("frontend/platform/admin/index.html")

    pending_section = html.split('id="pending-deposits-table"', 1)[1].split("</tbody>", 1)[0]
    assert 'colspan="6"' in pending_section
    assert 'colspan="4"' not in pending_section


def test_admin_dashboard_unknown_activity_does_not_pollute_user_filter():
    js = read("frontend/platform/static/js/admin-dashboard.js")

    activity_category = js.split("function activityCategory(act)", 1)[1].split(
        "// ---- CSV export",
        1,
    )[0]
    assert 'return "other";' in activity_category
    assert 'return "user";\n}' not in activity_category


def test_admin_health_ok_halo_is_scoped_to_positioned_topbar_dot():
    css = read("frontend/platform/static/css/admin.css")
    extras = read("frontend/platform/static/css/admin-dashboard-extras.css")
    html = read("frontend/platform/admin/index.html")
    js = read("frontend/platform/static/js/admin-dashboard.js")

    assert ".admin-health-dot.admin-health-dot--ok::after" in css
    assert ".admin-health-dot--ok::after" not in css.replace(
        ".admin-health-dot.admin-health-dot--ok::after",
        "",
    )
    assert ".admin-health-tile-dot" in extras
    assert ".admin-health-tile-dot--ok" in extras
    assert 'class="admin-health-tile-dot admin-health-tile-dot--ok"' in html
    assert 'dot.classList.add("admin-health-tile-dot--ok")' in js
    assert 'class="admin-health-tile-dot admin-health-dot--ok"' not in html
    assert 'dot.classList.add("admin-health-dot--ok")' not in js
