from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SERVICE_RS = ROOT / "backend/src/marketplace/service.rs"


def test_enabled_trading_is_not_blocked_by_weekend_guard():
    source = SERVICE_RS.read_text()
    validator_start = source.index("fn validate_runtime_settings_for_order")
    validator_end = source.index("pub async fn resolve_asset_id")
    validator = source[validator_start:validator_end]

    assert "settings.trading_enabled" in validator
    assert "settings.maintenance_window" in validator
    assert "weekend_trading" not in validator
    assert "chrono::Weekday::Sat" not in validator
    assert "chrono::Weekday::Sun" not in validator
