"""
POOOL E2E — Developer: Add Asset & Submit
==========================================
Tests every input, dropdown, upload, and button in the asset-creation wizard:

  Step 1  /developer/add-asset             — asset-type card selection
  Step 2  /developer/application-form      — property details + financials
  Step 3  /developer/document-upload-step3 — file uploads per section
  Step 4  /developer/property-content      — descriptions, media, financials
  Submit  /developer/submission-success    — final landing page

Key fix: PooolDropdown wraps native <select>s. The form JS reads from
_pooolDropdown.getValue(), NOT from native select.value. All dropdown
interactions go through set_dropdown() which calls _pooolDropdown.setValue()
via JS, identical to how setDropdownVal() works in the app.

Run:
    pytest tests/e2e/test_developer_add_asset.py -v
    HEADED=1 pytest tests/e2e/test_developer_add_asset.py -v   # watch it
"""

import re
import os
import tempfile
import pytest
from pathlib import Path
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 15_000


# ─── Helpers ─────────────────────────────────────────────────────────────────

def set_dropdown(page: Page, select_id: str, value: str):
    """
    Set a PooolDropdown via _pooolDropdown.setValue().
    Falls back to native select + change event if not yet initialised.
    """
    page.evaluate(
        """({ id, val }) => {
            const el = document.getElementById(id);
            if (!el) throw new Error('element not found: ' + id);
            const wrapper = el.closest('[data-dropdown]');
            if (wrapper && wrapper._pooolDropdown) {
                wrapper._pooolDropdown.setValue(val);
            } else {
                el.value = val;
                el.dispatchEvent(new Event('change', { bubbles: true }));
            }
        }""",
        {"id": select_id, "val": value},
    )


def get_dropdown_val(page: Page, select_id: str) -> str:
    return page.evaluate(
        """id => {
            const el = document.getElementById(id);
            const wrapper = el?.closest('[data-dropdown]');
            if (wrapper?._pooolDropdown) return wrapper._pooolDropdown.getValue() || '';
            return el?.value || '';
        }""",
        select_id,
    )


def fill_currency(page: Page, selector: str, value: str):
    """Currency inputs have comma-formatting — fill then dispatch input event."""
    el = page.locator(selector)
    el.click()
    el.fill(value)
    el.dispatch_event("input")


def make_temp_pdf(name: str = "test-doc.pdf") -> str:
    """Create a minimal valid PDF in /tmp."""
    tmp = Path(tempfile.mkdtemp()) / name
    tmp.write_bytes(
        b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        b"3 0 obj<</Type/Page/MediaBox[0 0 612 792]>>endobj\n"
        b"xref\n0 4\n0000000000 65535 f\ntrailer<</Size 4/Root 1 0 R>>\nstartxref\n9\n%%EOF"
    )
    return str(tmp)


def make_temp_jpg(name: str = "test-img.jpg") -> str:
    """Create a minimal 1×1 JPEG in /tmp."""
    tmp = Path(tempfile.mkdtemp()) / name
    tmp.write_bytes(bytes([
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
        0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09,
        0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
        0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
        0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
        0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00,
        0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
        0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
        0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0xFB, 0xD7,
        0xFF, 0xD9,
    ]))
    return str(tmp)


def wait_ready(page: Page, extra_ms: int = 300):
    page.wait_for_load_state("domcontentloaded")
    page.wait_for_timeout(extra_ms)


# ─── Step wrappers ────────────────────────────────────────────────────────────

def go_add_asset(page, tracker):
    tracker.navigate_and_check(f"{BASE_URL}/developer/add-asset", timeout=TIMEOUT)
    wait_ready(page)


def select_real_estate(page: Page):
    card = page.locator("#asset-type-card-real-estate")
    expect(card).to_be_visible(timeout=TIMEOUT)
    card.click()
    expect(card).to_have_class(re.compile(r"selected"), timeout=5_000)
    page.locator("#add-asset-next-btn").click()
    page.wait_for_url("**/developer/application-form**", timeout=TIMEOUT)


def fill_application_form(page: Page):
    wait_ready(page, 600)   # PooolDropdown needs ~100 ms to init
    page.fill("#property-name", "E2E Villa Seminyak")
    set_dropdown(page, "property-type", "villa")
    set_dropdown(page, "area", "seminyak")
    page.fill("#address", "Jl. Kayu Aya 99")
    page.fill("#city", "Denpasar")
    page.fill("#country", "Indonesia")
    set_dropdown(page, "lease-type", "leasehold")
    page.fill("#lease-term", "25")
    page.fill("#land-size", "400")
    page.fill("#building-size", "250")
    page.fill("#bedrooms", "3")
    page.fill("#bathrooms", "3")
    set_dropdown(page, "status", "ready")
    page.fill("#year-built", "2023")
    fill_currency(page, "#purchase-price", "500000")
    fill_currency(page, "#minimum-share-price", "500")


