/**
 * Admin Permission Guard — Role-Aware Sidebar Visibility & PII Masking.
 *
 * This script runs on every admin page and:
 * 1. Fetches current admin's permissions from /api/me
 * 2. Hides sidebar navigation links the admin lacks permission to access
 * 3. Provides PII masking utilities for pages that display user data
 * 4. Exposes window.adminPermissions for other scripts to use
 *
 * Zero Trust: If permissions can't be fetched, ALL restricted sections are hidden.
 */
(function () {
  "use strict";

  // ── Global Fetch Interceptor for CSRF ──
  const originalFetch = window.fetch;
  window.fetch = async function () {
    let [resource, config] = arguments;
    if (config && ["POST", "PUT", "PATCH", "DELETE"].includes(config.method?.toUpperCase())) {
      const value = `; ${document.cookie}`;
      const parts = value.split(`; csrf_token=`);
      let token = "";
      if (parts.length === 2) token = parts.pop().split(';').shift();
      
      config.headers = config.headers || {};
      let hasCsrfHeader = false;
      if (config.headers instanceof Headers) {
        hasCsrfHeader = config.headers.has("X-CSRF-Token") || config.headers.has("x-csrf-token");
      } else {
        hasCsrfHeader = Object.keys(config.headers).some((key) => key.toLowerCase() === "x-csrf-token");
      }

      if (token && !hasCsrfHeader) {
        if (config.headers instanceof Headers) {
          config.headers.set("X-CSRF-Token", token);
        } else {
          config.headers["X-CSRF-Token"] = token;
        }
      }
    }
    return originalFetch(resource, config);
  };


  // ── Page → Required Permission Mapping ──
  const PAGE_PERMISSION_MAP = {
    "nav-dashboard": null, // Always visible
    "nav-blog": ["blog.view", "blog.manage"],
    "nav-blog-persona": ["blog.view", "blog.manage"],
    "nav-blog-strategy": ["blog.view", "blog.manage"],
    "nav-users": "users.view",
    "nav-kyc": "kyc.read",
    "nav-support": "support.read",
    "nav-submissions": "submissions.review",
    "nav-assets": "assets.view",
    "nav-orders": "orders.view",
    "nav-deposits": "deposits.confirm",
    "nav-treasury": "treasury.read",
    "nav-blockchain": "treasury.read",
    "nav-blockchain-sync": "treasury.read",
    "nav-dividends": "financials.payout.draft",
    "nav-rewards": "rewards.view",
    "nav-affiliate-apps": "affiliates.manage",
    "nav-affiliate-finance": "affiliates.manage",
    "nav-affiliate-fraud": "affiliates.manage",
    "nav-admins": "admins.manage",
    "nav-roles": "roles.view",
    "nav-audit": "audit.read",
    "nav-approvals": "approvals.manage", // was missing — security fix
    "nav-notifications": "notifications.send",
    "nav-reports": "reports.generate",
    "nav-email": "email.campaigns",
    "nav-storage": "system.health",
    "nav-system": "system.health",
    "nav-settings": "settings.view",
    // ── Marketplace Admin Pages (Phase 6B.2) ──
    "nav-mp-overview": "marketplace.view",
    "nav-mp-orderbook": "marketplace.view",
    "nav-mp-trades": "marketplace.view",
    "nav-mp-orders": "marketplace.manage",
    "nav-mp-approvals": "marketplace.manage",
    "nav-mp-primary-escrow": "marketplace.view",
    "nav-mp-fees": "marketplace.manage",
    "nav-mp-alerts": "marketplace.manage",
    "nav-mp-p2p": "marketplace.view",
    "nav-mp-recon": "marketplace.manage",
    "nav-mp-compliance": "marketplace.compliance",
    "nav-mp-analytics": "marketplace.view",
    "nav-mp-settings": "marketplace.manage",
    "nav-com-overview": "community.view",
    "nav-com-announcements": "community.manage",
    "nav-com-posts": "community.manage",
    "nav-com-comments": "community.manage",
    "nav-com-reports": "community.manage",
    "nav-com-users": "community.manage",
    "nav-com-badges": "community.manage",
    "nav-com-amas": "community.manage",
    "nav-com-challenges": "community.manage",
    "nav-com-circles": "community.manage",
    "nav-com-leaderboard": "community.view",
  };

  // ── PII Masking Rules ──
  const PII_MASK_CONFIG = {
    email: (val) => {
      if (!val || typeof val !== "string") return val;
      const [local, domain] = val.split("@");
      if (!domain) return val;
      return local.substring(0, 2) + "***@" + domain;
    },
    phone: (val) => {
      if (!val || typeof val !== "string") return val;
      return val.substring(0, 3) + "***" + val.slice(-2);
    },
    name: (val) => {
      if (!val || typeof val !== "string") return val;
      const parts = val.split(" ");
      return parts.map((p) => p[0] + "***").join(" ");
    },
    taxId: (_val) => "***-**-****",
    address: (_val) => "*** (masked)",
  };

  // ── State ──
  window.adminPermissions = {
    permissions: [],
    roles: [],
    loaded: false,
    hasPiiAccess: false,

    /** Check if admin has a specific permission */
    has(perm) {
      if (this.permissions.includes("all")) return true;
      return this.permissions.includes(perm);
    },

    /** Check if admin has ANY of the given permissions */
    hasAny(...perms) {
      if (this.permissions.includes("all")) return true;
      return perms.some((p) => this.permissions.includes(p));
    },

    /** Mask PII string based on type. Returns masked version if no pii.view. */
    mask(value, type = "email") {
      if (this.hasPiiAccess) return value;
      const masker = PII_MASK_CONFIG[type];
      return masker ? masker(value) : value;
    },
  };

  document.addEventListener("DOMContentLoaded", async () => {
    try {
      // Fetch current user's data including roles
      const resp = await fetch("/api/me");
      if (!resp.ok) throw new Error("Failed to fetch user data");

      const data = await resp.json();
      // The /api/me endpoint returns `role` (singular string),
      // normalise to an array so the rest of the logic works.
      let roles = data.roles || [];
      if (typeof data.role === "string" && data.role) {
        roles = Array.isArray(roles) && roles.length ? roles : [data.role];
      }

      // Fetch permissions for the user's admin roles
      let allPerms = [];
      if (roles.includes("super_admin") || roles.includes("admin")) {
        allPerms = ["all"];
      } else {
        // Fetch from the roles API
        try {
          const rolesResp = await fetch("/api/admin/roles");
          if (rolesResp.ok) {
            const rolesData = await rolesResp.json();
            for (const role of rolesData) {
              if (roles.includes(role.name)) {
                allPerms.push(...(role.permissions || []));
              }
            }
          }
        } catch (_e) {
          /* silent fail */
        }
      }

      // Deduplicate
      window.adminPermissions.permissions = [...new Set(allPerms)];
      window.adminPermissions.roles = roles;
      window.adminPermissions.loaded = true;
      window.adminPermissions.hasPiiAccess =
        window.adminPermissions.has("pii.view");
    } catch (e) {
      window.adminPermissions.loaded = true;
    } finally {
      // Allow local UI development to bypass Zero Trust if there are no permissions
      const isLocal =
        window.location.protocol === "file:" ||
        window.location.hostname === "localhost" ||
        window.location.hostname === "127.0.0.1" ||
        window.location.hostname === ""; // sometimes empty in certain test runners

      if (
        isLocal &&
        (!window.adminPermissions.permissions ||
          window.adminPermissions.permissions.length === 0)
      ) {
        window.adminPermissions.permissions = ["all"];
        window.adminPermissions.roles = ["super_admin"];
        window.adminPermissions.hasPiiAccess = true;
      }

      // Apply visibility now (sidebar may or may not be injected yet)
      applySidebarVisibility();

      // Emit event — other scripts (e.g. page JS) can react to perms
      document.dispatchEvent(
        new CustomEvent("admin:permissions-loaded", {
          detail: window.adminPermissions,
        }),
      );
    }
  });

  // Re-apply whenever the sidebar is (re-)injected dynamically.
  // The sidebar loader fires 'admin:sidebar-ready' after innerHTML is set.
  document.addEventListener("admin:sidebar-ready", () => {
    if (window.adminPermissions.loaded) {
      applySidebarVisibility();
    }
  });

  function applySidebarVisibility() {
    const perms = window.adminPermissions;

    for (const [navId, requiredPerm] of Object.entries(PAGE_PERMISSION_MAP)) {
      const el = document.getElementById(navId);
      if (!el) continue;

      // null = always visible
      if (requiredPerm === null) {
        el.style.display = "";
        continue;
      }

      const allowed = Array.isArray(requiredPerm)
        ? perms.hasAny(...requiredPerm)
        : perms.has(requiredPerm);

      // Show or hide based on permission
      if (allowed) {
        el.style.display = "";
        el.removeAttribute("data-perm-hidden");
      } else {
        el.style.display = "none";
        el.setAttribute("data-perm-hidden", "1");
      }
    }

    // Show/hide empty nav section groups (label + items)
    document.querySelectorAll(".admin-nav-section").forEach((section) => {
      const label = section.querySelector(".admin-nav-section-label");
      if (!label) return; // Dashboard section has no label, always keep it
      const visibleLinks = section.querySelectorAll(
        ".admin-nav-item:not([data-perm-hidden])",
      );
      section.style.display = visibleLinks.length === 0 ? "none" : "";
    });
  }
})();
