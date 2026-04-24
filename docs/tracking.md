# POOOL Dashboard Refactoring Tracking

This file tracks the progress of refactoring the static HTML prototypes into dynamic, production-ready applications.

## Refactoring Phases
For each page, we go through 4 phases:
- **Phase 1: Datenmodell & Typisierung** (TypeScript Interfaces/Types)
- **Phase 2: Backend-Logik & Data Fetching** (Data fetching logic, API integration)
- **Phase 3: Frontend UI & State-Binding** (Data-binding, conditional rendering, UX consistency)
- **Phase 4: QA, Edge Cases & UX** (Empty states, loading states, responsiveness)

---

## Pages to Refactor

### 1. Rewards Page (`frontend/platform/rewards.html`)
- [x] Phase 1: Datenmodell & Typisierung — enums + interfaces in `rewards-service.js`
- [x] Phase 2: Backend-Logik & Data Fetching — `rewards-service.js` consuming `/api/rewards`
- [x] Phase 3: Frontend UI & State-Binding — `rewards.js` renders data into DOM
- [x] Phase 4: QA, Edge Cases & UX — loading / error / empty state layers; skeleton CSS; sidebar included
- **Notes:** Layout fixed to match wallet/portfolio pattern (`rewards-page` flex wrapper + sidebar include)

### 2. Wallet Page (`frontend/platform/wallet.html`)
- [x] Phase 1: Datenmodell & Typisierung — strict Rust models in `backend/src/wallet/models.rs` (enums: `TransactionType`, `TransactionStatus`, `WalletType`; structs: `WalletTransaction`, `WalletPageContext`, `WalletBalanceResponse`, `WalletTransactionsResponse`)
- [x] Phase 2: Backend-Logik & Data Fetching — `wallet-service.js` consuming `/api/wallet/balance` & `/api/wallet/transactions`; full Rust service layer in `backend/src/wallet/routes.rs`
- [x] Phase 3: Frontend UI & State-Binding — `wallet.js` orchestrates state layers; live balance card refresh with pulse animation; dynamic transaction row builder
- [x] Phase 4: QA, Edge Cases & UX — 4 state layers (loading skeleton / error / empty / content); `wallet.css` extended with shimmer animation; graceful SSR fallback when API unavailable; sidebar confirmed included
- **Notes:** SSR-first approach — Rust renders full page with real DB data; JS enhances with live refresh

### 3. Portfolio Page (`frontend/platform/portfolio.html`)
- [x] Phase 1: Datenmodell & Typisierung — JSDoc interfaces for `InvestmentItem`, `AnnualLimit`, `PortfolioResponse`; enums for status via `mapInvestmentStatus()`
- [x] Phase 2: Backend-Logik & Data Fetching — `portfolio-service.js` consuming `/api/portfolio`; business logic: appreciation calc, limit progress, pie chart normalization, status mapping; auth-aware (redirects to login on 401)
- [x] Phase 3: Frontend UI & State-Binding — `portfolio-data.js` rewritten as clean UI controller; all desktop + mobile targets updated from a single data pass
- [x] Phase 4: QA, Edge Cases & UX — skeleton loading layout, error panel, empty panel; portfolio sections hidden by default and revealed by state machine; `portfolio.css` extended with shimmer animation and state panels
- **Notes:** Fully client-side rendered (vs wallet's SSR-first); `portfolio-service.js` → `portfolio-data.js` pipeline separates concerns cleanly

### 4. Settings Page (`frontend/platform/settings.html`)
- [x] Phase 1: Datenmodell & Typisierung — JSDoc types + existing `settings.d.ts` extended; covers `SettingsResponse`, `ApiResponse`, all form types (`UpdateProfileForm`, `ChangeEmailForm`, `ChangePasswordForm`, `ChangePhoneForm`)
- [x] Phase 2: Backend-Logik & Data Fetching — `settings.js` owns settings API calls and client-side pre-flight validation in `changeEmail()` / `changePassword()` to avoid redundant round-trips
- [x] Phase 3: Frontend UI & State-Binding — `settings.js` rewritten as clean UI controller with integrated service helpers; `setButtonState()` helper for consistent loading UX; all modal close/open logic consolidated
- [x] Phase 4: QA, Edge Cases & UX — Skeleton replaced (Tailwind → native shimmer CSS); default active tab fixed (`tab-more` → `tab-mydetails` in HTML + `switchTab('mydetails')` called after load); settings page uses one JS file
- **Notes:** Uses existing `marketplace-page` layout wrapper (sidebar included); error state reuses existing `settings-empty-state` panel

*(Weitere Seiten werden hier hinzugefügt)*