def submit_application_form(page: Page):
    with page.expect_response(
        lambda r: "/api/developer/draft" in r.url and r.request.method in ("POST", "PUT"),
        timeout=TIMEOUT,
    ) as resp_info:
        page.locator("#form-next-btn").click()
    resp = resp_info.value
    assert resp.ok, f"Draft save failed: {resp.status} — {resp.text()}"
    page.wait_for_url("**/developer/document-upload**", timeout=TIMEOUT)


def upload_document(page: Page, file_input_id: str, file_path: str):
    """Upload to a hidden-file-input and wait for simulated progress to finish."""
    page.locator(f"#{file_input_id}").set_input_files(file_path)
    page.locator(".file-upload-item").first.wait_for(state="visible", timeout=8_000)
    page.wait_for_function(
        """() => {
            const fills = document.querySelectorAll('.ds-progress__fill');
            return fills.length > 0 &&
                   [...fills].every(f => parseFloat(f.style.width) >= 100);
        }""",
        timeout=8_000,
    )


def step3_upload_and_advance(page: Page):
    wait_ready(page, 400)
    upload_document(page, "file-input-1", make_temp_pdf())
    page.locator("#form-next-btn").click()
    page.wait_for_url("**/developer/property-content**", timeout=TIMEOUT)


def fill_property_content(page: Page):
    wait_ready(page, 400)
    page.fill("#asset-title", "E2E Villa Seminyak")
    page.fill("#short-description", "A 3-bed e2e test villa in Seminyak.")
    page.fill("#full-description", "E2E test property — not for production.")
    page.fill("#maps-link", "https://maps.google.com/?q=-8.69,115.16")
    page.fill("#location-description", "Steps from Seminyak beach.")
    page.fill("#youtube-link", "https://youtube.com/watch?v=e2e_test")
    # Upload image and wait for it to appear in the gallery (not a blind timeout)
    with page.expect_response(
        lambda r: "/api/developer/draft" in r.url and "/images" in r.url and r.request.method == "POST",
        timeout=TIMEOUT,
    ):
        page.locator("#file-input-media").set_input_files(make_temp_jpg())
    page.locator(".uploaded-image-item").first.wait_for(state="visible", timeout=TIMEOUT)
    page.fill("#rental-yield", "10")
    page.fill("#capital-appreciation", "8")
    page.fill("#investor-share", "70")
    page.fill("#occupancy-rate", "85")


def submit_property_content(page: Page):
    # Wait for both the draft PUT and the submit POST to complete
    with page.expect_response(
        lambda r: "/api/developer/draft" in r.url and "/submit" in r.url and r.request.method == "POST",
        timeout=TIMEOUT,
    ):
        page.locator("#form-next-btn").click()
    page.wait_for_url("**/developer/submission-success**", timeout=TIMEOUT)


# ═══════════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_add_asset_page_loads(developer_page):
    """Smoke: /developer/add-asset renders for a developer user."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("#asset-type-card-real-estate")).to_be_visible()
    expect(page.locator("#add-asset-next-btn")).to_be_visible()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_asset_type_card_selection(developer_page):
    """Real Estate card selectable; coming-soon cards stay unselected."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)

    real_estate = page.locator("#asset-type-card-real-estate")
    commercial = page.locator("#asset-type-card-commercial-property")
    real_estate.click()
    expect(real_estate).to_have_class(re.compile(r"selected"))
    expect(commercial).not_to_have_class(re.compile(r"selected"))


@pytest.mark.developer
def test_application_form_inputs_accept_values(developer_page):
    """Every text input + all PooolDropdowns + currency fields accept values."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    wait_ready(page, 600)

    text_cases = [
        ("#property-name", "Test Villa"),
        ("#address", "Jl. Test 123"),
        ("#city", "Denpasar"),
        ("#country", "Indonesia"),
        ("#lease-term", "30"),
        ("#land-size", "500"),
        ("#building-size", "300"),
        ("#bedrooms", "4"),
        ("#bathrooms", "3"),
        ("#year-built", "2022"),
    ]
    for sel, val in text_cases:
        page.fill(sel, val)
        expect(page.locator(sel)).to_have_value(val)

    dropdown_cases = [
        ("property-type", "villa"),
        ("area", "canggu"),
        ("lease-type", "freehold"),
        ("status", "ready"),
    ]
    for sid, val in dropdown_cases:
        set_dropdown(page, sid, val)
        got = get_dropdown_val(page, sid)
        assert got == val, f"Dropdown #{sid}: expected '{val}', got '{got}'"

    fill_currency(page, "#purchase-price", "750000")
    raw = page.locator("#purchase-price").input_value().replace(",", "")
    assert raw == "750000", f"purchase-price: expected '750000', got '{raw}'"

    fill_currency(page, "#minimum-share-price", "500")
    raw2 = page.locator("#minimum-share-price").input_value().replace(",", "")
    assert raw2 == "500", f"minimum-share-price: expected '500', got '{raw2}'"


@pytest.mark.developer
def test_application_form_validation_errors(developer_page):
    """Next Step with empty form stays on page and shows field errors."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    wait_ready(page, 600)

    page.locator("#form-next-btn").click()
    page.wait_for_timeout(600)

    assert "/application-form" in page.url, f"Should stay on form, got: {page.url}"
    err_count = page.locator(".field-error-msg").count()
    assert err_count > 0, "Expected validation errors, found none"


