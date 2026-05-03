# 🌍 POOOL Community — Expert Deep Dive & Brainstorm

> **Status:** Brainstorm / Ideation — Not yet in development  
> **Last updated:** 2026-03-12  
> **Author:** Martin + AI  

---

## 1. Why Community? — The Strategic Case

### 1.1 The Problem

Poool is currently a **transactional platform** — investors come, buy commodity shares, and leave. There's no reason to return until a dividend is paid. This creates:

- **Low engagement between investments** — the app goes dormant
- **No network effects** — each user exists in isolation
- **High acquisition cost** — you can't rely on organic, word-of-mouth growth
- **Zero switching cost** — if a competitor offers slightly better yields, investors leave

### 1.2 The Opportunity

A community layer transforms Poool from a **transaction tool** into a **destination**. The best fintech platforms (eToro, Public.com, Wealthsimple) have proven that social features:

- **Increase DAU/MAU by 3-5x** — users come back to check the feed, not just their portfolio
- **Reduce churn by 30-40%** — social connections create lock-in (people don't leave their friends)
- **Lower CAC by 50%+** — community members recruit each other
- **Increase AUM per user** — social proof drives larger and more frequent investments

### 1.3 Poool's Unique Advantage

Unlike stock trading apps, Poool's commodity investments are:
- **Tangible** — real cocoa farms, timber plantations, actual harvests
- **Story-rich** — each commodity has a narrative (planting → growing → harvest → payout)
- **Long-hold** — 12+ month terms create a natural community timeline
- **Mission-driven** — sustainable agriculture appeals to values-based investors

This means the community content writes itself: farm updates, harvest photos, impact reports, fellow investors sharing their "why."

---

## 2. Existing Platform Assets to Leverage

> Before building anything new, here's what Poool already has that the community can plug into.

### 2.1 Referral & Rewards System ✅
Already built: referral codes, click tracking, signup tracking, qualified investor tracking, campaign metrics, tier system (Intro → Bronze → Silver → Gold → Premium), cashback, commission payouts.

**Community integration:** Referral circles, social leaderboards, group challenges, circle-based rewards.

### 2.2 Leaderboard ✅
Already built: leaderboard page with score bars, yearly rankings.

**Community integration:** Community rankings, top contributor badges, "investor of the month."

### 2.3 User Profiles ✅ (Partial)
Already built: `user_profiles` table (first name, last name, display name, avatar, country, nationality), `user_settings` (language, notifications), KYC verification status, tier info.

**Community integration:** These become the foundation of public investor profiles. Add bio, interests, investment philosophy.

### 2.4 Notifications ✅
Already built: notification system with types (kyc, investment, payout, system, promo), read/unread, action URLs.

**Community integration:** Add community notification types (new follower, post reply, milestone unlocked, AMA starting).

### 2.5 Assets & Investments ✅
Already built: 26-table database schema covering assets, investments, dividends, milestones.

**Community integration:** Asset-specific discussion threads, community reviews per commodity, investment milestone celebrations.

### 2.6 Blog System ✅
Already built: blog with articles, table of contents, social sharing.

**Community integration:** Transform blog into community-contributed content, AMA archives, "investor stories."

---

## 3. Feature Deep Dives

### 3.1 👤 Investor Profiles

**The vision:** A public-facing profile that builds trust and identity without exposing financial details.

#### What's visible (user controls each toggle):

| Field | Source | Default Visibility |
|-------|--------|--------------------|
| Display name | `user_profiles.display_name` | ✅ Public |
| Avatar | `users.avatar_url` | ✅ Public |
| Country flag | `user_profiles.country` | ✅ Public |
| Member since | `users.created_at` | ✅ Public |
| Tier badge (Bronze/Silver/Gold/Premium) | `user_tiers` | ✅ Public |
| KYC verified badge | `kyc_records.status = 'approved'` | ✅ Public |
| Bio (new field) | New | ✅ Public |
| Investment interests (new) | New | ✅ Public |
| Investment philosophy (new) | New | ✅ Public |
| Number of investments | Derived from `investments` count | 🔒 Optional |
| Commodity categories invested in | Derived (without amounts) | 🔒 Optional |
| Follower / following count | New | ✅ Public |
| Badges & achievements | New | ✅ Public |

#### What's NEVER visible:

- Portfolio value, individual investment amounts
- Personal details (real name if different from display, DOB, address, tax ID)
- KYC documents
- Wallet balances

#### Expert Recommendations:

1. **Anonymous mode** — Let users participate under a pseudonym. Some investors want community without identity. Think "CacaoKing_42" instead of "Martin F." This massively increases participation rates.

2. **Progressive disclosure** — New users start with a minimal profile. As they engage more, prompts encourage them to add a bio, photo, interests. Don't force everything upfront.

3. **Profile completeness score** — Gamify profile completion: "Your profile is 60% complete. Add a bio to unlock the ability to post in the feed."

4. **Investor archetypes** — Let users self-select a tag:
   - 🌱 Sustainable Investor
   - 📈 Growth Seeker
   - 💎 Long-term Holder
   - 🔬 Research-driven
   - 🌍 Impact First
   - 🆕 Curious Beginner

   These become excellent filters for the feed and for finding like-minded investors.

5. **Trust signals** — Stack visual indicators of trust:
   - ✅ KYC Verified
   - 🏆 Gold Tier
   - 📅 Member since 2025
   - 🌾 Invested in 5+ commodities
   - 🤝 Referred 10+ investors

---

### 3.2 📰 Activity Feed

**The vision:** The heart of the community — a scrollable feed that keeps investors coming back daily.

#### Feed Content Types (prioritized):

| Priority | Content Type | Source | Example |
|----------|-------------|--------|---------|
| 🔴 High | Platform announcements | Admin-created | "New cocoa opportunity launching next Monday" |
| 🔴 High | Commodity updates | Developer-created | "Month 3 harvest report: 2.4 tons of dried vanilla processed" |
| 🟡 Medium | Milestone celebrations | Auto-generated (opt-in) | "🎉 Sarah just made her 5th investment!" |
| 🟡 Medium | Community reviews | User-created | "My experience investing in the Bali cocoa farm..." |
| 🟡 Medium | Investment tips | User-created | "3 things I learned about timber investing" |
| 🟢 Low | Market sentiment | Aggregated | "📊 Cocoa is trending — 12 new investments this week" |
| 🟢 Low | Expert content | Curated | AMA highlights, educational articles |
| 🟢 Low | Social interactions | Auto-generated | "Martin started following you" |

#### Feed Algorithms — Two Modes:

1. **Chronological** (default) — Shows everything in time order. Builds trust because users know nothing is hidden or deprioritized.

2. **"For You"** (opt-in) — Ranked by relevance based on:
   - Commodities you've invested in
   - People you follow
   - Your investor archetype
   - Engagement signals (liked posts get boosted)

#### Engagement Mechanics:

- **Reactions** — Not just 👍. Use investment-themed reactions:
  - 🌱 Growing (support/encouragement)
  - 💡 Insightful (valuable information)
  - 🔥 Exciting (hype)
  - 🤝 Thank you (gratitude)
  - 📊 Data-driven (analytical posts)

- **Comments** — Threaded, with @mentions. Rate-limited for new users to prevent spam.

- **Bookmarks** — Save posts privately for later reference.

- **Share** — Share to the Poool feed (repost) or externally (link + OpenGraph preview).

#### Expert Recommendations:

1. **"Cold start" problem** — The biggest risk. An empty feed kills the feature. Solutions:
   - Pre-populate with admin-written commodity updates, team posts, and educational content
   - Automatically create milestone posts for existing users' past achievements
   - Create 5-10 "seed" investor stories before launch
   - The Poool team should actively post daily for the first 3 months

2. **Content quality over quantity** — A feed with 100 low-quality posts is worse than 5 great ones. Consider:
   - Post length minimum (at least 50 characters) to prevent "nice" spam
   - Featured/pinned posts by moderators
   - "Quality contributor" badges for consistently insightful posts

3. **Don't copy Twitter/Reddit** — The feed should feel like a premium investor community, not a social network. Think:
   - Long-form posts encouraged (like LinkedIn, not Twitter)
   - No viral mechanics (no public share counts)
   - No doom-scrolling dark patterns
   - Clean, spacious design (like you already have on the platform)

4. **Commodity-specific feeds** — Each commodity/asset gets its own sub-feed. When you open the cocoa fund page, there's a "Community" tab showing all posts tagged with that commodity. This creates focused, high-value discussions.

---

### 3.3 🔗 Referral Circles

**The vision:** Evolve the existing referral system from a solo activity into a social, team-based experience.

#### Current State (already built):
- Referral codes ✅
- Click & signup tracking ✅
- Qualified investor tracking ($1,000 threshold) ✅
- Commission payouts ✅
- Campaign metrics (subid tracking) ✅

#### Proposed Evolution:

**Circles** = Named groups of investors linked by referral relationships.

```
┌─────────────────────────────────────┐
│     🌿 Martin's Green Circle        │
│                                     │
│  👤 Martin (Founder)                │
│  ├── 👤 Anna (Tier: Gold)          │
│  │   ├── 👤 Tom                    │
│  │   └── 👤 Lisa                   │
│  ├── 👤 Felix (Tier: Silver)       │
│  └── 👤 Sarah (Tier: Bronze)       │
│                                     │
│  📊 Circle Stats:                   │
│  Total Members: 6                   │
│  Combined AUM: visible only to      │
│  founder (privacy)                  │
│  Active Investors: 5/6              │
│                                     │
│  🏆 Circle Rank: #12 of 89         │
└─────────────────────────────────────┘
```

#### Circle Features:

1. **Circle Dashboard** — The circle founder sees aggregated (not individual) metrics:
   - Total members
   - Active investors count
   - Circle rank on the global leaderboard
   - Recent circle activity

2. **Circle Chat** — Lightweight group messaging for circle members:
   - Circle founder can post announcements
   - Members can share tips and discuss
   - No push notifications by default (to avoid spam)

3. **Circle Challenges** — Time-limited group goals:
   - "Grow your circle to 10 members by April → everyone gets 2% bonus cashback"
   - "Circle members collectively invest $50K → unlock exclusive commodity access"
   - Challenges are set by the Poool team, not user-created (to maintain quality)

4. **Circle Leaderboard** — A dedicated leaderboard ranking circles by:
   - Total members
   - Activity score
   - Combined investment milestones (without revealing amounts)

#### Expert Recommendations:

1. **Keep it lightweight** — Circles should feel like a bonus feature, not a second job. Founders shouldn't have to "manage" their circle.

2. **Auto-create circles** — Every user with referrals automatically gets a circle. No manual setup needed.

3. **Privacy-first** — Circle members can see each other's profiles (if public) but never each other's investment amounts.

4. **Cross-pollination** — Surface interesting posts from circle members in the main feed with a "From your circle" tag.

---

### 3.4 🎓 Expert AMAs (Ask Me Anything)

**The vision:** Scheduled, curated sessions where commodity experts, farmers, financial advisors, or the Poool team answer community questions.

#### AMA Format:

```
┌─────────────────────────────────────┐
│  🎓 UPCOMING AMA                    │
│                                     │
│  "Understanding Cocoa Supply Chains" │
│  with Dr. Ana Suarez                │
│  (Agricultural Economist, 15yr exp)  │
│                                     │
│  📅 March 20, 2026 — 7:00 PM CET   │
│  ⏱ 45 minutes                       │
│                                     │
│  [Submit a Question]  [Set Reminder] │
│                                     │
│  📊 23 questions submitted          │
│  👥 87 RSVPs                        │
└─────────────────────────────────────┘
```

#### AMA Flow:

1. **Announcement phase** (1 week before) — Admin creates AMA, community is notified
2. **Question submission** (opens immediately) — Users submit and upvote questions
3. **Live session** — Expert answers top-voted questions; text-based, not video (lower friction)
4. **Archive** — AMA is saved as a searchable, formatted post in the community/blog

#### Expert Types:

| Type | Examples | Frequency |
|------|----------|-----------|
| 🌾 Commodity Experts | Cocoa farmer, timber specialist, vanillin processor | Monthly |
| 💼 Financial Advisors | Commodity investment strategy, portfolio diversification | Quarterly |
| 👨‍💻 Poool Team | Platform roadmap, new features, behind-the-scenes | Bi-weekly |
| 🏆 Top Investors | Share strategy, portfolio approach, lessons learned | Monthly |
| 🏛 Regulatory Experts | Tax implications, compliance updates | Quarterly |

#### Expert Recommendations:

1. **Text-based, not video** — Lower barrier to entry for both experts and participants. Video AMAs require scheduling, streaming infrastructure, and timezone coordination. Text AMAs can be async (expert answers at their own pace within a window).

2. **Async option** — Allow AMAs to run over 24-48 hours instead of a live window. This solves timezone issues for a global investor base.

3. **Repurpose content** — Every AMA should automatically become:
   - A blog post ("Top insights from our cocoa expert AMA")
   - An educational resource linked from the commodity page
   - Social media content (quotes, key takeaways)

4. **Start with the Poool team** — Don't wait for external experts. The first 5 AMAs should be with Martin or the team: "Ask us anything about how Poool works," "Our vision for 2026," "How we select commodities."

---

### 3.5 💡 Investment Tips & Reviews

**The vision:** User-generated content that creates a knowledge base around each commodity.

#### Community Reviews:

```
┌─────────────────────────────────────┐
│  ⭐⭐⭐⭐⭐  "Solid first investment"  │
│                                     │
│  👤 Anna K. — Gold Tier | Verified  │
│  🌾 Bali Premium Cocoa Fund         │
│  📅 Invested: 6 months ago          │
│                                     │
│  "I invested in this fund as my     │
│   first commodity. The monthly       │
│   updates from the farm were         │
│   excellent — photos of the          │
│   plantation, harvest progress.      │
│   Just received my first dividend    │
│   payout. Would recommend for        │
│   beginners..."                      │
│                                     │
│  💡 12 found this helpful            │
│  📊 5 replies                        │
└─────────────────────────────────────┘
```

#### Review Rules:

- **Only investors can review** — You can only review a commodity you've actually invested in (verified against `investments` table)
- **One review per commodity** — Can be updated, not duplicated
- **Minimum 100 characters** — No low-effort reviews
- **Star rating** — 1-5 stars on: Communication, Transparency, Returns vs Promise
- **Moderation** — Reviews are visible immediately but flaggable for moderation

#### "My Why" Stories:

A special post type where investors share why they chose a specific commodity:

- Prompted after first investment: "Tell the community why you invested in [commodity]. Your story might inspire others."
- Displayed on the commodity detail page under a "Why investors chose this" section
- Powerful social proof for new investors browsing the marketplace

#### Expert Recommendations:

1. **Incentivize quality reviews** — Offer 50 reward points for detailed reviews (250+ characters with specific details). Don't pay for star ratings alone (creates fake reviews).

2. **Review verification badges** — 
   - "✅ Verified Investor" (owns the commodity)
   - "📊 Received Dividend" (has gotten at least one payout)
   - "📅 6+ Month Holder" (long-term holder perspective)

3. **Developer responses** — Allow commodity developers/operators to respond to reviews. This creates a dialogue and builds trust.

4. **Aggregate ratings** — Show a summary on each commodity card: "4.7 ⭐ from 23 investors" — social proof in the marketplace.

---

### 3.6 🔔 Announcements Feed

**The vision:** Replace one-off popup modals with a persistent, categorized news feed.

#### Categories:

| Category | Icon | Example |
|----------|------|---------|
| New Commodities | 🌿 | "New opportunity: Sustainable timber plantation in Kalimantan" |
| Dividend News | 💰 | "Q1 2026 dividends distributed — check your wallet" |
| Platform Updates | 🛠 | "New feature: Portfolio analytics dashboard" |
| Community Highlights | 🏆 | "Community milestone: 1,000 investors and growing!" |
| Market Insights | 📊 | "Global cocoa prices up 15% — what it means for your investments" |
| Regulatory Updates | ⚖️ | "New tax guidelines for commodity investments in EU" |
| Events | 📅 | "Upcoming AMA with our cocoa farm partner — March 20" |

#### Design Concept:

- Each announcement can have: title, rich text, images, embedded video, call-to-action button
- Priority system: 🔴 Urgent | 🟡 Important | 🟢 Informational
- Users can subscribe/unsubscribe from categories
- Comment threads on each announcement for Q&A
- Push notification settings per category

#### The "What's New?" Changelog:

A dedicated page (accessed from the sidebar or the modal button) showing ALL platform updates chronologically. Think: Notion's changelog, Linear's changelog.

```
📅 March 2026
├── 🛠 Community feature launched (v2.1)
├── 🌿 New: Sustainable Timber Plantation
├── 💰 Q4 2025 dividends distributed
└── 📊 Portfolio analytics redesigned

📅 February 2026
├── 🛠 Mobile-responsive redesign
├── 🌿 New: Ghana Premium Cocoa II
└── 🎓 First Expert AMA held
```

---

### 3.7 🏅 Milestone Celebrations

**The vision:** Make every investment journey feel rewarding and share-worthy.

#### Milestone System:

| Milestone | Badge | Trigger | Community Post |
|-----------|-------|---------|----------------|
| First Investment | 🎉 "First Step" | `investments` count = 1 | "🎉 [Name] just made their first investment!" |
| 5 Investments | 🌱 "Growing Roots" | `investments` count = 5 | "🌱 [Name] is growing their portfolio!" |
| First Dividend | 💰 "First Harvest" | `dividend_payouts` count = 1 | "💰 [Name] received their first dividend!" |
| $1K Invested | 📈 "Thousand Club" | Total >= $1,000 | Opt-in shoutout |
| $10K Invested | 💎 "Diamond Hands" | Total >= $10,000 | Opt-in shoutout |
| $100K Invested | 🏛 "Institution" | Total >= $100,000 | Private badge, no public post |
| 1 Year Member | 📅 "Founding Member" | `created_at` ≥ 1 year | Anniversary recap card |
| 5 Referrals | 🤝 "Connector" | Referrals ≥ 5 | "🤝 [Name] has grown their circle!" |
| 25 Referrals | 🌐 "Network Builder" | Referrals ≥ 25 | Exclusive badge |
| KYC Verified | ✅ "Verified" | `kyc_records.status = approved` | Silent badge (no feed post) |
| Multi-Commodity | 🌍 "Diversified" | 3+ unique `asset_id` in investments | "🌍 [Name] is diversifying across commodities!" |
| Community Contributor | ✍️ "Thought Leader" | 10+ posts with 50+ combined reactions | Special community badge |

#### Celebration UX:

1. **In-app moment** — When a milestone is hit, show a beautiful full-screen animation (confetti, badge reveal) with options to:
   - Share to the community feed (optional)
   - Share externally (generates a branded card image)
   - Dismiss

2. **Year-in-review** — Annual recap card (December/January):
   - Total invested, dividends earned
   - Number of commodities
   - Community contributions
   - Tier progress
   - Beautiful, Instagram-shareable design

#### Expert Recommendations:

1. **Never auto-post financial milestones** — The "$10K invested" milestone should be opt-in only. Some investors don't want to broadcast their wealth.

2. **Make badges visible on profiles** — Users should be able to pin their top 3 badges on their profile.

3. **Seasonal/limited badges** — Create time-limited badges: "🎄 Holiday Investor 2026" (invested during Dec), "🌍 Earth Day Investor" (invested in sustainable commodities in April). These create FOMO and urgency.

4. **Progressive badge design** — Each badge should have levels: Bronze → Silver → Gold → Platinum. "First Harvest" badge upgrades as you receive more dividends.

---

### 3.8 💬 Direct Messaging

**The vision:** Private, trust-gated communication between investors.

#### Access Levels:

| Level | Who | Can message |
|-------|-----|-------------|
| Level 0 | New users (< 30 days, no investment) | Nobody (DMs locked) |
| Level 1 | Verified investors (KYC + 1 investment) | People they follow who follow them back |
| Level 2 | Active investors (3+ investments, 90+ days) | Anyone who has DMs enabled |
| Level 3 | Premium tier | Anyone + priority delivery |

#### Safety Features:

- **Request-based** — Messages from non-connections go to a "Requests" folder
- **Rate limiting** — Max 10 new conversations per day
- **Block/report** — One-click block, with option to report for review
- **No file sharing** — Text only (prevents phishing/scam documents)
- **Auto-expire** — Message requests expire after 14 days if not accepted
- **No investment solicitation** — Clear ToS: using DMs to solicit investments is a bannable offense
- **Admin moderation tools** — Flag suspicious patterns (mass messaging, link spam)

#### Message Types:

- 💬 Text messages
- 🔗 Commodity page links (renders as rich preview)
- 📰 Feed post shares (renders inline)
- 🤝 Circle invitations

#### Expert Recommendations:

1. **Launch last** — DMs are the highest-risk feature (scams, harassment, regulatory risk). Build trust with the other community features first.

2. **Investor-to-Developer channel** — A moderated way for investors to ask commodity operators questions. Not free-form DMs, but structured Q&A (like support tickets but public answers visible to all investors in that commodity).

3. **Consider not building DMs at all** — Many successful communities thrive without private messaging. The feed + commodity discussions + circle chat might be enough. Evaluate after launching the other features.

---

## 4. Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Empty feed / no engagement** | 🔴 Critical | Seed content before launch; team posts daily for 3 months; auto-generate milestone posts for existing users |
| **Spam / low-quality content** | 🟡 High | Rate limits, character minimums, moderation queue, report system |
| **Scams via DMs** | 🟡 High | Delay DM launch, strict access gating, ban investment solicitation |
| **Privacy concerns** | 🟡 High | All profile fields opt-in, never expose financial details, anonymous mode available |
| **Regulatory risk (investment advice)** | 🟡 High | Clear disclaimers: "Community content is not financial advice." Flag posts that look like financial advice. |
| **Moderation overhead** | 🟡 Medium | Start with community reporting + admin review; add automated moderation later if needed |
| **Feature bloat** | 🟡 Medium | Phase the rollout; each phase must prove value before the next ships |
| **Toxicity** | 🟢 Low | Investment communities tend to be less toxic than general social; real-money users are more accountable |

---

## 5. Monetization Opportunities

The community isn't just a cost center — it can generate revenue:

1. **Premium community features** — Part of higher tiers:
   - Gold/Premium users get early access to AMA question slots
   - Premium badge customization
   - Advanced feed filters

2. **Featured commodities in feed** — Developers/operators can pay to boost their commodity updates (clearly labeled as "Sponsored")

3. **Exclusive investment access** — Certain commodities only available to active community members (posted X times, verified profile, etc.)

4. **Data insights** — Aggregated, anonymized community sentiment data sold to commodity operators: "82% of our investor community is interested in sustainable timber."

---

## 6. Technical Considerations (High Level)

> Not a technical spec — just strategic notes for when we're ready to build.

### New Backend Module
```
backend/src/community/
├── mod.rs
├── models.rs      # Posts, comments, reactions, follows, badges, AMAs
├── routes.rs      # Feed, profiles, messaging, moderation endpoints
├── service.rs     # Feed ranking, milestone detection, notification triggers
└── moderation.rs  # Content filtering, rate limiting, reporting
```

### New Database Tables (estimated)
- `community_profiles` — Bio, interests, archetype, privacy settings
- `posts` — Feed posts (text, type, commodity_id link)
- `comments` — Threaded comments on posts
- `reactions` — Post reactions (type-based)
- `follows` — User-to-user follow relationships
- `badges` — Badge definitions
- `user_badges` — Earned badges per user
- `milestones` — Milestone definitions
- `user_milestones` — Triggered milestones per user
- `amas` — AMA sessions
- `ama_questions` — Submitted + upvoted questions
- `reviews` — Commodity reviews (rating + text)
- `messages` — Direct messages (if we build DMs)
- `message_threads` — Conversation threads
- `reports` — Content/user reports

### Integration Points with Existing System
- **Notifications** → New types: community_mention, community_reply, milestone_unlocked, ama_reminder
- **Rewards/Tiers** → Badges sync with tier system; invite community events
- **Leaderboard** → Community contribution score integrated into leaderboard  
- **Investments** → Milestone triggers fire on investment creation/dividend payout
- **Settings** → Community privacy preferences added to user settings page

### Estimated Effort
| Phase | Effort | New DB Tables | New API Routes | New HTML Pages |
|-------|--------|---------------|----------------|----------------|
| Phase 1 (Profiles + Feed + Announcements) | 3-4 weeks | ~8 | ~15 | 3-4 |
| Phase 2 (Milestones + Badges + Reviews) | 2-3 weeks | ~5 | ~10 | 2-3 |
| Phase 3 (Circles + AMAs) | 2-3 weeks | ~4 | ~10 | 2-3 |
| Phase 4 (DMs) | 2-3 weeks | ~3 | ~8 | 1-2 |
| **Total** | **9-13 weeks** | **~20** | **~43** | **8-12** |

---

## 7. Success Metrics

How do we know the community is working?

| Metric | Target (6 months post-launch) | How to measure |
|--------|-------------------------------|----------------|
| DAU/MAU ratio | > 25% (currently ~5-10% estimated) | Analytics |
| Posts per week | > 50 (organic, non-admin) | `posts` table count |
| Average feed session time | > 3 minutes | Analytics |
| Users with public profiles | > 60% of active investors | `community_profiles` count |
| Review coverage | > 50% of commodities have 3+ reviews | `reviews` table |
| Referral conversion (circle effect) | +20% vs non-circle referrals | Compare conversion rates |
| NPS improvement | +15 points | User surveys |
| Investment frequency | +25% (users invest more often due to social proof) | `investments` created_at intervals |

---

## 8. Open Questions for Further Brainstorming

> Let's discuss these before moving to implementation.

1. **Moderation model:** Should we use community-elected moderators (like Reddit), or keep moderation fully internal (admin team)?

2. **Anonymity vs. real identity:** Should we allow fully anonymous profiles, or require at least a display name + avatar? Anonymity increases participation but decreases accountability.

3. **Content language:** The platform supports multiple languages. Should the community feed be one global feed with auto-translate, or language-specific feeds?

4. **Developer (commodity operator) participation:** Should developers/operators have special verified profiles and their own post type? Or treat them like any other user?

5. **Mobile experience:** Community features are most used on mobile. Should we build a PWA or native app considerations? Or keep it mobile-responsive web?

6. **Content outside Poool:** Should community members be able to cross-post to social media? If yes, should Poool watermark shared content (free marketing)?

7. **Voting on new commodities:** Should the community be able to vote/express interest in what commodities Poool should offer next? ("I want to invest in olive oil" — 342 upvotes)

8. **Integration with external communities:** Should Poool have a Discord/Telegram community alongside the in-app community, or keep everything in-platform?

9. **Content moderation AI:** Should we use AI to auto-flag potentially problematic content (financial advice, hate speech, scams), or rely entirely on human moderation?

10. **Notification fatigue:** With feed, DMs, milestones, AMAs, and announcements, how do we prevent notification overload? What's the notification hierarchy?

---

## 9. Competitive Analysis

| Platform | Social Feature | What Works | What Poool Can Learn |
|----------|---------------|------------|---------------------|
| **eToro** | Copy trading + feed | Social proof drives investment; "See what top investors buy" | The follow system + showing portfolio themes (not amounts) |
| **Public.com** | Social feed + reactions | Real-time activity feed showing what people are buying | Trending commodities, milestone celebrations |
| **Wealthsimple** | Community forums | Long-form discussions; educational content | Quality over quantity; moderated discussions |
| **Seedrs** | Investor forums per deal | Deal-specific discussions build conviction | Commodity-specific discussion tabs |
| **Republic** | Investor updates | Founders post monthly updates to investors | Developer/operator monthly updates (already can leverage `asset_milestones`) |
| **Robinhood** | None (removed Snacks) | They removed social features — proof it needs to be done right | Don't bolt on; integrate deeply |

---

## 10. Recommended Rollout Strategy

### Pre-Launch (2-4 weeks before)
- [ ] Seed 20+ high-quality posts (commodity stories, team intros, educational content)
- [ ] Create milestone posts for existing users' achievements retroactively
- [ ] Design the "Community soon" teaser modal (✅ already done per screenshot!)
- [ ] Invite 10-15 power users to a beta community

### Phase 1 — Foundation (Weeks 1-4)
- [ ] Investor profiles (bio, interests, privacy controls)
- [ ] Activity feed (chronological, with reactions)
- [ ] Announcements feed
- [ ] "What's New" changelog page
- [ ] Community entry in sidebar (replace "Soon" badge)

### Phase 2 — Engagement (Weeks 5-7)
- [ ] Milestone & badge system
- [ ] Commodity reviews
- [ ] "My Why" stories on commodity pages
- [ ] Profile completeness gamification

### Phase 3 — Social (Weeks 8-10)
- [ ] Referral circles dashboard
- [ ] Circle chat (lightweight)
- [ ] Expert AMAs (text-based, async)
- [ ] Leaderboard integration with community scores

### Phase 4 — Communication (Weeks 11-13, *if validated*)
- [ ] Direct messaging (gated access)
- [ ] Investor-to-Developer Q&A channel
- [ ] Advanced moderation tools

### Post-Launch Continuous
- [ ] Monitor metrics (Section 7)
- [ ] Iterate on feed algorithm
- [ ] Monthly AMA cadence
- [ ] Community-driven feature requests

---

*This is a living document. Add your thoughts, questions, and ideas below.* ✏️

## 💬 Notes & Ideas

<!-- Add your brainstorming notes here -->


