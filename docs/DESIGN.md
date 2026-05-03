# POOOL Design System

> Canonical design reference for POOOL frontend and UI work.
>
> Sources used:
> - `docs/brand/01_POOOL brand book (2).pdf`
> - `docs/design/FRONTEND_COMPONENTS.md`
> - `frontend/platform/static/css/dashboard-tokens.css`
> - `frontend/platform/static/css/ds-*.css`
> - Existing platform UI patterns

This document translates the brand book and current platform CSS into practical rules for investor dashboard UI, developer dashboard UI, shared product components, audits, and automated fixes.

Scope note: this file is the canonical design reference for the investor dashboard and developer dashboard. Admin dashboard design may reuse tokens/components, but admin-specific UI decisions are outside the scope of this file unless explicitly stated elsewhere.

## 1. Brand Position

POOOL is a fractional ownership and investment platform. The design must feel:

- Trustworthy and secure
- Clear about money, ownership, and risk
- Modern and premium, without being decorative at the cost of usability
- Community-oriented, built around shared ownership
- Direct and simple: the product should make investment actions easy to understand

The brand concept is co-ownership made simple. The overlapping OOO in the logo and the repeated use of circles express people pooling capital and owning together.

## 2. Design Priorities By Surface

### Investor And Developer Dashboards

Product surfaces are tools. They must prioritize clarity, scanability, correctness, and safe action.

Use:

- Dense but organized information
- Clear hierarchy
- Conservative spacing
- Explicit states
- Real data and useful empty states
- Tables, forms, filters, tabs, and modals that behave predictably

Avoid:

- Marketing-style hero sections inside dashboard tools
- Decorative gradients or background effects that reduce readability
- Oversized typography inside compact UI panels
- Visual treatments that make financial state ambiguous
- Mixing multiple visual systems on one page, for example `ds-*` cards beside visually similar but separately defined custom cards and bespoke filter buttons

### Investor Dashboard Web UI

The investor dashboard is a web application, not a brand flyer. The brand book informs color, typography, logo, and identity, but the dashboard must use a stricter product UI system.

Investor dashboard pages include, but are not limited to:

- `frontend/platform/portfolio.html`
- `frontend/platform/wallet.html`
- `frontend/platform/rewards-v2.html`
- `frontend/platform/marketplace*.html`
- `frontend/platform/my-trading.html`
- `frontend/platform/transactions.html`
- `frontend/platform/checkout.html`
- Shared investor shell components such as `frontend/platform/components/sidebar.html` and `frontend/platform/components/investor-topbar.html`

The dashboard standard is:

- One page shell
- One topbar system
- One sidebar system
- One card system
- One button system
- One form system
- One table system
- One status/badge system

The current preferred component foundation is the `ds-*` system backed by `dashboard-tokens.css`. Page-specific classes may handle layout and data-specific composition, but they should not redefine the visual language when a `ds-*` primitive already exists.

Examples:

- Good: `class="ds-card wallet-balance-card"` where `wallet-balance-card` only controls layout/content arrangement.
- Good: `class="ds-btn ds-btn--primary"` for primary actions.
- Good: `class="ds-table-container portfolio-assets-table"` where page CSS defines columns, not a new table look.
- Avoid: separate `portfolio-show-more-btn`, `rv2-history-filter-btn`, or custom CTA classes that visually duplicate button behavior without using `ds-btn`.
- Avoid: using several custom card/icon/button systems that visually compete with `ds-card`/`ds-btn`, such as legacy `holo-card` references, `glass-icon-*`, or large illustrated hero panels in the same dashboard workflow.

Dashboard pages should feel like one application even when the content differs. Portfolio, wallet, rewards, trading, and checkout may have different information architecture, but their controls, surfaces, typography, spacing, and state patterns should remain consistent.

Developer dashboard pages should follow the same base card, button, typography, spacing, icon, and state rules as the investor dashboard unless the developer workflow requires a denser operational layout.

### Preferred Investor Dashboard Card Style

The preferred investor-dashboard card style is the card language already used in the wallet and portfolio empty states:

- White surface
- 12px border radius
- Light neutral border: `#E5E7EB` or `--card-border-color`
- Subtle shadow: `--card-shadow`
- 18-24px internal padding depending on density
- TT Norms Pro typography
- Muted uppercase micro-labels where useful
- Optional small Electric Blue to Greeny Green top accent for preview/metric cards
- Optional very subtle brand watermark or radial accent only for empty/onboarding states
- Hover may add a small border/shadow lift, but should not feel animated or decorative

Exact default dashboard card spec:

