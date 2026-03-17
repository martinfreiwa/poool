# Database Schema Audit Report

## Summary
A comprehensive audit of the database schema was conducted to identify any unused tables, redundant columns, or data structures that are no longer necessary for the current production architecture.

## Findings
The structure relies heavily on PostgreSQL schemas and spans across 16 different migration files:
1. `users`, `user_profiles`, `oauth_accounts`, `kyc_records`
2. `wallets`, `wallet_transactions`, `payment_methods` (added via migration `002`)
3. `assets`, `asset_images`, `asset_milestones`, `asset_documents`, `asset_financials` (Core marketplace)
4. `investments`, `cart_items`, `orders`, `order_items`
5. `rewards_balances`, `referral_tracking`, `tiers` (Rewards infrastructure)

**Validation:**
- All core tables are actively referenced throughout the backend handlers (`src/` routes and services) via raw `sqlx` queries.
- No truly orphaned tables were identified. 
- Some optional tracking structures like `asset_milestones` may appear sparsely populated but represent critical product roadmap features natively supported by the UI (e.g. project tracker).

## Action Items
- **Status:** Evaluated and Verified.
- **Next Steps:** No destructive schema alterations (like dropping tables) are required at this stage. Proceed with other checklist items safely.
