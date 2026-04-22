---
description: Implement all settings page features (My Details, Preferences, Security, More) with comprehensive testing
---

# Settings Page – Full Implementation Workflow

// turbo-all

> This workflow makes the Settings page fully functional with real backend API endpoints,
> database persistence, form validation, user feedback, and comprehensive testing.

## Architecture Overview

```
Frontend: frontend/platform/settings.html
          frontend/platform/static/css/settings.css
          frontend/platform/static/js/settings.js (NEW)

Backend:  backend/src/settings/mod.rs (NEW)
          backend/src/settings/models.rs (NEW)
          backend/src/settings/service.rs (NEW)
          backend/src/settings/routes.rs (NEW)

Database: user_profiles  (existing – stores name, phone, country, etc.)
          user_settings   (existing – stores language, notifications; extend with currency + timezone)
          users           (existing – stores email, password_hash, avatar_url)

Tests:    tests/test_settings.py (NEW – standalone settings test suite)
          tests/test_platform.py (MODIFY – expand existing test_settings function)
```

**API Pattern**: Form submission → JSON response with `{ success: bool, message: string }` → JavaScript toast notification

---

## Phase 1: Database Schema Updates

> Do this first so the backend can reference the new columns immediately.

### Step 1.1 – Create migration file

Create `database/003_settings_extensions.sql`:

```sql
-- ═══════════════════════════════════════════════════════════════════
-- Migration 003: Settings Extensions
-- Adds currency and timezone columns to user_settings for
-- the Preferences tab on the Settings page.
-- ═══════════════════════════════════════════════════════════════════

-- Currency preference (ISO 4217 code)
ALTER TABLE user_settings
  ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- Timezone (IANA timezone identifier, e.g. 'America/Los_Angeles')
ALTER TABLE user_settings
  ADD COLUMN IF NOT EXISTS timezone VARCHAR(64) NOT NULL DEFAULT 'UTC';

-- Comment for documentation
COMMENT ON COLUMN user_settings.currency IS 'ISO 4217 currency code (USD, EUR, GBP, SGD, IDR)';
COMMENT ON COLUMN user_settings.timezone IS 'IANA timezone identifier (e.g. America/New_York)';
```

### Step 1.2 – Run migration

```bash
psql "dbname=poool user=martin host=localhost" -f /Users/martin/Projects/poool/database/003_settings_extensions.sql
```

### Step 1.3 – Verify migration

```bash
psql "dbname=poool user=martin host=localhost" -c "SELECT column_name, data_type, column_default FROM information_schema.columns WHERE table_name = 'user_settings' ORDER BY ordinal_position;"
```

Expected output should include `currency` and `timezone` columns.

---

## Phase 2: Backend – Settings Module

### Step 2.1 – Create module structure

Create file `backend/src/settings/mod.rs`:

```rust
pub mod models;
pub mod routes;
pub mod service;
```

### Step 2.2 – Create `backend/src/settings/models.rs`

Define all request/response structs using `serde::Deserialize` for form data
and `serde::Serialize` for responses.

| Struct | Fields | Purpose |
|--------|--------|---------|
| `SettingsResponse` | `email, first_name, last_name, phone_number, country, timezone, role, language, currency` | Full settings data for `GET /api/settings` |
| `UpdateProfileForm` | `first_name, last_name, phone_number, country, timezone` | My Details → Save button |
| `UpdatePreferencesForm` | `language, currency` | Preferences → Save button |
| `ChangeEmailForm` | `new_email, current_password` | Security → Change Email |
| `ChangePasswordForm` | `current_password, new_password, confirm_password` | Security → Change Password |
| `ChangePhoneForm` | `new_phone` | Security → Change Phone |
| `ApiResponse` | `success: bool, message: String` | Standard JSON response for all mutations |

