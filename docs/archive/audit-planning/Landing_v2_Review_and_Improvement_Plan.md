# POOOL Landing v2 — Review & Improvement Plan

**Scope:** Review of `landing-v2.html` (intended replacement for the live landing at poool.app), compared against the previous `landing.html` and the official POOOL Brand Book. Focus areas: visual polish, copy & messaging, information architecture, brand consistency.

**Date:** April 19, 2026
**Prepared for:** Jonas

---

## 1. TL;DR — The honest take

The v2 has strong bones: the asymmetric hero with the ambient villa video, the dark gradient CTA banner with the fanning card stack, and the reorganized "Simple Process" block are genuinely impressive upgrades over the current live site. But the page currently reads like three different landing pages stitched together. Fix the brand drift, unify the CTAs, resolve the pricing contradiction, and tighten the hero headline and this becomes a conversion-ready page.

**The five changes that matter most:**

1. Fix the "$15 vs. $500" pricing contradiction that appears on the same page.
2. Rewrite the hero headline — the "JOIN W[flags]RLDWIDE" construct breaks readability.
3. Re-skin the "sovereign commodities" section to match the brand (currently in grey + sky-blue, not POOOL Electric Blue + Lime).
4. Put the official tagline — **"Own a share. Get paid. Repeat."** — on the page. It isn't there.
5. Add a trust bar immediately under the hero (investor count, licenses, press/partners).

Everything else is polish on top of that foundation.

---

## 2. Brand baseline (what the page *should* match)

Pulled from the POOOL Brand Book and the existing `landing.css` tokens:

**Primary colors**
- Electric Blue — `#0000FF` (primary accent, main CTA)
- Greeny Green — `#03FF88` (highlight / accent)
- Bright Lime / Mint — `#98FB96` (the soft lime used across cards and buttons)
- Deep navy variants — `#00044A`, `#000ABF`, `#1B2559` (gradients, dark surfaces)
- Ink / text — `#181D27` (headlines), `#344054` (body), `#667085` (secondary)

**Typography (per brand book)**
- Headlines: **Funnel Display** (Bold / Extra Bold)
- Body: **Inter** (Medium / Regular)
- *Current site uses TT Norms Pro — this is a brand-book violation and should be reconciled.*

**Tagline**
- **"Own a share. Get paid. Repeat."**
- *Not present anywhere on v2. This is the single biggest brand asset the page is missing.*

**Logo color rules**
- Light backgrounds → Electric Blue or Greeny Green logo
- Dark backgrounds → Soft Blue or Bright Lime logo
- Over photography → Soft Blue or Bright Lime only

---

## 3. Section-by-section findings

### 3.1 Navigation
**What works:** Clean sticky header, clear nav, logical actions (Log in / Sign up).

**Issues**
- "Sign up" is the *secondary* style here but the *primary* action everywhere below. Make it visually louder — full Electric Blue, with a subtle glow — so nav and hero reinforce the same goal.
- The scroll-state "pill" (`Location | Volume | Status`) is a clever Airbnb-style detail, but the labels are vague. "Volume" in particular reads as financial jargon, not a property filter. Rename to `Location | Budget | Type`.

### 3.2 Hero
**What works:** The asymmetric layout with the background villa video is the strongest creative moment on the page. The video-zoom animation gives the feel of a product reel, not a static page.

**Issues**
- **Headline grammar is broken.** `JOIN W 🇺🇸🇪🇪🇯🇵 RLDWIDE INVESTORS COMMUNITY` is visually clever (flags replace the "O" in WORLDWIDE) but reads as "Join W-rld-wide Investors Community." The missing article ("our") and the flag-substitution together make the line unscannable at a glance. Options:
  - **Safer:** `JOIN OUR WORLDWIDE INVESTOR COMMUNITY` (flags in a smaller trust-strip below).
  - **Punchier, on-brand:** `OWN A SHARE. GET PAID. REPEAT.` with a sub-line "Join 850+ investors in 40+ countries" — uses the official tagline and moves the flag concept into the sub.
