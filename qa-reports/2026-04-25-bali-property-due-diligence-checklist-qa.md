# QA Report: Bali Property Due Diligence Checklist

**Draft reviewed:** `/Users/martin/Projects/poool/drafts/2026-04-25-bali-property-due-diligence-checklist.md`  
**Review time:** 2026-04-25 22:16 CEST  
**Automation:** POOOL Daily Blog Draft QA Gate  
**Verdict:** BLOCKED  
**Safe to manually publish:** No  
**Recommended frontmatter:** `qa_status: blocked`, `approved_for_distribution: false`

## Executive Decision

The draft is useful, conservative in tone, and generally avoids promised-return language. It should not be manually published yet because it contains explicit unresolved legal/source-review notes, uses compliance-sensitive statements about Indonesian property rights, SPV/entity structure, escrow/process controls, ERSA, ROI assumptions, rental licensing, tax, liquidity, and fractional-investor rights, and does not yet include inline citations, image metadata, canonical URL, social metadata, or schema-ready fields.

The hard blockers are publication-packaging and compliance support, not overall article concept.

## Hard Blockers

| # | Blocker | Evidence | Required fix |
|---|---|---|---|
| 1 | Legal and product review is explicitly unresolved | Lines 263-269 and 289-295 say legal, licensing, POOOL process, SPV, escrow, ERSA, ROI, and disclosure language still need approval. | Get Indonesian counsel/product/compliance approval or remove/soften claims that cannot be approved before publishing. |
| 2 | Legal/property-rule claims are not cited inline | Lines 54, 64, 99, 231-245 make statements about Basic Agrarian Law, PP 18/2021, PP 16/2021, PP 28/2025, Hak Pakai, HGB/PT PMA, leasehold, nominee arrangements, PPAT/notary process, and rental legality, but sources are only listed at the bottom. | Add inline citations next to each legal/regulatory claim and keep "not legal advice" language near the sections, not only in the intro. |
| 3 | POOOL-specific process claims rely on internal whitepaper and need approval | Line 195 describes title verification, BPN/notarial review, encumbrance checks, appraisal, SPV structuring, ERSA, and ongoing monitoring. | Replace with compliance-approved public wording, or cite approved public POOOL disclosure materials. Do not publish internal whitepaper-derived process claims without approval. |
| 4 | CTA lacks adjacent risk disclosure | Lines 251-253 direct readers to property opportunities and projected performance assumptions, but no immediate investment-risk disclaimer appears beside the CTA. | Add a one-sentence risk disclosure directly under the CTA: no guaranteed returns/liquidity, read offering documents, seek legal/tax/financial advice. |
| 5 | Publication metadata is incomplete | Frontmatter lacks canonical URL, cover/OG image, image alt text, author/reviewer, schema type, dates, and social fields. | Add canonical URL, `og_image_url` or cover image, descriptive alt text, author/reviewer fields, `schema_type: BlogPosting`, and publish/modified dates before CMS import. |

## Pass/Fail Checklist

| Area | Status | Notes |
|---|---|---|
| Title tag | PASS | `Bali Property Due Diligence Checklist for Investors` is 51 characters and includes the primary keyword early. |
| Meta description | FAIL | 147 characters, slightly below the 150-160 target; includes ROI language but no source/stat. |
| URL slug | PASS | `bali-property-due-diligence-checklist` is clean, lowercase, keyword-rich, and not date-based. |
| H1 | PASS | Exactly one H1 and it matches search intent. |
| Heading hierarchy | PASS | H1 > H2 > H3 structure is clean; no skipped levels found. |
| H2 question format | PASS | 8 of 9 main H2s are question-formatted, strong for AEO/GEO. |
| Internal links | NEEDS WORK | Internal links are suggestions only. Convert them to body links and verify target slugs are live in production. |
| External links | NEEDS WORK | Six authoritative source URLs are listed, but none are cited inline in the body. |
| Duplicate links | PASS | No duplicate markdown links in body content. |
| Images and alt text | FAIL | No images are embedded and no cover/OG image metadata exists. |
| Schema opportunities | FAIL | BlogPosting, BreadcrumbList, Organization, Person/Reviewer, and ImageObject are appropriate; draft does not provide complete schema-ready metadata. Do not add FAQPage rich-result schema because FAQ rich results are restricted to government/health authority sites. |
| AI citation readiness | NEEDS WORK | Strong Q&A structure and tables, but sections need inline citations and tighter 120-180 word answer-first passages. |
| Factual support | NEEDS WORK | BPS tourism numbers and cited regulation pages are plausible/reachable, but legal interpretation and POOOL process claims require approval. |
| ROI/investment compliance | NEEDS WORK | Tone is cautious and says returns are not guaranteed, but `ROI assumptions` in metadata/sections need compliance review and source-backed definitions. |
| Legal/tax/liquidity compliance | BLOCKED | Draft itself flags legal, tax, liquidity, SPV, escrow, ERSA, ownership, and licensing review needs. |
| CTA clarity | NEEDS WORK | CTA is clear, but should link to `/marketplace` or approved opportunity page and include adjacent risk disclosure. |
| Conversion path | NEEDS WORK | CTA exists as plain text; add one approved link and risk language. |