**Validation rules** to enforce:
- `first_name` / `last_name`: max 100 chars, no HTML/script tags
- `phone_number`: 7-20 chars, starts with `+`, digits only after prefix
- `country`: exactly 2 uppercase letters (ISO 3166-1 alpha-2)
- `language`: one of `en`, `de`, `fr`, `es`, `id`
- `currency`: one of `USD`, `EUR`, `GBP`, `SGD`, `IDR`
- `timezone`: valid IANA timezone string
- `new_email`: use existing `validation::validate_email()`
- `new_password`: use existing `validation::validate_password()`

### Step 2.3 – Create `backend/src/settings/service.rs`

Business logic functions – **all take `pool: &PgPool` and `user_id: Uuid`**:

```
get_settings(pool, user_id) → Result<SettingsResponse, AppError>
```
- JOIN query across `users`, `user_profiles`, `user_settings`, `user_roles` → `roles`
- Returns composite `SettingsResponse`

```
update_profile(pool, user_id, form) → Result<(), AppError>
```
- Validate inputs (name length, phone format, country code)
- `UPDATE user_profiles SET first_name=$1, last_name=$2, phone_number=$3, country=$4 WHERE user_id=$5`
- `UPDATE user_settings SET timezone=$1 WHERE user_id=$2`

```
update_preferences(pool, user_id, form) → Result<(), AppError>
```
- Validate language ∈ {en, de, fr, es, id}
- Validate currency ∈ {USD, EUR, GBP, SGD, IDR}
- `UPDATE user_settings SET language=$1, currency=$2 WHERE user_id=$3`

```
change_email(pool, user_id, form) → Result<(), AppError>
```
- Validate new email format
- Verify current password (fetch hash from `users`, use `verify_password()`)
- Check new email doesn't already exist
- `UPDATE users SET email=$1, email_verified=FALSE WHERE id=$2`
- Log to `audit_logs` via `common::audit::log()`

```
change_password(pool, user_id, form) → Result<(), AppError>
```
- Verify `confirm_password == new_password` (client-side too, but validate server-side)
- Verify current password
- Validate new password strength via `validation::validate_password()`
- Hash new password with `hash_password()`
- `UPDATE users SET password_hash=$1 WHERE id=$2`
- Log to `audit_logs`

```
change_phone(pool, user_id, form) → Result<(), AppError>
```
- Validate phone format
- `UPDATE user_profiles SET phone_number=$1 WHERE user_id=$2`

### Step 2.4 – Create `backend/src/settings/routes.rs`

All handlers extract `user_id` from the session cookie using the existing
auth middleware pattern (see `auth::middleware`).

| Method | Route | Handler | Content-Type |
|--------|-------|---------|--------------|
| `GET` | `/api/settings` | `get_settings_handler` | → JSON |
| `POST` | `/api/settings/profile` | `update_profile_handler` | Form → JSON |
| `POST` | `/api/settings/preferences` | `update_preferences_handler` | Form → JSON |
| `POST` | `/api/settings/email` | `change_email_handler` | Form → JSON |
| `POST` | `/api/settings/password` | `change_password_handler` | Form → JSON |
| `POST` | `/api/settings/phone` | `change_phone_handler` | Form → JSON |

**Response format** for all POST endpoints:
```json
{ "success": true, "message": "Profile updated successfully." }
```
or on error:
```json
{ "success": false, "message": "Current password is incorrect." }
```

**Error response status codes**:
- `200` with `success: false` for validation errors (user-facing)
- `401` for unauthenticated requests
- `500` for internal errors (generic message, log real error)

### Step 2.5 – Register module in `main.rs`

Add at the top of `main.rs`:
```rust
mod settings;
```

Register routes in the router builder (after the existing auth routes):
```rust
// Settings API
.route("/api/settings", get(settings::routes::get_settings_handler))
.route("/api/settings/profile", post(settings::routes::update_profile_handler))
.route("/api/settings/preferences", post(settings::routes::update_preferences_handler))
.route("/api/settings/email", post(settings::routes::change_email_handler))
.route("/api/settings/password", post(settings::routes::change_password_handler))
.route("/api/settings/phone", post(settings::routes::change_phone_handler))
```

### Step 2.6 – Verify backend compiles

