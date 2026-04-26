/**
 * MP Settings — mp-settings.js
 * Loads and saves supported marketplace settings from/to the backend.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/settings';
  const DEFAULT_SETTINGS = {
    matching_algorithm: 'price-time',
    tick_size_cents: 5,
    min_order_size: 1,
    max_order_size: 10000,
    settlement_mode: 'instant',
    max_gas_gwei: 5,
    settlement_batch_size: 50,
    trading_enabled: true,
    maintenance_window: false,
    weekend_trading: false,
  };

  const tradingHours = [
    { enabled: true, label: '24/7 Trading', desc: 'Marketplace is open continuously with no scheduled downtime', key: 'trading_enabled' },
    { enabled: false, label: 'Maintenance Window', desc: 'Daily maintenance from 04:00-04:30 UTC (matching engine offline)', key: 'maintenance_window' },
    { enabled: false, label: 'Weekend Trading', desc: 'Allow trading on Saturday and Sunday (disabled = weekday only)', key: 'weekend_trading' },
  ];

  function csrfToken() {
    if (typeof window.getCsrfToken === 'function') return window.getCsrfToken();
    const value = `; ${document.cookie}`;
    const parts = value.split('; csrf_token=');
    return parts.length === 2 ? parts.pop().split(';').shift() : '';
  }

  function csrfHeaders(headers = {}) {
    const token = csrfToken();
    return token ? { ...headers, 'X-CSRF-Token': token } : headers;
  }

  function setFieldValue(id, value) {
    const field = document.getElementById(id);
    if (field) field.value = value;
  }

  function parsePositiveInt(id, label) {
    const raw = document.getElementById(id)?.value || '';
    if (!/^\d+$/.test(raw.trim())) {
      throw new Error(`${label} must be a positive whole number.`);
    }
    const value = Number(raw);
    if (!Number.isSafeInteger(value) || value < 1) {
      throw new Error(`${label} must be a positive whole number.`);
    }
    return value;
  }

  function parseUsdToCents(id, label) {
    const raw = (document.getElementById(id)?.value || '').trim();
    const match = raw.match(/^(\d+)(?:\.(\d{1,2}))?$/);
    if (!match) {
      throw new Error(`${label} must be a dollar amount with at most two decimals.`);
    }

    const dollars = Number(match[1]);
    const cents = Number((match[2] || '').padEnd(2, '0'));
    if (!Number.isSafeInteger(dollars) || !Number.isSafeInteger(cents)) {
      throw new Error(`${label} is too large.`);
    }

    const total = dollars * 100 + cents;
    if (!Number.isSafeInteger(total) || total < 1) {
      throw new Error(`${label} must be at least $0.01.`);
    }
    return total;
  }

  function formatCents(cents) {
    const safeCents = Number.isSafeInteger(Number(cents)) ? Number(cents) : DEFAULT_SETTINGS.tick_size_cents;
    const dollars = Math.floor(safeCents / 100);
    const remainder = String(safeCents % 100).padStart(2, '0');
    return `${dollars}.${remainder}`;
  }

  function renderToggles(containerId, items) {
    const container = document.getElementById(containerId);
    if (!container) return;

    container.replaceChildren();

    items.forEach((item, index) => {
      const row = document.createElement('div');
      row.className = 'mp-settings-toggle-row';
      row.style.display = 'flex';
      row.style.alignItems = 'center';
      row.style.justifyContent = 'space-between';
      row.style.gap = '16px';
      row.style.padding = '12px 0';
      if (index < items.length - 1) row.style.borderBottom = '1px solid var(--admin-border)';

      const textWrap = document.createElement('div');
      textWrap.style.flex = '1';

      const labelText = document.createElement('div');
      labelText.style.fontSize = '14px';
      labelText.style.fontWeight = '600';
      labelText.style.color = 'var(--admin-text-primary)';
      labelText.textContent = item.label;

      const desc = document.createElement('div');
      desc.style.fontSize = '12px';
      desc.style.color = 'var(--admin-text-muted)';
      desc.style.marginTop = '2px';
      desc.textContent = item.desc;

      textWrap.append(labelText, desc);

      const label = document.createElement('label');
      label.className = 'mp-toggle';
      label.style.flexShrink = '0';

      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = item.enabled;
      input.dataset.idx = String(index);

      const slider = document.createElement('span');
      slider.className = 'mp-toggle-slider';

      input.addEventListener('change', function () {
        const idx = Number(this.dataset.idx);
        tradingHours[idx].enabled = this.checked;
      });

      label.append(input, slider);
      row.append(textWrap, label);
      container.appendChild(row);
    });
  }

  function collectSettings() {
    const tickSizeCents = parseUsdToCents('setting-tick', 'Tick size');
    const minOrderSize = parsePositiveInt('setting-min-order', 'Min order size');
    const maxOrderSize = parsePositiveInt('setting-max-order', 'Max order size');
    const maxGasGwei = parsePositiveInt('setting-gas', 'Max gas price');
    const settlementBatchSize = parsePositiveInt('setting-batch', 'Settlement batch size');

    if (minOrderSize > maxOrderSize) {
      throw new Error('Min order size cannot exceed max order size.');
    }

    return {
      matching_algorithm: document.getElementById('setting-algo')?.value || DEFAULT_SETTINGS.matching_algorithm,
      tick_size_cents: tickSizeCents,
      min_order_size: minOrderSize,
      max_order_size: maxOrderSize,
      settlement_mode: document.getElementById('setting-settlement')?.value || DEFAULT_SETTINGS.settlement_mode,
      max_gas_gwei: maxGasGwei,
      settlement_batch_size: settlementBatchSize,
      trading_enabled: tradingHours[0].enabled,
      maintenance_window: tradingHours[1].enabled,
      weekend_trading: tradingHours[2].enabled,
    };
  }

  function applySettings(data) {
    const settings = { ...DEFAULT_SETTINGS, ...(data || {}) };
    setFieldValue('setting-algo', settings.matching_algorithm);
    setFieldValue('setting-tick', formatCents(settings.tick_size_cents));
    setFieldValue('setting-min-order', settings.min_order_size);
    setFieldValue('setting-max-order', settings.max_order_size);
    setFieldValue('setting-settlement', settings.settlement_mode);
    setFieldValue('setting-gas', settings.max_gas_gwei);
    setFieldValue('setting-batch', settings.settlement_batch_size);

    tradingHours[0].enabled = settings.trading_enabled !== false;
    tradingHours[1].enabled = !!settings.maintenance_window;
    tradingHours[2].enabled = !!settings.weekend_trading;
  }

  async function parseJsonError(res) {
    try {
      const data = await res.json();
      return data.error || data.message || `HTTP ${res.status}`;
    } catch (_) {
      return `HTTP ${res.status}`;
    }
  }

  async function loadSettings() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(await parseJsonError(res));
      const data = await res.json();
      applySettings(data);
    } catch (err) {
      console.warn('[mp-settings] Load failed:', err);
      if (typeof mpToast === 'function') {
        mpToast('Marketplace settings could not be loaded. Showing defaults only.', 'warning');
      }
      applySettings(DEFAULT_SETTINGS);
    }
    renderToggles('trading-hours-body', tradingHours);
  }

  async function saveSettings(payload, successMessage) {
    const res = await fetch(API, {
      method: 'POST',
      credentials: 'same-origin',
      headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify(payload),
    });
    if (!res.ok) throw new Error(await parseJsonError(res));
    if (typeof mpToast === 'function') mpToast(successMessage, 'success');
  }

  document.addEventListener('DOMContentLoaded', () => {
    loadSettings();

    document.getElementById('btn-save-settings')?.addEventListener('click', async function () {
      const btn = this;
      btn.disabled = true;
      btn.textContent = 'Saving...';

      try {
        const payload = collectSettings();
        await saveSettings(payload, 'Marketplace settings saved successfully');
      } catch (err) {
        console.warn('[mp-settings] Save failed:', err);
        if (typeof mpToast === 'function') {
          mpToast(err.message || 'Marketplace settings were not saved.', 'error');
        }
      }

      btn.disabled = false;
      btn.textContent = 'Save All Settings';
    });

    document.getElementById('btn-reset-settings')?.addEventListener('click', async function () {
      const confirmed = typeof window.pooolConfirm === 'function'
        ? await window.pooolConfirm({
          title: 'Reset marketplace settings',
          message: 'This will persist the default marketplace settings and update the trading flag.',
          confirmText: 'Reset settings',
          cancelText: 'Cancel',
          type: 'warning',
        })
        : window.confirm('Reset marketplace settings to defaults?');
      if (!confirmed) return;

      const btn = this;
      btn.disabled = true;
      btn.textContent = 'Resetting...';

      try {
        await saveSettings(DEFAULT_SETTINGS, 'Marketplace settings reset to defaults');
        applySettings(DEFAULT_SETTINGS);
        renderToggles('trading-hours-body', tradingHours);
      } catch (err) {
        console.warn('[mp-settings] Reset failed:', err);
        if (typeof mpToast === 'function') {
          mpToast(err.message || 'Marketplace settings were not reset.', 'error');
        }
      }

      btn.disabled = false;
      btn.textContent = 'Reset to Defaults';
    });
  });
})();
