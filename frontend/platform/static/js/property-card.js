(function () {
  function getCardFromElement(element) {
    return element ? element.closest(".property-card") : null;
  }

  function getContainerFromElement(element) {
    return element ? element.closest(".property-image-container") : null;
  }

  function setBackgroundImages(root) {
    root.querySelectorAll(".property-image[data-bg-image]").forEach(function (img) {
      var imageUrl = img.getAttribute("data-bg-image");
      if (!imageUrl) return;

      img.style.backgroundImage = "url('" + imageUrl.replace(/'/g, "\\'") + "')";
      img.style.backgroundSize = "cover";
      img.style.backgroundPosition = "center center";
      img.style.backgroundRepeat = "no-repeat";
    });
  }

  function setActiveImage(container, newIndex) {
    if (!container) return;

    var images = Array.from(container.querySelectorAll(".property-image"));
    if (!images.length || newIndex < 0 || newIndex >= images.length) return;

    images.forEach(function (img, index) {
      img.classList.toggle("active", index === newIndex);
    });

    container.querySelectorAll(".property-dot").forEach(function (dot, index) {
      dot.classList.toggle("active", index === newIndex);
      dot.setAttribute("aria-pressed", index === newIndex ? "true" : "false");
    });
  }

  function getActiveIndex(container) {
    var images = Array.from(container.querySelectorAll(".property-image"));
    var currentIndex = images.findIndex(function (img) {
      return img.classList.contains("active");
    });

    return currentIndex >= 0 ? currentIndex : 0;
  }

  function navigateFromElement(element, direction) {
    var container = getContainerFromElement(element);
    if (!container) return;

    var images = container.querySelectorAll(".property-image");
    if (images.length <= 1) return;

    var nextIndex = getActiveIndex(container) + direction;
    if (nextIndex < 0) nextIndex = images.length - 1;
    if (nextIndex >= images.length) nextIndex = 0;

    setActiveImage(container, nextIndex);
  }

  function initializeDots(root) {
    root.querySelectorAll(".property-image-container").forEach(function (container) {
      var dots = Array.from(container.querySelectorAll(".property-dot"));
      var activeIndex = getActiveIndex(container);

      dots.forEach(function (dot, index) {
        dot.setAttribute("tabindex", "0");
        dot.setAttribute("role", "button");
        dot.setAttribute("aria-label", "View image " + (index + 1));
        dot.setAttribute("aria-pressed", index === activeIndex ? "true" : "false");
        dot.dataset.imageIndex = dot.dataset.imageIndex || String(index);
      });
    });
  }

  function initializeSwipe(root) {
    root.querySelectorAll(".property-image-container").forEach(function (container) {
      if (container.dataset.propertyCardSwipeReady === "true") return;
      container.dataset.propertyCardSwipeReady = "true";

      var startX = 0;
      var startY = 0;
      var isDragging = false;
      var threshold = 50;

      container.addEventListener("touchstart", function (event) {
        if (!event.touches.length) return;
        startX = event.touches[0].clientX;
        startY = event.touches[0].clientY;
        isDragging = true;
      }, { passive: true });

      container.addEventListener("touchmove", function (event) {
        if (!isDragging || !event.touches.length) return;

        var diffX = Math.abs(event.touches[0].clientX - startX);
        var diffY = Math.abs(event.touches[0].clientY - startY);
        if (diffX > diffY) {
          event.preventDefault();
        }
      }, { passive: false });

      container.addEventListener("touchend", function (event) {
        if (!isDragging || !event.changedTouches.length) return;
        isDragging = false;

        var diffX = startX - event.changedTouches[0].clientX;
        if (Math.abs(diffX) < threshold) return;

        navigateFromElement(container, diffX > 0 ? 1 : -1);
      }, { passive: true });
    });
  }

  function initializePropertyCards(root) {
    var scope = root || document;
    setBackgroundImages(scope);
    initializeDots(scope);
    initializeSwipe(scope);
  }

  window.cardPrevImage = function (button) {
    navigateFromElement(button, -1);
  };

  window.cardNextImage = function (button) {
    navigateFromElement(button, 1);
  };

  window.initializePropertyCards = initializePropertyCards;

  document.addEventListener("click", function (event) {
    var dot = event.target.closest(".property-dot");
    if (!dot) return;

    var container = getContainerFromElement(dot);
    var index = parseInt(dot.dataset.imageIndex || "0", 10);
    if (!Number.isFinite(index)) return;

    event.stopPropagation();
    setActiveImage(container, index);
  });

  document.addEventListener("keydown", function (event) {
    var focusedDot = document.activeElement;
    if (!focusedDot || !focusedDot.classList.contains("property-dot")) return;

    var direction = 0;
    if (event.key === "ArrowLeft" || event.key === "a" || event.key === "A") {
      direction = -1;
    }
    if (event.key === "ArrowRight" || event.key === "d" || event.key === "D") {
      direction = 1;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      focusedDot.click();
      return;
    }
    if (!direction) return;

    event.preventDefault();
    navigateFromElement(focusedDot, direction);

    var container = getContainerFromElement(focusedDot);
    var activeDot = container ? container.querySelector(".property-dot.active") : null;
    if (activeDot) activeDot.focus();
  });

  document.addEventListener("DOMContentLoaded", function () {
    initializePropertyCards(document);
  });

  document.addEventListener("htmx:afterSwap", function (event) {
    initializePropertyCards(event.detail && event.detail.target ? event.detail.target : document);
  });
})();
