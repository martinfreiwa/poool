---
description: Deploy POOOL platform to production — phased deployment plan
---

# POOOL Deployment Workflow

## Pre-Deployment Checklist

### 1. Fresh POOOL Database
> ⚠️ The current Cloud SQL instance has tables from another app (proxies, blog_articles, etc.).
> POOOL needs its **own database**.

- [ ] Option A: Create a new database on the existing Cloud SQL instance: `CREATE DATABASE poool;`
- [ ] Option B: Create a new Cloud SQL instance entirely.
- [ ] Run all migrations in order against the fresh DB:
  ```
  database/001_initial_schema.sql
  database/002_seed_data.sql
  database/002_payment_methods.sql
  database/003_settings_extensions.sql
  database/004_rewards_schema.sql
  database/005_payments_checkout.sql
  ```
- [ ] Verify key tables exist: `users`, `user_profiles`, `roles`, `user_roles`, `kyc_records`, `wallets`, `assets`, `orders`, `investments`, `support_tickets`, etc.
- [ ] Verify seed data loaded: admin user, roles, tiers, sample assets.

### 2. Environment Variables
- [ ] Copy `backend/.env.example` → `backend/.env`
- [ ] `DATABASE_URL` → point to fresh POOOL database
- [ ] `SERVER_HOST=0.0.0.0`, `SERVER_PORT=8080`
- [ ] `SESSION_SECRET` → generate secure random: `openssl rand -hex 32`
- [ ] `POSTMARK_API_KEY` → for transactional emails (optional for MVP)
- [ ] `GOOGLE_CLIENT_ID` + `GOOGLE_CLIENT_SECRET` → OAuth (optional for MVP)
- [ ] `FACEBOOK_APP_ID` + `FACEBOOK_APP_SECRET` → OAuth (optional for MVP)
- [ ] `GCS_BUCKET_NAME` → GCS bucket for file uploads (e.g. `poool-assets-primary`) — **required for uploads to work**

### 3. Build
// turbo
```bash
cd /Users/martin/Projects/poool/backend && cargo build --release
```

---

## Phase 1 — Core Platform (MVP)

**Goal:** User can sign up → browse marketplace → view properties.

| Route | Page | Status |
|-------|------|--------|
| `/auth/login` | Login | ✅ Ready |
| `/auth/signup` | Signup | ✅ Ready |
| `/auth/verify-email` | Email verification | ✅ Ready |
| `/auth/forgot-password` | Password reset | ✅ Ready |
| `/marketplace` | Asset marketplace | ✅ Ready |
| `/property` | Property detail | ✅ Ready |
| `/commodities-marketplace` | Commodities | ✅ Ready |
| `/portfolio` | Investments | ✅ Ready |
| `/settings` | Profile/preferences | ✅ Ready |
| `/rewards` | Rewards overview | ✅ Ready |
| `/tier` | Tier detail | ✅ Ready (static) |
| `/api/me` | User identity API | ✅ Ready |

**Test:** Sign up → verify email → log in → browse marketplace → view property.

---

## Phase 2 — Investment Flow (Revenue)

**Goal:** User can deposit → add to cart → checkout → own tokens.

| Route | Page | Status |
|-------|------|--------|
| `/wallet` | Wallet (deposit/withdraw) | ✅ Ready |
| `/cart` | Shopping cart | ✅ Ready |
| `/checkout` | Payment form | ⚠️ Skeleton (78 lines) |
| `/payment-success` | Order confirmation | ⚠️ Needs dynamic data |
| `/payment-in-progress` | Pending payment | ⚠️ Needs dynamic data |
| `/kyc` | KYC submission | ⚠️ Page exists, API missing |
| `POST /api/kyc/submit` | Submit KYC | 🔴 **MISSING** |
| `GET /api/kyc/status` | Check KYC status | 🔴 **MISSING** |

**Blockers to fix before deploy:**
1. Add `POST /api/kyc/submit` and `GET /api/kyc/status` endpoints.
2. Rebuild `/checkout` with order summary from cart API.
3. Wire `/payment-success` to display order details.

---

## Phase 3 — Admin Panel (Operations)

**Goal:** Admin can approve KYC, confirm deposits, manage assets.

| Route | Page | Status |
|-------|------|--------|
| `/admin/` | Dashboard KPIs | ✅ Ready |
| `/admin/users.html` | User management | ✅ Ready |
| `/admin/user-details.html` | User deep-dive | ✅ Ready |
| `/admin/kyc.html` | KYC queue | ✅ Ready |
| `/admin/deposits.html` | Deposit confirmation | ✅ Ready |
| `/admin/orders.html` | Order monitoring | ✅ Ready |
| `/admin/assets.html` | Asset management | ✅ Ready |
| `/admin/developer-submissions.html` | Submission review | ✅ Ready |
| `/admin/treasury.html` | Treasury overview | ✅ Ready |

**No blockers** — all admin APIs exist.

---

## Phase 4 — Engagement

**Goal:** Support, legal pages, notifications.

| Feature | Status |
|---------|--------|
| `/support` page | ⚠️ Needs ticket API |
| Legal pages (terms, privacy, cookies, currency) | ✅ Ready (static) |
| Cookie consent banner | ❌ Not built |
| Terms acceptance on signup | ❌ Not built |
| Support ticket API (`POST /api/support/tickets`) | 🔴 **MISSING** |

---

## Phase 5 — Admin Advanced

| Page | Status |
|------|--------|
| `admin/rewards.html` | Needs content |
| `admin/support.html` | Needs content |
| `admin/audit-logs.html` | Needs content |
| `admin/reports.html` | Needs content |
| `admin/notifications.html` | Needs content |
| `admin/settings.html` | Needs content |
| `admin/system.html` | Not created |

---

## Cloud Run Deployment Commands

```bash
# Set the correct working project
gcloud config set project my-project-35266-489713

# Deploy backend (serves both API + frontend static files)
cd /Users/martin/Projects/poool

# Option 1: Deploy from source (Recommended)
gcloud run deploy poool-backend \
  --source . \
  --region europe-west1 \
  --allow-unauthenticated \
  --set-env-vars "DATABASE_URL=postgres://..." \
  --set-env-vars "SERVER_HOST=0.0.0.0,SERVER_PORT=8080" \
  --set-env-vars "GCS_BUCKET_NAME=poool-assets-primary"

# Option 2: Build Docker image first to Artifact Registry
docker build -t poool-backend -f Dockerfile .
docker tag poool-backend europe-west1-docker.pkg.dev/my-project-35266-489713/cloud-run-source-deploy/poool-backend:latest
docker push europe-west1-docker.pkg.dev/my-project-35266-489713/cloud-run-source-deploy/poool-backend:latest
gcloud run deploy poool-backend \
  --image europe-west1-docker.pkg.dev/my-project-35266-489713/cloud-run-source-deploy/poool-backend:latest \
  --region europe-west1 \
  --allow-unauthenticated
```