```bash
cd /Users/martin/Projects/poool/backend && cargo check 2>&1 | tail -20
```

---

## Phase 3: Frontend – JavaScript (`static/js/settings.js`)

### Step 3.1 – Create `frontend/platform/static/js/settings.js`

This is the main JS file that wires all settings page interactions.

**Structure:**

```javascript
(function() {
  'use strict';

  // ─── State ──────────────────────────────────────────────────
  let savedSettings = null; // Cache of last-saved values for Cancel

  // ─── Toast Notification System ──────────────────────────────
  function showToast(message, type) { /* type: 'success' | 'error' */ }

  // ─── Load Settings on Page Load ─────────────────────────────
  async function loadSettings() {
    const res = await fetch('/api/settings', { credentials: 'same-origin' });
    if (!res.ok) return;
    savedSettings = await res.json();
    populateMyDetails(savedSettings);
    populatePreferences(savedSettings);
    populateSecurity(savedSettings);
  }

  // ─── Tab: My Details ────────────────────────────────────────
  function populateMyDetails(data) { /* fill form inputs */ }
  async function saveProfile() { /* POST /api/settings/profile */ }
  function cancelProfile() { /* reset to savedSettings */ }

  // ─── Tab: Preferences ───────────────────────────────────────
  function populatePreferences(data) { /* fill dropdowns */ }
  async function savePreferences() { /* POST /api/settings/preferences */ }

  // ─── Tab: Security ──────────────────────────────────────────
  function populateSecurity(data) { /* fill email, phone */ }
  function openChangeEmailModal() { /* show modal */ }
  function openChangePasswordModal() { /* show modal */ }
  function openChangePhoneModal() { /* show modal */ }
  async function submitChangeEmail() { /* POST /api/settings/email */ }
  async function submitChangePassword() { /* POST /api/settings/password */ }
  async function submitChangePhone() { /* POST /api/settings/phone */ }

  // ─── Init ───────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', loadSettings);

  // Expose to global scope for onclick handlers
  window.saveProfile = saveProfile;
  window.cancelProfile = cancelProfile;
  window.savePreferences = savePreferences;
  window.openChangeEmailModal = openChangeEmailModal;
  window.openChangePasswordModal = openChangePasswordModal;
  window.openChangePhoneModal = openChangePhoneModal;
})();
```

**Toast notification behavior:**
- Slide in from top-right
- Auto-dismiss after 4 seconds
- Green background for success, red for error
- Small close (×) button

### Step 3.2 – Add `settings.js` script tag to `settings.html`

Add before `</body>`:
```html
<script src="/static/js/settings.js"></script>
```

---

## Phase 4: Frontend – HTML Updates (`settings.html`)

### Step 4.1 – Fix hardcoded/placeholder data

| Location | Current Value | Fix |
|----------|---------------|-----|
| Security → Email input | `value="mail.poool.app"` | Remove `value`, add `id="settings-security-email"`. JS populates from API. |
| Security → Phone input | `value="+77777777777777777"` | Remove `value`, add `id="settings-security-phone"`. JS populates from API. |
| Security → Password input | Hardcoded dots | Keep dots (correct – never expose real password) |

### Step 4.2 – Fix text typos

| Location | Current | Fixed |
|----------|---------|-------|
| Preferences tab subtitle | `"Choose you language and currency preferences"` | `"Choose your language and currency preferences"` |
| More tab → Refer a friend description | `"your friends and receive rewards"` | `"Invite your friends and receive rewards"` |
| Security → MFA → Email option description | `"You will recieve sms with one-time code"` | `"You will receive an email with a one-time code"` |
| Security → MFA → Phone option description | `"You will recieve sms with one-time code"` | `"You will receive an SMS with a one-time code"` |

### Step 4.3 – Wire Save/Cancel buttons with `onclick` handlers

**My Details tab:**
```html
<button class="settings-btn settings-btn--secondary" onclick="cancelProfile()">Cancel</button>
<button class="settings-btn settings-btn--primary" onclick="saveProfile()">Save</button>
```

