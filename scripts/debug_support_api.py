import requests
import psycopg2
import sys

BASE_URL = "http://localhost:8888"
DB_DSN = "dbname=poool user=martin host=127.0.0.1"

def test_api():
    session = requests.Session()
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            JOIN user_roles ur ON ur.user_id = u.id
            JOIN roles r ON r.id = ur.role_id
            WHERE r.name IN ('admin', 'super_admin') AND s.expires_at > NOW()
            ORDER BY s.created_at DESC LIMIT 1
        """)
        row = cur.fetchone()
        if not row:
            print("No admin session found in DB")
            return
        token = row[0]
        session.cookies.set("poool_session", token)

        # Get the latest ticket ID
        cur.execute("SELECT id FROM support_tickets ORDER BY created_at DESC LIMIT 1")
        row = cur.fetchone()
        if not row:
            print("No tickets found")
            return
        tid = row[0]
        
        url = f"{BASE_URL}/api/admin/support/{tid}"
        print(f"Testing GET {url}")
        r = session.get(url)
        print(f"Status: {r.status_code}")
        if r.status_code != 200:
            print(f"Response: {r.text}")
        else:
            print("Success!")
            print(r.json())
            
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    test_api()
