# POOOL Public Landing Page & Marketplace Plan
**Status:** In Progress (Airbnb-Style Pivot)
**Architecture:** Vanilla HTML, CSS, JS with Jinja (SSR) & HTMX for interactivity.

## 1. Vision: The Airbnb Experience for RWAs
We are abandoning the traditional dense SaaS or Finance landing page layouts. Instead, we are pivoting to an ultra-clean, highly visual, trust-building experience inspired by **Airbnb**.

### Core Philosophy
- **Whitespace & Typography over Borders:** Let the property assets speak for themselves. Large, edge-to-edge imagery. Minimal borders (`border: none` or very soft `1px solid #E4E7EC`).
- **Immediate Value:** The user lands and immediately sees the properties (the marketplace grid), categorized visually with clear iconography.
- **Intuitive Discovery:** A powerful, sticky, pill-shaped search bar acting as the compass.
- **Seamless Conversion:** Trust is built visually. The transition from swiping through property images to signing up for checkout is frictionless.

---

## 2. Layout & UI Architecture (Single Page Focus)
We will build a single, unified `landing.html` consisting of the following sections:

### A. The "Smart" Header & Search (Sticky)
- **Top Row:** Minimalist header.
  - Left: POOOL Logo.
  - Right: "Globe" icon (Language), "Host/Invest" link, and a pill-shaped User menu (Hamburger + Avatar icon) that opens the login/signup drawer.
- **The "Pill" Search Bar:** Centered layout. Divided into segments: `Location` | `Asset Type` | `Target Yield` | `Search Button (Accent Green)`.
- **Category Icon Ribbon:** A horizontally scrollable row of icons right below the header: *🌴 Villas*, *🏖️ Beachfront*, *🏙️ Urban*, *☕ Commodities*.

### B. The Immersive Hero (Optional / Soft)
- Instead of a massive blue block, we use a stunning, screen-filling background image or video of a premium Bali location.
- **Content:** Minimalist overlay text: *"Invest in the extraordinary. From $500."*
- Unobtrusive, fading seamlessly into the white background of the grid below.

### C. The Visual Asset Grid (Marketplace)
- **Aspect Ratio & Images:** Square or 4:3 cards for properties. Large, high-resolution initial image with an embedded dot-indicator and tiny arrow controls for swiping through a gallery without leaving the grid.
- **Floating Badges:** A subtle heart icon (Save) on the top right.
- **Card Metadata (Below Image):** 
  - **Title:** `Bali, Indonesia` (Bold)
  - **Subtitle:** `Villa Akashi • 4 Beds` (Soft Gray)
  - **Financials:** `12.9% ROI`
  - **Price:** `$500 per share` (Bold, accent).
- No borders around the cards. Just clean spacing.

### D. The Experience / Trust Section
- A clean, horizontally scrolling section showing *"How it works"*: 1. Create Account, 2. Fund Wallet, 3. Own Shares.
- Testimonials presented as beautiful, soft cards with large quote marks.

---

## 3. Technical Execution Plan

### Phase 1: The UI Shell (HTML/CSS)
- [ ] Create `landing.html` in `frontend/platform/`.
- [ ] Build the sticky Airbnb-style header and the pill-shaped segmented search bar.
- [ ] Build the horizontal Category Ribbon (Icons + Text).
- [ ] Implement the new Property Card design:
  - Remove all heavy borders.
  - Implement an inner-card image carousel (CSS scroll-snap).
  - Clean typography for locations and ROI.

### Phase 2: HTMX & Interactivity
- [ ] Add filtering logic: Clicking a category in the ribbon updates the grid via `hx-get="/api/properties?category=..."`.
- [ ] Implement the User pill dropdown menu.

### Phase 3: Backend Integration (Cart & Checkout)
- [ ] Introduce Anonymous Sessions (`poool_guest_session` cookie).
- [ ] Build `public-asset.html`: A detailed view for a single asset, similar to an Airbnb listing page. Sticky booking/buying widget on the right.
- [ ] If user clicks "Buy Share" as guest -> add to anonymous cart -> redirect to Unified Signup/Login with Cart persistency.

## 4. Design System Updates Required
- **Fonts:** Keep TT Norms Pro.
- **Colors:** Dominant white background, stark black text for primary labels, soft gray (`#667085`) for secondary. Use POOOL Blue and Accent Green very sparingly for buttons and active states to maintain high contrast.
- **Radii:** Pill shapes (50px) for general buttons and search bars, soft rounded corners (16px) for property images.
