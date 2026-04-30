// Profile Dropdown JavaScript
//
// This file handles:
// 1. Toggle/close/click-outside for the profile dropdown (fallback; sidebar.html has its own inline version)
// 2. Account switching click delegation (investor / developer / admin)
// 3. Loading saved profile state
// 4. Redirecting to the correct dashboard on profile switch

function submitPooolLogout() {
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

window.submitPooolLogout = submitPooolLogout;

function toggleProfileDropdown() {
  const dropdown = document.getElementById("profile-dropdown-menu");
  const accountCard =
    document.getElementById("nav-account-card") ||
    document.querySelector(".sidebar__account") ||
    document.querySelector(".admin-sidebar-user");

  if (!dropdown || !accountCard) {
    return;
  }

  const isVisible = dropdown.style.display === "flex";

  if (isVisible) {
    closeProfileDropdown();
  } else {
    // Move to body to escape stacking context (Safari fix)
    if (dropdown.parentNode !== document.body) {
      document.body.appendChild(dropdown);
    }

    // Position relative to account card
    const rect = accountCard.getBoundingClientRect();

    dropdown.style.display = "flex";
    dropdown.style.position = "fixed";
    dropdown.style.bottom = window.innerHeight - rect.top + 10 + "px";
    dropdown.style.left = rect.left + "px";
    dropdown.style.zIndex = "99999";
    dropdown.style.width = rect.width + "px";
    dropdown.className = "profile-dropdown-menu";

    // Add active class to card
    accountCard.classList.add("active");

    // Close on click outside
    setTimeout(() => {
      document.addEventListener("click", closeDropdownOnClickOutside);
    }, 100);
  }
}

function closeProfileDropdown() {
  const dropdown = document.getElementById("profile-dropdown-menu");
  const button = document.getElementById("account-menu-button-inner");
  const accountCard = document.getElementById("nav-account-card");

  if (dropdown) {
    dropdown.style.display = "none";
  }

  if (button) button.classList.remove("active");
  if (accountCard) accountCard.classList.remove("active");

  // Remove click outside listener
  document.removeEventListener("click", closeDropdownOnClickOutside);
}

function closeDropdownOnClickOutside(event) {
  const dropdown = document.getElementById("profile-dropdown-menu");
  const accountCard = document.getElementById("nav-account-card");

  if (
    accountCard &&
    !accountCard.contains(event.target) &&
    dropdown &&
    !dropdown.contains(event.target)
  ) {
    closeProfileDropdown();
  }
}

// Determine profile type from an account item's id
function getProfileTypeFromItem(item) {
  const explicitProfile = item.getAttribute("data-profile");
  if (explicitProfile) return explicitProfile;

  const profileId = item.getAttribute("data-profile-id") || "";
  if (profileId.includes("admin")) return "admin";
  if (profileId.includes("developer")) return "developer";

  const itemId = item.id || "";
  if (itemId.includes("admin")) return "admin";
  if (itemId.includes("developer")) return "developer";
  return "investor";
}

function getProfileForCurrentPath() {
  const currentPath = window.location.pathname;
  if (currentPath.startsWith("/developer")) return "developer";
  if (currentPath.startsWith("/admin")) return "admin";
  return "investor";
}

function getDefaultAccountId(profileType) {
  if (profileType === "developer") return "olivia-developer";
  if (profileType === "admin") return "admin";
  return "olivia-investor";
}

// Check if user needs to be redirected (on wrong section for profile)
function needsRedirectForProfile(profileType) {
  const currentPath = window.location.pathname;
  if (profileType === "admin" && !currentPath.startsWith("/admin/")) return true;
  if (profileType === "developer" && !currentPath.startsWith("/developer/")) return true;
  if (profileType === "investor" && (currentPath.startsWith("/admin/") || currentPath.startsWith("/developer/"))) return true;
  return false;
}

// Navigate to the correct dashboard for the given profile type
function reloadWithProfileFlow(profileType) {
  if (profileType === "admin") {
    window.location.href = "/admin/";
  } else if (profileType === "developer") {
    window.location.href = "/developer/dashboard";
  } else {
    window.location.href = "/marketplace";
  }
}

// Handle account switching with event delegation — only initialize once
if (!window.__PROFILE_DROPDOWN_INITIALIZED) {
  window.__PROFILE_DROPDOWN_INITIALIZED = true;

  function initProfileDropdown() {
    loadSavedProfile();
    setupProfileEventHandlers();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initProfileDropdown);
  } else {
    initProfileDropdown();
  }
}

