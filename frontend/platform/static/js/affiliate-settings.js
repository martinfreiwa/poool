(function () {
  "use strict";

  function csrfToken() {
    if (typeof window.getCsrfToken === "function") return window.getCsrfToken() || "";
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    return parts.length === 2 ? parts.pop().split(";").shift() : "";
  }

  function csrfHeaders(headers) {
    const token = csrfToken();
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value || "";
  }

  function setMessage(type, message) {
    const el = document.getElementById("affiliate-settings-message");
    if (!el) return;
    el.hidden = !message;
    el.classList.toggle("is-success", type === "success");
    el.classList.toggle("is-error", type === "error");
    el.textContent = message || "";
  }

  function setBadge(id, label) {
    const badge = document.getElementById(id);
    if (!badge) return;
    badge.textContent = label || "Unknown";
    badge.classList.toggle("is-success", label === "Active" || label === "Verified");
    badge.classList.toggle("is-warning", label === "On hold" || label === "Pending review" || label === "Under review");
    badge.classList.toggle("is-error", label === "Suspended" || label === "Incomplete");
  }

  function normalizeOptional(value) {
    const trimmed = String(value || "").trim();
    return trimmed ? trimmed : null;
  }

  document.addEventListener("DOMContentLoaded", () => {
    const form = document.getElementById("affiliate-settings-form");
    const saveBtn = document.getElementById("save-settings-btn");
    const certify = document.getElementById("tax_certify");
    const taxIdInput = document.getElementById("tax_id");
    const taxClass = document.getElementById("tax_class");
    const taxName = document.getElementById("tax_name");
    const vatNumber = document.getElementById("vat_number");
    const payoutMethod = document.getElementById("payout_method");
    const payoutHoldReason = document.getElementById("payout_hold_reason");

    if (!form || !saveBtn || !certify || !taxClass || !taxName || !vatNumber || !payoutMethod) {
      return;
    }

    const defaultButtonHtml = saveBtn.innerHTML;
    let dirty = false;
    let loaded = false;

    function updateSaveState() {
      saveBtn.disabled = !dirty || !certify.checked || !form.checkValidity();
    }

    function markDirty() {
      if (!loaded) return;
      dirty = true;
      setMessage("", "");
      updateSaveState();
    }

    function applySettings(settings) {
      if (!settings) return;
      taxClass.value = settings.tax_class || "";
      taxName.value = settings.tax_name || "";
      vatNumber.value = settings.vat_number || "";
      payoutMethod.value = settings.payout_method || "poool_wallet";
      if (taxIdInput) taxIdInput.value = "";
      setText("tax_id_masked", settings.tax_id_masked || "Not on file");
      setBadge("tax_status_badge", settings.tax_status);
      setBadge("payout_status_badge", settings.payout_status);

      if (payoutHoldReason) {
        payoutHoldReason.hidden = !settings.payout_hold_reason;
        payoutHoldReason.textContent = settings.payout_hold_reason || "";
      }

      dirty = false;
      certify.checked = false;
      loaded = true;
      updateSaveState();
    }

    async function loadSettings() {
      setMessage("", "Loading affiliate settings...");
      try {
        const res = await fetch("/api/affiliate/settings", {
          credentials: "include",
          headers: { "Accept": "application/json" },
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          throw new Error(data.error || "Failed to load affiliate settings.");
        }
        applySettings(data.settings);
        setMessage("", "");
      } catch (err) {
        loaded = true;
        updateSaveState();
        setMessage("error", err.message || "Failed to load affiliate settings.");
      }
    }

    form.addEventListener("input", markDirty);
    form.addEventListener("change", markDirty);

    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      if (!form.reportValidity()) {
        updateSaveState();
        return;
      }
      if (!certify.checked) {
        setMessage("error", "Confirm the tax certification before saving.");
        updateSaveState();
        return;
      }

      saveBtn.disabled = true;
      saveBtn.textContent = "Saving...";

      const payload = {
        tax_class: taxClass.value,
        tax_id: taxIdInput ? normalizeOptional(taxIdInput.value) : null,
        tax_name: taxName.value.trim(),
        vat_number: normalizeOptional(vatNumber.value),
        payout_method: payoutMethod.value,
        tax_certified: certify.checked,
      };

      try {
        const res = await fetch("/api/affiliate/settings", {
          method: "POST",
          credentials: "include",
          headers: csrfHeaders({
            "Accept": "application/json",
            "Content-Type": "application/json",
          }),
          body: JSON.stringify(payload),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          throw new Error(data.error || "Failed to update settings.");
        }
        applySettings(data.settings);
        setMessage("success", "Settings saved. Tax changes are pending compliance review before payouts resume.");
      } catch (err) {
        setMessage("error", err.message || "Failed to update settings.");
      } finally {
        saveBtn.innerHTML = defaultButtonHtml;
        updateSaveState();
      }
    });

    loadSettings();
  });
})();
