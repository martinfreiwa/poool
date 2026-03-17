// Marketplace Gallery Functionality

/**
 * Switch between marketplace status tabs (Available, Funded, Exited)
 */
function initializeStatusTabs() {
  const tabs = document.querySelectorAll(".status-tabs .status-tab");
  tabs.forEach((tab) => {
    tab.addEventListener("click", function () {
      // Remove active from all siblings
      tabs.forEach((t) => t.classList.remove("active"));
      // Add active to current
      this.classList.add("active");
    });
  });
}

/**
 * Navigate property images using arrow buttons
 * @param {HTMLElement} buttonElement - The clicked arrow button
 * @param {number} direction - Direction to navigate (-1 for prev, 1 for next)
 */
function navigatePropertyImage(buttonElement, direction) {
  const propertyId = buttonElement.dataset.propertyId;
  const propertyCard = document.getElementById(`property-card-${propertyId}`);
  if (!propertyCard) return;

  // Get all images and find current active
  const images = propertyCard.querySelectorAll(".property-image");
  const currentActiveImage = propertyCard.querySelector(
    ".property-image.active",
  );
  const currentIndex = currentActiveImage
    ? parseInt(currentActiveImage.id.split("-").pop())
    : 0;

  // Calculate new index with wrap-around
  let newIndex = currentIndex + direction;
  if (newIndex < 0) newIndex = images.length - 1;
  if (newIndex >= images.length) newIndex = 0;

  // Switch to new image using existing function
  const targetDot = propertyCard.querySelector(
    `#property-card-${propertyId}-dot-${newIndex}`,
  );
  if (targetDot) {
    switchPropertyImage(targetDot);
  }
}

/**
 * Switch property image when pagination dot is clicked
 * @param {HTMLElement} dotElement - The clicked dot element
 */
function switchPropertyImage(dotElement) {
  const propertyId = dotElement.dataset.propertyId;
  const newImageIndex = parseInt(dotElement.dataset.imageIndex);

  // Get the property card container
  const propertyCard = document.getElementById(`property-card-${propertyId}`);
  if (!propertyCard) return;

  // Get all images in this property's gallery
  const images = propertyCard.querySelectorAll(".property-image");
  const dots = propertyCard.querySelectorAll(".property-dot");

  // Find currently active image
  const currentActiveImage = propertyCard.querySelector(
    ".property-image.active",
  );
  const currentIndex = currentActiveImage
    ? parseInt(currentActiveImage.id.split("-").pop())
    : -1;

  // Don't do anything if clicking the same image
  if (currentIndex === newImageIndex) return;

  // Remove active class from all dots immediately
  dots.forEach((dot) => dot.classList.remove("active"));

  // Add active class to the new dot immediately
  const targetDot = propertyCard.querySelector(
    `#property-card-${propertyId}-dot-${newImageIndex}`,
  );
  if (targetDot) {
    targetDot.classList.add("active");
  }

  // Fade out current image
  if (currentActiveImage) {
    currentActiveImage.classList.remove("active");
  }

  // Fade in new image after a tiny delay to ensure smooth transition
  const targetImage = propertyCard.querySelector(
    `#property-card-${propertyId}-image-${newImageIndex}`,
  );
  if (targetImage) {
    // Use requestAnimationFrame for smoother transition
    requestAnimationFrame(() => {
      targetImage.classList.add("active");
    });
  }
}

// Optional: Add keyboard navigation support
document.addEventListener("keydown", function (event) {
  // Get the currently focused dot
  const focusedDot = document.activeElement;
  if (!focusedDot || !focusedDot.classList.contains("property-dot")) return;

  const propertyId = focusedDot.dataset.propertyId;
  const currentIndex = parseInt(focusedDot.dataset.imageIndex);

  let newIndex = currentIndex;

  // Left arrow or A key - previous image
  if (event.key === "ArrowLeft" || event.key === "a" || event.key === "A") {
    const propertyCard = document.getElementById(`property-card-${propertyId}`);
    const totalImages = propertyCard.querySelectorAll(".property-image").length;
    newIndex = currentIndex > 0 ? currentIndex - 1 : totalImages - 1;
    event.preventDefault();
  }

  // Right arrow or D key - next image
  if (event.key === "ArrowRight" || event.key === "d" || event.key === "D") {
    const propertyCard = document.getElementById(`property-card-${propertyId}`);
    const totalImages = propertyCard.querySelectorAll(".property-image").length;
    newIndex = currentIndex < totalImages - 1 ? currentIndex + 1 : 0;
    event.preventDefault();
  }

  // Switch to the new image if index changed
  if (newIndex !== currentIndex) {
    const newDot = document.getElementById(
      `property-card-${propertyId}-dot-${newIndex}`,
    );
    if (newDot) {
      switchPropertyImage(newDot);
      newDot.focus();
    }
  }
});