**Preferences tab** (add Save/Cancel buttons if not present):
```html
<button class="settings-btn settings-btn--secondary" onclick="cancelPreferences()">Cancel</button>
<button class="settings-btn settings-btn--primary" onclick="savePreferences()">Save</button>
```

### Step 4.4 – Wire Security action buttons

```html
<button class="settings-btn settings-btn--primary" onclick="openChangeEmailModal()">Change Email</button>
<button class="settings-btn settings-btn--primary" onclick="openChangePasswordModal()">Change Password</button>
<button class="settings-btn settings-btn--primary" onclick="openChangePhoneModal()">Change Phone number</button>
```

### Step 4.5 – Wire Legal links to actual URLs

```html
<a href="/privacy-policy" class="settings-legal-pill">View Privacy Policy</a>
<a href="/terms" class="settings-legal-pill">Terms of Use</a>
<a href="/key-risks" class="settings-legal-pill">Key Risks</a>
<a href="/cookies" class="settings-legal-pill">Cookie Notice</a>
```

### Step 4.6 – Wire More tab links

| Card | `href` Target |
|------|---------------|
| Rate us | `https://www.poool.app/rate` or `#` with modal |
| Submit feedback | `/support` |
| Refer a friend | `/rewards` |
| Glossary | `https://www.poool.app/glossary` |
| POOOL Blog | `https://www.poool.app/blog` |
| How it works | `https://www.poool.app/how-it-works` |

### Step 4.7 – Add toast container HTML

Add at the end of `<body>`, before scripts:
```html
<div id="settings-toast-container" style="position:fixed;top:24px;right:24px;z-index:9999;display:flex;flex-direction:column;gap:8px;"></div>
```

---

## Phase 5: Change Email / Password / Phone Modals

### Step 5.1 – Add modal HTML to `settings.html`

Add before `</body>`. Three modals:

1. **Change Email Modal** (`#modal-change-email`):
   - Input: New email address
   - Input: Current password (for verification)
   - Buttons: Cancel / Save

2. **Change Password Modal** (`#modal-change-password`):
   - Input: Current password
   - Input: New password
   - Input: Confirm new password
   - Inline validation: passwords must match
   - Password strength indicator (optional but recommended)
   - Buttons: Cancel / Save

3. **Change Phone Modal** (`#modal-change-phone`):
   - Input: New phone number (with `+` prefix hint)
   - Buttons: Cancel / Save

### Step 5.2 – Add modal CSS to `settings.css`

```css
/* ─── Settings Modal Overlay ─── */
.settings-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  display: none;           /* shown via JS */
  align-items: center;
  justify-content: center;
  z-index: 10000;
}
.settings-modal-overlay.active { display: flex; }

.settings-modal {
  background: #FFFFFF;
  border-radius: 16px;
  padding: 32px;
  width: 100%;
  max-width: 480px;
  box-shadow: 0 20px 60px rgba(0,0,0,0.15);
}

.settings-modal__title {
  font-size: 20px;
  font-weight: 600;
  color: #181D27;
  margin-bottom: 8px;
}

.settings-modal__subtitle {
  font-size: 14px;
  color: #535862;
  margin-bottom: 24px;
}

.settings-modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}
```

### Step 5.3 – Wire modal JS in `settings.js`

- `openChangeEmailModal()` → show `#modal-change-email`
- `openChangePasswordModal()` → show `#modal-change-password`
- `openChangePhoneModal()` → show `#modal-change-phone`
- Each modal's Cancel button → hide modal
- Each modal's Save button → submit to API → show toast → close modal on success
- Click on overlay background → close modal
- `Escape` key → close modal

---

## Phase 6: Photo Upload (Optional – can defer)

### Step 6.1 – Backend: Add upload endpoint

| Method | Route | Handler | Content-Type |
|--------|-------|---------|--------------|
| `POST` | `/api/settings/avatar` | `upload_avatar_handler` | `multipart/form-data` → JSON |

