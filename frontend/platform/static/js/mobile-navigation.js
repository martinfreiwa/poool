// Mobile Navigation JavaScript
// Handles mobile header and burger menu interactions

// ─── XSS-safe HTML escaper ───────────────────────────────────
function escHtmlNav(str) {
  if (typeof str !== 'string') return String(str);
  var d = document.createElement('div');
  d.appendChild(document.createTextNode(str));
  return d.innerHTML;
}
// Toggle Mobile Menu
function toggleMobileMenu() {
  const burgerBtn = document.getElementById("mobile-burger-btn");
  const burgerMenu = document.getElementById("mobile-burger-menu");
  const overlay = document.getElementById("mobile-menu-overlay");
  const body = document.body;

  if (burgerMenu && overlay) {
    const isActive = burgerMenu.classList.contains("active");

    if (isActive) {
      closeMobileMenu();
    } else {
      // Open menu
      burgerMenu.classList.add("active");
      overlay.classList.add("active");
      burgerBtn?.classList.add("active");
      body.style.overflow = "hidden"; // Prevent body scroll when menu is open
    }
  }
}

// Close Mobile Menu
function closeMobileMenu() {
  const burgerBtn = document.getElementById("mobile-burger-btn");
  const burgerMenu = document.getElementById("mobile-burger-menu");
  const overlay = document.getElementById("mobile-menu-overlay");
  const body = document.body;

  if (burgerMenu) {
    burgerMenu.classList.remove("active");
  }
  if (overlay) {
    overlay.classList.remove("active");
  }
  if (burgerBtn) {
    burgerBtn.classList.remove("active");
  }
  body.style.overflow = ""; // Restore body scroll
}

// Handle Escape key to close menu
document.addEventListener("keydown", function (e) {
  if (e.key === "Escape") {
    closeMobileMenu();
  }
});

// Handle swipe gestures for mobile menu (only close, not open)
let touchStartX = 0;
let touchEndX = 0;

function handleSwipe() {
  const burgerMenu = document.getElementById("mobile-burger-menu");
  if (!burgerMenu) return;

  // Only allow swipe left to close (if menu is open) - remove swipe to open
  if (touchStartX - touchEndX > 50 && burgerMenu.classList.contains("active")) {
    closeMobileMenu();
  }
}

// Add touch listeners for swipe support (only for closing)
document.addEventListener(
  "touchstart",
  function (e) {
    touchStartX = e.changedTouches[0].screenX;
  },
  { passive: true },
);

document.addEventListener(
  "touchend",
  function (e) {
    touchEndX = e.changedTouches[0].screenX;
    handleSwipe();
  },
  { passive: true },
);

// Handle KYC banner action
function handleKYCAction() {
  // Navigate to KYC page or trigger KYC modal
  window.location.href = "/kyc";
}

// Update cart and notification badges
function updateMobileBadges() {
  // Get cart count from localStorage or API
  const cartCount = localStorage.getItem("cartCount") || 0;
  const notificationCount = localStorage.getItem("notificationCount") || 0;

  // Update cart badge
  const cartBadge = document.querySelector(
    "#mobile-cart-btn .mobile-header__badge",
  );
  if (cartBadge) {
    if (cartCount > 0) {
      cartBadge.textContent = cartCount;
      cartBadge.style.display = "flex";
    } else {
      cartBadge.style.display = "none";
    }
  }

  // Update notification badge
  const notificationBadge = document.querySelector(
    "#mobile-notification-btn .mobile-header__badge",
  );
  if (notificationBadge) {
    if (notificationCount > 0) {
      notificationBadge.textContent = notificationCount;
      notificationBadge.style.display = "flex";
    } else {
      notificationBadge.style.display = "none";
    }
  }
}

