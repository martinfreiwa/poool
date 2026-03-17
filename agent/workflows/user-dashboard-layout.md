---
description: Enforce a strict, professional design system for layout, spacing, and typography across all user and developer dashboard pages
---

# POOOL Dashboard Design System Standard

> **Goal:** Every dashboard page must adhere to the exact same layout grid, spacing rhythm, and typography standard.
> A user navigating between pages should perceive **zero visual inconsistency** — identical title placement, identical section gaps, identical padding.
> This document defines the authoritative design tokens and structural rules. No page-specific overrides are permitted.

---

## Current Problems Identified (Audit Summary)

| Problem | Example |
|---------|---------|
| **Page title vertical position varies wildly** | KYC title sits at ~59px from top, Cart at ~117px, Transactions at ~153px |
| **Inconsistent top padding** | Some pages have 96px padding-top (Wallet, Rewards), Transactions has ~107px, KYC has ~20px |
| **Whitespace when KYC banner is hidden** | The 96px padding remains even when the banner is `display: none`, leaving a large empty gap |
| **Section gaps not uniform** | Some pages use 24px between cards, others 32px or 48px |
| **Horizontal content alignment differs** | Content starts at x=171px on Cart but x=326px on KYC |
| **Title icon sizes and styles differ** | Portfolio uses a thin-line icon, Cart uses a filled icon, KYC uses a blue circle icon |
| **Some pages have breadcrumbs, others don't** | Transactions has breadcrumbs pushing the title down, other pages don't |
| **Title font inconsistencies** | Most pages use 36px/700 but some developer pages use different sizes |

---

## Pages In Scope

### User Dashboard (7 pages)
1. `portfolio.html`
2. `wallet.html`
3. `settings.html`
4. `rewards.html`
5. `transactions.html`
6. `support.html`
7. `cart.html`

### Standalone Dashboard Pages (1 page)
8. `kyc.html`

### Developer Dashboard (9 pages)
9. `developer/dashboard.html`
10. `developer/assets.html`
11. `developer/add-asset.html`
12. `developer/asset-detail.html`
13. `developer/application-form.html`
14. `developer/document-upload-step3.html`
15. `developer/property-content.html`
16. `developer/settings.html`
17. `developer/submission-success.html`

### Admin Dashboard (System Pages)
18. `admin/admin-dashboard.html` (and variations)
19. `admin/email-marketing.html`
20. All other `.html` files in `/admin/`

---

## Design Tokens — Complete Specification

All values below are **mandatory**. Every dashboard page must reference these tokens via CSS custom properties. **No hardcoded pixel values are allowed.**

### 1. Page-Level Layout Tokens

| CSS Variable | Value | Purpose |
|---|---|---|
| `--sidebar-width` | `256px` | Fixed width of the sidebar navigation |
| `--content-bg` | `#FAFAFA` | Background color for the main content area |
| `--page-padding-top` | `32px` | Distance from top edge of content area to the first element (banner or title) |
| `--page-padding-bottom` | `48px` | Distance from the last section to the bottom of the page |
| `--page-padding-x` | `32px` | Horizontal padding left & right within the main content area |
| `--page-max-width` | `1200px` | Maximum width of the content inside the main area (optional, for ultra-wide screens) |

### 2. KYC / Notification Banner Tokens

| CSS Variable | Value | Purpose |
|---|---|---|
| `--banner-height` | `auto` | Height of the KYC/notification banner (must be auto, not fixed) |
| `--banner-padding-y` | `12px` | Vertical padding inside the banner |
| `--banner-padding-x` | `16px` | Horizontal padding inside the banner |
| `--banner-margin-bottom` | `24px` | Gap between the banner and the page title below it |
| `--banner-bg` | `#EEF4FF` | Background color for info-level banners |
| `--banner-border-radius` | `12px` | Border radius for banners |

