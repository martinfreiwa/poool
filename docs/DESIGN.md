# Design System Strategy: The Wealth Terminal

> **⚠️ SCOPE: This design system applies ONLY to the Investor Dashboard and the Developer Dashboard.**
> The Admin Dashboard has its own design language and is NOT governed by this document.
> Do NOT apply these styles, tokens, or component patterns to any page under `/admin/`.

## 1. Overview & Creative North Star
The Creative North Star for this design system is **"The Wealth Terminal."**

POOOL is a fractional real-world asset investment platform. Investing in property is personal, high-stakes, and institutional. Instead of the cluttered, "fintech-maximalist" aesthetic common in crypto and DeFi platforms, this system mimics the atmosphere of a premium wealth management terminal: **clean, spacious, data-forward, and quietly authoritative.**

We break the "standard template" look through **Tonal Layering** and **Chromatic Minimalism**. By using a near-monochrome surface hierarchy punctuated by two high-contrast brand accents — Electric Blue and Signal Green — we create a platform that feels institutional yet contemporary. Data density is never sacrificed, but information is always breathing. Every pixel of whitespace is intentional.

The result is a platform that feels more like a Bloomberg terminal redesigned by a Swiss design studio than a typical crypto dashboard.

---

## 2. Colors: Chromatic Authority

Our palette is deliberately restrained. Two high-saturation brand colors dominate an otherwise neutral canvas.

### Brand Signature
*   **Electric Blue** (`--primary-color: #0000FF`) — The anchor color. Used for primary CTAs, active navigation states, focus rings, links, and brand identity. Pure, uncompromised blue signals trust, technology, and institutional presence.
*   **Signal Green** (`--brand-greeny-green: #03FF88`) — The action color. Used for progress bars, growth indicators, active sidebar states, and page header icon backgrounds. This green says *"money is moving"* without resorting to clichéd finance-green.
*   **Mint Green** (`--btn-primary-color: #98FB96`) — The button text color on primary buttons, and the secondary button fill. Creates the distinctive blue-on-green / green-on-blue brand pairing.

### Surface Hierarchy
Treat the UI as **stacked sheets of paper on a desk**. Depth is created through background color shifts, not shadow stacking.
*   **Base Canvas:** `#FAFAFA` (`--content-bg`) — The workspace. Every page's main content area.
*   **Card Surface:** `#FFFFFF` (`--card-bg`) — Cards and containers. White on off-white creates a natural, soft lift.
*   **Table Header / Recessed:** `#FAFAFA` (`--table-header-bg`) — Resets table headers to match the canvas, creating a "flush" appearance.
*   **Hover Surface:** `#F8F9FC` (`--table-row-hover-bg`) — Subtle blue-tinted hover state for interactive rows.
*   **Ghost Hover:** `#F2F4F7` (`--btn-ghost-hover-bg`) — Slightly warmer grey for button hover states.
*   **Sidebar:** `#FFFFFF` with `1px solid #E9EAEB` border-right — Clean, bright sidebar that stays out of the content's way.

### Text Hierarchy
*   **Primary Text:** `#181D27` (`--page-title-color`) — Near-black with a hint of navy. Used for all headings, primary values, and account names.
*   **Body Text:** `#344054` (`--body-color`) — Warm dark grey for comfortable long-form reading.
*   **Secondary Text:** `#535862` (`--text-secondary`) — Muted grey for supporting information, timestamps, and metadata.
*   **Tertiary/Caption:** `#667085` — Subdued grey for captions, hints, and placeholder text.
*   **Label Text:** `#475467` (`--label-color`) — Mid-grey for form labels and subtitles.
*   **Table Headers:** `#717680` (`--table-header-color`) — Uppercase, tracked-out, deliberately understated.

### Borders & Dividers
*   **Card Border:** `1px solid #E5E7EB` (`--card-border-color`) — Barely there. Defines containers without drawing the eye.
*   **Table/Divider Border:** `1px solid #E9EAEB` (`--table-border-color`) — The universal structural line.
*   **Input Border:** `1px solid #D0D5DD` (`--input-border-color`) — Slightly stronger for interactive affordance.

### Semantic Colors
*   **Success:** `#027A48` on `#ECFDF3` — Muted forest green. Not screaming, just affirming.
*   **Warning:** `#B54708` on `#FFFAEB` — Warm amber. Cautious, not alarming.
*   **Danger:** `#B42318` / `#D92D20` on `#FEF3F2` — Deep red, professional urgency.
*   **Info:** `#1B2559` on `#EEF4FF` — Brand-adjacent navy on pale blue.

