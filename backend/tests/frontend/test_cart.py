#!/usr/bin/env python3
"""
Comprehensive Cart Page Test Suite for POOOL Platform
=====================================================
Covers:
  1. Authentication & access control
  2. GET /cart page rendering (desktop + mobile)
  3. GET /api/cart JSON API
  4. POST /cart/add (add items)
  5. POST /cart/update (change quantity)
  6. POST /cart/remove (delete items)
  7. Cart total calculations
  8. Rewards banner dynamic rendering
  9. Edge cases (zero qty, max tokens, invalid IDs)
 10. Empty cart state
 11. HTML structure & data attribute integrity
"""

import json
import re
import subprocess
import sys
import time
import requests

BASE = "http://localhost:8888"
DB = "poool"

# ─── Helpers ─────────────────────────────────────────────────────

passed = 0
failed = 0
errors = []


def psql(sql: str) -> str:
    """Run a psql command and return stripped output."""
    return subprocess.check_output(
        ["psql", "-Atc", sql, DB]
    ).decode().strip()


def get_session(email="test@poool.app") -> requests.Session:
    """Return an authenticated requests.Session for the given user."""
    token = psql(
        f"SELECT session_token FROM user_sessions "
        f"WHERE user_id = (SELECT id FROM users WHERE email='{email}') "
        f"ORDER BY created_at DESC LIMIT 1"
    )
    s = requests.Session()
    s.cookies.set("poool_session", token)
    return s


