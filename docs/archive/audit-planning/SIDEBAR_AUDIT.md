# Sidebar Audit — POOOL Platform

This document catalogs all pages in the `frontend/platform/` directory and identifies which sidebar version they implement.

## Sidebar Versions

| Version | Name | Implementation File | Loading Mechanism | Target Container |
| :--- | :--- | :--- | :--- | :--- |
| **V1** | **Investor Sidebar** | `components/sidebar.html` | Server-Side Include (`{% include %}`) | `aside[data-profile="investor"]` |
| **V2** | **Developer Sidebar** | `components/sidebar.html` | Server-Side Include + JS Template Clone | `template#developer-sidebar-template` |
| **V3** | **Admin Sidebar** | `static/js/admin-sidebar-loader.js` | Client-Side JS Loader | `div#admin-sidebar-placeholder` |

---

## Page Mapping

### 1. Investor Pages (V1 Sidebar)
These pages use the standard investor sidebar provided by `components/sidebar.html`.

*   `marketplace.html`
*   `portfolio.html`
*   `wallet.html`
*   `rewards.html`
*   `community.html`
*   `leaderboard.html`
*   `cart.html`
*   `checkout.html`
*   `settings.html`
*   `support.html`
*   `transactions.html`
*   `my-trading.html`
*   `marketplace-secondary.html`
*   `commodity.html` (Dynamic)
*   `property.html` (Dynamic)
*   `affiliate-dashboard.html`
*   `affiliate-referrals.html`
*   `affiliate-materials.html`
*   `affiliate-settings.html`
*   `kyc.html`

### 2. Developer Pages (V2 Sidebar)
These pages use the developer-specific sidebar defined in the template within `components/sidebar.html`.

*   `developer/dashboard.html`
*   `developer/assets.html`
*   `developer/submissions.html`
*   `developer/add-asset.html`
*   `/developer/settings` (renders shared `settings.html`)
*   `developer/asset-detail.html` (Dynamic)
*   `developer/application-form.html`

### 3. Admin Pages (V3 Sidebar)
These pages use the dynamic admin sidebar loader from `admin-sidebar-loader.js`.

*   `admin/index.html` (Dashboard)
*   `admin/users.html`
*   `admin/user-details.html` (Dynamic)
*   `admin/kyc.html`
*   `admin/support.html`
*   `admin/orders.html`
*   `admin/deposits.html`
*   *...and all other files in `frontend/platform/admin/`*

### 4. Special & Multi-layout Pages
*   `statistics-template.html`: Uses Admin Sidebar (V3) for testing
*   `forms-template.html`: Uses Admin Sidebar (V3)
*   `table-template.html`: Uses Admin Sidebar (V3)
*   `cards-template.html`: Uses Admin Sidebar (V3)

### 5. No Sidebar (Full Screen)
The following pages do not include any sidebar and are designed as full-screen experiences.

*   **Auth Flow**: `login.html`, `signup.html`, `forgot-password.html`, `reset-password.html`, `verify-email.html`, `auth-2fa.html`, `auth-2fa-setup.html`
*   **Legal/Policy**: `privacy-policy.html`, `terms.html`, `cookies.html`, `aml-kyc-policy.html`, `currency-policy.html`, `imprint.html`
*   **Landing/Welcome**: `welcome.html`, `index.html` (Marketing bridge)
*   **System**: `403.html`, `404.html`, `500.html`, `maintenance.html`
*   **Checkout Success**: `payment-success.html`, `trade-success.html`

---

## Technical Details

### V1 & V2 Switching Logic
The `components/sidebar.html` file contains an inline script that detects the path:
```javascript
var isDeveloperPage = path.indexOf("/developer/") === 0 || path === "/developer";
if (isDeveloperPage) {
  // Logic to hide investor and clone developer template
}
```

### V3 Loading Logic
Admin pages include a placeholder:
```html
<div id="admin-sidebar-placeholder"></div>
<script src="/static/js/admin-sidebar-loader.js"></script>
```
The script then fetches and injects the sidebar HTML synchronously.
