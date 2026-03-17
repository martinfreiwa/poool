#!/usr/bin/env python3
"""
POOOL Admin Dashboard – Security Middleware Tests
=================================================
Verifies that regular users and unauthenticated users cannot access
admin-protected routes and APIs.

Usage:
  python3 tests/admin/test_admin_security.py
"""

import os
import sys
import psycopg2
import requests

BASE_URL = "http://localhost:8888"
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")
REQUEST_TIMEOUT = 5

class SecurityLogger:
    def __init__(self):
        self.passed = 0
        self.failed = 0

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ [PASS] {msg}")

    def fail(self, msg):
        self.failed += 1
        print(f"  ❌ [FAIL] {msg}")

def get_standard_user_session():
    # Attempt to fetch an existing standard investor session from DB
    session = requests.Session()
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            JOIN user_roles ur ON ur.user_id = u.id
            JOIN roles r ON r.id = ur.role_id
            WHERE r.name = 'investor' 
              AND s.expires_at > NOW()
              AND NOT EXISTS (
                  SELECT 1 FROM user_roles ur2 
                  JOIN roles r2 ON r2.id = ur2.role_id
                  WHERE ur2.user_id = u.id AND r2.name IN ('admin', 'super_admin')
              )
            ORDER BY s.created_at DESC LIMIT 1
        """)
        row = cur.fetchone()
        cur.close()
        conn.close()
        if row:
            session.cookies.set("poool_session", row[0])
            r = session.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
            if r.status_code == 200:
                return session
    except: pass

    # If no existing session found, fallback by trying known users or creating one (simplified by registering)
    try:
        import uuid
        test_email = f"standard_{uuid.uuid4().hex[:8]}@poool.app"
        r = session.post(f"{BASE_URL}/auth/register", data={
            "email": test_email,
            "password": "Password123!",
            "confirm_password": "Password123!"
        }, timeout=REQUEST_TIMEOUT)
        
        # After register, normally there is an email verification step
        # So we might not be logged in. In poool, registration might log them in directly
        # or we might need to manually verify them in DB.
        
        # We will instead grab a known db user WITHOUT admin role and login manually,
        # but registering does usually create an investor. Let's force an update if needed.
        if "poool_session" in session.cookies:
            return session
            
        # fallback to manually finding ANY non-admin user and inserting a session via DB?
        # That's what we originally tried. Let's just create a session for an existing non-admin user.
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT u.id FROM users u
            WHERE NOT EXISTS (
                SELECT 1 FROM user_roles ur 
                JOIN roles r ON r.id = ur.role_id 
                WHERE ur.user_id = u.id AND r.name IN ('admin', 'super_admin')
            )
            LIMIT 1
        """)
        uid = cur.fetchone()
        if uid:
            token = "test_security_token_" + uuid.uuid4().hex
            cur.execute("""
                INSERT INTO user_sessions (session_token, user_id, expires_at)
                VALUES (%s, %s, NOW() + INTERVAL '1 day')
            """, (token, uid[0]))
            conn.commit()
            session.cookies.set("poool_session", token)
            return session
        cur.close()
        conn.close()
    except Exception as e:
        print("Failed to setup a standard user session fallback:", e)
        
    return requests.Session()

def test_admin_routes(session, logger, is_unauth=False):
    prefix = "Unauthenticated User" if is_unauth else "Standard Investor"
    print(f"\n================== TESTING: {prefix} ==================")
    
    protected_apis = [
        "/api/admin/stats/overview",
        "/api/admin/users",
        "/api/admin/treasury",
        "/api/admin/assets",
        "/api/admin/deposits",
        "/api/admin/orders",
        "/api/admin/rewards",
        "/api/admin/support",
        "/api/admin/kyc",
        "/api/admin/system",
        "/api/admin/settings",
        "/api/admin/audit-logs",
        "/api/admin/emails"
    ]
    
    for api in protected_apis:
        r = session.get(f"{BASE_URL}{api}", timeout=REQUEST_TIMEOUT)
        if r.status_code in [401, 403]:
            logger.ok(f"Access correctly denied ({r.status_code}) -> {api}")
        else:
            logger.fail(f"Access allowed or incorrect error ({r.status_code}) -> {api}")

    print(f"\n=======================================================")

def main():
    print("POOOL Admin Dashboard - Security Tests")
    logger = SecurityLogger()
    
    # 1. Unauthenticated test
    unauth_session = requests.Session()
    test_admin_routes(unauth_session, logger, is_unauth=True)
    
    # 2. Standard user test
    std_session = get_standard_user_session()
    # Check if we successfully got a non-admin session
    r = std_session.get(f"{BASE_URL}/api/me")
    if r.status_code == 200 and "admin" not in r.json().get("role", "").lower():
        test_admin_routes(std_session, logger, is_unauth=False)
    else:
        print("Could not find a valid standard investor session to test with.")
        
    print(f"\n  Security Summary: {logger.passed} Passed, {logger.failed} Failed")
    if logger.failed > 0:
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()
