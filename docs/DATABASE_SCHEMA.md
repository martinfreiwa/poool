# 🗄️ POOOL – Vollständiges Datenbank-Design (PostgreSQL)

Dieses Dokument ist die "Single Source of Truth" für das gesamte relationale Datenbankschema der POOOL-Plattform.
Jede Tabelle, jede Spalte und jede Beziehung wurde direkt aus der Analyse der bestehenden Frontend-Seiten abgeleitet.

> **WICHTIG:** Alle Geldbeträge werden in **Cents (INTEGER)** gespeichert (z. B. $2,732 = 273200). Floating-Point-Zahlen (FLOAT/DOUBLE) sind in Finanzsystemen **streng verboten**, da sie zu Rundungsfehlern führen.

---

## Übersicht der Tabellen

| Nr | Tabelle | Zweck | Abgeleitet von (Frontend-Seite) |
|----|---------|-------|--------------------------------|
| 1 | `users` | Haupttabelle aller Benutzer | signup, login, profile-dropdown |
| 2 | `user_profiles` | Persönliche Daten (KYC-relevant) | KYC-Seite, Settings |
| 3 | `user_sessions` | Login-Sessions (HttpOnly Cookies) | Login, "Remember me" |
| 4 | `oauth_accounts` | Social Login (Google, Facebook, Apple) | signup.html, login.html |
| 5 | `kyc_records` | Identitätsprüfung (SumSub o.ä.) | KYC-Banner auf jeder Seite |
| 6 | `wallets` | Fiat-Guthaben (Cash + Rewards) | wallet.html |
| 7 | `wallet_transactions` | Transaktionshistorie des Wallets | wallet.html (Transactions-Tabelle) |
| 8 | `assets` | Immobilien & Rohstoffe | marketplace.html, commodities-marketplace.html |
| 9 | `asset_images` | Bilder pro Asset (Gallery) | Marketplace-Cards |
| 10 | `asset_milestones` | Timeline und Checkpoints pro Asset | Funding Timeline / Roadmap |
| 11 | `asset_documents` | Rechtliche Dokumente (Exposé etc.) | Asset-Detailseite |
| 12 | `asset_financials` | Finanzkennzahlen pro Asset | Portfolio "Key financials" |
| 13 | `investments` | Gehaltene Anteile pro User/Asset | portfolio.html "My Assets"-Tabelle |
| 14 | `cart_items` | Warenkorb | cart.html |
| 15 | `orders` | Abgeschlossene Kaufvorgänge | Cart → Checkout |
| 16 | `order_items` | Einzelpositionen einer Bestellung | Cart-Items → Order-Items |
| 17 | `dividend_payouts` | Mieteinnahmen-Ausschüttungen | Portfolio "Payout expected" Status |
| 18 | `notifications` | Benachrichtigungen | Notification-Badge im Header |
| 19 | `support_tickets` | Support-Anfragen | support.html, Chat-Karte |
| 20 | `user_settings` | Nutzer-Einstellungen & 2FA | settings.html |
| 21 | `roles` | Rollen (investor, developer, admin) | Profile-Switcher |
| 22 | `user_roles` | Zuordnung User ↔ Rolle | "Switch account" Dropdown |
| 23 | `developer_projects` | Developer-Projekte/Assets | developer/dashboard.html |
| 24 | `audit_logs` | Unveränderliches Audit-Protokoll | Compliance-Pflicht |
| 25 | `password_reset_tokens` | Passwort-Zurücksetzen | forgot-password.html |
| 26 | `investment_limits` | Jährliches Investmentlimit | Portfolio "Annual investment limit" |

---

## 1. `users`
*Abgeleitet von: signup.html (Email-Feld), login.html (Email + Password), Profile-Dropdown (Name, Avatar)*

```sql
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(255) NOT NULL UNIQUE,
    password_hash   VARCHAR(255),              -- NULL wenn nur OAuth
    email_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    avatar_url      VARCHAR(512),              -- "/images/Image.png" aus Frontend
    status          VARCHAR(20) NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'suspended', 'deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_users_email ON users(email);
```

## 2. `user_profiles`
*Abgeleitet von: KYC-Seite (kyc.html), Profil-Dropdown ("Olivia Rhye"), Settings-Seite*

