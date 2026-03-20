/**
 * Marketplace Overview — mp-index.js
 * Mock data for KPIs, Live Trades table, and System Health indicators.
 */
(function () {
  'use strict';

  // ===== MOCK DATA =====
  const MOCK_TRADES = [
    { time: '14:32:07', asset: 'Bali Villa Resort', side: 'BUY',  price: 52.40, qty: 120, buyer: 'USR-8291', seller: 'USR-4410' },
    { time: '14:31:44', asset: 'Jakarta Office Tower', side: 'SELL', price: 105.00, qty: 50,  buyer: 'USR-1738', seller: 'USR-9203' },
    { time: '14:31:12', asset: 'Surabaya Warehouse', side: 'BUY',  price: 23.75, qty: 400, buyer: 'USR-3384', seller: 'USR-7712' },
    { time: '14:30:58', asset: 'Bandung Tech Hub', side: 'SELL', price: 87.20, qty: 75,  buyer: 'USR-5561', seller: 'USR-2290' },
    { time: '14:30:31', asset: 'Yogya Heritage Hotel', side: 'BUY',  price: 34.90, qty: 200, buyer: 'USR-6643', seller: 'USR-1105' },
    { time: '14:29:55', asset: 'Bali Villa Resort', side: 'SELL', price: 52.35, qty: 80,  buyer: 'USR-4410', seller: 'USR-8291' },
    { time: '14:29:22', asset: 'Medan Logistics Park', side: 'BUY',  price: 15.60, qty: 500, buyer: 'USR-7829', seller: 'USR-3344' },
    { time: '14:28:47', asset: 'Jakarta Office Tower', side: 'BUY',  price: 104.80, qty: 30,  buyer: 'USR-2201', seller: 'USR-9987' },
    { time: '14:28:11', asset: 'Semarang Retail Mall', side: 'SELL', price: 42.10, qty: 150, buyer: 'USR-5518', seller: 'USR-6632' },
    { time: '14:27:39', asset: 'Bali Villa Resort', side: 'BUY',  price: 52.50, qty: 95,  buyer: 'USR-1234', seller: 'USR-5678' },
  ];

  const MOCK_HEALTH = [
    { label: 'Database Latency', value: '4ms', status: 'ok' },
    { label: 'Matching Engine', value: '< 1ms', status: 'ok' },
    { label: 'Active WebSockets', value: '1,247', status: 'ok' },
    { label: 'Order Queue Depth', value: '23', status: 'ok' },
    { label: 'Settlement Pipeline', value: 'Healthy', status: 'ok' },
    { label: 'Last Reconciliation', value: '04:00 UTC', status: 'ok' },
  ];

  // ===== RENDER TRADES TABLE =====
  function renderTrades() {
    const tbody = document.getElementById('live-trades-body');
    if (!tbody) return;

    tbody.innerHTML = MOCK_TRADES.map(t => {
      const total = (t.price * t.qty).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
      const sideClass = t.side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      return `
        <tr>
          <td style="font-variant-numeric: tabular-nums; font-family: monospace; font-size: 12px; color: var(--admin-text-muted);">${t.time}</td>
          <td style="font-weight: 600; color: var(--admin-text-primary);">${t.asset}</td>
          <td><span class="${sideClass}">${t.side}</span></td>
          <td style="font-variant-numeric: tabular-nums;">$${t.price.toFixed(2)}</td>
          <td style="font-variant-numeric: tabular-nums;">${t.qty.toLocaleString()}</td>
          <td style="font-weight: 600; font-variant-numeric: tabular-nums;">${total}</td>
          <td><code style="font-size: 11px; padding: 2px 6px; background: var(--admin-code-bg); border-radius: 4px;">${t.buyer}</code></td>
          <td><code style="font-size: 11px; padding: 2px 6px; background: var(--admin-code-bg); border-radius: 4px;">${t.seller}</code></td>
        </tr>
      `;
    }).join('');
  }

  // ===== RENDER SYSTEM HEALTH =====
  function renderHealth() {
    const grid = document.getElementById('health-grid');
    if (!grid) return;

    grid.innerHTML = MOCK_HEALTH.map(h => `
      <div class="mp-health-item">
        <span class="mp-health-dot mp-health-dot--${h.status}"></span>
        <span class="mp-health-label">${h.label}</span>
        <span class="mp-health-value">${h.value}</span>
      </div>
    `).join('');
  }

  // ===== INIT =====
  document.addEventListener('DOMContentLoaded', () => {
    renderTrades();
    renderHealth();
  });
})();
