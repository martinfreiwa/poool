/**
 * settings.js  –  Phase 5: Polished UI Controller
 *
 * Fixes from UX audit:
 *   - Loading states properly resolved
 *   - Consistent badge rendering
 *   - Clean session/OAuth card rendering
 *   - Profile completeness
 *   - Tab ARIA roles
 */
(function () {
  "use strict";

  // ─── State ──────────────────────────────────────────────────
  let savedSettings = null;
  let pendingPhotoFile = null;
  let pendingPhotoPreview = null;

  // ─── Toast Notification System ──────────────────────────────

  function showToast(message, type) {
  if(window.showPooolToast) {
    window.showPooolToast(null, message, type);
  }
}

  // ─── State Layer Switcher ────────────────────────────────────

  function showLayer(which) {
    const skeleton = document.getElementById("settings-loading-skeleton");
    const content = document.getElementById("settings-content");
    const emptyState = document.getElementById("settings-empty-state");

    const hidden = (el) => el && el.classList.add("hidden");
    const shown = (el) => el && el.classList.remove("hidden");

    hidden(skeleton); hidden(content); hidden(emptyState);

    if (which === "loading") shown(skeleton);
    else if (which === "content") shown(content);
    else if (which === "error") shown(emptyState);
  }

  // ─── Load Settings ────────────────────────────────────────────

  async function loadSettings() {
    showLayer("loading");

    try {
      if (typeof SettingsDataService === "undefined") {
        throw new Error("SettingsDataService not loaded");
      }

      const data = await SettingsDataService.getSettings();

      if (!data || data.error) {
        throw new Error(data?.error || "Failed to fetch settings");
      }

      savedSettings = { ...data };
      populateMyDetails(data);
      populatePreferences(data);
      populateNotifications(data);
      populateSecurity(data);

      showLayer("content");
      switchTab("mydetails");

    } catch (err) {
      console.error("Settings load failed:", err);
      showToast(err.message || "Failed to load settings. Please refresh.", "error");
      showLayer("error");
    }
  }

  // ─── Tab: My Details ─────────────────────────────────────────

  function populateMyDetails(data) {
    setVal("settings-first-name", data.first_name || "");
    setVal("settings-last-name", data.last_name || "");
    setVal("settings-email", data.email || "");
    setVal("settings-phone", data.phone_number || "");
    setVal("settings-role", data.role || "investor");

    setVal("settings-dob", data.date_of_birth || "");
    setVal("settings-nationality", data.nationality || "");
    setVal("settings-address-1", data.address_line_1 || "");
    setVal("settings-address-2", data.address_line_2 || "");
    setVal("settings-city", data.city || "");
    setVal("settings-state", data.state_province || "");
    setVal("settings-postal", data.postal_code || "");
    setVal("settings-tax-id", data.tax_id || "");
    setVal("settings-annual-income", data.annual_income_cents ? data.annual_income_cents / 100 : "");

    // KYC Badge — properly resolve instead of showing "Loading"
    const kycBadge = document.getElementById("settings-kyc-status");
    if (kycBadge) {
      const statusStr = (data.kyc_status || "not_started").toLowerCase();
      const displayMap = {
        approved: "Verified",
        pending: "Pending",
        in_review: "In Review",
        rejected: "Rejected",
        missing: "Not Started",
        not_started: "Not Started",
      };
      kycBadge.textContent = displayMap[statusStr] || statusStr.replace("_", " ");
      kycBadge.className = `settings-badge settings-badge--${statusStr}`;
    }

    const imgPreview = document.querySelector(".settings-photo-current img");
    if (imgPreview && data.avatar_url) {
      imgPreview.src = data.avatar_url;
    }

    const countrySelect = document.getElementById("settings-country");
    if (countrySelect && data.country) {
      countrySelect.value = data.country;
      if (typeof updateCountryFlag === "function") updateCountryFlag();
    }

    const tzSelect = document.getElementById("settings-timezone");
    if (tzSelect && data.timezone) tzSelect.value = data.timezone;

    // Financial Limits
    const fmtMoney = (cents) => {
      if (cents == null) return "—";
      return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(cents / 100);
    };
    const invLimit = document.getElementById("settings-investment-limit");
    if (invLimit) {
      invLimit.textContent = fmtMoney(data.investment_limit_cents);
    }
    const inv12m = document.getElementById("settings-invested-12m");
    if (inv12m) {
      inv12m.textContent = fmtMoney(data.invested_12m_cents);
    }
    const limAvail = document.getElementById("settings-limit-available");
    if (limAvail) {
      limAvail.textContent = fmtMoney(data.limit_available_cents);
    }

    // Tier & Referral
    const tierEl = document.getElementById("settings-tier-name");
    if (tierEl) tierEl.textContent = data.tier_name || "Basic";
    const refEl = document.getElementById("settings-referral-code");
    if (refEl) refEl.textContent = data.referral_code || "—";

    // Profile Completeness
    updateProfileCompleteness(data);
  }

  function updateProfileCompleteness(data) {
    const fields = [
      "first_name", "last_name", "date_of_birth", "nationality",
      "address_line_1", "city", "state_province", "postal_code", "tax_id", "annual_income_cents"
    ];
    const filled = fields.filter(f => data[f] && String(data[f]).trim() !== "").length;
    const pct = Math.round((filled / fields.length) * 100) || 0;

    const bar = document.getElementById("profile-completeness-bar");
    const txt = document.getElementById("profile-completeness-text");
    const hint = document.getElementById("profile-completeness-hint");

    if (bar && txt) {
      bar.style.width = pct + "%";
      txt.textContent = pct + "%";

      if (pct === 100) {
        bar.style.background = "#12B76A";
        txt.style.color = "#12B76A";
        if (hint) hint.textContent = "Your profile is complete! You're ready to invest.";
      } else {
        bar.style.background = "var(--primary-color, #2E2EF9)";
        txt.style.color = "var(--primary-color, #2E2EF9)";
        if (hint) hint.textContent = "Fill in all your details to complete your profile.";
      }
    }
  }

  async function saveProfile() {
    const e = window.event || arguments[0] || (typeof event !== 'undefined' ? event : null);
    const btn = e && e.target;
    setButtonState(btn, true, "Saving...");

    const body = {
      first_name: getVal("settings-first-name"),
      last_name: getVal("settings-last-name"),
      phone_number: getVal("settings-phone"),
      country: getVal("settings-country"),
      timezone: getVal("settings-timezone"),
      date_of_birth: getVal("settings-dob"),
      nationality: getVal("settings-nationality"),
      address_line_1: getVal("settings-address-1"),
      address_line_2: getVal("settings-address-2"),
      city: getVal("settings-city"),
      state_province: getVal("settings-state"),
      postal_code: getVal("settings-postal"),
      tax_id: getVal("settings-tax-id"),
      annual_income_cents: getVal("settings-annual-income") ? parseInt(getVal("settings-annual-income")) * 100 : null,
    };

    // 1. Upload photo if pending
    if (pendingPhotoFile) {
      showToast("Uploading photo...", "info");
      const uploadRes = await SettingsDataService.uploadAvatar(pendingPhotoFile);
      if (uploadRes && uploadRes.avatar_url) {
        savedSettings.avatar_url = uploadRes.avatar_url;
      } else {
        showToast(uploadRes?.error || "Photo upload failed.", "error");
        setButtonState(btn, false, "Save");
        return;
      }
    }

    const res = await SettingsDataService.saveProfile(body);
    setButtonState(btn, false, "Save");

    if (res && res.success) {
      showToast(res.message, "success");
      pendingPhotoFile = null;
      pendingPhotoPreview = null;
      updateSidebarName(body.first_name, body.last_name);
      loadSettings(); // Refresh calculated limits
    } else if (res) {
      showToast(res.message || "Failed to save.", "error");
    }
  }

  function cancelProfile() {
    if (!savedSettings) return;
    populateMyDetails(savedSettings);
    showToast("Changes discarded.", "success");
  }

  function updateSidebarName(firstName, lastName) {
    const name = [firstName, lastName].filter(Boolean).join(" ") || "User";
    document.querySelectorAll(".user-name, .sidebar-user-name")
      .forEach((el) => { el.textContent = name; });
  }

  function handleProfilePhotoUpload(input) {
    if (!input.files || !input.files[0]) return;
    const file = input.files[0];

    if (!file.type.match("image.*")) {
      showToast("Please select a valid image file (JPG, PNG, SVG).", "error");
      return;
    }

    const reader = new FileReader();
    reader.onload = function (e) {
      const imgPreview = document.querySelector(".settings-photo-current img");
      if (imgPreview) imgPreview.src = e.target.result;

      document.querySelectorAll(
        ".profile-avatar img, .mobile-burger-menu__avatar img, .sidebar__account-avatar img"
      ).forEach((el) => { el.src = e.target.result; });

      pendingPhotoFile = file;
      pendingPhotoPreview = e.target.result;
      showToast("Photo selected. Click Save to apply changes.", "success");
    };
    reader.readAsDataURL(file);
  }

  // ─── Tab: Preferences ────────────────────────────────────────

  function populatePreferences(data) {
    const langSelect = document.getElementById("settings-language");
    if (langSelect && data.language) langSelect.value = data.language;

    const currSelect = document.getElementById("settings-currency");
    if (currSelect && data.currency) currSelect.value = data.currency;
  }

  async function savePreferences() {
    const e = window.event || arguments[0] || (typeof event !== 'undefined' ? event : null);
    const btn = e && e.target;
    setButtonState(btn, true, "Saving...");

    const body = {
      language: getVal("settings-language") || "en",
      currency: getVal("settings-currency") || "USD",
    };

    const res = await SettingsDataService.savePreferences(body);
    setButtonState(btn, false, "Save");

    if (res && res.success) {
      showToast(res.message, "success");
      savedSettings = { ...savedSettings, ...body };
    } else if (res) {
      showToast(res.message || "Failed to save.", "error");
    }
  }

  function cancelPreferences() {
    if (!savedSettings) return;
    populatePreferences(savedSettings);
    showToast("Changes discarded.", "success");
  }

  // ─── Tab: Notifications ───────────────────────────────────────

  function populateNotifications(data) {
    const notifyEmail = document.getElementById("settings-notify-email");
    if (notifyEmail) notifyEmail.checked = !!data.email_notifications;

    const notifyPush = document.getElementById("settings-notify-push");
    if (notifyPush) notifyPush.checked = !!data.push_notifications;
  }

  async function saveNotifications() {
    const e = window.event || arguments[0] || (typeof event !== 'undefined' ? event : null);
    const btn = e && e.target;
    setButtonState(btn, true, "Saving...");

    const body = {
      email_notifications: !!document.getElementById("settings-notify-email")?.checked,
      push_notifications: !!document.getElementById("settings-notify-push")?.checked,
    };

    const res = await SettingsDataService.saveNotifications(body);
    setButtonState(btn, false, "Save");

    if (res && res.success) {
      showToast(res.message, "success");
      savedSettings = { ...savedSettings, ...body };
    } else if (res) {
      showToast(res.message || "Failed to save notifications.", "error");
    }
  }

  function cancelNotifications() {
    if (!savedSettings) return;
    populateNotifications(savedSettings);
    showToast("Changes discarded.", "success");
  }

  // ─── Tab: Security ────────────────────────────────────────────

  function populateSecurity(data) {
    const secEmail = document.getElementById("settings-security-email");
    if (secEmail) secEmail.value = data.email || "";

    const verifiedBadge = document.getElementById("settings-email-verified");
    if (verifiedBadge) {
      if (data.email_verified) {
        verifiedBadge.textContent = "Verified";
        verifiedBadge.className = "settings-badge settings-badge--approved";
      } else {
        verifiedBadge.textContent = "Unverified";
        verifiedBadge.className = "settings-badge settings-badge--pending";
      }
    }

    const secPhone = document.getElementById("settings-security-phone");
    if (secPhone) secPhone.value = data.phone_number || "";

    // 2FA Badge & Actions
    const totpBadge = document.getElementById("settings-2fa-badge");
    const totpActions = document.getElementById("settings-2fa-actions");
    if (totpBadge && totpActions) {
      if (data.totp_enabled) {
        totpBadge.textContent = "Enabled";
        totpBadge.className = "settings-badge settings-badge--approved";
        totpActions.innerHTML = `<button class="settings-btn settings-btn--secondary" style="border-color: #FDA29B; color: #D92D20;" onclick="disable2FA()">Disable 2FA</button>`;
      } else {
        totpBadge.textContent = "Disabled";
        totpBadge.className = "settings-badge settings-badge--missing";
        totpActions.innerHTML = `<a href="/auth/2fa/setup" class="settings-btn settings-btn--primary">Enable 2FA</a>`;
      }
    }

    // KYC Details Badge in Security Tab
    const kycDetailBadge = document.getElementById("settings-kyc-detail-badge");
    const kycActionBtn = document.getElementById("settings-kyc-action-btn");
    if (kycDetailBadge && kycActionBtn) {
      const status = (data.kyc_status || "not_started").toLowerCase();
      const displayMap = {
        approved: "Verified",
        pending: "Pending Review",
        in_review: "In Review",
        rejected: "Rejected",
        missing: "Not Started",
        not_started: "Not Started",
      };
      kycDetailBadge.textContent = displayMap[status] || status.replace("_", " ");
      kycDetailBadge.className = `settings-badge settings-badge--${status}`;

      if (status === "approved") {
        kycActionBtn.textContent = "View status";
      } else if (status === "rejected") {
        kycActionBtn.textContent = "View reason";
      } else {
        kycActionBtn.textContent = "Start Verification";
      }
    }

    // Active Sessions
    const sessionsList = document.getElementById("settings-sessions-list");
    if (sessionsList) {
      sessionsList.innerHTML = "";
      const sessions = data.active_sessions || [];
      if (sessions.length === 0) {
        sessionsList.innerHTML = `<div class="settings-session-card"><div class="settings-session-card__info"><div class="settings-session-card__device">No active sessions found</div></div></div>`;
      } else {
        sessions.forEach(s => {
          const uAgent = s.user_agent || "Unknown device";
          const ip = s.ip_address || "Unknown IP";
          const dStr = s.created_at ? `Logged in ${s.created_at}` : "";
          const card = document.createElement("div");
          card.className = "settings-session-card";
          card.innerHTML = `
            <div class="settings-session-card__info">
              <div class="settings-session-card__device">
                ${escapeHtml(uAgent)}
                ${s.is_current ? `<span class="settings-session-card__current">Current</span>` : ""}
              </div>
              <div class="settings-session-card__meta">${escapeHtml(ip)} &bull; ${escapeHtml(dStr)}</div>
            </div>
            <button class="settings-btn settings-btn--secondary" style="font-size: 12px; padding: 6px 10px;" aria-label="Revoke session">Revoke</button>
          `;
          sessionsList.appendChild(card);
        });
      }
    }

    // Linked OAuth Accounts
    const oauthList = document.getElementById("settings-oauth-list");
    if (oauthList) {
      oauthList.innerHTML = "";
      const oauths = data.oauth_accounts || [];
      if (oauths.length === 0) {
        oauthList.innerHTML = `<div class="settings-oauth-card"><div class="settings-oauth-card__provider"><span class="settings-oauth-card__provider-name">No linked accounts</span></div></div>`;
      } else {
        oauths.forEach(o => {
          const card = document.createElement("div");
          card.className = "settings-oauth-card";
          card.innerHTML = `
            <div class="settings-oauth-card__provider">
              <span class="settings-oauth-card__provider-name">${escapeHtml(o.provider)}</span>
              ${o.provider_email ? `<span class="settings-oauth-card__email">${escapeHtml(o.provider_email)}</span>` : ""}
            </div>
            <span class="settings-oauth-card__date">Linked ${escapeHtml(o.created_at || "")}</span>
          `;
          oauthList.appendChild(card);
        });
      }
    }

    // Consent Management
    const termsVersionEl = document.getElementById("settings-terms-version");
    const termsDateEl = document.getElementById("settings-terms-date");
    if (termsVersionEl && termsDateEl) {
      if (data.latest_terms_version) {
        termsVersionEl.textContent = `Terms Version: ${data.latest_terms_version}`;
        termsDateEl.textContent = `Accepted on: ${data.latest_terms_accepted_at}`;
      } else {
        termsVersionEl.textContent = `Terms Version: 1.0`;
        termsDateEl.textContent = `Legacy user — please review latest terms`;
      }
    }
  }

  /** Escape HTML to prevent XSS in dynamic content */
  function escapeHtml(str) {
    if (!str) return "";
    const div = document.createElement("div");
    div.textContent = str;
    return div.innerHTML;
  }

  async function disable2FA() {
    if (!await pooolConfirm({ title: 'Disable Two-Factor Authentication', message: 'This will make your account less secure. Are you sure?', confirmText: 'Disable 2FA', type: 'danger' })) return;
    const e = window.event || arguments[0] || (typeof event !== 'undefined' ? event : null);
    const btn = e && e.target;
    setButtonState(btn, true, "Disabling...");

    const res = await SettingsDataService.disable2FA();
    setButtonState(btn, false, "Disable 2FA");

    if (res && res.success) {
      showToast(res.message, "success");
      if (savedSettings) savedSettings.totp_enabled = false;
      populateSecurity(savedSettings || { totp_enabled: false });
    } else {
      showToast((res && res.message) || "Failed to disable 2FA.", "error");
    }
  }

  async function exportData() {
    showToast("Preparing data export...", "info");
    const res = await SettingsDataService.requestDataExport();
    if (res.success) {
      showToast(res.message, "success");
    } else {
      showToast("Failed to request export.", "error");
    }
  }

  // ─── Modals ──────────────────────────────────────────────────

  function openModal(id) {
    const modal = document.getElementById(id);
    if (!modal) return;
    modal.style.display = "flex";
    modal.classList.add("active");
    modal.setAttribute("aria-modal", "true");
    modal.setAttribute("role", "dialog");
    document.body.style.overflow = "hidden";
    const firstInput = modal.querySelector("input");
    if (firstInput) setTimeout(() => firstInput.focus(), 100);
  }

  function closeModal(id) {
    const modal = document.getElementById(id);
    if (!modal) return;
    modal.style.display = "none";
    modal.classList.remove("active");
    modal.removeAttribute("aria-modal");
    document.body.style.overflow = "";
    modal.querySelectorAll("input").forEach((inp) => (inp.value = ""));
    modal.querySelectorAll(".settings-modal__error")
      .forEach((el) => (el.textContent = ""));
  }

  const openChangeEmailModal = () => openModal("modal-change-email");
  const openChangePasswordModal = () => openModal("modal-change-password");
  const openChangePhoneModal = () => openModal("modal-change-phone");

  // ─── Modal: Change Email ──────────────────────────────────────

  async function submitChangeEmail() {
    const newEmail = getVal("modal-new-email");
    const password = getVal("modal-email-password");
    const errorEl = document.getElementById("modal-email-error");
    const btn = document.getElementById("modal-email-submit");

    setButtonState(btn, true, "Saving...");
    const res = await SettingsDataService.changeEmail(newEmail, password);
    setButtonState(btn, false, "Save");

    if (!res) return;
    if (res.success) {
      showToast(res.message, "success");
      closeModal("modal-change-email");
      savedSettings.email = newEmail;
      populateSecurity(savedSettings);
      populateMyDetails(savedSettings);
    } else {
      if (errorEl) errorEl.textContent = res.message;
    }
  }

  // ─── Modal: Change Password ───────────────────────────────────

  async function submitChangePassword() {
    const current = getVal("modal-current-password");
    const newPw = getVal("modal-new-password");
    const confirm = getVal("modal-confirm-password");
    const errorEl = document.getElementById("modal-password-error");
    const btn = document.getElementById("modal-password-submit");

    setButtonState(btn, true, "Saving...");
    const res = await SettingsDataService.changePassword(current, newPw, confirm);
    setButtonState(btn, false, "Save");

    if (!res) return;
    if (res.success) {
      showToast(res.message, "success");
      closeModal("modal-change-password");
    } else {
      if (errorEl) errorEl.textContent = res.message;
    }
  }

  // ─── Modal: Change Phone ──────────────────────────────────────

  async function submitChangePhone() {
    const newPhone = getVal("modal-new-phone");
    const errorEl = document.getElementById("modal-phone-error");
    const btn = document.getElementById("modal-phone-submit");

    setButtonState(btn, true, "Saving...");
    const res = await SettingsDataService.changePhone(newPhone);
    setButtonState(btn, false, "Save");

    if (!res) return;
    if (res.success) {
      showToast(res.message, "success");
      closeModal("modal-change-phone");
      savedSettings.phone_number = newPhone;
      populateSecurity(savedSettings);
    } else {
      if (errorEl) errorEl.textContent = res.message;
    }
  }

  // ─── Tab Switching with ARIA ──────────────────────────────────

  function switchTab(tab) {
    document.querySelectorAll(".settings-tab")
      .forEach((t) => {
        t.classList.remove("active");
        t.setAttribute("aria-selected", "false");
      });
    document.querySelectorAll(".settings-panel")
      .forEach((p) => p.classList.remove("active"));

    const tabEl = document.getElementById("tab-" + tab);
    const panelEl = document.getElementById("panel-" + tab);
    if (tabEl) {
      tabEl.classList.add("active");
      tabEl.setAttribute("aria-selected", "true");
    }
    if (panelEl) panelEl.classList.add("active");

    const titles = {
      mydetails: "Settings – My Details",
      preferences: "Settings – Preferences",
      notifications: "Settings – Notifications",
      security: "Settings – Security & Privacy",
      more: "Settings – More",
    };
    document.title = titles[tab] || "Settings – POOOL";
  }

  // ─── Initialise ARIA on tabs ──────────────────────────────────

  function initTabARIA() {
    const tabContainer = document.querySelector(".settings-tabs-container");
    if (tabContainer) {
      tabContainer.setAttribute("role", "tablist");
      tabContainer.setAttribute("aria-label", "Settings sections");
    }
    document.querySelectorAll(".settings-tab").forEach(t => {
      t.setAttribute("role", "tab");
      const tabId = t.id.replace("tab-", "");
      t.setAttribute("aria-controls", "panel-" + tabId);
      t.setAttribute("aria-selected", t.classList.contains("active") ? "true" : "false");
    });
    document.querySelectorAll(".settings-panel").forEach(p => {
      p.setAttribute("role", "tabpanel");
      const panelId = p.id.replace("panel-", "");
      p.setAttribute("aria-labelledby", "tab-" + panelId);
    });
  }

  // ─── DOM Helpers ─────────────────────────────────────────────

  function setVal(id, value) {
    const el = document.getElementById(id);
    if (el) el.value = value;
  }

  function getVal(id) {
    const el = document.getElementById(id);
    return el ? el.value.trim() : "";
  }

  function setButtonState(btn, disabled, text) {
    if (!btn) return;
    btn.disabled = disabled;
    btn.textContent = text;
  }

  // ─── Keyboard & Overlay ──────────────────────────────────────

  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      ["modal-change-email", "modal-change-password", "modal-change-phone"]
        .forEach(closeModal);
    }
  });

  document.addEventListener("click", function (e) {
    if (e.target.classList.contains("settings-modal-overlay")) {
      const id = e.target.id;
      if (id) closeModal(id);
    }
  });

  // ─── Expose to HTML onclick handlers ─────────────────────────

  window.saveProfile = saveProfile;
  window.cancelProfile = cancelProfile;
  window.savePreferences = savePreferences;
  window.cancelPreferences = cancelPreferences;
  window.saveNotifications = saveNotifications;
  window.cancelNotifications = cancelNotifications;
  window.openChangeEmailModal = openChangeEmailModal;
  window.openChangePasswordModal = openChangePasswordModal;
  window.openChangePhoneModal = openChangePhoneModal;
  window.submitChangeEmail = submitChangeEmail;
  window.submitChangePassword = submitChangePassword;
  window.submitChangePhone = submitChangePhone;
  window.closeModal = closeModal;
  window.handleProfilePhotoUpload = handleProfilePhotoUpload;
  window.disable2FA = disable2FA;

  // ─── Event Listeners (CSP Compliant) ────────────────────────

  function setupListeners() {
    // Tabs
    document.querySelectorAll('.settings-tab').forEach(btn => {
      btn.addEventListener('click', () => {
        const tabId = btn.id.replace('tab-', '');
        switchTab(tabId);
      });
    });

    // Profile Save/Cancel
    document.querySelectorAll('.btn-save-profile').forEach(btn => btn.addEventListener('click', saveProfile));
    document.querySelectorAll('.btn-cancel-profile').forEach(btn => btn.addEventListener('click', cancelProfile));
    
    // Photo upload
    const photoUploadBtn = document.getElementById('btn-photo-upload');
    if (photoUploadBtn) {
      photoUploadBtn.addEventListener('click', () => {
        const input = document.getElementById('profile-photo-upload');
        if (input) input.click();
      });
    }

    const photoInput = document.getElementById('profile-photo-upload');
    if (photoInput) {
      photoInput.addEventListener('change', (e) => handleProfilePhotoUpload(e.target));
    }

    // Preferences
    const savePref = document.getElementById('btn-save-preferences');
    if (savePref) savePref.addEventListener('click', savePreferences);
    const cancelPref = document.getElementById('btn-cancel-preferences');
    if (cancelPref) cancelPref.addEventListener('click', cancelPreferences);

    // Notifications
    const saveNotify = document.getElementById('btn-save-notifications');
    if (saveNotify) saveNotify.addEventListener('click', saveNotifications);
    const cancelNotify = document.getElementById('btn-cancel-notifications');
    if (cancelNotify) cancelNotify.addEventListener('click', cancelNotifications);

    // Security Actions
    const changeEmail = document.getElementById('btn-change-email');
    if (changeEmail) changeEmail.addEventListener('click', openChangeEmailModal);
    const changePass = document.getElementById('btn-change-password');
    if (changePass) changePass.addEventListener('click', openChangePasswordModal);
    const changePhone = document.getElementById('btn-change-phone');
    if (changePhone) changePhone.addEventListener('click', openChangePhoneModal);
    const exportBtn = document.getElementById('btn-export-data');
    if (exportBtn) exportBtn.addEventListener('click', exportData);

    // Modals
    const emailSubmit = document.getElementById('modal-email-submit-btn');
    if (emailSubmit) emailSubmit.addEventListener('click', submitChangeEmail);
    
    document.querySelectorAll('.btn-close-email-modal').forEach(btn => {
      btn.addEventListener('click', () => closeModal('modal-change-email'));
    });
    
    // Missing IDs for other modals' Save/Cancel buttons in HTML edit — adding listeners by selector
    document.querySelector('[onclick="submitChangePassword()"]')?.addEventListener('click', submitChangePassword);
    document.querySelector('[onclick="closeModal(\'modal-change-password\')"]')?.addEventListener('click', () => closeModal('modal-change-password'));
    document.querySelector('[onclick="submitChangePhone()"]')?.addEventListener('click', submitChangePhone);
    document.querySelector('[onclick="closeModal(\'modal-change-phone\')"]')?.addEventListener('click', () => closeModal('modal-change-phone'));
  }

  // ─── Boot ────────────────────────────────────────────────────

  function boot() {
    initTabARIA();
    setupListeners();
    loadSettings();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
