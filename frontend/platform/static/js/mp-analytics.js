/**
 * mp-analytics.js — Admin Marketplace Analytics (Task 6B.12)
 *
 * Provides built-in analytics charts as a fallback when Metabase is unavailable.
 * Fetches data from GET /api/admin/marketplace/stats and
 * GET /api/admin/marketplace/recent-trades, then renders
 * volume timeline, top assets, and fee revenue charts.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';

  // ═══════════════════════════════════════
  // ── API ──────────────────────────────
  // ═══════════════════════════════════════

  async function fetchStats() {
    try {
      const res = await fetch(`${API_BASE}/stats`, { credentials: 'same-origin' });
      if (!res.ok) return null;
      return await res.json();
    } catch { return null; }
  }

  async function fetchTrades(limit = 100) {
    try {
      const res = await fetch(`${API_BASE}/trades?limit=${limit}`, { credentials: 'same-origin' });
      if (!res.ok) return [];
      const data = await res.json();
      return data.trades || data || [];
    } catch { return []; }
  }

  // ═══════════════════════════════════════
  // ── CHART RENDERING ──────────────────
  // ═══════════════════════════════════════

  function fmtUSD(cents) {
    return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 });
  }

  function buildVolumeChart(container, trades) {
    if (typeof ApexCharts === 'undefined' || !trades.length) {
      container.innerHTML = '<div class="mp-analytics-empty">No trade data available</div>';
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
    if (typeof ApexCharts === 'undefined' || !trades.length) return;

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
      container.innerHTML = '<div class="mp-analytics-empty">Stats unavailable</div>';
      return;
    }

    const items = [
      { label: 'Total Trades', value: (stats.total_trades || 0).toLocaleString(), icon: '📊' },
      { label: 'Total Volume', value: fmtUSD(stats.total_volume_cents || 0), icon: '💰' },
      { label: 'Open Orders', value: (stats.open_orders || 0).toLocaleString(), icon: '📋' },
      { label: 'Active Assets', value: (stats.active_assets || 0).toLocaleString(), icon: '🏠' },
      { label: 'Fee Revenue', value: fmtUSD(stats.total_fees_cents || 0), icon: '💵' },
      { label: 'Pending Orders', value: (stats.pending_orders || 0).toLocaleString(), icon: '⏳' },
    ];

    container.innerHTML = items.map(item => `
      <div class="mp-analytics-stat">
        <span class="mp-analytics-stat-icon">${item.icon}</span>
        <div>
          <div class="mp-analytics-stat-value">${item.value}</div>
          <div class="mp-analytics-stat-label">${item.label}</div>
        </div>
      </div>
    `).join('');
  }

  // ═══════════════════════════════════════
  // ── INIT ─────────────────────────────
  // ═══════════════════════════════════════

  async function init() {
    const fallbackContainer = document.getElementById('analytics-fallback');
    if (!fallbackContainer) return;

    // Check if Metabase iframe loaded successfully
    const iframe = document.getElementById('metabase-frame');
    let metabaseOk = false;

    if (iframe) {
      try {
        // Give Metabase 3 seconds to respond
        await new Promise(resolve => setTimeout(resolve, 3000));
        // If the iframe errored (CORS or unreachable), show fallback
        metabaseOk = true; // Assume ok — we can't detect cross-origin iframe errors
      } catch {}
    }

    // Always build fallback charts below Metabase
    fallbackContainer.style.display = 'block';

    const [stats, trades] = await Promise.all([fetchStats(), fetchTrades(200)]);

    // Stats cards
    const statsGridEl = document.getElementById('analytics-stats-grid');
    if (statsGridEl) buildStatsCards(statsGridEl, stats);

    // Volume timeline
    const volumeChartEl = document.getElementById('analytics-volume-chart');
    if (volumeChartEl) buildVolumeChart(volumeChartEl, trades);

    // Top assets by volume
    const assetsChartEl = document.getElementById('analytics-assets-chart');
    if (assetsChartEl) buildTradeCountChart(assetsChartEl, trades);
  }

  document.addEventListener('DOMContentLoaded', init);
})();
