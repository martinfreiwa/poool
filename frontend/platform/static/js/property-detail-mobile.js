// Mobile Property Detail Gallery JavaScript
let currentSlideIndex = 0;

// New gallery scroll functions
function scrollGallery(direction) {
  const gallery = document.getElementById("mobileGalleryScroll");
  if (!gallery) return;

  const slides = gallery.querySelectorAll(".mobile-gallery-slide");
  const totalSlides = slides.length;
  if (totalSlides === 0) return;

  currentSlideIndex += direction;

  // Wrap around
  if (currentSlideIndex < 0) {
    currentSlideIndex = totalSlides - 1;
  } else if (currentSlideIndex >= totalSlides) {
    currentSlideIndex = 0;
  }

  // Scroll to the slide
  const slideWidth = slides[0].offsetWidth;
  gallery.scrollTo({
    left: currentSlideIndex * slideWidth,
    behavior: "smooth",
  });

  // Update indicators
  updateIndicators(currentSlideIndex);
}

function goToSlide(index) {
  const gallery = document.getElementById("mobileGalleryScroll");
  if (!gallery) return;

  const slides = gallery.querySelectorAll(".mobile-gallery-slide");
  if (slides.length === 0) return;

  currentSlideIndex = index;

  const slideWidth = slides[0].offsetWidth;
  gallery.scrollTo({
    left: index * slideWidth,
    behavior: "smooth",
  });

  updateIndicators(index);
}

function updateIndicators(activeIndex) {
  const dots = document.querySelectorAll(".mobile-gallery-dot");
  dots.forEach((dot, index) => {
    if (index === activeIndex) {
      dot.classList.add("active");
    } else {
      dot.classList.remove("active");
    }
  });
}

// Setup scroll listener to update indicators
function setupScrollListener() {
  const gallery = document.getElementById("mobileGalleryScroll");
  if (!gallery) return;

  let isScrolling;

  gallery.addEventListener("scroll", function () {
    // Clear timeout while scrolling
    clearTimeout(isScrolling);

    // Set a timeout to run after scrolling ends
    isScrolling = setTimeout(function () {
      const slideWidth = gallery.querySelector(
        ".mobile-gallery-slide",
      )?.offsetWidth;
      if (!slideWidth) return;

      const scrollPosition = gallery.scrollLeft;
      const newIndex = Math.round(scrollPosition / slideWidth);

      if (newIndex !== currentSlideIndex) {
        currentSlideIndex = newIndex;
        updateIndicators(currentSlideIndex);
      }
    }, 50);
  });
}

// Touch swipe support for gallery
function initializeMobileGallerySwipe() {
  const galleryScroll = document.getElementById("mobileGalleryScroll");
  if (!galleryScroll) return;

  // Just let CSS handle the scroll snap behavior
  // We already have scroll-snap-stop: always in CSS
}

// Dot navigation
function initializeDotNavigation() {
  const dots = document.querySelectorAll(".mobile-gallery-dot");
  dots.forEach((dot, index) => {
    dot.addEventListener("click", function () {
      goToSlide(index);
    });
  });
}

// Quick amount buttons
function initializeQuickAmounts() {
  if (document.body.classList.contains("property-public-body")) return;
  // Fix: Use the correct class selector for mobile quick buttons
  const quickBtns = document.querySelectorAll(".mobile-quick-btn");
  const amountInput = document.getElementById("mobile-investment-amount");

  quickBtns.forEach((btn) => {
    btn.addEventListener("click", function (e) {
      e.preventDefault();
      const amount = parseInt(this.getAttribute("data-amount"));
      if (amountInput && amount) {
        const currentValue = parseInt(amountInput.value.replace(/,/g, "")) || 0;
        const newValue = currentValue + amount;
        amountInput.value = newValue.toLocaleString();
      }
    });
  });
}