### The "No Raw Black" Rule
**`#000000` is strictly prohibited** for text on the platform dashboard. Always use `#181D27` (headings) or `#344054` (body). The only place `#000000` should appear is in the brand logo SVG.

---

## 3. Typography: Single-Font Authority

Unlike multi-font systems, POOOL uses a **single typeface** for everything. This creates an unmistakable identity and eliminates font-conflict bugs.

### The Typeface: TT Norms Pro
A modern geometric sans-serif loaded in four weights:
*   **Regular (400)** — Body text, descriptions, form inputs
*   **Medium (500)** — Labels, badge text, tab inactive state, form labels
*   **Bold (700)** — Page titles, section titles, card titles, stat values
*   **ExtraBold (800)** — Reserved for marketing/hero moments (rarely used in-app)

### Type Scale

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|-------|------|--------|-------------|----------------|-----|
| `ds-text-display` | 36px | 700 | 1.2 | -0.02em | Hero numbers, portfolio balances |
| `ds-text-xl` | 36px | 700 | 1.2 | -0.02em | Page titles (H1) |
| `ds-text-lg` | 24px | 700 | 32px | — | Section titles (H2) |
| `ds-text-md` | 18px | 600 | 28px | — | Card titles, modal titles |
| `ds-text-sm-heading` | 16px | 600 | 24px | — | Sub-section headings |
| `ds-text-body-lg` | 16px | 400 | 24px | — | Prominent body copy |
| `ds-text-body` | 14px | 400 | 20px | — | Default body text |
| `ds-text-subtitle` | 15px | 400 | 22px | — | Page subtitles, descriptions |
| `ds-text-caption` | 13px | 400 | 18px | — | Labels, hints, timestamps |
| `ds-text-xs` | 12px | 400 | 18px | — | Badges, fine print, table headers |

### Financial Numbers
Use `ds-text-money` class. Applies `font-variant-numeric: tabular-nums` and `font-weight: 600` so dollar amounts and percentages align in columns. **All financial data must use this class.**

### The "Tight Headlines" Rule
All headings at `ds-text-xl` (36px) and `ds-text-display` must use `letter-spacing: -0.02em`. This creates the premium, "locked-in" typographic feel. Body text uses default tracking for readability.

---

## 4. Elevation & Depth: Tonal Layering

Traditional shadow-heavy UIs look dated. POOOL achieves depth through color shifts and minimal, purpose-driven shadows.

### The Layering Principle
Standard cards have **no visible shadow** in the default state. Structural hierarchy is achieved by placing `#FFFFFF` cards on the `#FAFAFA` canvas. The `1px solid #E5E7EB` border provides just enough definition.

### Allowed Shadows
*   **Card Shadow (default):** `0px 1px 2px rgba(10, 13, 18, 0.05)` — Barely perceptible. Just enough to give the card "weight" without lifting it.
*   **Interactive Card Hover:** `0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)` — Used on marketplace property cards. Signals clickability with a gentle lift.
*   **Modal Shadow:** `0px 8px 8px -4px rgba(16, 24, 40, 0.03), 0px 20px 24px -4px rgba(16, 24, 40, 0.08)` — Reserved exclusively for modals and floating panels. This is the most dramatic shadow in the system.
*   **Focus Ring:** `0 0 0 3px rgba(0, 0, 255, 0.08)` — The Electric Blue glow for focused inputs.

### Glassmorphism (Selective)
Used sparingly for **stat card icons** (`.glass-icon-container`) and **badge overlays** on property images:
*   **Glass Icon Front:** `backdrop-filter: blur(8px)` + `rgba(255, 255, 255, 0.5)` background + `1px solid rgba(255, 255, 255, 0.6)` border.
*   **Badge Overlay:** `backdrop-filter: blur(4px)` + `rgba(255, 255, 255, 0.92)` background.
*   **Modal Overlay:** `backdrop-filter: blur(6px)` + `rgba(52, 64, 84, 0.6)` background.

### Glass Icon Color Variants
The rotated background squares behind glass icons use gradients:
*   **Purple:** `linear-gradient(135deg, #8B5CF6 0%, #A78BFA 100%)`
*   **Green:** `linear-gradient(135deg, #059669 0%, #34D399 100%)`
*   **Blue:** `linear-gradient(135deg, #0000FF 0%, #60A5FA 100%)`

---