```sql
CREATE TABLE user_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    first_name      VARCHAR(100),             -- "Olivia"
    last_name       VARCHAR(100),             -- "Rhye"
    display_name    VARCHAR(200),             -- Angezeigt in Sidebar/Header
    date_of_birth   DATE,
    nationality     VARCHAR(3),               -- ISO 3166-1 alpha-3
    address_line_1  VARCHAR(255),
    address_line_2  VARCHAR(255),
    city            VARCHAR(100),
    state_province  VARCHAR(100),
    postal_code     VARCHAR(20),
    country         VARCHAR(3),               -- ISO 3166-1 alpha-3
    phone_number    VARCHAR(30),
    tax_id          VARCHAR(50),              -- Steuer-ID (verschlüsselt)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 3. `user_sessions`
*Abgeleitet von: login.html ("Remember me" Checkbox), Session-basiertes Auth*

```sql
CREATE TABLE user_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token   VARCHAR(255) NOT NULL UNIQUE,
    ip_address      INET,
    user_agent      TEXT,
    remember_me     BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_sessions_user ON user_sessions(user_id);
```

## 4. `oauth_accounts`
*Abgeleitet von: signup.html & login.html (Google, Facebook, Apple Buttons)*

```sql
CREATE TABLE oauth_accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        VARCHAR(20) NOT NULL       -- 'google', 'facebook', 'apple'
                    CHECK (provider IN ('google', 'facebook', 'apple')),
    provider_id     VARCHAR(255) NOT NULL,     -- ID beim Provider
    provider_email  VARCHAR(255),
    access_token    TEXT,                      -- verschlüsselt
    refresh_token   TEXT,                      -- verschlüsselt
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, provider_id)
);
CREATE INDEX idx_oauth_user ON oauth_accounts(user_id);
```

## 5. `kyc_records`
*Abgeleitet von: KYC-Banner ("Complete identity verification (KYC) to buy..."), KYC-Seite*

```sql
CREATE TABLE kyc_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider            VARCHAR(50) NOT NULL DEFAULT 'sumsub',
    provider_ref_id     VARCHAR(255),          -- Referenz beim KYC-Provider
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'in_review', 'approved', 'rejected', 'expired')),
    rejection_reason    TEXT,
    document_type       VARCHAR(50),           -- 'passport','id_card','drivers_license'
    pep_check_passed    BOOLEAN,               -- Politically Exposed Person
    sanctions_check     BOOLEAN,               -- Sanktionslisten
    verified_at         TIMESTAMPTZ,
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_kyc_user ON kyc_records(user_id);
CREATE INDEX idx_kyc_status ON kyc_records(status);
```

## 6. `wallets`
*Abgeleitet von: wallet.html ("Cash balance: USD 2,732" + "Rewards balance: USD 1,700")*

```sql
CREATE TABLE wallets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_type     VARCHAR(20) NOT NULL       -- 'cash', 'rewards'
                    CHECK (wallet_type IN ('cash', 'rewards')),
    balance_cents   BIGINT NOT NULL DEFAULT 0  -- $2,732 = 273200 (immer USD)
                    CHECK (balance_cents >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, wallet_type)
);
CREATE INDEX idx_wallets_user ON wallets(user_id);
```

## 7. `wallet_transactions`
*Abgeleitet von: wallet.html Transactions-Tabelle (Spalten: Type, Status, Date, Wallet, Amount, Actions)*

```sql
CREATE TABLE wallet_transactions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id           UUID NOT NULL REFERENCES wallets(id),
    type                VARCHAR(30) NOT NULL
                        CHECK (type IN (
                            'deposit', 'withdrawal', 'purchase',
                            'sale', 'dividend', 'reward', 'refund', 'fee'
                        )),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled')),
    amount_cents        BIGINT NOT NULL,        -- positiv = Eingang, negativ = Ausgang (immer USD)
    description         TEXT,
    external_ref_id     VARCHAR(255),           -- Referenz zum PSP (Mangopay etc.)
    related_order_id    UUID,                   -- FK zu orders (optional)
    metadata            JSONB,                  -- Flexible Zusatzdaten
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ
);
CREATE INDEX idx_wtx_wallet ON wallet_transactions(wallet_id);
CREATE INDEX idx_wtx_type ON wallet_transactions(type);
CREATE INDEX idx_wtx_status ON wallet_transactions(status);
CREATE INDEX idx_wtx_created ON wallet_transactions(created_at DESC);
```

## 8. `assets`
*Abgeleitet von: marketplace.html (Property-Cards), commodities-marketplace.html, developer/assets.html*

```sql
CREATE TABLE assets (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_user_id   UUID REFERENCES users(id), -- wer hat es eingestellt
    title               VARCHAR(255) NOT NULL,     -- Asset Title / Property Name
    slug                VARCHAR(255) NOT NULL UNIQUE,
    short_description   VARCHAR(500),              -- Aus property-content (Kurzbeschreibung)
    description         TEXT,                      -- Full Description
    asset_type          VARCHAR(30) NOT NULL       -- Aus developer/add-asset.html Step 1
                        CHECK (asset_type IN (
                            'real_estate', 'commercial_property', 'commodity',
                            'business', 'startup', 'land_plot'
                        )),
    
    -- Spezifische Property-Details (Aus application-form Step 2 & 4)
    property_type       VARCHAR(50),               -- 'villa', 'house', 'apartment', 'commercial'
    area                VARCHAR(100),              -- z.B. 'canggu', 'uluwatu'
    lease_type          VARCHAR(30)                -- 'leasehold', 'freehold'
                        CHECK (lease_type IN ('leasehold', 'freehold', NULL)),
    lease_term_years    INTEGER,                   -- verbleibende Jahre
    land_size_sqm       DECIMAL(10, 2),            -- Grundstücksgröße
    building_size_sqm   DECIMAL(10, 2),            -- Gebäudegröße
    bedrooms            INTEGER,
    bathrooms           INTEGER,
    construction_status VARCHAR(50)                -- Status Quo
                        CHECK (construction_status IN ('ready', 'construction', 'renovation', NULL)),
    year_built          INTEGER,                   -- Baujahr oder Jahr der Renovierung

    -- Location & Media
    location_city       VARCHAR(100),              
    location_country    VARCHAR(3),                
    location_address    VARCHAR(255),              -- Genauer Address-String
    location_lat        DECIMAL(10, 7),
    location_lng        DECIMAL(10, 7),
    location_description TEXT,                     -- Beschreibung der Nachbarschaft
    google_maps_url     VARCHAR(512),
    video_url           VARCHAR(512),              -- YouTube/Video Tour

    -- Finanzdaten (Aus application-form Step 2 & 4)
    total_value_cents   BIGINT NOT NULL,           -- Purchase price
    token_price_cents   BIGINT NOT NULL,           -- Minimum Share Price
    tokens_total        INTEGER NOT NULL,          
    tokens_available    INTEGER NOT NULL,          
    
    -- Rendite (Properties)
    annual_yield_bps    INTEGER,                    -- Expected Rental Yield (750 = 7.50%)
    capital_appreciation_bps INTEGER,               -- Expected Capital Appreciation (350 = 3.50%)
    occupancy_rate_bps  INTEGER,                    -- Belegungsrate

    -- Commodity spezifische Details (Aus commodity.html)
    operator_name       VARCHAR(255),              -- "PT. NEO AGRO SOLUTIONS"
    term_months         INTEGER,                   -- "12 months"
    fixed_roi_bps       INTEGER,                   -- "35%" (3500)
    revenue_min_cents   BIGINT,                    -- "$5.4M"
    revenue_max_cents   BIGINT,                    -- "$9M"
    expenses_cents      BIGINT,                    -- "$450K"
    net_profit_min_cents BIGINT,
    net_profit_max_cents BIGINT,
    investor_payout_cents BIGINT,
    operator_split_pct  INTEGER,                   -- "55"
    poool_split_pct     INTEGER,                   -- "45"
    
    -- Status
    funding_status      VARCHAR(30) NOT NULL DEFAULT 'upcoming'
                        CHECK (funding_status IN (
                            'upcoming', 'funding_open', 'funding_in_progress',
                            'funded', 'rented', 'payout_pending', 'exited'
                        )),
    featured            BOOLEAN NOT NULL DEFAULT FALSE,
    published           BOOLEAN NOT NULL DEFAULT FALSE,
    -- Timestamps
    funding_start_at    TIMESTAMPTZ,
    funding_end_at      TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_assets_type ON assets(asset_type);
CREATE INDEX idx_assets_status ON assets(funding_status);
CREATE INDEX idx_assets_developer ON assets(developer_user_id);
CREATE INDEX idx_assets_slug ON assets(slug);
```

## 9. `asset_images`
*Abgeleitet von: Marketplace-Cards (villa1.jpg, villa2_1.jpg etc.), Bildergalerie*

```sql
CREATE TABLE asset_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    image_url       VARCHAR(512) NOT NULL,
    alt_text        VARCHAR(255),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_cover        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_images_asset ON asset_images(asset_id);
```

## 10. `asset_milestones`
*Abgeleitet von: Funding Timeline (property.html) und Roadmap (commodity.html)*

```sql
CREATE TABLE asset_milestones (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    title           VARCHAR(255) NOT NULL,     -- "Property funded" / "Month 1 - Land prep & planting"
    description     TEXT,                      -- "Harvesting, processing (pasta/dried), first sales"
    milestone_date  TIMESTAMPTZ,               -- Konkretes Datum (falls bekannt, z.B. bei Property Funding)
    month_index     INTEGER,                   -- Oder generischer Monat (1, 3, 6, 12 für Commodities)
    is_completed    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_milestones_asset ON asset_milestones(asset_id);
```

## 11. `asset_documents`
*Abgeleitet von: Asset-Detailseite (Exposé, Gutachten, Kaufvertrag)*

```sql
CREATE TABLE asset_documents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    document_type   VARCHAR(50) NOT NULL
                    CHECK (document_type IN (
                        'proof_of_title', 'legal_basis', 'building_permit',
                        'site_plan', 'tax_npwp', 'tax_pbb', 'tax_bphtb',
                        'license_nib', 'id_card', 'owner_npwp',
                        'expose', 'appraisal', 'financial', 'floor_plan', 'other'
                    )),
    title           VARCHAR(255) NOT NULL,
    file_url        VARCHAR(512) NOT NULL,
    file_size_bytes BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_docs_asset ON asset_documents(asset_id);
```

## 12. `asset_financials`
*Abgeleitet von: Portfolio "Key financials" (Monthly income, Total rental income, Total appreciation)*

```sql
CREATE TABLE asset_financials (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id                    UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    period_month                INTEGER NOT NULL,      -- 1-12
    period_year                 INTEGER NOT NULL,      -- 2026
    rental_income_cents         BIGINT DEFAULT 0,      -- Mieteinnahmen im Monat
    appreciation_cents          BIGINT DEFAULT 0,      -- Wertsteigerung
    occupancy_rate_bps          INTEGER,               -- 8000 = 80%
    expenses_cents              BIGINT DEFAULT 0,      -- Betriebskosten
    net_income_cents            BIGINT DEFAULT 0,      -- Netto = rental - expenses
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, period_month, period_year)
);
CREATE INDEX idx_asset_fin_asset ON asset_financials(asset_id);
```

## 13. `investments`
*Abgeleitet von: portfolio.html "My Assets" Tabelle (Property, Investment value, Total rental income, Status)*

```sql
CREATE TABLE investments (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(id),
    asset_id                UUID NOT NULL REFERENCES assets(id),
    tokens_owned            INTEGER NOT NULL DEFAULT 0
                            CHECK (tokens_owned > 0),
    purchase_value_cents    BIGINT NOT NULL,        -- Gesamtkaufwert
    current_value_cents     BIGINT NOT NULL,        -- aktuell berechneter Wert
    total_rental_cents      BIGINT NOT NULL DEFAULT 0, -- kumulierte Mieteinnahmen
    appreciation_pct_bps    INTEGER DEFAULT 0,       -- "+3.5%" = 350
    status                  VARCHAR(30) NOT NULL DEFAULT 'active'
                            CHECK (status IN (
                                'active', 'funded', 'rented', 'payout_pending',
                                'in_process', 'funding_in_progress', 'exited'
                            )),
    payout_expected_at      TIMESTAMPTZ,            -- "Payout expected: Sept"
    purchased_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, asset_id)
);
CREATE INDEX idx_investments_user ON investments(user_id);
CREATE INDEX idx_investments_asset ON investments(asset_id);
```

## 14. `cart_items`
*Abgeleitet von: cart.html (Warenkorb mit Properties, Token-Menge, Preis)*

```sql
CREATE TABLE cart_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id        UUID NOT NULL REFERENCES assets(id),
    tokens_quantity INTEGER NOT NULL DEFAULT 1
                    CHECK (tokens_quantity > 0),
    token_price_cents BIGINT NOT NULL,          -- Preis zum Zeitpunkt des Hinzufügens
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, asset_id)
);
CREATE INDEX idx_cart_user ON cart_items(user_id);
```

## 15. `orders`
*Abgeleitet von: Cart → Checkout Ablauf*

```sql
CREATE TABLE orders (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id),
    order_number        VARCHAR(30) NOT NULL UNIQUE,
    total_cents         BIGINT NOT NULL,        -- immer USD
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled', 'refunded')),
    payment_method      VARCHAR(30),            -- 'wallet', 'bank_transfer', 'card'
    payment_ref_id      VARCHAR(255),           -- PSP Referenz
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ
);
CREATE INDEX idx_orders_user ON orders(user_id);
CREATE INDEX idx_orders_status ON orders(status);
```

## 16. `order_items`
```sql
CREATE TABLE order_items (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id            UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    asset_id            UUID NOT NULL REFERENCES assets(id),
    tokens_quantity     INTEGER NOT NULL,
    token_price_cents   BIGINT NOT NULL,
    subtotal_cents      BIGINT NOT NULL
);
CREATE INDEX idx_order_items_order ON order_items(order_id);
```

## 17. `dividend_payouts`
*Abgeleitet von: Portfolio Status "Payout expected: Sept/Oct"*

```sql
CREATE TABLE dividend_payouts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    investment_id       UUID NOT NULL REFERENCES investments(id),
    user_id             UUID NOT NULL REFERENCES users(id),
    asset_id            UUID NOT NULL REFERENCES assets(id),
    amount_cents        BIGINT NOT NULL,
    payout_type         VARCHAR(20) NOT NULL     -- 'rental', 'exit', 'bonus'
                        CHECK (payout_type IN ('rental', 'exit', 'bonus')),
    status              VARCHAR(20) NOT NULL DEFAULT 'scheduled'
                        CHECK (status IN ('scheduled', 'processing', 'paid', 'failed')),
    scheduled_at        TIMESTAMPTZ,
    paid_at             TIMESTAMPTZ,
    wallet_tx_id        UUID REFERENCES wallet_transactions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_dividends_user ON dividend_payouts(user_id);