// Initialize mobile navigation
document.addEventListener("DOMContentLoaded", function () {
  // Update badges on load
  updateMobileBadges();

  // Debug: Check if HTMX is loaded
  if (typeof htmx !== "undefined") {
  }

  // Test what's blocking events
  setTimeout(function () {
    // Add a global click listener to see what's being clicked
    document.addEventListener("click", function (e) {}, true);

    document.addEventListener("touchstart", function (e) {}, true);

    // Check what element is on top of tabs
    const tabs = document.querySelectorAll(".status-tab");
    tabs.forEach((tab) => {
      const rect = tab.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      const elementAtPoint = document.elementFromPoint(centerX, centerY);

      // Force add a click handler using a different method
      tab.onclick = function () {
        const url = this.getAttribute("hx-get");
        const target = this.getAttribute("hx-target");
        if (url && target && typeof htmx !== "undefined") {
          // Update active state
          document
            .querySelectorAll(".status-tab")
            .forEach((t) => t.classList.remove("active"));
          this.classList.add("active");

          // Trigger HTMX request
          htmx.ajax("GET", url, target).then(function () {
            // Re-initialize property images after content swap
            setTimeout(function () {
              if (typeof initializePropertyDots === "function") {
                initializePropertyDots();
              }
              if (typeof initializeMobileSwipe === "function") {
                initializeMobileSwipe();
              }
            }, 100);
          });
        }
      };
    });

    // Do the same for galleries
    const galleries = document.querySelectorAll(".property-gallery");
    galleries.forEach((gallery, index) => {
      const rect = gallery.getBoundingClientRect();
    });
  }, 1000);

  // Close menu when clicking on nav links
  const navLinks = document.querySelectorAll(".mobile-burger-menu__nav-item");
  navLinks.forEach((link) => {
    if (!link.disabled) {
      link.addEventListener("click", function () {
        closeMobileMenu();
      });
    }
  });

  // Handle featured card close button
  const featuredCloseBtn = document.querySelector(
    ".mobile-burger-menu__featured-close",
  );
  if (featuredCloseBtn) {
    featuredCloseBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      const card = this.closest(".mobile-burger-menu__featured-card");
      if (card) {
        card.style.display = "none";
      }
    });
  }

  // Handle featured card dismiss button
  const dismissBtn = document.querySelector(
    ".mobile-burger-menu__featured-button--dismiss",
  );
  if (dismissBtn) {
    dismissBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      const card = this.closest(".mobile-burger-menu__featured-card");
      if (card) {
        card.style.display = "none";
      }
    });
  }

  // Prevent menu close when clicking inside the menu
  const burgerMenu = document.getElementById("mobile-burger-menu");
  if (burgerMenu) {
    burgerMenu.addEventListener("click", function (e) {
      e.stopPropagation();
    });
  }
});

// Mobile Profile Dropdown Functions
function toggleMobileProfileDropdown(event) {
  if (event) {
    event.stopPropagation();
  }

  const dropdown = document.getElementById("mobile-profile-dropdown-menu");
  const accountCard = document.getElementById("mobile-account-card");

  if (!dropdown) {
    return;
  }

  if (!dropdown.classList.contains("active")) {
    // Open dropdown
    dropdown.classList.add("active");
    if (accountCard) accountCard.classList.add("active");

    // Add click outside listener after a brief delay to prevent immediate closure
    setTimeout(() => {
      document.addEventListener(
        "click",
        closeMobileDropdownOnClickOutside,
        true,
      );
    }, 10);
  } else {
    closeMobileProfileDropdown();
  }
}

function closeMobileProfileDropdown() {
  const dropdown = document.getElementById("mobile-profile-dropdown-menu");
  const accountCard = document.getElementById("mobile-account-card");

  if (dropdown) {
    dropdown.classList.remove("active");
  }

  if (accountCard) {
    accountCard.classList.remove("active");
  }

  // Remove click outside listener
  document.removeEventListener(
    "click",
    closeMobileDropdownOnClickOutside,
    true,
  );
}

