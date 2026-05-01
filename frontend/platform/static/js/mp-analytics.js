/**
 * mp-analytics.js — Admin Marketplace Analytics (Task 6B.12)
 *
 * Provides built-in analytics charts as a fallback when Metabase is unavailable.
   * Fetches data from GET /api/admin/marketplace/stats and
   * GET /api/admin/marketplace/trades, then renders
 * volume timeline, top assets, and fee revenue charts.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';

  // ═══════════════════════════════════════
  // ── API ──────────────────────────────
  // ═══════════════════════════════════════

  async function fetchJson(url) {
    try {
      const res = await fetch(url, { credentials: 'same-origin' });
      if (!res.ok) {
        return { ok: false, message: `Request failed with HTTP ${res.status}` };
      }
      return { ok: true, data: await res.json() };
    } catch (error) {
      return {
        ok: false,
        message: error instanceof Error ? error.message : 'Request failed',
      };
    }
  }

  async function fetchStats() {
    return fetchJson(`${API_BASE}/stats`);
  }

  async function fetchTrades(limit = 100) {
    const result = await fetchJson(`${API_BASE}/trades?limit=${limit}`);
    if (!result.ok) return result;
    const payload = result.data;
    if (Array.isArray(payload)) return { ok: true, data: payload };
    if (Array.isArray(payload.trades)) return { ok: true, data: payload.trades };
    if (Array.isArray(payload.data)) return { ok: true, data: payload.data };
    return { ok: false, message: 'Unexpected trades response shape' };
  }

  // ═══════════════════════════════════════
  // ── CHART RENDERING ──────────────────
  // ═══════════════════════════════════════

  function fmtUSD(cents) {
    return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 });
  }

  function renderMessage(container, message, options = {}) {
    container.replaceChildren();
    const el = document.createElement('div');
    el.className = options.error ? 'mp-analytics-empty mp-analytics-error' : 'mp-analytics-empty';
    el.textContent = message;
    container.appendChild(el);
  }

  function buildVolumeChart(container, trades) {
    if (typeof ApexCharts === 'undefined' || !trades.length) {
      renderMessage(container, 'No trade data available');
      return;
    }

    // Group trades by day
    const dayMap = {};
    trades.forEach(t => {
      const day = (t.executed_at || t.created_at || '').slice(0, 10);
      if (!day) return;
      if (!dayMap[day]) dayMap[day] = { volume: 0, count: 0, fees: 0 };
      dayMap[day].volume += (t.price_cents || 0) * (t.quantity || 0);
      dayMap[day].count += 1;
      dayMap[day].fees += (t.fee_cents || 0);
    });

    const days = Object.keys(dayMap).sort();
    const volumeData = days.map(d => ({ x: d, y: dayMap[d].volume / 100 }));
    const feeData = days.map(d => ({ x: d, y: dayMap[d].fees / 100 }));

    const options = {
      chart: {
        type: 'bar',
        height: 280,
        fontFamily: "'TT Norms Pro', sans-serif",
        toolbar: { show: false },
        background: 'transparent',
      },
      series: [
        { name: 'Volume ($)', data: volumeData },
        { name: 'Fees ($)', data: feeData },
      ],
      plotOptions: {
        bar: { columnWidth: '60%', borderRadius: 4 },
      },
      colors: ['#0000FF', '#16a34a'],
      xaxis: {
        type: 'category',
        labels: {
          style: { fontSize: '11px', colors: '#7a7f87' },
          rotate: -45,
          rotateAlways: days.length > 7,
        },
      },
      yaxis: {
        labels: {
          style: { fontSize: '11px', colors: '#7a7f87' },
          formatter: v => '$' + v.toLocaleString(),
        },
      },
      grid: {
        borderColor: 'var(--admin-border, #e5e7eb)',
        strokeDashArray: 3,
      },
      legend: {
        position: 'top',
        horizontalAlign: 'right',
        fontSize: '12px',
        labels: { colors: '#7a7f87' },
      },
      tooltip: {
        y: { formatter: v => '$' + v.toLocaleString('en-US', { minimumFractionDigits: 2 }) },
      },
      dataLabels: { enabled: false },
    };

    new ApexCharts(container, options).render();
  }

  function buildTradeCountChart(container, trades) {
    if (typeof ApexCharts === 'undefined' || !trades.length) {
      renderMessage(container, 'No asset trade data available');
      return;
    }

    // Group by asset
    const assetMap = {};
    trades.forEach(t => {
      const name = t.asset_title || t.asset_name || t.asset_id || 'Unknown';
      if (!assetMap[name]) assetMap[name] = { count: 0, volume: 0 };
      assetMap[name].count += 1;
      assetMap[name].volume += (t.price_cents || 0) * (t.quantity || 0);
    });

    const sorted = Object.entries(assetMap).sort((a, b) => b[1].volume - a[1].volume).slice(0, 8);
    const labels = sorted.map(([n]) => n.length > 25 ? n.slice(0, 22) + '…' : n);
    const values = sorted.map(([, v]) => v.volume / 100);

    const options = {
      chart: {
        type: 'bar',
        height: 280,
        fontFamily: "'TT Norms Pro', sans-serif",
        toolbar: { show: false },
        background: 'transparent',
      },
      series: [{ name: 'Volume ($)', data: values }],
      plotOptions: {
        bar: { horizontal: true, borderRadius: 4, barHeight: '60%' },
      },
      colors: ['#0000FF'],
      xaxis: {
        labels: {
          style: { fontSize: '11px', colors: '#7a7f87' },
          formatter: v => '$' + v.toLocaleString(),
        },
      },
      yaxis: {
        labels: { style: { fontSize: '11px', colors: '#7a7f87' } },
        categories: labels,
      },
      grid: {
        borderColor: 'var(--admin-border, #e5e7eb)',
        strokeDashArray: 3,
      },
      tooltip: {
        y: { formatter: v => '$' + v.toLocaleString('en-US', { minimumFractionDigits: 2 }) },
      },
      dataLabels: { enabled: false },
    };

    new ApexCharts(container, options).render();
  }

  // ═══════════════════════════════════════
  // ── STATS CARDS ──────────────────────
  // ═══════════════════════════════════════

  function buildStatsCards(container, stats) {
    if (!stats) {
      renderMessage(container, 'Stats unavailable', { error: true });
      return;
    }

    const ICONS = {
      trades: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="17 1 21 5 17 9"/><path d="M3 11V9a4 4 0 0 1 4-4h14"/><polyline points="7 23 3 19 7 15"/><path d="M21 13v2a4 4 0 0 1-4 4H3"/></svg>',
      volume: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>',
      orders: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z"/><line x1="3" y1="6" x2="21" y2="6"/><path d="M16 10a4 4 0 0 1-8 0"/></svg>',
      assets: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 7V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v2"/><line x1="12" y1="12" x2="12" y2="16"/><line x1="10" y1="14" x2="14" y2="14"/></svg>',
      fees: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 2a10 10 0 1 0 0 20A10 10 0 0 0 12 2z"/><path d="M9 9h.01M15 15h.01"/><line x1="15.5" y1="8.5" x2="8.5" y2="15.5"/></svg>',
      pending: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    };

    const items = [
      { label: 'Trades 24h',     value: (stats.trades_24h ?? stats.total_trades ?? 0).toLocaleString(),                  icon: ICONS.trades,  iconClass: 'mp-analytics-stat-icon--trades'  },
      { label: 'Volume 24h',     value: fmtUSD(stats.volume_24h_cents ?? stats.total_volume_cents ?? 0),                  icon: ICONS.volume,  iconClass: 'mp-analytics-stat-icon--volume'  },
      { label: 'Open Orders',    value: (stats.open_orders || 0).toLocaleString(),                                        icon: ICONS.orders,  iconClass: 'mp-analytics-stat-icon--orders'  },
      { label: 'Trading Assets', value: (stats.total_assets_trading ?? stats.active_assets ?? 0).toLocaleString(),        icon: ICONS.assets,  iconClass: 'mp-analytics-stat-icon--assets'  },
      { label: 'Fees 24h',       value: fmtUSD(stats.fees_collected_24h_cents ?? stats.total_fees_cents ?? 0),            icon: ICONS.fees,    iconClass: 'mp-analytics-stat-icon--fees'    },
      { label: 'Pending Reviews',value: (stats.pending_reviews ?? stats.pending_orders ?? 0).toLocaleString(),            icon: ICONS.pending, iconClass: 'mp-analytics-stat-icon--pending' },
    ];

    container.replaceChildren();
    items.forEach(item => {
      const card = document.createElement('div');
      card.className = 'mp-analytics-stat';

      const icon = document.createElement('span');
      icon.className = `mp-analytics-stat-icon ${item.iconClass}`;
      icon.innerHTML = item.icon;

      const body = document.createElement('div');
      const value = document.createElement('div');
      value.className = 'mp-analytics-stat-value';
      value.textContent = item.value;
      const label = document.createElement('div');
      label.className = 'mp-analytics-stat-label';
      label.textContent = item.label;

      body.append(value, label);
      card.append(icon, body);
      container.appendChild(card);
    });
  }

  function initMetabase() {
    const card = document.getElementById('metabase-card');
    if (!card) return;

    const baseUrl = (card.dataset.metabaseBaseUrl || '').replace(/\/+$/, '');
    const publicDashboardPath = card.dataset.metabasePublicDashboardPath || '';
    const dashboardId = card.dataset.metabaseDashboardId || '';
    const frame = document.getElementById('metabase-frame');
    const empty = document.getElementById('metabase-empty');
    const openBtn = document.getElementById('btn-open-metabase');
    const refreshBtn = document.getElementById('btn-refresh-metabase');

    document.querySelectorAll('[data-metabase-path]').forEach(link => {
      if (!baseUrl) return;
      link.href = `${baseUrl}${link.dataset.metabasePath}`;
      link.classList.remove('mp-analytics-disabled-link');
      link.removeAttribute('aria-disabled');
    });

    if (!baseUrl || !publicDashboardPath || !frame) {
      return;
    }

    const frameUrl = `${baseUrl}${publicDashboardPath}`;
    frame.src = frameUrl;
    frame.style.display = 'block';
    if (empty) empty.style.display = 'none';

    if (openBtn && dashboardId) {
      openBtn.disabled = false;
      openBtn.addEventListener('click', () => {
        window.open(`${baseUrl}/dashboard/${dashboardId}`, '_blank', 'noopener,noreferrer');
      });
    }
    if (refreshBtn) {
      refreshBtn.disabled = false;
      refreshBtn.addEventListener('click', () => {
        frame.src = frame.src;
      });
    }
  }

  // ═══════════════════════════════════════
  // ── INIT ─────────────────────────────
  // ═══════════════════════════════════════

  async function init() {
    const fallbackContainer = document.getElementById('analytics-fallback');
    if (!fallbackContainer) return;

    // Always build fallback charts below Metabase
    fallbackContainer.style.display = 'block';
    initMetabase();

    const [statsResult, tradesResult] = await Promise.all([fetchStats(), fetchTrades(200)]);

    // Stats cards
    const statsGridEl = document.getElementById('analytics-stats-grid');
    if (statsGridEl) {
      if (statsResult.ok) {
        buildStatsCards(statsGridEl, statsResult.data);
      } else {
        renderMessage(statsGridEl, `Stats unavailable: ${statsResult.message}`, { error: true });
      }
    }

    // Volume timeline
    const volumeChartEl = document.getElementById('analytics-volume-chart');
    if (volumeChartEl) {
      if (tradesResult.ok) {
        buildVolumeChart(volumeChartEl, tradesResult.data);
      } else {
        renderMessage(volumeChartEl, `Trade data unavailable: ${tradesResult.message}`, { error: true });
      }
    }

    // Top assets by volume
    const assetsChartEl = document.getElementById('analytics-assets-chart');
    if (assetsChartEl) {
      if (tradesResult.ok) {
        buildTradeCountChart(assetsChartEl, tradesResult.data);
      } else {
        renderMessage(assetsChartEl, `Asset data unavailable: ${tradesResult.message}`, { error: true });
      }
    }
  }

  document.addEventListener('DOMContentLoaded', init);
})();
