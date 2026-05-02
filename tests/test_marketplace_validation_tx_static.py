from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION_RS = ROOT / "backend/src/marketplace/validation.rs"


def test_transactional_validation_does_not_swallow_database_errors():
    source = VALIDATION_RS.read_text()

    for fn_name in [
        "check_buyer_balance",
        "check_seller_tokens",
        "check_concentration_limit_tx",
    ]:
        start = source.index(f"pub async fn {fn_name}")
        next_fn = source.find("\npub async fn ", start + 1)
        body = source[start : next_fn if next_fn != -1 else len(source)]

        assert ".ok()" not in body
        assert ".map_err(AppError::Database)" in body


def test_concentration_limit_locks_rows_before_aggregate():
    source = VALIDATION_RS.read_text()
    start = source.index("pub async fn check_concentration_limit_tx")
    next_fn = source.find("\npub async fn ", start + 1)
    body = source[start : next_fn if next_fn != -1 else len(source)]

    assert "FROM (" in body
    assert "FOR UPDATE" in body
    assert "locked_investments" in body
    assert "SUM(tokens_owned), 0)::int4\n         FROM investments" not in body

