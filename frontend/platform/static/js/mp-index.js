/**
 * Marketplace Overview — mp-index.js
 * Fetches KPIs, Live Trades, and System Health from backend APIs.
 * Falls back to mock data if API is unavailable.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';

  // ===== API FETCHERS =====

  async function fetchJSON(url) {
    try {
      const res = await fetch(url, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    } catch (err) {
      console.warn(`[mp-index] API fetch failed: ${url}`, err);
      return null;
    }
  }

  // ===== RENDER KPI CARDS =====

  function renderKPIs(stats) {
    const statusEl = document.getElementById('kpi-trading-status');
    const openOrdersEl = document.getElementById('kpi-open-orders');
    const volumeEl = document.getElementById('kpi-volume');
    const pendingEl = document.getElementById('kpi-pending');

    if (statusEl) {
      statusEl.textContent = stats.trading_status;
      statusEl.style.color = stats.trading_status === 'LIVE'
        ? 'var(--admin-success)' : 'var(--admin-danger, #ef4444)';
    }
    if (openOrdersEl) {
      openOrdersEl.textContent = stats.open_orders.toLocaleString();
    }
    if (volumeEl) {
      volumeEl.textContent = '$' + (stats.volume_24h_cents / 100).toLocaleString('en-US', {
        minimumFractionDigits: 0,
        maximumFractionDigits: 0
      });
    }
    if (pendingEl) {
      pendingEl.textContent = (stats.pending_reviews || 0).toLocaleString();
      pendingEl.style.color = (stats.pending_reviews || 0) > 0
        ? 'var(--admin-warning)' : 'var(--admin-text-primary)';
    }
  }

  // ===== RENDER TRADES TABLE =====

  function renderTrades(trades) {
    const tbody = document.getElementById('live-trades-body');
    if (!tbody) return;

    if (!trades || trades.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="8" style="text-align: center; color: var(--admin-text-muted); padding: 24px;">
            No recent trades
          </td>
        </tr>`;
      return;
    }

    tbody.innerHTML = trades.map(t => {
      const time = new Date(t.executed_at).toLocaleTimeString('en-US', { hour12: false });
      const total = (t.total_cents / 100).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
      const price = (t.price_cents / 100).toFixed(2);
      const assetName = t.asset_name || t.asset_id.substring(0, 8);
      const buyerLabel = t.buyer_email ? t.buyer_email.split('@')[0] : t.buyer_id.substring(0, 8);
      const sellerLabel = t.seller_email ? t.seller_email.split('@')[0] : t.seller_id.substring(0, 8);
      // Determine side based on taker (simplified)
      const sideClass = 'mp-side-buy';

      return `
        <tr>
          <td style="font-variant-numeric: tabular-nums; font-family: monospace; font-size: 12px; color: var(--admin-text-muted);">${time}</td>
          <td style="font-weight: 600; color: var(--admin-text-primary);">${assetName}</td>
          <td><span class="${sideClass}">TRADE</span></td>
          <td style="font-variant-numeric: tabular-nums;">$${price}</td>
          <td style="font-variant-numeric: tabular-nums;">${t.quantity.toLocaleString()}</td>
          <td style="font-weight: 600; font-variant-numeric: tabular-nums;">${total}</td>
          <td><code style="font-size: 11px; padding: 2px 6px; background: var(--admin-code-bg); border-radius: 4px;">${buyerLabel}</code></td>
          <td><code style="font-size: 11px; padding: 2px 6px; background: var(--admin-code-bg); border-radius: 4px;">${sellerLabel}</code></td>
        </tr>
      `;
    }).join('');
  }

  // ===== RENDER SYSTEM HEALTH =====

  function renderHealth(health) {
    const grid = document.getElementById('health-grid');
    if (!grid) return;

    const items = [
      {
        label: 'Database Latency',
        value: health.database_latency_ms.toFixed(1) + 'ms',
        status: health.database_latency_ms < 50 ? 'ok' : health.database_latency_ms < 200 ? 'warn' : 'error'
      },
      {
        label: 'Matching Engine',
        value: health.matching_engine_status === 'healthy' ? '< 1ms' : health.matching_engine_status,
        status: health.matching_engine_status === 'healthy' ? 'ok' : 'error'
      },
      {
        label: 'Active WebSockets',
        value: health.active_ws_connections.toLocaleString(),
        status: 'ok'
      },
      {
        label: 'Order Queue Depth',
        value: health.order_queue_depth.toLocaleString(),
        status: health.order_queue_depth < 1000 ? 'ok' : 'warn'
      },
      {
        label: 'Redis',
        value: health.redis_connected ? (health.redis_latency_ms ? health.redis_latency_ms.toFixed(1) + 'ms' : 'Connected') : 'Disconnected',
        status: health.redis_connected ? 'ok' : 'warn'
      },
      {
        label: 'Last Trade',
        value: health.last_trade_at ? new Date(health.last_trade_at).toLocaleTimeString('en-US', { hour12: false }) : 'N/A',
        status: 'ok'
      },
    ];

    grid.innerHTML = items.map(h => `
      <div class="mp-health-item">
        <span class="mp-health-dot mp-health-dot--${h.status}"></span>
        <span class="mp-health-label">${h.label}</span>
        <span class="mp-health-value">${h.value}</span>
      </div>
    `).join('');
  }

  // ===== FALLBACK MOCK DATA =====

  function useMockData() {
    renderKPIs({
      trading_status: 'LIVE',
      open_orders: 247,
      volume_24h_cents: 128450000,
      pending_approvals: 3
    });

    renderHealth({
      database_latency_ms: 4,
      matching_engine_status: 'healthy',
      active_ws_connections: 1247,
      order_queue_depth: 23,
      redis_connected: true,
      redis_latency_ms: 1.2,
      last_trade_at: new Date().toISOString()
    });

    // Mock trades
    const MOCK_TRADES = [
      { time: '14:32:07', asset: 'Bali Villa Resort', side: 'BUY',  price: 52.40, qty: 120, buyer: 'USR-8291', seller: 'USR-4410' },
      { time: '14:31:44', asset: 'Jakarta Office Tower', side: 'SELL', price: 105.00, qty: 50,  buyer: 'USR-1738', seller: 'USR-9203' },
      { time: '14:31:12', asset: 'Surabaya Warehouse', side: 'BUY',  price: 23.75, qty: 400, buyer: 'USR-3384', seller: 'USR-7712' },
    ];

    const tbody = document.getElementById('live-trades-body');
    if (tbody) {
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
  }

  // ===== INIT =====

  document.addEventListener('DOMContentLoaded', async () => {
    // Try fetching real data from API
    const [stats, trades, health] = await Promise.all([
      fetchJSON(`${API_BASE}/stats`),
      fetchJSON(`${API_BASE}/recent-trades`),
      fetchJSON(`${API_BASE}/health`),
    ]);

    if (stats && health) {
      renderKPIs(stats);
      renderTrades(trades || []);
      renderHealth(health);
    } else {
      // API unavailable — fall back to mock data
      console.info('[mp-index] Using mock data (API unavailable)');
      useMockData();
    }

    // Auto-refresh every 30 seconds
    setInterval(async () => {
      const [s, t, h] = await Promise.all([
        fetchJSON(`${API_BASE}/stats`),
        fetchJSON(`${API_BASE}/recent-trades`),
        fetchJSON(`${API_BASE}/health`),
      ]);
      if (s) renderKPIs(s);
      if (t) renderTrades(t);
      if (h) renderHealth(h);
    }, 30_000);
  });
})();
