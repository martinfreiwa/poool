/**
 * settings.js — Settings page controller.
 *
 * Loads /api/settings, populates the card-based layout in settings.html,
 * and wires per-form save handlers, toggle switches, modals, and GDPR actions.
 *
 * Server I/O, CSRF handling, and 401 redirects are kept in this file so the
 * settings page has one page script.
 */
(function () {
  "use strict";

  function ensureCanonicalSettingsStylesheet() {
    const href = "/static/css/settings.css?v=1.0.16";
    const hasCanonical = Array.from(document.styleSheets).some((sheet) => {
      try {
        return sheet.href && sheet.href.includes("/static/css/settings.css");
      } catch (_) {
        return false;
      }
    });
    if (hasCanonical) return;

    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href;
    document.head.appendChild(link);
  }

  ensureCanonicalSettingsStylesheet();

  const SettingsDataService = (function () {
    function csrfToken() {
      return typeof window.getCsrfToken === "function" ? window.getCsrfToken() : "";
    }

    async function apiFetch(url, method, body) {
      const opts = {
        method,
        credentials: "same-origin",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": csrfToken() },
      };
      if (body) opts.body = JSON.stringify(body);

      try {
        const res = await fetch(url, opts);
        if (res.status === 401) {
          window.location.href = "/auth/login";
          return null;
        }

        const contentType = res.headers.get("content-type") || "";
        if (contentType.includes("application/json")) return await res.json();

        const text = await res.text();
        console.warn(`Settings API non-JSON response [${method}] ${url}: ${res.status} ${text.substring(0, 200)}`);
        return { success: false, message: `Server error (${res.status}). Please refresh and try again.` };
      } catch (err) {
        console.error(`Settings API error [${method}] ${url}:`, err);
        if (typeof Sentry !== "undefined") Sentry.captureException(err);
        return { success: false, message: "Network error. Please try again later." };
      }
    }

    async function apiUpload(url, file) {
      const formData = new FormData();
      formData.append("file", file);

      try {
        const res = await fetch(url, {
          method: "POST",
          credentials: "same-origin",
          headers: { "X-CSRF-Token": csrfToken() },
          body: formData,
        });
        if (res.status === 401) {
          window.location.href = "/auth/login";
          return null;
        }
        return await res.json();
      } catch (err) {
        console.error(`Upload error ${url}:`, err);
        return { error: "Network error during upload." };
      }
    }

    function validatePasswordChange(current, next, confirm) {
      if (!current || !next || !confirm) return "Please fill in all fields.";
      if (next !== confirm) return "New passwords do not match.";
      if (next.length < 8) return "Password must be at least 8 characters.";
      return "";
    }

    return {
      getSettings: () => apiFetch("/api/settings", "GET"),
      saveProfile: (data) => apiFetch("/api/settings/profile", "POST", data),
      savePreferences: (data) => apiFetch("/api/settings/preferences", "POST", data),
      saveNotifications: (data) => apiFetch("/api/settings/notifications", "POST", data),
      uploadAvatar: (file) => apiUpload("/api/upload/avatar", file),
      disable2FA: () => apiFetch("/api/settings/2fa/disable", "POST"),
      saveLeaderboard: (data) => apiFetch("/api/settings/leaderboard", "POST", data),
      saveSocialLinks: (data) => apiFetch("/api/settings/social", "POST", data),
      saveDeveloperProfile: (data) => apiFetch("/api/settings/developer/profile", "POST", data),
      saveDeveloperLinks: (data) => apiFetch("/api/settings/developer/links", "POST", data),
      uploadDeveloperLogo: (file) => apiUpload("/api/upload/developer-logo", file),
      listOAuthConnections: () => apiFetch("/api/settings/oauth", "GET"),
      linkOAuth: (provider) => apiFetch(`/api/settings/oauth/${encodeURIComponent(provider)}/link`, "POST"),
      unlinkOAuth: (id) => apiFetch(`/api/settings/oauth/${encodeURIComponent(id)}`, "DELETE"),
      changePhone: (newPhone) => apiFetch("/api/settings/phone", "POST", { new_phone: newPhone || "" }),
      requestDataExport: () => {
        window.location.href = "/api/settings/export-data";
        return Promise.resolve({ success: true });
      },
      deleteAccount: (password, confirmPhrase) => {
        if (!password) return Promise.resolve({ success: false, message: "Password required." });
        if (confirmPhrase !== "DELETE") return Promise.resolve({ success: false, message: "Type DELETE to confirm." });
        return apiFetch("/api/settings/delete-account", "POST", { current_password: password, confirm: confirmPhrase });
      },
      changePassword: (current, next, confirm) => {
        const message = validatePasswordChange(current, next, confirm);
        if (message) return Promise.resolve({ success: false, message });
        return apiFetch("/api/settings/password", "POST", {
          current_password: current,
          new_password: next,
          confirm_password: confirm,
        });
      },
    };
  })();

  // ─── State ────────────────────────────────────────────────────
  let savedSettings = null;
  let pendingAvatarFile = null;
  let pendingDevLogoFile = null;

  // ─── Small helpers ────────────────────────────────────────────

  function $(id) { return document.getElementById(id); }
  function setVal(id, v) { const el = $(id); if (el) el.value = v == null ? "" : v; }
  function getVal(id) { const el = $(id); return el ? el.value.trim() : ""; }

  function toast(message, type) {
    if (window.showPooolToast) window.showPooolToast(null, message, type || "info");
  }

  function escapeHtml(s) {
    if (s == null) return "";
    return String(s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    })[c]);
  }

  const LOGO_PLACEHOLDER_SVG = `
    <svg viewBox="0 0 24 24" width="28" height="28" fill="none" stroke="#B0B8C9" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
      <circle cx="8.5" cy="8.5" r="1.5" />
      <polyline points="21 15 16 10 5 21" />
    </svg>
  `;

  function setLogoPreview(id, url) {
    const target = $(id);
    if (!target) return;
    target.replaceChildren();
    if (url) {
      const img = document.createElement("img");
      img.src = url;
      img.alt = "";
      target.appendChild(img);
      return;
    }
    target.innerHTML = LOGO_PLACEHOLDER_SVG;
  }

  function setDeveloperLogos(url) {
    setLogoPreview("settings-dev-logo-preview", url);
    setLogoPreview("settings-dev-preview-logo", url);
  }

  function updateDeveloperPreview() {
    const company = $("settings-dev-preview-company");
    const description = $("settings-dev-preview-description");
    if (company) company.textContent = getVal("settings-dev-company") || "Company preview";
    if (description) description.textContent = getVal("settings-dev-description") || "Your developer description will appear here.";
  }

  // ─── State Layer Switcher ─────────────────────────────────────

  function showLayer(which) {
    const skeleton = $("settings-loading-skeleton");
    const content = $("settings-content");
    const errorState = $("settings-empty-state");
    [skeleton, content, errorState].forEach((el) => el && el.classList.add("hidden"));
    if (which === "loading") skeleton && skeleton.classList.remove("hidden");
    else if (which === "content") content && content.classList.remove("hidden");
    else if (which === "error") errorState && errorState.classList.remove("hidden");
  }

  // ─── Load Settings ────────────────────────────────────────────

  async function loadSettings() {
    showLayer("loading");
    try {
      if (typeof SettingsDataService === "undefined") throw new Error("SettingsDataService missing");
      const data = await SettingsDataService.getSettings();
      if (!data || data.error) throw new Error(data?.error || "Failed to fetch settings");

      savedSettings = { ...data };
      populateCore(data);
      populateAddress(data);
      populateIdentity(data);
      populateSecurity(data);
      populatePreferences(data);
      populateLeaderboard(data);
      populateSocial(data);
      populateDeveloper(data);
      updateProfileCompleteness(data);
      applyRoleGate(data.role);
      showLayer("content");
    } catch (err) {
      console.error("Settings load failed:", err);
      toast(err.message || "Failed to load settings.", "error");
      showLayer("error");
    }
  }

  // ─── Populate: Core Profile ───────────────────────────────────

  function populateCore(d) {
    setVal("settings-first-name", d.first_name);
    setVal("settings-middle-name", d.middle_name);
    setVal("settings-last-name", d.last_name);
    setVal("settings-email", d.email);
    setVal("settings-phone", d.phone_number);
    setVal("settings-gender", d.gender);

    const avatar = $("settings-avatar-img");
    if (avatar && d.avatar_url) avatar.src = d.avatar_url;

    const emailStatus = $("settings-email-verified");
    if (emailStatus) {
      emailStatus.textContent = d.email_verified ? "Verified" : "Not verified";
      emailStatus.className = "settings-badge " + (d.email_verified ? "settings-badge--success" : "settings-badge--warn");
    }

    const displayName = $("settings-display-name");
    if (displayName) {
      const fn = [d.first_name, d.last_name].filter(Boolean).join(" ").trim();
      displayName.textContent = fn || d.email || "Unnamed user";
    }
    const displayEmail = $("settings-display-email");
    if (displayEmail) displayEmail.textContent = d.email || "";
  }

  // ─── Populate: Address ────────────────────────────────────────

  function populateAddress(d) {
    setVal("settings-address-1", d.address_line_1);
    setVal("settings-address-2", d.address_line_2);
    setVal("settings-city", d.city);
    setVal("settings-state", d.state_province);
    setVal("settings-postal", d.postal_code);
    setVal("settings-country", d.country);
  }

  // ─── Populate: Identity Vault ─────────────────────────────────

  function populateIdentity(d) {
    setVal("settings-dob", d.date_of_birth);
    setVal("settings-nationality", d.nationality);
    setVal("settings-tax-id", d.tax_id);

    const kycBadge = $("settings-kyc-status");
    if (kycBadge) {
      const status = (d.kyc_status || "not_started").toLowerCase();
      const map = {
        approved: ["Verified", "success"],
        pending: ["Pending review", "warn"],
        in_progress: ["In progress", "warn"],
        rejected: ["Rejected", "danger"],
        not_started: ["Not started", "muted"],
      };
      const [label, tone] = map[status] || map.not_started;
      kycBadge.textContent = label;
      kycBadge.className = "settings-badge settings-badge--" + tone;
    }

    const kycBtn = $("btn-kyc-action");
    if (kycBtn) {
      const s = (d.kyc_status || "not_started").toLowerCase();
      kycBtn.textContent = s === "approved" ? "View details"
        : s === "pending" || s === "in_progress" ? "Continue"
        : s === "rejected" ? "Retry verification"
        : "Start verification";
    }
  }

  // ─── Populate: Security ───────────────────────────────────────

  function populateSecurity(d) {
    const emailDisplay = $("settings-security-email-display");
    if (emailDisplay) emailDisplay.textContent = d.email || "—";

    const totpStatus = $("settings-2fa-badge");
    if (totpStatus) {
      totpStatus.textContent = d.totp_enabled ? "Enabled" : "Disabled";
      totpStatus.className = "settings-badge " + (d.totp_enabled ? "settings-badge--success" : "settings-badge--muted");
    }

    renderOAuthList(d.oauth_connections || d.oauth_accounts || []);
  }

  function renderOAuthList(connections) {
    const list = $("settings-oauth-list");
    if (!list) return;
    const providers = ["google"];
    const byProvider = {};
    connections.forEach((c) => (byProvider[c.provider] = c));
    list.innerHTML = providers.map((p) => {
      const conn = byProvider[p];
      const label = p.charAt(0).toUpperCase() + p.slice(1);
      const email = conn && (conn.email || conn.provider_email);
      const connectionId = conn && conn.id;
      return `
        <div class="settings-oauth-row" data-provider="${p}">
          <div class="settings-oauth-row__provider">${label}</div>
          <div class="settings-oauth-row__status">${conn ? escapeHtml(email || "Connected") : "Not connected"}</div>
          <button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-action="${conn ? "unlink-oauth" : "link-oauth"}" data-provider="${p}" ${conn && connectionId ? `data-connection-id="${escapeHtml(connectionId)}"` : ""}>
            ${conn ? "Disconnect" : "Connect"}
          </button>
        </div>
      `;
    }).join("");
  }

  // ─── Populate: Preferences ────────────────────────────────────

  function populatePreferences(d) {
    setVal("settings-language", d.language || "en");
    setVal("settings-timezone", d.timezone || "UTC");
    setVal("settings-currency", d.currency || "USD");

    setToggle("settings-notify-email", d.email_notifications);
    setToggle("settings-notify-push", d.push_notifications);
  }

  // ─── Populate: Leaderboard ────────────────────────────────────

  function populateLeaderboard(d) {
    const lb = d.leaderboard || {
      visible: d.lb_visible,
      show_avatar: d.lb_avatar,
      display_name: d.lb_display_name,
      bio: d.lb_bio,
    };
    setToggle("settings-lb-visible", lb.visible);
    setToggle("settings-lb-avatar", lb.show_avatar);
    setVal("settings-lb-display-name", lb.display_name);
    setVal("settings-lb-bio", lb.bio);
    updateBioCounter();
  }

  // ─── Populate: Social ─────────────────────────────────────────

  function populateSocial(d) {
    const s = d.social_links || {};
    setVal("settings-social-twitter", s.twitter);
    setVal("settings-social-linkedin", s.linkedin);
    setVal("settings-social-instagram", s.instagram);
    setVal("settings-social-telegram", s.telegram);
    setVal("settings-social-discord", s.discord);
    setVal("settings-social-website", s.website);
  }

  // ─── Populate: Developer ──────────────────────────────────────

  function populateDeveloper(d) {
    if (d.role !== "developer") return;
    const dev = d.developer_profile || {};
    setVal("settings-dev-company", dev.company_name);
    setVal("settings-dev-description", dev.description);
    updateDevDescriptionCounter();
    updateDeveloperPreview();

    const links = dev.links || {};
    setVal("settings-dev-website", links.website);
    setVal("settings-dev-github", links.github);
    setVal("settings-dev-twitter", links.twitter);
    setVal("settings-dev-linkedin", links.linkedin);
    setVal("settings-dev-youtube", links.youtube);

    setDeveloperLogos(dev.logo_url || "");
  }

  function applyRoleGate(role) {
    document.querySelectorAll('[data-role="developer"]').forEach((el) => {
      if (role === "developer") el.removeAttribute("hidden");
      else el.setAttribute("hidden", "");
    });
  }

  // ─── Profile Completeness ─────────────────────────────────────

  function updateProfileCompleteness(d) {
    const fields = [
      !!d.first_name,
      !!d.last_name,
      !!d.email,
      !!d.phone_number,
      !!d.date_of_birth,
      !!d.nationality,
      !!d.address_line_1,
      !!d.city,
      !!d.country,
      !!d.postal_code,
      !!d.avatar_url,
      (d.kyc_status || "").toLowerCase() === "approved",
    ];
    const done = fields.filter(Boolean).length;
    const pct = Math.round((done / fields.length) * 100);

    const bar = $("profile-completeness-bar");
    const text = $("profile-completeness-text");
    const hint = $("profile-completeness-hint");
    if (bar) {
      bar.style.width = pct + "%";
      bar.setAttribute("aria-valuenow", String(pct));
    }
    if (text) text.textContent = pct + "%";
    if (hint) {
      const isDeveloperSettings = !!document.querySelector(".developer-settings-main");
      hint.textContent = pct === 100
        ? "All done. Your profile is fully complete."
        : pct >= 80 ? "Almost there — add a few more details to finish."
        : pct >= 50 ? (isDeveloperSettings
          ? "You're halfway there. Complete more developer review details."
          : "You're halfway there. Complete more to unlock features.")
        : (isDeveloperSettings
          ? "Complete developer identity, links, and account details for review readiness."
          : "Complete your profile to unlock trading and KYC features.");
    }
  }

  // ─── Toggle Switches ──────────────────────────────────────────

  function setToggle(id, active) {
    const el = $(id);
    if (!el) return;
    const on = !!active;
    el.setAttribute("aria-checked", on ? "true" : "false");
    el.setAttribute("data-state", on ? "active" : "inactive");
  }

  function readToggle(id) {
    const el = $(id);
    return el ? el.getAttribute("aria-checked") === "true" : false;
  }

  function toggleSwitch(el) {
    const cur = el.getAttribute("aria-checked") === "true";
    const next = !cur;
    el.setAttribute("aria-checked", next ? "true" : "false");
    el.setAttribute("data-state", next ? "active" : "inactive");
  }

  function bindToggleSwitches() {
    document.addEventListener("click", (e) => {
      const btn = e.target.closest('[role="switch"]');
      if (!btn) return;
      if (btn.disabled) return;
      toggleSwitch(btn);
    });
    document.addEventListener("keydown", (e) => {
      const btn = e.target.closest('[role="switch"]');
      if (!btn) return;
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        toggleSwitch(btn);
      }
    });
  }

  // ─── Form submit handlers ─────────────────────────────────────

  async function onSubmitCore(e) {
    e.preventDefault();
    const payload = {
      first_name: getVal("settings-first-name"),
      middle_name: getVal("settings-middle-name"),
      last_name: getVal("settings-last-name"),
      phone_number: getVal("settings-phone"),
      gender: getVal("settings-gender"),
    };
    const res = await SettingsDataService.saveProfile(payload);
    handleSaveResult(res, "Profile saved.");
  }

  async function onSubmitAddress(e) {
    e.preventDefault();
    const payload = {
      address_line_1: getVal("settings-address-1"),
      address_line_2: getVal("settings-address-2"),
      city: getVal("settings-city"),
      state_province: getVal("settings-state"),
      postal_code: getVal("settings-postal"),
      country: getVal("settings-country"),
    };
    const res = await SettingsDataService.saveProfile(payload);
    handleSaveResult(res, "Address saved.");
  }

  async function onSubmitIdentity(e) {
    e.preventDefault();
    const payload = {
      date_of_birth: getVal("settings-dob"),
      nationality: getVal("settings-nationality"),
      tax_id: getVal("settings-tax-id"),
    };
    const res = await SettingsDataService.saveProfile(payload);
    handleSaveResult(res, "Identity details saved.");
  }

  async function onSubmitPreferences(e) {
    e.preventDefault();
    const payload = {
      language: getVal("settings-language"),
      timezone: getVal("settings-timezone"),
      currency: getVal("settings-currency"),
    };
    const prefs = await SettingsDataService.savePreferences(payload);
    const notifs = await SettingsDataService.saveNotifications({
      email_notifications: readToggle("settings-notify-email"),
      push_notifications: readToggle("settings-notify-push"),
    });
    if (prefs?.success && notifs?.success) toast("Preferences saved.", "success");
    else toast((prefs && prefs.message) || (notifs && notifs.message) || "Save failed.", "error");
  }

  async function onSubmitLeaderboard(e) {
    e.preventDefault();
    const payload = {
      visible: readToggle("settings-lb-visible"),
      show_avatar: readToggle("settings-lb-avatar"),
      display_name: getVal("settings-lb-display-name"),
      bio: getVal("settings-lb-bio"),
    };
    const res = await SettingsDataService.saveLeaderboard(payload);
    handleSaveResult(res, "Leaderboard preferences saved.");
  }

  async function onSubmitSocial(e) {
    e.preventDefault();
    const payload = {
      twitter: getVal("settings-social-twitter"),
      linkedin: getVal("settings-social-linkedin"),
      instagram: getVal("settings-social-instagram"),
      telegram: getVal("settings-social-telegram"),
      discord: getVal("settings-social-discord"),
      website: getVal("settings-social-website"),
    };
    const res = await SettingsDataService.saveSocialLinks(payload);
    handleSaveResult(res, "Social links saved.");
  }

  async function onSubmitDevProfile(e) {
    e.preventDefault();
    const payload = {
      company_name: getVal("settings-dev-company"),
      description: getVal("settings-dev-description"),
    };
    const res = await SettingsDataService.saveDeveloperProfile(payload);
    handleSaveResult(res, "Developer profile saved.");
  }

  async function onSubmitDevLinks(e) {
    e.preventDefault();
    const payload = {
      website: getVal("settings-dev-website"),
      github: getVal("settings-dev-github"),
      twitter: getVal("settings-dev-twitter"),
      linkedin: getVal("settings-dev-linkedin"),
      youtube: getVal("settings-dev-youtube"),
    };
    const res = await SettingsDataService.saveDeveloperLinks(payload);
    handleSaveResult(res, "Developer links saved.");
  }

  function handleSaveResult(res, okMsg) {
    if (res?.success) toast(okMsg, "success");
    else toast(res?.message || "Save failed.", "error");
  }

  // ─── Modal Helpers ────────────────────────────────────────────

  function openModal(id) {
    const m = $(id);
    if (!m) return;
    m.removeAttribute("hidden");
    m.classList.add("is-open");
    const firstInput = m.querySelector("input, select, textarea, button");
    firstInput && firstInput.focus();
    document.body.classList.add("modal-open");
  }

  function closeModal(m) {
    if (!m) return;
    m.setAttribute("hidden", "");
    m.classList.remove("is-open");
    const form = m.querySelector("form");
    form && form.reset();
    const err = m.querySelector("[id$=-error]");
    if (err) { err.hidden = true; err.textContent = ""; }
    document.body.classList.remove("modal-open");
  }

  function showModalError(id, msg) {
    const el = $(id);
    if (!el) return;
    el.textContent = msg;
    el.hidden = false;
  }

  function bindModals() {
    document.addEventListener("click", (e) => {
      const opener = e.target.closest("[data-modal-target]");
      if (opener) {
        e.preventDefault();
        openModal(opener.getAttribute("data-modal-target"));
        return;
      }
      const closer = e.target.closest("[data-modal-close]");
      if (closer) {
        e.preventDefault();
        closeModal(closer.closest(".settings-modal"));
      }
    });
    document.addEventListener("keydown", (e) => {
      if (e.key !== "Escape") return;
      const open = document.querySelector(".settings-modal:not([hidden])");
      if (open) closeModal(open);
    });
  }

  async function onSubmitChangePassword(e) {
    e.preventDefault();
    const res = await SettingsDataService.changePassword(
      getVal("modal-password-current"),
      getVal("modal-password-new"),
      getVal("modal-password-confirm"),
    );
    if (res?.success) {
      toast("Password changed.", "success");
      closeModal($("modal-change-password"));
    } else {
      showModalError("modal-password-error", res?.message || "Failed.");
    }
  }

  async function onSubmitChangePhone(e) {
    e.preventDefault();
    const res = await SettingsDataService.changePhone(getVal("modal-phone-new"));
    if (res?.success) {
      toast(res.message || "Verification code sent.", "success");
      closeModal($("modal-change-phone"));
    } else {
      showModalError("modal-phone-error", res?.message || "Failed.");
    }
  }

  async function onSubmitDeleteAccount(e) {
    e.preventDefault();
    const res = await SettingsDataService.deleteAccount(
      getVal("modal-delete-password"),
      getVal("modal-delete-confirm"),
    );
    if (res?.success) {
      toast("Account deletion requested.", "success");
      closeModal($("modal-delete-account"));
      setTimeout(() => { window.location.href = "/"; }, 1500);
    } else {
      showModalError("modal-delete-error", res?.message || "Failed.");
    }
  }

  // ─── Action Delegation ────────────────────────────────────────

  function bindActions() {
    document.addEventListener("click", async (e) => {
      const act = e.target.closest("[data-action]");
      if (!act) return;
      const action = act.getAttribute("data-action");

      switch (action) {
        case "disable-2fa": {
          e.preventDefault();
          if (!confirm("Disable two-factor authentication?")) return;
          const res = await SettingsDataService.disable2FA();
          if (res?.success) { toast("2FA disabled.", "success"); loadSettings(); }
          else toast(res?.message || "Failed.", "error");
          break;
        }
        case "request-data-export": {
          e.preventDefault();
          act.disabled = true;
          const res = await SettingsDataService.requestDataExport();
          act.disabled = false;
          handleSaveResult(res, "Data export download started.");
          break;
        }
        case "link-oauth": {
          e.preventDefault();
          const provider = act.getAttribute("data-provider");
          const res = await SettingsDataService.linkOAuth(provider);
          if (res?.redirect_url) window.location.href = res.redirect_url;
          else toast(res?.message || "OAuth link failed.", "error");
          break;
        }
        case "unlink-oauth": {
          e.preventDefault();
          const id = act.getAttribute("data-connection-id");
          if (!confirm("Disconnect this account?")) return;
          const res = await SettingsDataService.unlinkOAuth(id);
          if (res?.success) { toast("Disconnected.", "success"); loadSettings(); }
          else toast(res?.message || "Failed.", "error");
          break;
        }
        case "retry-load": {
          e.preventDefault();
          loadSettings();
          break;
        }
      }
    });
  }

  // ─── File Inputs ──────────────────────────────────────────────

  function bindFileInputs() {
    const avatarBtn = $("btn-photo-upload");
    const avatarInput = $("profile-photo-upload");
    if (avatarBtn && avatarInput) {
      avatarBtn.addEventListener("click", () => avatarInput.click());
      avatarInput.addEventListener("change", async () => {
        const file = avatarInput.files && avatarInput.files[0];
        if (!file) return;
        if (file.size > 5 * 1024 * 1024) return toast("File too large (max 5 MB).", "error");
        pendingAvatarFile = file;
        const preview = $("settings-avatar-img");
        if (preview) preview.src = URL.createObjectURL(file);
        const res = await SettingsDataService.uploadAvatar(file);
        const url = res && (res.url || res.avatar_url);
        if (url) { toast("Avatar updated.", "success"); if (preview) preview.src = url; }
        else toast(res?.error || res?.message || "Upload failed.", "error");
      });
    }

    const devBtn = $("btn-upload-dev-logo");
    const devInput = $("settings-dev-logo-input");
    if (devBtn && devInput) {
      devBtn.addEventListener("click", () => devInput.click());
      devInput.addEventListener("change", async () => {
        const file = devInput.files && devInput.files[0];
        if (!file) return;
        if (file.size > 2 * 1024 * 1024) return toast("Logo too large (max 2 MB).", "error");
        pendingDevLogoFile = file;
        const res = await SettingsDataService.uploadDeveloperLogo(file);
        if (res?.url) {
          toast("Logo updated.", "success");
          setDeveloperLogos(res.url);
        } else toast(res?.error || res?.message || "Upload failed.", "error");
      });
    }
  }

  // ─── Char Counters ────────────────────────────────────────────

  function updateBioCounter() {
    const el = $("settings-lb-bio");
    const out = $("settings-lb-bio-count");
    if (el && out) out.textContent = String(el.value.length);
  }

  function updateDevDescriptionCounter() {
    const el = $("settings-dev-description");
    const out = $("settings-dev-description-count");
    if (el && out) out.textContent = String(el.value.length);
  }

  function bindCounters() {
    $("settings-lb-bio")?.addEventListener("input", updateBioCounter);
    $("settings-dev-description")?.addEventListener("input", () => {
      updateDevDescriptionCounter();
      updateDeveloperPreview();
    });
    $("settings-dev-company")?.addEventListener("input", updateDeveloperPreview);
  }

  // ─── Direct modal triggers (buttons with explicit IDs) ────────

  function bindDirectModalButtons() {
    $("btn-change-password")?.addEventListener("click", (e) => { e.preventDefault(); openModal("modal-change-password"); });
    $("btn-change-phone")?.addEventListener("click", (e) => { e.preventDefault(); openModal("modal-change-phone"); });
    $("btn-delete-account")?.addEventListener("click", (e) => { e.preventDefault(); openModal("modal-delete-account"); });
    $("btn-retry-load")?.addEventListener("click", (e) => { e.preventDefault(); loadSettings(); });
  }

  // ─── Init ─────────────────────────────────────────────────────

  function init() {
    // Form submit handlers
    $("form-core-profile")?.addEventListener("submit", onSubmitCore);
    $("form-address")?.addEventListener("submit", onSubmitAddress);
    $("form-identity")?.addEventListener("submit", onSubmitIdentity);
    $("form-preferences")?.addEventListener("submit", onSubmitPreferences);
    $("form-leaderboard")?.addEventListener("submit", onSubmitLeaderboard);
    $("form-social")?.addEventListener("submit", onSubmitSocial);
    $("form-developer-identity")?.addEventListener("submit", onSubmitDevProfile);
    $("form-developer-links")?.addEventListener("submit", onSubmitDevLinks);

    // Modal form submits
    $("form-change-password")?.addEventListener("submit", onSubmitChangePassword);
    $("form-change-phone")?.addEventListener("submit", onSubmitChangePhone);
    $("form-delete-account")?.addEventListener("submit", onSubmitDeleteAccount);

    // Reset handlers — repopulate from savedSettings
    document.querySelectorAll("form[id^=form-]").forEach((f) => {
      f.addEventListener("reset", () => {
        setTimeout(() => savedSettings && (
          populateCore(savedSettings),
          populateAddress(savedSettings),
          populateIdentity(savedSettings),
          populatePreferences(savedSettings),
          populateLeaderboard(savedSettings),
          populateSocial(savedSettings),
          populateDeveloper(savedSettings)
        ), 0);
      });
    });

    bindToggleSwitches();
    bindActions();
    bindModals();
    bindDirectModalButtons();
    bindFileInputs();
    bindCounters();
    bindSectionNav();

    loadSettings();
  }

  // ─── Section nav scroll-spy ───────────────────────────────────

  function bindSectionNav() {
    const links = document.querySelectorAll(".settings-nav__link");
    if (!links.length || !("IntersectionObserver" in window)) return;

    const anchorIds = Array.from(links).map((a) => a.getAttribute("href").slice(1));
    const anchors = anchorIds
      .map((id) => document.getElementById(id))
      .filter(Boolean);

    const setActive = (id) => {
      links.forEach((a) =>
        a.classList.toggle("is-active", a.getAttribute("href") === `#${id}`)
      );
    };

    // IntersectionObserver — anchor enters upper band of viewport = active
    const io = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (visible.length) setActive(visible[0].target.id);
      },
      { rootMargin: "-96px 0px -60% 0px", threshold: 0 }
    );
    anchors.forEach((el) => io.observe(el));

    // Click-to-scroll with smooth behavior + focus management
    links.forEach((link) => {
      link.addEventListener("click", (e) => {
        const id = link.getAttribute("href").slice(1);
        const target = document.getElementById(id);
        if (!target) return;
        e.preventDefault();
        target.scrollIntoView({ behavior: "smooth", block: "start" });
        setActive(id);
        history.replaceState(null, "", `#${id}`);
      });
    });

    // Honor initial hash
    if (location.hash) {
      const id = location.hash.slice(1);
      if (document.getElementById(id)) setActive(id);
    } else {
      setActive(anchorIds[0]);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
