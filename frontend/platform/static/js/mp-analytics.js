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

    const items = [
      { label: 'Trades 24h', value: (stats.trades_24h ?? stats.total_trades ?? 0).toLocaleString(), icon: '📊' },
      { label: 'Volume 24h', value: fmtUSD(stats.volume_24h_cents ?? stats.total_volume_cents ?? 0), icon: '💰' },
      { label: 'Open Orders', value: (stats.open_orders || 0).toLocaleString(), icon: '📋' },
      { label: 'Trading Assets', value: (stats.total_assets_trading ?? stats.active_assets ?? 0).toLocaleString(), icon: '🏠' },
      { label: 'Fees 24h', value: fmtUSD(stats.fees_collected_24h_cents ?? stats.total_fees_cents ?? 0), icon: '💵' },
      { label: 'Pending Reviews', value: (stats.pending_reviews ?? stats.pending_orders ?? 0).toLocaleString(), icon: '⏳' },
    ];

    container.replaceChildren();
    items.forEach(item => {
      const card = document.createElement('div');
      card.className = 'mp-analytics-stat';

      const icon = document.createElement('span');
      icon.className = 'mp-analytics-stat-icon';
      icon.setAttribute('aria-hidden', 'true');
      icon.textContent = item.icon;

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