**Logic:**
1. Accept image file (validate: JPG/PNG/SVG, max 800×400px, max 2MB)
2. Generate unique filename: `{user_id}_{timestamp}.{ext}`
3. Save to `frontend/platform/images/avatars/`
4. Update `users.avatar_url` in DB
5. Return `{ success: true, avatar_url: "/images/avatars/..." }`

### Step 6.2 – Frontend: Wire drag-and-drop upload

- Click on upload area → trigger hidden `<input type="file">`
- Drag file onto upload area → same handler
- Preview image before save (FileReader API)
- Send `FormData` via `fetch` to `/api/settings/avatar`
- On success, update avatar in sidebar + settings page

> **Note**: This can be deferred. For MVP, the upload area can exist
> but photo is not persisted until this phase is implemented.

---

## Phase 7: Comprehensive Testing

### Step 7.1 – Create standalone settings test suite

Create `tests/test_settings.py`:

```python
#!/usr/bin/env python3
"""
POOOL Platform – Settings Page Test Suite
==========================================
Comprehensive tests for all Settings page features.

Tests cover:
  1. Page load and structure
  2. GET /api/settings endpoint
  3. POST /api/settings/profile (My Details)
  4. POST /api/settings/preferences (Preferences)
  5. POST /api/settings/email (Change Email)
  6. POST /api/settings/password (Change Password)
  7. POST /api/settings/phone (Change Phone)
  8. Input validation and edge cases
  9. Security checks (auth required, CSRF, XSS)
  10. Database integrity after changes

Run:  python3 tests/test_settings.py
Requires: requests, psycopg2
"""

import json
import sys
import time
import psycopg2
import requests

BASE_URL = "http://localhost:8888"
DB_DSN = "dbname=poool user=martin host=localhost"
TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"
```

### Step 7.2 – Test categories and cases

**Category 1: Page Structure Tests**

| Test | Expected |
|------|----------|
| `GET /settings` → 200 | Page loads for authenticated user |
| Settings page includes `settings.css` | Stylesheet is linked |
| Settings page includes `settings.js` | New JS file is linked |
| Settings page includes `user-data.js` | Existing JS is still linked |
| 4 tab buttons exist | `#tab-mydetails`, `#tab-preferences`, `#tab-security`, `#tab-more` |
| 4 panels exist | `#panel-mydetails`, `#panel-preferences`, `#panel-security`, `#panel-more` |
| My Details form has all inputs | first_name, last_name, email, phone, role, country, timezone |
| Preferences has language dropdown | Options: en, de, fr, es, id |
| Preferences has currency dropdown | Options: USD, EUR, GBP, SGD, IDR |
| Security has Change Email button | Button exists and has onclick |
| Security has Change Password button | Button exists and has onclick |
| No hardcoded `mail.poool.app` | Placeholder removed |
| No hardcoded phone `+7777...` | Placeholder removed |
| No typo `"Choose you language"` | Fixed to `"your"` |
| Toast container exists | `#settings-toast-container` in DOM |

**Category 2: API – GET /api/settings**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Must require session cookie |
| Authenticated → 200 JSON | Returns settings object |
| Response has `email` field | Matches logged-in user's email |
| Response has `first_name` field | String or null |
| Response has `last_name` field | String or null |
| Response has `phone_number` field | String or null |
| Response has `country` field | String or null |
| Response has `timezone` field | Defaults to `"UTC"` if not set |
| Response has `role` field | `"investor"` or `"developer"` |
| Response has `language` field | Defaults to `"en"` if not set |
| Response has `currency` field | Defaults to `"USD"` if not set |

**Category 3: API – POST /api/settings/profile**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Valid update → `{ success: true }` | Profile saved |
| Verify DB after save | `user_profiles` row updated |
| Update first_name only | Other fields unchanged |
| Update all fields at once | All fields updated |
| Empty first_name → validation error | `success: false` with message |
| Name with HTML tags → sanitized or rejected | XSS prevention |
| Very long name (>100 chars) → error | Length validation |
| Invalid country code → error | Must be 2 uppercase letters |
| Cancel then re-load → original values | Cancel resets form |

