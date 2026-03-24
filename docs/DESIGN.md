# Design System: The Wealth Terminal — Holographic Edition

> **⚠️ SCOPE: This design system applies ONLY to the Investor Dashboard and the Developer Dashboard.**
> The Admin Dashboard has its own design language and is NOT governed by this document.
> Do NOT apply these styles, tokens, or component patterns to any page under `/admin/`.

---

## 1. Overview & Creative North Star

The Creative North Star for this design system is **"The Wealth Terminal — Holographic Edition."**

POOOL is a fractional real-world asset investment platform. The previous iteration of this design system used flat tonal layering and chromatic minimalism. This updated system evolves the original vision into a **premium Apple HIG-inspired holographic glassmorphism** language — frosted glass cards, physical-depth shadows, layered translucent icon systems, and sparkline data overlays — while maintaining the institutional restraint and data-forward clarity of the original Wealth Terminal concept.

The result feels like a Bloomberg terminal redesigned by Apple's Human Interface team: **frosted, luminous, physically grounded, and quietly authoritative.** Every card has mass. Every icon has optical depth. Every interaction is smooth, precise, and confident.

### Key Evolution Points
*   **Flat cards → Frosted glass cards** (`.holo-card`) with `backdrop-filter: blur(20px)` and inner glare simulation.
*   **Static icon boxes → Holographic icon system** (`.p-icon`) with layered rotated backgrounds, frosted glass fronts, and ambient glow.
*   **Single card border-radius → Larger, more physical radii** (`24px` for holographic cards, `12px` for inner containers).
*   **Design system colors remain identical** — the palette is unchanged; only the rendering technique evolved.

---

## 2. Colors: Chromatic Authority

The palette is deliberately restrained. Two high-saturation brand colors dominate an otherwise neutral canvas. **All color values are unchanged from the original system.**

### Brand Signature
*   **Electric Blue** (`--primary-color: #0000FF` / `--btn-primary-bg`) — The anchor color. Used for primary CTAs, active navigation states, focus rings, links, holographic icon strokes, and brand identity.
*   **Signal Green** (`--brand-greeny-green: #03FF88`) — The action color. Used for progress bars, growth indicators, active sidebar states, sparkline charts, and holographic icon background layers.
*   **Mint Green** (`--btn-primary-color: #98FB96`) — The button text color on primary buttons, and the secondary button fill.

### Surface Hierarchy — Holographic Layer Model
Treat the UI as **frosted glass panels floating above a canvas.** Depth is created through translucency, backdrop blur, inner glare, and physically-motivated shadows.

*   **Base Canvas:** `#FAFAFA` (`--content-bg`) — The workspace background.
*   **Holographic Card Surface:** `linear-gradient(160deg, rgba(255,255,255,0.95) 0%, rgba(255,255,255,0.75) 100%)` — Frosted glass. Not opaque white — translucent with a directional gradient.
*   **Flat Card Surface:** `#FFFFFF` (`--card-bg`) — Used for legacy cards that haven't been converted yet.
*   **Recessed Container:** `#F9FAFB` — Used inside cards for detail boxes, metrics grids, and code blocks.
*   **Hover Surface:** `#F8F9FC` (`--table-row-hover-bg`) — Subtle blue-tinted hover state for interactive rows.
*   **Sidebar:** `#FFFFFF` with `1px solid #E9EAEB` border-right.

### Text Hierarchy
*   **Primary Text:** `#181D27` (`--page-title-color`) — Near-black with a hint of navy. All headings and primary values.
*   **Body Text:** `#344054` (`--body-color`) — Warm dark grey for all body copy.
*   **Secondary Text:** `#535862` (`--text-secondary`) — Muted grey for timestamps and metadata.
*   **Tertiary/Caption:** `#667085` — Subdued grey for captions, hints, and placeholders.
*   **Label Text:** `#475467` (`--label-color`) — Mid-grey for form labels, subtitles, supporting descriptions.
*   **Table Headers:** `#717680` (`--table-header-color`) — Uppercase, tracked-out, deliberately understated.
*   **Empty State:** `#98A2B3` — Italic, for "Not provided" placeholder values.