- Background: `#FFFFFF` / `--card-bg`
- Border: `1px solid var(--card-border-color, #E5E7EB)`
- Border radius: `12px`
- Shadow: `var(--card-shadow, 0 1px 2px rgba(10, 13, 18, 0.05))`
- Padding compact: `18px`
- Padding default: `20px`
- Padding spacious/summary: `24px`
- Non-interactive cards: no hover transform and no hover color change
- Interactive cards: optional `translateY(-1px)` with slightly stronger neutral shadow; no dramatic scale, glass, or color wash
- Accent strip: 3px top accent using Electric Blue to Greeny Green, applied only through a shared class/token

Use this style for investor dashboard cards such as:

- Wallet balance cards
- Portfolio value and metric cards
- Rewards stat/benefit cards
- Trading summary cards
- Checkout summary cards
- Payment method cards
- Empty-state metric, step, and trust cards
- Support/settings summary cards

Do not use glass, holographic, heavy gradient, or highly decorative card treatments for normal dashboard cards.

Brand accent rule: the small Electric Blue to Greeny Green accent is a standard card accent when it can be applied through one shared class/token without touching each card manually. Prefer a reusable primitive such as a shared accent modifier over page-specific copies.

Exception: property, asset, and commodity cards keep their richer media-card style because they sell and explain investable assets. Property and commodity cards need photography, funding status, price, yield, location, progress, and asset-specific metadata, so they follow the property/commodity-card standard rather than the generic dashboard-card standard.

Canonical property/commodity card surfaces are used on:

- Main landing page property cards
- `/developer/assets`
- `/marketplace`
- `/marketplace-secondary`
- `/commodities-marketplace`

Canonical implementation files:

- `frontend/platform/static/css/property-card.css`
- `frontend/platform/static/js/property-card.js`
- Markup using `.property-card` and related property-card child classes

Commodity cards should use the same visual card language as property cards, with content adjusted for commodity-specific metadata.

### Preferred Investor Dashboard Button Style

The preferred investor-dashboard button style is the button language used in the wallet and portfolio empty states:

- Base class: `.ds-btn`
- Primary: `.ds-btn ds-btn--primary`
- Primary buttons may come in multiple sizes: small, medium, large, full-width, and icon variants.
- Secondary: neutral/simple button style, not green and not blue-accented
- Minimum height: 32px for compact actions, 40px for normal actions, 44-48px for prominent CTAs
- Radius: `--btn-border-radius`
- Primary color pair: Electric Blue background with mint/green text
- Secondary action: white or very light neutral surface, neutral border, dark/muted text
- Secondary hover: no color change and no blue hover border; only subtle non-color feedback is allowed if necessary, such as cursor/focus outline required for accessibility
- Hover lift must be subtle, usually `translateY(-1px)` and a small blue-tinted shadow for prominent CTAs only

Exact target secondary button spec:

- Background: `#FFFFFF`
- Text: `#414651` or `--body-color`
- Border: `1px solid #D5D7DA` or `--input-border-color`
- Border radius: `8px`
- Font: TT Norms Pro, 600
- Height small: `32px`
- Height medium: `40px`
- Height large/CTA: `44px`
- Padding small: `6px 12px`
- Padding medium: `10px 16px`
- Padding large/CTA: `10px 20px`
- Hover: no background color change, no text color change, no blue border
- Active: no color change; optional tiny press transform if already used by the shared button primitive
- Focus-visible: accessible outline is required and may use Electric Blue for accessibility only
- Disabled: opacity around `0.5`, `cursor: not-allowed`, no hover/active feedback

Use this style for:

- Primary page actions
- Empty-state CTAs
- Retry actions
- Copy/export actions
- Show more/show less actions
- Filter buttons that behave like actions

Do not create a new button visual style per page.

### Marketing, Public Landing, Campaigns

Brand surfaces may be more expressive.

Use:

- Large confident typography
- Real product/property/community imagery
- Circle and Triple-O graphic language
- Strong brand color pairings
- Simple slogans and direct calls to action

Avoid:

- Weak contrast
- Stock-like imagery that does not explain the offer
- Decorative type/image combinations where neither element has clear priority

## 3. Logo

The POOOL logo is a primary brand asset. It should be treated as a fixed mark, not as a flexible graphic.

### Approved Logo Colors

The brand book permits logo usage in:

- Electric Blue
- Greeny Green
- Soft Blue
- Bright Lime

Logo color must be chosen for maximum contrast:

