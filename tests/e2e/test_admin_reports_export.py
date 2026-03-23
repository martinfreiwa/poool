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
def test_admin_reports_csv_download(admin_page):
    """
    Verifies that the CSV download buttons on the Admin Reports page 
    trigger a file download successfully.
    """
    page, tracker = admin_page
    
    # 1. Page load
    tracker.navigate_and_check(f"{BASE_URL}/admin/reports.html")
    tracker.assert_page_loaded()
    
    # Wait for the report grids to render
    expect(page.locator("#grid-financial").first).to_be_visible(timeout=5000)

    # 2. Grab the download button
    # The ID for the download button maps to the report ID 'monthly-financial'
    download_btn = page.locator("#dl-btn-monthly-financial")
    expect(download_btn).to_be_visible()

    # 3. Wait for the download event when clicking the button
    with page.expect_download() as download_info:
        download_btn.click()
        
    download = download_info.value
    
    # Check that the filename ends with .csv and it downloaded successfully
    assert download.suggested_filename.endswith(".csv")
    assert download.suggested_filename.startswith("poool_monthly-financial")
    
    # Ensure the downloaded file is not empty by checking its size
    download_path = download.path()
    assert os.path.getsize(download_path) > 0

    # 4. Final health check
    tracker.assert_no_critical_errors()