### Borders & Dividers
*   **Holographic Card Border:** `1px solid rgba(0, 0, 0, 0.03)` — Nearly invisible. The frosted glass surface provides its own definition.
*   **Flat Card Border:** `1px solid #E5E7EB` (`--card-border-color`) — For legacy cards.
*   **Table/Divider Border:** `1px solid #E9EAEB` (`--table-border-color`).
*   **Settings Divider:** `1px solid rgba(0,0,0,0.06)` — Slightly lighter than card borders, used inside holographic settings cards.
*   **Input Border:** `1px solid #D0D5DD` (`--input-border-color`).

### Semantic Colors
*   **Success:** `#027A48` on `#ECFDF3`
*   **Warning:** `#B54708` on `#FFFAEB`
*   **Danger:** `#B42318` / `#D92D20` on `#FEF3F2`
*   **Info:** `#1B2559` on `#EEF4FF`

### The "No Raw Black" Rule
**`#000000` is strictly prohibited** for text on the platform dashboard. Use `#181D27` (headings) or `#344054` (body). The only place `#000000` may appear is in the brand logo SVG.

---

## 3. Typography: Single-Font Authority

POOOL uses a **single typeface** for everything. This creates an unmistakable identity.

### The Typeface: TT Norms Pro
A modern geometric sans-serif loaded in four weights:
*   **Regular (400)** — Body text, descriptions, form inputs
*   **Medium (500)** — Labels, badge text, tab inactive state, form labels, metadata
*   **Bold (700)** — Page titles, section titles, card titles, stat values, display numbers
*   **ExtraBold (800)** — Reserved for marketing/hero moments (rarely used in-app)

### Type Scale

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|-------|------|--------|-------------|----------------|-----|
| `ds-text-display` | 36px | 700 | 1.2 | -0.02em | Hero numbers, portfolio balances |
| `ds-text-xl` | 36px | 700 | 1.2 | -0.02em | Page titles (H1) |
| `ds-text-lg` | 24px | 700 | 32px | — | Section titles (H2) |
| `ds-text-md` | 18px | 700 | 28px | — | Card titles, modal titles, settings card headings |
| `ds-text-sm-heading` | 16px | 600–700 | 24px | — | Sub-section headings |
| `ds-text-body-lg` | 16px | 400 | 24px | — | Prominent body copy |
| `ds-text-body` | 14px | 400–500 | 20px | — | Default body text |
| `ds-text-subtitle` | 15px | 400 | 22px | — | Page subtitles, descriptions |
| `ds-text-caption` | 13px | 400–500 | 18px | — | Labels, hints, timestamps |
| `ds-text-xs` | 12px | 400–600 | 18px | — | Badges, fine print, table headers, metric labels |
| *stat-card-value* | 32px | 700 | 1.2 | -0.02em | Stat card hero numbers |
| *wallet-balance* | 48px | 700 | 1.2 | -0.02em | Wallet balance display |

### Financial Numbers
Use `ds-text-money` class. Applies `font-variant-numeric: tabular-nums` and `font-weight: 600` so dollar amounts align in columns. **All financial data must use this class.**

### The "Tight Headlines" Rule
All headings ≥ 20px must use `letter-spacing: -0.02em` or `-0.01em`. This creates the premium, "locked-in" typographic feel. Body text uses default tracking.

---

## 4. Elevation & Depth: Holographic Layering

The original POOOL system used flat tonal layering. The holographic system adds **physical depth through frosted glass, directional lighting, and layered shadows.**

### The Holographic Card (`.holo-card`)
The primary container for all new UI modules. Replaces `.ds-card` for dashboard content.

```css
.holo-card {
    background: linear-gradient(160deg, rgba(255,255,255,0.95) 0%, rgba(255,255,255,0.75) 100%);
    backdrop-filter: blur(20px) saturate(1.8);
    border: 1px solid rgba(0, 0, 0, 0.03);
    border-radius: 24px;
    padding: 28px;
    box-shadow:
        0 4px 6px -2px rgba(0,0,0,0.02),
        0 12px 32px -4px rgba(0,0,0,0.04),
        inset 0 1px 0 rgba(255,255,255,1),
        inset 0 -1px 2px rgba(0,0,0,0.01);
}
```

Key properties:
*   **Frosted glass:** `backdrop-filter: blur(20px) saturate(1.8)` gives the card physical mass and optical depth.
*   **Inner glare:** A `::before` pseudo-element adds a top-40% `linear-gradient(180deg, rgba(255,255,255,0.4) 0%, transparent 100%)` to simulate overhead lighting.
*   **Physical shadow stack:** Four-layer shadow — tiny contact shadow, medium depth shadow, bright inset highlight, and subtle inset bottom shadow.
*   **Hover:** `translateY(-3px) scale(1.003)` with elevated shadow. Clean physical lift only — **no colored glows on hover.**