> **CRITICAL RULE:** The banner must be **inside** the page's Flexbox column layout with `gap`. When the banner is hidden (`display: none` or removed from DOM), the gap collapses automatically and the page title moves up to `--page-padding-top`. **No phantom whitespace allowed.**

### 3. Page Title Section Tokens

| CSS Variable | Value | Purpose |
|---|---|---|
| `--page-title-icon-size` | `28px` | Width & height of the icon next to the page title |
| `--page-title-icon-gap` | `12px` | Horizontal gap between the icon and the title text |
| `--page-title-font-size` | `36px` | Font size for all page titles (H1) |
| `--page-title-font-weight` | `700` | Font weight for all page titles |
| `--page-title-color` | `#181D27` | Color for all page titles |
| `--page-title-line-height` | `1.2` | Line height for all page titles |
| `--page-title-margin-bottom` | `24px` | Gap below the page title to the next element (tabs, subtitle, or first section) |

### 4. Subtitle / Description Tokens (below the title)

| CSS Variable | Value | Purpose |
|---|---|---|
| `--page-subtitle-font-size` | `15px` | Font size for the subtitle text below the title |
| `--page-subtitle-font-weight` | `400` | Font weight for subtitles |
| `--page-subtitle-color` | `#475467` | Color for subtitles |
| `--page-subtitle-gap` | `8px` | Gap between the title and subtitle when both are present |
| `--subtitle-to-content-gap` | `24px` | Gap from subtitle to first content section |

### 5. Navigation Tabs Tokens (Rewards, Settings, etc.)

| CSS Variable | Value | Purpose |
|---|---|---|
| `--tabs-margin-top` | `0px` | Tabs should sit directly under the title (use `--page-title-margin-bottom` for spacing) |
| `--tabs-margin-bottom` | `24px` | Gap from tabs to first content section below |
| `--tab-font-size` | `15px` | Font size for tab labels |
| `--tab-font-weight-active` | `600` | Font weight for the active tab |
| `--tab-font-weight-inactive` | `400` | Font weight for inactive tabs |
| `--tab-color-active` | `#1B2559` | Color for the active tab |
| `--tab-color-inactive` | `#667085` | Color for inactive tabs |
| `--tab-indicator-color` | `#1B2559` | Color for the active tab underline |
| `--tab-gap` | `32px` | Horizontal gap between tab items |

### 6. Section & Card Spacing Tokens

| CSS Variable | Value | Purpose |
|---|---|---|
| `--section-gap` | `24px` | **Vertical gap between any two consecutive sections or cards.** This is the single most important token — it MUST be the same everywhere. |
| `--card-padding` | `24px` | Internal padding inside card containers |
| `--card-border-radius` | `12px` | Border radius for card containers |
| `--card-border-color` | `#E5E7EB` | Border color for cards |
| `--card-bg` | `#FFFFFF` | Background color for cards |
| `--card-shadow` | `0 1px 3px rgba(0,0,0,0.04)` | Subtle box-shadow for cards |
| `--card-gap-horizontal` | `24px` | Horizontal gap when cards are placed side by side |

### 7. Section Title Tokens (H2 inside the content)

| CSS Variable | Value | Purpose |
|---|---|---|
| `--section-title-font-size` | `20px` | Font size for section headings (H2) |
| `--section-title-font-weight` | `700` | Font weight for section headings |
| `--section-title-color` | `#101828` | Color for section headings |
| `--section-title-margin-bottom` | `16px` | Gap between a section heading and its content |

### 8. Breadcrumb Tokens (for sub-pages like Transactions)

| CSS Variable | Value | Purpose |
|---|---|---|
| `--breadcrumb-font-size` | `14px` | Font size for breadcrumbs |
| `--breadcrumb-color` | `#475467` | Color for breadcrumb text |
| `--breadcrumb-active-color` | `#1B2559` | Color for the active/current breadcrumb |
| `--breadcrumb-margin-bottom` | `12px` | Gap from breadcrumbs to the page title |