## 5. Spacing: The 8px Grid

All spacing values are multiples of 8px. This creates the rhythmic, institutional feel.

| Token | Value | Use |
|-------|-------|-----|
| `--ds-page-pt` | 48px | Top padding on every page |
| `--ds-page-pb` | 64px | Bottom padding on every page |
| `--ds-page-px` | 24px | Horizontal padding on page content |
| `--ds-page-gap` | 24px | Gap between major page sections |
| `--section-gap` | 24px | Gap between cards/sections within a page |
| `--card-padding` | 24px | Internal padding of cards |
| `--card-gap-horizontal` | 24px | Horizontal gap between side-by-side cards |
| `--ds-header-icon-gap` | 12px | Gap between page icon and title |
| `--tab-gap` | 32px | Gap between navigation tabs |
| `--btn-icon-gap` | 8px | Gap between icon and text in buttons |

### The "Content Width" Rule
*   **Sidebar width:** `256px` — Fixed. Never changes above 768px.
*   **Page max-width:** `1200px` — Content never stretches beyond this.
*   **Main content width:** `calc(100% - 256px)` — Always fills remaining space.

---

## 6. Components: Refined Primitives

### Page Layout (`.ds-main`)
Every dashboard page uses the same skeleton:
```html
<main class="ds-main">
  <!-- Optional: KYC banner -->
  <div class="ds-page-header">
    <div class="ds-page-header__title-row">
      <div class="ds-page-header__icon poool-icon-custom">
        <svg>...</svg>
      </div>
      <h1 class="ds-page-header__title">Page Title</h1>
    </div>
    <div class="ds-page-header__actions">
      <button class="ds-btn ds-btn--primary">Action</button>
    </div>
  </div>
  <!-- Page content -->
</main>
```
*   The page header icon uses a `44×44px` blue background (`--btn-primary-bg`) with `border-radius: 10px` and green SVG strokes (`#03FF88`).

### Cards (`.ds-card`)
*   **Background:** `#FFFFFF`
*   **Border:** `1px solid #E5E7EB`
*   **Radius:** `12px` — Consistent everywhere. No card uses a different radius.
*   **Padding:** `24px` (default), `16px` (`.ds-card--sm`), `0` (`.ds-card--flush`)
*   **Interactive cards** get `translateY(-2px)` on hover with a slightly elevated shadow.

### Buttons (`.ds-btn`)
Four variants, three sizes:

| Variant | Background | Text | Border | Hover |
|---------|-----------|------|--------|-------|
| **Primary** | `#0000FF` | `#98FB96` | none | `#0000CC` |
| **Secondary** | `#98FB96` | `#0000FF` | none | `#7BE079` |
| **Danger** | `#D92D20` | `#FFFFFF` | none | `#B42318` |
| **Ghost** | transparent | `#475467` | none | `#F2F4F7` bg |

| Size | Height | Padding | Font |
|------|--------|---------|------|
| **Small** | 32px | 6px 12px | 13px |
| **Medium** | 40px | 10px 18px | 14px |
| **Large** | 48px | 12px 24px | 16px |

*   All buttons use `border-radius: 8px` and `font-weight: 600`.
*   Icon-only buttons (`.ds-btn--icon`) are square at the variant height.

### Badges (`.ds-badge`)
Pill-shaped status indicators: `border-radius: 16px`, `padding: 2px 8px`, `font-size: 12px`, `font-weight: 500`.

| Variant | Background | Text |
|---------|-----------|------|
| **Success** | `#ECFDF3` | `#027A48` |
| **Warning** | `#FFFAEB` | `#B54708` |
| **Danger** | `#FEF3F2` | `#B42318` |
| **Info** | `#EEF4FF` | `#1B2559` |
| **Neutral** | `#F2F4F7` | `#344054` |

Property-type badges (Leasehold, Freehold, Commercial, Commodity) include a `1px solid` border in a tinted color variant.

### Navigation Tabs (`.page-tabs`)
*   **Inactive:** `15px`, weight `400`, color `#667085`.
*   **Active:** weight `600`, color `#1B2559`, with a `2px solid #1B2559` bottom border.
*   **Gap:** `32px` between tabs.
*   **Border-bottom:** `1px solid #E5E7EB` on the tab row.

### Form Inputs (`.ds-input`, `.ds-select`, `.ds-textarea`)
*   **Height:** `44px`
*   **Border:** `1px solid #D0D5DD`
*   **Radius:** `8px`
*   **Focus:** Border becomes `#0000FF` with `0 0 0 3px rgba(0, 0, 255, 0.08)` glow.
*   **Placeholder:** `#667085`
*   **Disabled:** `#F9FAFB` background, `#667085` text.