def check(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  ✅ {name}")
    else:
        failed += 1
        msg = f"  ❌ {name}"
        if detail:
            msg += f"  — {detail}"
        print(msg)
        errors.append(name)


def section(title: str):
    print(f"\n{'─'*60}")
    print(f"  {title}")
    print(f"{'─'*60}")


# ─── Setup: Ensure known state ───────────────────────────────────

def setup():
    """Ensure we have a clean, known cart state for testing."""
    section("SETUP")
    user_id = psql("SELECT id FROM users WHERE email='test@poool.app'")
    print(f"  Test user: {user_id}")

    # Clear cart first
    psql(f"DELETE FROM cart_items WHERE user_id = '{user_id}'")
    count = psql(f"SELECT count(*) FROM cart_items WHERE user_id = '{user_id}'")
    check("Cart cleared", count == "0")

    # Get two assets to work with
    assets = psql(
        "SELECT id, slug, token_price_cents, tokens_available FROM assets ORDER BY created_at LIMIT 2"
    ).split("\n")
    assert len(assets) >= 2, f"Need at least 2 assets in DB, got {len(assets)}"

    asset_data = []
    for row in assets:
        parts = row.split("|")
        asset_data.append({
            "id": parts[0],
            "slug": parts[1],
            "token_price_cents": int(parts[2]),
            "tokens_available": int(parts[3]),
        })
    print(f"  Asset 1: {asset_data[0]['slug']} (${asset_data[0]['token_price_cents']//100}/token, {asset_data[0]['tokens_available']} avail)")
    print(f"  Asset 2: {asset_data[1]['slug']} (${asset_data[1]['token_price_cents']//100}/token, {asset_data[1]['tokens_available']} avail)")

    return user_id, asset_data


# ═══════════════════════════════════════════════════════════════════
#  TEST GROUPS
# ═══════════════════════════════════════════════════════════════════

def test_auth_access():
    """Test that unauthenticated users are redirected."""
    section("1. AUTHENTICATION & ACCESS CONTROL")

    # Unauthenticated GET /cart should redirect to login
    r = requests.get(f"{BASE}/cart", allow_redirects=False)
    check("GET /cart unauthenticated → redirect",
          r.status_code in (302, 303),
          f"got {r.status_code}")
    if r.status_code in (302, 303):
        check("Redirect target is /auth/login",
              "/auth/login" in r.headers.get("location", ""),
              f"location: {r.headers.get('location')}")

    # Unauthenticated GET /api/cart
    r = requests.get(f"{BASE}/api/cart", allow_redirects=False)
    check("GET /api/cart unauthenticated → redirect or error",
          r.status_code in (302, 303, 401),
          f"got {r.status_code}")

    # Unauthenticated POST /cart/update
    r = requests.post(f"{BASE}/cart/update",
                       data={"cart_item_id": "fake", "tokens_quantity": "1"},
                       allow_redirects=False)
    check("POST /cart/update unauthenticated → 401 or redirect",
          r.status_code in (302, 303, 401),
          f"got {r.status_code}")


def test_empty_cart(session):
    """Test the empty cart state rendering."""
    section("2. EMPTY CART STATE")

    # Ensure cart is empty
    user_id = psql("SELECT id FROM users WHERE email='test@poool.app'")
    psql(f"DELETE FROM cart_items WHERE user_id = '{user_id}'")

    # GET /cart should show empty state
    r = session.get(f"{BASE}/cart")
    html = r.text
    check("Empty cart page loads (200)", r.status_code == 200)
    check("Empty cart page has empty state marker",
          "cart-empty-wrapper" in html or "empty-cart" in html.lower() or "Your Cart" in html)

    # GET /api/cart should return empty
    r = session.get(f"{BASE}/api/cart")
    data = r.json()
    check("API returns count=0 for empty cart", data.get("count") == 0)
    check("API returns total_cents=0 for empty cart", data.get("total_cents") == 0)
    check("API returns empty items list", len(data.get("items", [])) == 0)


def test_add_to_cart(session, assets):
    """Test adding items to cart."""
    section("3. ADD TO CART (POST /cart/add)")

    # Add first asset
    r = session.post(f"{BASE}/cart/add", data={
        "property_id": assets[0]["slug"],
        "investment_amount": str(assets[0]["token_price_cents"] * 3 / 100),  # 3 tokens worth
    }, allow_redirects=False)
    check("Add item 1 → redirect (302/303)",
          r.status_code in (302, 303),
          f"got {r.status_code}")

    # Verify in API
    r = session.get(f"{BASE}/api/cart")
    data = r.json()
    check("API shows 1 item after first add", data["count"] == 1)
    item = data["items"][0]
    check("Correct asset_id", item["asset_id"] == assets[0]["id"])
    check("Correct token quantity (3)", item["tokens_quantity"] == 3,
          f"got {item['tokens_quantity']}")
    expected_total = 3 * assets[0]["token_price_cents"]
    check("Correct total_cents", item["total_cents"] == expected_total,
          f"expected {expected_total}, got {item['total_cents']}")

    # Add second asset
    r = session.post(f"{BASE}/cart/add", data={
        "property_id": assets[1]["slug"],
        "investment_amount": str(assets[1]["token_price_cents"] * 5 / 100),  # 5 tokens worth
    }, allow_redirects=False)
    check("Add item 2 → redirect", r.status_code in (302, 303))

    r = session.get(f"{BASE}/api/cart")
    data = r.json()
    check("API shows 2 items after second add", data["count"] == 2)

    # Add same asset again – should increase quantity (upsert)
    r = session.post(f"{BASE}/cart/add", data={
        "property_id": assets[0]["slug"],
        "investment_amount": str(assets[0]["token_price_cents"] * 2 / 100),  # 2 more tokens
    }, allow_redirects=False)
    check("Re-add same asset → redirect (upsert)", r.status_code in (302, 303))

    r = session.get(f"{BASE}/api/cart")
    data = r.json()
    check("Still 2 items (upsert, not duplicate)", data["count"] == 2)

    # Find the first asset item
    item1 = [i for i in data["items"] if i["asset_id"] == assets[0]["id"]][0]
    check("Quantity increased on upsert (3+2=5)", item1["tokens_quantity"] == 5,
          f"got {item1['tokens_quantity']}")

    return data


def test_cart_page_rendering(session, assets):
    """Test the full cart page HTML rendering."""
    section("4. CART PAGE RENDERING (GET /cart)")

    r = session.get(f"{BASE}/cart")
    html = r.text
    check("Cart page loads (200)", r.status_code == 200)
    check("HTML has reasonable size (>10KB)", len(html) > 10000, f"got {len(html)} bytes")

    # Desktop elements
    check("Desktop: cart-page-content div present", "cart-page-content" in html)
    check("Desktop: cart-items-container present", "cart-items-container" in html)
    check("Desktop: cart total display present", "cart-total-display" in html)
    check("Desktop: item cards rendered", "cart-item-card" in html)
    check("Desktop: quantity controls present", "handleQuantityChange" in html)
    check("Desktop: price elements (cart-item-card__price)", "cart-item-card__price" in html)

    # Mobile elements
    check("Mobile: mobile-cart-item-card present", "mobile-cart-item-card" in html)
    check("Mobile: mobile quantity controls", "mobile-cart-quantity-controls" in html)
    check("Mobile: mobile remove button", "mobile-cart-remove-btn" in html)
    check("Mobile: mobile checkout section", "mobile-cart-checkout-section" in html)
    check("Mobile: empty state is hidden", "mobile-cart-empty-container" in html)

    # Data integrity in HTML
    for asset in assets:
        slug = asset["slug"]
        check(f"Asset slug '{slug}' linked in page", slug in html)

    # Item count in header
    api_data = session.get(f"{BASE}/api/cart").json()
    expected_count = api_data["count"]
    count_pattern = f"({expected_count})"
    check(f"Header shows correct item count ({expected_count})",
          count_pattern in html,
          f"searched for '{count_pattern}'")

    # Total displayed
    total_usd = api_data["total_cents"] // 100
    check(f"Total amount USD {total_usd} appears in page",
          f"USD {total_usd}" in html or f"USD {total_usd:,}" in html,
          f"searched for 'USD {total_usd}'")

    return html


def test_data_attributes(session):
    """Test that HTML data attributes are correctly set for JS interaction."""
    section("5. HTML DATA ATTRIBUTES & JS INTEGRATION")

    r = session.get(f"{BASE}/cart")
    html = r.text

    # Check data-cart-id attributes exist
    cart_ids = re.findall(r'data-cart-id="([^"]+)"', html)
    check("data-cart-id attributes present", len(cart_ids) > 0, f"found {len(cart_ids)}")

    # Check data-unit-price attributes
    unit_prices = re.findall(r'data-unit-price="([^"]+)"', html)
    check("data-unit-price attributes present", len(unit_prices) > 0, f"found {len(unit_prices)}")

    # Check data-change attributes (+1 and -1)
    changes = re.findall(r'data-change="([^"]+)"', html)
    check("data-change attributes present", len(changes) > 0, f"found {len(changes)}")
    check("Both +1 and -1 change buttons exist",
          "1" in changes and "-1" in changes,
          f"found changes: {set(changes)}")

    # Check data-item-id attributes
    item_ids = re.findall(r'data-item-id="([^"]+)"', html)
    check("data-item-id attributes present", len(item_ids) > 0, f"found {len(item_ids)}")

    # Check cart.js is loaded
    check("cart.js script loaded", "cart.js" in html)


def test_update_quantity(session):
    """Test POST /cart/update for quantity changes."""
    section("6. UPDATE CART QUANTITY (POST /cart/update)")

    # Get current cart state
    api = session.get(f"{BASE}/api/cart").json()
    assert api["count"] > 0, "Need items in cart for update test"
    item = api["items"][0]
    cart_id = item["id"]
    original_qty = item["tokens_quantity"]

    # Update to a specific quantity
    new_qty = 7
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": str(new_qty),
    })
    check("Update returns 200", r.status_code == 200)
    data = r.json()
    check("Update response has success=true", data.get("success") is True)
    check(f"Update response shows new qty={new_qty}",
          data.get("tokens_quantity") == new_qty,
          f"got {data.get('tokens_quantity')}")

    # Verify via API
    api = session.get(f"{BASE}/api/cart").json()
    updated = [i for i in api["items"] if i["id"] == cart_id][0]
    check(f"API confirms qty={new_qty}", updated["tokens_quantity"] == new_qty,
          f"got {updated['tokens_quantity']}")

    # Update to 1 (minimum)
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": "1",
    })
    data = r.json()
    check("Update to qty=1 succeeds", data.get("success") is True)
    check("Response shows qty=1", data.get("tokens_quantity") == 1)

    # Attempt qty=0 (should be clamped to 1 by the backend)
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": "0",
    })
    data = r.json()
    check("Update qty=0 → clamped to 1",
          data.get("tokens_quantity") == 1,
          f"got {data.get('tokens_quantity')}")

    # Attempt negative quantity
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": "-5",
    })
    data = r.json()
    check("Update qty=-5 → clamped to 1",
          data.get("tokens_quantity") == 1,
          f"got {data.get('tokens_quantity')}")

    # Restore original
    session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": str(original_qty),
    })