### Shadow Scale

| Context | Shadow | Notes |
|---------|--------|-------|
| **Holo card (default)** | `0 4px 6px -2px rgba(0,0,0,0.02), 0 12px 32px -4px rgba(0,0,0,0.04), inset 0 1px 0 #fff, inset 0 -1px 2px rgba(0,0,0,0.01)` | Physical mass |
| **Holo card (hover)** | `0 12px 20px -4px rgba(0,0,0,0.04), 0 24px 48px -12px rgba(0,0,0,0.08), inset 0 1px 0 #fff, inset 0 -1px 2px rgba(0,0,0,0.01)` | Lifted state |
| **Settings card** | `0 1px 3px rgba(0,0,0,0.02)` | Subtle weight |
| **Modal** | `0px 8px 8px -4px rgba(16,24,40,0.03), 0px 20px 24px -4px rgba(16,24,40,0.08)` | Dramatic float |
| **Focus ring** | `0 0 0 3px rgba(0,102,255,0.15)` | Electric Blue glow for focused inputs |
| **Icon front face** | Multi-layer: contact, depth, blue-tinted ambient, inset highlights | See §5 |

### The "No Colored Glow" Rule for Cards
Card hover states must use **physically motivated neutral shadows only.** No `rgba(0,0,255,...)` or `rgba(3,255,136,...)` box-shadows on card containers. Colored ambient glow is reserved exclusively for the `.p-icon` system.

---

## 5. The Holographic Icon System (`.p-icon`)

The `.p-icon` system is the signature visual element of the platform. Each icon is a **three-dimensional frosted glass cube** with layered rotated backgrounds, a frosted glass front face, and an ambient glow.

### Anatomy

```
┌──────────────────────────────────┐
│  Ambient Glow (::before)         │   ← Blurred color haze behind icon
│  ┌─────────────────────────┐     │
│  │  Background Layer 1     │     │   ← Small rotated square (Signal Green gradient)
│  │    (rotate 18deg)       │     │
│  │  ┌──────────────────┐   │     │
│  │  │  Background Layer 2│  │     │   ← Larger rotated square (Electric Blue gradient)
│  │  │    (rotate 10deg)  │  │     │
│  │  │  ┌──────────────┐ │  │     │
│  │  │  │  Front Face   │ │  │     │   ← Frosted glass square with key light + noise grain
│  │  │  │  ┌──────┐     │ │  │     │
│  │  │  │  │ SVG  │     │ │  │     │   ← Icon stroke (Electric Blue #0000EE)
│  │  │  │  └──────┘     │ │  │     │
│  │  │  └──────────────┘ │  │     │
│  │  └──────────────────┘   │     │
│  └─────────────────────────┘     │
└──────────────────────────────────┘
```

### Size Variants

| Size | Class | Dimensions | SVG Size | Use |
|------|-------|-----------|----------|-----|
| **Default** | `.p-icon` | 56×56px | 22×22px | Standalone icons, page headers |
| **Medium** | `.p-icon--md` | 44×44px | 18×18px | Settings card icons, card headers |
| **Small** | `.p-icon--sm` | 36×36px | 15×15px | Completion badges, compact inline, action grid |
| **Extra Small** | `.p-icon--xs` | 32×32px | 13×13px | Activity list rows, transaction items |

### Color Styles

| Style | Class | BG-1 Gradient | BG-2 Gradient | Front Tint | SVG Stroke |
|-------|-------|--------------|--------------|-----------|-----------|
| **Blue** (default) | `.p-style-blue` | `#03FF88 → #00CC6F` | `#0000FF → #3344FF` | Cool blue-white | `#0000FF` |
| **Green** | `.p-style-green` | `#03FF88 → #00CC6F` | `#0000FF → #3344FF` | Warm green-white | `#00BB66` |
| **Purple** | `.p-style-purple` | `#8B5CF6 → #A78BFA` | `#A78BFA → #C4B5FD` | Lavender-white | `#6D28D9` |

### Front Face Construction
The frosted glass front is the most complex element:

```css
.p-front {
    background: linear-gradient(160deg,
        rgba(255,255,255,0.95) 0%,
        rgba(244,246,255,0.78) 40%,
        rgba(236,240,255,0.62) 100%);
    backdrop-filter: blur(20px) saturate(1.85);
    box-shadow:
        0 .5px .5px rgba(0,0,0,0.04),        /* Contact shadow */
        0 1px 2px rgba(0,0,0,0.04),            /* Depth shadow */
        0 4px 12px rgba(0,0,255,0.04),         /* Blue ambient */
        0 8px 24px rgba(0,0,255,0.025),        /* Deep blue ambient */
        inset 0 .5px 0 rgba(255,255,255,0.95), /* Top edge highlight */
        inset 0 0 0 .5px rgba(255,255,255,0.7),/* Border highlight */
        inset 0 -1px 3px rgba(180,195,255,0.08);/* Bottom edge shadow */
}
```

Additional layers:
*   **`::before` (Key light):** `radial-gradient(ellipse at 30% 20%, rgba(255,255,255,0.22) → transparent)` — overhead light simulation.
*   **`::after` (Noise grain):** Fractal noise SVG at `opacity: 0.015` with `mix-blend-mode: soft-light` — adds physical material texture.

### Interactive Behavior
*   **List item hover:** Icons morph from Blue to Green style on row hover (activity lists).
*   **Card icon:** Static — no hover morph on card-level icons.
*   **Transition:** `transform 0.35s cubic-bezier(0.4, 0, 0.2, 1)`.

### HTML Structure
```html
<div class="p-icon p-style-blue">
    <div class="p-bg p-bg-1"></div>
    <div class="p-bg p-bg-2"></div>
    <div class="p-front">
        <svg viewBox="0 0 24 24"><!-- icon path --></svg>
    </div>
</div>
```

---

## 6. Spacing: The 8px Grid

All spacing values are multiples of 8px. Unchanged from the original system.

| Token | Value | Use |
|-------|-------|-----|
| `--page-padding-top` | 48px | Top padding on every page |
| `--page-padding-bottom` | 48px | Bottom padding on every page |
| `--page-padding-x` | 32px | Horizontal padding on page content |
| `--section-gap` | 24px | Gap between major sections and cards |
| `--card-padding` (holo) | 28px | Internal padding of holographic cards |
| `--card-padding` (flat) | 24px | Internal padding of legacy flat cards |
| `--card-gap-horizontal` | 24px | Horizontal gap between side-by-side cards |
| `--page-title-icon-gap` | 12px | Gap between page icon and title |
| `--tab-gap` | 32px | Gap between navigation tabs |
| `--btn-icon-gap` | 8px | Gap between icon and text in buttons |

### Settings Card Layout
Settings cards use a **flex row with icon + content** pattern:
*   **Icon-to-content gap:** `24px`
*   **Content padding:** `28px` (inherited from holo-card)
*   **Full-bleed dividers:** `margin: Xpx -28px` (negative margin to span full card width)

### The "Content Width" Rule
*   **Sidebar width:** `256px` — Fixed above 768px.
*   **Page max-width:** `1200px`.
*   **Main content width:** `calc(100% - 256px)`.

---

## 7. Components: Holographic Primitives

### Page Layout
Every dashboard page uses the same skeleton:
```html
<main class="ds-main dashboard-content">
    <div class="page-header">
        <div class="page-header-left">
            <div class="page-header-icon" style="transform: scale(1.1); margin-right: 12px;">
                <div class="p-icon p-style-blue">
                    <div class="p-bg p-bg-1"></div>
                    <div class="p-bg p-bg-2"></div>
                    <div class="p-front">
                        <svg viewBox="0 0 24 24"><!-- icon --></svg>
                    </div>
                </div>
            </div>
            <div>
                <h1 class="page-title">Page Title</h1>
            </div>
        </div>
        <div class="page-header-actions">
            <button class="ds-btn ds-btn--primary">Action</button>
        </div>
    </div>
    <p class="page-subtitle">Page description text.</p>
    <!-- page content -->
</main>
```

### Holographic Cards (`.ds-card.holo-card`)
The primary card component for all new UI work.

*   **Background:** `linear-gradient(160deg, rgba(255,255,255,0.95), rgba(255,255,255,0.75))`
*   **Border:** `1px solid rgba(0,0,0,0.03)`
*   **Radius:** `24px`
*   **Padding:** `28px`
*   **Backdrop:** `blur(20px) saturate(1.8)`
*   **Inner glare:** `::before` with top-40% white gradient
*   **Hover:** `translateY(-3px) scale(1.003)` with elevated neutral shadows
*   **Content z-index:** All direct children get `position: relative; z-index: 1` to render above the glare layer.

