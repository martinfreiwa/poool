# POOOL Blog Automation Smoke Test

Date: 2026-04-24
Goal: validate whether the current automations can support one blog article per day.
Mode: manual sequential dry run, because the Codex app exposes automation create/update/view but no direct "run now" trigger.

## Automation Run Order

1. POOOL Blog Research & Strategy
2. POOOL Daily Blog Brief & Outline Queue
3. POOOL Daily Blog Draft Builder
4. POOOL Daily Blog Draft QA Gate
5. POOOL Daily Blog Conversion Review
6. POOOL Daily Blog Distribution Campaigns
7. POOOL Audience Objection Mining
8. POOOL Blog Performance Feedback
9. POOOL Authority & Backlink Opportunities
10. POOOL Blog SEO Health Audit

## Local Inputs Found

- Persona exists: `/Users/martin/.codex/skills/blog/references/personas/poool.json`
- Briefs found: 1
  - `/Users/martin/Projects/poool/briefs/bali-property-due-diligence-checklist-brief.md`
- Published blog articles in local DB: 6
- Draft articles in local DB: 0
- Local analytics/Search Console credentials found: none obvious
- Local base URL: `http://localhost:8888`

## Live Research Snapshot

Recent SERP and forum checks show that Bali property investment content is crowded and risk-heavy.

Competitor/content patterns found:

- Due diligence checklist pages are recent and detailed.
- Common strengths: BPN/title verification, leasehold/PT PMA explanations, zoning, PBG/IMB, SLF, rental legality, nominee warnings.
- Common weaknesses: weak fractional-investor angle, limited platform/SPV due diligence explanation, weak cash-flow realism, weak conversion from risk education to responsible next step.

Audience objections found:

- "Can foreigners legally own property in Bali?"
- "Is Bali property actually worth it or just hype?"
- "Are 10-16% ROI claims realistic?"
- "What happens if a leasehold extension is vague?"
- "Can the villa legally operate as short-term rental?"
- "Is Bali oversupplied with villas/apartments?"
- "How do I avoid nominee or license risk?"

## Sequential Smoke Test Results

### 1. POOOL Blog Research & Strategy

Status: PASS WITH DATA LIMITS

What worked:

- Persona is available.
- Niche is clear: trust-led fractional property investment in Bali.
- Current SERP confirms a strong risk/due-diligence angle.
- Current database inventory is readable.

Issues:

- No connected keyword volume/ranking data was found locally.
- No Search Console, GA4, Semrush, Ahrefs, or DataForSEO credentials were found.
- Strategy output can only use SERP/manual research unless those tools are connected.

Fix:

- Connect Search Console/analytics or add a manual keyword export folder the automation can read.

### 2. POOOL Daily Blog Brief & Outline Queue

Status: PASS BUT PIPELINE TOO THIN

What worked:

- One strong brief exists for `Bali property due diligence checklist`.
- The brief has target keywords, search intent, outline, CTA direction, visuals, and compliance notes.

Issues:

- Only 1 brief exists, but one article per day requires at least 7-14 briefs queued.
- The current automation can prepare 1-2 briefs daily, but it starts from a nearly empty queue.
- No explicit file naming/output directory is specified in the automation prompt.

Fix:

- Add a durable output convention:
  - briefs in `/Users/martin/Projects/poool/briefs/`
  - outlines in `/Users/martin/Projects/poool/outlines/`
  - drafts in `/Users/martin/Projects/poool/drafts/`
- Seed 14 initial topics immediately.

### 3. POOOL Daily Blog Draft Builder

Status: PASS FOR FIRST ARTICLE, BLOCKED AFTER THAT

What worked:

- The existing due diligence brief is ready enough to draft.
- The prompt correctly selects one article per run.
- Tone and compliance guardrails are present.

Issues:

- There is no existing `/drafts/` directory or clear draft storage convention.
- The prompt says "Save or report the draft according to existing project conventions when available", but no draft convention was found.
- Without a queue of approved briefs, day 2+ will fail or repeat similar content.

Fix:

- Create a draft convention or update the prompt to always write to `/drafts/YYYY-MM-DD-topic.md`.
- Add duplicate prevention by checking slugs/titles against DB and draft files.

### 4. POOOL Daily Blog Draft QA Gate

Status: PASS, HIGH VALUE

What worked:

- The QA prompt checks SEO, schema, fact support, GEO, image SEO, conversion path, and risky investment/property claims.
- It is correctly manual-publish only.

Issues:

- Existing published article copy contains risky/hype language patterns that should be caught in future QA:
  - "exceptionally difficult to find elsewhere"
  - "no signs of slowing"
  - "legally enforceable" without visible citation in the checked SQL seed content
  - specific yield and appreciation claims that require source verification
- QA currently checks newest draft/generated file, but no canonical draft location exists.

Fix:

- Define the draft directory and require QA to pick the newest unreviewed draft.
- Add a hard blocker for unsupported ROI, legal enforceability, OJK, SPV, escrow, and liquidity claims.

### 5. POOOL Daily Blog Conversion Review

Status: PASS WITH IMPROVEMENT NEEDED

What worked:

