# Blog Draft QA Report: Leasehold vs Hak Pakai vs PT PMA vs Fractional Exposure in Bali

**Draft reviewed:** `/Users/martin/Projects/poool/drafts/2026-04-26-leasehold-hak-pakai-pt-pma-fractional-exposure-bali.md`  
**QA date:** 2026-04-26  
**Automation:** POOOL Daily Blog Draft QA Gate  
**Recommendation:** **BLOCKED - do not manually publish yet**  
**Recommended frontmatter:** `qa_status: blocked`, `approved_for_distribution: false`

## Executive Decision

This draft is useful, cautious, and directionally aligned with POOOL's compliance posture, but it is **not safe to publish** until legal/product review and source integration are complete. It discusses Indonesian land rights, Hak Pakai, PT PMA/HGB, leasehold enforceability, fractional investor rights, tax/reporting, exit/liquidity limits, and market data. The draft itself states that counsel review is required, and the current body does not inline authoritative citations for the practical legal and product claims.

## Pass/Fail Checklist

| Area | Status | Notes | Required fix |
|---|---:|---|---|
| Publish safety | FAIL | High-sensitivity legal/investment content remains unapproved. Lines 27, 319-328 explicitly require counsel/product review. | Obtain Indonesian counsel review and POOOL product/legal approval before publish. |
| Title tag | PASS | `Leasehold vs Hak Pakai vs PT PMA vs Fractional Bali` is 51 characters and includes the core keyword early. | Optional: use `Leasehold vs Hak Pakai vs PT PMA in Bali` if truncation testing prefers shorter wording. |
| Meta description | FAIL | 130 characters, below the 150-160 target and has no publication-ready CTA. | Replace with a 150-160 character description, e.g. `Compare Bali leasehold, Hak Pakai, PT PMA/HGB, and fractional exposure by rights, eligibility, control, cost, liquidity, and risk before investing.` |
| URL slug | PASS | `leasehold-hak-pakai-pt-pma-fractional-exposure-bali` is clean, lowercase, keyword-rich, and under 75 characters. | No change required. |
| Heading structure | WARN | One H1 and no skipped heading levels. However, 17 H2s includes author-only sections that should not publish. One H2 is 75 characters. | Remove `Internal Link Suggestions`, `External Source Needs`, `Visual and Chart Ideas`, `Fact-Check Notes`, and `Compliance Notes` from publish body; shorten the quick-comparison H2. |
| Internal links | FAIL | Body has 0 real markdown links. Suggestions are listed as notes instead of embedded links. | Add 3-5 contextual links in the body to live posts, including `/blog/real-estate-101`, `/blog/understanding-real-estate-tokenization`, `/blog/bali-property-due-diligence-checklist`, `/blog/can-foreigners-buy-property-bali-2026`, and `/blog/how-to-own-bali-villa-without-500000` where live. |
| External links | FAIL | Source URLs are parked in a note section, not cited inline where claims appear. | Add inline citations near the land-rights and BPS claims; cite only authoritative sources. |
| Image SEO | FAIL | No hero image or inline image exists, and no alt text can be audited. | Add an approved hero image with descriptive alt text, dimensions, and WebP/AVIF variant. Add at least one comparison/risk chart if design capacity allows. |
| Schema | FAIL | No schema fields beyond draft metadata; no `canonical_url`, `og_image_url`, author, category, or FAQ data payload are present. | Add BlogPosting metadata, canonical URL, OG image, author, category/tags, and FAQ data if the CMS requires structured FAQ ingestion. |
| AI citation readiness | WARN | Strong comparison tables and Q&A headings. Weakness: citations are not inline, no 40-60 word answer capsules, and final author notes dilute extraction. | Add sourced answer-first capsules under major H2s and keep the FAQ concise. |
| Factual support | FAIL | BPS tourism numbers and broad BPK regulation existence are verifiable. Practical legal/product claims are not fully source-backed. | Inline official sources and attach counsel/product approvals for high-risk claims. |
| CTA clarity | WARN | CTA is educational and low-risk, but appears mid-article as an H3 rather than a conversion path. | Convert to a clear in-body CTA module after risk/comparison sections; keep wording document-led and non-advisory. |
| Conversion path | WARN | Draft points investors to documents and risk disclosures, but does not define the next safe product action. | Add a compliant next step such as `Compare current POOOL project documents and risk disclosures` without implying suitability or returns. |
| Compliance risk | FAIL | Subject matter includes Indonesian property rules, legal enforceability, tax, ownership, liquidity, fractional rights, and investment uncertainty. | Treat as blocked until legal/product sign-off is recorded. |

