/**
 * POOOL – Dynamic User Data Injection
 *
 * Fetches the current user's profile from /api/me and replaces
 * all hardcoded placeholder names ("Olivia Rhye") with real data.
 *
 * Loaded on every authenticated page via <script src="/static/js/user-data.js">
 */

/**
 * getCsrfToken – reads the csrf_token cookie.
 * Defined here (in user-data.js) so it is available on EVERY authenticated
 * page without needing a separate csrf.js include in each HTML file.
 * @returns {string} The CSRF token or an empty string.
 */
if (typeof window.getCsrfToken === "undefined") {
  window.getCsrfToken = function getCsrfToken() {
    var value = "; " + document.cookie;
    var parts = value.split("; csrf_token=");
    if (parts.length === 2) return parts.pop().split(";").shift();
    return "";
  };
  // Legacy alias used in some older scripts
  window.csrfToken = window.getCsrfToken;
}

/**
 * Auto-load poool-confirm.js — custom confirmation modal.
 * Replaces all native window.confirm() usage across the platform.
 */
(function () {
  if (typeof window.pooolConfirm === "undefined" &&
      !document.querySelector('script[src*="poool-confirm"]')) {
    var s = document.createElement("script");
    s.src = "/static/js/poool-confirm.js";
    document.head.appendChild(s);
  }
})();
(function () {
  "use strict";

  /** Escape HTML special characters to prevent XSS in innerHTML */
  function escHtml(str) {
    if (!str) return "";
    return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#x27;');
  }

  function getRouteProfile() {
    const path = window.location.pathname;
    if (path.startsWith("/developer")) return "developer";
    if (path.startsWith("/admin")) return "admin";
    return "investor";
  }

  function getDefaultAccountId(profile) {
    if (profile === "developer") return "olivia-developer";
    if (profile === "admin") return "admin";
    return "olivia-investor";
  }

  function normalizeProfileStateForRoute() {
    const routeProfile = getRouteProfile();
    const savedProfile = localStorage.getItem("selectedProfile");

    if (savedProfile !== routeProfile) {
      localStorage.setItem("selectedProfile", routeProfile);
      localStorage.setItem("selectedAccountId", getDefaultAccountId(routeProfile));
      return routeProfile;
    }

    if (!localStorage.getItem("selectedAccountId")) {
      localStorage.setItem("selectedAccountId", getDefaultAccountId(routeProfile));
    }

    return routeProfile;
  }

  // Fetch current user profile from the backend
  const savedProfile = normalizeProfileStateForRoute();

  // Public pages (e.g. /p/:slug) don't require auth — skip the login redirect.
  var isPublicPage = window.location.pathname.startsWith("/p/");

  fetch("/api/me", { credentials: "same-origin" })
    .then(function (res) {
      if (!res.ok) {
        if (res.status === 401 && !isPublicPage) {
          window.location.href = "/auth/login";
        }
        return null;
      }
      return res.json();
    })
    .then(function (user) {
      if (!user) return;

      // Store user data for other scripts that may need it
      window.__POOOL_USER = user;
      // Notify late subscribers (community kebab menu, etc.) that the user
      // payload is now available without forcing them to poll.
      try {
        window.dispatchEvent(new CustomEvent('poool:user-ready', { detail: user }));
      } catch (_) { /* no CustomEvent — older Edge etc., harmless */ }

      // ── Enrich Sentry with user context ────────────────────
      if (typeof Sentry !== 'undefined' && Sentry.setUser) {
        Sentry.setUser({
          id: user.id,
          email: user.email,
          username: user.name,
        });
      }

      // ── Replace all user name elements ──────────────────────
      var nameSelectors = [
        ".sidebar__account-name",
        ".mobile-burger-menu__account-name",
        ".profile-account-name",
        ".mobile-profile-account-name",
        ".admin-sidebar-user-name",
      ];

      nameSelectors.forEach(function (sel) {
        document.querySelectorAll(sel).forEach(function (el) {
          el.textContent = user.name;
        });
      });

      // ── Replace all user email elements ─────────────────────
      var emailSelectors = [
        ".sidebar__account-email",
        ".mobile-burger-menu__account-email",
      ];

      emailSelectors.forEach(function (sel) {
        document.querySelectorAll(sel).forEach(function (el) {
          el.textContent = user.email;
        });
      });

      // ── Replace all user role elements (Admin) ──────────────
      document
        .querySelectorAll(".admin-sidebar-user-role")
        .forEach(function (el) {
          el.textContent = user.role
            .replace("_", " ")
            .replace(/\b\w/g, (l) => l.toUpperCase());
        });

      // ── Replace avatar alt text ─────────────────────────────
      document
        .querySelectorAll(
          ".sidebar__account-avatar img, " +
          ".mobile-burger-menu__avatar img, " +
          ".profile-avatar img, " +
          ".mobile-profile-avatar img, " +
          ".admin-sidebar-avatar",
        )
        .forEach(function (img) {
          img.alt = user.name;
        });

      // ── Generate initials avatar (replace ALL placeholder images) ──
      var initialsDataUrl = generateInitialsAvatar(user.initials);
      var avatarSrc = user.avatar_url || initialsDataUrl;
      var PLACEHOLDERS = ["Image.webp", "Featured%20icon", "Featured icon"];

      function isPlaceholder(src) {
        return PLACEHOLDERS.some(function (p) { return src.includes(p); });
      }

      function replaceAvatars(selector) {
        document.querySelectorAll(selector).forEach(function (img) {
          if (isPlaceholder(img.src)) {
            img.src = avatarSrc;
            img.alt = user.name;
          }
        });
      }

      // Main sidebar + mobile avatars — always replace (placeholder is an inline SVG, not a URL)
      function paintAvatarSlots() {
        document.querySelectorAll(".sidebar__account-avatar img, .mobile-burger-menu__avatar img, .admin-sidebar-avatar, #my-profile-avatar-img").forEach(function(img) {
          img.src = avatarSrc;
          img.alt = user.name;
        });
      }
      paintAvatarSlots();
      // Re-run after HTMX swaps — partials like community_feed inject
      // #my-profile-avatar-img AFTER this script first runs.
      document.body.addEventListener("htmx:afterSwap", paintAvatarSlots);
      // Profile switcher rows (Investor / Developer / Admin) — always replace.
      // The static markup in sidebar.html uses a `data:image/svg+xml,...` blue-circle
      // placeholder that does NOT match the isPlaceholder() keyword list, so
      // replaceAvatars() would skip every row. Same user → same avatar across all
      // rows; force the swap so initials/uploaded photo show in every row.
      document.querySelectorAll(
        ".profile-switcher .profile-avatar img, " +
        "#profile-switch-section .profile-avatar img, " +
        ".profile-menu-item .profile-avatar img, " +
        ".mobile-profile-menu-item .mobile-profile-avatar img"
      ).forEach(function (img) {
        img.src = avatarSrc;
        img.alt = user.name;
      });

      // ── Update data attributes for profile switcher ─────────
      document
        .querySelectorAll('[data-profile-id="olivia-investor"]')
        .forEach(function (el) {
          el.setAttribute("data-profile-id", user.email + "-investor");
        });
      document
        .querySelectorAll('[data-profile-id="olivia-developer"]')
        .forEach(function (el) {
          el.setAttribute("data-profile-id", user.email + "-developer");
        });

      // ── Update selected state in switcher ───────────────────
      // (savedProfile is already initialized at the top of the IIFE)

      // ── Admin Profile Injection ──────────────────────────────
      if (user.role === "admin" || user.role === "super_admin") {
        // 1. Desktop Injection
        var desktopSwitchSection = document.getElementById(
          "profile-switch-section",
        );
        if (
          desktopSwitchSection &&
          !document.getElementById("menu-item-account-admin")
        ) {
          var adminItem = document.createElement("div");
          adminItem.id = "menu-item-account-admin";
          adminItem.className = "profile-menu-item account-item";
          adminItem.setAttribute("data-profile-id", user.email + "-admin");
          adminItem.style.cursor = "pointer";

          adminItem.innerHTML = `
                        <div class="profile-account-content">
                            <div class="profile-avatar-group">
                                <div class="profile-avatar">
                                    <img src="${avatarSrc}" alt="${escHtml(user.name)}">
                                    <div class="profile-avatar-border"></div>
                                    <div class="profile-online-indicator"></div>
                                </div>
                                <div class="profile-text-wrapper">
                                    <span class="profile-account-name">${escHtml(user.name)}</span>
                                    <span class="profile-account-type">Admin Profile</span>
                                </div>
                            </div>
                            <div class="profile-checkbox">
                                <div class="profile-checkbox-check"></div>
                            </div>
                        </div>
                    `;
          desktopSwitchSection.appendChild(adminItem);
        }

        // 2. Mobile Injection
        var mobileSwitchSection = document.getElementById(
          "mobile-profile-switch-section",
        );
        if (
          mobileSwitchSection &&
          !document.getElementById("mobile-menu-item-account-admin")
        ) {
          var mobileAdminItem = document.createElement("div");
          mobileAdminItem.id = "mobile-menu-item-account-admin";
          mobileAdminItem.className =
            "mobile-profile-menu-item account-item mobile-profile-switcher";
          mobileAdminItem.setAttribute(
            "data-profile-id",
            user.email + "-admin",
          );
          mobileAdminItem.setAttribute("data-account-type", "Admin Profile");
          mobileAdminItem.style.cursor = "pointer";

          mobileAdminItem.innerHTML = `
                        <div class="mobile-profile-account-content">
                            <div class="mobile-profile-avatar-group">
                                <div class="mobile-profile-avatar">
                                    <img src="${avatarSrc}" alt="${escHtml(user.name)}">
                                    <div class="mobile-profile-avatar-border"></div>
                                    <div class="mobile-profile-online-indicator"></div>
                                </div>
                                <div class="mobile-profile-text-wrapper">
                                    <span class="mobile-profile-account-name">${escHtml(user.name)}</span>
                                    <span class="mobile-profile-account-type">Admin Profile</span>
                                </div>
                            </div>
                            <div class="mobile-profile-checkbox">
                                <div class="mobile-profile-checkbox-check"></div>
                            </div>
                        </div>
                    `;
          mobileSwitchSection.appendChild(mobileAdminItem);

          // Re-initialize mobile switchers if needed
          if (typeof window.initializeProfileSwitchers === "function") {
            window.initializeProfileSwitchers();
          }
        }
        // 3. Admin Dashboard Injection (if we are in /admin/)
        if (window.location.pathname.startsWith("/admin/")) {
          setupAdminDashboardSwitcher(user, avatarSrc);
        }
      }

      if (savedProfile) {
        const selector =
          savedProfile === "admin"
            ? '[id*="account-admin"]'
            : savedProfile === "developer"
              ? '[id*="developer"]'
              : '[id*="current-account"]';

        document.querySelectorAll(".account-item").forEach((item) => {
          if (item.matches(selector)) {
            item.classList.add("selected");
            const checkbox = item.querySelector(
              ".profile-checkbox, .mobile-profile-checkbox",
            );
            if (checkbox) checkbox.classList.add("selected");
          } else {
            item.classList.remove("selected");
            const checkbox = item.querySelector(
              ".profile-checkbox, .mobile-profile-checkbox",
            );
            if (checkbox) checkbox.classList.remove("selected");
          }
        });
      }

      // ── Fill Settings Form if present ─────────────────────────
      if (window.location.pathname.includes("/settings")) {
        const firstNameInput = document.getElementById("settings-first-name");
        if (firstNameInput && user.first_name)
          firstNameInput.value = user.first_name;

        const lastNameInput = document.getElementById("settings-last-name");
        if (lastNameInput && user.last_name)
          lastNameInput.value = user.last_name;

        const emailInput = document.getElementById("settings-email");
        if (emailInput && user.email) emailInput.value = user.email;

        const phoneInput = document.getElementById("settings-phone");
        if (phoneInput && user.phone_number)
          phoneInput.value = user.phone_number;

        const countrySelect = document.getElementById("settings-country");
        if (countrySelect && user.country) {
          countrySelect.value = user.country.toUpperCase();
          // trigger change to update flag
          const event = new Event("change");
          countrySelect.dispatchEvent(event);
        }

        const roleSelect = document.getElementById("settings-role");
        if (roleSelect && user.role) roleSelect.value = user.role;
      }
    })
    .catch(function (err) {
      if (typeof Sentry !== 'undefined' && Sentry.captureException) {
        Sentry.captureException(err);
      }
    });

  /**
   * Setup the profile switcher for the Admin Dashboard sidebar.
   */
  function setupAdminDashboardSwitcher(user, initialsDataUrl) {
    var sidebarFooter = document.querySelector(".admin-sidebar-footer");
    var sidebarUser = document.querySelector(".admin-sidebar-user");

    if (
      sidebarFooter &&
      sidebarUser &&
      !document.getElementById("nav-account-card")
    ) {
      // 1. Make the user card clickable
      sidebarUser.id = "nav-account-card";
      sidebarUser.style.cursor = "pointer";
      sidebarUser.style.position = "relative";
      sidebarUser.addEventListener("click", function () {
        if (window.toggleProfileDropdown) window.toggleProfileDropdown();
      });

      // 2. Ensure CSS is loaded
      if (!document.querySelector('link[href*="profile-dropdown.css"]')) {
        var link = document.createElement("link");
        link.rel = "stylesheet";
        link.href = "/static/css/profile-dropdown.css";
        document.head.appendChild(link);
      }

      // 3. Ensure JS is loaded
      if (
        typeof window.toggleProfileDropdown === "undefined" &&
        !document.querySelector('script[src*="profile-dropdown.js"]')
      ) {
        var script = document.createElement("script");
        script.src = "/static/js/profile-dropdown.js";
        document.body.appendChild(script);
      }

      // 4. Inject the dropdown markup
      if (!document.getElementById("profile-dropdown-menu")) {
        var dropdown = document.createElement("div");
        dropdown.id = "profile-dropdown-menu";
        dropdown.className = "profile-dropdown-menu";
        dropdown.style.display = "none";
        dropdown.style.left = "260px"; // Admin sidebar width

        dropdown.innerHTML = `
                    <div id="profile-menu-wrapper" class="profile-menu-wrapper">
                        <div id="profile-menu-items-top" class="profile-menu-items">
                            <div id="menu-item-documentation" class="profile-menu-item">
                                <div class="profile-menu-content">
                                    <div class="profile-icon-text">
                                        <svg class="profile-menu-icon" width="20" height="20" viewBox="0 0 20 20" fill="none">
                                            <path d="M4 4V16H16V4H4Z" stroke="#717680" stroke-width="1.67"></path>
                                            <path d="M8 8H12M8 12H12" stroke="#717680" stroke-width="1.67" stroke-linecap="round"></path>
                                        </svg> 
                                        <span class="profile-menu-text">Documentation</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div id="profile-switch-section" class="profile-switch-section">
                            <div class="profile-switch-header"><span class="profile-switch-title">Switch account</span></div>
                            
                            <div id="menu-item-current-account" class="profile-menu-item account-item ${!savedProfile || savedProfile === "investor" ? "selected" : ""}">
                                <div class="profile-account-content">
                                    <div class="profile-avatar-group">
                                        <div class="profile-avatar"><img src="${initialsDataUrl}" alt="${escHtml(user.name)}">
                                            <div class="profile-avatar-border"></div>
                                            <div class="profile-online-indicator"></div>
                                        </div>
                                        <div class="profile-text-wrapper"><span class="profile-account-name">${escHtml(user.name)}</span> <span class="profile-account-type">Investor Profile</span></div>
                                    </div>
                                    <div class="profile-checkbox ${!savedProfile || savedProfile === "investor" ? "selected" : ""}">
                                        <div class="profile-checkbox-check"></div>
                                    </div>
                                </div>
                            </div>

                            <div id="menu-item-account-developer" class="profile-menu-item account-item ${savedProfile === "developer" ? "selected" : ""}" data-profile-id="${escHtml(user.email)}-developer">
                                <div class="profile-account-content">
                                    <div class="profile-avatar-group">
                                        <div class="profile-avatar"><img src="${initialsDataUrl}" alt="${escHtml(user.name)}">
                                            <div class="profile-avatar-border"></div>
                                            <div class="profile-online-indicator"></div>
                                        </div>
                                        <div class="profile-text-wrapper"><span class="profile-account-name">${escHtml(user.name)}</span> <span class="profile-account-type">Developer Profile</span></div>
                                    </div>
                                    <div class="profile-checkbox ${savedProfile === "developer" ? "selected" : ""}">
                                        <div class="profile-checkbox-check"></div>
                                    </div>
                                </div>
                            </div>

                            <div id="menu-item-account-admin" class="profile-menu-item account-item ${savedProfile === "admin" ? "selected" : ""}" data-profile-id="${escHtml(user.email)}-admin">
                                <div class="profile-account-content">
                                    <div class="profile-avatar-group">
                                        <div class="profile-avatar"><img src="${initialsDataUrl}" alt="${escHtml(user.name)}">
                                            <div class="profile-avatar-border"></div>
                                            <div class="profile-online-indicator"></div>
                                        </div>
                                        <div class="profile-text-wrapper"><span class="profile-account-name">${escHtml(user.name)}</span> <span class="profile-account-type">Admin Profile</span></div>
                                    </div>
                                    <div class="profile-checkbox ${savedProfile === "admin" ? "selected" : ""}">
                                        <div class="profile-checkbox-check"></div>
                                    </div>
                                </div>
                            </div>

                        </div>
                    </div>
                    <div id="profile-footer-items" class="profile-footer-items">
                        <div id="menu-item-sign-out" class="profile-menu-item">
                            <div class="profile-menu-content">
                                <div class="profile-icon-text">
                                    <svg class="profile-menu-icon" width="20" height="20" viewBox="0 0 20 20" fill="none">
                                        <path d="M14 15L19 10L14 5" stroke="#717680" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"></path>
                                        <path d="M19 10H7M7 19H3C2.44772 19 2 18.5523 2 18V2C2 1.44772 2.44772 1 3 1H7" stroke="#717680" stroke-width="1.67" stroke-linecap="round"></path>
                                    </svg> 
                                    <span class="profile-menu-text">Sign out</span>
                                </div>
                            </div>
                        </div>
                    </div>
                `;
        document.body.appendChild(dropdown);
      }
    }
  }

  /**
   * Generate a simple initials-based avatar as a data URL.
   * Creates a colored circle with the user's initials.
   */
  function generateInitialsAvatar(initials) {
    var canvas = document.createElement("canvas");
    canvas.width = 80;
    canvas.height = 80;
    var ctx = canvas.getContext("2d");

    // Background: POOOL brand blue
    ctx.fillStyle = "#2E2EF9";
    ctx.beginPath();
    ctx.arc(40, 40, 40, 0, Math.PI * 2);
    ctx.fill();

    // Text: white, centered
    ctx.fillStyle = "#FFFFFF";
    ctx.font =
      'bold 28px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(initials || "?", 40, 42);

    return canvas.toDataURL("image/webp");
  }
})();