- Light backgrounds: use Electric Blue or Greeny Green.
- Dark backgrounds: use Soft Blue or Bright Lime.
- Photography: prefer Soft Blue or Bright Lime, provided the mark stays legible.
- Do not use secondary palette colors for the logo.

### Logo Misuse

Do not:

- Add gradients to the logo
- Make the logo 3D
- Stretch or distort the logo
- Add strokes
- Fill the internal counterforms
- Add extra spacing between the logo letters
- Use multiple colors inside one logo instance

### Logo Space And Size

- Keep clear space around the logo equal to the brand-book spacing system based on the logo proportions.
- Do not place text, icons, borders, cards, or imagery too close to the mark.
- Minimum logo size from the brand book: 30px for small digital usage.
- The favicon is the compact Triple-O mark and should remain legible at small sizes.

### Co-Branding

When pairing POOOL with a partner logo:

- Separate the logos with clear space based on the POOOL logo width.
- Partner logo height should not exceed the height of the POOOL symbol, except for minor overhanging details.
- Keep the POOOL mark visually primary or equal, never subordinate without explicit brand approval.

## 4. Color System

### Brand Book Palette

| Token | Use | HEX | RGB |
|-------|-----|-----|-----|
| Electric Blue / Deep Blue | Primary accent, core brand action color | `#0000FF` | 0, 0, 255 |
| Bright Lime | Primary base/accent pairing, energetic highlight | `#DBE2E9` in PDF text extraction; see note below | 219, 226, 233 |
| Soft Blue | Complimentary accent, dark/photo backgrounds | `#03FF88` in PDF text extraction; see note below | 3, 255, 136 |
| Greeny Green | Secondary/brand accent | `#9BABB8` in PDF text extraction; see note below | 155, 171, 184 |
| Dark Blue | Secondary dark foundation | `#08232F` | 8, 35, 47 |
| White | Neutral surface/text contrast | `#FFFFFF` | 255, 255, 255 |

Note: the brand PDF text extraction appears to mix some color names and values on the palette page. Existing platform CSS uses the following practical mapping:

- Electric Blue: `#0000FF`
- Bright Lime / brand green: `#98FB96` in dashboard button tokens and progress tokens
- Greeny Green / vivid green: `#03FF88` in badge/funding tokens
- Soft neutral blue/gray: `#DBE2E9` or `#9BABB8`
- Dark Blue: `#08232F`

When implementing product UI, prefer the existing CSS tokens unless the brand source file is corrected.

### Product UI Tokens

Core product tokens currently live in:

- `frontend/platform/static/css/dashboard-tokens.css`
- `frontend/platform/static/css/ds-buttons.css`
- `frontend/platform/static/css/ds-forms.css`
- `frontend/platform/static/css/ds-cards.css`
- `frontend/platform/static/css/ds-tables.css`
- `frontend/platform/static/css/ds-modals.css`
- `frontend/platform/static/css/ds-badges.css`
- `frontend/platform/static/css/ds-typography.css`

Important current tokens:

| Purpose | Token / Value |
|---------|---------------|
| Primary action background | `--btn-primary-bg: #0000FF` |
| Primary action text | `--btn-primary-color: #98FB96` |
| Secondary action background | Current legacy token: `--btn-secondary-bg: #98FB96`; target dashboard style: neutral/white |
| Secondary action text | Current legacy token: `--btn-secondary-color: #0000FF`; target dashboard style: dark/muted neutral |
| Page background | `--content-bg: #FAFAFA` |
| Card background | `--card-bg: #FFFFFF` |
| Page title | `--page-title-color: #181D27` |
| Body text | `--body-color: #344054` |
| Muted text | `#667085`, `#717680` |
| Border | `#E5E7EB`, `#E9EAEB`, `#D0D5DD` |
| Focus border | `#0000FF` |
| Success | `#027A48`, `#17B26A`, `#ECFDF3` |
| Warning | `#B54708`, `#F79009`, `#FFFAEB` |
| Danger | `#D92D20`, `#B42318`, `#FEF3F2` |

Secondary button note: existing CSS still defines green/blue secondary button tokens. For investor and developer dashboards, this is now considered legacy. The target secondary button style is neutral: white or very light neutral background, neutral border, dark/muted text, and no blue/green hover color change.

### Color Rules

- Maintain strong contrast for text and interactive controls.
- Do not layer bright colors on bright colors if legibility suffers.
- Do not layer dark colors on dark colors if legibility suffers.
- Use Bright Lime or Electric Blue as highlight colors, not as large uncontrolled backgrounds in dense product UI.
- Use status colors semantically and consistently:
  - Green: success, completed, active, positive movement
  - Amber/orange: pending, warning, needs attention
  - Red: destructive, declined, failed, error
  - Blue: information, primary action, selected state
  - Gray: neutral, disabled, metadata