## Hard Blockers

1. **Legal review is explicitly unresolved.** Lines 27 and 319-324 say Indonesian counsel review is required before publication. That is a hard blocker because the article explains foreign eligibility, Hak Pakai, HGB, PT PMA, leasehold enforceability, nominee risk, transferability, and permitted use.

2. **Product/legal review is unresolved for fractional exposure.** Lines 127-140 and 217-228 describe SPV/holding structures, economic rights, distributions, reporting, and exit/liquidity terms, but line 320 says internal POOOL review is still needed. Do not publish until approved language exists for investor rights, certificates/tokens if any, fees, distributions, platform role, escrow if mentioned in final copy, and liquidity limits.

3. **External sources are not integrated into the article body.** Lines 296-304 list source needs, but the body has no markdown citations. For legal/investment content, parked source notes are not enough.

4. **Draft-only production notes would publish as article content.** Lines 288-328 are internal workflow notes, not reader-facing copy. Remove or convert them into metadata/editorial tasks before publication.

5. **No image/OG asset exists.** A publishable blog post needs at least a hero image, alt text, `og_image_url`, and image dimensions for social sharing and image SEO.

## Source Verification

| Claim | Status | Evidence |
|---|---:|---|
| Law No. 5 of 1960 exists and is current in BPK database | VERIFIED | BPK lists `Undang-undang (UU) Nomor 5 Tahun 1960 tentang Peraturan Dasar Pokok-Pokok Agraria`, status `Berlaku`, effective 24 September 1960: https://peraturan.bpk.go.id/Home/Details/51310/uu-no-5-tahun-1960 |
| Government Regulation No. 18 of 2021 covers land rights, apartment units, and land registration | VERIFIED | BPK lists `PP Nomor 18 Tahun 2021` on `Hak Pengelolaan, Hak Atas Tanah, Satuan Rumah Susun, dan Pendaftaran Tanah`, status `Berlaku`: https://peraturan.bpk.go.id/Home/Details/161848/pp-no-18-tahun-2021 |
| BPS Bali reported 492,289 direct foreign tourist visits in February 2026 and 55.44% star-rated hotel occupancy | VERIFIED | BPS Bali April 1, 2026 release reports both figures: https://bali.bps.go.id/en/pressrelease/2026/04/01/718028/tourism-overview-of-bali-province--february-2026.html |
| BPS Bali reported February 2026 direct foreign tourist visits increased 9.23% year over year | VERIFIED | BPS Bali April 1, 2026 news item reports the 9.23% YoY increase: https://bali.bps.go.id/en/news/2026/04/01/354/kunjungan-wisman-ke-bali-pada-februari-2026-meningkat.html |
| Practical claims about foreign eligibility, transfer, mortgage, inheritance, rental use, company route, PT PMA/HGB suitability, and enforceability | UNVERIFIED FOR PUBLICATION | Official regulations support the topic area, but the article's practical guidance needs counsel-approved interpretation and inline source/counsel notes. |
| POOOL fractional exposure documents, economic rights, fee/distribution policy, reporting, and exit limits | UNVERIFIED FOR PUBLICATION | Requires POOOL product/legal source of truth and approved disclosure language. |

