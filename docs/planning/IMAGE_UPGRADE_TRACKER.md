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
| Test Save Villa (Clifftop Uluwatu) | `luxury-clifftop-villa-uluwatu` | agent-r8f3 | 6 | 2026-03-21 |
| Beachfront Retreat – Successfully Exited | `beachfront-retreat-sanur-exited` | agent-q3v8 | 6 | 2026-03-21 |
| Vacation Rental Villa with Temple Views | `vacation-rental-villa-uluwatu` | agent-p9m4 | 8 | 2026-03-21 |
| Renovation Flip Project – Canggu | `renovation-flip-canggu` | agent-q3v8 | 6 | 2026-03-21 |
| Organic Cacao – Single Origin Bali | `organic-cacao-bali-2026` | agent-q3v8 | 5 | 2026-03-21 |
| Specialty Coffee – Kintamani | `specialty-coffee-kintamani-2026` | agent-r8f3 | 5 | 2026-03-21 |
| Modern Surf Villa near Echo Beach | `modern-surf-villa-canggu` | agent-r8f3 | 10 | 2026-03-21 |
| The Canopy Estate – Hillside Retreat | `canopy-estate-hillside-retreat` | agent-q3v8 | 12 | 2026-03-21 |
| Luxury Pool Villa – Fully Funded | `luxury-pool-villa-canggu-funded` | agent-r8f3 | 8 | 2026-03-21 |
| Central Plaza Commerce | `central-plaza-commerce` | agent-r8f3 | 11 | 2026-03-21 |
| Coastal Modern Villa – Bukit Peninsula | `coastal-modern-villa-bukit` | agent-q3v8 | 8 | 2026-03-21 |
| Green Field Agriculture | `green-field-agriculture` | agent-q3v8 | 6 | 2026-03-21 |

---

## 🔒 CLAIMED (in progress — do NOT touch)

<!-- Agents write here when they start working on an asset -->
<!-- Format: | Asset Slug | Agent ID | Claimed At | -->

| Slug | Agent | Claimed At |
|------|-------|------------|

---

## 🔄 AVAILABLE (needs upgrade — pick from here)

### Real Estate

| # | Asset | Slug | UUID | Value (USD) | Bedrooms | Location | Status | Current Images |
|---|-------|------|------|-------------|----------|----------|--------|----------------|









### Commodities

| # | Asset | Slug | UUID | Value (USD) | Location | Status | Current Images |
|---|-------|------|------|-------------|----------|--------|----------------|



---

## ❌ SKIPPED (unpublished/test — ignore)

All `published = false` assets and Premium Bali Rice (already has real photos).
