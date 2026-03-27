// Property Price Card Sticky Functionality
document.addEventListener("DOMContentLoaded", function () {
  const priceCard = document.getElementById("property-price-card");
  const kycBannerElement = document.querySelector(".kyc-banner");
  // Look for similar properties first, fallback to main card bottom boundary
  const stopElement = document.querySelector(".similar-properties-wrapper") || document.getElementById("property-main-card");

  if (!priceCard || !stopElement) {
    return;
  }

  // Get initial position and dimensions
  const priceCardRect = priceCard.getBoundingClientRect();
  
  // Create a reliable measurement for the top boundary
  function getStickyTopOffset() {
    if (kycBannerElement) {
      return {
        height: kycBannerElement.getBoundingClientRect().height,
        bottom: kycBannerElement.getBoundingClientRect().bottom
      };
    }
    // Fallback if no KYC banner - this is a sidebar app, so there is no top navbar!
    // We just return 0 to stick it near the top of the viewport.
    return { height: 0, bottom: 0 };
  }

  const initialTopOffset = getStickyTopOffset();

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
  const DISTANCE_FROM_BANNER = 24; // 24px distance from KYC banner or header

  // Calculate the scroll position where sticky should START
  const stickyStartPoint =
    originalPosition.offsetTop - initialTopOffset.height - DISTANCE_FROM_BANNER;

  function updateStickyPosition() {
    const scrollY = window.scrollY;
    const currentTopOffset = getStickyTopOffset();
    const stopElementRect = stopElement.getBoundingClientRect();
    const priceCardHeight = priceCard.offsetHeight;

    // Calculate when to stop sticking
    let boundaryY;
    if (stopElement.classList.contains('similar-properties-wrapper')) {
      boundaryY = stopElementRect.top + scrollY;
    } else {
      boundaryY = stopElementRect.bottom + scrollY;
    }

    const maxScrollBeforeStop =
      boundaryY -
      priceCardHeight -
      currentTopOffset.height -
      DISTANCE_FROM_BANNER;

    // Determine if we should stick the card
    const shouldStick = scrollY >= stickyStartPoint;
    const shouldStopSticking = scrollY >= maxScrollBeforeStop;

    if (shouldStick && !shouldStopSticking) {
      // Make sticky: position fixed with calculated top
      priceCard.style.position = "fixed";
      const stickyTopPos = Math.max(currentTopOffset.bottom, 0) + DISTANCE_FROM_BANNER;
      priceCard.style.top = `${stickyTopPos}px`;
      priceCard.style.left = `${originalPosition.offsetLeft}px`;
      priceCard.style.zIndex = "999";
    } else if (shouldStopSticking) {
      // Stop at boundary: position absolute
      priceCard.style.position = "absolute";
      priceCard.style.top = `${boundaryY - priceCardHeight - DISTANCE_FROM_BANNER}px`;
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
