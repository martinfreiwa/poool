#!/usr/bin/env python3
"""
Comprehensive Test Suite for POOOL Admin Audit Logs
"""

import json
import subprocess
import sys
import requests

BASE = "http://localhost:8888"
DB = "poool"

passed = 0
failed = 0
errors = []

def psql(sql: str) -> str:
    try:
        return subprocess.check_output(["psql", "-Atc", sql, DB]).decode().strip()
    except Exception:
        return ""

def get_session(email="test@poool.app") -> requests.Session:
    token = psql(f"SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email='{email}') ORDER BY created_at DESC LIMIT 1")
    s = requests.Session()
    s.cookies.set("poool_session", token)
    return s

def check(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  ✅ {name}")
    else:
        failed += 1
        msg = f"  ❌ {name}"
        if detail:
            msg += f"  — {detail}"
        print(msg)
        errors.append(name)

def section(title: str):
    print(f"\n{'─'*60}")
    print(f"  {title}")
    print(f"{'─'*60}")

def setup_mock_audit_log():
    # Insert a dummy audit log to ensure the DB has one with full states
    admin_id = psql("SELECT id FROM users WHERE email='test@poool.app'")
    count = psql("SELECT count(*) FROM audit_logs WHERE action='admin.test_action'")
    if count == "0":
        psql(f"""
        INSERT INTO audit_logs (actor_user_id, action, entity_type, previous_state, new_state, ip_address) 
        VALUES ('{admin_id}', 'admin.test_action', 'system', '{{"setting": "old"}}', '{{"setting": "new"}}', '127.0.0.1')
        """)

def test_api_structure(session):
    section("1. API STRUCTURE & CONTENT")
    r = session.get(f"{BASE}/api/admin/audit-logs")
    check("Audit logs API returns 200", r.status_code == 200)
    data = r.json()
    check("Response has 'logs' field", "logs" in data)
    
    logs = data.get("logs", [])
    if logs:
        log = logs[0]
        required_fields = ["id", "action", "entity_type", "actor_email", "created_at", "previous_state", "new_state", "ip_address"]
        for f in required_fields:
            check(f"Log object has '{f}' field", f in log)
            
        test_log = next((l for l in logs if l["action"] == "admin.test_action"), None)
        check("Found mock test audit log in response", test_log is not None)
        if test_log:
            check("Mock log has correct new_state JSON", test_log["new_state"] == {"setting": "new"})
            check("Mock log has correct previous_state JSON", test_log["previous_state"] == {"setting": "old"})

def test_page_rendering(session):
    section("2. PAGE RENDERING & INTERACTIVITY")
    r = session.get(f"{BASE}/admin/audit-logs.html")
    check("Audit logs page loads (200)", r.status_code == 200)
    html = r.text
    check("admin-audit.js is linked", "admin-audit.js" in html)
    check("Search input exists", "audit-search" in html)
    check("Entity type filter dropdown exists", "filter-entity" in html)
    check("Diff modal exists in DOM", "diff-modal" in html)
    check("Diff modal title exists", "diff-modal-title" in html)

def main():
    print("=" * 60)
    print("  POOOL ADMIN AUDIT LOGS — EXTENDED TEST SUITE")
    print("=" * 60)

    try:
        requests.get(f"{BASE}/", timeout=3)
    except requests.ConnectionError:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    setup_mock_audit_log()
    session = get_session()

    test_api_structure(session)
    test_page_rendering(session)

    print(f"\n{'='*60}")
    total = passed + failed
    print(f"  RESULTS: {passed}/{total} passed, {failed} failed")
    if errors:
        print(f"\n  FAILURES:")
        for e in errors:
            print(f"    • {e}")
    print(f"{'='*60}")
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