- **No social proof above the fold.** The meta description brags about "850+ investors trust POOOL" but this stat is nowhere on the rendered page. Add a one-line trust strip under the CTAs: stars + rating, investor count, and 2–3 partner/press logos.
- **Two CTAs, two color schemes, same destination depth.** "Browse Properties" (blue/mint) and "Sign Up" (mint/blue) are flipped-color twins. This is visual noise, not a choice architecture. Make one clearly primary ("Browse Properties" → solid Electric Blue, Mint text) and the other clearly secondary ("Sign Up" → ghost button, Blue border, Blue text).
- **Sub-copy is weak.** `Invest in fully notarised Bali real estate and global commodities.` mixes two asset classes in one sentence and buries the hook (monthly yields, $15 minimum). Rewrite tighter: *"Own fractions of notarised Bali villas from $15. Earn monthly rental yields — exit anytime."*
- **Hero body text color is hard-coded to `#000000`** in inline style, overriding the design token. Use `var(--body)` so it stays on-brand if tokens change.

### 3.3 Featured Properties
**What works:** Image carousels on each card, clear progress bars, consistent card structure. This section is the most polished.

**Issues**
- **Pricing header confusion.** The cards show "USD 500, 380, 280, 420" but these are *minimum share prices*, not property values — and the hero advertises "starting from $15." This is the #1 conversion killer on the page. Either (a) add a clearly-labeled field (`Min. share · $500`) or (b) show a per-share price that matches the $15 promise if one exists.
- **"Projected annualised net return" of 12–16%** is aggressive without footnotes. In financial-services landing pages regulators increasingly require that any return figure is accompanied by a disclaimer link at the first instance. Add a small footnote anchor.
- **Investment duration shows "—" on every card.** Either populate it, hide the row, or replace with "Flexible — exit on marketplace."
- **The "View all properties" link sits under the cards**, but on the current live site it's in the section header (a more common pattern). Pick one — I'd recommend keeping it top-right in the header for desktop, bottom-centred for mobile.

### 3.4 Inline CTA Banner ("Own Bali's most profitable real estate")
**What works:** This is the design highlight of the page. The deep-blue gradient, architectural grid overlay, corner glow, and the fanning Swiper card deck on the right are genuinely premium. The "Limited shares remaining" pulsing dot adds urgency without screaming.

**Issues**
- **"Profitable" is a risky word** on an investment site and may trip compliance reviews. Consider "Own Bali's most **in-demand** real estate" or "…most **sought-after** villas" — same vibe, less regulatory exposure.
- **"No credit card needed" trust line** is great but the phrase implies a free trial — unusual for an investment platform. Replace with something truer: *"Takes 2 minutes. No obligation."*
- **The fanning card deck on the right clips off-screen on narrower desktop widths** (transform: scale(0.95) rotate(8deg) translateX(50px) pushes it past the container). Make sure there's breathing room at 1280px and below.
- **Swiper slides 2 and 3 show less info than slide 1** (missing investment duration row in slide 3). Keep them structurally identical so the "fan" reads as a consistent stack.

### 3.5 Commodities / "Institutional Briefing" section
**This section is the biggest brand-consistency problem on the page.**

- Background uses a **dark grey gradient** (`#1f2937 → #111827`) instead of the POOOL navy gradient used in the CTA banner above.
- The primary button uses **sky blue `#60a5fa`**, not Electric Blue `#0000FF`. This is a Tailwind-default color, not a POOOL color.
- The "Alternative Asset" badge uses the same sky-blue tint (`rgba(96,165,250,0.1)`).
- Net effect: this panel looks like it was lifted from a different product.

**Recommendations**
- Re-skin in the **POOOL navy gradient** (`#00044A → #000ABF`) — matches the CTA banner above so the two dark blocks visually connect.
- Swap the sky-blue CTA for **Bright Lime (`#98FB96`) with Electric Blue text** — consistent with the banner directly above it.
- Tone down "National Chilli Reserve Project" — it's a real product but on a landing for retail investors the name reads as oddly specific. Lead with the category ("Sovereign-backed agri-commodities") and then name the reserve.
- The heading "Exclusive access to sovereign-yield commodities" is strong; the body copy is dense. Cut paragraph 2 and merge the points into a 3-bullet trust list (Government-backed · Fixed 14.2% APY · 36-month term).