CREATE INDEX idx_dividends_investment ON dividend_payouts(investment_id);
```

## 18. `notifications`
*Abgeleitet von: Notification-Bell im Header (Badge "3")*

```sql
CREATE TABLE notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(255) NOT NULL,
    message         TEXT,
    type            VARCHAR(30) NOT NULL        -- 'kyc', 'investment', 'payout', 'system', 'promo'
                    CHECK (type IN ('kyc', 'investment', 'payout', 'system', 'promo')),
    is_read         BOOLEAN NOT NULL DEFAULT FALSE,
    action_url      VARCHAR(512),              -- Deep-Link
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read) WHERE is_read = FALSE;
```

## 19. `support_tickets`
*Abgeleitet von: support.html, Mobile "Chat to support" Karte*

```sql
CREATE TABLE support_tickets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    subject         VARCHAR(255) NOT NULL,
    message         TEXT NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'in_progress', 'resolved', 'closed')),
    priority        VARCHAR(10) DEFAULT 'normal'
                    CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    assigned_to     UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_support_user ON support_tickets(user_id);
CREATE INDEX idx_support_status ON support_tickets(status);
```

## 20. `user_settings`
*Abgeleitet von: settings.html, 2FA-Anforderung*

```sql
CREATE TABLE user_settings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    totp_secret     VARCHAR(255),              -- 2FA Secret (verschlüsselt)
    totp_enabled    BOOLEAN NOT NULL DEFAULT FALSE,
    language        VARCHAR(5) DEFAULT 'en',
    email_notifications   BOOLEAN DEFAULT TRUE,
    push_notifications    BOOLEAN DEFAULT TRUE,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 21. `roles`