## 5. Typography

### Brand Typography

The brand book defines:

- Primary headline type: Funnel Display Bold / Extra Bold
- Secondary and product type: Inter Regular / Medium / Semi Bold

The brand book suggests Funnel Display for expressive headlines and Inter for secondary/product type. In this codebase, TT Norms Pro is the required web font for both product and marketing surfaces unless a separate approved typography migration is created.

### Current Product Typography

The current platform implementation uses TT Norms Pro as the sole product UI font.

Defined in:

- Canonical font assets: `frontend/www/fonts/TTNormsPro/`
- `frontend/platform/static/css/fonts.css`

Available weights:

- 400 Regular
- 500 Medium
- 700 Bold
- 800 ExtraBold

Always use TT Norms Pro for web UI in this repository.

### Practical Typography Rule

- Investor dashboard, developer dashboard, public/marketing web pages, and shared product components should use TT Norms Pro.
- Do not introduce Funnel, Inter, or another web font unless the user explicitly approves a typography migration.
- Use existing `ds-text-*` styles where practical.

### Product Type Scale

Use existing design-system classes and tokens:

| Use | Class / Token | Size |
|-----|---------------|------|
| Display value | `.ds-text-display` | 36px |
| Page title | `.ds-text-xl`, `--page-title-font-size` | 36px |
| Section title | `.ds-text-lg` | 24px |
| Card title | `.ds-text-md` | 18px |
| Subsection heading | `.ds-text-sm-heading` | 16px |
| Body | `.ds-text-body` | 14px |
| Body large | `.ds-text-body-lg` | 16px |
| Caption | `.ds-text-caption` | 13px |
| Tiny/meta | `.ds-text-xs` | 12px |
| Money | `.ds-text-money` | tabular numbers |

Typography rules:

- Financial numbers must use tabular numeric styling where possible.
- Page titles belong at page level only; do not use hero-sized text inside cards, tables, modals, or compact panels.
- Use medium/semibold weight for labels and important values.
- Keep line lengths readable.
- Do not use negative letter spacing in compact UI unless already defined by a token/class.

## 6. Identity Graphics

### Circles And Triple-O

The primary graphic language is based on circles and interactions between circles.

Use circles and Triple-O inspired shapes to:

- Reinforce shared ownership
- Frame photography on marketing/public pages
- Create campaign graphics
- Support explanatory brand moments

Do not overuse circles inside dense product UI. In dashboards, circles should mostly appear as:

- Avatars
- Status dots
- Progress indicators
- Icon containers
- Small brand moments

### Type And Art

When combining type with imagery or graphic elements:

- Choose a clear focal point.
- Do not give type and image equal visual dominance.
- If the message is the hero, make typography dominant.
- If the asset/property is the hero, make imagery dominant and keep type supportive.

## 7. Imagery

Imagery should reveal the actual subject wherever possible:

- Properties
- Commodities
- Investor/community context
- Product state
- Team or operational credibility

Avoid:

- Generic stock-like imagery
- Dark, blurred, cropped, or atmospheric images when the user needs to inspect something
- Decorative imagery that competes with financial or compliance information

On marketplace and property pages, images must help the user understand the investment. They are not just decoration.

## 8. Layout And Spacing

### Dashboard Layout

Use existing layout tokens:

| Purpose | Token |
|---------|-------|
| Sidebar width | `--sidebar-width: 256px` |
| Page top/bottom padding | `--page-padding-top/bottom: 48px` |
| Page horizontal padding | `--page-padding-x: 32px` |
| Page max width | `--page-max-width: 1200px` |
| Section gap | `--section-gap: 24px` |
| Card padding | `--card-padding: 24px` |
| Card radius | `--card-border-radius: 12px` |
| Topbar height | Use the shared investor/developer topbar height; do not redefine per page |
| Topbar-to-content gap | Match `/leaderboard`: first content container starts `24px` below the topbar |
| Sidebar-to-content gap | Use `--page-padding-x` after the `--sidebar-width` offset |

### Layout Rules

