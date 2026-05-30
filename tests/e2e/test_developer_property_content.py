"""POOOL E2E — Developer: /developer/property-content

Coverage:
  * Page loads clean (no console errors, no 5xx).
  * Yield / Capital Appreciation / Investor Share / Occupancy / Total Return
    inputs are ALL blank by default (C-6 regression guard — confirm no
    value="10" or value="85" sneaks back in).
  * Total Return input is readonly (typing into it doesn't change value).
  * Live-computed Total Return: typing 5 into Rental Yield + 3 into Capital
    Appreciation → Total Return updates to "8".
  * Image upload zone renders.
  * Save & Exit (without a draft_id) redirects to /developer/submissions.

Run:
    pytest tests/e2e/test_developer_property_content.py -v
    HEADED=1 pytest tests/e2e/test_developer_property_content.py -v
"""
import os
import re
import pytest
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 15_000


def _goto_property_content(page: Page, tracker):
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/property-content",
        timeout=TIMEOUT,
        wait_until="domcontentloaded",
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)
    # Wait for the recomputeTotalReturn listeners to attach.
    page.wait_for_selector("#rental-yield", state="visible", timeout=TIMEOUT)


@pytest.mark.developer
@pytest.mark.smoke
def test_loads_clean(developer_page):
    """Page loads with no console errors, no failed network requests."""
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    expect(page).to_have_title(re.compile(r"Property Content", re.IGNORECASE))
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404])


@pytest.mark.developer
def test_yield_inputs_blank_by_default(developer_page):
    """C-6 regression guard: no fallback default values on percent inputs."""
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    blank_fields = [
        "#rental-yield",
        "#capital-appreciation",
        "#investor-share",
        "#occupancy-rate",
        "#total-return",
    ]
    for sel in blank_fields:
        # input_value() returns whatever the form actually carries — empty string
        # means C-6 is correctly fixed (no fallback "10"/"85" default).
        value = page.locator(sel).input_value()
        assert value == "", f"Expected blank default on {sel}, got {value!r}"


@pytest.mark.developer
def test_total_return_is_readonly(developer_page):
    """Total Return must be readonly — typing into it should not change the value."""
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    total_return = page.locator("#total-return")
    expect(total_return).to_have_attribute("readonly", "")

    # Attempt to type — readonly inputs may still accept keys depending on impl,
    # but the value must remain empty. We verify via input_value() after a fill.
    # Note: page.fill() on a readonly input raises in Playwright strict mode;
    # use type() and verify nothing changes.
    initial = total_return.input_value()
    try:
        total_return.click()
        page.keyboard.type("99")
    except Exception:
        pass  # readonly may block keyboard input entirely
    final = total_return.input_value()
    assert final == initial, f"Total Return should be readonly; was {initial!r}, now {final!r}"


@pytest.mark.developer
def test_total_return_live_computed(developer_page):
    """Rental Yield 5 + Capital Appreciation 3 → Total Return shows 8."""
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    page.locator("#rental-yield").fill("5")
    page.locator("#rental-yield").dispatch_event("input")
    page.locator("#capital-appreciation").fill("3")
    page.locator("#capital-appreciation").dispatch_event("input")
    page.wait_for_timeout(150)

    # parsePercentField + recomputeTotalReturn → "8" (toFixed(2) strips .00).
    total = page.locator("#total-return").input_value()
    assert total == "8", f"Expected Total Return = '8', got {total!r}"


@pytest.mark.developer
def test_image_upload_zone_renders(developer_page):
    """The dropzone + hidden file input for image upload is in the DOM."""
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    expect(page.locator("#file-upload-area-media")).to_be_visible(timeout=TIMEOUT)
    # The actual <input type="file"> is hidden but must be attached.
    expect(page.locator("#file-input-media")).to_be_attached()
    expect(page.locator("#image-gallery")).to_be_attached()


@pytest.mark.developer
def test_save_and_exit_redirects_to_submissions(developer_page):
    """
    Save & Exit without a draft_id navigates straight to /developer/submissions
    (no API call when there's no draft to persist).
    """
    page, tracker, _ = developer_page
    _goto_property_content(page, tracker)

    page.locator("#save-exit-btn").click()
    page.wait_for_url("**/developer/submissions**", timeout=TIMEOUT)
    assert "/developer/submissions" in page.url


@pytest.mark.developer
@pytest.mark.mobile
def test_loads_on_mobile(developer_page):
    """Mobile viewport: form renders cleanly."""
    page, tracker, _ = developer_page
    page.set_viewport_size({"width": 375, "height": 812})
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/property-content", timeout=TIMEOUT
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)
    expect(page.locator("#asset-title")).to_be_visible(timeout=TIMEOUT)
    tracker.assert_no_critical_errors()
