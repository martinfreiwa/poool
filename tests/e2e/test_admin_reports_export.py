"""
POOOL E2E Tests — Admin Reports Export functionality
=====================================================
Tests that CSV reports are successfully downloaded from the Admin Dashboard.
"""

import pytest
import os
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

@pytest.mark.admin
@pytest.mark.financial
def test_admin_reports_csv_download(admin_page, tmp_path):
    """
    Verifies that the CSV download buttons on the Admin Reports page 
    trigger a file download successfully.
    """
    page, tracker = admin_page
    
    # 1. Page load
    tracker.navigate_and_check(f"{BASE_URL}/admin/reports.html")
    tracker.assert_page_loaded()
    
    # 2. Get all report cards that have a CSV format and a download button
    # Based on the JS, buttons have IDs like 'dl-btn-{report.id}'
    report_buttons = page.locator("button[id^='dl-btn-']").all()
    
    print(f"\n📊 Found {len(report_buttons)} reports to download.")
    assert report_buttons, "No report download buttons found"

    btn = report_buttons[0]
    btn_id = btn.get_attribute("id")
    report_id = btn_id.replace("dl-btn-", "")
    print(f"📥 Downloading report: {report_id}...")

    with page.expect_download(timeout=10000) as download_info:
        btn.click()

    download = download_info.value
    save_path = tmp_path / download.suggested_filename
    download.save_as(str(save_path))
    assert save_path.stat().st_size > 0
    print(f"✅ Saved to: {save_path}")

    # 4. Final health check
    tracker.assert_no_critical_errors()