### 3.6 How It Works
**What works:** The video on the right side showing the phone app is compelling product proof.

**Issues**
- **The v1 had a 4-step flow** (Discover → Explore → Invest → Earn). In v2 this has collapsed into **one big paragraph card** with the label "Simple Process." You've lost the instructional scannability — a first-time visitor can't glance and know what the process *is*. Bring back 3–4 steps inside the card (keep the visual container, but split the content). Example:
  1. Pick an asset from $15
  2. Buy your share — notarised in minutes
  3. Earn monthly rental yield
  4. Exit anytime on the marketplace
- The section label "How it works?" with a question mark reads casual. "How it works" (no ?) is cleaner and more on-brand for a fintech.

### 3.7 Why Us ("pick it. love it. own it.")
**What works:** The lowercase tagline is charming and distinctive. The fanning property cards on the right are a great callback to the CTA banner's fan-stack.

**Issues**
- **"$500" contradicts the hero's "$15".** The body copy says "starting from just $500 – powered by cutting-edge technology…" This has to be reconciled across the page. Pick one minimum and stick to it. If $15 is the true share minimum and $500 is a typical property allocation, make that distinction explicit.
- "What began as a market trend has evolved into a global investment platform…" is vague filler. Replace with a real milestone: *"From a single villa in Canggu in 2024 to 40+ assets across three markets today."*
- The three feature cards (`Direct ownership`, `Monthly payouts`, `Highest ROI`) reuse the same `best-price.webp` image for both Monthly payouts and Highest ROI. Give them distinct icons.
- **"Highest ROI" is a superlative** — high legal risk on an investment page. Soften to "Operator-cost pricing" or "No profit-hungry middlemen."
- The `fan-disclaimer` text — *"POOOL platform is currently under development and this content is intended for demonstration and concept presentation purposes only."* — is a killer. It signals to every serious investor that the numbers on the page are fake. If the platform is live, remove it. If it's not yet live, move it to the footer and reword ("Beta — figures illustrative").

### 3.8 Testimonials
**What works:** Three distinct personas (analyst, retail, nomad) cover the page's target audiences.

**Issues**
- **All three testimonials feel copywritten.** They use the same structure and tone ("POOOL democratized…", "The ability to diversify…", "I started with $500, strictly testing the waters…"). Either source real quotes (G2, Trustpilot, Instagram DMs) or rewrite so each voice sounds distinct.
- **Avatar images don't match the names.** `Khai.webp`, `Team.webp`, `monique-howeth-mobile.webp` — the file names betray that these are stock or placeholder images. Use real investor photos with permission, or switch to illustrated avatars.
- **Missing the third conversion lever: outcome proof.** Add a single-line numeric outcome under each quote ("$420 earned in month 1" / "14 properties in portfolio" / "Sold out in 48h").

### 3.9 FAQ
Same content as the current live site. Five questions is a healthy length for a landing page. Two small improvements:

- Add an **"Is POOOL regulated?"** question — this is the #1 silent objection on fractional-investment landing pages.
- The "When can I sell my shares?" answer says "ensure liquidity without the typical real estate lockups — you are always in control of your exit" but the Why Us section's disclaimer contradicts that (if the platform is under development, liquidity promises are risky). Reconcile.

### 3.10 Final CTA ("Ready to own a piece of paradise?")
Strong headline. Sub-copy is generic. The single "Create Free Account" CTA is the right call.

- Add a secondary reassurance below the button: *"No credit card · Cancel anytime · Notarised contracts."*
- Consider moving the 850+ investors proof point here as well — strongest CTA is right before the sign-up click.

### 3.11 Footer
- **Typo:** "Bali 80361, Indonesien" — "Indonesien" is the German spelling. Change to "Indonesia" on the EN site.
- The "A Word Of Caution" block is wisely thorough but visually heavy. Collapse into an accordion or move the bulkier legal paragraphs to a dedicated `/legal` page.
- Licence block image is promising — but make it a link to the licence detail page so it reads as verified proof, not decoration.

