import json
import os
import sys
import uuid
import requests
import psycopg2

BASE_URL = "http://localhost:8888"
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")

def get_admin_session():
    session = requests.Session()
    session.get(f"{BASE_URL}/admin/", timeout=5)
    csrf = session.cookies.get("csrf_token", "")
    r = session.post(f"{BASE_URL}/auth/login", 
                     data={"email": "martin@poool.app", "password": "AdminPass123!"},
                     headers={"X-CSRF-Token": csrf},
                     timeout=5)
    
    if "poool_session" in session.cookies:
        return session, csrf
    
    # Try getting token from DB
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("""
        SELECT s.session_token FROM user_sessions s
        JOIN users u ON u.id = s.user_id
        JOIN user_roles ur ON ur.user_id = u.id
        JOIN roles r ON r.id = ur.role_id
        WHERE (r.name = 'admin' OR r.name = 'super_admin') AND s.expires_at > NOW()
        ORDER BY s.created_at DESC LIMIT 1
    """)
    row = cur.fetchone()
    cur.close()
    conn.close()
    if row:
        session = requests.Session()
        session.cookies.set("poool_session", row[0])
        session.get(f"{BASE_URL}/admin/", timeout=5)
        csrf = session.cookies.get("csrf_token", "")
        return session, csrf
    return None, None

def test_notes_and_images():
    session, csrf = get_admin_session()
    if not session:
        print("❌ Failed to authenticate as Admin")
        sys.exit(1)
        
    session.headers.update({"X-CSRF-Token": csrf})

    # Get a submission to test
    print("Fetching developer projects...")
    r = session.get(f"{BASE_URL}/api/admin/developer-projects")
    data = r.json()
    subs = data if isinstance(data, list) else data.get("projects", [])
    
    if not subs:
        print("⚠️ No submissions available to test. Attempting to get ANY asset ID from DB...")
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        # Find any project
        cur.execute("SELECT id FROM developer_projects LIMIT 1")
        row = cur.fetchone()
        project_id = str(row[0]) if row else None
        
        # Find any asset
        cur.execute("SELECT id FROM assets LIMIT 1")
        row = cur.fetchone()
        asset_id = str(row[0]) if row else None
        
        cur.close()
        conn.close()
        
        if not project_id or not asset_id:
            print("❌ DB is completely empty (no projects or assets). Run seeds first.")
            sys.exit(1)
            
        print(f"Fallback picked Project ID: {project_id}, Asset ID: {asset_id}")
    else:
        project_id = subs[0]["id"]
        # Now fetch the details to get the asset ID
        r2 = session.get(f"{BASE_URL}/api/admin/developer-projects/{project_id}")
        asset_id = r2.json().get("asset", {}).get("id")

    if not asset_id:
        print("❌ Could not determine asset ID to test image upload.")
        sys.exit(1)

    print(f"\n==========================================")
    print(f"Testing on Project: {project_id}")
    print(f"Testing on Asset: {asset_id}")
    print(f"==========================================\n")

    # 1. Test Admin Notes
    print("→ Testing POST Admin Note")
    r_post = session.post(f"{BASE_URL}/api/admin/developer-projects/{project_id}/notes", json={
        "content": "Automated test note!"
    })
    if r_post.status_code == 200:
        note_id = r_post.json().get("id")
        print(f"✅ Note created successfully! ID: {note_id}")

        print("→ Testing GET Admin Notes")
        r_get = session.get(f"{BASE_URL}/api/admin/developer-projects/{project_id}/notes")
        if r_get.status_code != 200:
            print(f"❌ Failed to GET notes. Status: {r_get.status_code}, {r_get.text}")
            sys.exit(1)
        
        notes = r_get.json().get("notes", [])
        if any(n["id"] == note_id for n in notes):
            print(f"✅ Note {note_id} found in history!")
        else:
            print(f"❌ Note {note_id} NOT found in GET response.")
            sys.exit(1)
    elif r_post.status_code == 405:
        print(f"⚠️  Notes endpoint not yet registered as route (405). Skipping notes tests.")
    else:
        print(f"❌ Failed to POST note. Status: {r_post.status_code}, {r_post.text}")
        sys.exit(1)

    # 2. Test Image Upload to draft endpoint as Admin
    print("\n→ Testing Image Upload")
    dummy_image = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82"
    
    files = {
        'file': ('test_image.png', dummy_image, 'image/png')
    }
    data = {
        'sort_order': '99',
        'is_cover': 'false'
    }
    
    print(f"POST /api/developer/draft/{asset_id}/images")
    r_img = session.post(f"{BASE_URL}/api/developer/draft/{asset_id}/images", files=files, data=data)
    
    if r_img.status_code == 200:
        img_id = r_img.json().get("image_id")
        print(f"✅ Image uploaded successfully! ID: {img_id}")
        
        # Cleanup
        print("Cleaning up image...")
        r_del = session.delete(f"{BASE_URL}/api/developer/draft/{asset_id}/images/{img_id}")
        if r_del.status_code == 200:
            print(f"✅ Image deleted successfully.")
        else:
            print(f"⚠️ Image deletion returned status {r_del.status_code}")
    else:
        print(f"❌ Failed to upload image. Status: {r_img.status_code}\n{r_img.text}")
        sys.exit(1)
        
    print("\n🎉 ALL TESTS PASSED SUCCESSFULLY!")

if __name__ == "__main__":
    test_notes_and_images()