function closeMobileDropdownOnClickOutside(event) {
  const dropdown = document.getElementById("mobile-profile-dropdown-menu");
  const accountCard = document.getElementById("mobile-account-card");

  // Check if click is outside both the account card and dropdown
  if (
    accountCard &&
    !accountCard.contains(event.target) &&
    dropdown &&
    !dropdown.contains(event.target)
  ) {
    closeMobileProfileDropdown();
  }
}

// Handle mobile account switching
document.addEventListener("DOMContentLoaded", function () {
  // Use event delegation for mobile account items
  document.body.addEventListener("click", function (e) {
    // Check for mobile account item clicks
    const accountItem = e.target.closest(
      ".mobile-profile-menu-item.account-item",
    );

    if (accountItem) {
      e.stopPropagation();

      if (!accountItem.classList.contains("selected")) {
        // Get all mobile account items
        const allAccountItems = document.querySelectorAll(
          ".mobile-profile-menu-item.account-item",
        );

        // Remove selected from all items
        allAccountItems.forEach((accItem) => {
          accItem.classList.remove("selected");
          const checkbox = accItem.querySelector(".mobile-profile-checkbox");
          if (checkbox) {
            checkbox.classList.remove("selected");
          }
        });

        // Add selected to clicked item
        accountItem.classList.add("selected");
        const checkbox = accountItem.querySelector(".mobile-profile-checkbox");
        if (checkbox) {
          checkbox.classList.add("selected");
        }

        // Update the main account display in burger menu
        updateMobileAccountDisplay(accountItem);
      } else {
        // Already selected — but still navigate if we're not on the right page
        const itemId = accountItem.id;
        let profileType = "investor";
        if (itemId.includes("admin")) {
          profileType = "admin";
        } else if (itemId.includes("developer")) {
          profileType = "developer";
        }

        const currentPath = window.location.pathname;
        const needsRedirect =
          (profileType === "admin" && !currentPath.startsWith("/admin/")) ||
          (profileType === "developer" && !currentPath.startsWith("/developer/")) ||
          (profileType === "investor" && (currentPath.startsWith("/admin/") || currentPath.startsWith("/developer/")));

        if (needsRedirect) {
          closeMobileProfileDropdown();
          closeMobileMenu();
          if (typeof reloadWithProfileFlow === "function") {
            reloadWithProfileFlow(profileType);
          }
        }
      }
    }

    // Handle other mobile menu item clicks
    const menuItem = e.target.closest(
      ".mobile-profile-menu-item:not(.account-item)",
    );

    if (menuItem) {
      const itemId = menuItem.id;

      switch (itemId) {
        case "mobile-menu-item-documentation":
          window.open("/docs", "_blank");
          break;
        case "mobile-menu-item-sign-out":
          if (confirm("Are you sure you want to sign out?")) {
            window.location.href = "/logout";
          }
          break;
      }

      closeMobileProfileDropdown();
    }
  });
});

function updateMobileAccountDisplay(selectedItem) {
  // Get the selected account data
  const avatar = selectedItem.querySelector(".mobile-profile-avatar img");
  const name = selectedItem.querySelector(".mobile-profile-account-name");
  const type = selectedItem.querySelector(".mobile-profile-account-type");

  // Update burger menu account display
  const mobileAvatar = document.querySelector(
    ".mobile-burger-menu__avatar img",
  );
  const mobileName = document.querySelector(
    ".mobile-burger-menu__account-name",
  );
  const mobileEmail = document.querySelector(
    ".mobile-burger-menu__account-email",
  );

  if (avatar && mobileAvatar) {
    mobileAvatar.src = avatar.src;
    mobileAvatar.alt = avatar.alt;
  }

  if (name && mobileName) {
    mobileName.textContent = name.textContent;
  }

  if (type && mobileEmail) {
    mobileEmail.textContent = type.textContent;
  }

  // Determine profile type and save
  const accountId = selectedItem.id
    .replace("mobile-menu-item-account-", "")
    .replace("mobile-menu-item-current-account", "olivia-investor");
  let profileType = "investor";
  if (accountId.includes("admin")) {
    profileType = "admin";
  } else if (accountId.includes("developer")) {
    profileType = "developer";
  }

  localStorage.setItem("selectedProfile", profileType);
  localStorage.setItem("selectedAccountId", accountId);

  // Close dropdown and reload with appropriate flow
  closeMobileProfileDropdown();
  closeMobileMenu();

  setTimeout(() => {
    if (typeof reloadWithProfileFlow === "function") {
      reloadWithProfileFlow(profileType);
    } else {
      window.location.reload();
    }
  }, 100);
}

