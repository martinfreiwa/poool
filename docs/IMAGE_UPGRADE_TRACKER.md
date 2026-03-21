# Asset Image Upgrade Tracker

> Track which assets have been upgraded with AI-generated images.
> **Rule:** Only upgrade `published = true` assets. Skip drafts/test assets.
> **Multi-agent:** Agents MUST claim assets before working. Check CLAIMED and COMPLETED before starting.

---

## ✅ COMPLETED

| Asset | Slug | Agent | Images | Date |
|-------|------|-------|--------|------|
| Sample Asset - Grand Resort | `sample-asset-grand-resort-38040b` | manual | 11 | 2026-03-21 |
| Martins Villa | `martins-villa-ffea037b` | manual | 12 | 2026-03-21 |
| Boutique Resort with 6 Villas in Central Ubud | `boutique-resort-ubud` | agent-x7k2 | 12 | 2026-03-21 |
| Sunset Heights Villa | `sunset-heights-villa` | agent-r8f3 | 8 | 2026-03-21 |
| Uluwatu Luxury Retreat | `uluwatu-luxury-retreat` | agent-p9m4 | 10 | 2026-03-21 |
| New Development Project – 4 Villa Complex | `new-development-seminyak` | agent-q3v8 | 10 | 2026-03-21 |

---

## 🔒 CLAIMED (in progress — do NOT touch)

<!-- Agents write here when they start working on an asset -->
<!-- Format: | Asset Slug | Agent ID | Claimed At | -->

| Slug | Agent | Claimed At |
|------|-------|------------|

| beachfront-retreat-sanur-exited | agent-q3v8 | 2026-03-21T07:32:00+07:00 |

| modern-surf-villa-canggu | agent-k5w9 | 2026-03-21T01:09:00+07:00 |
| luxury-pool-villa-canggu-funded | agent-m3j7 | 2026-03-21T01:09:13+07:00 |
| vacation-rental-villa-uluwatu | agent-p9m4 | 2026-03-21T07:31:00+07:00 |

---

## 🔄 AVAILABLE (needs upgrade — pick from here)

### Real Estate

| # | Asset | Slug | UUID | Value (USD) | Bedrooms | Location | Status | Current Images |
|---|-------|------|------|-------------|----------|----------|--------|----------------|




| 5 | Modern Surf Villa near Echo Beach | modern-surf-villa-canggu | 6ddc6ee5-ed72-4d3c-90f9-8cf4b6704771 | 1150000 | 4 | Canggu, Bali | funded | villa2_1.webp, villa2_2.webp |
| 6 | Luxury Pool Villa – Fully Funded | luxury-pool-villa-canggu-funded | 89295282-6302-42d3-9961-a168b4578a40 | 950000 | 3 | Canggu, Bali | funded | villa3_1.webp, villa1_2.webp |
| 7 | Vacation Rental Villa with Temple Views | vacation-rental-villa-uluwatu | 0303245f-a4b0-4723-87f7-3cfcd64910d5 | 785000 | 3 | Uluwatu, Bali | funding_in_progress | villa4_1.webp, villa4_2.webp |
| 8 | Beachfront Retreat – Successfully Exited | beachfront-retreat-sanur-exited | cb932002-0cdd-46e4-bb35-209e658060c9 | 650000 | 2 | Sanur, Bali | exited | villa2_1.webp |
| 9 | Test Save Villa (Clifftop Uluwatu) | luxury-clifftop-villa-uluwatu | 0a64f742-eb99-41f7-bd0a-9dd941b11011 | 500000 | 3 | Denpasar, ID | funded | villa1.webp + 3 old |
| 10 | Renovation Flip Project – Canggu | renovation-flip-canggu | ebdc983b-5792-43a0-b764-1c0517701e2b | 450000 | 3 | Canggu, Bali | funding_open | villa5.webp |

### Commodities

| # | Asset | Slug | UUID | Value (USD) | Location | Status | Current Images |
|---|-------|------|------|-------------|----------|--------|----------------|
| 11 | Organic Cacao – Single Origin Bali | organic-cacao-bali-2026 | 0081d064-996e-46a2-b19c-92a9d7389767 | 80000 | Bali | funding_open | rice photo (wrong!) |
| 12 | Specialty Coffee – Kintamani | specialty-coffee-kintamani-2026 | 754e5be6-22f6-44fb-8cd7-f3aa90d86d25 | 30000 | Bali | funded | rice photo (wrong!) |

---

## ❌ SKIPPED (unpublished/test — ignore)

All `published = false` assets and Premium Bali Rice (already has real photos).
