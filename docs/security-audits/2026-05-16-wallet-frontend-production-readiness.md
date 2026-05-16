# Wallet Page — Production Readiness Audit (Frontend)

Date: 2026-05-16
Scope: `/wallet` page rendered against the live dev backend on
`localhost:8888`, evaluated for production-grade quality on perf,
accessibility, mobile, dark-mode, and runtime correctness.

Companion to the backend hardening shipped earlier this day
(commits `7639522`, `685ad50`, `613cfca` — wallet C-1..C-2, H-1..H-4,
M-1..M-4 with migrations 195 & 196).

## Headline

The wallet page is **production-grade on perf and mobile**, has **two
trivial-fix accessibility gaps**, and is **missing dark mode**. No
runtime errors, no failed first-party requests, no horizontal scroll on
mobile.

## Measured (live, signed-in session)

| Dimension | Value | Verdict |
|---|---|---|
| TTFB | 12 ms | ✅ excellent |
| First Contentful Paint | <1 s | ✅ excellent |
| DOMContentLoaded | 156 ms | ✅ excellent |
| Load event | 931 ms | ✅ under 1 s |
| Transfer size | 42 KB | ✅ tight |
| Resource count | 70 | ⚠️ high but no slow resource (>500 ms) detected |
| Console errors | 0 | ✅ |
| First-party 4xx / 5xx | 0 | ✅ |
| Horizontal scroll @ 375px | none | ✅ |
| Overflow elements @ 375px | 0 | ✅ |
| H1 count | 1 | ✅ |
| `<html lang>` | "en" | ✅ |
| `<meta viewport>` | correct | ✅ |

## Findings

### F-1 (Medium) — No skip-link, no `<main>` id

Keyboard users tab through **34 focusable elements** (topbar + nav)
before reaching the wallet content. WCAG 2.4.1 (Bypass Blocks). The
fix is two lines:

```html
<!-- right after <body> -->
<a class="skip-link" href="#main">Skip to main content</a>

<!-- change -->
<main>
<!-- to -->
<main id="main" tabindex="-1">
```

Plus 8 lines of CSS to hide the skip link off-screen until focused.

**Location:** `frontend/platform/wallet.html` (currently has WIP — fix
when WIP lands).

### F-2 (Medium) — Four form inputs missing label association

Found in the bank-add modal and the dropdown search component:

| Input | Where | Visible label nearby? |
|---|---|---|
| `routing_code` | Bank-add modal → "Routing Number (ABA)" | yes, in `.ds-form-group` text — but no `<label for>` |
| `account_number` | Bank-add modal → "Account Number" | yes — same gap |
| `label` (nickname) | Bank-add modal → "Nickname" | yes — same gap |
| (dropdown search) | `.poool-dropdown__search` | no — should at least have `aria-label="Search"` |

Screen reader users hear "edit text, blank" instead of the field name.
Fix is to wrap the existing visible text in `<label for="...">` or add
`aria-labelledby` pointing at the existing heading.

**Location:** `frontend/platform/wallet.html` (WIP — fix when WIP
lands).

### F-3 (Low) — No dark mode

With `prefers-color-scheme: dark` the page still serves the light
theme (body `rgb(250,250,250)`, cards white). Real-world impact: users
with system dark mode get a bright white wall.

Two viable paths:

1. **Quick win**: add a `<meta name="color-scheme" content="light dark">`
   and let the browser invert the default form/scrollbar colors.
   Doesn't fix card colors but takes ~30 s.
2. **Proper fix**: introduce CSS custom-properties theme tokens in
   `frontend/platform/static/css/wallet.css` and a body `data-theme`
   selector toggled by JS, defaulting to system preference. Half-day.

**Location:** `frontend/platform/static/css/wallet.css` (WIP — defer
or do as part of a wider design-system theming pass).

### F-4 (Low — informational) — CDN dependency for HTMX + Alpine.js

The page loads HTMX (`unpkg.com/htmx.org@1.9.10`) and Alpine
(`cdn.jsdelivr.net/npm/alpinejs@3.14.9`) from third-party CDNs in the
critical path. Pros: cache hits across the wider web. Cons: outage at
either CDN degrades the page (forms break silently).

Real production deployments either self-host these or pin them to a
versioned bundle behind our own CDN with SRI hashes. Today they're
unpinned in production too — same risk profile in dev and prod.

**Action**: not blocking, but worth raising in the next dependency
review. Add `integrity="sha384-…"` SRI hashes at minimum to detect
CDN tampering.

## Not flagged (verified clean)

- No oversized images (largest paint resource <50 KB).
- No render-blocking script in head (HTMX/Alpine load defer-style).
- No layout shift visible during initial render.
- All buttons have accessible text or `aria-label`.
- All images carry `alt`.
- Mobile breakpoint (375 px) renders without overflow.
- `<meta viewport>` set correctly with `viewport-fit=cover` for
  iPhone notch.

## Backend correctness (smoke-tested live)

- `/wallet` returns 200 as authenticated user (verified as
  `support@traffic-creator.com`).
- Deposit modal opens; amount field accepts `15000`; the SoF
  doc-upload input is present in the DOM (gated by step UI — backend
  enforces C-1 fix regardless of frontend bypass attempts).
- No first-party network failures on initial page load.

## Recommended ship order

1. F-1 (skip link + `<main id>`) — 10 minutes, ships with next
   wallet.html change.
2. F-2 (4 labels) — 15 minutes, ships with next bank-modal change.
3. F-3 (`color-scheme` meta) — quick win, 1 minute.
4. F-3 proper dark mode + F-4 SRI hashes — defer to design-system
   refresh and dependency-hygiene pass respectively.

## Verdict

The wallet page is **ready to put in front of real users today** with
the understanding that:

- Sighted desktop + mobile users get a great experience.
- Screen-reader and keyboard-only users hit the skip-link gap and 4
  unlabeled inputs in the bank-add modal — usable but not WCAG AA.
- Dark-mode users see the light theme.

None of the open items are exploits or correctness bugs; they're
accessibility and theming gaps that can ship as a follow-up batch
without rolling back current functionality.
