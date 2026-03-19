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
        "/static/images/villa1.webp";

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

      // Send AJAX request to add to cart
      // fetch() follows 302 redirects automatically. We check response.url
      // to see if the server redirected to /kyc or /auth/login instead of /cart.
      fetch("/cart/add", {
        method: "POST",
        body: formData,
      })
        .then((response) => {
          // Check where the server actually sent us after following redirects
          const finalUrl = new URL(response.url, window.location.origin);
          if (finalUrl.pathname !== "/cart") {
            // Server redirected somewhere else (e.g. /kyc, /auth/login)
            window.location.href = finalUrl.pathname + finalUrl.search;
          } else {
            // Successfully added to cart, go to cart page
            window.location.href = "/cart";
          }
        })
        .catch((error) => {
          console.error("Add to cart error:", error);
          window.location.href = "/cart";
        });
    });
  }
});