### Modals (`.ds-modal`)
*   **Border Radius:** `16px` — More rounded than cards to distinguish overlay context.
*   **Max Width:** `480px` (default), `360px` (sm), `640px` (lg), `800px` (xl).
*   **Animation:** Slide-in from `translateY(-12px) scale(0.97)` to origin, `0.25s ease-out`.
*   **Overlay:** `rgba(52, 64, 84, 0.6)` with `backdrop-filter: blur(6px)`.
*   **Mobile:** Becomes bottom-sheet style — `border-radius: 16px 16px 0 0`, aligned to `flex-end`.

### Tables (`.ds-table`)
*   **Header:** Uppercase, `12px`, `font-weight: 600`, `#717680`, `letter-spacing: 0.04em`.
*   **Cells:** `14px`, `#535862`, `padding: 16px 24px`.
*   **Row hover:** `#F8F9FC`.
*   **Wrapped in:** `.ds-table-container` which provides the card-like border and radius.

### Progress Bars (`.ds-progress`)
*   **Track:** `#D5D7DA` background with `border-radius: 30px` (fully rounded).
*   **Fill:** `rgb(152, 251, 150)` — Brand green.
*   **Heights:** `6px` (sm), `8px` (md/default), `12px` (lg).
*   **Animation:** `width` transition over `0.6s ease`.
*   Color variants: `--blue` (uses `#0000FF`), `--warning`, `--danger`.

### Sidebar Navigation
*   **Width:** `256px` fixed.
*   **Background:** `#FFFFFF` with `1px solid #E9EAEB` right border.
*   **Nav items:** `36px` height, `6px 10px` padding, `6px` border-radius.
*   **Active state:** `rgba(3, 255, 136, 0.15)` background (translucent brand green).
*   **Active icon:** CSS filter tuned to Electric Blue (`#0000FF`).
*   **Active text:** `--brand-dark-blue: #08232F`.
*   **Hover state:** `#F5F5F6` background.
*   **Featured card (Community):** `rgba(3, 255, 136, 0.1)` background, `12px` border radius.

---

## 7. Animations & Transitions

### Page Transitions (View Transitions API)
POOOL uses the **cross-document View Transitions API** for SPA-like page navigation:
*   **Sidebar:** No animation — stays perfectly pinned via `view-transition-name: sidebar`.
*   **Main content:** `fade + translateY(6px)` slide. Out: `220ms`, In: `280ms`, `cubic-bezier(0.4, 0, 0.2, 1)`.
*   **Respects `prefers-reduced-motion`:** Animation duration collapsesed to `0.01ms`.

### Micro-Animations
*   **Button hover:** `all 0.15s ease` — covers background, color, and transform.
*   **Card hover (interactive):** `transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease`.
*   **Modal entry:** `0.25s ease-out` slide-in with scale.
*   **Modal overlay:** `0.2s ease-out` fade-in.
*   **Progress bar fill:** `width 0.6s ease` — smooth growth animation.
*   **Input focus:** `border-color 0.2s ease, box-shadow 0.2s ease`.
*   **Nav item hover:** `all 0.2s ease`.
*   **FOUC prevention:** `opacity 0.15s ease-in` on body reveal.

### The "No Bounce" Rule
**Spring/bounce easing is prohibited.** All animations use `ease`, `ease-out`, or `cubic-bezier(0.4, 0, 0.2, 1)`. Overshooting animations undermine the institutional tone.

---

## 8. Responsive Design

### Breakpoints

| Breakpoint | Width | Behavior |
|-----------|-------|----------|
| **Desktop** | > 1024px | Full sidebar + content |
| **Tablet** | 768px–1024px | Sidebar narrows to 240px |
| **Mobile** | ≤ 768px | Sidebar collapses, content goes full-width |

### Mobile Adaptations
*   **Sidebar:** Becomes relative, full-width, horizontal. Border switches from right to bottom.
*   **Page padding:** `16px` horizontal, `80px` top (for mobile header).
*   **Page title:** Drops from `36px` to `28px`.
*   **Page header:** Stacks vertically, `align-items: flex-start`.
*   **Grid layouts:** Collapse from `2-column` to `1-column`.
*   **Modal:** Becomes bottom-sheet style with `border-radius: 16px 16px 0 0`.
*   **Large buttons:** Shrink to medium size specs.