**Category 4: API – POST /api/settings/preferences**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Change language to `de` → success | `user_settings.language` updated |
| Change currency to `EUR` → success | `user_settings.currency` updated |
| Invalid language `xx` → error | Validation rejects |
| Invalid currency `XYZ` → error | Validation rejects |
| Verify DB after save | Both columns updated in `user_settings` |

**Category 5: API – POST /api/settings/email**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Change email with correct password → success | `users.email` updated |
| Change email resets `email_verified` to false | Security measure |
| Wrong current password → error | `success: false`, "password is incorrect" |
| Invalid new email format → error | Validation |
| Email already exists → error | Conflict check |
| Audit log created | `audit_logs` has entry for email change |
| **Restore original email after test** | Clean up to keep test repeatable |

**Category 6: API – POST /api/settings/password**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Change password with correct current → success | `users.password_hash` updated |
| Wrong current password → error | `success: false` |
| New password doesn't meet requirements → error | Too short, missing uppercase, etc. |
| Confirm password mismatch → error | `confirm_password ≠ new_password` |
| Can login with new password after change | Verify the new hash works |
| Audit log created | `audit_logs` has entry for password change |
| **Restore original password after test** | Clean up |

**Category 7: API – POST /api/settings/phone**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Valid phone `+1234567890` → success | `user_profiles.phone_number` updated |
| Invalid phone `abc` → error | Format validation |
| Empty phone → clears field | Allowed (phone is optional) |

**Category 8: Security & Edge Cases**

| Test | Expected |
|------|----------|
| All POST endpoints reject unauthenticated requests | 401 for each |
| SQL injection in name field | Sanitized, no DB error |
| XSS payload in name field | Escaped or rejected |
| Very large request body (>1MB) | 413 or graceful rejection |
| Concurrent save requests | No data corruption |
| Expired session token | Returns 401 |

**Category 9: Database Integrity**

| Test | Expected |
|------|----------|
| After profile save, `user_profiles.updated_at` changed | Trigger fires |
| After email change, `users.updated_at` changed | Trigger fires |
| After password change, old hash is gone | Only new hash in DB |
| `user_settings` row exists for test user | Created at registration |
| Foreign key constraints hold | No orphan records |

### Step 7.3 – Update existing `tests/test_platform.py`

Expand the `test_settings()` function (currently at line 1034) to include:

```python
def test_settings(session, results: TestResults):
    """Test /settings page – COMPREHENSIVE."""
    results.section("PAGE: /settings")

    html = test_page(session, results, "/settings", "Settings",
        expected_styles=["settings.css"],
        expected_scripts=["settings.js"],
    )

    if html:
        r = session.get(f"{BASE_URL}/settings")

        # ── Tab structure ──────────────────────────────────
        for tab_id in ["tab-mydetails", "tab-preferences", "tab-security", "tab-more"]:
            if tab_id in r.text:
                results.ok(f"  Tab #{tab_id} exists")
            else:
                results.fail(f"  Tab #{tab_id} MISSING")

        for panel_id in ["panel-mydetails", "panel-preferences", "panel-security", "panel-more"]:
            if panel_id in r.text:
                results.ok(f"  Panel #{panel_id} exists")
            else:
                results.fail(f"  Panel #{panel_id} MISSING")

        # ── My Details form fields ─────────────────────────
        expected_inputs = [
            "settings-first-name", "settings-last-name",
            "settings-email", "settings-phone",
            "settings-country", "settings-role",
        ]
        for inp_id in expected_inputs:
            if inp_id in r.text:
                results.ok(f"  Input #{inp_id} exists")
            else:
                results.fail(f"  Input #{inp_id} MISSING")

        # ── No hardcoded placeholders ──────────────────────
        if "mail.poool.app" not in r.text:
            results.ok("  No hardcoded email placeholder")
        else:
            results.fail("  Hardcoded email 'mail.poool.app' still present")

        if "+77777" not in r.text:
            results.ok("  No hardcoded phone placeholder")
        else:
            results.fail("  Hardcoded phone '+7777...' still present")

        # ── Typo fixes ─────────────────────────────────────
        if "Choose you language" not in r.text:
            results.ok("  Typo 'Choose you language' is fixed")
        else:
            results.fail("  Typo 'Choose you language' still present")

        # ── settings.js loaded ─────────────────────────────
        if "settings.js" in r.text:
            results.ok("  settings.js script loaded")
        else:
            results.fail("  settings.js NOT loaded")

    # ── API: GET /api/settings ─────────────────────────────
    results.section("API: GET /api/settings")

    r_unauth = requests.get(f"{BASE_URL}/api/settings")
    if r_unauth.status_code == 401:
        results.ok("GET /api/settings returns 401 when unauthenticated")
    else:
        results.fail(f"GET /api/settings unauthenticated: expected 401, got {r_unauth.status_code}")

    r_auth = session.get(f"{BASE_URL}/api/settings")
    if r_auth.status_code == 200:
        results.ok("GET /api/settings returns 200 when authenticated")
        try:
            data = r_auth.json()
            required_fields = ["email", "first_name", "last_name", "phone_number",
                             "country", "timezone", "role", "language", "currency"]
            for field in required_fields:
                if field in data:
                    results.ok(f"  Field '{field}' present: {data[field]}")
                else:
                    results.fail(f"  Field '{field}' MISSING from response")
        except json.JSONDecodeError:
            results.fail("GET /api/settings does not return valid JSON")
    else:
        results.fail(f"GET /api/settings authenticated: status={r_auth.status_code}")

    # ── API: POST /api/settings/profile ────────────────────
    results.section("API: POST /api/settings/profile")

    r = session.post(f"{BASE_URL}/api/settings/profile",
        json={"first_name": "Test", "last_name": "User",
              "phone_number": "+1234567890", "country": "US",
              "timezone": "America/New_York"})
    if r.status_code == 200:
        data = r.json()
        if data.get("success"):
            results.ok("Profile update succeeded")
        else:
            results.fail(f"Profile update failed: {data.get('message')}")
    else:
        results.fail(f"POST /api/settings/profile: status={r.status_code}")

    # Verify DB
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT p.first_name, p.last_name, p.phone_number, p.country
            FROM user_profiles p
            JOIN users u ON u.id = p.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row and row[0] == "Test" and row[1] == "User":
            results.ok("  DB: Profile data persisted correctly")
        else:
            results.fail(f"  DB: Profile data mismatch: {row}")
        cur.close()
        conn.close()
    except Exception as e:
        results.fail(f"  DB verification failed: {e}")

    # ── API: POST /api/settings/preferences ────────────────
    results.section("API: POST /api/settings/preferences")

    r = session.post(f"{BASE_URL}/api/settings/preferences",
        json={"language": "de", "currency": "EUR"})
    if r.status_code == 200:
        data = r.json()
        if data.get("success"):
            results.ok("Preferences update succeeded")
            # Restore defaults
            session.post(f"{BASE_URL}/api/settings/preferences",
                json={"language": "en", "currency": "USD"})
        else:
            results.fail(f"Preferences update failed: {data.get('message')}")
    else:
        results.fail(f"POST /api/settings/preferences: status={r.status_code}")
```

### Step 7.4 – Run the full test suite

```bash
cd /Users/martin/Projects/poool && python3 tests/test_settings.py
```

### Step 7.5 – Run the existing platform test suite

```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py
```

### Step 7.6 – Manual browser verification

1. Navigate to `http://localhost:8888/settings`
2. **My Details tab**:
   - [ ] All fields are populated from API (not hardcoded)
   - [ ] Change first name → Save → Refresh → name persists
   - [ ] Cancel restores to previous values
   - [ ] Save shows success toast
3. **Preferences tab**:
   - [ ] Language dropdown works and saves
   - [ ] Currency dropdown works and saves
   - [ ] Typo "Choose you language" is fixed