### 9. Global Typography
| CSS Variable | Value | Purpose |
|---|---|---|
| `--font-family` | `'TT Norms Pro', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` | Global font stack |
| `--body-font-size` | `14px` | Default body text size |
| `--body-color` | `#344054` | Default body text color |
| `--label-font-size` | `14px` | Labels, captions, small text |
| `--label-color` | `#475467` | Color for labels |
| `--value-font-size` | `36px` | Large value displays (balances, totals) |
| `--value-font-weight` | `600` | Weight for large value displays |
| `--value-color` | `#101828` | Color for large values |

### 10. Action Buttons in Page Header (e.g. "+ Add More" on Cart)

| CSS Variable | Value | Purpose |
|---|---|---|
| `--header-action-height` | `40px` | Height of header action buttons |
| `--header-action-font-size` | `14px` | Font size inside header action buttons |
| `--header-action-border-radius` | `8px` | Border radius for action buttons |

---

## Standard Page Structure (Mandatory HTML Pattern)

Every dashboard page MUST follow this container structure:

```html
<main class="dashboard-content">
  <!-- OPTIONAL: conditional banner — auto-collapses when hidden -->
  <div class="page-banner" id="kyc-banner">
    <!-- banner content -->
  </div>

  <!-- OPTIONAL: breadcrumbs for sub-pages -->
  <nav class="breadcrumbs">
    <a href="/wallet">Wallet</a> <span>›</span> <span class="current">All transactions</span>
  </nav>

  <!-- REQUIRED: page header with icon + title -->
  <div class="page-header">
    <div class="page-header-left">
      <span class="page-header-icon"><!-- SVG icon, 28×28 --></span>
      <h1 class="page-title">Page Name</h1>
    </div>
    <!-- OPTIONAL: header actions -->
    <div class="page-header-actions">
      <button class="btn-secondary">+ Add More</button>
    </div>
  </div>

  <!-- OPTIONAL: subtitle -->
  <p class="page-subtitle">Brief description of the page.</p>

  <!-- OPTIONAL: navigation tabs -->
  <nav class="page-tabs">
    <a class="tab active">Tab 1</a>
    <a class="tab">Tab 2</a>
  </nav>

  <!-- REQUIRED: content sections -->
  <div class="page-body">
    <section class="card"><!-- Section 1 --></section>
    <section class="card"><!-- Section 2 --></section>
    <section class="card"><!-- Section 3 --></section>
  </div>
</main>
```

### CSS Layout Rules for `.dashboard-content`

```css
.dashboard-content {
  margin-left: var(--sidebar-width);
  padding: var(--page-padding-top) var(--page-padding-x) var(--page-padding-bottom);
  background: var(--content-bg);
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  gap: 0; /* gaps are controlled by individual margin tokens */
}

.page-banner + .page-header,
.page-banner + .breadcrumbs {
  /* When banner is present, use banner-margin-bottom as spacing */
}

/* When banner is display:none, the title sits at page-padding-top naturally */
```

---

## Execution Phases & Implementation Strategy

Um die Absicht wirklich und vor allem *nachhaltig* in die Tat umzusetzen, fehlen hier noch die konkreten Umsetzungsschritte (Wie wir den Code effektiv synchronisieren, sodass wir nicht 20 verschiedene HTML-Seiten manuell verwalten müssen).

### Phase 1: CSS Foundation & Global Variables
1. **Zentrale CSS-Datei anlegen:** Erstelle `frontend/platform/static/css/dashboard-tokens.css`.
2. Schreibe alle Tokens als `:root { ... }` Variablen in diese Datei.
3. **Globale Injektion:** Binde `<link rel="stylesheet" href="/static/css/dashboard-tokens.css">` in den `<head>` **jedes einzelnen** HTML-Files aus der Scope-Liste ein.

