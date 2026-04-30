# Blog Draft QA Report: Bali Villa ROI: Gross Yield vs Net Investor Return

**Draft reviewed:** `/Users/martin/Projects/poool/drafts/2026-04-27-bali-villa-roi-gross-yield-vs-net-investor-return.md`  
**QA date:** 2026-04-27  
**Automation:** POOOL Daily Blog Draft QA Gate  
**Recommendation:** **BLOCKED - do not manually publish yet**  
**Recommended frontmatter:** `qa_status: blocked`, `approved_for_distribution: false`

## Executive Decision

This draft has the right editorial posture: it avoids guaranteed-return language, repeatedly separates market context from property-level performance, and tells investors to review documents and qualified advice. It is still **not safe to publish** because it discusses ROI, yield ranges, projected returns, fractional distributions, taxes, licensing, FX, liquidity, and platform/admin fees without inline citations, counsel/product approval, complete metadata, or a publish-ready image/schema package.

The strongest blocker is not tone; it is substantiation. Lines 78, 86, 112, 115-116, 148-166, 203, 208-209, 218, 222, 226, and 230-234 contain financial, tax, legal, liquidity, and distribution-related statements that need authoritative source support or approved internal disclosure language before publication.

## Pass/Fail Checklist

| Area | Status | Notes | Required fix |
|---|---:|---|---|
| Publish safety | FAIL | High-sensitivity ROI/investment/tax/legal/liquidity content remains unapproved. Compliance notes at lines 267-274 explicitly say not to publish without review. | Obtain financial, tax, legal, marketing-compliance, and POOOL product/legal review. |
| Title tag | PASS | `Bali Villa ROI: Gross Yield vs Net Investor Return` is 50 characters and includes the primary keyword at the front. | No required change. |
| Meta description | FAIL | 135 characters, below the 150-160 target; no statistic or clear value proposition endpoint. | Replace with a 150-160 character version, e.g. `See how Bali villa ROI shifts after 55.44% hotel occupancy context, operating costs, taxes, reserves, FX, fees, and exit limits.` |
| URL slug | PASS | `bali-villa-roi-gross-yield-vs-net-investor-return` is lowercase, keyword-rich, and 49 characters. | No required change. |
| Heading structure | WARN | One H1 and no skipped levels, but 13 H2s is heavy and four editorial/admin sections would publish as article body. | Remove `Internal Link Suggestions`, `External Source Needs`, `Visual And Chart Ideas`, and `Compliance Notes` from public content. |
| Internal links | FAIL | Body has 0 real markdown internal links; suggested links are parked in notes. | Add 3-5 contextual body links to live POOOL articles. Do not link to draft-only URLs unless those posts are published first. |
| External links | FAIL | Source URLs are listed at lines 247-255 but not cited inline where the claims appear. | Add inline links next to BPS and third-party range claims; use counsel-approved citations for tax/licensing claims. |
| Image SEO | FAIL | No hero image, cover image URL, OG image, dimensions, or alt text exists. | Add approved 1200x630 hero/OG image with descriptive alt text and WebP/AVIF variant. |
| Schema | FAIL | Draft lacks `canonical_url`, `og_image_url`, author/reviewer, category/tags, FAQ data, and explicit schema fields beyond `template`. | Add BlogPosting metadata, BreadcrumbList support fields, FAQ data if used, image metadata, and author/reviewer attribution. |
| AI citation readiness | WARN | Good tables, FAQs, and answer-first warnings. Weaknesses: no inline citations, no named author/reviewer, and no 40-60 word source-backed citation capsules. | Add short cited answer capsules under the major ROI, market-data, cost, and fractional-return H2s. |
| Factual support | FAIL | BPS figures and cited third-party ranges are reachable. Tax/licensing/fractional/distribution/liquidity claims remain unapproved for publication. | Inline source the market claims and get qualified Indonesian tax/legal review plus internal product/legal sign-off. |
| CTA clarity | WARN | CTA is compliant in tone, but appears as an H3 in the article flow and does not define a concrete safe next step. | Convert it to a CTA module focused on comparing project documents, risk disclosures, fee schedules, and scenario assumptions. |
| Conversion path | WARN | Educational path is present, but no safe destination is defined for current POOOL users. | Link to a document-review, marketplace-risk-disclosure, or waitlist path that does not imply suitability, fixed return, or liquidity. |
| Compliance risk | FAIL | The article uses ROI/yield/return language throughout and touches tax, licensing, currency, platform fees, distributions, and exit limits. | Treat as blocked until all high-risk claims have approved wording and citation support. |

## Hard Blockers

