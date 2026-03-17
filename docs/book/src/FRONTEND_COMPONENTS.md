# Frontend Component Explorer (HTMX & Alpine.js)

This document serves as the technical catalog for the POOOL platform's hybrid frontend architecture. We combine **Server-Side Rendering (Axum/Askama)** for speed, **HTMX** for dynamic partial updates, and **Alpine.js** for client-side state management.

## 1. Architectural Strategy: The "Hybrid Power" Trio

| Technology | Role | Usage Example |
| :--- | :--- | :--- |
| **Rust Admin/SSR** | Navigation & Page Shell | Sidebar, Header, Initial Page Load |
| **HTMX** | Atomic Content Swaps | Form submissions, Wallet filtering, Live Search |
| **Alpine.js** | Micro-State & UI Logic | Modals, Dropdowns, Tab switching, Expandable Cards |

---

## 2. Alpine.js Component Catalog

These components manage complex client-side interactions without high-overhead frameworks.

### `tierData()` (Rewards Tier Page)
**File**: `static/js/tier-page.js`
**Scope**: Manages the multi-step "Stepper" and the dynamic progress card on the /tier page.

| Property | Type | Description |
| :--- | :--- | :--- |
| `data.tier_name` | `string` | Current user tier (Intro, Plus, Pro, Elite, Premium). |
| `data.invested_12m` | `number` | Total investment value (cents) in the rolling window. |
| `data.progress_pct` | `number` | Calculated percentage (0-100) toward the next tier. |
| `data.tier_target` | `string` | Name of the next tier to unlock. |

**Key Methods**:
- `initTier()`: Fetches `/api/rewards` and `/api/rewards/tiers`.
- `getStepperState(stepName)`: Returns `active`, `past`, or `inactive` for visual stepper logic.

### `portfolioData` (Portfolio Overview)
**File**: Embedded in `portfolio.html`
**Scope**: Manages chart visibility and time-period tab switching.

| State | Description |
| :--- | :--- |
| `expanded` | Global boolean for "Show more" chart section. |
| `activeTab` | Tracks current chart view (`twelveMonths`, `thirtyDays`, etc.). |

---

## 3. HTMX Fragment Catalog (Server-Returned UI)

HTMX interactions expect specific HTML fragments from the Rust backend.

### User Deposit Feedback
**Endpoint**: `POST /api/payments/deposit`
**Target**: `#deposit-feedback-container`
**Response**:
```html
<div class="deposit-success">
    <!-- Instructions, Reference ID, and status message -->
</div>
```

### Wallet Filtering
**Endpoint**: `GET /api/wallets/transactions?type={type}`
**Target**: `#transactions-tbody`
**Response**: A list of `<tr>` elements representing filtered wallet actions.

### Admin Notifications
**Endpoint**: `GET /api/admin/notifications`
**Target**: `#notification-list`
**Response**: A vertical list of individual notification cards.

---

## 4. UI Primitives & Design System

POOOL uses a premium "FinTech Modern" design system.

### Color Tokens (CSS Variables)
- `--brand-primary`: `#0000FF` (Deep Blue)
- `--success-500`: `#17B26A` (Emerald Green)
- `--warning-500`: `#F79009` (Amber)
- `--error-500`: `#F04438` (Sunset Red)
- `--gray-900`: `#101828` (Crow Black)

### Standard Interactions (Animations)
- **Modals**: Fade-in with a subtle `translate-y-4` to `translate-y-0` slide.
- **Buttons**: Scale `0.98` on active click; elevation shift on hover.
- **HTMX Swaps**: Use `hx-swap="innerHTML transition:true"` for smooth opacity cross-fades.

---

## 5. Development Guidelines
1. **Never use placeholders**: Every component must have a fallback or a skeleton loader.
2. **Alpine for State, HTMX for Data**: If the data needs to touch the database, use HTMX. If the UI just needs to toggle, use Alpine.
3. **Prefix IDs**: Use page prefixes for IDs (e.g., `portfolio-chart-container`) to avoid collisions in the global DOM.
