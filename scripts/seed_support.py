import psycopg2
import uuid
from datetime import datetime, timedelta

DB_DSN = "dbname=poool user=martin host=127.0.0.1"


def seed_support():
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()

    # Get some users
    cur.execute("SELECT id, email FROM users LIMIT 10")
    users = cur.fetchall()
    
    if len(users) < 2:
        print("Not enough users to seed support data. Please run seed_data.sql first or create users.")
        # Attempt to create a test user
        user_id = str(uuid.uuid4())
        cur.execute("INSERT INTO users (id, email, password_hash) VALUES (%s, 'test_support@poool.app', 'hash') ON CONFLICT DO NOTHING", (user_id,))
        conn.commit()
        cur.execute("SELECT id, email FROM users LIMIT 10")
        users = cur.fetchall()

    ADMIN_ID = users[0][0]
    USER1_ID = users[1][0] if len(users) > 1 else users[0][0]
    USER2_ID = users[2][0] if len(users) > 2 else users[0][0]
    USER3_ID = users[3][0] if len(users) > 3 else users[0][0]
    USER4_ID = users[4][0] if len(users) > 4 else users[0][0]

    print("Cleaning existing sample support data...")
    cur.execute("DELETE FROM support_ticket_replies")
    cur.execute("DELETE FROM support_tickets")
    conn.commit()

    def create_ticket(user_id, subject, message, status, priority, messages=[]):
        ticket_id = str(uuid.uuid4())
        created_at = datetime.now() - timedelta(days=2)
        
        # Get author name
        cur.execute("SELECT email FROM users WHERE id = %s", (user_id,))
        row = cur.fetchone()
        author_name = row[0] if row else "Unknown User"

        cur.execute("""
            INSERT INTO support_tickets (id, user_id, subject, message, status, priority, created_at, updated_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """, (ticket_id, user_id, subject, message, status, priority, created_at, created_at))

        # Initial message in replies
        cur.execute("""
            INSERT INTO support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
        """, (str(uuid.uuid4()), ticket_id, user_id, author_name, 'user', 'initial', message, created_at))

        # Subsequent replies
        last_time = created_at
        for rep_author_id, role, content in messages:
            last_time += timedelta(hours=2)
            cur.execute("SELECT email FROM users WHERE id = %s", (rep_author_id,))
            rep_row = cur.fetchone()
            rep_author_name = rep_row[0] if rep_row else "Unknown Agent"
            
            cur.execute("""
                INSERT INTO support_ticket_replies (id, ticket_id, author_id, author_name, author_role, type, content, created_at)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            """, (str(uuid.uuid4()), ticket_id, rep_author_id, rep_author_name, role, 'reply', content, last_time))
            
        return ticket_id

    print("Seeding tickets...")

    # Ticket 1: Verification issue
    create_ticket(
        USER1_ID, 
        "Cannot verify my identity", 
        "I tried uploading my passport but it keeps failing with a generic error. I've tried 3 times now.",
        "open", 
        "normal"
    )

    # Ticket 2: Withdrawal delay
    create_ticket(
        USER2_ID,
        "Withdrawal pending for 3 days",
        "My withdrawal to my bank account is still pending. Usually it takes 24h. Can you please check?",
        "in_progress",
        "high",
        messages=[
            (ADMIN_ID, 'admin', "Hello, we are checking this with our payment provider. There's a slight delay in processing SEPA transfers today."),
            (USER2_ID, 'user', "Okay, thank you for the update. Do you have an ETA? I need the funds by Friday."),
            (ADMIN_ID, 'admin', "We expect it to be cleared by tomorrow morning. We've prioritized your request.")
        ]
    )

    # Ticket 3: Limits
    create_ticket(
        USER3_ID,
        "How do I increase my investment limit?",
        "I want to invest more than $10,000. How can I increase my limit? Is there a VIP program?",
        "resolved",
        "low",
        messages=[
            (ADMIN_ID, 'admin', "You need to upgrade to the 'Pro' tier by completing Advanced KYC. This involves providing a Proof of Wealth document."),
            (USER3_ID, 'user', "Got it, I will upload the documents tonight. Thanks!")
        ]
    )

    # Ticket 4: Missing transaction
    create_ticket(
        USER4_ID,
        "URGENT: Transaction not appearing",
        "I sent $5,000 via bank transfer but my balance still shows zero after 48 hours. Here is my reference: POOOL-X-9982. Please help!",
        "open",
        "urgent"
    )

    conn.commit()
    cur.close()
    conn.close()
    print("Support data seeded successfully!")

if __name__ == "__main__":
    seed_support()
