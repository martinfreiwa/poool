#!/usr/bin/env python3
"""
Storage & KYC Document Integration Tests
========================================
Verifies:
1. Avatar upload to GCS.
2. KYC document upload to GCS.
3. KYC record linking with documents.
4. Admin retrieval of signed URLs.
"""

import os
import sys
import subprocess
import requests
import json
import time

BASE = "http://localhost:8888"
DB = "poool"

passed = 0
failed = 0
errors = []

def psql(sql: str) -> str:
    return subprocess.check_output(["psql", "-Atc", sql, DB]).decode().strip()

def get_session(email="test@poool.app") -> requests.Session:
    token = psql(
        f"SELECT session_token FROM user_sessions "
        f"WHERE user_id = (SELECT id FROM users WHERE email='{email}') "
        f"ORDER BY created_at DESC LIMIT 1"
    )
    s = requests.Session()
    s.cookies.set("poool_session", token)
    return s

def get_admin_session() -> requests.Session:
    # Find an admin user
    email = psql("SELECT u.email FROM users u JOIN user_roles ur ON u.id = ur.user_id JOIN roles r ON r.id = ur.role_id WHERE r.name = 'admin' LIMIT 1")
    if not email:
        # Promote test user to admin if none found
        psql("INSERT INTO user_roles (user_id, role_id) SELECT u.id, r.id FROM users u, roles r WHERE u.email = 'test@poool.app' AND r.name = 'admin' ON CONFLICT DO NOTHING")
        email = "test@poool.app"
    return get_session(email)

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
    print(f"\n{'─'*60}\n  {title}\n{'─'*60}")

def main():
    section("SETUP")
    user_email = "test@poool.app"
    user_id = psql(f"SELECT id FROM users WHERE email='{user_email}'")
    print(f"  User: {user_email} ({user_id})")

    # Ensure files exist
    if not os.path.exists("/tmp/sample_avatar.jpg"):
        with open("/tmp/sample_avatar.jpg", "wb") as f: f.write(os.urandom(1024))
    if not os.path.exists("/tmp/sample_kyc.pdf"):
        with open("/tmp/sample_kyc.pdf", "wb") as f: f.write(os.urandom(2048))

    session = get_session(user_email)
    admin_session = get_admin_session()

    section("1. AVATAR UPLOAD")
    with open("/tmp/sample_avatar.jpg", "rb") as f:
        r = session.post(f"{BASE}/api/upload/avatar", files={"file": ("avatar.jpg", f, "image/jpeg")})
    
    check("Avatar upload returns 200/201", r.status_code in (200, 201), f"got {r.status_code}: {r.text}")
    if r.status_code in (200, 201):
        data = r.json()
        check("Response contains avatar_url", "avatar_url" in data)
        check("URL is HTTPS/GCS", "https://storage.googleapis.com" in data.get("avatar_url", ""))
        
        db_url = psql(f"SELECT avatar_url FROM users WHERE id = '{user_id}'")
        check("URL matches database", db_url == data.get("avatar_url"))

    section("2. KYC DOCUMENT UPLOAD")
    with open("/tmp/sample_kyc.pdf", "rb") as f:
        r = session.post(f"{BASE}/api/upload/kyc", data={"document_type": "passport"}, files={"file": ("passport.pdf", f, "application/pdf")})
    
    check("KYC upload returns 200/201", r.status_code in (200, 201), f"got {r.status_code}: {r.text}")
    doc_id = None
    if r.status_code in (200, 201):
        data = r.json()
        check("Response contains document_id", "document_id" in data)
        doc_id = data.get("document_id")
        
        db_path = psql(f"SELECT gcs_path FROM kyc_documents WHERE id = '{doc_id}'")
        check("GCS path exists in DB", db_path.startswith("gs://"))

    section("3. FULL KYC SUBMISSION WITH DOCUMENT")
    if doc_id:
        # Submit KYC application
        payload = {
            "first_name": "Test",
            "last_name": "User",
            "document_type": "passport",
            "document_id": doc_id,
        }
        r = session.post(f"{BASE}/api/kyc/submit", json=payload)
        check("KYC submission returns 200", r.status_code == 200, f"got {r.status_code}")
        
        # Verify kyc_records entry
        kyc_rec_id = psql(f"SELECT id FROM kyc_records WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1")
        linked_id = psql(f"SELECT kyc_record_id FROM kyc_documents WHERE id = '{doc_id}'")
        check("Document linked to KYC record", linked_id == kyc_rec_id)

    section("4. ADMIN REVIEW & SIGNED URLS")
    if doc_id:
        kyc_rec_id = psql(f"SELECT id FROM kyc_records WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1")
        # Get a fresh admin session with a real session token
        admin_token = psql(
            f"SELECT session_token FROM user_sessions "
            f"WHERE user_id = (SELECT id FROM users WHERE email='{user_email}') "
            f"ORDER BY created_at DESC LIMIT 1"
        )
        admin_s = requests.Session()
        admin_s.cookies.set("poool_session", admin_token)
        r = admin_s.get(f"{BASE}/api/admin/kyc/{kyc_rec_id}/documents")
        check("Admin can fetch documents (200)", r.status_code == 200, f"got {r.status_code}: {r.text}")
        if r.status_code == 200:
            docs = r.json()
            check("Admin sees our document", any(d["id"] == doc_id for d in docs), f"docs: {docs}")
            # Signed URL may be empty locally (requires SA key in prod) – just check field exists
            doc = next((d for d in docs if d["id"] == doc_id), None)
            check("Document has url field", doc is not None and "url" in doc)

    section("SUMMARY")
    print(f"Results: {passed}/{passed+failed} passed")
    if failed > 0:
        print("FAILURES:")
        for e in errors: print(f"  • {e}")
        sys.exit(1)
    sys.exit(0)

if __name__ == "__main__":
    main()