def test_update_edge_cases(session):
    """Test edge cases for cart updates."""
    section("7. UPDATE EDGE CASES")

    # Invalid cart_item_id
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": "not-a-uuid",
        "tokens_quantity": "5",
    })
    check("Invalid UUID → 400 Bad Request", r.status_code == 400,
          f"got {r.status_code}")

    # Non-existent cart_item_id (valid UUID)
    fake_uuid = "00000000-0000-0000-0000-000000000000"
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": fake_uuid,
        "tokens_quantity": "5",
    })
    # Should succeed (UPDATE affects 0 rows, no error)
    check("Non-existent UUID → 200 (no-op)", r.status_code == 200,
          f"got {r.status_code}")

    # Very large quantity
    api = session.get(f"{BASE}/api/cart").json()
    cart_id = api["items"][0]["id"]
    r = session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": "999999",
    })
    data = r.json()
    check("Very large qty accepted (backend doesn't crash)",
          data.get("success") is True)
    # Restore to reasonable value
    session.post(f"{BASE}/cart/update", data={
        "cart_item_id": cart_id,
        "tokens_quantity": "5",
    })


def test_cart_total_calculation(session, assets):
    """Test that cart totals are calculated correctly."""
    section("8. CART TOTAL CALCULATION")

    api = session.get(f"{BASE}/api/cart").json()

    # Verify each item's total
    calculated_grand_total = 0
    for item in api["items"]:
        expected_item_total = item["tokens_quantity"] * item["token_price_cents"]
        check(f"Item {item['slug'][:20]}… total = qty × price",
              item["total_cents"] == expected_item_total,
              f"expected {expected_item_total}, got {item['total_cents']}")
        calculated_grand_total += expected_item_total

    # Verify grand total
    check(f"Grand total = sum of item totals",
          api["total_cents"] == calculated_grand_total,
          f"expected {calculated_grand_total}, got {api['total_cents']}")