*Abgeleitet von: Profile-Switcher ("Investor Profile" / "Developer profile")*

```sql
CREATE TABLE roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(30) NOT NULL UNIQUE, -- 'investor', 'developer', 'admin'
    description     VARCHAR(255)
);
INSERT INTO roles (name, description) VALUES
    ('investor', 'Standard-Investor mit Zugang zum Marketplace und Portfolio'),
    ('developer', 'Immobilien-Entwickler, der Assets einstellen kann'),
    ('admin', 'Plattform-Administrator');
```

## 22. `user_roles`
*Abgeleitet von: "Switch account" Dropdown – ein User kann Investor UND Developer sein*

```sql
CREATE TABLE user_roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id         UUID NOT NULL REFERENCES roles(id),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, role_id)
);
CREATE INDEX idx_user_roles_user ON user_roles(user_id);
```

## 23. `developer_projects`
*Abgeleitet von: developer/dashboard.html, developer/assets.html, developer/add-asset.html*

```sql
CREATE TABLE developer_projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_id    UUID NOT NULL REFERENCES users(id),
    asset_id        UUID REFERENCES assets(id),  -- verknüpftes Asset
    project_name    VARCHAR(255) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'submitted', 'in_review', 'approved', 'rejected', 'live')),
    -- Statistiken für das Developer Dashboard
    total_raised_cents      BIGINT DEFAULT 0,
    investors_count         INTEGER DEFAULT 0,
    funding_progress_bps    INTEGER DEFAULT 0,   -- 5000 = 50%
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_dev_projects_developer ON developer_projects(developer_id);
```

