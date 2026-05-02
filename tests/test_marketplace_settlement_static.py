from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SETTLEMENT_RS = ROOT / "backend/src/marketplace/settlement.rs"


def test_settlement_upserts_seller_cash_wallet_before_crediting_proceeds():
    source = SETTLEMENT_RS.read_text(encoding="utf-8")

    assert "async fn credit_seller_cash_wallet" in source
    assert "INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)" in source
    assert "ON CONFLICT (user_id, wallet_type, currency)" in source
    assert "balance_cents = wallets.balance_cents + EXCLUDED.balance_cents" in source
    assert "credit_seller_cash_wallet(&mut tx, event.seller_user_id, seller_proceeds).await?" in source
    assert "Seller wallet not uniquely matched" not in source


def test_settlement_creates_canonical_platform_fee_wallet_if_seed_is_missing():
    source = SETTLEMENT_RS.read_text(encoding="utf-8")

    assert "async fn credit_platform_fee_wallet" in source
    assert "WHERE wallet_type = 'platform_fee' AND currency = 'USD'" in source
    assert "email IN ('admin@poool.app', 'support@traffic-creator.com')" in source
    assert "VALUES ($1, 'platform_fee', 'USD', 0)" in source
    assert "credit_platform_fee_wallet(&mut tx, total_fee_cents).await?" in source
    assert "Platform fee wallet not uniquely matched" not in source
