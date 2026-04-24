/**
 * settings.js — Settings page controller.
 *
 * Loads /api/settings, populates the card-based layout in settings-3.html,
 * and wires per-form save handlers, toggle switches, modals, and GDPR actions.
 *
 * All server I/O flows through SettingsDataService (settings-service.js).
 * CSRF + 401 handling is inherited from that layer.
 */
(function () {
  "use strict";

  // ─── State ────────────────────────────────────────────────────
  let savedSettings = null;
  let pendingAvatarFile = null;
  let pendingDevLogoFile = null;
  let pendingRevokeSessionId = null;

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
      populateFinancial(data);
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

  // ─── Populate: Financial Overview ─────────────────────────────

  function populateFinancial(d) {
    const refCode = $("settings-referral-code");
    if (refCode) refCode.textContent = d.referral_code || "—";

    const tier = $("settings-tier-name");
    if (tier) tier.textContent = d.membership_tier || "Standard";

    renderPaymentMethods(d.payment_methods || []);
  }

  function renderPaymentMethods(methods) {
    const list = $("settings-payment-methods-list");
    if (!list) return;
    if (!methods.length) {
      list.innerHTML = `<div class="settings-empty-row">No payment methods saved yet.</div>`;
      return;
    }
    list.innerHTML = methods.map((m) => `
      <div class="settings-payment-row" data-method-id="${escapeHtml(m.id)}">
        <div class="settings-payment-row__brand">${escapeHtml(m.brand || "Card")}</div>
        <div class="settings-payment-row__last4">•••• ${escapeHtml(m.last4 || "----")}</div>
        <div class="settings-payment-row__expiry">${escapeHtml(m.exp_month || "")}/${escapeHtml(m.exp_year || "")}</div>
        <button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-action="delete-payment" data-method-id="${escapeHtml(m.id)}">Remove</button>
      </div>
    `).join("");
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

    renderOAuthList(d.oauth_connections || []);
    loadSessionsAsync();
  }

  function renderOAuthList(connections) {
    const list = $("settings-oauth-list");
    if (!list) return;
    const providers = ["google", "facebook", "apple", "github"];
    const byProvider = {};
    connections.forEach((c) => (byProvider[c.provider] = c));
    list.innerHTML = providers.map((p) => {
      const conn = byProvider[p];
      const label = p.charAt(0).toUpperCase() + p.slice(1);
      return `
        <div class="settings-oauth-row" data-provider="${p}">
          <div class="settings-oauth-row__provider">${label}</div>
          <div class="settings-oauth-row__status">${conn ? escapeHtml(conn.email || "Connected") : "Not connected"}</div>
          <button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-action="${conn ? "unlink-oauth" : "link-oauth"}" data-provider="${p}" ${conn ? `data-connection-id="${escapeHtml(conn.id)}"` : ""}>
            ${conn ? "Disconnect" : "Connect"}
          </button>
        </div>
      `;
    }).join("");
  }

  async function loadSessionsAsync() {
    try {
      const res = await SettingsDataService.listSessions();
      renderSessions(res?.sessions || []);
    } catch (_) { /* non-fatal */ }
  }

  function renderSessions(sessions) {
    const list = $("settings-sessions-list");
    if (!list) return;
    if (!sessions.length) {
      list.innerHTML = `<div class="settings-empty-row">No active sessions.</div>`;
      return;
    }
    list.innerHTML = sessions.map((s) => `
      <div class="settings-session-row" data-session-id="${escapeHtml(s.id)}">
        <div class="settings-session-row__device">
          <strong>${escapeHtml(s.device || "Unknown device")}</strong>${s.current ? ' <span class="settings-badge settings-badge--info">This device</span>' : ""}
        </div>
        <div class="settings-session-row__meta">
          ${escapeHtml(s.browser || "")} · ${escapeHtml(s.location || "")} · ${escapeHtml(s.last_seen || "")}
        </div>
        ${s.current ? "" : `<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-action="revoke-session" data-session-id="${escapeHtml(s.id)}">Revoke</button>`}
      </div>
    `).join("");
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
    const lb = d.leaderboard || {};
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

    const links = dev.links || {};
    setVal("settings-dev-website", links.website);
    setVal("settings-dev-github", links.github);
    setVal("settings-dev-twitter", links.twitter);
    setVal("settings-dev-linkedin", links.linkedin);
    setVal("settings-dev-youtube", links.youtube);

    const logoPreview = $("settings-dev-logo-preview");
    if (logoPreview && dev.logo_url) {
      logoPreview.innerHTML = `<img src="${escapeHtml(dev.logo_url)}" alt="">`;
    }
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
      hint.textContent = pct === 100
        ? "All done. Your profile is fully complete."
        : pct >= 80 ? "Almost there — add a few more details to finish."
        : pct >= 50 ? "You're halfway there. Complete more to unlock features."
        : "Complete your profile to unlock trading and KYC features.";
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

  async function onSubmitChangeEmail(e) {
    e.preventDefault();
    const res = await SettingsDataService.changeEmail(
      getVal("modal-email-new"),
      getVal("modal-email-password"),
    );
    if (res?.success) {
      toast(res.message || "Verification email sent.", "success");
      closeModal($("modal-change-email"));
    } else {
      showModalError("modal-email-error", res?.message || "Failed.");
    }
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

  async function onConfirmRevokeSession() {
    if (!pendingRevokeSessionId) return;
    const res = await SettingsDataService.revokeSession(pendingRevokeSessionId);
    pendingRevokeSessionId = null;
    closeModal($("modal-revoke-session"));
    if (res?.success) { toast("Session revoked.", "success"); loadSessionsAsync(); }
    else toast(res?.message || "Failed to revoke session.", "error");
  }

  // ─── Action Delegation ────────────────────────────────────────

  function bindActions() {
    document.addEventListener("click", async (e) => {
      const act = e.target.closest("[data-action]");
      if (!act) return;
      const action = act.getAttribute("data-action");

      switch (action) {
        case "copy-referral": {
          e.preventDefault();
          const code = $("settings-referral-code")?.textContent?.trim() || "";
          if (!code || code === "—") return toast("No referral code yet.", "error");
          try { await navigator.clipboard.writeText(code); toast("Referral code copied.", "success"); }
          catch (_) { toast("Copy failed — please copy manually.", "error"); }
          break;
        }
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
          handleSaveResult(res, "Export requested — check your email.");
          break;
        }
        case "revoke-session": {
          e.preventDefault();
          pendingRevokeSessionId = act.getAttribute("data-session-id");
          const detail = $("modal-revoke-session-detail");
          if (detail) detail.textContent = "This device will be signed out immediately.";
          openModal("modal-revoke-session");
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
        case "delete-payment": {
          e.preventDefault();
          const id = act.getAttribute("data-method-id");
          if (!confirm("Remove this payment method?")) return;
          const res = await SettingsDataService.deletePaymentMethod(id);
          if (res?.success) { toast("Payment method removed.", "success"); loadSettings(); }
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
        if (res?.url) { toast("Avatar updated.", "success"); if (preview) preview.src = res.url; }
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
          const preview = $("settings-dev-logo-preview");
          if (preview) preview.innerHTML = `<img src="${escapeHtml(res.url)}" alt="">`;
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
    $("settings-dev-description")?.addEventListener("input", updateDevDescriptionCounter);
  }

  // ─── Direct modal triggers (buttons with explicit IDs) ────────

  function bindDirectModalButtons() {
    $("btn-change-email")?.addEventListener("click", (e) => { e.preventDefault(); openModal("modal-change-email"); });
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
    $("form-change-email")?.addEventListener("submit", onSubmitChangeEmail);
    $("form-change-password")?.addEventListener("submit", onSubmitChangePassword);
    $("form-change-phone")?.addEventListener("submit", onSubmitChangePhone);
    $("form-delete-account")?.addEventListener("submit", onSubmitDeleteAccount);
    $("btn-confirm-revoke-session")?.addEventListener("click", onConfirmRevokeSession);

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

    loadSettings();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