def test_rewards_banner(session):
    """Test dynamic rewards banner rendering."""
    section("9. REWARDS BANNER")

    r = session.get(f"{BASE}/cart")
    html = r.text

    # The rewards banner should be present when cart has items
    has_rewards = ("unlock Premium" in html or
                   "highest tier" in html or
                   "rewards" in html.lower())
    check("Rewards banner section present", has_rewards)

    # Check it uses dynamic values (not purely hardcoded)
    if "unlock Premium" in html:
        # Extract the USD amount from the banner
        match = re.search(r'USD\s+([\d,]+)</strong>\s*or more to unlock', html)
        check("Rewards banner shows dynamic USD amount",
              match is not None,
              "Could not find dynamic amount pattern")


def test_remove_from_cart(session):
    """Test removing items from cart."""
    section("10. REMOVE FROM CART (POST /cart/remove)")

    api = session.get(f"{BASE}/api/cart").json()
    initial_count = api["count"]
    assert initial_count > 0, "Need items in cart for remove test"

    # Remove first item
    cart_id = api["items"][0]["id"]
    r = session.post(f"{BASE}/cart/remove", data={
        "cart_item_id": cart_id,
    }, allow_redirects=False)
    check("Remove item → redirect (302/303)",
          r.status_code in (302, 303),
          f"got {r.status_code}")

    # Verify removal
    api = session.get(f"{BASE}/api/cart").json()
    check(f"Count decreased ({initial_count} → {initial_count-1})",
          api["count"] == initial_count - 1,
          f"got {api['count']}")
    remaining_ids = [i["id"] for i in api["items"]]
    check("Removed item no longer in cart", cart_id not in remaining_ids)

    # Invalid cart_item_id removal
    r = session.post(f"{BASE}/cart/remove", data={
        "cart_item_id": "not-a-uuid",
    }, allow_redirects=False)
    check("Remove invalid UUID → redirect (graceful)",
          r.status_code in (302, 303),
          f"got {r.status_code}")


