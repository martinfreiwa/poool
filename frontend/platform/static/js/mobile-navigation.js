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
let mobileNavTouchStartX = 0;
let mobileNavTouchEndX = 0;

function handleSwipe() {
  const burgerMenu = document.getElementById("mobile-burger-menu");
  if (!burgerMenu) return;

  // Only allow swipe left to close (if menu is open) - remove swipe to open
  if (mobileNavTouchStartX - mobileNavTouchEndX > 50 && burgerMenu.classList.contains("active")) {
    closeMobileMenu();
  }
}

// Add touch listeners for swipe support (only for closing)
document.addEventListener(
  "touchstart",
  function (e) {
    mobileNavTouchStartX = e.changedTouches[0].screenX;
  },
  { passive: true },
);

document.addEventListener(
  "touchend",
  function (e) {
    mobileNavTouchEndX = e.changedTouches[0].screenX;
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
  function submitLogout() {
    if (typeof window.submitPooolLogout === "function") {
      window.submitPooolLogout();
      return;
    }

    const form = document.createElement("form");
    form.method = "POST";
    form.action = "/auth/logout";
    form.style.display = "none";

    const token = typeof window.getCsrfToken === "function" ? window.getCsrfToken() : "";
    if (!token) {
      window.location.href = "/logout";
      return;
    }

    if (token) {
      const input = document.createElement("input");
      input.type = "hidden";
      input.name = "csrf_token";
      input.value = token;
      form.appendChild(input);
    }

    document.body.appendChild(form);
    form.submit();
  }

  // Update badges on load
  updateMobileBadges();

  // Debug: Check if HTMX is loaded
  if (typeof htmx !== "undefined") {
  }



  // Close menu when clicking on nav links
  const navLinks = document.querySelectorAll(
    ".mobile-burger-menu__nav-item[href], .mobile-burger-menu__nav-child[href]",
  );
  navLinks.forEach((link) => {
    link.addEventListener("click", function () {
      closeMobileMenu();
    });
  });

  // NOTE: featured-card close / dismiss buttons are handled via event
  // delegation in sidebar-community.js (works even for template-injected menus).

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
  if (window.__MOBILE_ACCOUNT_HANDLER_REGISTERED) return;
  window.__MOBILE_ACCOUNT_HANDLER_REGISTERED = true;

  // Use event delegation for mobile account items
  document.body.addEventListener("click", async function (e) {
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
          submitLogout();
          return;
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

let __globalSearchIndex = [];

function initGlobalSearch() {
  const searchInputs = Array.from(document.querySelectorAll('input[type="search"]'))
    .filter(input => input.id !== 'sidebar-search-input');
  const isDeveloperPage = window.location.pathname.startsWith('/developer');

  // 1. Build the index from static pages and dynamic assets on current page
  const staticPages = [
    { title: "Properties Marketplace", type: "Page", url: "/marketplace", icon: "home-05.svg" },
    { title: "Commodities Marketplace", type: "Page", url: "/commodities-marketplace", icon: "home-05.svg" },
    { title: "Resale Market", type: "Page", url: "/marketplace-secondary", icon: "refresh-cw-01.svg" },
    { title: "Wallet", type: "Page", url: "/wallet", icon: "wallet-02.svg" },
    { title: "Portfolio", type: "Page", url: "/portfolio", icon: "line-chart-up-02.svg" },
    { title: "Cart", type: "Page", url: "/cart", icon: "shopping-cart-01.svg" },
    { title: "Settings", type: "Page", url: isDeveloperPage ? "/developer/settings" : "/settings", icon: "settings-01.svg" },
    { title: "Support", type: "Page", url: isDeveloperPage ? "/developer/support" : "/support", icon: "message-chat-circle-grey.svg" },
    { title: "Rewards", type: "Page", url: "/rewards", icon: "star-01.svg" },
    { title: "Leaderboard", type: "Page", url: "/leaderboard", icon: "award-05.svg" },
    { title: "Community", type: "Page", url: "/community", icon: "users-01.svg" },
  ];

  const dynamicAssets = Array.from(document.querySelectorAll(".property-card")).map(card => {
    const titleEl = card.querySelector(".property-title");
    const imgEl = card.querySelector(".property-image");
    const type = card.dataset.assetType || "Property";
    const slug = card.dataset.propertyId;
    const baseUrl = type.toLowerCase() === 'commodity' ? '/commodity/' : '/property/';

    let imgSrc = "/static/images/icons/home-05.svg";
    if (imgEl) {
      if (imgEl.tagName === 'IMG' && imgEl.src) imgSrc = imgEl.src;
      else if (imgEl.style.backgroundImage) {
        const urlMatch = imgEl.style.backgroundImage.match(/url\(['"]?(.*?)['"]?\)/);
        if (urlMatch && urlMatch[1]) imgSrc = urlMatch[1];
      }
    }

    return {
      title: titleEl ? titleEl.textContent.trim() : "Unknown Asset",
      type: type.charAt(0).toUpperCase() + type.slice(1),
      url: slug ? (baseUrl + slug) : "#",
      image: imgSrc,
      isDynamic: true
    };
  });

  __globalSearchIndex = [...staticPages, ...dynamicAssets];

  // 2. Initialize inputs
  searchInputs.forEach((input) => {
    if (input.dataset.searchInitialized) return;
    input.dataset.searchInitialized = "true";
    input.setAttribute("autocomplete", "off");

    const dropdown = document.createElement("div");
    dropdown.className = "search-dropdown-overlay";
    Object.assign(dropdown.style, {
      position: "fixed",
      zIndex: "99999",
      minWidth: "240px",
      backgroundColor: "#fff",
      border: "1px solid #E9EAEB",
      borderRadius: "8px",
      boxShadow: "0 12px 24px -4px rgba(16, 24, 40, 0.08), 0 4px 6px -2px rgba(16, 24, 40, 0.03)",
      display: "none",
      maxHeight: "360px",
      overflowY: "auto",
    });
    document.body.appendChild(dropdown);

    function positionDropdown() {
      const rect = input.getBoundingClientRect();
      dropdown.style.top = rect.bottom + 4 + "px";
      dropdown.style.left = rect.left + "px";
      dropdown.style.width = Math.max(rect.width, 300) + "px";
    }

    function performSearch(query) {
      if (!query || query.length < 1) {
        dropdown.style.display = "none";
        return;
      }
      const lowerQuery = query.toLowerCase();
      
      const results = __globalSearchIndex.filter((item) => {
        const itemTitle = item.title.toLowerCase();
        const itemType = item.type.toLowerCase();
        if (itemType === lowerQuery) return true;
        if (itemType === 'page') {
            return (lowerQuery.length < 4) ? itemTitle.startsWith(lowerQuery) : itemTitle.includes(lowerQuery);
        } else {
            return itemTitle.includes(lowerQuery);
        }
      });

      if (results.length === 0) {
        dropdown.innerHTML = `<div style="padding: 16px; color: #717680; font-size: 14px; text-align: center;">No results found for "${escHtmlNav(query)}"</div>`;
      } else {
        dropdown.innerHTML = results.slice(0, 10).map(item => `
          <a href="${item.url}" class="search-result-item" style="display: flex; align-items: center; padding: 10px 16px; text-decoration: none; border-bottom: 1px solid #F2F4F7; transition: background 0.15s;">
            <div style="width: 36px; height: 36px; border-radius: 6px; overflow: hidden; background: #F9FAFB; display: flex; align-items: center; justify-content: center; margin-right: 12px; flex-shrink: 0; border: 1px solid #F2F4F7;">
              ${item.isDynamic 
                ? `<img src="${item.image}" style="width: 100%; height: 100%; object-fit: cover;">`
                : `<img src="/static/images/${item.icon}" onerror="this.src='/static/images/icons/home-05.svg'" style="width: 18px; height: 18px; opacity: 0.7;">`
              }
            </div>
            <div style="flex:1; min-width:0;">
              <div style="font-size: 14px; font-weight: 500; color: #101828; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${escHtmlNav(item.title)}</div>
              <div style="font-size: 11px; color: #667085; text-transform: uppercase; letter-spacing: 0.02em; margin-top: 1px;">${escHtmlNav(item.type)}</div>
            </div>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="opacity: 0.4;"><path d="M6 12L10 8L6 4" stroke="#717680" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </a>
        `).join("");

        dropdown.querySelectorAll(".search-result-item").forEach((el) => {
          el.addEventListener("mouseenter", () => (el.style.backgroundColor = "#F9FAFB"));
          el.addEventListener("mouseleave", () => (el.style.backgroundColor = "transparent"));
        });
      }
      positionDropdown();
      dropdown.style.display = "block";
    }

    input.addEventListener("input", (e) => performSearch(e.target.value.trim()));
    input.addEventListener("focus", (e) => {
      const val = e.target.value.trim();
      if (val) performSearch(val);
    });
    document.addEventListener("click", (e) => {
      if (e.target !== input && !dropdown.contains(e.target)) dropdown.style.display = "none";
    });
    window.addEventListener("resize", () => {
      if (dropdown.style.display !== "none") positionDropdown();
    });
  });
}
