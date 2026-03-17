// HTMX Configuration and Event Handlers
(function () {
  "use strict";

  // Defer body-dependent listeners until the DOM is ready
  function initBodyListeners() {
    if (!document.body) {
      setTimeout(initBodyListeners, 50);
      return;
    }

    try {
      // Configure HTMX
      document.body.addEventListener("htmx:configRequest", function (evt) {
        // Add cache-busting headers
        evt.detail.headers["X-Requested-With"] = "XMLHttpRequest";
        evt.detail.headers["Cache-Control"] = "no-cache";
      });

      // ── Anti-flicker: hide main before swap starts ──
      document.body.addEventListener("htmx:beforeSwap", function (evt) {
        var target = evt.detail.target;
        if (target && (target.tagName === "MAIN" || target.id === "marketplace-main" || target.classList.contains("app-content"))) {
          target.style.opacity = "0";
          target.style.transition = "none";
        }
      });

      // Handle successful swaps
      document.body.addEventListener("htmx:afterSwap", function (evt) {
        var target = evt.detail.target;
        if (!target) return;

        // Only process main element swaps
        if (
          target.tagName === "MAIN" ||
          target.id === "marketplace-main" ||
          target.classList.contains("app-content")
        ) {
          var path = window.location.pathname;

          // Update navbar active state
          updateNavbarState(path);

          // Re-initialize page-specific JavaScript
          reinitializePageScripts(path);

          // ── Anti-flicker: fade in after swap ──
          requestAnimationFrame(function () {
            target.style.transition = "opacity 0.15s ease-in";
            target.style.opacity = "1";
          });
        }
      });

      // Handle HTMX load complete
      document.body.addEventListener("htmx:load", function (evt) {
        // Signal CSS readiness without layout thrashing
        if (evt.detail.elt && (evt.detail.elt.tagName === "MAIN" || evt.detail.elt.id === "marketplace-main")) {
          evt.detail.elt.classList.add("css-loaded");
        }
      });
    } catch (err) {
      console.error("HTMX-Init: Error attaching listeners:", err);
    }
  }

  // Update navbar active state
  function updateNavbarState(path) {
    try {
      // Remove synchronous style tag if it exists
      var dynamicStyle = document.getElementById("dynamic-sidebar-style");
      if (dynamicStyle) dynamicStyle.remove();

      // Remove all active classes
      document.querySelectorAll(".nav-item, .sidebar__nav-item").forEach(function (item) {
        item.classList.remove("active");
        item.classList.remove("sidebar__nav-item--active");
        item.classList.remove("nav-item--active");
      });

      // Determine active page
      var activePage = "";
      if (path === "/" || path.includes("/marketplace")) {
        activePage = "marketplace";
      } else if (path.includes("/wallet")) {
        activePage = "wallet";
      } else if (path.includes("/portfolio")) {
        activePage = "portfolio";
      } else if (path.includes("/cart") || path.includes("/checkout")) {
        activePage = "cart";
      } else if (path.includes("/rewards")) {
        activePage = "rewards";
      } else if (path.includes("/developer/dashboard")) {
        activePage = "dashboard";
      } else if (path.includes("/developer/assets")) {
        activePage = "assets";
      } else if (path.includes("/settings")) {
        activePage = "settings";
      } else if (path.includes("/support")) {
        activePage = "support";
      }

      // Set active class
      if (activePage) {
        var selectors = [
          "#nav-item-" + activePage,
          ".nav-item-" + activePage,
          "#sidebar-nav-item-" + activePage
        ];

        selectors.forEach(function (sel) {
          var item = document.querySelector(sel);
          if (item) {
            item.classList.add("active");
            item.classList.add("sidebar__nav-item--active");
          }
        });
      }
    } catch (err) {
      console.error("HTMX-Init: Error updating navbar:", err);
    }
  }

  // Re-initialize page-specific scripts
  function reinitializePageScripts(path) {
    try {
      // Marketplace specific
      if (path.includes("/marketplace")) {
        if (typeof initializePropertyDots === "function") {
          setTimeout(initializePropertyDots, 100);
        }
      }

      // Wallet specific
      if (path.includes("/wallet")) {
        if (window.initializeWallet) window.initializeWallet();
      }

      // Re-initialize Alpine.js components if present
      if (window.Alpine) {
        var main = document.querySelector("main") || document.getElementById("marketplace-main");
        if (main) {
          window.Alpine.initTree(main);
        }
      }
    } catch (err) {
      console.error("HTMX-Init: Error reinitializing scripts:", err);
    }
  }

  // Handle browser navigation
  window.addEventListener("popstate", function () {
    updateNavbarState(window.location.pathname);
  });

  // Initialize on DOM ready or immediate if ready
  function startInit() {
    initBodyListeners();
    updateNavbarState(window.location.pathname);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", startInit);
  } else {
    startInit();
  }
})();