- Use `dashboard-content` for dashboard page shells.
- Investor dashboard pages may use `ds-main` plus page-specific wrappers such as `portfolio-main`, `wallet-main`, or `rv2-main`, but those wrappers must preserve the same sidebar offset, topbar position, content width, and vertical rhythm.
- Use `/leaderboard` as the canonical spacing reference for investor dashboard pages. Its shell pattern is the baseline: main content width is `calc(100% - var(--sidebar-width, 256px))`, left offset is `margin-left: var(--sidebar-width, 256px)`, content max width is `var(--page-max-width, 1200px)`, content top padding is `24px`, horizontal padding is `var(--page-padding-x, 32px)`, bottom padding is `var(--page-padding-bottom, 48px)`, and section gap is `var(--section-gap, 24px)`.
- The topbar must have the same visual height on every investor and developer dashboard page. Do not change topbar padding, line-height, min-height, icon size, or action button height inside page-specific CSS unless the shared topbar component itself is updated for all dashboard pages.
- The first content block below the topbar must always start at the same vertical distance as `/leaderboard`: `24px` content-container top padding on desktop. Use the shared content container padding token/value instead of page-specific `margin-top`, negative margins, or custom top padding.
- Page content must always start at the same horizontal distance from the sidebar. The main wrapper should offset by `--sidebar-width`, and the content container should use `--page-padding-x`; do not hardcode custom left margins or per-page sidebar gaps.
- Keep page sections full-width or naturally constrained; do not nest cards inside cards.
- Cards are for repeated items, stat panels, modals, and genuinely framed tools.
- Avoid floating decorative section cards.
- Preserve predictable scan patterns: page header, actions, filters/tabs, content.
- Fixed-format UI elements such as boards, tables, counters, cards, and toolbars need stable dimensions to avoid layout shift.

### Investor Dashboard Layout Contract

Investor dashboard pages must share the same structural rhythm:

1. Sidebar
2. Investor topbar
3. Content container
4. Optional page-level alert/banner
5. Primary summary region
6. Secondary cards/tables/forms
7. Empty/error/loading states in the same content footprint

Required consistency:

- Sidebar width: use `--sidebar-width`.
- Content background: use `--content-bg`.
- Content horizontal padding: use `--page-padding-x`.
- Topbar height: use the shared topbar component without page-specific height overrides.
- Topbar-to-content spacing: match `/leaderboard`; desktop content container top padding is `24px`.
- Sidebar-to-content spacing: use the same sidebar offset plus `--page-padding-x` on every page.
- Main content max width: use `--page-max-width` unless the page has a documented full-width reason.
- Vertical section spacing: use `--section-gap`.
- Card padding: use `--card-padding`.
- Avoid page-specific hardcoded replacements for `256px`, `1200px`, `24px`, `32px`, and `48px` when tokens exist.
- Do not use negative margins to pull content under, into, or away from the shared topbar. If a page needs a different offset, change the shared layout token/component or document the exception in this file first.

The investor topbar should be the single source for the page title and primary header actions. Avoid duplicating a second desktop page title inside the content unless the page intentionally has a subsection title.

Mobile-specific headers are allowed only where the shared mobile navigation/topbar does not provide the needed title. They must not create duplicate visible page titles at the same breakpoint.

### Empty States In Dashboards

Empty states may use light brand expression, but they remain product UI.

Allowed:

- A concise headline
- A short explanation
- One primary action and at most one secondary action
- Small supporting steps or trust markers
- Simple brand illustration if it does not dominate the page

Avoid:

- Full marketing hero sections inside dashboards
- Large decorative SVGs that compete with the next action
- Unsupported trust claims or placeholder metrics
- Claims like AUM, ratings, or regulated status unless backed by real, approved data
- Multiple competing CTAs

Other empty-state pages should not automatically copy the wallet/portfolio empty-state hero. The card/button surface is the default, but the amount of illustration, onboarding copy, and supporting content should be decided page by page.

## 9. Components

### Buttons

Use:

- `.ds-btn`
- `.ds-btn--primary`
- `.ds-btn--secondary`
- `.ds-btn--danger`
- `.ds-btn--ghost`
- `.ds-btn--sm`
- `.ds-btn--lg`
- `.ds-btn--full`
- `.ds-btn--icon`

Rules:

- Every action button must have a visible label or accessible name.
- Use icon-only buttons only for familiar actions and provide an accessible label/title.
- Primary buttons are for the main safe action.
- Danger buttons are for destructive or irreversible actions.
- Disabled buttons must look disabled and must not submit actions.
- Async buttons need loading/disabled states where double-submit would be harmful.
- Financial or destructive buttons require backend validation regardless of frontend state.
- Investor dashboard filters, tabs, exports, show-more controls, copy buttons, and retry actions should use `ds-btn` or a documented segmented/tab primitive. Do not create a new button visual style per page.
- The wallet/portfolio empty-state CTA style is the preferred investor-dashboard CTA treatment. Reuse it through shared classes/tokens rather than duplicating `wallet-empty__cta-*` or `portfolio-empty__cta-*` CSS on each page.

