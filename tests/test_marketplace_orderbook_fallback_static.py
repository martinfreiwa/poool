import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SERVICE_RS = ROOT / "backend/src/marketplace/service.rs"


def test_market_order_uses_database_when_redis_has_no_best_price():
    source = SERVICE_RS.read_text()
    ok_none_branch = re.search(
        r"Ok\(None\) => \{(?P<body>.*?)\n\s*\}\n\s*Err\(e\) =>",
        source,
        re.S,
    )

    assert ok_none_branch is not None
    body = ok_none_branch.group("body")
    assert "Falling back to Database" in body
    assert "get_best_price_from_db(pool, asset_uuid, side).await?" in body
    assert "return Err(AppError::OrderRejected" not in body