---

## 9. Z-Index Scale

| Level | Z-Index | Element |
|-------|---------|---------|
| Base | 0 | Default content |
| Cards | 10 | Elevated cards, glass icons |
| Sticky headers | 20 | Sticky table headers |
| Navigation | 30 | Secondary overlays |
| Sidebar | 100 / 1000 | Main sidebar navigation |
| Modals | 10000 | Modal overlays |

---

## 10. Do's and Don'ts

### Do
*   **Do** use CSS custom properties from `dashboard-tokens.css` for every value. Hardcoded pixels in page-specific CSS are a code smell.
*   **Do** use the `ds-` prefixed design system classes for all new components.
*   **Do** create depth through background color shifts (`#FFFFFF` card on `#FAFAFA` canvas).
*   **Do** use `font-variant-numeric: tabular-nums` (via `.ds-text-money`) for all financial numbers so columns align.
*   **Do** specify `box-sizing: border-box` on every layout container.
*   **Do** use `transition: all 0.15s ease` or `0.2s ease` for interactive feedback. Users should see *immediate* response.
*   **Do** use the Electric Blue + Signal Green brand pairing for primary actions. This is the visual fingerprint of POOOL.
*   **Do** respect `prefers-reduced-motion` by collapsing animation durations.

### Don't
*   **Don't** use `#000000` for text. Use `#181D27` for headings, `#344054` for body.
*   **Don't** use heavy drop shadows on cards. Use tonal layering instead.
*   **Don't** use colors outside the token system. Every hex value should trace back to a `--variable`.
*   **Don't** use more than one typeface. TT Norms Pro handles everything.
*   **Don't** use `!important` in design system files. Reserve it only for the universal page title standardization overrides in `dashboard-tokens.css`.
*   **Don't** use border-radius values other than `8px` (buttons/inputs), `12px` (cards), or `16px` (modals). Three radii, that's it.
*   **Don't** stack multiple shadows. One shadow token per element, maximum.
*   **Don't** use bounce/spring easing. The platform should feel assured, not playful.
*   **Don't** add CSS framework dependencies (Tailwind, Bootstrap). This is a vanilla CSS design system by design.
*   **Don't** place business logic in the frontend. Styling only.

---

## 11. File Architecture

```
frontend/platform/static/css/
├── bundle.css                    ← Auto-generated, do not edit
├── fonts.css                     ← @font-face declarations for TT Norms Pro
├── dashboard-tokens.css          ← ALL design tokens (single source of truth)
├── main.css                      ← Base reset, app layout, utilities
├── ds-page-layout.css            ← .ds-main, .ds-page-header
├── ds-typography.css             ← .ds-text-* classes
├── ds-buttons.css                ← .ds-btn variants and sizes
├── ds-cards.css                  ← .ds-card base and modifiers
├── ds-forms.css                  ← .ds-input, .ds-select, .ds-textarea
├── ds-modals.css                 ← .ds-modal overlay and container
├── ds-badges.css                 ← .ds-badge status pills
├── ds-tables.css                 ← .ds-table and flex-based tables
├── ds-progress.css               ← .ds-progress bar variants
├── ds-utilities.css              ← .ds-flex, .ds-mt-*, .ds-hidden, etc.
├── sidebar-navigation.css        ← Sidebar nav component
└── [page-name].css               ← Page-specific overrides
```

### The Token Cascade
1. `dashboard-tokens.css` defines all `--variables`.
2. `ds-*.css` files consume those variables.
3. Page-specific CSS files should **only** override layout/positioning — never redefine token values.

---

## 12. CSS Naming Conventions

| Pattern | Example | Use |
|---------|---------|-----|
| `ds-[component]` | `.ds-card`, `.ds-btn` | Design system base class |
| `ds-[component]--[variant]` | `.ds-btn--primary`, `.ds-card--sm` | Variant modifier |
| `ds-[component]__[element]` | `.ds-card__header`, `.ds-modal__close` | Child element (BEM-style) |
| `ds-text-[scale]` | `.ds-text-xl`, `.ds-text-body` | Typography utility |
| `ds-text--[modifier]` | `.ds-text--muted`, `.ds-text--success` | Color/alignment modifier |
| `page-[name]-[element]` | `.page-header`, `.page-tabs` | Token-driven layout classes |
| `[page]-[element]` | `.wallet-header`, `.portfolio-main` | Legacy page-specific (being standardized) |
