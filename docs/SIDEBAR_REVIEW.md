# Technical & Design Review: Investor & Developer Sidebar

This review evaluates the implementation of the primary navigation component (`frontend/platform/components/sidebar.html`).

## 1. Structural Architecture

### 🛑 Critical Issue: Code Duplication
The sidebar implementation suffers from extreme duplication. The structure is repeated across:
1.  The primary `aside` (rendered by default as Investor).
2.  The `<template id="investor-sidebar-template">`.
3.  The `<template id="developer-sidebar-template">`.

This leads to a high risk of "split-brain" bugs where a fix applied to the Investor template isn't mirrored in the Developer template.
> [!IMPORTANT]
> **Recommendation**: Refactor navigation items into Jinja2 macros or shared partials. Instead of cloning entire templates, consider a single `aside` structure where only the `sidebar__nav` content changes based on the profile.

### ⚙️ Synchronization (Flicker Prevention)
The use of an inline synchronous script (`document.write`) to inject active styles based on `localStorage` is effective at preventing "flash of unstyled content" (FOUC). However, the profile state should ideally be managed by the Rust backend (SSR) to eliminate reliance on client-side storage for the initial paint.

---

## 2. Navigation & Logic

### 🧭 Active State Handling
The active tab logic is path-dependent and manually managed in JavaScript (Lines 17-38).
-   **Pros**: Fast, prevents flicker.
-   **Cons**: Hard to maintain. Adding a new route (e.g., `/affiliate/payouts`) requires updating multiple `else if` chains and CSS selector strings.

### 👥 Profile Switching
The switcher is well-executed. It saves the `selectedProfile` in `localStorage` and provides a smooth transition between Investor and Developer views. The inclusion of an "Online" indicator and professional avatar group adds a premium feel.

---

## 3. UI/UX & Design

### 🎨 Design System Integrity
-   **Colors**: Several hardcoded colors (e.g., `#0000FF`, `#98FB96`, `rgba(3, 255, 136, 0.15)`) bypass the CSS variable system.
-   **Typography**: Consistent use of TT Norms Pro (via global styles).
-   **Badges**: The "New" and "Soon" badges are visually distinct but use inline styles instead of utility classes.

### 🖼 Iconography
There is an inconsistency between:
-   **`<img>` tags**: Used for most icons (e.g., `wallet-02.svg`, `home-05.svg`). Cannot be styled via CSS `stroke` or `fill`.
-   **Inline `<svg>` tags**: Used for Community, Notifications, and Search.
> [!TIP]
> **Improvement**: Standardize on inline SVGs or a specialized SVG sprite system to allow hover states and active states to be controlled via `color` or `stroke: currentColor`.

---

## 4. Component Analysis

### Investor Sidebar (Marketplace/Resale)
-   **Clear Hierarchy**: Good use of parent/child relationships (Expandable "Marketplace" and "Resale Market").
-   **Dynamic Items**: The Affiliate/Rewards logic correctly toggles based on status.

### Developer Sidebar (Dashboard/Submissions)
-   **Focus**: Clean, focused on asset management and metrics.
-   **Status Badges**: Correctly uses "Soon" for unimplemented features (Ranking), managing user expectations.

---

## 5. Actionable Recommendations

| Task | Priority | Description |
| :--- | :--- | :--- |
| **Deduplication** | 🔴 High | Move shared elements (Logo, Search, Account Card) outside the templates. |
| **Theme Tokens** | 🟡 Medium | Replace hardcoded hex colors with `var(--color-...)` tokens. |
| **Active States** | 🟡 Medium | Implement a more robust active state detector (e.g., a regex-based map). |
| **Icon Standardization** | 🟢 Low | Convert all `img` icons to inline SVGs for better styling control. |
| **Macro Migration** | 🟢 Low | Convert the navigation lists into Jinja2 loops to simplify menu management. |