- The automation maps articles to business goals: investor signup, developer inquiry, seller inquiry, newsletter signup, trust education, due-diligence support.
- It checks CTA, trust signals, objection handling, lead magnets, and internal conversion links.

Issues:

- Current blog templates have broad CTAs such as "Build your real estate portfolio today" and "Start with as little as $50"; these may be too generic and potentially sensitive for an investment-related product.
- No dedicated lead magnet pages/files were found for due diligence checklist, risk guide, developer funding guide, or seller checklist.

Fix:

- Add one primary CTA per article.
- Build 3 lead magnets:
  - Bali Property Due Diligence Checklist
  - Bali Fractional Investment Risk Guide
  - Developer Funding Readiness Checklist

### 6. POOOL Daily Blog Distribution Campaigns

Status: PASS, NEEDS PUBLISHED/APPROVED ARTICLE SIGNAL

What worked:

- Prompt now creates a real 7-day campaign: LinkedIn founder post, company post, email snippet, hooks, visuals, UTM suggestions, repost plan.

Issues:

- No clear "approved" marker exists for drafts.
- If it uses newest published article, it may repeatedly campaign older posts until daily publishing starts.

Fix:

- Add a simple status convention in draft frontmatter:
  - `status: draft`
  - `qa_status: pass|blocker`
  - `approved_for_distribution: true|false`

### 7. POOOL Audience Objection Mining

Status: PASS

What worked:

- Live SERP/forum research produced strong objections around legal ownership, ROI realism, oversupply, licensing, lease extensions, nominee risk, and trust.

Issues:

- Reddit/forum content is useful but not authoritative. It should feed content ideas, not legal claims.
- Needs a structured objection library so ideas are not lost between runs.

Fix:

- Write objections to `/content-intelligence/poool-objection-map.md` or similar.

### 8. POOOL Blog Performance Feedback

Status: BLOCKED BY DATA

What worked:

- Local DB inventory is available.

Issues:

- No Search Console/GA4/conversion data credentials were found.
- No obvious local performance export files were found.
- Cannot accurately report clicks, impressions, rankings, CTR, signup conversion, or declining pages.

Fix:

- Connect `blog-google`/`seo-google`, or add CSV exports in a known folder:
  - `/analytics/search-console/`
  - `/analytics/ga4/`
  - `/analytics/conversions/`

### 9. POOOL Authority & Backlink Opportunities

Status: PASS WITH TOOL LIMITS

What worked:

- Manual research found possible authority/partnership categories:
  - Bali Expat / Seven Stones property content
  - Bali.com investment guide pages
  - Emerhub Indonesia legal/property investment content
  - AREBI Bali
  - REI Bali
  - Bali property/developer blogs
  - Indonesia/Bali expat publications and newsletters

Issues:

- No backlink database credentials were found, so this cannot score domain authority, competitor link gaps, or link velocity accurately.
- Outreach sending is intentionally disabled, which is correct.

Fix:

- Connect backlink data or keep this as a manual opportunity list.
- Create a relationship-first outreach tracker, not automated cold sending.

### 10. POOOL Blog SEO Health Audit

Status: PARTIAL PASS

What worked:

- Local blog routes/templates and DB articles are available.
- Basic inventory and content-risk review can run.

Issues:

- Full SEO health requires a running site crawl, sitemap check, Search Console, PageSpeed/CrUX, and preferably production domain.
- `BASE_URL` is local only.
- No production domain was found in local env.

Fix:

- Add production domain to env or automation prompt.
- Run monthly audit against both local preview and production.

## Biggest Issues Found

1. The daily publishing goal is currently under-supplied.
   - Need 14 briefs/outlines ready, not 1.

2. There is no durable content pipeline directory convention.
   - Need `briefs/`, `outlines/`, `drafts/`, `qa-reports/`, `distribution/`, `content-intelligence/`.

3. Analytics/performance automation is blocked.
   - Need Google Search Console, GA4, or manual exports.

4. Current published content likely needs compliance cleanup.
   - Some ROI/legal/liquidity language is too confident without visible source support.

5. Distribution needs an "approved article" signal.
   - Without it, it may campaign stale or unapproved content.

## Recommended Immediate Fixes

1. Update automation prompts to use fixed output directories.
2. Create an initial 14-day topic backlog.
3. Add hard QA blockers for unsupported legal/ROI/OJK/SPV/escrow/liquidity claims.
4. Add a performance data input folder for CSV exports until APIs are connected.
5. Add frontmatter status fields to all generated drafts.

## Initial 14-Day Topic Backlog

1. Bali Property Due Diligence Checklist
2. Can Foreigners Own Property in Bali?
3. Leasehold vs Hak Pakai vs PT PMA in Bali
4. Bali Villa ROI: Gross Yield vs Net Investor Return
5. Bali Property Red Flags Investors Should Check First
6. What Is Fractional Property Investment?
7. How POOOL Reviews a Property Before Listing It
8. Bali Villa Rental Licensing: What Investors Should Know
9. SPV Structures for Fractional Real Estate Explained
10. Bali Property Exit Risk and Liquidity Explained
11. How Developers Can Raise Capital for Bali Projects
12. How to Sell a Bali Property or Business to Investors
13. Bali Tourism Demand and Property Investment Risk
14. Fractional Property Investment vs Buying a Villa Directly