## Fact-Check Notes

Verified/low-risk source support:

- BPS Bali 2025 foreign arrival claim at line 162: `6,948,754` and `9.72%` match the cited BPS release.
- BPS Bali February 2026 claim at line 162: `492,289` and `9.23%` match the cited BPS release.
- Law No. 5 of 1960, PP No. 18 of 2021, PP No. 16 of 2021, and PP No. 28 of 2025 source pages are valid source leads for the legal/regulatory topics named in the draft.

Unverified or approval-required:

- Any interpretation of what foreign investors can legally own or control in Bali.
- Any statement that nominee arrangements are illegal, enforceable, or unenforceable in a specific way.
- Any statement about rental/tourism licensing requirements for a specific Bali regency or short-term rental model.
- Any POOOL claim about BPN verification, notarial review, SPV structuring, ERSA documentation, escrow/fund handling, appraisal, approval committee review, or ongoing monitoring.
- Any tax outcome or investor-level tax statement beyond general caveat language.

## Exact Next Fixes

1. Add inline citations after legal/regulatory claims:
   - Line 54: Basic Agrarian Law and PP 18/2021.
   - Line 99: PP 16/2021 and PP 28/2025.
   - Lines 231-245: land-rights and counsel/notary claims.
2. Move or rewrite the source-needs section before publication:
   - Remove `External Source Needs`, `Visual and Chart Ideas`, and internal `Compliance Notes` from the publishable article body.
   - Keep those notes in the CMS/editorial ticket, not on the public page.
3. Rewrite line 195 unless legal/product approves it:
   - Safer version: "POOOL listings should explain the completed due-diligence categories, the responsible reviewers, investor rights, fees, reporting, and exit limits in the listing documents."
4. Add risk disclosure directly below the CTA:
   - "Property investments involve legal, operating, currency, tax, liquidity, and loss-of-capital risk; projected performance is not guaranteed and should be reviewed with qualified advisers."
5. Convert internal link suggestions into contextual links:
   - Add 3-5 links in relevant paragraphs, not a raw suggestion list.
   - Use only confirmed live slugs.
6. Improve metadata:
   - Meta description target: 150-160 chars, with no implication that ROI can be fully verified.
   - Suggested replacement: "Use this Bali property due diligence checklist to verify title, zoning, permits, lease terms, rental legality, fees, tax, and exit risks."
7. Add media fields:
   - Cover image/OG image: 1200x630 minimum.
   - Alt text: "Bali property due diligence checklist with villa documents, map, and contract review notes."
8. Add schema-ready metadata:
   - `schema_type: BlogPosting`
   - `canonical_url: https://www.poool.app/blog/bali-property-due-diligence-checklist`
   - `author`, `reviewed_by`, `date_published`, `date_modified`, `cover_image_url`, `cover_image_alt`.
9. Add reviewer attribution:
   - Because this is high compliance risk, include "Reviewed by legal/compliance" only after a real review is complete.
10. Final compliance sweep:
   - Search again for ROI, OJK, SPV, escrow, ERSA, legal enforceability, ownership, liquidity, tax, projected return, guaranteed, passive income, and currency language.

## SEO / AEO / GEO Score

| Category | Score | Rationale |
|---|---:|---|
| Content quality | 24/30 | Comprehensive, readable, strong checklist structure; needs public-ready cleanup and inline support. |
| SEO optimization | 18/25 | Strong keyword/title/slug/headings; metadata, body links, and external citations need work. |
| E-E-A-T | 8/15 | Good source list and cautious tone, but no author/reviewer credentials or inline source placement. |
| Technical elements | 5/15 | No image metadata, incomplete social/canonical fields, no schema-ready metadata in draft. |
| AI citation readiness | 10/15 | Strong Q&A/table structure; needs answer-first citation capsules and inline citations. |
| Compliance readiness | 9/20 | Good disclaimers and no guaranteed returns, but unresolved legal/product review is a hard block. |

**Overall readiness:** 74/120, blocked.

## Final Recommendation

Do not publish manually. Set or keep:

```yaml
qa_status: blocked
approved_for_distribution: false
```

The draft can move to publish review after legal/product approval, inline citation placement, metadata/image/schema completion, contextual internal links, and CTA risk disclosure are added.
