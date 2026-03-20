/**
 * MP Settings — mp-settings.js
 */
(function () {
  'use strict';

  const tradingHours = [
    { enabled: true, label: '24/7 Trading', desc: 'Marketplace is open continuously with no scheduled downtime' },
    { enabled: false, label: 'Maintenance Window', desc: 'Daily maintenance from 04:00-04:30 UTC (matching engine offline)' },
    { enabled: false, label: 'Weekend Trading', desc: 'Allow trading on Saturday and Sunday (disabled = weekday only)' },
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
        mpToast(`"${arr[idx].label}" ${this.checked ? 'enabled' : 'disabled'}`, this.checked ? 'success' : 'warning');
      });
    });
  }

  document.addEventListener('DOMContentLoaded', () => {
    renderToggles('trading-hours-body', tradingHours, 'hours');
    renderToggles('notif-prefs-body', notifPrefs, 'notifs');

    document.getElementById('btn-save-settings')?.addEventListener('click', function () {
      mpButtonAction(this, 'All marketplace settings saved successfully', 1500);
    });

    document.getElementById('btn-reset-settings')?.addEventListener('click', function () {
      mpButtonAction(this, 'Settings reset to factory defaults', 1000);
    });
  });
})();
