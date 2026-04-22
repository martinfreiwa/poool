---
description: Implement robust, industry-standard cart page functionality
---

# Cart Page Implementation Workflow

The cart page (`/cart`) currently has several high-priority functionality and synchronization bugs. This workflow outlines an industry-standard implementation strategy that covers optimistic UI updates, debounced API calls, secure atomic backend operations, and responsive parity between desktop and mobile versions.

## Prerequisites
- The backend server should be running locally.
- Access to the `/cart` and `/api/cart` routes.
- A user logged into the application with at least one item in their cart.
- You should have `cart.js`, `cart.html`, and `backend/src/cart/routes.rs` open.

---

## Phase 1: Secure Backend Quantity Updating

The current backend implementation of `/cart/update` ignores the user input and forces all token quantities to `1`, meaning cart increments/decrements fail to persist. 
We need to refactor `update_cart_item` to be atomic and secure.

1. **Fix Token Parsing**:
   - Locate `update_cart_item` in `backend/src/cart/routes.rs`.
   - Update `UpdateCartForm` to accept `tokens_quantity` or a `delta` instead of raw `amount_cents`. Wait, no, the existing form has `amount_cents` but since the client only knows the increment (+1/-1), it's much safer to accept a relative adjustment, e.g., `UpdateCartForm { cart_item_id, delta: i32 }`. Alternatively, the client should send the exact `tokens_quantity`. 
   - We will update the struct to `pub tokens_quantity: i32;` 
   - Update `routes.rs` to parse `tokens_quantity`.

2. **Atomic DB Update**:
   - Ensure the query checks that `tokens_quantity >= 1` before updating to prevent negative token counts.
   - Update the SQL query in `update_cart_item` to:
     ```sql
     UPDATE cart_items 
     SET tokens_quantity = $1 
     WHERE id = $2 AND user_id = $3
     ```
   - Bind the actual calculated `tokens_quantity`.

3. **Return Standard Responses**:
   - Send back proper JSON or redirect appropriately so the frontend knows the operation succeeded.

---

## Phase 2: Frontend Optimistic UI & Local State (cart.js)

The current `cart.js` only updates the display `USD 1025` but ignores the actual quantity counter and never persists changes to the database. We need to implement optimistic tracking with debouncing.

1. **State Tracking**:
   - Update `cart.js` `handleQuantityChange` to find *both* the price box AND the actual quantity display span (e.g. `.quantity-display` beside the buttons).
   - Change the numerical counter locally immediately (Optimistic Update).

2. **Format Prices Consistently**:
   - Implement a robust `formatCurrency(cents)` or `formatUSD(dollars)` utility in `cart.js` that outputs `USD 1,025` (with commas) string instead of just concatenating `USD ` and the number.

3. **Debounced DB Persistence**:
   - Since users spam `+` and `-` buttons rapidly, firing an API call on every click could race-condition or spam the backend. 
   - Introduce a `debounce(fn, delay)` helper function.
   - Wait ~500ms after the user finishes clicking before making a `fetch()` POST call to `/cart/update` asynchronously.
   - If the API returns an error, gracefully revert the local UI state back to what the server indicated.

4. **Update Loading States**:
   - During the fetch call, optionally disable the checkout button or show a minor loading context.

---

## Phase 3: HTML Template Structure Parity (`cart.html`)

1. **Mobile Cart Parity**:
   - The desktop cart dynamically renders items based on the backend loop. The Mobile Cart section (`.mobile-cart-wrapper`) statically loops into an "Empty Cart" message.
   - Re-use the Rust `cart_items_html` injection logic or adapt it so that mobile items ALSO populate dynamically alongside desktop items, or use responsive CSS to shape the desktop cards into mobile cards without duplicating the DOM loops.

2. **Dynamic Rewards Banner**:
   - The summary box shows a hardcoded "Invest USD 17,500 or more to unlock Premium".
   - Modify the `cart_page_header` and API logic to optionally query the user's `tier_target_amount` and display the genuine distance to their next tier.

3. **Dynamic Asset Images**:
   - In `routes.rs` DB query, also fetch `a.image_url` or similar (from `asset_images`). 
   - Render the actual property image rather than the hardcoded placeholder string `/static/images/Portfolio asset details/Property image.png`.

---

## Phase 4: End-to-End Validation

1. **Check Persistence**:
   - Open the browser, add an item to the cart.
   - Use the `+` button to increase the quantity to 5 tokens.
   - Wait 1-2 seconds, then refresh the browser. Validate that the quantity stays at 5 and the price updates accordingly.

2. **Check Formatting**:
   - Ensure the total display correctly uses commas for thousands (e.g. `USD 1,500` instead of `USD 1500`).

3. **Check Mobile**:
   - Inspect the page using mobile device simulation (width <= 768px). 
   - Validate that the cart items do not show an empty state when they are loaded via the backend.

4. **Run Integration Tests**:
   - Write a python script using `requests` to login and navigate to `/cart`.
   - Programmatically POST to `/cart/update` with `tokens_quantity=3` and verify `GET /api/cart` reflects the changes correctly.

---

**Execution Step:** Run these phases systematically prioritizing Phase 1 & 2 to fix the primary data loss bugs.