## Blog Quality Score

| Category | Score | Max | Notes |
|---|---:|---:|---|
| Content Quality | 24 | 30 | Clear, comprehensive comparison with useful tables; draft-only notes need removal. |
| SEO Optimization | 14 | 25 | Good keyword/title/slug; fails links, meta length, source integration. |
| E-E-A-T Signals | 6 | 15 | Strong caution language, but no named author/credentials and no inline authoritative citations. |
| Technical Elements | 5 | 15 | No image asset, no canonical/OG image metadata, no schema payload. |
| AI Citation Readiness | 10 | 15 | Good Q&A/table structure; inline sourcing and citation capsules missing. |
| **Total** | **59** | **100** | **Below standard until blockers are fixed.** |

## SEO And AEO Notes

- **Primary keyword:** Present in title/H1/frontmatter, but exact phrase density is low in body. This is acceptable because the topic has many proper-noun variants.
- **Search intent:** Good comparison intent match. The article answers "which structure fits me?" without pretending to give personalized advice.
- **AEO/GEO:** The article has 9 question-format H2s and 5 FAQ questions, which helps answer extraction. It needs source-backed answer capsules under the legal and market-data sections.
- **Tables:** The comparison, scenario, and risk tables are useful for snippets and AI extraction. Ensure final renderer outputs semantic table headers.
- **Appendices:** Editorial notes should not be indexed as article sections.

## Schema Opportunities

Add or confirm CMS fields for:

- `schema_type: BlogPosting`
- `canonical_url: https://www.poool.app/blog/leasehold-hak-pakai-pt-pma-fractional-exposure-bali`
- `og_image_url: [approved absolute hero image URL]`
- `author: [named reviewer/POOOL editorial entity with credentials or reviewed-by field]`
- `category: Bali Property Investing` or equivalent
- FAQ data for the five FAQ questions, only if POOOL intentionally uses FAQ schema for AI parsing. Do not rely on FAQ rich results because Google restricts FAQ rich results mostly to government and health authority sites.
- Consider `reviewedBy` or visible editorial review text if the CMS/schema pipeline supports it and legal counsel approval is obtained.

## Exact Next Fixes

1. Get Indonesian counsel approval for all land-rights and foreign-eligibility statements, especially lines 39-49, 64-82, 84-101, 103-121, 152-157, 165-173, and 253-271.
2. Get POOOL product/legal approval for fractional exposure language, especially lines 123-140, 217-228, 286, and any final CTA.
3. Move source URLs from lines 296-304 into inline citations near the relevant claims. Remove the `External Source Needs` section before publishing.
4. Remove `Internal Link Suggestions`, `Visual and Chart Ideas`, `Fact-Check Notes`, and `Compliance Notes` from the public body after converting their tasks into real links/assets/metadata.
5. Add 3-5 contextual internal links in the article body. Prefer links to already-published POOOL explainers and the April 25 due-diligence article once published.
6. Expand the meta description to 150-160 characters and add `canonical_url`, `og_image_url`, tags/category, and author/reviewer fields.
7. Add an approved hero image with descriptive alt text, width/height, and WebP/AVIF variant. Suggested alt: `Bali villa documents and comparison checklist for leasehold, Hak Pakai, PT PMA, and fractional exposure`.
8. Add a short, compliant CTA module after the risk/tradeoff section: invite readers to compare project documents and risk disclosures, not to choose a personalized investment path.
9. Add a visible disclaimer near the top and near the CTA that the article is educational and not legal, tax, financial, or investment advice.
10. Re-run QA after counsel/product sign-off and source integration.

## Manual Publish Decision

**Not safe to manually publish today.** The right status is:

```yaml
qa_status: blocked
approved_for_distribution: false
```

The draft can move to publish review only after legal/product approval, inline source integration, removal of editorial notes, image/metadata completion, and a second QA pass.
