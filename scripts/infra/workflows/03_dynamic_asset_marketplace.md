---
description: Build the Dynamic Asset Marketplace (Phase 3)
---

# Workflow: Dynamic Asset Marketplace

This workflow connects the frontend marketplace (`/platform/marketplace` and `/platform/property`) to the PostgreSQL database, replacing the hardcoded HTML properties with dynamic data. It implements **Workflow 3** from the masterplan.

Follow these steps sequentially to build the dynamic marketplace.

## Step 1: Verify / Update the Database Schema
First, ensure that the `assets` and `asset_documents` tables exist in PostgreSQL based on the masterplan.
1. Inspect the `database/001_initial_schema.sql` file.
2. If the `assets` table is missing, write a SQL migration to create it:
   - `id` (UUID, primary key)
   - `title` (VARCHAR)
   - `description` (TEXT)
   - `asset_type` (VARCHAR - e.g., 'REAL_ESTATE', 'COMMODITY')
   - `total_value_cents` (BIGINT)
   - `tokens_total` (INTEGER)
   - `tokens_available` (INTEGER)
   - `token_price_cents` (BIGINT)
   - `funding_status` (VARCHAR - e.g., 'FUNDING', 'FUNDED')
   - `image_url` (VARCHAR)
   - `location` (VARCHAR)
   - `annualized_return` (VARCHAR)
3. If necessary, execute the specific SQL queries using the DB tool or `psql`.
4. Seed the database with at least 3 dummy properties so you have data to display.

## Step 2: Create the Rust `assets` Module
Create the backend logic to interact with the assets table.
1. Create `backend/src/assets/mod.rs`.
2. Create `backend/src/assets/models.rs` and define the `Asset` struct. Derive `Serialize`, `Deserialize`, `FromRow`.
3. Create `backend/src/assets/routes.rs`.

## Step 3: Implement the Marketplace Route
In `backend/src/assets/routes.rs`:
1. Write a function `pub async fn page_marketplace(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse`.
2. Inside this function, verify the user session.
3. Fetch all assets from the database: `sqlx::query_as!(Asset, "SELECT * FROM assets").fetch_all(&state.db)`.
4. Render the `marketplace.html` template using MiniJinja, passing the assets in the context: `state.templates.render("marketplace.html", context! { assets: assets })`.
   *(Hint: You may need to adjust `templates::create_engine()` to use `env.get_template` correctly instead of reading the file raw via `ServeDir` in `main.rs`).*

## Step 4: Update `main.rs` Routing
Modify `backend/src/main.rs`:
1. Add `mod assets;`.
2. Remove the old `get(page_marketplace)` and replace it with `get(assets::routes::page_marketplace)`.
3. Ensure the MiniJinja templates engine is properly configured to parse templates from `../frontend/platform`.

## Step 5: Make `marketplace.html` Dynamic (MiniJinja)
Modify the frontend HTML to use exactly what the backend passes down.
1. Open `frontend/platform/marketplace.html`.
2. Find the HTML container holding the hardcoded property cards.
3. Replace the hardcoded cards with a MiniJinja loop:
   ```html
   {% for asset in assets %}
   <!-- Property Card Template Here -->
   <h3 class="property-title">{{ asset.title }}</h3>
   <p class="property-price">{{ asset.token_price_cents / 100 }} EUR</p>
   <!-- Link to dynamic property page -->
   <a href="/property/{{ asset.id }}" class="btn">View Property</a>
   {% endfor %}
   ```
4. Verify the UI still looks identical to the original, but is now driven by the DB.

## Step 6: Implement Dynamic Property Detail Page
Repeat the process for the specific property detail page.
1. In `assets::routes.rs`, create `pub async fn page_property(...)` that takes a `Path(id): Path<Uuid>`.
2. Query the exact asset by ID from the database.
3. In `main.rs`, update the route: `.route("/property/:id", get(assets::routes::page_property))`. (Remove the old `/property` route).
4. Modify `frontend/platform/property.html` using MiniJinja to inject `{{ asset.title }}`, `{{ asset.description }}`, and financial metrics.

## Step 7: Test and Verify
1. Run `cargo run` in the backend.
2. Visit `http://localhost:8888/marketplace` in the browser.
3. Verify that the properties displayed are exactly the ones inserted into the database in Step 1.
4. Click on a property to view its details page and ensure the URL `id` routing works.
