from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_bank_transfer_backend_requires_ack_before_proof():
    source = read("backend/src/payments/routes.rs")

    assert 'const TEST_BANK_DETAILS_EMAILS_ENV: &str = "POOOL_TEST_BANK_DETAILS_EMAILS";' in source
    assert "fn should_use_test_bank_details_with_config" in source
    assert "fn app_env_allows_local_upload_placeholder" in source
    assert "local-test-proof://" in source
    assert "GCS_BUCKET_NAME not configured; using local-only proof placeholder" in source
    assert '"bank_transfer_ack"' in source
    assert "bank_transfer_ack = check(\"bank_transfer_ack\");" in source

    ack_guard = source.index("if !bank_transfer_ack")
    proof_guard = source.index("if proof_url.is_none()")
    assert ack_guard < proof_guard
    assert "You must confirm that the bank transfer reference" in source
    assert "Proof of transfer is required for bank transfer payments." in source


def test_bank_transfer_does_not_allocate_investments_before_approval():
    source = read("backend/src/payments/service.rs")

    checkout_loop = source[
        source.index("// 7. Create order items and reserve asset availability."):
        source.index("// 7.5 Update investment limits")
    ]
    assert 'if payment_method == "wallet"' in checkout_loop
    assert "upsert_active_investment" in checkout_loop
    assert "investment_status" not in checkout_loop
    assert "INSERT INTO investments" not in checkout_loop

    approve_order = source[
        source.index("pub async fn approve_order"):
        source.index("/// Admin: Reject a pending order.")
    ]
    assert "allocate_order_item_investment" in approve_order
    assert "check_and_track_affiliate_commission" in approve_order
    assert "payment_method IN ('bank', 'bank_transfer')" in source

    checkout_referrals = source[
        source.index("let postback_data = if payment_method == \"wallet\""):
        source.index("// 12. Commit everything")
    ]
    assert "check_and_track_affiliate_commission" in checkout_referrals

    lib_source = read("backend/src/lib.rs")
    assert "pending.pending_reserved" in lib_source
    assert "investments plus pending bank orders" in lib_source


def test_checkout_uses_server_fx_and_gates_confirm_button():
    source = read("frontend/platform/checkout.html")

    assert 'id="checkout-confirm-btn"' in source
    assert 'id="checkout-confirm-btn" style="width: 100%; gap: 8px;" disabled' in source
    assert 'name="bank_transfer_ack"' in source
    assert "function displayIdrRate()" in source
    assert "cartData.usd_to_idr_rate" in source
    assert "CurrencyService.fetchExchangeRate" not in source
    assert "btn.disabled = !(requiredDisclosuresAccepted() && bankTransferReady());" in source
    assert 'formData.append(input.name, "on");' in source


def test_cart_replaces_repeat_add_and_shows_terms_errors():
    source = read("backend/src/cart/routes.rs")
    frontend = read("frontend/platform/static/js/cart.js")

    assert "SET tokens_quantity = LEAST($3, $5)" in source
    assert "cart_items.tokens_quantity + $3" not in source
    assert "crate::payments::service::get_usd_to_idr_rate_i64().await" in source
    assert '"usd_to_idr_rate": usd_to_idr_rate' in source
    assert '"usd_to_idr_rate": crate::config::DEFAULT_USD_TO_IDR_RATE_I64' not in source
    assert 'id="cart-validation-error"' in source
    assert "Please accept the Terms and Conditions" in source
    assert "Please read and acknowledge the Key Facts Statement" in source
    assert "totalTokens - availableTokens + newQty" not in frontend
    assert "const newSoldTokens = totalTokens - availableTokens;" in frontend


def test_demo_apartment_fixture_is_repeatable_and_sanitized():
    migration = read("database/125_fix_demo_apartment_checkout_e2e_data.sql")
    model = read("backend/src/assets/models.rs")

    assert "INSERT INTO assets" in migration
    assert "ON CONFLICT (slug) DO UPDATE" in migration
    assert "Demo Apartment 01 - Investment" in migration
    assert "Villa Janoor" not in migration
    assert "annual_yield_bps" in migration
    assert "default_rental_yield_bps" in migration
    assert "default_value_growth_bps" in migration
    assert "clean_display_text" in model
    assert "clean_country_display" in model
    assert '"ID" | "id" => "Indonesia".to_string()' in model
