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
      if (token) {
        if (config.headers instanceof Headers) {
          config.headers.append("X-CSRF-Token", token);
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
    "nav-users": "users.view",
    "nav-kyc": "kyc.read",
    "nav-support": "support.read",
    "nav-submissions": "submissions.review",
    "nav-assets": "assets.view",
    "nav-orders": "orders.view",
    "nav-deposits": "deposits.confirm",
    "nav-treasury": "treasury.read",
    "nav-dividends": "financials.payout.draft",
    "nav-rewards": "rewards.view",
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
      if (roles.includes("super_admin")) {
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

      // Show or hide based on permission
      if (perms.has(requiredPerm)) {
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