### Settings Cards (`.ds-card.holo-card.settings-card`)
Extended holographic cards for settings/profile pages. Use a **flex row layout** with icon + content:

```html
<div class="ds-card holo-card settings-card">
    <div style="display:flex; gap:24px; align-items:flex-start;">
        <div class="p-icon p-icon--md"><!-- holographic icon --></div>
        <div style="flex:1;">
            <h2>Section Title</h2>
            <p>Description text.</p>
            <!-- form fields, read-only rows, etc. -->
        </div>
    </div>
</div>
```

### Stat Cards
Holographic cards with a sparkline chart overlay:

*   **Header:** Flex row — `.p-icon` left, optional `.ds-badge` right.
*   **Body:** Label (14px/500 `--label-color`) + Value (32px/700 `--page-title-color`, `letter-spacing: -0.02em`).
*   **Sparkline:** Absolutely positioned SVG at bottom of card, `height: 72px`, `z-index: 0`, `border-radius: 0 0 24px 24px`.
    *   Green sparkline: `stroke: var(--brand-greeny-green)` with `rgba(3,255,136,0.15)` gradient fill.
    *   Blue sparkline: `stroke: var(--btn-primary-bg)` with `rgba(0,0,255,0.12)` gradient fill.

### Property Investment Cards
Holographic cards with full-bleed image carousel:

*   **Image container:** `margin: -28px -28px 24px -28px`, `height: 220px`, `border-radius: 23px 23px 0 0`.
*   **Glass badge:** `background: rgba(255,255,255,0.95)`, `backdrop-filter: blur(12px)`, `border-radius: 20px`, positioned top-left.
*   **Carousel controls:** Frosted glass circles (`rgba(255,255,255,0.15)` + `backdrop-filter: blur(8px)`), opacity: 0 → 1 on container hover.
*   **Carousel dots:** Apple-style active indicator — `6px` circles, active dot stretches to `16px` pill with opaque white.
*   **Funding progress bar:** `6px` track, `var(--brand-greeny-green)` fill, `border-radius: 4px`.
*   **Details box:** `#F9FAFB` background, `border-radius: 12px`, `1px solid rgba(0,0,0,0.04)`, inner padding `12px 16px`.

### Buttons (`.ds-btn`)
Four variants, three sizes — unchanged from the original system.

| Variant | Background | Text | Hover |
|---------|-----------|------|-------|
| **Primary** | `#0000FF` | `#98FB96` (or `#FFFFFF` for action buttons) | `#0000CC` |
| **Secondary** | `#98FB96` | `#0000FF` | `#7BE079` |
| **Danger** | `#D92D20` | `#FFFFFF` | `#B42318` |
| **Ghost** | transparent | `#475467` | `#F2F4F7` bg |

| Size | Height | Padding | Font |
|------|--------|---------|------|
| **Small** | 32px | 6px 12px | 13px |
| **Medium** | 40px | 10px 18px | 14px |
| **Large** | 48px | 12px 24px | 16px |

All buttons: `border-radius: 8px`, `font-weight: 600`.

> **Note:** Settings/dashboard action buttons commonly use `background: var(--primary-color); color: #FFFFFF` instead of the mint green text. This is acceptable for context-specific CTAs where white text provides better readability.

### Toggle Switches (`.ds-switch`)
Apple-style toggle switches for boolean settings:

*   **Track:** `44×24px`, `border-radius: 20px`, `background: #EAECF0` (inactive) / `var(--system-blue-light, #0066FF)` (active).
*   **Thumb:** `20×20px` white circle with micro-shadow, `transform: translateX(20px)` when active.
*   **Transition:** `0.3s cubic-bezier(0.2, 0.8, 0.2, 1)`.
*   **Interaction:** `data-state="active|inactive"` toggled via JS `onclick`.

### Badges (`.ds-badge`)
Pill-shaped status indicators — unchanged.

| Variant | Background | Text |
|---------|-----------|------|
| **Success** | `#ECFDF3` | `#027A48` |
| **Warning** | `#FFFAEB` | `#B54708` |
| **Danger** | `#FEF3F2` | `#B42318` |
| **Info** | `#EEF4FF` | `#1B2559` |
| **Neutral** | `#F2F4F7` | `#344054` |

