/**
 * Marketplace Overview — mp-index.js
 * Fetches KPIs, Live Trades, and System Health from backend APIs.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';
  let lastStatsError = null;
  let lastTradesError = null;
  let lastHealthError = null;

  // ===== API FETCHERS =====

  async function fetchJSON(url) {
    const res = await fetch(url, { credentials: 'same-origin' });
    if (!res.ok) {
      let message = `HTTP ${res.status}`;
      try {
        const body = await res.json();
        message = body.error || body.message || message;
      } catch (_) {
        // Response was not JSON; keep the HTTP status message.
      }
      throw new Error(message);
    }
    return res.json();
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  }

  function formatCurrency(cents) {
    return (Number(cents || 0) / 100).toLocaleString('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0
    });
  }

  function firstSegment(value, fallback) {
    const text = String(value || fallback || '');
    return text.includes('@') ? text.split('@')[0] : text;
  }

  function shortId(value) {
    return String(value || '').substring(0, 8) || 'unknown';
  }

  function clearElement(el) {
    while (el.firstChild) el.removeChild(el.firstChild);
  }

  function makeCell(text, className) {
    const td = document.createElement('td');
    if (className) td.className = className;
    td.textContent = text;
    return td;
  }

  function makeCodeCell(text) {
    const td = document.createElement('td');
    const code = document.createElement('code');
    code.className = 'mp-inline-code';
    code.textContent = text;
    td.appendChild(code);
    return td;
  }

  function renderTableMessage(message, tone) {
    const tbody = document.getElementById('live-trades-body');
    if (!tbody) return;
    clearElement(tbody);
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 8;
    cell.className = `mp-table-message${tone ? ` mp-table-message--${tone}` : ''}`;
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function renderHealthMessage(message, tone) {
    const grid = document.getElementById('health-grid');
    if (!grid) return;
    clearElement(grid);
    const item = document.createElement('div');
    item.className = `mp-health-message${tone ? ` mp-health-message--${tone}` : ''}`;
    item.textContent = message;
    grid.appendChild(item);
  }

  function renderStatsError(error) {
    const message = error && error.message ? error.message : 'Stats unavailable';
    setText('kpi-trading-status', 'Unavailable');
    setText('kpi-trading-subtext', message);
    setText('kpi-open-orders', '--');
    setText('kpi-open-orders-subtext', 'Open order count unavailable');
    setText('kpi-volume', '--');
    setText('kpi-volume-subtext', '24h volume unavailable');
    setText('kpi-pending', '--');
    setText('kpi-pending-subtext', 'Review queue unavailable');
    const statusEl = document.getElementById('kpi-trading-status');
    if (statusEl) statusEl.style.color = 'var(--admin-danger, #ef4444)';
  }

  // ===== RENDER KPI CARDS =====

  function renderKPIs(stats) {
    const statusEl = document.getElementById('kpi-trading-status');
    const openOrdersEl = document.getElementById('kpi-open-orders');
    const volumeEl = document.getElementById('kpi-volume');
    const pendingEl = document.getElementById('kpi-pending');
    const tradingStatus = stats.trading_status || 'UNKNOWN';

    if (statusEl) {
      statusEl.textContent = tradingStatus;
      statusEl.style.color = tradingStatus === 'LIVE'
        ? 'var(--admin-success)'
        : tradingStatus === 'HALTED'
          ? 'var(--admin-warning)'
          : 'var(--admin-danger, #ef4444)';
    }
    if (openOrdersEl) {
      openOrdersEl.textContent = Number(stats.open_orders || 0).toLocaleString();
    }
    if (volumeEl) {
      volumeEl.textContent = formatCurrency(stats.volume_24h_cents);
    }
    if (pendingEl) {
      const pending = Number(stats.pending_reviews || 0);
      pendingEl.textContent = pending.toLocaleString();
      pendingEl.style.color = (stats.pending_reviews || 0) > 0
        ? 'var(--admin-warning)' : 'var(--admin-text-primary)';
    }

    setText(
      'kpi-trading-subtext',
      tradingStatus === 'LIVE'
        ? 'Trading enabled'
        : tradingStatus === 'HALTED'
          ? 'Trading halted by kill-switch'
          : 'Trading status could not be verified'
    );
    setText('kpi-open-orders-subtext', `${Number(stats.total_assets_trading || 0).toLocaleString()} active assets`);
    setText('kpi-volume-subtext', `${Number(stats.trades_24h || 0).toLocaleString()} trades in the last 24h`);
    setText('kpi-pending-subtext', Number(stats.pending_reviews || 0) > 0
      ? 'Large orders awaiting review'
      : 'No orders awaiting review');
  }

  // ===== RENDER TRADES TABLE =====

  function renderTrades(trades) {
    const tbody = document.getElementById('live-trades-body');
    if (!tbody) return;

    clearElement(tbody);

    if (!Array.isArray(trades) || trades.length === 0) {
      renderTableMessage('No recent trades', 'muted');
      return;
    }

    trades.forEach((trade) => {
      const row = document.createElement('tr');
      const executed = trade.executed_at ? new Date(trade.executed_at) : null;
      const time = executed && !Number.isNaN(executed.getTime())
        ? executed.toLocaleTimeString('en-US', { hour12: false })
        : '--';
      const assetName = trade.asset_name || shortId(trade.asset_id);
      const buyerLabel = firstSegment(trade.buyer_email, shortId(trade.buyer_id));
      const sellerLabel = firstSegment(trade.seller_email, shortId(trade.seller_id));

      row.appendChild(makeCell(time, 'mp-cell-muted mp-cell-tabular'));
      row.appendChild(makeCell(assetName, 'mp-cell-strong'));

      const sideCell = document.createElement('td');
      const badge = document.createElement('span');
      badge.className = 'mp-side-buy';
      badge.textContent = 'TRADE';
      sideCell.appendChild(badge);
      row.appendChild(sideCell);

      row.appendChild(makeCell(formatCurrency(trade.price_cents), 'mp-cell-tabular'));
      row.appendChild(makeCell(Number(trade.quantity || 0).toLocaleString(), 'mp-cell-tabular'));
      row.appendChild(makeCell(formatCurrency(trade.total_cents), 'mp-cell-strong mp-cell-tabular'));
      row.appendChild(makeCodeCell(buyerLabel));
      row.appendChild(makeCodeCell(sellerLabel));
      tbody.appendChild(row);
    });
  }

  // ===== RENDER SYSTEM HEALTH =====

  function componentStatus(condition, degraded) {
    if (condition) return 'ok';
    return degraded ? 'warn' : 'error';
  }

  function setTopbarHealth(id, status, label) {
    const dot = document.getElementById(id);
    if (!dot) return;
    dot.className = `admin-health-dot admin-health-dot--${status}`;
    dot.title = label;
  }

  function updateTopbarHealth(health) {
    const dbStatus = componentStatus(Boolean(health.database_connected), false);
    const redisStatus = componentStatus(Boolean(health.redis_connected), true);
    const wsStatus = health.websocket_status === 'healthy'
      ? 'ok'
      : health.websocket_status === 'not_tracked'
        ? 'warn'
        : 'error';

    setTopbarHealth('health-dot-db', dbStatus, `Database: ${dbStatus}`);
    setTopbarHealth('health-dot-matching', redisStatus, `Matching Engine: ${health.matching_engine_status || 'unknown'}`);
    setTopbarHealth('health-dot-ws', wsStatus, `WebSocket Gateway: ${health.websocket_status || 'unknown'}`);
  }

  function renderHealth(health) {
    const grid = document.getElementById('health-grid');
    if (!grid) return;
    clearElement(grid);
    updateTopbarHealth(health);

    const databaseStatus = componentStatus(Boolean(health.database_connected), false);
    const redisStatus = componentStatus(Boolean(health.redis_connected), true);
    const matchingStatus = health.matching_engine_status === 'healthy'
      ? 'ok'
      : health.matching_engine_status === 'not_configured'
        ? 'warn'
        : 'error';
    const websocketStatus = health.websocket_status === 'healthy'
      ? 'ok'
      : health.websocket_status === 'not_tracked'
        ? 'warn'
        : 'error';

    const items = [
      {
        label: 'Database Latency',
        value: Number(health.database_latency_ms || 0).toFixed(1) + 'ms',
        status: databaseStatus
      },
      {
        label: 'Matching Engine',
        value: health.matching_engine_status || 'unknown',
        status: matchingStatus
      },
      {
        label: 'Active WebSockets',
        value: health.websocket_status === 'not_tracked'
          ? 'Not tracked'
          : Number(health.active_ws_connections || 0).toLocaleString(),
        status: websocketStatus
      },
      {
        label: 'Order Queue Depth',
        value: Number(health.order_queue_depth || 0).toLocaleString(),
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

    items.forEach((item) => {
      const row = document.createElement('div');
      row.className = 'mp-health-item';

      const dot = document.createElement('span');
      dot.className = `mp-health-dot mp-health-dot--${item.status}`;
      row.appendChild(dot);

      const label = document.createElement('span');
      label.className = 'mp-health-label';
      label.textContent = item.label;
      row.appendChild(label);

      const value = document.createElement('span');
      value.className = 'mp-health-value';
      value.textContent = item.value;
      row.appendChild(value);

      grid.appendChild(row);
    });
  }

  // ===== INIT =====

  async function refreshAll() {
    const [statsResult, tradesResult, healthResult] = await Promise.allSettled([
      fetchJSON(`${API_BASE}/stats`),
      fetchJSON(`${API_BASE}/recent-trades`),
      fetchJSON(`${API_BASE}/health`),
    ]);

    if (statsResult.status === 'fulfilled') {
      lastStatsError = null;
      renderKPIs(statsResult.value);
    } else {
      lastStatsError = statsResult.reason;
      console.warn('[mp-index] Stats unavailable', lastStatsError);
      renderStatsError(lastStatsError);
    }

    if (tradesResult.status === 'fulfilled') {
      lastTradesError = null;
      renderTrades(tradesResult.value);
    } else {
      lastTradesError = tradesResult.reason;
      console.warn('[mp-index] Recent trades unavailable', lastTradesError);
      renderTableMessage(`Recent trades unavailable: ${lastTradesError.message || 'request failed'}`, 'error');
    }

    if (healthResult.status === 'fulfilled') {
      lastHealthError = null;
      renderHealth(healthResult.value);
    } else {
      lastHealthError = healthResult.reason;
      console.warn('[mp-index] Health unavailable', lastHealthError);
      setTopbarHealth('health-dot-db', 'error', 'Database: health check unavailable');
      setTopbarHealth('health-dot-matching', 'error', 'Matching Engine: health check unavailable');
      setTopbarHealth('health-dot-ws', 'error', 'WebSocket Gateway: health check unavailable');
      renderHealthMessage(`System health unavailable: ${lastHealthError.message || 'request failed'}`, 'error');
    }
  }

  document.addEventListener('DOMContentLoaded', async () => {
    renderTableMessage('Loading recent trades', 'muted');
    renderHealthMessage('Checking system health', 'muted');
    await refreshAll();

    // Auto-refresh every 30 seconds
    setInterval(refreshAll, 30_000);
  });

  window.PooolMarketplaceOverview = {
    refreshAll,
    getLastErrors() {
      return {
        stats: lastStatsError,
        trades: lastTradesError,
        health: lastHealthError
      };
    }
  };
})();