// Add to cart functionality
function initializeAddToCart() {
  const addToCartBtn = document.getElementById("mobile-add-to-cart-btn");
  const amountInput = document.getElementById("mobile-investment-amount");

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
    let errorEl = document.getElementById("mobile-property-cart-error");
    if (!errorEl) {
      errorEl = document.createElement("div");
      errorEl.id = "mobile-property-cart-error";
      errorEl.className = "property-cart-error mobile-property-cart-error";
      errorEl.setAttribute("role", "alert");
      addToCartBtn.insertAdjacentElement("afterend", errorEl);
    }
    errorEl.textContent = message;
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

  if (addToCartBtn) {
    addToCartBtn.addEventListener("click", function (e) {
      e.preventDefault();

      // Get EXACTLY what desktop sends - from mobile elements
      const amount = amountInput ? amountInput.value.replace(/,/g, "") : "2000";
      let propertyId = new URLSearchParams(window.location.search).get("id");
      if (!propertyId) {
        const pathParts = window.location.pathname.split('/').filter(Boolean);
        if (pathParts.length >= 2 && (pathParts[0] === 'property' || pathParts[0] === 'commodity')) {
          propertyId = pathParts[1];
        } else {
          propertyId = "property-1";
        }
      }

      // Get property title from mobile element
      const propertyTitle =
        document.querySelector(".mobile-property-title")?.textContent ||
        "Property Details";

      // Get first image from gallery
      const galleryImg = document.querySelector(".mobile-gallery-img");
      let propertyImage = galleryImg?.src || "/static/images/seed/villa1.webp";
      if (propertyImage.startsWith("http")) {
        propertyImage = new URL(propertyImage).pathname;
      }

      // Get location from the location badge
      const locationBadge = Array.from(
        document.querySelectorAll(".mobile-category-badge"),
      ).find((badge) => badge.querySelector('img[alt="Location"]'));
      const propertyLocation =
        locationBadge?.querySelector("span")?.textContent || "Bali, Indonesia";

      // Extract dynamic values from the page
      const priceEl = document.querySelector(".price-amount") || document.querySelector(".mobile-price-amount");
      const unitPrice = priceEl ? priceEl.textContent.replace(/[^0-9]/g, "") + "00" : "0";
      const fundedEl = document.querySelector(".funded-text") || document.querySelector(".mobile-funded-text");
      const fundedPct = fundedEl ? fundedEl.textContent.replace(/[^0-9]/g, "") : "0";
      const returnRows = document.querySelectorAll(".returns-row .returns-value, .mobile-returns-value");
      const projectedReturn = returnRows[1]?.textContent.trim() || "0%";
      const annualizedReturn = returnRows[2]?.textContent.trim() || "0%";

      // Send EXACT same data as desktop
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
          } else {
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
}

// Tooltip click handling for mobile
function initializeTooltips() {
  // Toggle tooltip on icon click
  document.addEventListener("click", function (e) {
    // Check if clicked element is a tooltip icon
    const tooltipIcon = e.target.closest(
      ".mobile-help-icon, .mobile-info-bar-icon",
    );

    if (tooltipIcon) {
      e.preventDefault();
      e.stopPropagation();

      const container = tooltipIcon.closest(".mobile-tooltip-container");
      if (container) {
        // Close all other tooltips
        document
          .querySelectorAll(".mobile-tooltip-container.active")
          .forEach(function (activeContainer) {
            if (activeContainer !== container) {
              activeContainer.classList.remove("active");
            }
          });

        // Toggle current tooltip
        container.classList.toggle("active");
      }
    } else {
      // Close all tooltips when clicking outside
      if (!e.target.closest(".mobile-tooltip-container")) {
        document
          .querySelectorAll(".mobile-tooltip-container.active")
          .forEach(function (container) {
            container.classList.remove("active");
          });
      }
    }
  });
}

// Financial tabs switching
function switchMobileFinancialTab(tabName) {
  // Get tab buttons
  const propertyCostTab = document.getElementById("mobile-tab-property-cost");
  const rentalIncomeTab = document.getElementById("mobile-tab-rental-income");

  // Get content sections
  const propertyCostContent = document.getElementById(
    "mobile-property-cost-content",
  );
  const rentalIncomeContent = document.getElementById(
    "mobile-rental-income-content",
  );

  if (tabName === "property-cost") {
    // Activate property cost tab
    if (propertyCostTab) {
      propertyCostTab.classList.add("active");
    }
    if (rentalIncomeTab) {
      rentalIncomeTab.classList.remove("active");
    }
    if (propertyCostContent) {
      propertyCostContent.style.display = "block";
    }
    if (rentalIncomeContent) {
      rentalIncomeContent.style.display = "none";
    }
  } else if (tabName === "rental-income") {
    // Activate rental income tab
    if (rentalIncomeTab) {
      rentalIncomeTab.classList.add("active");
    }
    if (propertyCostTab) {
      propertyCostTab.classList.remove("active");
    }
    if (rentalIncomeContent) {
      rentalIncomeContent.style.display = "block";
    }
    if (propertyCostContent) {
      propertyCostContent.style.display = "none";
    }
  }

  // Close all tooltips when switching tabs
  document
    .querySelectorAll(".mobile-tooltip-container.active")
    .forEach(function (container) {
      container.classList.remove("active");
    });
}

// Lightbox swipe navigation
function initializeLightboxSwipe() {
  const lightbox = document.getElementById("lightbox-modal");
  if (!lightbox) return;

  let touchStartX = 0;
  let touchEndX = 0;
  let touchStartY = 0;
  let touchEndY = 0;

  lightbox.addEventListener(
    "touchstart",
    function (e) {
      // Only listen to touches on the image itself
      if (e.target.classList.contains("lightbox-content")) {
        touchStartX = e.changedTouches[0].screenX;
        touchStartY = e.changedTouches[0].screenY;
      }
    },
    { passive: true },
  );

  lightbox.addEventListener(
    "touchend",
    function (e) {
      // Only handle swipes on the image
      if (e.target.classList.contains("lightbox-content")) {
        touchEndX = e.changedTouches[0].screenX;
        touchEndY = e.changedTouches[0].screenY;

        const deltaX = touchEndX - touchStartX;
        const deltaY = touchEndY - touchStartY;

        // Check if horizontal swipe is more significant than vertical
        if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > 50) {
          if (deltaX > 0) {
            // Swipe right - previous image
            if (typeof changeImage === "function") {
              changeImage(-1);
            }
          } else {
            // Swipe left - next image
            if (typeof changeImage === "function") {
              changeImage(1);
            }
          }
        }
      }
    },
    { passive: true },
  );

  // Prevent pinch zoom in lightbox
  lightbox.addEventListener(
    "touchmove",
    function (e) {
      if (e.touches.length > 1) {
        e.preventDefault();
      }
    },
    { passive: false },
  );
}

// Initialize everything when DOM is ready
document.addEventListener("DOMContentLoaded", function () {
  setupScrollListener();
  initializeMobileGallerySwipe();
  initializeDotNavigation();
  initializeQuickAmounts();
  initializeAddToCart();
  initializeTooltips();
  initializeLightboxSwipe();

  // Event delegation for gallery clicks
  document.addEventListener("click", function (e) {
    // Handle gallery image clicks
    if (e.target.classList.contains("gallery-clickable")) {
      const index = parseInt(e.target.getAttribute("data-lightbox-index"));
      if (!isNaN(index)) {
        openLightbox(index);
      }
    }

    // Handle dot clicks
    if (e.target.classList.contains("gallery-dot-clickable")) {
      const index = parseInt(e.target.getAttribute("data-slide-index"));
      if (!isNaN(index)) {
        goToSlide(index);
      }
    }
  });
});

// Play property video
function playPropertyVideo() {
  const youtubeModalOverlay = document.getElementById("youtube-modal-overlay");
  const youtubeIframe = document.getElementById("youtube-iframe");

  // Extract YouTube video ID from the desktop video thumbnail
  const videoThumbnail = document.querySelector('#video-play-button .video-thumbnail');
  let youtubeVideoId = '';
  if (videoThumbnail && videoThumbnail.src) {
    const match = videoThumbnail.src.match(/\/vi\/([^\/]+)\//);
    if (match) youtubeVideoId = match[1];
  }
  const youtubeEmbedUrl = youtubeVideoId ? `https://www.youtube.com/embed/${youtubeVideoId}?autoplay=1&rel=0` : '';

  if (youtubeModalOverlay && youtubeIframe) {
    // Set the iframe source
    youtubeIframe.src = youtubeEmbedUrl;

    // First make it visible
    youtubeModalOverlay.style.display = "flex";
    document.body.classList.add("youtube-modal-active");

    // Force a reflow to ensure display change is applied
    youtubeModalOverlay.offsetHeight;

    // Then add the active class for animated transition
    requestAnimationFrame(function () {
      youtubeModalOverlay.classList.add("active");
    });
  }
}

// Export functions for global use
window.scrollGallery = scrollGallery;
window.goToSlide = goToSlide;
window.playPropertyVideo = playPropertyVideo;
window.switchMobileFinancialTab = switchMobileFinancialTab;