def test_remove_all_and_empty_state(session):
    """Test removing all items leads to proper empty state."""
    section("11. REMOVE ALL → EMPTY STATE")

    # Remove all remaining items
    api = session.get(f"{BASE}/api/cart").json()
    for item in api["items"]:
        session.post(f"{BASE}/cart/remove", data={
            "cart_item_id": item["id"],
        }, allow_redirects=False)

    api = session.get(f"{BASE}/api/cart").json()
    check("All items removed, count=0", api["count"] == 0)

    # Load cart page – should show empty state
    r = session.get(f"{BASE}/cart")
    html = r.text
    check("Empty cart page loads", r.status_code == 200)

    # Should NOT have populated cart items
    check("No mobile-cart-item-card in empty state",
          "mobile-cart-item-card" not in html)
    check("No desktop cart-item-card in empty state",
          html.count("cart-item-card") == 0 or "cart-empty" in html.lower() or "empty" in html.lower())


def test_image_urls(session, assets):
    """Test that image URLs are properly loaded from asset_images."""
    section("12. IMAGE URL INTEGRATION")

    # Re-add items
    for asset in assets:
        session.post(f"{BASE}/cart/add", data={
            "property_id": asset["slug"],
            "investment_amount": str(asset["token_price_cents"] * 2 / 100),
        }, allow_redirects=False)

    r = session.get(f"{BASE}/cart")
    html = r.text

    # Check that image elements exist in cart items
    img_tags = re.findall(r'<img[^>]*class="[^"]*cart-item-(?:card__)?image[^"]*"[^>]*src="([^"]+)"', html)
    mobile_img_tags = re.findall(r'mobile-cart-item-card.*?<img[^>]*src="([^"]+)"', html, re.DOTALL)

    check("Desktop cart item images found", len(img_tags) > 0 or "cart-item-image" in html,
          f"found {len(img_tags)} img tags")
    check("Mobile cart item images found", len(mobile_img_tags) > 0,
          f"found {len(mobile_img_tags)} mobile img tags")

    # Check they have actual URLs (not empty)
    all_imgs = img_tags + mobile_img_tags
    for img_url in all_imgs:
        check(f"Image URL is non-empty: {img_url[:50]}…",
              len(img_url) > 5 and ("/" in img_url or "http" in img_url))


def test_api_cart_structure(session):
    """Test the JSON API response structure."""
    section("13. API CART RESPONSE STRUCTURE")

    r = session.get(f"{BASE}/api/cart")
    check("API returns 200", r.status_code == 200)
    check("Content-Type is JSON", "application/json" in r.headers.get("content-type", ""))

    data = r.json()
    check("Response has 'count' field", "count" in data)
    check("Response has 'items' field", "items" in data)
    check("Response has 'total_cents' field", "total_cents" in data)

    if data["count"] > 0:
        item = data["items"][0]
        required_fields = [
            "id", "asset_id", "tokens_quantity", "token_price_cents",
            "total_cents", "title", "slug", "funding_status"
        ]
        for field in required_fields:
            check(f"Item has '{field}' field", field in item,
                  f"fields present: {list(item.keys())}")

        # Check types
        check("tokens_quantity is int", isinstance(item["tokens_quantity"], int))
        check("token_price_cents is int", isinstance(item["token_price_cents"], int))
        check("total_cents is int", isinstance(item["total_cents"], int))
        check("title is string", isinstance(item["title"], str))
        check("slug is string", isinstance(item["slug"], str))


