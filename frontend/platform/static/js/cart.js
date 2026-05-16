// Cart functionality for property item cards

// XSS-safe HTML escaper
function escapeHtml(str) {
  if (str == null) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

// Debounce function for API calls
const debounce = (func, delay) => {
  let timeoutId;
  return (...args) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
      func.apply(null, args);
    }, delay);
  };
};

function showCartPageAlert() {
  const params = new URLSearchParams(window.location.search);
  const error = params.get("error");
  if (!error || !document.getElementById("cart-page-content")) return;

  const messages = {
    sold_out: "That asset is sold out, so it was not added to your cart.",
    invalid_amount: "Enter a valid investment amount before adding an asset to your cart.",
  };

  const message = messages[error];
  if (!message) return;

  const wrapper = document.getElementById("cart-content-wrapper") || document.getElementById("cart-page-content");
  if (!wrapper || document.querySelector(".cart-page-alert")) return;

  const alert = document.createElement("div");
  alert.className = "cart-page-alert";
  alert.setAttribute("role", "alert");
  alert.textContent = message;
  wrapper.prepend(alert);
}

let cachedUsdToIdrRate = null;

function getUsdToIdrRate() {
  if (Number.isFinite(cachedUsdToIdrRate) && cachedUsdToIdrRate > 0) {
    return cachedUsdToIdrRate;
  }

  const summaryBox = document.getElementById("payment-summary-box");
  const candidates = [
    window.POOOL_USD_TO_IDR_RATE,
    window.POOOL_CART_DATA && window.POOOL_CART_DATA.usd_to_idr_rate,
    summaryBox && summaryBox.dataset.usdToIdrRate,
    document.body && document.body.dataset.usdToIdrRate,
  ];

  for (const candidate of candidates) {
    const rate = Number(candidate);
    if (Number.isFinite(rate) && rate > 0) {
      cachedUsdToIdrRate = rate;
      return rate;
    }
  }

  return null;
}

function formatApproxIdrFromUsd(usdAmount) {
  const rate = getUsdToIdrRate();
  if (!rate) return "";
  return `≈ Rp ${Math.round(usdAmount * rate).toLocaleString("de-DE").replace(/,/g, ".")}`;
}

function setApproxIdrFromUsd(element, usdAmount) {
  if (!element) return;
  element.textContent = formatApproxIdrFromUsd(usdAmount);
}

async function hydrateCartFxRate() {
  if (getUsdToIdrRate()) return;
  if (!document.getElementById("cart-page-content") && !document.querySelector(".checkout-invest-button")) return;

  try {
    const response = await fetch("/api/cart", { headers: { Accept: "application/json" } });
    if (!response.ok) return;
    const data = await response.json();
    const rate = Number(data && data.usd_to_idr_rate);
    if (Number.isFinite(rate) && rate > 0) {
      cachedUsdToIdrRate = rate;
      if (document.getElementById("cart-page-content")) updateCartTotal();
      else if (document.querySelector(".checkout-invest-button")) updateCheckoutTotal();
    }
  } catch (e) {
    console.warn("Could not hydrate cart FX rate:", e);
  }
}

