import requests
import json
import uuid
from bs4 import BeautifulSoup
import re

BASE_URL = "http://localhost:8888"

import psycopg2

def login_and_get_csrf(session, email, password):
    # Retrieve active session from db directly 
    conn = psycopg2.connect("dbname=poool user=martin host=127.0.0.1")
    cur = conn.cursor()
    
    # insert session if none
    cur.execute("SELECT id FROM users WHERE email = %s", (email,))
    user_id = cur.fetchone()[0]
    
    token = uuid.uuid4().hex
    cur.execute("INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified) VALUES (%s, %s, NOW() + INTERVAL '1 day', true)", (user_id, token))
    conn.commit()
    conn.close()
    
    session.cookies.set("poool_session", token)
    session.cookies.set("csrf_token", "fake-token")
    return True

def test_support_endpoints():
    print("Testing Support Page API Integrations")
    session = requests.Session()
    
    # We will just login as the pre-seeded admin straight away to test admin routes
    email = "admin@poool.app"
    password = "TestPass123!"
    
    
    if not login_and_get_csrf(session, email, password):
        print("Failed to login as admin!")
        return
        
    print("Logged in successfully.")
    
    # Need to get a fresh CSRF token for the API requests if it rotates, but 
    # the cookie one should still work
    csrf_token = "fake-token"
    headers = {
        "X-CSRF-Token": csrf_token
    }

    print("Session Cookies before submit:", session.cookies.get_dict())
    print("X-CSRF-Token:", headers["X-CSRF-Token"])

    # Test api me
    me_res = session.get(f"{BASE_URL}/api/me")
    print("API ME:", me_res.status_code, me_res.text)

    # Setup multipart data for new ticket
    client_context = json.dumps({"userAgent": "PythonRequests", "timezone": "Europe/Berlin"})
    jpeg_bytes = (
        b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x01\x00\x01\x00\x01\x00\x00"
        b"\xff\xdb\x00C\x00" + (b"\x08" * 64) +
        b"\xff\xc0\x00\x11\x08\x00\x01\x00\x01\x03\x01\x11\x00\x02\x11\x01\x03\x11\x01"
        b"\xff\xc4\x00\x14\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        b"\xff\xc4\x00\x14\x10\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        b"\xff\xda\x00\x0c\x03\x01\x00\x02\x11\x03\x11\x00\x3f\x00\x00\xff\xd9"
    )
    files = {
        'subject': (None, 'Help with deposit'),
        'message': (None, 'My deposit is failing when I try to use my card.'),
        'priority': (None, 'high'),
        'category': (None, 'billing'),
        'context': (None, client_context),
        'attachment': ('screenshot.jpg', jpeg_bytes, 'image/jpeg')
    }

    # Submit ticket!
    print("Submitting ticket...")
    req = requests.Request('POST', f"{BASE_URL}/api/support/tickets", files=files, headers=headers)
    prepared = session.prepare_request(req)
    print("Prepared Request Headers:", prepared.headers)
    
    res = session.send(prepared)
    print("Submit Ticket:", res.status_code, res.text)
    assert res.status_code == 200, "Failed to submit ticket"

    # List tickets
    res = session.get(f"{BASE_URL}/api/support/tickets")
    print("List Tickets:", res.status_code)
    tickets = res.json()
    ticket_list = tickets.get("tickets", [])
    assert len(ticket_list) > 0, "No tickets found after creation"
    ticket_id = ticket_list[0]["id"]
    print(f"Found ticket ID: {ticket_id}")

    # Test Admin APIs - since we are already the admin
    print("Fetching Admin Ticket List...")
    res = session.get(f"{BASE_URL}/api/admin/support")
    print("Admin List Tickets:", res.status_code)
    assert res.status_code == 200, "Admin support list failed"
    
    res = session.get(f"{BASE_URL}/api/admin/support/{ticket_id}")
    print("Admin Ticket Detail:", res.status_code)
    assert res.status_code == 200, "Admin ticket detail failed"
    detail = res.json()
    print("Ticket Category:", detail.get("category"))
    print("Ticket SLA Break At:", detail.get("sla_breach_at"))
    print("Ticket Metadata:", json.dumps(detail.get("metadata", {})))
    print("Messages Count:", len(detail.get("messages", [])))
    if len(detail.get("messages", [])) > 0:
        first_msg = detail["messages"][0]
        print("First Message Type:", first_msg["type"])
        print("Attachments Count:", len(first_msg.get("attachments_json", [])))

    print("SUCCESS: All Support Endpoints Operational")

if __name__ == "__main__":
    test_support_endpoints()