## 24. `audit_logs` (IMMUTABLE – niemals UPDATE oder DELETE!)
*Regulatorische Pflicht: Jede kritische Aktion wird lückenlos protokolliert*

```sql
CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,      -- Auto-increment, niemals UUID
    actor_user_id   UUID REFERENCES users(id),
    action          VARCHAR(100) NOT NULL,       -- 'user.created', 'investment.purchased', 'kyc.approved'
    entity_type     VARCHAR(50) NOT NULL,        -- 'user', 'investment', 'wallet_transaction'
    entity_id       UUID,                        -- ID des betroffenen Datensatzes
    previous_state  JSONB,                       -- Zustand VOR der Änderung
    new_state       JSONB,                       -- Zustand NACH der Änderung
    ip_address      INET,
    user_agent      TEXT,
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- KEIN UPDATE/DELETE erlaubt!
CREATE INDEX idx_audit_user ON audit_logs(actor_user_id);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
```

## 25. `password_reset_tokens`
*Abgeleitet von: forgot-password.html*

```sql
CREATE TABLE password_reset_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      VARCHAR(255) NOT NULL UNIQUE,   -- gehashter Token
    expires_at      TIMESTAMPTZ NOT NULL,
    used_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_prt_token ON password_reset_tokens(token_hash);
```