@pytest.mark.developer
def test_back_button_returns_to_add_asset(developer_page):
    """Back button on application form navigates back to /developer/add-asset."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    wait_ready(page, 400)
    page.locator("#form-back-btn").click()
    page.wait_for_url("**/developer/add-asset**", timeout=TIMEOUT)


@pytest.mark.developer
def test_save_and_exit_creates_draft(developer_page):
    """Save & Exit POSTs a draft and redirects to /developer/submissions."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)

    with page.expect_response(
        lambda r: "/api/developer/draft" in r.url,
        timeout=TIMEOUT,
    ) as resp_info:
        page.locator("#save-exit-btn").click()

    resp = resp_info.value
    assert resp.ok, f"Save & Exit API failed: {resp.status}"
    page.wait_for_url("**/developer/submissions**", timeout=TIMEOUT)


@pytest.mark.developer
def test_document_upload_shows_in_list(developer_page):
    """PDF upload on step 3 appears in file list with name visible."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)
    submit_application_form(page)
    wait_ready(page, 400)

    pdf_path = make_temp_pdf("upload-test.pdf")
    upload_document(page, "file-input-1", pdf_path)

    expect(page.locator(".file-name")).to_contain_text("upload-test.pdf")
    expect(page.locator(".file-delete-btn").first).to_be_visible()


@pytest.mark.developer
def test_document_upload_delete(developer_page):
    """Uploaded file is removed from DOM when delete button clicked."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)
    submit_application_form(page)
    wait_ready(page, 400)

    upload_document(page, "file-input-1", make_temp_pdf("del-test.pdf"))
    file_item = page.locator(".file-upload-item").first
    expect(file_item).to_be_visible()
    page.locator(".file-delete-btn").first.click()
    expect(file_item).not_to_be_visible(timeout=3_000)


@pytest.mark.developer
def test_property_content_inputs_accept_values(developer_page):
    """Every field on the property-content form accepts its expected value."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)
    submit_application_form(page)
    step3_upload_and_advance(page)
    wait_ready(page, 400)

    cases = [
        ("#asset-title", "My E2E Villa"),
        ("#short-description", "Short desc"),
        ("#full-description", "Full desc text"),
        ("#maps-link", "https://maps.google.com/?q=-8,115"),
        ("#location-description", "Near the beach"),
        ("#youtube-link", "https://youtube.com/watch?v=abc"),
        ("#rental-yield", "10"),
        ("#capital-appreciation", "8"),
        ("#investor-share", "70"),
        ("#occupancy-rate", "85"),
    ]
    for sel, val in cases:
        page.fill(sel, val)
        expect(page.locator(sel)).to_have_value(val)


@pytest.mark.developer
def test_property_content_image_upload(developer_page):
    """Image upload on step 4 populates the gallery."""
    page, tracker, _ = developer_page
    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)
    submit_application_form(page)
    step3_upload_and_advance(page)
    wait_ready(page, 400)

    page.locator("#file-input-media").set_input_files(make_temp_jpg("gallery.jpg"))
    page.wait_for_timeout(800)

    gallery = page.locator("#image-gallery")
    expect(gallery).to_be_visible()
    assert gallery.inner_html().strip() != "", "Image gallery is empty after upload"


@pytest.mark.developer
def test_full_submission_flow(developer_page):
    """
    Full happy-path: select asset type → fill all wizard steps →
    upload document + image → submit → land on success page.
    """
    page, tracker, _ = developer_page

    go_add_asset(page, tracker)
    select_real_estate(page)
    fill_application_form(page)
    submit_application_form(page)
    step3_upload_and_advance(page)
    fill_property_content(page)
    submit_property_content(page)

    assert "/developer/submission-success" in page.url, (
        f"Expected submission-success, got: {page.url}"
    )
    expect(page.locator("body")).to_be_visible()
    tracker.full_health_check()