### Completion Badges
Blue-tinted pills for profile completion checklists:
*   `background: #EEF4FF`, `color: var(--primary-color)`, `border: 1px solid rgba(0,0,255,0.08)`, `border-radius: 100px`.
*   Include a checkmark SVG icon with `6px` gap.

### Navigation Tabs (`.page-tabs`)
*   **Inactive:** `15px`, weight `400`, color `#667085`.
*   **Active:** weight `600`, color `#1B2559`, with `2px solid #1B2559` bottom border.
*   **Gap:** `32px` between tabs.

### Form Inputs (`.settings-input`)
*   **Max width:** `440px` (pushed right via `margin-left: auto` in form rows).
*   **Padding:** `10px 14px`
*   **Border:** `1px solid #D0D5DD`
*   **Radius:** `8px`
*   **Focus:** Border becomes `var(--system-blue-light, #0066FF)` with `0 0 0 3px rgba(0,102,255,0.15)` glow.
*   **Disabled:** `#F9FAFB` background, `#717680` text, `cursor: not-allowed`.

### Activity Lists
*   **Item:** Flex row — `.p-icon` (36px) + details column + amount.
*   **Padding:** `16px 24px` per item.
*   **Separator:** `1px solid var(--card-border-color)`.
*   **Hover:** Gradient highlight `linear-gradient(90deg, transparent → rgba(2,122,72,0.02) → transparent)` + icon morphs to green style.
*   **Amount colors:** `.positive` = `#027A48`, `.negative` = `var(--page-title-color)`.
*   **Load more:** Full-width ghost button in card footer, separated by `1px solid` border-top.

### Modals (`.ds-modal`)
*   **Border Radius:** `16px`.
*   **Max Width:** `480px` (default), `360px` (sm), `640px` (lg), `800px` (xl).
*   **Animation:** Slide-in from `translateY(-12px) scale(0.97)`, `0.25s ease-out`.
*   **Overlay:** `rgba(52, 64, 84, 0.6)` with `backdrop-filter: blur(6px)`.
*   **Mobile:** Bottom-sheet style — `border-radius: 16px 16px 0 0`.

### Tables (`.ds-table`)
*   **Header:** Uppercase, `12px`, `font-weight: 600`, `#717680`, `letter-spacing: 0.04em`.
*   **Cells:** `14px`, `#535862`, `padding: 16px 24px`.
*   **Row hover:** `#F8F9FC`.

### Progress Bars
*   **Track:** `#D5D7DA` (generic) / `#E9EAEB` (property) / `#EAECF0` (completion).
*   **Fill:** `var(--brand-greeny-green, #03FF88)`.
*   **Completion fill:** `linear-gradient(90deg, #34D399, #03FF88)` with `box-shadow: 0 0 10px rgba(3,255,136,0.3)`.
*   **Heights:** `6px` (sm/property), `8px` (default).

### Sidebar Navigation
*   **Width:** `256px` fixed.
*   **Background:** `#FFFFFF` with `1px solid #E9EAEB` right border.
*   **Active state:** `rgba(3, 255, 136, 0.15)` background.
*   **Active icon:** CSS filter tuned to Electric Blue.

---

## 8. Animations & Transitions

### Interaction Timings

| Element | Property | Duration | Easing |
|---------|----------|----------|--------|
| **Holographic card hover** | `all` | `0.5s` | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| **Button hover** | `all` | `0.15s` | `ease` |
| **P-icon transform** | `transform` | `0.35s` | `cubic-bezier(0.4, 0, 0.2, 1)` |
| **P-icon BG layers** | `all` | `0.4s` | `cubic-bezier(0.4, 0, 0.2, 1)` |
| **Activity row hover** | `background` | `0.25s` | `ease` |
| **List icon hover morph (SVG)** | `all` | `0.35s` | `cubic-bezier(0.4, 0, 0.2, 1)` |
| **Input focus** | `all` | `0.3s` | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| **Toggle switch** | `all, transform` | `0.3s` | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| **Completion progress fill** | `width` | `1s` | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| **Modal entry** | slide + scale | `0.25s` | `ease-out` |
| **Page transition (main)** | fade + translateY | `280ms in/220ms out` | `cubic-bezier(0.4, 0, 0.2, 1)` |
| **Image hover (property)** | `transform` | `0.6s` | `cubic-bezier(0.2, 0.8, 0.2, 1)` |
| **Carousel controls** | `all` | `0.3s` | `ease` |
| **Nav item hover** | `all` | `0.2s` | `ease` |