### Tabs, Filters, And Segmented Controls

Dashboard tabs and filters must be consistent across investor pages.

Use a shared pattern for:

- Topbar tabs
- Chart timeframe tabs
- Table filters
- History filters
- Marketplace filters

Rules:

- Active state must be visible by more than color alone where practical.
- Tab/filter buttons must be keyboard reachable.
- Use consistent height, radius, padding, font size, and active indicator.
- Prefer a `ds-*` primitive or shared class over page-specific classes like `portfolio-chart-tab` and `rv2-history-filter-btn` if those classes only restyle a button.
- Do not use inline `onclick` for new tab/filter behavior if a page JS module can attach listeners.

### Forms

Use:

- `.ds-form-group`
- `.ds-form-label`
- `.ds-input`
- `.ds-select`
- `.ds-textarea`
- `.ds-form-error`
- `.ds-form-hint`

Rules:

- Every input needs a visible label or equivalent accessible label.
- Required fields must be marked.
- Validation errors must be visible next to the field or form section.
- Client validation is UX only; backend validation is mandatory.
- Sensitive forms must not leak secrets or personal data into URLs.
- Money inputs must make currency/minor-unit behavior clear and must be converted safely on the backend.
- Investor and developer dashboard forms must be built from reusable form primitives, not page-specific one-off input styles.
- New form UI should extend `ds-form-*`, `ds-input`, `ds-select`, and `ds-textarea` rather than creating new visual classes.
- Custom form controls such as upload/dropzone, amount steppers, card inputs, and search/autocomplete must document their shared class names and reuse the same label, hint, error, disabled, loading, and focus patterns.
- Reusable form controls must be easy to copy between wallet, checkout, KYC, settings, support, and developer submission flows without changing visual CSS.

### Cards

Use:

- `.ds-card`
- `.ds-card--elevated`
- `.ds-card--sm`
- `.ds-card--flush`
- `.ds-card--interactive`
- `.ds-card__header`
- `.ds-card__title`
- `.ds-card__body`
- `.ds-card__footer`

Rules:

- Product cards should be calm and information-forward.
- Interactive cards need clear hover/focus behavior.
- Avoid glass, holographic, or heavily decorative effects in investor dashboard cards except for a documented, isolated premium/marketing surface.
- Do not mix separately defined card surface systems in the same primary dashboard workflow when they are meant to represent the same kind of information.
- Do not put cards inside cards.
- Stat cards across wallet, portfolio, rewards, and trading should share the same radius, padding, shadow, label style, value style, and trend/badge treatment.
- The wallet/portfolio empty-state card surface is the preferred investor-dashboard card surface. Turn it into, or map it to, a shared card primitive over time.
- Property/asset cards are the main exception. Keep their richer media-card treatment when the card is selling, comparing, or explaining an investable asset.

### Icons

Dashboard icons should use one consistent visual style.

Preferred icon style:

- Line icons
- 20px default size for normal controls
- 16px for compact/table actions
- 24px for prominent card/title icons
- Stroke width around 1.5-2px
- Use `currentColor` where possible
- Neutral default color
- Brand blue only for active/selected/primary emphasis
- Semantic colors only for status or destructive actions

Icon containers:

- Use a consistent circular or softly rounded container.
- Use the same container size, radius, background, and border for equivalent cards.
- Do not mix raster icon files, inline SVGs, glass icons, and inconsistent stroke styles in the same dashboard workflow.
- Logos, property imagery, commodity imagery, and brand marks are exceptions.

When adding new icons, prefer shared SVG/icon patterns already used by the dashboard. Do not introduce a new icon family for one page.

### Metric And Financial Cards

Financial summary cards must be especially consistent.

Required pattern:

- Label: 14px, medium/regular, muted text
- Value: tabular numbers, semibold/bold, product value scale
- Supporting change/status: semantic badge or compact trend indicator
- Optional help icon: same size and color everywhere
- Optional action: `ds-btn` or text link with consistent styling

Rules:

- Use integer/cent-safe backend values; formatting happens for display only.
- Do not use decorative gradients behind financial values.
- Do not rely on green/red alone to communicate positive/negative changes.
- Values must not shift layout when loading or when numbers grow.

### Progress Bars

Progress bars must use the shared dashboard progress system.

Use:

- `.ds-progress`
- `.ds-progress__fill`
- `.ds-progress--sm`
- `.ds-progress--lg`

Default progress color:

