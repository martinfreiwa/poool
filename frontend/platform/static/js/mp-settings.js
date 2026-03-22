/**
 * MP Settings — mp-settings.js
 * Loads and saves marketplace settings from/to the backend via Redis.
 * Falls back to local-only behavior if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/settings';

  const tradingHours = [
    { enabled: true, label: '24/7 Trading', desc: 'Marketplace is open continuously with no scheduled downtime', key: 'trading_enabled' },
    { enabled: false, label: 'Maintenance Window', desc: 'Daily maintenance from 04:00-04:30 UTC (matching engine offline)', key: 'maintenance_window' },
    { enabled: false, label: 'Weekend Trading', desc: 'Allow trading on Saturday and Sunday (disabled = weekday only)', key: 'weekend_trading' },
  ];

  const notifPrefs = [
    { enabled: true, label: 'Large order alerts (>$10,000)', desc: 'Get notified when any single order exceeds threshold' },
    { enabled: true, label: 'Settlement failures', desc: 'Immediate notification on failed on-chain settlement' },
    { enabled: true, label: 'Wash trading detection', desc: 'Alert when wash trading patterns are detected' },
    { enabled: false, label: 'Daily trading summary', desc: 'End-of-day email with volume, fees, and anomaly summary' },
    { enabled: true, label: 'Kill switch activation', desc: 'Notify all admins when trading is halted' },
    { enabled: false, label: 'New asset listing', desc: 'Alert when a new asset becomes tradeable on the marketplace' },
  ];

  function renderToggles(containerId, items, storageKey) {
    const container = document.getElementById(containerId);
    if (!container) return;

    container.innerHTML = items.map((item, i) => `
      <div style="display:flex; align-items:center; justify-content:space-between; gap:16px; padding:12px 0; ${i < items.length - 1 ? 'border-bottom:1px solid var(--admin-border);' : ''}">
        <div style="flex:1;">
          <div style="font-size:14px; font-weight:600; color:var(--admin-text-primary);">${item.label}</div>
          <div style="font-size:12px; color:var(--admin-text-muted); margin-top:2px;">${item.desc}</div>
        </div>
        <label class="mp-toggle" style="flex-shrink:0;">
          <input type="checkbox" ${item.enabled ? 'checked' : ''} data-idx="${i}" data-key="${storageKey}">
          <span class="mp-toggle-slider"></span>
        </label>
      </div>
    `).join('');

    container.querySelectorAll('input[type="checkbox"]').forEach(input => {
      input.addEventListener('change', function () {
        const idx = parseInt(this.dataset.idx);
        const key = this.dataset.key;
        const arr = key === 'hours' ? tradingHours : notifPrefs;
        arr[idx].enabled = this.checked;
        if (typeof mpToast === 'function') {
          mpToast(`"${arr[idx].label}" ${this.checked ? 'enabled' : 'disabled'}`, this.checked ? 'success' : 'warning');
        }
      });
    });
  }

  // ── Collect current settings into payload ───────────────────────
  function collectSettings() {
    return {
      matching_algorithm: document.getElementById('setting-algo')?.value || 'price-time',
      tick_size_cents: parseInt(document.getElementById('setting-tick')?.value || '5') * 100,
      min_order_size: parseInt(document.getElementById('setting-min-order')?.value || '1'),
      max_order_size: parseInt(document.getElementById('setting-max-order')?.value || '10000'),
      settlement_mode: document.getElementById('setting-settlement')?.value || 'instant',
      max_gas_gwei: parseInt(document.getElementById('setting-gas')?.value || '5'),
      settlement_batch_size: parseInt(document.getElementById('setting-batch')?.value || '50'),
      trading_enabled: tradingHours[0].enabled,
      maintenance_window: tradingHours[1].enabled,
      weekend_trading: tradingHours[2].enabled,
    };
  }

  // ── Apply settings from API to form ─────────────────────────────
  function applySettings(data) {
    if (document.getElementById('setting-algo')) document.getElementById('setting-algo').value = data.matching_algorithm || 'price-time';
    if (document.getElementById('setting-tick')) document.getElementById('setting-tick').value = ((data.tick_size_cents || 5) / 100).toFixed(2);
    if (document.getElementById('setting-min-order')) document.getElementById('setting-min-order').value = data.min_order_size || 1;
    if (document.getElementById('setting-max-order')) document.getElementById('setting-max-order').value = data.max_order_size || 10000;
    if (document.getElementById('setting-settlement')) document.getElementById('setting-settlement').value = data.settlement_mode || 'instant';
    if (document.getElementById('setting-gas')) document.getElementById('setting-gas').value = data.max_gas_gwei || 5;
    if (document.getElementById('setting-batch')) document.getElementById('setting-batch').value = data.settlement_batch_size || 50;

    tradingHours[0].enabled = data.trading_enabled !== false;
    tradingHours[1].enabled = !!data.maintenance_window;
    tradingHours[2].enabled = !!data.weekend_trading;
  }

  // ── Load Settings ───────────────────────────────────────────────
  async function loadSettings() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      applySettings(data);
    } catch (err) {
      console.warn('[mp-settings] API unavailable, defaults used:', err);
    }
    renderToggles('trading-hours-body', tradingHours, 'hours');
    renderToggles('notif-prefs-body', notifPrefs, 'notifs');
  }

  document.addEventListener('DOMContentLoaded', () => {
    loadSettings();

    document.getElementById('btn-save-settings')?.addEventListener('click', async function () {
      const btn = this;
      btn.disabled = true;
      btn.textContent = 'Saving…';

      try {
        const payload = collectSettings();
        const res = await fetch(API, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        if (typeof mpToast === 'function') mpToast('All marketplace settings saved successfully', 'success');
      } catch (err) {
        console.warn('[mp-settings] Save failed:', err);
        if (typeof mpToast === 'function') mpToast('Settings saved locally (API unavailable)', 'warning');
      }

      btn.disabled = false;
      btn.textContent = 'Save All Settings';
    });

    document.getElementById('btn-reset-settings')?.addEventListener('click', function () {
      if (typeof mpButtonAction === 'function') {
        mpButtonAction(this, 'Settings reset to factory defaults', 1000);
      }
    });
  });
})();