// Persist quantity change to DB
const persistQuantityUpdate = debounce(async (cartId, newQuantity) => {
  try {
    const formData = new URLSearchParams();
    formData.append("cart_item_id", cartId);
    formData.append("tokens_quantity", newQuantity);

    const response = await fetch("/cart/update", {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: formData.toString(),
    });

    const data = await response.json();
    if (!data.success) {
      const errMsg = data.error || "Unknown error";
      console.error("Cart update failed:", errMsg);
      // Surface lock / availability failures to the user
      if (
        response.status === 409 ||
        response.status === 423 ||
        /lock|availability/i.test(errMsg)
      ) {
        showCartInlineError("This asset is temporarily unavailable. Please try again.");
      }
    }
  } catch (e) {
    console.error("Error updating cart quantity:", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
}, 500);

// Show a temporary inline error banner at the top of the cart/checkout page
function showCartInlineError(message) {
  // Reuse the existing cart-page-alert mechanism if present
  const wrapper = document.getElementById("cart-content-wrapper") ||
                  document.getElementById("cart-page-content") ||
                  document.querySelector(".checkout-payment-wrapper");
  if (!wrapper) return;

  // Remove any existing inline error so we don't stack duplicates
  const existing = wrapper.querySelector(".cart-inline-error");
  if (existing) existing.remove();

  const alert = document.createElement("div");
  alert.className = "cart-page-alert cart-inline-error";
  alert.setAttribute("role", "alert");
  alert.textContent = message;
  wrapper.prepend(alert);

  // Auto-dismiss after 5 s
  setTimeout(() => {
    if (alert.parentNode) alert.remove();
  }, 5000);
}

// Handle quantity changes for property items
function handleQuantityChange(button) {
  const itemId = button.dataset.itemId;
  const cartId = button.dataset.cartId;
  const unitPrice = parseFloat(button.dataset.unitPrice) || 0;
  const change = parseInt(button.dataset.change);

  // Find the relevant DOM elements
  const priceBox = document.getElementById(`${itemId}-price`);
  const qtyBox = document.getElementById(`${itemId}-qty`);

  if (!priceBox || !qtyBox) {
    return;
  }

  // Calculate new quantity
  const isInput = qtyBox.tagName === "INPUT";
  let currentQty = parseInt(isInput ? qtyBox.value : qtyBox.textContent) || 1;
  let newQty = currentQty + change;

  // Enforce minimum of 1 token
  if (newQty < 1) {
    newQty = 1;
  }

  // Enforce maximum tokens available
  const availableTokens = parseInt(button.dataset.available) || 0;
  const totalTokens = parseInt(button.dataset.total) || 1;

  if (newQty > availableTokens) {
    newQty = availableTokens;
  }

  // Since we changed quantity, if it didn't actually change, do nothing
  if (newQty === currentQty) return;

  // Optimistically update the UI metrics
  if (isInput) {
    qtyBox.value = newQty;
  } else {
    qtyBox.textContent = newQty;
  }

  const newPrice = newQty * unitPrice;

  // Format using standard locale string for commas
  const formatOpts = { minimumFractionDigits: 2, maximumFractionDigits: 2 };
  priceBox.textContent = `USD ${newPrice.toLocaleString("en-US", formatOpts)}`;

  const tokensLabel = document.getElementById(`${itemId}-tokens-label`);
  if (tokensLabel) {
    tokensLabel.textContent = newQty;
  }

  // Update Order Summary panel line item (summary-item-{idx}-*)
  const idxMatch = itemId.match(/(\d+)$/);
  if (idxMatch) {
    const idx = idxMatch[1];
    const summaryQtyEl = document.getElementById(`summary-item-${idx}-qty`);
    if (summaryQtyEl) summaryQtyEl.textContent = `${newQty} × $${unitPrice.toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
    const summaryUsdEl = document.getElementById(`summary-item-${idx}-usd`);
    if (summaryUsdEl) summaryUsdEl.textContent = `USD ${newPrice.toLocaleString("en-US", formatOpts)}`;
    const summaryIdrEl = document.getElementById(`summary-item-${idx}-idr`);
    setApproxIdrFromUsd(summaryIdrEl, newPrice);
  }

  // Update Progress Bar
  if (totalTokens > 0) {
    const newSoldTokens = totalTokens - availableTokens;
    let newPct = Math.min(
      100,
      Math.max(0, Math.round((newSoldTokens / totalTokens) * 100)),
    );
    if (newPct === 0 && newSoldTokens > 0) newPct = 1;

    const progressFill = document.getElementById(`${itemId}-progress`);
    if (progressFill) progressFill.style.width = `${newPct}%`;

    const textDark = document.getElementById(`${itemId}-funded-text-dark`);
    if (textDark) textDark.textContent = `${newPct}% funded`;

    const textLight = document.getElementById(`${itemId}-funded-text-light`);
    if (textLight) textLight.textContent = `${newPct}% funded`;

    const progressTextFallback = document.getElementById(`${itemId}-funded-text`);
    if (progressTextFallback) progressTextFallback.textContent = `${newPct}% funded`;
  }

  // Update totals based on which page we're on
  if (document.getElementById("cart-page-content")) {
    updateCartTotal();
  } else if (document.querySelector(".checkout-invest-button")) {
    updateCheckoutTotal();
  }

  // Trigger debounced backend save
  if (cartId) {
    persistQuantityUpdate(cartId, newQty);
  }
}

// Handle manual quantity input
function handleQuantityInput(input) {
  const itemId = input.dataset.itemId;
  const cartId = input.dataset.cartId;
  const unitPrice = parseFloat(input.dataset.unitPrice) || 0;

  const priceBox = document.getElementById(`${itemId}-price`);
  if (!priceBox) return;

  // Parse input
  let newQty = parseInt(input.value);

  // Wait if empty (user clearing field before typing)
  if (isNaN(newQty)) {
    return;
  }

  // Enforce limits
  if (newQty < 1) newQty = 1;

  const availableTokens = parseInt(input.dataset.available) || 0;
  const totalTokens = parseInt(input.dataset.total) || 1;

  if (newQty > availableTokens) {
    newQty = availableTokens;
  }

  // Set corrected value back to input
  input.value = newQty;

  // Update UI components
  const newPrice = newQty * unitPrice;
  const formatOpts = { minimumFractionDigits: 2, maximumFractionDigits: 2 };
  priceBox.textContent = `USD ${newPrice.toLocaleString("en-US", formatOpts)}`;

  const tokensLabel = document.getElementById(`${itemId}-tokens-label`);
  if (tokensLabel) tokensLabel.textContent = newQty;

  // Update Order Summary panel line item (summary-item-{idx}-*)
  const idxMatchInput = itemId.match(/(\d+)$/);
  if (idxMatchInput) {
    const idx = idxMatchInput[1];
    const summaryQtyEl = document.getElementById(`summary-item-${idx}-qty`);
    if (summaryQtyEl) summaryQtyEl.textContent = `${newQty} × $${unitPrice.toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
    const summaryUsdEl = document.getElementById(`summary-item-${idx}-usd`);
    if (summaryUsdEl) summaryUsdEl.textContent = `USD ${newPrice.toLocaleString("en-US", formatOpts)}`;
    const summaryIdrEl = document.getElementById(`summary-item-${idx}-idr`);
    setApproxIdrFromUsd(summaryIdrEl, newPrice);
  }

  // Update Progress Bar
  if (totalTokens > 0) {
    const newSoldTokens = totalTokens - availableTokens;
    let newPct = Math.min(
      100,
      Math.max(0, Math.round((newSoldTokens / totalTokens) * 100)),
    );
    if (newPct === 0 && newSoldTokens > 0) newPct = 1;

    const progressFill = document.getElementById(`${itemId}-progress`);
    if (progressFill) progressFill.style.width = `${newPct}%`;

    const textDark = document.getElementById(`${itemId}-funded-text-dark`);
    if (textDark) textDark.textContent = `${newPct}% funded`;

    const textLight = document.getElementById(`${itemId}-funded-text-light`);
    if (textLight) textLight.textContent = `${newPct}% funded`;

    const progressTextFallback = document.getElementById(`${itemId}-funded-text`);
    if (progressTextFallback) progressTextFallback.textContent = `${newPct}% funded`;
  }

  if (document.getElementById("cart-page-content")) {
    updateCartTotal();
  } else if (document.querySelector(".checkout-invest-button")) {
    updateCheckoutTotal();
  }

  if (cartId) {
    persistQuantityUpdate(cartId, newQty);
  }
}

// Update cart total
function updateCartTotal() {
  const totalElement = document.querySelector(".cart-total-row .total-amount");
  const subtotalElement = document.getElementById("cart-subtotal-amount");
  const feeElement = document.getElementById("cart-fee-amount");
  const feeIdrElement = document.getElementById("cart-fee-idr");

  let total = 0;

  // Calculate total from data attributes to avoid brittle DOM text parsing
  const quantityInputs = document.querySelectorAll(".quantity-input");
  
  if (quantityInputs.length > 0) {
    quantityInputs.forEach((input) => {
      // Check if this input belongs to the active view (desktop vs mobile cart)
      if (input.closest(".mobile-cart-item-card") && window.innerWidth > 768) return;
      if (input.closest(".cart-item-card") && window.innerWidth <= 768) return;
      
      const qty = parseInt(input.value) || 0;
      const unitPrice = parseFloat(input.dataset.unitPrice) || 0;
      total += (qty * unitPrice);
    });
  } else {
    // Fallback for static summary displays without inputs
    const priceElements = document.querySelectorAll(
      ".property-item-price, .cart-item-card__price",
    );
    priceElements.forEach((priceEl) => {
      let price = priceEl.textContent.trim();
      price = price.replace(/USD\s*/g, "").replace(/[$,]/g, "");
      total += parseFloat(price) || 0;
    });
  }

  // Read fee percentage from data attribute set by the backend
  const summaryBox = document.getElementById("payment-summary-box");
  const feePct = summaryBox ? parseFloat(summaryBox.dataset.feePct) || 0 : 0;
  const fee = Math.round(total * feePct) / 100;
  const grandTotal = total + fee;

  // Format options to match backend (2 decimal places)
  const formatOpts = { minimumFractionDigits: 2, maximumFractionDigits: 2 };

  // Format and update all amounts
  const formattedSubtotal = "USD " + total.toLocaleString("en-US", formatOpts);
  const formattedTotal = "USD " + grandTotal.toLocaleString("en-US", formatOpts);
  const formattedFee = "USD " + fee.toLocaleString("en-US", formatOpts);

  if (totalElement) {
    totalElement.textContent = formattedTotal;
  }

  if (subtotalElement) {
    subtotalElement.textContent = formattedSubtotal;
  }

  if (feeElement) {
    feeElement.textContent = formattedFee;
  }

  if (feeIdrElement) {
    setApproxIdrFromUsd(feeIdrElement, fee);
  }

  // Update subtotal IDR
  const subtotalIdrEl = subtotalElement ? subtotalElement.closest(".summary-line-values") : null;
  if (subtotalIdrEl) {
    const idrSpan = subtotalIdrEl.querySelector(".summary-line-idr");
    setApproxIdrFromUsd(idrSpan, total);
  }

  // Also update summary section if it exists
  const summaryTotal = document.querySelector("#cart-summary-amount");
  if (summaryTotal) {
    summaryTotal.textContent = formattedTotal;
  }

  // Update total IDR
  const totalIdrEl = document.getElementById("cart-total-idr");
  if (totalIdrEl) {
    setApproxIdrFromUsd(totalIdrEl, grandTotal);
  }

  // Update checkout invest button if on checkout page
  updateCheckoutInvestButton(grandTotal);
}

// Handle item removal
function handleItemAction(button) {
  const action = button.dataset.action;
  const itemId = button.dataset.itemId;

  if (action === "removeFromCart") {
    removeFromCart(itemId);
  }
}

// Remove item from cart
function removeFromCart(itemId) {
  const itemElement = document.getElementById(itemId);
  if (itemElement) {
    itemElement.remove();
    updateCartTotal();

    // Check if cart is empty
    const remainingItems = document.querySelectorAll(".property-item-card, .cart-item-card");
    if (remainingItems.length === 0) {
      // Show empty cart state
      showEmptyCart();
    }
  }
}

// Show empty cart state
function showEmptyCart() {
  const cartContent = document.getElementById("cart-page-content");
  if (cartContent) {
    // This would typically be handled by the server
    // For now, just reload the page
    window.location.reload();
  }
}

// Proceed to checkout from cart
function proceedToCheckout() {
  window.location.href = "/checkout";
}

// Update checkout invest button amount
function updateCheckoutInvestButton(total) {
  // Check if we're on mobile to target the correct button
  const isMobile = window.innerWidth <= 768;

  // On mobile, find button within mobile container; on desktop, within desktop container
  let investButton;
  if (isMobile) {
    const mobileContainer = document.querySelector(".mobile-checkout-payment");
    if (mobileContainer) {
      investButton = mobileContainer.querySelector(".checkout-invest-text");
    }
  } else {
    const desktopContainer = document.querySelector(
      ".checkout-payment-wrapper",
    );
    if (desktopContainer) {
      investButton = desktopContainer.querySelector(".checkout-invest-text");
    }
  }

  // Fallback to querySelector if container not found
  if (!investButton) {
    investButton = document.querySelector(".checkout-invest-text");
  }

  if (investButton) {
    investButton.textContent = "Invest USD " + total;
  }
}

// Calculate total from checkout page property items
function updateCheckoutTotal() {
  let total = 0;

  // Check if we're on mobile to avoid double-counting
  const isMobile = window.innerWidth <= 768;

  // Get all property item prices on checkout page
  // On mobile: only query mobile elements (desktop elements are hidden but still in DOM)
  // On desktop: only query desktop elements
  const priceSelector = isMobile
    ? ".mobile-cart-price-text"
    : ".property-item-price";
  const priceElements = document.querySelectorAll(priceSelector);

  priceElements.forEach((priceEl) => {
    let price = priceEl.textContent.trim();
    // Remove USD prefix, $ sign and commas
    price = price.replace(/USD\s*/g, "").replace(/[$,]/g, "");
    total += parseFloat(price) || 0;
  });

  // Update invest button
  updateCheckoutInvestButton(total);

  // Also update any total displays on checkout
  const checkoutTotalElements = document.querySelectorAll(
    ".checkout-total-amount, .checkout-summary-total, .mobile-cart-total-amount",
  );
  checkoutTotalElements.forEach((elem) => {
    elem.textContent = "USD " + total;
  });
}

// Initialize on page load
document.addEventListener("DOMContentLoaded", function () {
  showCartPageAlert();
  // Check if we're on cart or checkout page
  if (document.getElementById("cart-page-content")) {
    updateCartTotal();
  } else if (document.querySelector(".checkout-invest-button")) {
    updateCheckoutTotal();
  }
  hydrateCartFxRate();
  startCheckoutTimer();
});

// Handle HTMX events if using HTMX
document.body.addEventListener("htmx:afterSwap", function (evt) {
  if (
    evt.detail.target.id === "cart-page-content" ||
    evt.detail.target.id === "cart-page-summary"
  ) {
    updateCartTotal();
  } else if (evt.detail.target.id === "checkout-content") {
    updateCheckoutTotal();
  }
  hydrateCartFxRate();
  startCheckoutTimer();
});

function startCheckoutTimer() {
  const timerElement = document.getElementById("checkout-timer");
  if (!timerElement) return;

  // Clear any existing interval to prevent multiple timers running
  if (window.checkoutTimerInterval) {
    clearInterval(window.checkoutTimerInterval);
  }

  // Set time to 10 minutes (in seconds)
  let timeLeft = 10 * 60;

  window.checkoutTimerInterval = setInterval(() => {
    timeLeft--;

    if (timeLeft < 0) {
      clearInterval(window.checkoutTimerInterval);
      timerElement.textContent = "00:00";
      return;
    }

    const minutes = Math.floor(timeLeft / 60);
    const seconds = timeLeft % 60;

    // Formatting: add leading zero if needed
    const displayMinutes = minutes < 10 ? "0" + minutes : minutes;
    const displaySeconds = seconds < 10 ? "0" + seconds : seconds;

    timerElement.textContent = `${displayMinutes}:${displaySeconds}`;
  }, 1000);
}