- Track: `var(--progress-track-bg, #D5D7DA)`
- Fill: `var(--progress-fill-bg, #98FB96)`
- Radius: `var(--progress-border-radius, 30px)`

Rules:

- The standard fill color for normal progress is the brand green progress token, not a custom gradient.
- Use semantic variants only when the progress state itself is semantic: warning for approaching a limit, danger for failed/exceeded/error states.
- Do not create page-specific progress colors for profile completion, funding, upload, or submission progress when the shared progress tokens can represent the state.
- Inline styles may set dynamic width only, for example `style="width: 65%"`; color, radius, height, and transition belong in CSS.

### Tables

Use:

- `.ds-table-container`
- `.ds-table`
- `.ds-table-th`
- `.ds-table-td`
- `.ds-table-th--sortable`
- `.ds-table-empty`

Rules:

- Tables must support scanning, sorting/filtering where relevant, and empty states.
- Financial amounts should align consistently and use tabular numeric styling.
- Table headers should be clear and concise.
- Row actions must be explicit and accessible.
- Do not hide critical status information behind color alone.
- Investor dashboard tables should use one table system. Avoid custom table visuals unless the table has a documented layout requirement that `ds-table` cannot satisfy.
- Investor and developer dashboard tables must be reusable across wallet transactions, portfolio assets, rewards history, developer submissions, payment methods, and trading history.
- Page-specific table classes may define column widths and responsive behavior, but must not redefine header typography, row borders, cell padding, status badges, empty states, or row action button styles when `ds-table` primitives exist.
- Tables must have a reusable empty state, loading state, and error state.
- Mobile table behavior must be standardized: either responsive horizontal scroll or stacked row cards, chosen per table type and documented in the table component CSS.

### Modals

Use:

- `.ds-modal-overlay`
- `.ds-modal`
- `.ds-modal__header`
- `.ds-modal__title`
- `.ds-modal__body`
- `.ds-modal__footer`
- `.ds-modal__close`

Rules:

- Modals must be closable.
- Destructive actions must use clear confirmation copy.
- Modal content must fit mobile screens.
- Keyboard/focus handling must be considered.
- Error messages inside modals must be visible and actionable.

### Badges

Use:

- `.ds-badge`
- `.ds-badge--success`
- `.ds-badge--warning`
- `.ds-badge--danger`
- `.ds-badge--info`
- `.ds-badge--neutral`

Rules:

- Badges must describe status, not decorate.
- Status color must match semantic meaning.
- Add text, not just color.
- Keep labels short.

## 10. States

Every interactive page must account for:

- Loading
- Empty
- Success
- Error
- Disabled
- Unauthorized/forbidden
- Pending review
- Needs recheck/retry where relevant

Financial, KYC, developer submission, settings, support, and upload flows need especially clear failure states.

## 11. Accessibility

Target WCAG 2.2 AA where practical.

Rules:

- Text and controls must have sufficient contrast.
- Do not rely on color alone.
- Buttons and links need accessible names.
- Inputs need labels.
- Focus states must be visible.
- Modals, dropdowns, tabs, and menus must be keyboard usable.
- Error messages should be near the relevant input/action.
- Text must not overlap, truncate critical meaning, or overflow its container.
- Tap targets on mobile should be large enough for comfortable use.

## 12. Motion And Interaction

Motion should be quick and functional:

- Buttons may use subtle hover/active state changes.
- Modals may fade/slide in gently.
- HTMX swaps may use subtle cross-fades.
- Avoid large decorative motion in product UI.
- Respect users who prefer reduced motion where practical.

## 13. Responsive Rules

Minimum expectations:

- Desktop and mobile layouts must be checked for every changed page.
- Tables need a usable responsive strategy.
- Modals must not exceed viewport height.
- Header/sidebar/mobile navigation must not overlap content.
- Long words, emails, IDs, wallet references, and transaction references must wrap or truncate safely.

## 14. Content And Voice

POOOL copy should be direct, clear, and confidence-building.

Brand slogans from the brand book include:

- Own a share. Get paid. Repeat.
- Join our community.

Product UI copy should:

- Explain what will happen when a user clicks.
- Avoid vague labels like "Submit" when a more specific label is possible.
- Be especially clear for financial, KYC, developer submission, settings, support, and irreversible actions.
- Avoid hype in compliance, risk, payment, and operational flows.

## 15. Do And Do Not

### Do

- Use existing design-system CSS classes before creating new styles.
- Use tokens from `dashboard-tokens.css`.
- Keep product screens calm, structured, and reliable.
- Use brand color strongly but intentionally.
- Show real states and real data.
- Make errors actionable.
- Preserve accessibility and keyboard behavior.