## 26. `investment_limits`
*Abgeleitet von: Portfolio "Annual investment limit" Sektion (USD 250,000 limit, 35% used)*

```sql
CREATE TABLE investment_limits (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    annual_limit_cents      BIGINT NOT NULL DEFAULT 25000000,  -- USD 250,000
    invested_12m_cents      BIGINT NOT NULL DEFAULT 0,         -- USD 83,000
    available_cents         BIGINT GENERATED ALWAYS AS (annual_limit_cents - invested_12m_cents) STORED,
    limit_year              INTEGER NOT NULL,                  -- z.B. 2026
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, limit_year)
);
CREATE INDEX idx_inv_limits_user ON investment_limits(user_id);
```


---

## Marketplace Tables (Phase 2 — Migrations 050b-055)

### `wallets` — Extended Columns

```sql
-- New column added by migration 050b
held_balance_cents BIGINT NOT NULL DEFAULT 0  -- Funds blocked by open buy orders
-- Constraints:
--   chk_held_balance_non_negative: held_balance_cents >= 0
--   chk_held_lte_balance: held_balance_cents <= balance_cents
```

### `investments` — Extended Columns

```sql
-- New column added by migration 050c
held_tokens INTEGER NOT NULL DEFAULT 0  -- Tokens blocked by open sell orders
-- Constraints:
--   chk_held_tokens_non_negative: held_tokens >= 0
--   chk_held_tokens_lte_owned: held_tokens <= tokens_owned
```

### 27. `market_orders`
*All limit/market orders in the marketplace (open, filled, cancelled)*