### Phase 2: Component Synchronization (The "Dry" Principle)
Da POOOL statische HTML-Files für die Seiten nutzt, besteht die größte Gefahr darin, dass die Navigationsleiste (Sidebar) auf 20 Seiten kopiert wird.
1. Nimm die perfekt layoutete `<nav class="sidebar">` und den `<header>` aus `portfolio.html`.
2. Kopiere diesen perfekten Code-Block in **alle anderen Seiten**, um absolut sicherzustellen, dass die HTML-Struktur identisch ist. *(Dies behebt auch zukünftige Sidebar-Flicker!)*

### Phase 3: Fix Layout Wrappers & KYC Banner Bug
1. Öffne jede Seite und ersetze die äußeren Layout-Wrapper (z.B. `<div class="main-content">`) durch den Standard-Wrapper: `<main class="dashboard-content">`.
2. **Entferne hartkodierte CSS-Styles**: Lösche alte Styles wie `padding: 96px` aus spezifischen Klassen wie `.wallet-hero` oder `.settings-wrapper`.
3. Setze das KYC Banner in die Flexbox-Column-Struktur wie in der Vorlage oben.

### Phase 4: Refactor User Dashboard Pages
Wende das Token-System auf alle 7 User-Seiten und `kyc.html` an:
- `portfolio.html`, `wallet.html`, `settings.html`, `rewards.html`, `transactions.html`, `support.html`, `cart.html`, `kyc.html`
- Überschreibe bestehende H1-Klassen mit der zentralisierten `.page-title` Klasse und dem passenden Icon-Wrapper.

### Phase 5: Refactor Developer & Admin Pages
Wende exakt das gleiche Token-System auf das Developer Dashboard und alle Admin-Seiten (z.B. `email-marketing.html`) an. Hierbei müssen insbesondere die Tab-Menüs in den Developer Settings an die definierten `<nav class="page-tabs">` Standards angepasst werden.

### Phase 6: Anti-Flicker — Eliminate All Visual Flickering on Page Load & Navigation

> **Problem:** When reloading any dashboard page, the user sees a brief flash of unstyled/old content before the final page renders. This creates an unprofessional, broken-feeling experience.

**Root Causes Identified (from code audit):**

| Root Cause | Where | Explanation |
|------------|-------|-------------|
| **FOUC (Flash of Unstyled Content)** | All pages | Browser renders HTML before CSS is fully parsed. The sidebar, cards, and titles appear momentarily unstyled. |
| **JS-injected sidebar active state** | `htmx-init.js` | The `updateNavbarState()` function removes all `.active` classes and re-adds them via JS. During this brief window, the sidebar shows no active item → then snaps to the correct one. |
| **`forceCSSReflow()` hack** | `htmx-init.js:161-180` | This function literally does `element.style.display = "none"` then restores it to force a reflow. This causes a visible blink of the entire `<main>` content area. |
| **KYC Banner hide/show pop-in** | `kyc-banner.js` | The banner starts visible in HTML, then JS fetches `/api/kyc/status` and calls `banner.style.display = "none"`. In the gap between page load and API response, the banner flashes briefly. |
| **Sidebar dropdown animation on load** | `sidebar.css:1319-1326` | The `.sidebar--no-transition` class exists but may not be applied early enough, causing the Marketplace dropdown to animate open on every page load. |
| **Profile dropdown JS injection** | `profile-dropdown.js` | The profile dropdown menu is built/positioned by JS on `DOMContentLoaded`. Until JS runs, the account card area may appear incomplete. |

**Fixes (in order of implementation):**

#### 6.1 — CSS-First Rendering (Prevent FOUC)
```css
/* In dashboard-tokens.css — loaded FIRST in <head> before any other stylesheets */

/* Hide the entire page body until CSS is loaded */
body.loading {
  opacity: 0;
}
body {
  opacity: 1;
  transition: opacity 0.15s ease-in;
}
```
```html
<!-- In every HTML <body> tag -->
<body class="loading">
  ...
  <!-- At the very end, before </body> -->
  <script>document.body.classList.remove('loading');</script>
</body>
```
This ensures the user sees **nothing** until all CSS is parsed and JS has finished initial rendering. The `0.15s` transition makes the page "fade in" instead of popping.