1. **Financial and projected-return claims need approved substantiation.** Line 78 cites gross yield, net yield, operating-cost, and management-cost ranges from third-party market guides. Lines 50, 76, 124-134, 166, 218, 222, and 234 discuss projected return, distributable amounts, and investor distribution yield. These are not inherently improper, but they need inline citations, clear context, and review so POOOL does not appear to endorse unsupported ROI assumptions.

2. **Tax and licensing statements need qualified review.** Lines 62, 72-73, 112, 188, 208, and 230 reference licensing, local obligations, rental/hotel/business/entity tax treatment, and tax deductions. These are hard-blocker topics until Indonesian tax/licensing counsel or a qualified tax reviewer approves the exact wording.

3. **Fractional investor rights, fees, distributions, and liquidity language needs product/legal approval.** Lines 26, 48, 52-54, 148-166, 203, 209, 222, 226, and 232-234 describe the layer between property revenue and investor return, possible distributions, lockups, transfer limits, secondary-market limitations, exit process, and liquidity risk. POOOL must approve exact disclosure language before publication.

4. **No inline citations appear in the article body.** The draft has 0 markdown links. Source notes at lines 247-257 are editorial notes, not reader-facing substantiation. For financial and legal-adjacent content, parked source lists are insufficient.

5. **Draft-only editorial sections remain in the publish body.** Lines 236-274 are production notes. Publishing them would expose internal workflow language and compliance instructions.

6. **Image, metadata, and schema package is incomplete.** No cover/OG image, canonical URL, author/reviewer, category/tags, FAQ data payload, or complete BlogPosting image fields are present. The live template can output canonical, OG, Twitter, BlogPosting, BreadcrumbList, and FAQ schema, but the draft does not provide the data needed for a complete publish-ready page.

## Source Verification

| Claim | Status | Evidence |
|---|---:|---|
| BPS Bali reported 492,289 direct foreign tourist visits in February 2026 | VERIFIED | BPS Bali official release says in-person foreign tourist visits in February 2026 were recorded at 492,289: https://bali.bps.go.id/en/pressrelease/2026/04/01/718028/tourism-overview-of-bali-province--february-2026.html |
| BPS Bali reported a 1.97% decrease from January 2026 and 55.44% star-rated hotel occupancy | VERIFIED | Same BPS release states the 1.97% month-over-month decrease and 55.44% star-rated hotel room occupancy: https://bali.bps.go.id/en/pressrelease/2026/04/01/718028/tourism-overview-of-bali-province--february-2026.html |
| BPS Bali reported a 9.23% year-over-year increase in direct foreign tourist visits | VERIFIED | BPS Bali news item reports 492,289 direct visits and a 9.23% increase versus February 2025: https://bali.bps.go.id/en/news/2026/04/01/354/kunjungan-wisman-ke-bali-pada-februari-2026-meningkat.html |
| Property Plaza says 2026 Bali villa gross yields range from 5-15%, net yields from 4-10%, and operating costs can consume 30-40% of gross rental income | VERIFIED AS THIRD-PARTY CLAIM | Property Plaza page contains those ranges. It is a market/industry source, not an official or independent investment authority: https://www.property-plaza.id/en/guides/how-much-rental-yield-bali-villa |
| Bali Property Scout says marketed ROI can exceed after-cost outcomes by several percentage points | VERIFIED AS THIRD-PARTY CLAIM | Bali Property Scout states the gap between marketed ROI and actual returns is 6-10 percentage points and provides example after-cost outcomes. Treat as a third-party commercial claim: https://www.balipropertyscout.com/blog/bali-airbnb-costs-taxes-management |
| Propertia says management fees, OTA commissions, tax, maintenance, insurance, and staff can consume 40-50% of gross rental revenue | VERIFIED AS THIRD-PARTY CLAIM | Propertia states total operational costs typically consume 40-50% of gross rental revenue. Treat as a third-party commercial claim: https://propertia.com/bali-villa-management-guide/ |
| Indonesian rental, hospitality, PT PMA, OSS, PHR, VAT, PPh, regional obligation, and licensing treatment | UNVERIFIED FOR PUBLICATION | The draft correctly flags counsel review as needed, but no authoritative inline sources or counsel notes are integrated. |
| POOOL fee schedule, reserve policy, distribution policy, investor rights, secondary-market limits, and exit process | UNVERIFIED FOR PUBLICATION | Requires POOOL product/legal source of truth and approved disclosure language. |

## Blog Quality Score

