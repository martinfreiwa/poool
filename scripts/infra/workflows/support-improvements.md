---
description: Implement comprehensive, industry-standard FinTech customer support features
---

## Support Page & System Improvements Workflow

This workflow tracks the implementation of an enterprise-grade customer support system for the POOOL platform. As a FinTech application, support requires deep context enrichment, strict SLAs, secure messaging, and robust administrative tools to handle complex queries regarding wallets, investments, and KYC securely.

---

### Prerequisites
1. Ensure the Rust backend can be run:
```bash
cd /Users/martin/Projects/poool/backend && cargo run
```
2. PostgreSQL Database must be running.

---

### Phase 1: Enhanced Database Schema (Backend)

Upgrade the database to support a full ticketing system.

**SQL Changes:**
1. **`tickets` table enhancements:**
   - Add `status` enum: `open`, `in_progress`, `waiting_on_customer`, `resolved`, `closed`.
   - Add `priority` enum: `low`, `normal`, `high`, `urgent`.
   - Add `category` enum (matching frontend).
   - Add `metadata` (JSONB): To store context like browser info, last 5 transactions, and KYC status at the time of submission.
   - Add `assigned_admin_id` (UUID, nullable).
   - Add `sla_breach_at` (TIMESTAMP) based on priority.
   - Add `csat_score` (INT 1-5, nullable) and `csat_feedback` (TEXT).

2. **`ticket_messages` table:**
   - `id` (UUID, PK)
   - `ticket_id` (UUID, FK)
   - `sender_id` (UUID) - Can be user or admin.
   - `message` (TEXT)
   - `is_internal_note` (BOOLEAN) - For admins to whisper to each other.
   - `created_at` (TIMESTAMP)

3. **`ticket_attachments` table:**
   - `id` (UUID, PK)
   - `message_id` (UUID, FK)
   - `file_url` (VARCHAR) - GCS bucket path.
   - `file_type` (VARCHAR)
   - `file_size` (INT)

---

### Phase 2: Context-Enriched Ticket Submission (Frontend & Backend)

When a user submits a ticket, gather as much context as possible to reduce back-and-forth.

**Frontend (`frontend/platform/support.html` & `support.js`):**
1. Update `#support-form` to support `multipart/form-data` for file uploads.
2. Intercept form submission and append client-side context (e.g., `navigator.userAgent`, timezone, active console errors).
3. Implement **Dynamic FAQ Deflection**: as the user types the subject, query `GET /api/faq/suggest?q=...` and show top 3 articles above the submit button.

**Backend (`backend/src/support/routes.rs`):**
1. **Endpoint `POST /support/ticket` logic:**
   - Parse multipart form data.
   - Fetch backend context: User's KYC status (`pending`, `verified`, etc.), their last 3 wallet transactions, and current balances.
   - Combine client context and backend context into a JSON object and save it to the `metadata` column of the new ticket.
   - Calculate `sla_breach_at`: e.g., `Urgent` = Now + 2 hours, `Normal` = Now + 24 hours.
   - Iteratively upload files to Google Cloud Storage (GCS) using `storage/service.rs`. Save GCS URIs to `ticket_attachments`.
   - Return `201 Created` with the ticket ID.

---

### Phase 3: Ticket Thread & Secure Attachments

Provide a chat-like interface for secure back-and-forth communication.

**Frontend:**
1. Update the `#tickets-list` so clicking a ticket card opens a detailed thread view (either a slide-in modal or a dedicated route like `/support/ticket/{id}`).
2. Display message history with distinction between User messages (sent, right side) and Admin messages (received, left side).
3. Render image attachments as secure thumbnails and PDFs as styled links.

**Backend:**
1. **Endpoint `GET /support/ticket/{id}`:**
   - Fetch the ticket, assert `ticket.user_id == current_user.id` (Security).
   - Fetch all `ticket_messages` and related `ticket_attachments` ordered by `created_at ASC`.
   - For attachments, generate short-lived **signed URLs** from GCS so files remain private and secure (prevent public bucket access).
2. **Endpoint `POST /support/ticket/{id}/reply`:**
   - Add a new message to `ticket_messages`.
   - Update `ticket.updated_at`.
   - If ticket `status` was `waiting_on_customer`, change it to `open`.

---

### Phase 4: Admin Hub & SLA Tracking (Backend)

Admins need tools to effectively resolve issues, and the system must explicitly monitor SLAs.

**Admin Backend Hub:**
1. Create `GET /admin/support/tickets` with filtering (by status, priority, SLA breached).
2. Add functionality to "Assign to me".
3. Allow admins to add `is_internal_note = true` messages that the user cannot see.
4. Allow admins to use "**Canned Responses**" (Macros) for common issues (e.g., "Your KYC is rejected because the ID is blurry").

**SLA Tracking (Rust Background Task):**
1. In `main.rs`, spawn a background Tokio task (`tokio::spawn(async move { ... })`).
2. Run a loop every 5 minutes: `SELECT id FROM tickets WHERE status IN ('open', 'in_progress') AND sla_breach_at < NOW() AND sla_alert_sent = false`.
3. If breached tickets are found, send a webhook payload to a predefined Slack/Discord "#support-alerts" channel or email internal admins. Update `sla_alert_sent = true`.

---

### Phase 5: Resolution & CSAT (Customer Satisfaction)

Measure support quality continuously.

**Backend & Frontend Logic:**
1. When an admin marks a ticket as `resolved`, trigger an email/notification to the user: "Your ticket #XYZ has been resolved. How did we do?"
2. In the ticket detail view on the frontend, if `status == 'resolved'`, show a 5-star rating UI and a feedback text area.
3. Submit to `POST /support/ticket/{id}/rate`:
   - Validate rating is 1-5.
   - Update `csat_score` and `csat_feedback` on the ticket.
   - Change ticket status to `closed`.

---

### Phase 6: Implementation & Testing Plan

Use the following commands to execute tests step-by-step as you build:

// turbo
```bash
# Verify schema migrations
cd /Users/martin/Projects/poool && psql -U postgres -d poool -c "\d tickets"
```

// turbo
```bash
# Run support backend unit tests
cd /Users/martin/Projects/poool/backend && cargo test support::tests
```

// turbo
```bash
# Run platform E2E tests for support flows
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | grep -i "support"
```
