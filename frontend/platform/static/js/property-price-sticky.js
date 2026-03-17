// Property Price Card Sticky Functionality
document.addEventListener("DOMContentLoaded", function () {
  const priceCard = document.getElementById("property-price-card");
  const kycBanner = document.querySelector(".kyc-banner");
  const similarPropertiesWrapper = document.querySelector(
    ".similar-properties-wrapper",
  );

  if (!priceCard || !kycBanner || !similarPropertiesWrapper) {
    return;
  }

  // Get initial position and dimensions
  const priceCardRect = priceCard.getBoundingClientRect();
  const kycBannerRect = kycBanner.getBoundingClientRect();

  // Store original position values and calculate absolute positions
  const originalPosition = {
    position: window.getComputedStyle(priceCard).position,
    top: window.getComputedStyle(priceCard).top,
    left: window.getComputedStyle(priceCard).left,
    zIndex: window.getComputedStyle(priceCard).zIndex,
    offsetTop: priceCardRect.top + window.scrollY, // Absolute top position
    offsetLeft: priceCardRect.left, // Absolute left position from viewport
  };

  // Configuration
  const DISTANCE_FROM_BANNER = 24; // 24px distance from KYC banner

  // Calculate the scroll position where sticky should START
  // This is when the price card would normally scroll above the KYC banner + distance
  const stickyStartPoint =
    originalPosition.offsetTop - kycBannerRect.height - DISTANCE_FROM_BANNER;

  function updateStickyPosition() {
    const scrollY = window.scrollY;
    const kycBannerRect = kycBanner.getBoundingClientRect();
    const similarPropertiesRect =
      similarPropertiesWrapper.getBoundingClientRect();
    const priceCardHeight = priceCard.offsetHeight;

    // Calculate when to stop sticking (before hitting similar properties)
    const similarPropertiesTop = similarPropertiesRect.top + scrollY;
    const maxScrollBeforeStop =
      similarPropertiesTop -
      priceCardHeight -
      kycBannerRect.height -
      DISTANCE_FROM_BANNER;

    // Determine if we should stick the card
    const shouldStick = scrollY >= stickyStartPoint;
    const shouldStopSticking = scrollY >= maxScrollBeforeStop;

    if (shouldStick && !shouldStopSticking) {
      // Make sticky: position fixed with calculated top
      priceCard.style.position = "fixed";
      priceCard.style.top = `${kycBannerRect.bottom + DISTANCE_FROM_BANNER}px`;
      priceCard.style.left = `${originalPosition.offsetLeft}px`;
      priceCard.style.zIndex = "999";
    } else if (shouldStopSticking) {
      // Stop at similar properties: position absolute
      priceCard.style.position = "absolute";
      priceCard.style.top = `${similarPropertiesTop - priceCardHeight - DISTANCE_FROM_BANNER}px`;
      priceCard.style.left = originalPosition.left;
      priceCard.style.zIndex = originalPosition.zIndex;
    } else {
      // Reset to original position
      priceCard.style.position = originalPosition.position;
      priceCard.style.top = originalPosition.top;
      priceCard.style.left = originalPosition.left;
      priceCard.style.zIndex = originalPosition.zIndex;
    }
  }

  // Throttle scroll events for better performance
  let ticking = false;
  function onScroll() {
    if (!ticking) {
      requestAnimationFrame(function () {
        updateStickyPosition();
        ticking = false;
      });
      ticking = true;
    }
  }

  // Attach scroll listener
  window.addEventListener("scroll", onScroll);

  // Initial check in case page loads scrolled
  updateStickyPosition();

  // Handle window resize
  window.addEventListener("resize", function () {
    // Recalculate on resize
    setTimeout(updateStickyPosition, 100);
  });
});