### The "No Bounce" Rule
**Spring/bounce easing is prohibited.** All animations use `ease`, `ease-out`, or `cubic-bezier(...)` curves. Overshooting animations undermine the institutional tone.

### FOUC Prevention
*   `body.fouc-guard:not(.dom-ready)` sets `opacity: 0; visibility: hidden`.
*   `body.dom-ready` reveals with `opacity 0.15s ease-in`.

---

## 9. Responsive Design

### Breakpoints

| Breakpoint | Width | Behavior |
|-----------|-------|----------|
| **Desktop** | > 1024px | Full sidebar + content |
| **Tablet** | 768px–1024px | Stat grid → 2-column, sidebar narrows |
| **Mobile** | ≤ 768px | Sidebar collapses, all grids → 1-column, content full-width |

### Mobile Adaptations
*   **Sidebar:** Relative, full-width. Border switches from right to bottom.
*   **Page padding:** `16px` horizontal, `80px` top.
*   **Page title:** Drops from `36px` to `28px`.
*   **Grid layouts:** All collapse from multi-column to single-column.
*   **Modal:** Bottom-sheet style.
*   **Stat cards:** Grid goes from `repeat(3, 1fr)` → `repeat(2, 1fr)` → `1fr`.
*   **Property cards:** Grid goes from `repeat(3, 1fr)` → `1fr`.
*   **Settings metrics grid:** `repeat(4, 1fr)` → stack vertically.

---

## 10. Z-Index Scale

| Level | Z-Index | Element |
|-------|---------|---------| 
| Base | 0 | Default content, sparkline charts |
| Glare layer | 0 | `.holo-card::before` inner glare |
| Content | 1 | All direct children of `.holo-card` |
| Icon BG layers | 1–2 | `.p-bg-1`, `.p-bg-2` |
| Icon front face | 3 | `.p-front` |
| Icon key light | 5 | `.p-front::before` |
| Icon noise grain | 6 | `.p-front::after` |
| Icon SVG | 7 | `.p-front svg` |
| Carousel controls | 10 | `.carousel-btn`, `.carousel-dots` |
| Sticky headers | 20 | Sticky table headers |
| Navigation | 30 | Secondary overlays |
| Sidebar | 100 / 1000 | Main sidebar navigation |
| Modals | 10000 | Modal overlays |

---

## 11. Do's and Don'ts

### Do
*   **Do** use `.holo-card` for all new card components. The flat `.ds-card` is deprecated for new work.
*   **Do** use the `.p-icon` system for all card icons, page header icons, and list item icons. It is the visual signature of the platform.
*   **Do** use CSS custom properties from `dashboard-tokens.css` for all shared values.
*   **Do** create depth through frosted glass (`backdrop-filter: blur(20px)`) and physical shadow stacks.
*   **Do** use `font-variant-numeric: tabular-nums` (via `.ds-text-money`) for all financial numbers.
*   **Do** use `cubic-bezier(0.2, 0.8, 0.2, 1)` or `cubic-bezier(0.4, 0, 0.2, 1)` for smooth, premium animations.
*   **Do** use the Electric Blue + Signal Green brand pairing for primary actions.
*   **Do** respect `prefers-reduced-motion`.
*   **Do** use `border-radius: 24px` for holographic cards and `12px` for inner containers/detail boxes.
*   **Do** give holographic card content `position: relative; z-index: 1` so it renders above the glare.

### Don't
*   **Don't** use `#000000` for text. Use `#181D27` (headings) or `#344054` (body).
*   **Don't** add colored `box-shadow` glows to holographic cards. Colored ambient glow is exclusively for `.p-icon`.
*   **Don't** use colors outside the token system. Every hex value should trace back to a `--variable`.
*   **Don't** use more than one typeface. TT Norms Pro handles everything.
*   **Don't** use `!important` except in `dashboard-tokens.css` universal standardization rules and `.holo-card` overrides of `.ds-card`.
*   **Don't** use border-radius values other than `8px` (buttons/inputs), `12px` (inner containers/legacy cards), `16px` (modals), `20px` (learn cards), `24px` (holographic cards). Five radii, that's it.
*   **Don't** stack multiple unrelated shadows. Use the defined shadow tokens.
*   **Don't** use bounce/spring easing. The platform should feel assured, not playful.
*   **Don't** add CSS framework dependencies (Tailwind, Bootstrap).
*   **Don't** place business logic in the frontend.
*   **Don't** use flat `.ds-card` for new components — always use `.ds-card.holo-card`.
*   **Don't** hardcode icon styles inline when a `.p-style-*` class exists for the color variant.

