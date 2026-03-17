#!/usr/bin/env python3
import requests
import psycopg2
import os
import sys

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
TEST_EMAIL = os.environ.get("TEST_EMAIL", "test@poool.app")

def get_session():
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email=%s) ORDER BY created_at DESC LIMIT 1", (TEST_EMAIL,))
    row = cur.fetchone()
    if not row: return None
    token = row[0]
    cur.close()
    conn.close()
    session = requests.Session()
    session.cookies.set("poool_session", token)
    return session

def test_chart_functionality():
    session = get_session()
    if not session:
        print("❌ Could not get session")
        return False

    print("\n--- Testing Portfolio Chart Components ---")
    resp = session.get(f"{BASE_URL}/portfolio")
    if resp.status_code != 200:
        print(f"❌ Portfolio page failed with status {resp.status_code}")
        return False

    html = resp.text
    
    # Check for new CSS and JS
    checks = [
        ("portfolio-chart.css", "portfolio-chart.css link"),
        ("portfolio-chart.js", "portfolio-chart.js script"),
        ('id="portfolio-chart-tabs"', "Chart tabs container"),
        ('activeTab === \'twelveMonths\'', "Active tab state (Alpine)"),
        ('id="portfolio-chart-filter"', "Chart filter button"),
        ('id="portfolio-chart-section"', "Chart section"),
        ('class="chart-bar-week"', "Chart bars class"),
        ('class="chart-trend-line"', "Trend line class"),
        ('class="chart-title-text"', "Chart title text container"),
        ('class="chart-title-percentage"', "Chart title percentage container"),
    ]

    passed = 0
    for snippet, desc in checks:
        if snippet in html:
            print(f"  ✅ {desc}")
            passed += 1
        else:
            print(f"  ❌ {desc} NOT FOUND")

    # Check for the dynamic data injection
    if 'id="server-portfolio-json"' in html:
        print("  ✅ Server-side JSON injection for chart")
        passed += 1
    else:
        print("  ❌ Server-side JSON injection NOT FOUND")

    print(f"\nSummary: {passed}/{len(checks)+1} checks passed.")
    return passed == len(checks) + 1

if __name__ == "__main__":
    if test_chart_functionality():
        sys.exit(0)
    else:
        sys.exit(1)