```sql
CREATE TABLE market_orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    side            VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type      VARCHAR(10) NOT NULL DEFAULT 'limit'
                    CHECK (order_type IN ('limit', 'market')),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    quantity_filled INTEGER NOT NULL DEFAULT 0 CHECK (quantity_filled >= 0),
    status          VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN (
                        'open', 'partially_filled', 'filled', 'cancelled',
                        'admin_cancelled', 'expired', 'pending_review', 'rejected'
                    )),
    idempotency_key UUID UNIQUE,
    cancel_reason   TEXT,
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_filled_lte_qty CHECK (quantity_filled <= quantity)
);
-- Partial indexes: idx_orders_asset_status, idx_market_orders_user, idx_orders_expiry, idx_orders_pending
```

### 28. `trade_history`
*Immutable log of all executed trades (NEVER updated or deleted)*

```sql
CREATE TABLE trade_history (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    buy_order_id    UUID NOT NULL REFERENCES market_orders(id),
    sell_order_id   UUID NOT NULL REFERENCES market_orders(id),
    buyer_user_id   UUID NOT NULL REFERENCES users(id),
    seller_user_id  UUID NOT NULL REFERENCES users(id),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    total_cents     BIGINT GENERATED ALWAYS AS (price_cents * quantity) STORED,
    fee_cents       BIGINT NOT NULL DEFAULT 0 CHECK (fee_cents >= 0),
    fee_bps         INTEGER NOT NULL DEFAULT 0,
    on_chain_status VARCHAR(15) NOT NULL DEFAULT 'pending'
                    CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed')),
    on_chain_tx_hash VARCHAR(66),
    on_chain_batch_id UUID,
    executed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_no_self_trade CHECK (buyer_user_id != seller_user_id)
);
-- Indexes: idx_trade_asset_time, idx_trade_buyer, idx_trade_seller, idx_trade_onchain
```

### 29. `p2p_offers`
*Peer-to-peer (OTC) direct offers between users*

```sql
CREATE TABLE p2p_offers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id          UUID NOT NULL REFERENCES assets(id),
    maker_user_id     UUID NOT NULL REFERENCES users(id),
    taker_user_id     UUID NOT NULL REFERENCES users(id),
    side              VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    price_cents       BIGINT NOT NULL CHECK (price_cents > 0),
    quantity          INTEGER NOT NULL CHECK (quantity > 0),
    message           TEXT,
    status            VARCHAR(15) NOT NULL DEFAULT 'pending'
                      CHECK (status IN (
                          'pending', 'accepted', 'declined', 'expired',
                          'countered', 'cancelled', 'admin_cancelled'
                      )),
    parent_offer_id   UUID REFERENCES p2p_offers(id),
    trade_id          UUID REFERENCES trade_history(id),
    expires_at        TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '48 hours'),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_no_self_offer CHECK (maker_user_id != taker_user_id)
);
-- Indexes: idx_p2p_taker (partial), idx_p2p_asset, idx_p2p_expiry (partial)
```

### 30. `fee_configurations`
*4-tier fee hierarchy: Promotion > Developer Deal > Asset > Platform Default*

```sql
CREATE TABLE fee_configurations (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('platform', 'asset', 'developer')),
    asset_id      UUID REFERENCES assets(id),
    developer_id  UUID REFERENCES users(id),
    taker_fee_bps INTEGER NOT NULL DEFAULT 500 CHECK (taker_fee_bps >= 0 AND taker_fee_bps <= 1000),
    maker_fee_bps INTEGER NOT NULL DEFAULT 0 CHECK (maker_fee_bps >= 0 AND maker_fee_bps <= 1000),
    is_active     BOOLEAN NOT NULL DEFAULT true,
    reason        TEXT,
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_fee_scope UNIQUE (scope, asset_id, developer_id, is_active)
);
```

### 31. `fee_promotions`
*Time-bounded fee promotions (highest priority in fee lookup)*

```sql
CREATE TABLE fee_promotions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          VARCHAR(100) NOT NULL,
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('global', 'asset')),
    asset_id      UUID REFERENCES assets(id),
    taker_fee_bps INTEGER NOT NULL CHECK (taker_fee_bps >= 0),
    maker_fee_bps INTEGER NOT NULL CHECK (maker_fee_bps >= 0),
    starts_at     TIMESTAMPTZ NOT NULL,
    ends_at       TIMESTAMPTZ NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_promo_dates CHECK (ends_at > starts_at)
);
-- Index: idx_promo_active (partial, WHERE is_active = true)
```