4. **Security tab**:
   - [ ] Email field shows real email (not `mail.poool.app`)
   - [ ] Phone field shows real phone (not `+7777...`)
   - [ ] "Change Email" opens modal, requires password
   - [ ] "Change Password" opens modal, validates match
   - [ ] "Change Phone" opens modal
   - [ ] Each change shows success/error toast
5. **More tab**:
   - [ ] All 6 info cards have working links
   - [ ] Article cards display
   - [ ] "Refer a friend" text is fixed

### Step 7.7 – Security audit checklist

- [ ] All POST endpoints reject unauthenticated requests (401)
- [ ] Password change requires current password verification
- [ ] Email change requires current password verification
- [ ] New passwords are hashed with Argon2id (not stored in plaintext)
- [ ] Email change resets `email_verified` flag
- [ ] Audit log entries created for email/password changes
- [ ] No user enumeration via error messages
- [ ] Input length limits enforced
- [ ] HTML/script tags in name fields are sanitized or rejected

---

## Phase 8: Polish & Edge Cases

### Step 8.1 – Loading states

- Show spinner/skeleton on Save buttons while request is in flight
- Disable form inputs during save to prevent double-submission
- Re-enable inputs on response (success or error)

### Step 8.2 – Error handling

- Network errors (fetch fails) → show "Connection error" toast
- Session expired mid-edit → redirect to login
- Server 500 → show "Something went wrong" toast

### Step 8.3 – Mobile responsiveness

Verify `settings.css` media queries work for:
- Modals center properly on mobile
- Toast notifications don't overlap mobile navigation
- Form fields stack vertically on small screens

---

## Implementation Priority & Time Estimates

| Priority | Phase | Effort | Description |
|----------|-------|--------|-------------|
| 🔴 P0 | **Phase 1** | ~15m | Database migration (2 ALTER TABLE statements) |
| 🔴 P0 | **Phase 2** | ~2h | Backend settings module (models, service, routes) |
| 🔴 P0 | **Phase 3** | ~1.5h | Frontend JS (CRUD + toasts + form binding) |
| 🟡 P1 | **Phase 4** | ~45m | HTML fixes (placeholders, typos, button wiring) |
| 🟡 P1 | **Phase 5** | ~1h | Modals (email, password, phone change) |
| 🟢 P2 | **Phase 6** | ~1h | Photo upload (can defer) |
| 🔴 P0 | **Phase 7** | ~1.5h | Testing (test suite + manual verification) |
| 🟢 P2 | **Phase 8** | ~30m | Polish (loading states, error handling) |

**Total estimated effort: ~8.5 hours**

---

## Quick Reference: Database Schema

### `user_profiles` (existing)
```sql
-- Key columns for Settings page:
first_name VARCHAR(100)
last_name VARCHAR(100)
phone_number VARCHAR(20)
country VARCHAR(2)        -- ISO 3166-1 alpha-2
-- Also has: display_name, date_of_birth, nationality, address_*, city, state_province, postal_code, tax_id
```

### `user_settings` (existing + NEW columns)
```sql
language VARCHAR(5) DEFAULT 'en'
currency VARCHAR(3) DEFAULT 'USD'        -- NEW
timezone VARCHAR(64) DEFAULT 'UTC'       -- NEW
totp_secret, totp_enabled               -- For MFA (future)
email_notifications, push_notifications  -- For notification prefs (future)
```

### `users` (existing)
```sql
email VARCHAR(255) UNIQUE
password_hash TEXT
email_verified BOOLEAN DEFAULT FALSE
avatar_url TEXT
status VARCHAR(20) DEFAULT 'active'
```

### `audit_logs` (existing)
```sql
actor_user_id UUID
action VARCHAR(100)       -- e.g. 'email_changed', 'password_changed'
entity_type VARCHAR(50)   -- e.g. 'user'
entity_id UUID
ip_address INET
user_agent TEXT
created_at TIMESTAMPTZ    -- Immutable, never updated
```