#### 6.2 — Fix `forceCSSReflow()` in htmx-init.js
The current implementation hides and shows the entire `<main>` element to trigger a CSS reflow. This is the **worst** approach — it causes a full-page blink.

**Replace with:**
```js
function forceCSSReflow(element) {
  if (!element) return;
  // Trigger reflow WITHOUT hiding the element
  void element.offsetHeight;
  
  const tables = element.querySelectorAll("table");
  tables.forEach((table) => {
    void window.getComputedStyle(table).width;
    table.classList.add("css-loaded");
  });
}
```

#### 6.3 — Fix KYC Banner Pop-In
The banner should start **hidden by default** in CSS and only be revealed by JS when the KYC status requires it.

```css
/* In dashboard-tokens.css */
.page-banner {
  display: none; /* Hidden by default — JS will show if needed */
}
```
```js
// In kyc-banner.js — only show the banner when status requires it
function showBanner(banner) {
    banner.style.display = ""; // Reveal it
}
// Remove the initial hideBanner() call — it's already hidden via CSS.
```

#### 6.4 — Fix Sidebar Active-State Flash
The `updateNavbarState()` function in `htmx-init.js` strips all `.active` classes and re-adds them. Instead, the active state should be **set server-side via Tera templates** so the correct item is active from the first paint.

**For server-rendered pages:** The backend template should add `sidebar__nav-item--active` to the correct nav item at render time (using a Tera `{% if %}` block or passing the active page as a context variable).

**For HTMX-navigated pages:** Only update the active state on `htmx:afterSwap`, not on initial load.

#### 6.5 — Ensure `.sidebar--no-transition` is Applied Early
```html
<!-- In every HTML file, the sidebar nav MUST start with no-transition -->
<nav class="sidebar sidebar--no-transition">
  ...
</nav>

<!-- Then at end of body, remove it AFTER layout is stable -->
<script>
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      const sidebar = document.querySelector('.sidebar');
      if (sidebar) sidebar.classList.remove('sidebar--no-transition');
    });
  });
</script>
```
The double `requestAnimationFrame` ensures the browser has painted at least one frame before enabling transitions.

#### 6.6 — Remove Unnecessary `display: none / ""` Toggles
Audit all JS files for patterns like:
```js
element.style.display = "none";
element.offsetHeight;
element.style.display = "";
```
Replace with the non-destructive `void element.offsetHeight` pattern or remove entirely if no reflow is needed.

### Phase 7: Pixel-Perfect Verification

| Check | Expected Behavior |
|-------|-------------------|
| Page title top distance | Identical on all pages (32px when no banner, banner-height + 24px when banner is shown) |
| Page title bottom distance | Exactly 24px to first section/tabs/content on every page |
| Section gap | Exactly 24px between every pair of stacked cards/sections on every page |
| Horizontal padding | Exactly 32px on both sides of every page |
| KYC banner hidden | Title jumps to top, no phantom white space |
| Title font | 36px / 700 / #181D27 on every single page |
| Title icon | 28×28, 12px gap to text, on every page |
| Card styling | 24px padding, 12px radius, #E5E7EB border on every card |
| **Page reload flicker** | **Zero flicker on hard-reload (Cmd+Shift+R) for every page** |
| **Sidebar active state** | **Correct item highlighted from first paint — no flash to "no active" and back** |
| **KYC banner flash** | **Banner does NOT appear for a split second, then disappear** |
| **HTMX navigation** | **Smooth swap, no blink of entire content area** |
| Responsive | Layout degrades gracefully on mobile/tablet |