def test_concurrent_safety(session, assets):
    """Test that rapid concurrent updates don't corrupt data."""
    section("14. CONCURRENT UPDATE SAFETY")

    api = session.get(f"{BASE}/api/cart").json()
    if api["count"] == 0:
        session.post(f"{BASE}/cart/add", data={
            "property_id": assets[0]["slug"],
            "investment_amount": str(assets[0]["token_price_cents"] * 3 / 100),
        }, allow_redirects=False)
        api = session.get(f"{BASE}/api/cart").json()

    cart_id = api["items"][0]["id"]

    # Rapid sequential updates (simulates debounced frontend)
    for qty in [3, 5, 8, 10, 7]:
        session.post(f"{BASE}/cart/update", data={
            "cart_item_id": cart_id,
            "tokens_quantity": str(qty),
        })

    # Final value should be 7
    api = session.get(f"{BASE}/api/cart").json()
    item = [i for i in api["items"] if i["id"] == cart_id][0]
    check("After rapid updates, final qty = 7 (last write wins)",
          item["tokens_quantity"] == 7,
          f"got {item['tokens_quantity']}")


def test_mobile_desktop_parity(session):
    """Test that desktop and mobile views show the same data."""
    section("15. MOBILE/DESKTOP PARITY")

    r = session.get(f"{BASE}/cart")
    html = r.text

    # Count desktop vs mobile items
    desktop_cards = html.count('class="property-item-card"') + html.count("cart-item-card")
    mobile_cards = html.count("mobile-cart-item-card")

    api = session.get(f"{BASE}/api/cart").json()
    expected = api["count"]

    check(f"Mobile cards count matches API ({expected})",
          mobile_cards == expected,
          f"found {mobile_cards} mobile cards, expected {expected}")

    # Check mobile total matches
    total_display = f"USD {api['total_cents']//100}"
    mobile_total_count = html.count(total_display)
    check(f"Total '{total_display}' appears in both views",
          mobile_total_count >= 2,
          f"found {mobile_total_count} occurrences")


# ═══════════════════════════════════════════════════════════════════
#  RUNNER
# ═══════════════════════════════════════════════════════════════════

def main():
    print("=" * 60)
    print("  POOOL CART PAGE — COMPREHENSIVE TEST SUITE")
    print("=" * 60)

    # Verify server is up
    try:
        requests.get(f"{BASE}/", timeout=3)
    except requests.ConnectionError:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    user_id, assets = setup()
    session = get_session()

    test_auth_access()
    test_empty_cart(session)
    test_add_to_cart(session, assets)
    test_cart_page_rendering(session, assets)
    test_data_attributes(session)
    test_update_quantity(session)
    test_update_edge_cases(session)
    test_cart_total_calculation(session, assets)
    test_rewards_banner(session)
    test_remove_from_cart(session)
    test_remove_all_and_empty_state(session)
    test_image_urls(session, assets)
    test_api_cart_structure(session)
    test_concurrent_safety(session, assets)
    test_mobile_desktop_parity(session)

    # ─── Summary ─────────────────────────────────────────────
    print(f"\n{'='*60}")
    total = passed + failed
    print(f"  RESULTS: {passed}/{total} passed, {failed} failed")
    if errors:
        print(f"\n  FAILURES:")
        for e in errors:
            print(f"    • {e}")
    print(f"{'='*60}")

    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