// Export functions for global use
window.toggleMobileMenu = toggleMobileMenu;
window.closeMobileMenu = closeMobileMenu;
window.updateMobileBadges = updateMobileBadges;
window.toggleMobileProfileDropdown = toggleMobileProfileDropdown;
window.closeMobileProfileDropdown = closeMobileProfileDropdown;
window.initGlobalSearch = initGlobalSearch;

// ==========================================
// Global Search Functionality
// ==========================================
document.addEventListener("DOMContentLoaded", function () {
  initGlobalSearch();
});

// Re-init after HTMX swaps
document.addEventListener("htmx:afterSwap", function () {
  setTimeout(initGlobalSearch, 100);
});

function initGlobalSearch() {
  const searchInputs = document.querySelectorAll('input[type="search"]');

  // The searchable index
  const searchIndex = [
    {
      title: "Properties Marketplace",
      type: "Page",
      url: "/marketplace",
      icon: "home-05.svg",
    },
    {
      title: "Commodities Marketplace",
      type: "Page",
      url: "/commodities-marketplace",
      icon: "home-05.svg",
    },
    { title: "Wallet", type: "Page", url: "/wallet", icon: "wallet-02.svg" },
    {
      title: "Portfolio",
      type: "Page",
      url: "/portfolio",
      icon: "line-chart-up-02.svg",
    },
    { title: "Cart", type: "Page", url: "/cart", icon: "shopping-cart-01.svg" },
    {
      title: "Settings",
      type: "Page",
      url: "/settings",
      icon: "settings-01.svg",
    },
    {
      title: "Support",
      type: "Page",
      url: "/support",
      icon: "message-chat-circle-grey.svg",
    },
    { title: "Rewards", type: "Page", url: "/rewards", icon: "star-01.svg" },
    {
      title: "Transactions",
      type: "Page",
      url: "/transactions",
      icon: "wallet-02.svg",
    },
    {
      title: "Developer Dashboard",
      type: "Developer",
      url: "/developer/dashboard",
      icon: "home-05.svg",
    },
    {
      title: "Developer Assets",
      type: "Developer",
      url: "/developer/assets",
      icon: "home-05.svg",
    },
    {
      title: "The Regent Hotel",
      type: "Property",
      url: "/marketplace",
      icon: "home-05.svg",
    },
    {
      title: "Villa Horizon",
      type: "Property",
      url: "/marketplace",
      icon: "home-05.svg",
    },
    {
      title: "Gold Bullion",
      type: "Commodity",
      url: "/commodities-marketplace",
      icon: "home-05.svg",
    },
    {
      title: "Urban Loft",
      type: "Property",
      url: "/marketplace",
      icon: "home-05.svg",
    },
  ];

  searchInputs.forEach((input) => {
    // Only initialize once
    if (input.dataset.searchInitialized) return;
    input.dataset.searchInitialized = "true";

    // Disable default autocomplete
    input.setAttribute("autocomplete", "off");

    // Create dropdown container – mounted on body to avoid overflow clipping
    const dropdown = document.createElement("div");
    dropdown.className = "search-dropdown-overlay";
    Object.assign(dropdown.style, {
      position: "fixed",
      zIndex: "99999",
      minWidth: "240px",
      backgroundColor: "#fff",
      border: "1px solid #E9EAEB",
      borderRadius: "8px",
      boxShadow:
        "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)",
      display: "none",
      maxHeight: "320px",
      overflowY: "auto",
    });
    document.body.appendChild(dropdown);

    // Position the dropdown directly below the input using fixed positioning
    function positionDropdown() {
      const rect = input.getBoundingClientRect();
      dropdown.style.top = rect.bottom + 4 + "px";
      dropdown.style.left = rect.left + "px";
      dropdown.style.width = Math.max(rect.width, 240) + "px";
    }

    function performSearch(query) {
      if (!query) {
        dropdown.style.display = "none";
        return;
      }

      const lowerQuery = query.toLowerCase();
      const results = searchIndex.filter(
        (item) =>
          item.title.toLowerCase().includes(lowerQuery) ||
          item.type.toLowerCase().includes(lowerQuery),
      );

      if (results.length === 0) {
        dropdown.innerHTML = `
                    <div style="padding: 12px 16px; color: #717680; font-size: 14px; text-align: center;">
                        No results found for "${escHtmlNav(query)}"
                    </div>
                `;
      } else {
        dropdown.innerHTML = results
          .map(
            (item) => `
                    <a href="${item.url}" class="search-result-item" style="display: flex; align-items: center; padding: 12px 16px; text-decoration: none; border-bottom: 1px solid #F2F4F7; transition: background-color 0.2s;">
                        <div style="width: 32px; height: 32px; border-radius: 6px; background: #F9FAFB; display: flex; align-items: center; justify-content: center; margin-right: 12px; flex-shrink: 0;">
                            <img src="/images/${item.icon}" onerror="this.src='/static/images/${item.icon}'" style="width: 16px; height: 16px; opacity: 0.7;">
                        </div>
                        <div>
                            <div style="font-size: 14px; font-weight: 500; color: #101828;">${item.title}</div>
                            <div style="font-size: 12px; color: #667085; margin-top: 2px;">${item.type}</div>
                        </div>
                    </a>
                `,
          )
          .join("");

        // Add hover effect
        dropdown.querySelectorAll(".search-result-item").forEach((el) => {
          el.addEventListener(
            "mouseenter",
            () => (el.style.backgroundColor = "#F9FAFB"),
          );
          el.addEventListener(
            "mouseleave",
            () => (el.style.backgroundColor = "transparent"),
          );
        });
      }

      positionDropdown();
      dropdown.style.display = "block";
    }

    // Input event
    input.addEventListener("input", (e) => {
      performSearch(e.target.value.trim());
    });

    // Focus event
    input.addEventListener("focus", (e) => {
      const val = e.target.value.trim();
      if (val) performSearch(val);
    });

    // Reposition on scroll/resize (since we use fixed positioning)
    let scrollContainers = [];
    let el = input.parentNode;
    while (el && el !== document) {
      const ov = window.getComputedStyle(el).overflowY;
      if (ov === "auto" || ov === "scroll") {
        scrollContainers.push(el);
      }
      el = el.parentNode;
    }
    scrollContainers.push(window);
    scrollContainers.forEach((container) => {
      container.addEventListener(
        "scroll",
        () => {
          if (dropdown.style.display !== "none") {
            positionDropdown();
          }
        },
        { passive: true },
      );
    });

    // Click outside to close
    document.addEventListener("click", (e) => {
      if (
        e.target !== input &&
        e.target !== dropdown &&
        !dropdown.contains(e.target)
      ) {
        dropdown.style.display = "none";
      }
    });

    // Prevent closing when clicking inside input or dropdown
    input.addEventListener("click", (e) => {
      e.stopPropagation();
    });
    dropdown.addEventListener("click", (e) => {
      e.stopPropagation();
    });
  });
}
