# POOOL Admin Dashboard Testing 🧪

This directory contains the comprehensive test suite for the POOOL Admin Dashboard. These tests ensure that all admin pages, API endpoints, and functional features are working as expected, maintaining the high security and reliability standards required for a FinTech platform.

## 🗂 Test Structure

The test suite is divided into several specialized files:

1.  **`test_admin_dashboard.py`**: The primary UI and accessibility test. It verifies:
    *   All admin pages (`/admin/*.html`) return 200 OK.
    *   Core components (Sidebar, Admin CSS, HTMX, Alpine.js) are present.
    *   Correct navigation state (active links).
    *   All core Admin API endpoints return valid JSON.

2.  **`test_admin_features.py`**: Functional feature testing. It verifies:
    *   **User Management**: Listing, detailed view, balance adjustment, and status updates.
    *   **Financials**: Orders, Deposits, and Treasury overview.
    *   **Assets**: Submissions review and live asset management.
    *   **Support & KYC**: Ticket management and identity verification records.
    *   **Rewards**: Tiers, referral tracking, and reward balances.
    *   **System**: Health monitoring, audit logs, and global settings.

3.  **`test_admin_security.py`**: Critical security verification. It ensures:
    *   Unauthenticated users are blocked from admin resources.
    *   Standard 'investor' users are strictly denied access to all `/api/admin/*` endpoints (403 Forbidden).

4.  **`test_admin_sorting.py`**: UI/UX verification. It checks:
    *   Presence of sorting headers and pagination controls on lists (Users, KYC, Support, etc.).
    *   Inclusion of required JavaScript modules for interactive tables.

## 🚀 Running the Tests

### Prerequisites
*   The POOOL backend must be running (`cargo run` in `backend/`).
*   A database named `poool` must exist with the current schema.
*   An admin user must exist (default: `test@poool.app` / `TestPass123!`).

### Run all tests
```bash
python3 tests/admin/test_admin_dashboard.py
python3 tests/admin/test_admin_features.py
python3 tests/admin/test_admin_security.py
python3 tests/admin/test_admin_sorting.py
```

### Run a specific test
```bash
python3 tests/admin/test_admin_features.py
```

## 🛠 Adding New Tests

When adding new admin pages or features:
1.  **UI**: Add the new path to `test_admin_dashboard.py` in the `main()` function.
2.  **API**: Add a new test function in `test_admin_features.py` that calls the endpoint and asserts the response structure.
3.  **Security**: If it's a new sensitive endpoint, add it to the `test_admin_security.py` list of restricted paths.

## 📊 CI/CD Integration

These tests are designed to be run in CI environments. They return exit code `0` on success and `1` on failure, making them easy to integrate with GitHub Actions or other automation tools.
