// Property Detail Cart Functionality
document.addEventListener("DOMContentLoaded", function () {
  // Check if gallery image exists on page load
  setTimeout(() => {
    const testImg = document.querySelector("#gallery-main-image img");
    if (testImg) {
    }
  }, 100);
  // Get the investment amount input
  const amountInput = document.getElementById("investment-amount-input");

  // Get all quick add buttons
  const quickAddButtons = document.querySelectorAll(".quick-add-btn");

  // Get the main add to cart button
  const addToCartBtn = document.getElementById("add-to-cart-main-btn");

  function getCookie(name) {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) {
      return decodeURIComponent(parts.pop().split(";").shift());
    }
    return "";
  }

  function showCartError(message) {
    if (!addToCartBtn) return;
    let errorEl = document.getElementById("property-cart-error");
    if (!errorEl) {
      errorEl = document.createElement("div");
      errorEl.id = "property-cart-error";
      errorEl.className = "property-cart-error";
      errorEl.setAttribute("role", "alert");
      addToCartBtn.insertAdjacentElement("afterend", errorEl);
    }
    errorEl.textContent = message;
  }

  function messageForCartError(code) {
    switch (code) {
      case "invalid_amount":
        return "Enter a valid investment amount before adding this asset.";
      case "sold_out":
        return "This asset is currently sold out.";
      case "asset_not_found":
        return "This asset is no longer available.";
      case "cart_unavailable":
      case "add_failed":
        return "Your cart could not be updated. Please try again.";
      default:
        return "Unable to add this asset to your cart. Please try again.";
    }
  }

  function setCartLoading(isLoading) {
    if (!addToCartBtn) return;
    addToCartBtn.disabled = isLoading;
    addToCartBtn.setAttribute("aria-busy", isLoading ? "true" : "false");
    const text = addToCartBtn.querySelector(".btn-text");
    if (text) {
      text.textContent = isLoading ? "Adding..." : "Add to cart";
    }
  }

  // Handle quick add buttons - accumulate amounts
  quickAddButtons.forEach((button) => {
    button.addEventListener("click", function (e) {
      e.preventDefault();
      const addAmount = parseInt(this.getAttribute("data-amount")) || 0;
      if (amountInput) {
        // Parse current amount (remove commas)
        const currentAmount =
          parseInt(amountInput.value.replace(/,/g, "")) || 0;
        const newAmount = currentAmount + addAmount;
        // Format with comma for display
        amountInput.value = newAmount.toLocaleString();
      }
    });
  });

  // Handle add to cart button
  if (addToCartBtn) {
    addToCartBtn.addEventListener("click", function (e) {
      e.preventDefault();

      // Get the investment amount (remove commas for processing)
      const amount = amountInput ? amountInput.value.replace(/,/g, "") : "2000";

      // Get property data from the page
      const propertyTitle =
        document.getElementById("property-title")?.textContent ||
        "Property Details";
      // Get property ID from query param or URL path
      let propertyId = new URLSearchParams(window.location.search).get("id");
      if (!propertyId) {
        const pathParts = window.location.pathname.split('/').filter(Boolean);
        if (pathParts.length >= 2 && (pathParts[0] === 'property' || pathParts[0] === 'commodity')) {
          propertyId = pathParts[1];
        } else {
          propertyId = "property-1";
        }
      }

      // Get the property image from the main gallery
      const galleryImg = document.querySelector("#gallery-main-image img");

      // Try different methods to get the image
      let propertyImage =
        galleryImg?.src ||
        galleryImg?.getAttribute("src") ||
        "/static/images/seed/villa1.webp";

      // If the src is a full URL, we might need to extract the path
      if (propertyImage.startsWith("http")) {
        try {
          const url = new URL(propertyImage);
          propertyImage = url.pathname; // Get just the path part
        } catch (e) { }
      }

      // Get the property location
      const propertyLocation =
        document.getElementById("property-location")?.textContent.trim() ||
        "Bali, Indonesia";

      // Extract dynamic values from the page
      const priceEl = document.querySelector(".price-amount");
      const unitPrice = priceEl ? priceEl.textContent.replace(/[^0-9]/g, "") + "00" : "0"; // cents
      const fundedEl = document.querySelector(".funded-text");
      const fundedPct = fundedEl ? fundedEl.textContent.replace(/[^0-9]/g, "") : "0";
      const returnRows = document.querySelectorAll(".returns-row .returns-value");
      const projectedReturn = returnRows[1]?.textContent.trim() || "0%";
      const annualizedReturn = returnRows[2]?.textContent.trim() || "0%";

      // Prepare form data
      const formData = new URLSearchParams();
      formData.append("investment_amount", amount);
      formData.append("property_id", propertyId);
      formData.append("property_title", propertyTitle);
      formData.append("property_image", propertyImage);
      formData.append("location", propertyLocation);
      formData.append("unit_price", unitPrice);
      formData.append("funded_percentage", fundedPct);
      formData.append("duration", "5 years");
      formData.append("projected_return", projectedReturn);
      formData.append("annualized_return", annualizedReturn);
      const csrfToken = getCookie("csrf_token");
      if (csrfToken) {
        formData.append("csrf_token", csrfToken);
      }

      // Send AJAX request to add to cart
      // fetch() follows 302 redirects automatically. We check response.url
      // to see if the server redirected to /kyc or /auth/login instead of /cart.
      setCartLoading(true);
      fetch("/cart/add", {
        method: "POST",
        headers: csrfToken ? { "X-CSRF-Token": csrfToken } : {},
        body: formData,
      })
        .then((response) => {
          if (!response.ok) {
            throw new Error("Unable to add this property to your cart. Please refresh and try again.");
          }
          // Check where the server actually sent us after following redirects
          const finalUrl = new URL(response.url, window.location.origin);
          if (finalUrl.pathname !== "/cart") {
            // Server redirected somewhere else (e.g. /kyc, /auth/login)
            window.location.href = finalUrl.pathname + finalUrl.search;
          } else if (finalUrl.searchParams.has("error")) {
            setCartLoading(false);
            showCartError(messageForCartError(finalUrl.searchParams.get("error")));
          } else {
            // Successfully added to cart, go to cart page
            window.location.href = finalUrl.pathname + finalUrl.search;
          }
        })
        .catch((error) => {
          console.warn("Add to cart failed:", error);
          setCartLoading(false);
          showCartError(error.message || "Unable to add this property to your cart. Please try again.");
        });
    });
  }
});