### 32. `marketplace_alerts`
*Auto/manual alerts for suspicious marketplace activity*

```sql
CREATE TABLE marketplace_alerts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type   VARCHAR(50) NOT NULL,
    severity     VARCHAR(15) NOT NULL DEFAULT 'warning'
                 CHECK (severity IN ('info', 'warning', 'critical')),
    asset_id     UUID REFERENCES assets(id),
    user_id      UUID REFERENCES users(id),
    trade_id     UUID REFERENCES trade_history(id),
    message      TEXT NOT NULL,
    metadata     JSONB,
    status       VARCHAR(15) NOT NULL DEFAULT 'new'
                 CHECK (status IN ('new', 'acknowledged', 'resolved', 'false_positive')),
    resolved_by  UUID REFERENCES users(id),
    resolved_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Indexes: idx_alerts_status (partial), idx_alerts_severity (partial), idx_alerts_user
```

### 33. `marketplace_watchlist`
*Admin watchlist for suspicious users*

```sql
CREATE TABLE marketplace_watchlist (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id),
    reason     TEXT NOT NULL,
    added_by   UUID NOT NULL REFERENCES users(id),
    is_active  BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Unique partial index: idx_watchlist_user (one active entry per user)
```

### 34. `reconciliation_reports`
*Daily balance reconciliation check results*

```sql
CREATE TABLE reconciliation_reports (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date             DATE NOT NULL UNIQUE,
    total_wallet_cents      BIGINT NOT NULL,
    total_deposits_cents    BIGINT NOT NULL,
    total_withdrawals_cents BIGINT NOT NULL,
    total_purchases_cents   BIGINT NOT NULL,
    cash_delta_cents        BIGINT NOT NULL,
    total_fees_earned_cents BIGINT NOT NULL,
    fee_wallet_cents        BIGINT NOT NULL,
    fee_delta_cents         BIGINT NOT NULL,
    token_mismatches        INTEGER NOT NULL DEFAULT 0,
    token_details           JSONB,
    status                  VARCHAR(15) NOT NULL DEFAULT 'pass'
                            CHECK (status IN ('pass', 'warning', 'fail')),
    notes                   TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Entity-Relationship Diagram (Vereinfacht)

```
users ─────┬── user_profiles
            ├── user_sessions
            ├── oauth_accounts
            ├── kyc_records
            ├── wallets ──── wallet_transactions
            │   └── + held_balance_cents (marketplace)
            ├── user_roles ── roles
            ├── user_settings
            ├── cart_items ─────┐
            ├── orders ─────────┤── assets ──┬── asset_images
            │   └── order_items ┘            ├── asset_documents
            ├── investments ─────────────────┤── asset_milestones
            │   ├── + held_tokens (mktplace) ├── asset_financials
            │   └── dividend_payouts         │
            ├── market_orders ───────────────┘
            │   └── trade_history
            │       ├── p2p_offers
            │       └── marketplace_alerts
            ├── fee_configurations
            ├── fee_promotions
            ├── marketplace_watchlist
            ├── developer_projects ──────────┘
            ├── notifications
            ├── support_tickets
            ├── investment_limits
            ├── password_reset_tokens
            └── audit_logs
reconciliation_reports (standalone, no FKs to users)
```

---

## Hinweise

> [!NOTE]
> Dieses Schema bildet exakt die **bestehenden** Frontend-Seiten ab. Folgende Features existieren aktuell nur als Platzhalter im Frontend und haben bewusst **keine** Datenbank-Tabellen:
> - **Rewards** – in der Sidebar als "Soon" markiert
> - **Leaderboard** – in der Sidebar als "Soon" markiert
> - **Community** – keine bestehende Seite
> - **Multi-Währung** – aktuell nur USD
>
> **Marketplace-Tabellen** (27-34) wurden in Phase 2 hinzugefügt. TimescaleDB Hypertables (candles) stehen aus (Phase 2.9-2.10).

---
*Ende des Datenbankschema-Dokuments.*