---

## 4. Cross-cutting issues

### 4.1 Inline-style overload
Large blocks of the page are styled with inline `style="..."` attributes and `!important` overrides. This is the root cause of most brand-color drift (e.g., the sky-blue commodity CTA). The fix is structural: move every color and spacing value into CSS variables in `landing.css`, then use those variables in the HTML. Until that happens, any "fix" in one place will silently re-drift somewhere else.

### 4.2 CTA hierarchy across the whole page
Count of primary-style CTAs on the page: **6** (hero × 2, inline banner × 1, commodity × 1, Why Us buttons × 0, final CTA × 1, plus nav). On a landing page the *single most important sentence* should be the primary CTA. Right now none of them stand out because they all look important. Rule of thumb:

- **1 primary CTA per scroll section** (solid Electric Blue)
- Everything else is secondary or tertiary (ghost/text link)

### 4.3 Typography pairing
The brand book specifies **Funnel Display + Inter**, the site serves **TT Norms Pro**. Pick a lane:
- If TT Norms Pro is the new official pairing, update the brand book so designers stop pulling Funnel Display into comps.
- If Funnel Display is still canonical, swap the webfonts and fix the `--font` token.

### 4.4 Missing on-brand tagline
`Own a share. Get paid. Repeat.` is the official brand line from the book. It should appear at least twice — ideally as an accent line in the hero (below the headline) and as the eyebrow on the final CTA section. Without it, the page doesn't *sound* like POOOL.

### 4.5 Price/number consistency audit
The page currently claims all of the following simultaneously: `$15 minimum · $500 minimum · 12.4–16.2% annualised net return · 14.2% underwritten APY · 850+ investors · $2.6M in Nomad Palm · monthly yields`. Some of these are real, some are demonstrations. Do a full pass to mark the demo data clearly or replace with real figures.

---

## 5. Prioritized action list

**Priority 1 — Do before launch (blockers):**
1. Resolve the `$15 vs $500` contradiction everywhere on the page.
2. Remove or relocate the "demonstration purposes only" disclaimer.
3. Re-skin the commodities/institutional section in POOOL brand colors.
4. Rewrite the hero headline for readability.
5. Fix the "Indonesien" typo.

**Priority 2 — Ship in week 1:**
6. Add a trust bar under the hero (investors / countries / licences).
7. Restore the 3–4 step flow inside the "How It Works" card.
8. Consolidate CTA hierarchy — one primary per scroll section.
9. Unify the typography (reconcile brand book vs. live site).
10. Add the `Own a share. Get paid. Repeat.` tagline in the hero and final CTA.

**Priority 3 — Within the first month:**
11. Replace stock testimonials with real ones + outcome metrics.
12. Distinct icons on the 3 "Why Us" features.
13. Add "Is POOOL regulated?" to the FAQ.
14. Audit every color in the HTML and move to CSS variables.
15. Add a compliance footnote link on every return figure.

---

## 6. Appendix — brand-color cheat sheet for the redesign

| Role | Color | Hex | Usage |
|---|---|---|---|
| Primary accent | Electric Blue | `#0000FF` | Primary CTAs, links, emphasis |
| Highlight | Greeny Green | `#03FF88` | Numeric highlights, progress fills |
| Soft accent | Bright Lime | `#98FB96` | Button text on blue, pill badges |
| Deep gradient start | Navy | `#00044A` | Dark banners (start) |
| Deep gradient end | Royal | `#000ABF` | Dark banners (end) |
| Headline ink | Near-black | `#181D27` | H1/H2 |
| Body ink | Slate | `#344054` | Paragraph |
| Secondary ink | Muted | `#667085` | Captions, labels |
| Surface | White | `#FFFFFF` | Page background |
| Muted surface | Off-white | `#F9FAFB` | Alternating sections |

**Do not use on this page:** Tailwind sky-blue `#60a5fa`, Tailwind grey `#1f2937`/`#111827`, any purple. These currently appear in the commodities section and are off-brand.