---

## 12. File Architecture

```
frontend/platform/static/css/
├── bundle.css                    ← Auto-generated, do not edit
├── fonts.css                     ← @font-face declarations for TT Norms Pro
├── dashboard-tokens.css          ← ALL design tokens (single source of truth)
├── main.css                      ← Base reset, app layout, utilities
├── ds-page-layout.css            ← .ds-main, .ds-page-header
├── ds-typography.css             ← .ds-text-* classes
├── ds-buttons.css                ← .ds-btn variants and sizes
├── ds-cards.css                  ← .ds-card base (being superseded by holo-card)
├── ds-forms.css                  ← .ds-input, .ds-select, .ds-textarea
├── ds-modals.css                 ← .ds-modal overlay and container
├── ds-badges.css                 ← .ds-badge status pills
├── ds-tables.css                 ← .ds-table and flex-based tables
├── ds-progress.css               ← .ds-progress bar variants
├── ds-utilities.css              ← .ds-flex, .ds-mt-*, .ds-hidden, etc.
├── sidebar-navigation.css        ← Sidebar nav component
├── cards-template.css            ← Holographic card system, p-icon, settings cards
├── forms-template.css            ← Holographic form components
├── table-template.css            ← Holographic table components
├── statistics-template.css       ← Holographic stats/chart components
├── overlays-template.css         ← Holographic overlay components
└── [page-name].css               ← Page-specific overrides
```

### The Token + Template Cascade
1. `dashboard-tokens.css` defines all `--variables`.
2. `cards-template.css` defines the holographic card system (`.holo-card`, `.p-icon`, sparklines, settings cards).
3. Other `*-template.css` files define domain-specific holographic components.
4. `ds-*.css` files remain for base components; `.holo-card` overrides `.ds-card` with `!important`.
5. Page-specific CSS files should **only** override layout/positioning — never redefine token values or holographic card styles.

---

## 13. CSS Naming Conventions

| Pattern | Example | Use |
|---------|---------|-----|
| `ds-[component]` | `.ds-card`, `.ds-btn` | Design system base class |
| `ds-[component]--[variant]` | `.ds-btn--primary`, `.ds-card--sm` | Variant modifier |
| `ds-[component]__[element]` | `.ds-card__header`, `.ds-modal__close` | Child element (BEM-style) |
| `holo-card` | `.holo-card` | Holographic card modifier |
| `p-icon` | `.p-icon` | Holographic icon base |
| `p-icon--[size]` | `.p-icon--md`, `.p-icon--sm`, `.p-icon--xs` | Icon size variant |
| `p-style-[color]` | `.p-style-blue`, `.p-style-green` | Icon color style |
| `p-bg`, `p-bg-1`, `p-bg-2` | `.p-bg-1` | Icon background layers |
| `p-front` | `.p-front` | Icon front face |
| `ds-text-[scale]` | `.ds-text-xl`, `.ds-text-body` | Typography utility |
| `ds-switch` | `.ds-switch` | Toggle switch component |
| `settings-[element]` | `.settings-card`, `.settings-input` | Settings page components |
| `page-[element]` | `.page-header`, `.page-tabs` | Layout components |

---

## 14. Template Reference Pages

The following template pages serve as the living design reference. When building new features, **refer to these pages first** to ensure consistency:

| Template | URL | Contains |
|----------|-----|----------|
| **Cards** | `/cards-template.html` | Stat cards, wallet cards, profile cards, activity lists, property cards, settings cards, completion banners, social links, developer profile |
| **Forms** | `/forms-template.html` | All form input types, validation states, file uploads |
| **Tables** | `/table-template.html` | Data tables, sortable columns, pagination |
| **Statistics** | `/statistics-template.html` | Chart cards, KPI grids, trend indicators |
| **Overlays** | `/overlays-template.html` | Modals, toasts, confirmation dialogs |
| **Fonts** | `/fonts-template.html` | Typography specimens, weight comparison |
