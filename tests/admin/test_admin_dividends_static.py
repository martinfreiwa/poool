from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_dividend_routes_require_financial_permissions():
    treasury = read("backend/src/admin/treasury.rs")

    for handler in [
        "api_admin_dividends_calculate",
        "api_admin_dividends_process",
        "api_admin_dividends_list",
        "api_admin_dividends_create_distribution",
        "api_admin_dividends_distribution_detail",
        "api_admin_dividends_approve_distribution",
        "api_admin_dividends_execute_distribution",
        "api_admin_dividends_cancel_distribution",
    ]:
        segment = treasury[treasury.index(f"pub async fn {handler}") :]
        segment = segment[: segment.index(") -> Result<axum::response::Response, ApiError>") + 2000]
        assert "require_permission(&state.db" in segment

    assert '"financials.payout.draft"' in treasury
    assert '"financials.payout.approve"' in treasury


def test_dividend_service_enforces_maker_checker_and_audits_execution():
    service = read("backend/src/dividends/service.rs")

    assert "Creator cannot approve their own dividend distribution" in service
    assert "Creator cannot execute their own dividend distribution" in service
    assert "Distribution is missing creator metadata and cannot be approved" in service
    assert "approved_by.ok_or_else" in service
    assert "distributed_by = $1" in service
    assert "'dividend_distribution.executed'" in service
    assert "executor_user_id" in service


def test_dividend_calculation_persistence_errors_are_not_swallowed():
    service = read("backend/src/dividends/service.rs")

    assert "DB payout insert error" in service
    assert "Duplicate payout row for distribution" in service
    assert "Failed to update dividend distribution holder counts" in service
    assert "let _ = sqlx::query" not in service


def test_dividend_frontend_does_not_render_api_rows_with_inner_html_or_inline_handlers():
    js = read("frontend/platform/static/js/admin-dividends.js")
    html = read("frontend/platform/admin/dividends.html")

    assert "innerHTML" not in js
    assert "onclick=" not in js
    assert "onclick=" not in html
    assert "document.createElement" in js
    assert "textContent" in js
    assert "handleHistoryAction" in js
    assert "data-action" in js


def test_dividend_frontend_does_not_auto_approve_and_execute_phase9():
    js = read("frontend/platform/static/js/admin-dividends.js")

    assert "approveResp" not in js
    assert "execResp" not in js
    assert "showPhase9Queued" in js
    assert "Submit for Approval" in js
    assert "falling back to legacy" not in js


def test_dividend_copy_matches_current_offchain_snapshot_source_and_migration_exists():
    html = read("frontend/platform/admin/dividends.html")
    migration = read("database/090_dividend_distribution_execution_audit.sql")

    assert "platform investment ledger" in html
    assert "ERC-1155 smart contract" not in html
    assert "ADD COLUMN IF NOT EXISTS distributed_by" in migration
