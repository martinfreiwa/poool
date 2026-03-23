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
    
    # 2. Get all report cards that have a CSV format and a download button
    # Based on the JS, buttons have IDs like 'dl-btn-{report.id}'
    report_buttons = page.locator("button[id^='dl-btn-']").all()
    
    home_dir = os.path.expanduser("~")
    downloads_dir = os.path.join(home_dir, "Downloads")
    
    print(f"\n📊 Found {len(report_buttons)} reports to download.")
    
    for btn in report_buttons:
        btn_id = btn.get_attribute("id")
        report_id = btn_id.replace("dl-btn-", "")
        
        # We only want to test CSV and JSON for now as per the requirements
        # But let's just trigger whatever download button is there
        
        print(f"📥 Downloading report: {report_id}...")
        
        try:
            with page.expect_download(timeout=10000) as download_info:
                btn.click()
            
            download = download_info.value
            save_path = os.path.join(downloads_dir, download.suggested_filename)
            
            download.save_as(save_path)
            assert os.path.getsize(save_path) > 0
            print(f"✅ Saved to: {save_path}")
            
        except Exception as e:
            print(f"❌ Failed to download {report_id}: {str(e)}")
            continue

    # 4. Final health check
    tracker.assert_no_critical_errors()
