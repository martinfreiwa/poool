from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_affiliate_settings_uses_dedicated_api_and_csrf():
    js = read("frontend/platform/static/js/affiliate-settings.js")
    routes = read("backend/src/rewards/mod.rs")

    assert 'fetch("/api/affiliate/settings"' in js
    assert '"/api/affiliate/onboarding/submit"' not in js
    assert '"X-CSRF-Token"' in js
    assert '"/api/affiliate/settings"' in routes
    assert "get_affiliate_settings_handler" in routes
    assert "save_affiliate_settings_handler" in routes


def test_affiliate_settings_save_button_submits_form_once():
    html = read("frontend/platform/affiliate-settings.html")
    topbar = read("frontend/platform/components/investor-topbar.html")

    assert 'extra_js=[\'affiliate-settings\']' in html
    assert html.count("/static/js/affiliate-settings.js") == 0
    assert 'id="save-settings-btn"' in topbar
    assert 'type="submit"' in topbar
    assert 'form="affiliate-settings-form"' in topbar


def test_affiliate_settings_masks_tax_id_and_writes_audit_log():
    service = read("backend/src/rewards/service.rs")

    assert "fn mask_tax_id" in service
    assert "tax_id_masked" in service
    assert "AFFILIATE_SETTINGS_UPDATED" in service
    assert "previous_state" in service
    assert "new_state" in service


def test_affiliate_tax_ids_are_encrypted_at_rest():
    service = read("backend/src/rewards/service.rs")
    routes = read("backend/src/rewards/routes.rs")
    migration = read("database/092_affiliate_tax_id_encryption.sql")
    plaintext_drop_migration = read("database/154_affiliate_tax_id_drop_plaintext.sql")

    assert "TAX_ID_ENCRYPTION_KEY" in service
    assert "tax_id:v1" in service
    assert "encrypt_tax_id_for_storage" in service
    assert "tax_id_encrypted = $2" in service
    assert "tax_id_encrypted" in routes
    assert "tax_id_last4" in routes
    assert "ADD COLUMN IF NOT EXISTS tax_id_encrypted TEXT" in migration
    assert "ADD COLUMN IF NOT EXISTS tax_id_last4 VARCHAR(4)" in migration
    assert "DROP COLUMN tax_id" in plaintext_drop_migration
    assert "affiliate_legacy_tax_id_cleared" in plaintext_drop_migration