// Function to initialize property card dots and images
function initializePropertyDots() {
  // Initialize background images from data attributes
  const propertyImages = document.querySelectorAll(
    ".property-image[data-bg-image]",
  );
  propertyImages.forEach((img) => {
    const imageUrl = img.getAttribute("data-bg-image");
    if (imageUrl) {
      // Use style property directly instead of setAttribute to preserve dimensions
      img.style.backgroundImage = `url('${imageUrl}')`;
      img.style.backgroundSize = "cover";
      img.style.backgroundPosition = "center center";
      img.style.backgroundRepeat = "no-repeat";
    }
  });

  // Initialize dots for keyboard navigation
  const dots = document.querySelectorAll(".property-dot");
  dots.forEach((dot) => {
    dot.setAttribute("tabindex", "0");
    dot.setAttribute("role", "button");
    dot.setAttribute(
      "aria-label",
      `View image ${parseInt(dot.dataset.imageIndex) + 1}`,
    );
  });
}

// Add mobile swipe support for property images
function initializeMobileSwipe() {
  // Find all property galleries directly
  const galleries = document.querySelectorAll(".property-gallery");

  galleries.forEach((gallery) => {
    // Get the property ID from the gallery's ID
    const galleryId = gallery.id;
    const propertyId = galleryId
      ? galleryId.replace("property-card-", "").replace("-gallery", "")
      : null;

    if (!propertyId) {
      return;
    }

    let startX = 0;
    let startY = 0;
    let isDragging = false;
    const threshold = 50; // minimum swipe distance

    // Touch start
    gallery.addEventListener(
      "touchstart",
      function (e) {
        startX = e.touches[0].clientX;
        startY = e.touches[0].clientY;
        isDragging = true;
      },
      { passive: true },
    );

    // Touch move - prevent scrolling if swiping horizontally
    gallery.addEventListener(
      "touchmove",
      function (e) {
        if (!isDragging) return;

        const currentX = e.touches[0].clientX;
        const currentY = e.touches[0].clientY;
        const diffX = Math.abs(currentX - startX);
        const diffY = Math.abs(currentY - startY);

        // If horizontal swipe is stronger, prevent vertical scroll
        if (diffX > diffY) {
          e.preventDefault();
        }
      },
      { passive: false },
    );

    // Touch end - detect swipe direction
    gallery.addEventListener(
      "touchend",
      function (e) {
        if (!isDragging) return;
        isDragging = false;

        const endX = e.changedTouches[0].clientX;
        const diffX = startX - endX;

        // Check if swipe distance is enough
        if (Math.abs(diffX) < threshold) {
          return;
        }

        // Determine swipe direction and navigate
        const propertyCard = document.getElementById(
          `property-card-${propertyId}`,
        );

        if (diffX > 0) {
          // Swiped left - go to next image
          const nextArrow = propertyCard
            ? propertyCard.querySelector(".property-nav-arrow--next")
            : gallery.parentElement.querySelector(".property-nav-arrow--next");
          if (nextArrow) {
            navigatePropertyImage(nextArrow, 1);
          } else {
          }
        } else {
          // Swiped right - go to previous image
          const prevArrow = propertyCard
            ? propertyCard.querySelector(".property-nav-arrow--prev")
            : gallery.parentElement.querySelector(".property-nav-arrow--prev");
          if (prevArrow) {
            navigatePropertyImage(prevArrow, -1);
          } else {
          }
        }
      },
      { passive: true },
    );
  });
}

// Make dots focusable for keyboard navigation on initial load
document.addEventListener("DOMContentLoaded", function () {
  initializePropertyDots();
  initializeStatusTabs();

  // Delay swipe initialization to ensure elements are ready
  setTimeout(function () {
    initializeMobileSwipe();
  }, 100);
});

// Re-initialize dots after HTMX content swap
document.addEventListener("htmx:afterSwap", function (evt) {
  // Small delay to ensure DOM is fully ready
  setTimeout(function () {
    // Re-initialize property dots after content swap
    initializePropertyDots();
    // Re-initialize status tabs after content swap
    initializeStatusTabs();
    // Re-initialize mobile swipe after content swap
    initializeMobileSwipe();
  }, 10);
});

/**
 * Card image carousel — prev/next navigation for property cards.
 * Called from onclick handlers on marketplace and commodities-marketplace pages.
 * Works by finding the closest .property-image-container and cycling through
 * .property-image children. Gracefully handles single-image cards (no-op).
 */
function cardPrevImage(btn) {
  _cardNavigate(btn, -1);
}

function cardNextImage(btn) {
  _cardNavigate(btn, 1);
}

function _cardNavigate(btn, direction) {
  var container = btn.closest('.property-image-container');
  if (!container) return;

  var images = container.querySelectorAll('.property-image');
  if (images.length <= 1) return; // nothing to navigate

  var currentIndex = -1;
  images.forEach(function (img, i) {
    if (img.classList.contains('active')) currentIndex = i;
  });
  if (currentIndex === -1) currentIndex = 0;

  var newIndex = currentIndex + direction;
  if (newIndex < 0) newIndex = images.length - 1;
  if (newIndex >= images.length) newIndex = 0;

  // Switch active image
  images[currentIndex].classList.remove('active');
  images[newIndex].classList.add('active');

  // Update dots if present
  var dots = container.querySelectorAll('.property-dot');
  dots.forEach(function (dot, i) {
    dot.classList.toggle('active', i === newIndex);
  });
}
