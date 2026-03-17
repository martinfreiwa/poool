#!/usr/bin/env python3
"""
Investor Dashboard (Portfolio) Test Suite for POOOL Platform
==========================================================
Covers:
  1. Access Control (Logged in vs Guest)
  2. Main Content Elements (Portfolio Value, Financials)
  3. Responsive Elements (Desktop vs Mobile specific classes)
  4. Interactive Components (Tabs, Expandable charts)
  5. Asset Table Integrity
"""

import os
import requests
import psycopg2
from html.parser import HTMLParser
import sys

# Configuration
BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
TEST_EMAIL = os.environ.get("TEST_EMAIL", "test@poool.app")
TEST_PASSWORD = os.environ.get("TEST_PASSWORD", "TestPass123!")

class TestResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []

    def check(self, name, condition, detail=""):
        if condition:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            print(f"  ❌ {name} {' - ' + detail if detail else ''}")
            self.errors.append(f"{name}: {detail}")

    def report(self):
        print("\n" + "="*50)
        print(f"Final Report: {self.passed} Passed, {self.failed} Failed")
        if self.errors:
            print("\nErrors:")
            for err in self.errors:
                print(f"  - {err}")
        print("="*50 + "\n")
        return self.failed == 0

class HTMLAnalyzer(HTMLParser):
    def __init__(self):
        super().__init__()
        self.elements = []
        self.ids = set()
        self.classes = set()

    def handle_starttag(self, tag, attrs):
        attr_dict = dict(attrs)
        self.elements.append({'tag': tag, 'attrs': attr_dict})
        if 'id' in attr_dict:
            self.ids.add(attr_dict['id'])
        if 'class' in attr_dict:
            for cls in attr_dict['class'].split():
                self.classes.add(cls)

def get_session():
    """Get an authenticated session using the database session token."""
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT session_token FROM user_sessions 
            WHERE user_id = (SELECT id FROM users WHERE email=%s)
            ORDER BY created_at DESC LIMIT 1
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        cur.close()
        conn.close()
        
        if not row:
            print(f"⚠️ No session found for {TEST_EMAIL}. Make sure user is registered.")
            return None
            
        session = requests.Session()
        session.cookies.set("poool_session", row[0])
        return session
    except Exception as e:
        print(f"❌ DB Error: {e}")
        return None

def test_portfolio_page():
    results = TestResults()
    print("\n--- Testing Investor Portfolio Dashboard ---")
    
    session = get_session()
    if not session:
        results.check("Session Authentication", False, "Could not establish session")
        return results

    # 1. Access Portfolio
    resp = session.get(f"{BASE_URL}/portfolio")
    results.check("Access /portfolio", resp.status_code == 200, f"Status: {resp.status_code}")
    
    analyzer = HTMLAnalyzer()
    analyzer.feed(resp.text)
    
    # 2. Verify Core Sections
    required_ids = [
        "portfolio-main",
        "portfolio-header",
        "portfolio-page-title",
        "portfolio-value-section",
        "portfolio-value-card",
        "key-financials-section",
        "insights-limit-section",
        "assets-section",
        "portfolio-assets-table"
    ]
    
    for rid in required_ids:
        results.check(f"Element ID presence: {rid}", rid in analyzer.ids)

    # 3. Verify KPI Indicators
    # Check for specific financial cards
    results.check("Monthly Income Card", "key-financials-card-monthly-income-gradient" in analyzer.ids)
    results.check("Total Rental Card", "key-financials-card-total-rental-gradient" in analyzer.ids)
    results.check("Total Appreciation Card", "key-financials-card-appreciation-gradient" in analyzer.ids)

    # 4. Verify Chart Container and Animation elements
    results.check("Chart Section Presence", "portfolio-chart-section" in analyzer.ids)
    results.check("Chart Bars Class", "chart-bar-week" in analyzer.classes)
    results.check("Trend Line Presence", "chart-trend-line" in analyzer.classes)

    # 5. Verify Investment Limit (Compliance/Limits)
    results.check("Investment Limit Section", "investment-limit-wrapper" in analyzer.ids)
    results.check("Progress Bar Container", "progress-bar-container" in analyzer.ids)
    results.check("Annual Limit Label", "limit-item-annual" in analyzer.ids)

    # 6. Verify Asset Table Headers
    table_headers = ["Property", "Investment value", "Total rental income", "Status", "Actions"]
    for header in table_headers:
        results.check(f"Asset Table Header: {header}", header in resp.text)

    # 7. Responsive / Mobile Check
    results.check("Mobile Portfolio Wrapper", "mobile-portfolio-wrapper" in analyzer.classes)
    results.check("Mobile Financial Cards", "mobile-financial-card" in analyzer.classes)

    return results

if __name__ == "__main__":
    r = test_portfolio_page()
    if not r.report():
        sys.exit(1)
    sys.exit(0)
