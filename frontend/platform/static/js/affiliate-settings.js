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
    // Phase-5 add-ons
    loadWebhooks();
    wireWebhookForm();
    loadPayoutMethods();
    wirePayoutMethodForm();
  });

  // ═══════════════════════════════════════════════════════════════════
  // Phase-5: Webhook subscriptions manager
  // ═══════════════════════════════════════════════════════════════════
  async function loadWebhooks() {
    const tbody = document.getElementById('aff-webhook-list-body');
    if (!tbody) return;
    try {
      const res = await fetch('/api/affiliate/webhooks', { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      const items = data.items || [];
      if (!items.length) {
        tbody.innerHTML = '<tr><td colspan="5" style="padding:18px;text-align:center;color:#98A2B3;">No webhooks configured.</td></tr>';
        return;
      }
      tbody.innerHTML = items.map(function (w) {
        const safeUrl = String(w.url).replace(/[<>&"]/g, '');
        const events = String(w.event_types).replace(/[<>&"]/g, '');
        const last = w.last_success_at
          ? new Date(w.last_success_at).toLocaleString()
          : (w.last_failure_at
              ? '<span style="color:#B42318;">Failed ' + new Date(w.last_failure_at).toLocaleString() + '</span>'
              : '<span style="color:#98A2B3;">Never</span>');
        const status = w.is_active
          ? (w.failure_count > 5
              ? '<span style="color:#B54708;">Degraded</span>'
              : '<span style="color:#027A48;">Active</span>')
          : '<span style="color:#98A2B3;">Disabled</span>';
        return '<tr style="border-bottom:1px solid #F2F4F7;">'
          + '<td style="padding:10px;font-family:ui-monospace,monospace;font-size:11px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">' + safeUrl + '</td>'
          + '<td style="padding:10px;">' + events + '</td>'
          + '<td style="padding:10px;font-size:12px;">' + last + '</td>'
          + '<td style="padding:10px;">' + status + '</td>'
          + '<td style="padding:10px;text-align:right;">'
            + (w.is_active
                ? '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-webhook-test="' + w.id + '" title="Send a synthetic webhook_test event">Test fire</button> '
                  + '<button type="button" class="ds-btn ds-btn--secondary ds-btn--sm" data-webhook-delete="' + w.id + '">Disable</button>'
                : '')
          + '</td>'
          + '</tr>';
      }).join('');
      tbody.querySelectorAll('[data-webhook-delete]').forEach(function (btn) {
        btn.addEventListener('click', async function () {
          if (!confirm('Disable this webhook? Queued retries will drain and stop firing.')) return;
          const id = btn.getAttribute('data-webhook-delete');
          const res = await fetch('/api/affiliate/webhooks/' + encodeURIComponent(id), {
            method: 'DELETE',
            credentials: 'same-origin',
          });
          if (res.ok) loadWebhooks();
        });
      });
      // Phase-7: test-fire button. Queues a synthetic webhook_test event
      // via the same outbox path; consumer verifies their signature
      // handler without waiting for a real conversion.
      tbody.querySelectorAll('[data-webhook-test]').forEach(function (btn) {
        btn.addEventListener('click', async function () {
          const id = btn.getAttribute('data-webhook-test');
          btn.disabled = true;
          const original = btn.textContent;
          btn.textContent = 'Queuing…';
          try {
            const res = await fetch('/api/affiliate/webhooks/' + encodeURIComponent(id) + '/test', {
              method: 'POST',
              credentials: 'same-origin',
            });
            const data = await res.json().catch(function () { return {}; });
            if (res.ok) {
              btn.textContent = 'Queued ✓';
              if (window.showPooolToast) window.showPooolToast('Test fired', data.message || 'Synthetic event en route.', 'success');
              setTimeout(function () { btn.textContent = original; btn.disabled = false; }, 3000);
            } else if (res.status === 429) {
              btn.textContent = 'Wait 60s';
              if (window.showPooolToast) window.showPooolToast('Rate-limited', 'One test per minute per webhook.', 'warning');
              setTimeout(function () { btn.textContent = original; btn.disabled = false; }, 60000);
            } else {
              btn.textContent = 'Failed';
              setTimeout(function () { btn.textContent = original; btn.disabled = false; }, 3000);
            }
          } catch (_) {
            btn.textContent = original;
            btn.disabled = false;
          }
        });
      });
    } catch (_) {
      tbody.innerHTML = '<tr><td colspan="5" style="padding:18px;text-align:center;color:#B42318;">Failed to load webhooks.</td></tr>';
    }
  }
  function wireWebhookForm() {
    const newBtn = document.getElementById('aff-webhook-new-btn');
    const form = document.getElementById('aff-webhook-form');
    const cancel = document.getElementById('aff-webhook-cancel');
    if (!newBtn || !form) return;
    newBtn.addEventListener('click', function () {
      form.hidden = false;
      form.querySelector('#aff-webhook-url').focus();
    });
    cancel?.addEventListener('click', function () {
      form.hidden = true;
      form.reset();
      document.getElementById('aff-webhook-error').textContent = '';
    });
    form.addEventListener('submit', async function (e) {
      e.preventDefault();
      const errEl = document.getElementById('aff-webhook-error');
      errEl.textContent = '';
      const url = document.getElementById('aff-webhook-url').value.trim();
      const events = document.getElementById('aff-webhook-events').value.trim() || '*';
      const desc = document.getElementById('aff-webhook-desc').value.trim();
      try {
        const res = await fetch('/api/affiliate/webhooks', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ url: url, event_types: events, description: desc || null }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) { errEl.textContent = data.error || 'Create failed.'; return; }
        form.reset();
        form.hidden = true;
        const panel = document.getElementById('aff-webhook-secret-panel');
        const val = document.getElementById('aff-webhook-secret-value');
        if (val) val.textContent = data.secret || '(missing)';
        if (panel) panel.hidden = false;
        loadWebhooks();
      } catch (_) {
        errEl.textContent = 'Network error.';
      }
    });
    document.getElementById('aff-webhook-secret-dismiss')?.addEventListener('click', function () {
      const panel = document.getElementById('aff-webhook-secret-panel');
      const val = document.getElementById('aff-webhook-secret-value');
      if (panel) panel.hidden = true;
      if (val) val.textContent = '';
    });
    document.getElementById('aff-webhook-secret-copy')?.addEventListener('click', async function () {
      const val = document.getElementById('aff-webhook-secret-value');
      if (!val) return;
      try {
        await navigator.clipboard.writeText(val.textContent || '');
        if (window.showPooolToast) window.showPooolToast('Copied', 'Secret on clipboard.', 'success');
      } catch (_) {}
    });
  }

  // ═══════════════════════════════════════════════════════════════════
  // Phase-5: Payout-method manager
  // ═══════════════════════════════════════════════════════════════════
  async function loadPayoutMethods() {
    const tbody = document.getElementById('aff-payout-method-list');
    if (!tbody) return;
    try {
      const res = await fetch('/api/affiliate/payout-methods', { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      const items = data.items || [];
      if (!items.length) {
        tbody.innerHTML = '<tr><td colspan="4" style="padding:18px;text-align:center;color:#98A2B3;">No payout methods configured. Add one to receive commission payouts.</td></tr>';
        return;
      }
      const TYPE_LABELS = {
        sepa_iban: 'SEPA bank transfer',
        paypal_email: 'PayPal',
        wise_email: 'Wise',
        usdc_wallet: 'USDC (on-chain)',
        stripe_connect: 'Stripe Connect',
      };
      tbody.innerHTML = items.map(function (m) {
        const safeId = String(m.identifier_masked).replace(/[<>&"]/g, '');
        const label = m.label ? ' · ' + String(m.label).replace(/[<>&"]/g, '') : '';
        const def = m.is_default
          ? '<span style="background:#EFF3FF;color:#0000FF;padding:2px 8px;border-radius:999px;font-size:11px;font-weight:600;">Default</span>'
          : '';
        return '<tr style="border-bottom:1px solid #F2F4F7;">'
          + '<td style="padding:10px;font-weight:600;">' + (TYPE_LABELS[m.method_type] || m.method_type) + label + ' ' + def + '</td>'
          + '<td style="padding:10px;font-family:ui-monospace,monospace;font-size:12px;">' + safeId + '</td>'
          + '<td style="padding:10px;font-size:12px;">' + (m.is_active ? 'Active' : 'Disabled') + '</td>'
          + '<td style="padding:10px;text-align:right;">'
            + (m.is_active && !m.is_default
                ? '<button type="button" class="ds-btn ds-btn--secondary ds-btn--sm" data-method-default="' + m.id + '">Make default</button> '
                : '')
            + '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-method-delete="' + m.id + '">Remove</button>'
          + '</td>'
          + '</tr>';
      }).join('');
      tbody.querySelectorAll('[data-method-default]').forEach(function (btn) {
        btn.addEventListener('click', async function () {
          const id = btn.getAttribute('data-method-default');
          const res = await fetch('/api/affiliate/payout-methods/' + encodeURIComponent(id) + '/default', {
            method: 'POST', credentials: 'same-origin',
          });
          if (res.ok) loadPayoutMethods();
          else if (res.status === 401 || res.status === 403) {
            alert('2FA step-up required. Complete 2FA verification first.');
          }
        });
      });
      tbody.querySelectorAll('[data-method-delete]').forEach(function (btn) {
        btn.addEventListener('click', async function () {
          if (!confirm('Remove this payout method?')) return;
          const id = btn.getAttribute('data-method-delete');
          const res = await fetch('/api/affiliate/payout-methods/' + encodeURIComponent(id), {
            method: 'DELETE', credentials: 'same-origin',
          });
          if (res.ok) loadPayoutMethods();
        });
      });
    } catch (_) { /* silent */ }
  }
  function wirePayoutMethodForm() {
    const form = document.getElementById('aff-payout-method-form');
    if (!form) return;
    form.addEventListener('submit', async function (e) {
      e.preventDefault();
      const errEl = document.getElementById('aff-payout-method-error');
      errEl.textContent = '';
      const body = {
        method_type: document.getElementById('aff-payout-method-type').value,
        identifier: document.getElementById('aff-payout-method-identifier').value.trim(),
        label: document.getElementById('aff-payout-method-label').value.trim() || null,
        make_default: document.getElementById('aff-payout-method-default').checked,
      };
      try {
        const res = await fetch('/api/affiliate/payout-methods', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        if (!res.ok) {
          const j = await res.json().catch(() => ({}));
          errEl.textContent = j.error || 'Create failed.';
          return;
        }
        form.reset();
        loadPayoutMethods();
      } catch (_) {
        errEl.textContent = 'Network error.';
      }
    });
  }
})();