// Setup event handlers using delegation
function setupProfileEventHandlers() {
  // Guard: if sidebar.html already registered a handler, skip ours
  if (window.__SIDEBAR_ACCOUNT_HANDLER_REGISTERED) return;
  window.__SIDEBAR_ACCOUNT_HANDLER_REGISTERED = true;

  // Account card activation. This file is loaded before sidebar.html on many
  // pages, so it must register the toggle before setting the shared guard.
  document.body.addEventListener("click", function (e) {
    const card = e.target.closest("#nav-account-card");
    if (card && !e.target.closest("#profile-dropdown-menu")) {
      toggleProfileDropdown();
    }
  });

  document.body.addEventListener("keydown", function (e) {
    if (e.key !== "Enter" && e.key !== " ") return;

    const card = e.target.closest("#nav-account-card");
    if (card && !e.target.closest("#profile-dropdown-menu")) {
      e.preventDefault();
      toggleProfileDropdown();
    }
  });

  // Account item clicks
  document.body.addEventListener("click", function (e) {
    const accountItem = e.target.closest(".profile-menu-item.account-item");
    if (!accountItem) return;

    e.stopPropagation();

    const profileType = getProfileTypeFromItem(accountItem);
    const isSelected = accountItem.classList.contains("selected");
    const shouldRedirect = needsRedirectForProfile(profileType);

    if (!isSelected || shouldRedirect) {
      // Update visual selection
      document.querySelectorAll(".profile-menu-item.account-item").forEach((item) => {
        item.classList.remove("selected");
        const cb = item.querySelector(".profile-checkbox");
        if (cb) cb.classList.remove("selected");
      });

      accountItem.classList.add("selected");
      const checkbox = accountItem.querySelector(".profile-checkbox");
      if (checkbox) checkbox.classList.add("selected");

      // Save and redirect
      localStorage.setItem("selectedProfile", profileType);
      localStorage.setItem("selectedAccountId", getDefaultAccountId(profileType));
      closeProfileDropdown();

      setTimeout(() => {
        reloadWithProfileFlow(profileType);
      }, 50);
    } else {
      // Already selected & on correct page — just close
      closeProfileDropdown();
    }
  });

  // Non-account menu items (Documentation, Sign out, etc.)
  document.body.addEventListener("click", function (e) {
    const menuItem = e.target.closest(".profile-menu-item:not(.account-item)");
    if (!menuItem) return;

    const itemId = menuItem.id;

    switch (itemId) {
      case "menu-item-view-profile":
        window.location.href = "/profile";
        break;
      case "menu-item-account-settings":
        window.location.href = window.location.pathname.startsWith("/developer")
          ? "/developer/settings"
          : "/settings";
        break;
      case "menu-item-documentation":
        window.open("/docs", "_blank");
        break;
      case "menu-item-sign-out":
        submitPooolLogout();
        return;
    }

    closeProfileDropdown();
  });
}

// Load saved profile on page load
function loadSavedProfile() {
  const routeProfile = getProfileForCurrentPath();
  const storedProfile = localStorage.getItem("selectedProfile");
  const savedProfile = storedProfile === routeProfile ? storedProfile : routeProfile;
  let savedAccountId = localStorage.getItem("selectedAccountId");

  if (storedProfile !== routeProfile) {
    savedAccountId = getDefaultAccountId(routeProfile);
  }

  localStorage.setItem("selectedProfile", savedProfile);

  if (!savedAccountId && savedProfile) {
    savedAccountId = getDefaultAccountId(savedProfile);
  }
  localStorage.setItem("selectedAccountId", savedAccountId);

  if (savedProfile && savedAccountId) {
    const allAccountItems = document.querySelectorAll(
      ".profile-menu-item.account-item",
    );
    allAccountItems.forEach((item) => {
      const checkbox = item.querySelector(".profile-checkbox");
      if (getProfileTypeFromItem(item) === savedProfile) {
        item.classList.add("selected");
        if (checkbox) checkbox.classList.add("selected");
      } else {
        item.classList.remove("selected");
        if (checkbox) checkbox.classList.remove("selected");
      }
    });

    const selectedItem = Array.from(allAccountItems).find(
      (item) => getProfileTypeFromItem(item) === savedProfile,
    );

    if (selectedItem) {
      const avatar = selectedItem.querySelector(".profile-avatar img");
      const name = selectedItem.querySelector(".profile-account-name");
      const type = selectedItem.querySelector(".profile-account-type");

      const mainAvatar = document.querySelector("#account-avatar img");
      const mainName = document.querySelector("#account-name");
      const mainEmail = document.querySelector("#account-email");

      if (avatar && mainAvatar) {
        mainAvatar.src = avatar.src;
        mainAvatar.alt = avatar.alt;
      }
      if (name && mainName) {
        mainName.textContent = name.textContent;
      }
      if (type && mainEmail) {
        mainEmail.textContent = type.textContent;
      }
    }
  }
}

// Expose functions globally
window.toggleProfileDropdown = toggleProfileDropdown;
window.closeProfileDropdown = closeProfileDropdown;
window.closeDropdownOnClickOutside = closeDropdownOnClickOutside;
window.loadSavedProfile = loadSavedProfile;
window.reloadWithProfileFlow = reloadWithProfileFlow;