### Do Not

- Invent a new visual language for one page.
- Add gradients/orbs/bokeh as generic decoration.
- Use hardcoded values when a token exists.
- Add nested cards.
- Use hidden client logic for financial/business rules.
- Create UI that implies a backend action works before verifying the route exists.
- Rely on color alone for status.
- Use logo colors or brand shapes in ways forbidden by the brand book.

## 16. Agent Checklist For UI Work

Before changing UI:

1. Read this file.
2. Check `docs/design/FRONTEND_COMPONENTS.md`.
3. Inspect relevant `frontend/platform/static/css/ds-*.css`.
4. Inspect the existing page/component pattern.
5. Confirm any backend route needed for the UI actually exists.

Before finishing UI:

1. Verify desktop and mobile layout.
2. Check loading, empty, success, error, and disabled states.
3. Check keyboard/focus basics.
4. Check that text does not overflow or overlap.
5. Check that no sensitive or financial behavior depends only on client code.
6. Run relevant tests or document why they could not run.

## 17. Investor Dashboard Consistency Checklist

Use this checklist when auditing or fixing investor dashboard pages.

### Shell

- [ ] Page uses the shared investor sidebar.
- [ ] Page uses the shared investor topbar.
- [ ] Page title appears once per breakpoint.
- [ ] Main content uses tokenized sidebar offset, content background, padding, and max width.
- [ ] Topbar height matches other dashboard pages; no page-specific topbar padding, line-height, min-height, icon-size, or button-height override.
- [ ] First content block starts at the same vertical distance below the topbar as `/leaderboard` (`24px` desktop content-container top padding).
- [ ] Content/cards start at the same horizontal distance from the sidebar as `/leaderboard` (`--sidebar-width` offset plus `--page-padding-x`).
- [ ] Mobile header/navigation does not duplicate or overlap desktop shell elements.

### Components

- [ ] Primary and secondary actions use `ds-btn` variants.
- [ ] Cards use the preferred investor-dashboard card surface from the wallet/portfolio empty states, ideally through `ds-card` or a documented shared extension.
- [ ] Tables use `ds-table` / `ds-table-container` unless a documented exception exists.
- [ ] Forms use `ds-input`, `ds-select`, `ds-textarea`, and `ds-form-*` classes.
- [ ] Badges/status pills use `ds-badge` variants.
- [ ] Tabs and filters share the same active, hover, focus, and disabled behavior.

### Visual Consistency

- [ ] No page mixes unrelated `ds-card`, custom flat card, glass, holographic, and marketing-card systems in one workflow.
- [ ] No inline styles define spacing, typography, color, or layout unless they are dynamic values such as progress width.
- [ ] Hardcoded spacing/color/radius values are replaced with tokens where tokens exist.
- [ ] Metric cards use the same label/value/status structure across wallet, portfolio, rewards, and trading.
- [ ] Icon containers use consistent size, radius, stroke weight, and background treatment.
- [ ] Property/asset cards are the only routine exception to the generic dashboard-card surface.

### Product Safety

- [ ] Financial values use tabular numbers and stable dimensions.
- [ ] Positive/negative states include text or icons, not only color.
- [ ] CTAs do not imply unavailable backend functionality.
- [ ] Empty states do not contain unverified trust claims or placeholder business metrics.
- [ ] Loading and error states occupy a stable footprint and do not cause major layout jumps.

### Known Current Drift To Fix Over Time

The current investor dashboard already contains several patterns that should be normalized gradually:

- Portfolio contains stale "holographic" / `holo-card` CSS comments and some legacy selectors, but the current visible template mostly uses custom `portfolio-*` flat cards rather than `ds-card`.
- Wallet uses `ds-card wallet-*` cards, but `wallet-*` CSS still overrides parts of the visual surface, so it is close to the design system but not a pure shared card primitive.
- Rewards v2 uses `glass-icon-*`, custom stat cards, and custom table/filter classes instead of consistently extending `ds-*` primitives.
- Wallet, portfolio, and rewards use different empty-state hero treatments and illustration weights.
- Some dashboard templates contain inline styles for margin, padding, color, icon spacing, and dynamic state.
- Button-like controls appear under multiple page-specific class names instead of shared `ds-btn` or a shared segmented-control primitive.
- The empty-state card and CTA style should be promoted into shared `ds-*` primitives so other investor cards can adopt it without copying page-specific CSS.

Do not fix all drift in one large rewrite. Normalize one page or component family at a time, preserving behavior and backend contracts.
