import psycopg2
import uuid
from datetime import datetime, timedelta

DB_DSN = "dbname=poool user=martin host=127.0.0.1"

def add_replies():
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        
        # Get an admin user
        cur.execute("""
            SELECT u.id FROM users u 
            JOIN user_roles ur ON ur.user_id = u.id 
            JOIN roles r ON r.id = ur.role_id 
            WHERE r.name IN ('admin', 'super_admin') LIMIT 1
        """)
        admin_row = cur.fetchone()
        if not admin_row:
            print("No admin users found.")
            return
        admin_id = admin_row[0]

        # Get the last ticket
        cur.execute("SELECT id, user_id FROM support_tickets ORDER BY created_at DESC LIMIT 1")
        row = cur.fetchone()
        if not row:
            print("No tickets found to add replies to.")
            return
        
        ticket_id, user_id = row
        now = datetime.now()
        
        # 1. Agent Reply
        cur.execute("""
            INSERT INTO support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """, (str(uuid.uuid4()), ticket_id, admin_id, 'Support Agent Sarah', 'admin', 'reply', 
              "Hello! I'm sorry to hear you're having trouble with the KYC upload. Make sure the file is under 5MB and in JPG or PNG format.", 
              now + timedelta(minutes=5)))
              
        # 2. User Response
        cur.execute("""
            INSERT INTO support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """, (str(uuid.uuid4()), ticket_id, user_id, 'Martin Weber', 'user', 'reply', 
              "Thanks Sarah! I was trying to upload a TIFF file. I'll try with a JPG now.", 
              now + timedelta(minutes=10)))
              
        # 3. Agent Follow-up
        cur.execute("""
            INSERT INTO support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """, (str(uuid.uuid4()), ticket_id, admin_id, 'Support Agent Sarah', 'admin', 'reply', 
              "Great! Let me know if that works for you. I'll keep this ticket open until we confirm it's resolved.", 
              now + timedelta(minutes=15)))

        conn.commit()
        print(f"Added sample conversation replies to ticket {ticket_id}.")
        cur.close()
        conn.close()
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    add_replies()