| Category | Score | Max | Notes |
|---|---:|---:|---|
| Content Quality | 25 | 30 | Clear, useful, 3,200-word educational guide with strong tables and caution language; public body still contains internal notes. |
| SEO Optimization | 13 | 25 | Good title/slug/keyword placement; fails meta length, internal links, inline external links, and source integration. |
| E-E-A-T Signals | 6 | 15 | Fresh data and sensible risk framing, but no author/reviewer credentials and no inline authoritative citations. |
| Technical Elements | 5 | 15 | Structured tables and FAQ exist, but no image, canonical/OG data, category/tags, or complete schema fields in draft metadata. |
| AI Citation Readiness | 10 | 15 | Strong Q&A/table structure and entity clarity; needs source-backed citation capsules and cleaner publish body. |
| **Total** | **59** | **100** | **Below standard until blockers are fixed.** |

## SEO And AEO Notes

- **Search intent:** Strong match for investors trying to understand gross yield versus net investor return. The article answers the query without pushing a specific listing.
- **Keyword use:** `Bali villa ROI` appears in title, H1, first substantive sections, and several H2s. No stuffing issue.
- **Title:** Good for SERP length and clarity. It lacks a power word, but the comparison framing is stronger than adding generic wording.
- **Meta:** Needs expansion and should include one specific data point or clearer reader outcome.
- **Headings:** The article has 13 H2s including non-public editorial sections. The publish version should have roughly 7-9 reader-facing H2s plus FAQ.
- **Internal links:** Add contextual links in the main body rather than leaving a note section. Verify targets are actually live before publish.
- **External links:** Use BPS as the primary market-data citation. Treat Property Plaza, Bali Property Scout, and Propertia as secondary market color, not proof of expected returns.
- **AEO/GEO:** The FAQ and tables are extraction-friendly. Add concise, cited answer capsules such as: "Bali villa ROI is not one number: gross yield, net operating yield, IRR, and investor distribution yield measure different layers of the return stack."

## Schema Opportunities

Add or confirm CMS fields for:

- `schema_type: BlogPosting`
- `canonical_url: https://www.poool.app/blog/bali-villa-roi-gross-yield-vs-net-investor-return`
- `og_image_url: [approved absolute 1200x630 image URL]`
- `cover_image_url: [approved image URL]`
- `cover_image_alt: Bali villa ROI model with property documents, calculator, and gross-to-net return worksheet`
- `author: [named author or POOOL editorial entity]`
- `reviewed_by: [financial/legal/tax reviewer if available]`
- `category: Bali Property Investing` or equivalent
- `tags: ["Bali villa ROI", "gross yield", "net investor return", "fractional real estate", "property due diligence"]`
- FAQ data for the five FAQ questions if POOOL intentionally emits FAQPage schema. Do not rely on Google FAQ rich results; use it mainly for structured extraction.

## Exact Next Fixes

1. Get financial/marketing-compliance approval for ROI, yield, distribution, IRR, projected-return, and stress-test language, especially lines 22-35, 50-54, 78-80, 124-134, 166-170, 197-210, and 216-234.
2. Get Indonesian tax/licensing review for lines 62, 72-73, 112, 188, 208, and 230, including rental, hospitality, entity-level, regional, OSS/PT PMA, PHR, VAT, and PPh references.
3. Get POOOL product/legal approval for fractional investor rights, reserve policy, fee schedule, distribution policy, lockups, transfer restrictions, secondary-market limitations, and exit language at lines 26, 48, 52-54, 148-166, 203, 209, 222, 226, and 232-234.
4. Add inline source links near claims at lines 32, 78, and 86. Keep third-party yield/cost sources framed as market-guide examples, not proof of a specific property or POOOL return.
5. Remove lines 236-274 from the publish body after converting the tasks into actual body links, source citations, image requirements, and compliance checks.
6. Add 3-5 contextual internal links. Recommended anchors: `Bali property due diligence checklist`, `Bali property structures compared`, `fractional ownership basics`, and `passive rental income from fractional real estate`, but only if those targets are live.
7. Replace the meta description with a 150-160 character version that includes either the 55.44% BPS occupancy context or a clear gross-to-net value proposition.
8. Add a hero/OG image with alt text and dimensions. Avoid luxury-only visuals that imply effortless returns.
9. Convert the H3 CTA at lines 168-170 into a designed CTA module that invites readers to review assumptions, documents, risk disclosures, and fee schedules. Do not imply suitability, guaranteed income, fixed return, buyback, or exit timing.
10. Add author/reviewer/category/tags/canonical/OG/schema data and rerun QA after approvals and source integration.

## Manual Publish Decision

**Not safe to manually publish today.** The right status is:

```yaml
qa_status: blocked
approved_for_distribution: false
```

The draft can return to publish review only after approvals, inline source integration, removal of editorial sections, image/schema completion, and a second QA pass.
