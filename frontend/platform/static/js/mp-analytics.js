/**
 * mp-analytics.js — Admin Marketplace Analytics
 *
 * Built-in analytics view: action-required zone, KPI cards w/ trend deltas,
 * dual-axis volume/fees chart with period toggle, top-assets chart, recent
 * trades table, last-updated indicator with auto-refresh.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';

  // Persisted prefs (localStorage).
  const PREFS_KEY = 'mp.analytics.prefs.v1';
  function loadPrefs() {
    try { return JSON.parse(localStorage.getItem(PREFS_KEY)) || {}; }
    catch (_) { return {}; }
  }
  function savePrefs(p) {
    try { localStorage.setItem(PREFS_KEY, JSON.stringify(p)); } catch (_) { /* noop */ }
  }
  const prefs = loadPrefs();

  // State
  const state = {
    stats: null,
    trades: [],
    period: {
      volume: prefs.periodVolume || 7,
      assets: prefs.periodAssets || 7,
    },
    charts: { volume: null, assets: null },
    lastUpdated: null,
    refreshTimer: null,
    updatedTimer: null,
    refreshIntervalMs: prefs.refreshIntervalMs ?? 60000,
    tradesFilter: { status: 'all', sortKey: 'time', sortDir: 'desc' },
  };

  // ─── API ─────────────────────────────────────────────

  async function fetchJson(url) {
    try {
      const res = await fetch(url, { credentials: 'same-origin' });
      if (!res.ok) return { ok: false, message: `HTTP ${res.status}` };
      return { ok: true, data: await res.json() };
    } catch (error) {
      return { ok: false, message: error instanceof Error ? error.message : 'Request failed' };
    }
  }

  async function fetchStats()  { return fetchJson(`${API_BASE}/stats`); }
  async function fetchTrades(limit = 200) {
    const r = await fetchJson(`${API_BASE}/trades?limit=${limit}`);
    if (!r.ok) return r;
    const p = r.data;
    if (Array.isArray(p)) return { ok: true, data: p };
    if (Array.isArray(p.trades)) return { ok: true, data: p.trades };
    if (Array.isArray(p.data)) return { ok: true, data: p.data };
    return { ok: false, message: 'Unexpected trades response shape' };
  }

  // ─── FORMATTERS ──────────────────────────────────────

  function fmtUSD(cents) {
    return '$' + (cents / 100).toLocaleString('en-US', { maximumFractionDigits: 0 });
  }
  function fmtUSDfull(cents) {
    return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }
  function fmtNum(n) { return Number(n).toLocaleString('en-US'); }

  function fmtAge(seconds) {
    if (seconds == null || seconds < 0) return '—';
    const s = Math.floor(seconds);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h ${m % 60}m`;
    const d = Math.floor(h / 24);
    return `${d}d ${h % 24}h`;
  }

  function fmtRelative(ts) {
    if (!ts) return '';
    const diff = Math.floor((Date.now() - new Date(ts).getTime()) / 1000);
    if (diff < 5) return 'just now';
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    return `${Math.floor(diff / 3600)}h ago`;
  }

  function pctDelta(current, prev) {
    if (!prev || prev === 0) return current > 0 ? { dir: 'up', text: 'new' } : { dir: 'flat', text: '—' };
    const change = ((current - prev) / prev) * 100;
    if (Math.abs(change) < 0.5) return { dir: 'flat', text: '0.0%' };
    return {
      dir: change > 0 ? 'up' : 'down',
      text: (change > 0 ? '+' : '') + change.toFixed(1) + '%',
    };
  }

  function renderMessage(container, message, error = false) {
    container.replaceChildren();
    const el = document.createElement('div');
    el.className = 'mp-analytics-empty' + (error ? ' mp-analytics-error' : '');
    el.textContent = message;
    container.appendChild(el);
  }

  // ─── ACTION-REQUIRED ZONE ────────────────────────────

  function buildActionZone(container, stats) {
    if (!stats) {
      renderMessage(container, 'Action data unavailable', true);
      return;
    }

    const SLA = { warn: 60 * 60, alert: 24 * 60 * 60 }; // 1h warn, 24h alert
    const tierFor = (age) => age == null ? 'ok' : age >= SLA.alert ? 'alert' : age >= SLA.warn ? 'warn' : 'ok';

    const items = [
      {
        title: 'Open Orders',
        value: fmtNum(stats.open_orders || 0),
        meta: stats.oldest_open_order_age_seconds != null
          ? `Oldest: ${fmtAge(stats.oldest_open_order_age_seconds)}`
          : 'No open orders',
        tier: stats.open_orders > 0 ? tierFor(stats.oldest_open_order_age_seconds) : 'ok',
        href: '/admin/marketplace/orders.html?status=open',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z"/><line x1="3" y1="6" x2="21" y2="6"/></svg>',
      },
      {
        title: 'Pending Reviews',
        value: fmtNum(stats.pending_reviews || 0),
        meta: stats.oldest_pending_review_age_seconds != null
          ? `Oldest: ${fmtAge(stats.oldest_pending_review_age_seconds)}`
          : 'Queue clear',
        tier: stats.pending_reviews > 0 ? tierFor(stats.oldest_pending_review_age_seconds) : 'ok',
        href: '/admin/marketplace/orders.html?status=pending_review',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
      },
      {
        title: 'Trading Status',
        value: stats.trading_status || 'UNKNOWN',
        meta: stats.trading_status === 'LIVE' ? 'Market open' : stats.trading_status === 'HALTED' ? 'Trading halted' : 'Status unknown',
        tier: stats.trading_status === 'LIVE' ? 'ok' : stats.trading_status === 'HALTED' ? 'alert' : 'warn',
        href: '/admin/marketplace/index.html',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9 12l2 2 4-4"/></svg>',
      },
    ];

    // Fee-rate anomaly check (if rate > 2× prior period)
    const vol = stats.volume_24h_cents || 0;
    const fees = stats.fees_collected_24h_cents || 0;
    const volPrev = stats.volume_prev_24h_cents || 0;
    const feesPrev = stats.fees_prev_24h_cents || 0;
    if (vol > 0) {
      const rate = (fees / vol) * 100;
      const ratePrev = volPrev > 0 ? (feesPrev / volPrev) * 100 : null;
      let tier = 'ok';
      let meta = `Last 24h${ratePrev != null ? ` · prev ${ratePrev.toFixed(2)}%` : ''}`;
      if (ratePrev != null && rate > ratePrev * 1.5 && rate > 1) { tier = 'warn'; meta = `↑ vs prev ${ratePrev.toFixed(2)}%`; }
      if (rate > 5) { tier = 'alert'; meta = `Above 5% threshold (prev ${ratePrev != null ? ratePrev.toFixed(2) + '%' : 'n/a'})`; }
      items.push({
        title: 'Fee Rate 24h',
        value: rate.toFixed(2) + '%',
        meta,
        tier,
        href: '/admin/marketplace/trades.html',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="5" x2="5" y2="19"/><circle cx="6.5" cy="6.5" r="2.5"/><circle cx="17.5" cy="17.5" r="2.5"/></svg>',
      });
    }

    // Volume spike anomaly: 24h volume > 2× prior period.
    if (volPrev > 0 && vol > volPrev * 2) {
      const factor = (vol / volPrev).toFixed(1);
      items.push({
        title: 'Volume Spike',
        value: `${factor}×`,
        meta: `${fmtUSD(vol)} vs prev ${fmtUSD(volPrev)}`,
        tier: vol > volPrev * 5 ? 'alert' : 'warn',
        href: '/admin/marketplace/trades.html',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>',
      });
    }

    // Trade-count spike: 24h count > 2× prior.
    const tCnt = stats.trades_24h || 0;
    const tPrev = stats.trades_prev_24h || 0;
    if (tPrev > 0 && tCnt > tPrev * 2) {
      const factor = (tCnt / tPrev).toFixed(1);
      items.push({
        title: 'Trade Count Spike',
        value: `${factor}×`,
        meta: `${tCnt} vs prev ${tPrev}`,
        tier: tCnt > tPrev * 5 ? 'alert' : 'warn',
        href: '/admin/marketplace/trades.html',
        icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v18h18"/><polyline points="7 14 12 9 16 13 21 8"/></svg>',
      });
    }

    container.replaceChildren();
    items.forEach(it => {
      const a = document.createElement('a');
      a.className = `mp-action-card mp-action-card--${it.tier}`;
      a.href = it.href;
      a.innerHTML = `
        <span class="mp-action-card-icon">${it.icon}</span>
        <div>
          <div class="mp-action-card-title">${it.title}</div>
          <div class="mp-action-card-value">${it.value}</div>
          <div class="mp-action-card-meta">${it.meta}</div>
        </div>`;
      container.appendChild(a);
    });
  }

  // ─── STATS CARDS WITH DELTAS ─────────────────────────

  const STAT_ICONS = {
    trades:  '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="17 1 21 5 17 9"/><path d="M3 11V9a4 4 0 0 1 4-4h14"/><polyline points="7 23 3 19 7 15"/><path d="M21 13v2a4 4 0 0 1-4 4H3"/></svg>',
    volume:  '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>',
    orders:  '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z"/><line x1="3" y1="6" x2="21" y2="6"/><path d="M16 10a4 4 0 0 1-8 0"/></svg>',
    assets:  '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 7V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v2"/></svg>',
    fees:    '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="5" x2="5" y2="19"/><circle cx="6.5" cy="6.5" r="2.5"/><circle cx="17.5" cy="17.5" r="2.5"/></svg>',
    users:   '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/></svg>',
  };

  function deltaArrow(dir) {
    if (dir === 'up')   return '↑';
    if (dir === 'down') return '↓';
    return '·';
  }

  function buildStatsCards(container, stats) {
    if (!stats) return renderMessage(container, 'Stats unavailable', true);

    const items = [
      {
        label: 'Trades 24h',
        value: fmtNum(stats.trades_24h ?? 0),
        delta: pctDelta(stats.trades_24h ?? 0, stats.trades_prev_24h ?? 0),
        tooltip: 'Number of trades executed in the last 24 hours.',
        icon: STAT_ICONS.trades, iconClass: 'mp-analytics-stat-icon--trades',
        href: '/admin/marketplace/trades.html',
      },
      {
        label: 'Volume 24h',
        value: fmtUSD(stats.volume_24h_cents ?? 0),
        delta: pctDelta(stats.volume_24h_cents ?? 0, stats.volume_prev_24h_cents ?? 0),
        tooltip: 'Total notional traded value in the last 24 hours.',
        icon: STAT_ICONS.volume, iconClass: 'mp-analytics-stat-icon--volume',
        href: '/admin/marketplace/trades.html',
      },
      {
        label: 'Active Users 24h',
        value: fmtNum(stats.active_users_24h ?? 0),
        tooltip: 'Distinct users who placed an order in the last 24 hours.',
        icon: STAT_ICONS.users, iconClass: 'mp-analytics-stat-icon--assets',
        href: '/admin/community-users/',
      },
      {
        label: 'Trading Assets',
        value: fmtNum(stats.total_assets_trading ?? 0),
        tooltip: 'Distinct assets with active orders.',
        icon: STAT_ICONS.assets, iconClass: 'mp-analytics-stat-icon--assets',
        href: '/admin/marketplace/orders.html',
      },
      {
        label: 'Fees 24h',
        value: fmtUSD(stats.fees_collected_24h_cents ?? 0),
        delta: pctDelta(stats.fees_collected_24h_cents ?? 0, stats.fees_prev_24h_cents ?? 0),
        tooltip: 'Marketplace fees collected in the last 24 hours.',
        icon: STAT_ICONS.fees, iconClass: 'mp-analytics-stat-icon--fees',
        href: '/admin/marketplace/trades.html',
      },
      {
        label: 'Pending Reviews',
        value: fmtNum(stats.pending_reviews ?? 0),
        tooltip: 'Orders awaiting admin approval.',
        icon: STAT_ICONS.orders,
        // Neutral muted icon when 0 — orange only when items exist (avoid false-alarm code).
        iconClass: (stats.pending_reviews ?? 0) > 0 ? 'mp-analytics-stat-icon--pending' : 'mp-analytics-stat-icon--muted',
        href: '/admin/marketplace/orders.html?status=pending_review',
      },
      {
        label: 'Fee Rate 24h',
        value: (stats.volume_24h_cents > 0
          ? ((stats.fees_collected_24h_cents / stats.volume_24h_cents) * 100).toFixed(2) + '%'
          : '0.00%'),
        tooltip: 'Fees as % of volume — sanity-check vs. configured fee tiers.',
        icon: STAT_ICONS.fees, iconClass: 'mp-analytics-stat-icon--muted',
        href: '/admin/marketplace/trades.html',
      },
    ];

    // Apply hidden-card prefs from localStorage.
    const hidden = getHiddenCards();
    let visibleItems = items.filter(it => !hidden.has(it.label));
    // Apply persisted reorder.
    if (typeof window._mpApplyCardOrder === 'function') {
      visibleItems = window._mpApplyCardOrder(visibleItems);
    }
    // Apply pinning (pinned items move to front, preserve relative order otherwise).
    if (typeof window._mpApplyPinning === 'function') {
      visibleItems = window._mpApplyPinning(visibleItems);
    }
    renderHiddenBar(items, hidden);

    container.replaceChildren();
    visibleItems.forEach(item => {
      const card = document.createElement(item.href ? 'a' : 'div');
      card.className = 'mp-analytics-stat';
      card.draggable = true;
      if (item.href) card.href = item.href;
      if (item.tooltip) {
        card.dataset.pop = item.tooltip;  // custom popover
      }
      card.dataset.cardKey = item.label;

      const icon = document.createElement('span');
      icon.className = `mp-analytics-stat-icon ${item.iconClass}`;
      icon.innerHTML = item.icon;

      const body = document.createElement('div');
      body.className = 'mp-analytics-stat-body';
      const value = document.createElement('div');
      value.className = 'mp-analytics-stat-value';
      value.textContent = item.value;
      const label = document.createElement('div');
      label.className = 'mp-analytics-stat-label';
      label.textContent = item.label;
      if (item.tooltip) {
        const tip = document.createElement('span');
        tip.className = 'mp-info-tip';
        tip.textContent = 'i';
        tip.title = item.tooltip;
        tip.setAttribute('aria-label', item.tooltip);
        label.appendChild(tip);
      }
      body.append(value, label);

      // Hide button (top-right of card).
      const hideBtn = document.createElement('button');
      hideBtn.type = 'button';
      hideBtn.className = 'mp-stat-hide-btn';
      hideBtn.title = `Hide "${item.label}"`;
      hideBtn.setAttribute('aria-label', `Hide ${item.label} card`);
      hideBtn.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';
      hideBtn.addEventListener('click', (ev) => {
        ev.preventDefault();
        ev.stopPropagation();
        hideCard(item.label);
        buildStatsCards(container, stats);
      });
      card.appendChild(hideBtn);
      // Pin button.
      if (typeof window._mpAttachPinButton === 'function') {
        window._mpAttachPinButton(card, item.label);
      }

      if (item.delta) {
        const d = document.createElement('div');
        d.className = `mp-analytics-stat-delta mp-analytics-stat-delta--${item.delta.dir}`;
        d.textContent = `${deltaArrow(item.delta.dir)} ${item.delta.text} vs prior 24h`;
        body.appendChild(d);
      }

      card.append(icon, body);
      container.appendChild(card);
    });
  }

  // ─── CHARTS ──────────────────────────────────────────

  function filterByDays(trades, days) {
    // Custom date range overrides preset window when both endpoints set.
    const cr = state.customRange;
    if (cr && cr.start && cr.end) {
      const s = Date.parse(cr.start);
      const e = Date.parse(cr.end) + 86400_000; // include end day
      return trades.filter(t => {
        const ts = Date.parse(t.executed_at || t.created_at || '');
        return Number.isFinite(ts) && ts >= s && ts < e;
      });
    }
    const cutoff = Date.now() - days * 86400_000;
    return trades.filter(t => {
      const ts = t.executed_at || t.created_at;
      if (!ts) return false;
      return new Date(ts).getTime() >= cutoff;
    });
  }

  function buildVolumeChart(container, trades, days) {
    if (state.charts.volume) {
      try { state.charts.volume.destroy(); } catch (_) { /* noop */ }
      state.charts.volume = null;
    }
    if (typeof ApexCharts === 'undefined') return renderMessage(container, 'Chart library unavailable', true);
    const filtered = filterByDays(trades, days);
    if (!filtered.length) return renderMessage(container, `No trades in last ${days} days`);

    const dayMap = {};
    for (let i = 0; i < days; i++) {
      const d = new Date(Date.now() - i * 86400_000);
      const key = d.toISOString().slice(0, 10);
      dayMap[key] = { volume: 0, fees: 0 };
    }
    filtered.forEach(t => {
      const day = (t.executed_at || t.created_at || '').slice(0, 10);
      if (!day || !(day in dayMap)) return;
      dayMap[day].volume += (t.price_cents || 0) * (t.quantity || 0);
      dayMap[day].fees += (t.fee_cents || 0);
    });

    const sortedDays = Object.keys(dayMap).sort();
    const volumeData = sortedDays.map(d => ({ x: d, y: +(dayMap[d].volume / 100).toFixed(2) }));
    const feeData    = sortedDays.map(d => ({ x: d, y: +(dayMap[d].fees   / 100).toFixed(2) }));

    container.replaceChildren();
    const chart = new ApexCharts(container, {
      chart: {
        type: 'line',
        height: 280,
        fontFamily: "'TT Norms Pro', sans-serif",
        toolbar: { show: true, tools: { download: true, selection: false, zoom: false, zoomin: false, zoomout: false, pan: false, reset: false } },
        background: 'transparent',
        animations: { enabled: true, speed: 300 },
        events: {
          dataPointSelection: (_evt, _ctx, opts) => {
            const day = sortedDays[opts.dataPointIndex];
            if (day) window.location.href = `/admin/marketplace/trades.html?date=${encodeURIComponent(day)}`;
          },
        },
      },
      series: [
        { name: 'Volume', type: 'column', data: volumeData },
        { name: 'Fees',   type: 'line',   data: feeData },
      ],
      states: { hover: { filter: { type: 'lighten', value: 0.1 } } },
      stroke: { width: [0, 3], curve: 'smooth' },
      plotOptions: { bar: { columnWidth: '55%', borderRadius: 4 } },
      colors: ['#0000FF', '#16a34a'],
      xaxis: {
        type: 'category',
        labels: {
          style: { fontSize: '11px', colors: '#7a7f87' },
          rotate: -45,
          rotateAlways: sortedDays.length > 7,
        },
      },
      yaxis: [
        {
          title: { text: 'Volume ($)', style: { fontSize: '11px', color: '#7a7f87' } },
          labels: {
            style: { fontSize: '11px', colors: '#7a7f87' },
            formatter: v => '$' + Number(v).toLocaleString(),
          },
        },
        {
          opposite: true,
          title: { text: 'Fees ($)', style: { fontSize: '11px', color: '#16a34a' } },
          labels: {
            style: { fontSize: '11px', colors: '#16a34a' },
            formatter: v => '$' + Number(v).toLocaleString(),
          },
        },
      ],
      grid: { borderColor: '#e5e7eb', strokeDashArray: 3 },
      legend: { position: 'top', horizontalAlign: 'right', fontSize: '12px' },
      tooltip: {
        shared: true,
        y: { formatter: v => '$' + Number(v).toLocaleString('en-US', { minimumFractionDigits: 2 }) },
      },
      dataLabels: { enabled: false },
    });
    chart.render();
    state.charts.volume = chart;
  }

  function buildAssetsChart(container, trades, days) {
    if (state.charts.assets) {
      try { state.charts.assets.destroy(); } catch (_) { /* noop */ }
      state.charts.assets = null;
    }
    if (typeof ApexCharts === 'undefined') return renderMessage(container, 'Chart library unavailable', true);
    const filtered = filterByDays(trades, days);
    if (!filtered.length) return renderMessage(container, `No asset trades in last ${days} days`);

    const assetMap = {};
    filtered.forEach(t => {
      const name = t.asset_title || t.asset_name || (t.asset_id ? `Asset ${String(t.asset_id).slice(0, 8)}` : 'Unknown');
      if (!assetMap[name]) assetMap[name] = { count: 0, volume: 0, asset_id: t.asset_id || null };
      assetMap[name].count += 1;
      assetMap[name].volume += (t.price_cents || 0) * (t.quantity || 0);
    });

    const sorted = Object.entries(assetMap).sort((a, b) => b[1].volume - a[1].volume).slice(0, 8);

    // Edge case: <3 distinct assets → show ranked list instead of misleading 1-bar chart.
    if (sorted.length < 3) {
      container.replaceChildren();
      const list = document.createElement('div');
      list.className = 'mp-asset-rank-list';
      const total = sorted.reduce((s, [, v]) => s + v.volume, 0) || 1;
      sorted.forEach(([name, v], i) => {
        const row = document.createElement('a');
        row.className = 'mp-asset-rank-row';
        row.href = v.asset_id
          ? `/admin/marketplace/orders.html?asset_id=${encodeURIComponent(v.asset_id)}`
          : '/admin/marketplace/orders.html';
        const pct = (v.volume / total) * 100;
        row.innerHTML = `
          <span class="mp-asset-rank-num">${i + 1}</span>
          <span class="mp-asset-rank-name" title="${name}">${name.length > 30 ? name.slice(0, 27) + '…' : name}</span>
          <span class="mp-asset-rank-bar"><span style="width:${pct.toFixed(1)}%"></span></span>
          <span class="mp-asset-rank-value">${fmtUSD(v.volume)}</span>
          <span class="mp-asset-rank-count">${v.count} trade${v.count === 1 ? '' : 's'}</span>`;
        list.appendChild(row);
      });
      const note = document.createElement('div');
      note.className = 'mp-analytics-empty';
      note.style.cssText = 'padding:12px;font-size:12px;text-align:left;';
      note.textContent = `Need ≥3 traded assets in this period for chart view. Showing ${sorted.length} as ranked list.`;
      container.appendChild(list);
      container.appendChild(note);
      return;
    }

    const labels = sorted.map(([n]) => n.length > 28 ? n.slice(0, 25) + '…' : n);
    const values = sorted.map(([, v]) => +(v.volume / 100).toFixed(2));
    const counts = sorted.map(([, v]) => v.count);

    container.replaceChildren();
    const chart = new ApexCharts(container, {
      chart: {
        type: 'bar', height: 280,
        fontFamily: "'TT Norms Pro', sans-serif",
        toolbar: { show: true, tools: { download: true, selection: false, zoom: false, zoomin: false, zoomout: false, pan: false, reset: false } },
        background: 'transparent',
      },
      series: [{ name: 'Volume', data: values }],
      plotOptions: { bar: { horizontal: true, borderRadius: 4, barHeight: '60%' } },
      colors: ['#0000FF'],
      xaxis: {
        categories: labels,
        labels: {
          style: { fontSize: '11px', colors: '#7a7f87' },
          formatter: v => '$' + Number(v).toLocaleString(),
        },
      },
      yaxis: { labels: { style: { fontSize: '12px', colors: '#181d27' } } },
      grid: { borderColor: '#e5e7eb', strokeDashArray: 3 },
      tooltip: {
        y: {
          formatter: (v, opts) => {
            const idx = opts && opts.dataPointIndex != null ? opts.dataPointIndex : 0;
            const c = counts[idx] || 0;
            return '$' + Number(v).toLocaleString('en-US', { minimumFractionDigits: 2 }) + ` · ${c} trade${c === 1 ? '' : 's'}`;
          },
        },
      },
      dataLabels: { enabled: false },
    });
    chart.render();
    state.charts.assets = chart;
  }

  // ─── RECENT TRADES TABLE ─────────────────────────────

  function statusPill(status) {
    const s = (status || '').toLowerCase();
    const ok = ['confirmed', 'settled', 'success', 'completed'];
    const fail = ['failed', 'reverted', 'cancelled', 'canceled', 'error'];
    const cls = ok.includes(s) ? 'mp-pill--ok' : fail.includes(s) ? 'mp-pill--fail' : 'mp-pill--pending';
    return `<span class="mp-pill ${cls}">${status || '—'}</span>`;
  }

  function buildTradesTable(container, trades) {
    if (!trades.length) return renderMessage(container, 'No trades yet');
    const top = trades.slice(0, 10);
    const selSet = state.bulkSelection || new Set();
    const rows = top.map(t => {
      const total = t.total_cents != null ? t.total_cents : (t.price_cents || 0) * (t.quantity || 0);
      const ts = t.executed_at || t.created_at;
      const asset = t.asset_title || t.asset_name || 'Unknown';
      const tid = String(t.id || '');
      const checked = selSet.has(tid) ? 'checked' : '';
      return `
        <tr>
          <td class="mp-bulk-checkbox-col"><input type="checkbox" data-trade-id="${tid}" aria-label="Select trade" ${checked}/></td>
          <td><span title="${ts || ''}">${fmtRelative(ts)}</span></td>
          <td class="truncate" title="${asset}">${asset}</td>
          <td class="num">${fmtNum(t.quantity || 0)}</td>
          <td class="num">${fmtUSDfull(t.price_cents || 0)}</td>
          <td class="num"><strong>${fmtUSDfull(total)}</strong></td>
          <td class="num">${fmtUSDfull(t.fee_cents || 0)}</td>
          <td>${statusPill(t.on_chain_status)}</td>
        </tr>`;
    }).join('');
    container.innerHTML = `
      <table class="mp-trades-table">
        <thead>
          <tr>
            <th class="mp-bulk-checkbox-col"><input type="checkbox" id="mp-bulk-select-all" aria-label="Select all"/></th>
            <th>Time</th>
            <th>Asset</th>
            <th class="num">Qty</th>
            <th class="num">Price</th>
            <th class="num">Total</th>
            <th class="num">Fee</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>`;
    // Wire select-all.
    const all = container.querySelector('#mp-bulk-select-all');
    if (all) all.addEventListener('change', () => {
      container.querySelectorAll('input[type="checkbox"][data-trade-id]').forEach(cb => {
        cb.checked = all.checked;
        cb.dispatchEvent(new Event('change', { bubbles: true }));
      });
    });
  }

  // ─── FRAUD / RISK SIGNALS ────────────────────────────

  function computeFraudSignals(trades) {
    if (!trades.length) return null;
    const userVolume = {}; // user_id → cents
    const buyerVolume = {};
    const pairCount = {};  // "buyer_id::seller_id" → count
    let selfMatchCount = 0;
    let totalVolume = 0;
    const tradeSizes = [];

    for (const t of trades) {
      const total = t.total_cents != null ? t.total_cents : (t.price_cents || 0) * (t.quantity || 0);
      totalVolume += total;
      tradeSizes.push(total);
      if (t.buyer_id) {
        buyerVolume[t.buyer_id] = (buyerVolume[t.buyer_id] || 0) + total;
        userVolume[t.buyer_id] = (userVolume[t.buyer_id] || 0) + total;
      }
      if (t.seller_id) {
        userVolume[t.seller_id] = (userVolume[t.seller_id] || 0) + total;
      }
      if (t.buyer_id && t.seller_id) {
        if (t.buyer_id === t.seller_id) selfMatchCount += 1;
        const k = `${t.buyer_id}::${t.seller_id}`;
        pairCount[k] = (pairCount[k] || 0) + 1;
      }
    }

    const topBuyer = Object.entries(buyerVolume).sort((a, b) => b[1] - a[1])[0];
    const topBuyerPct = topBuyer && totalVolume > 0 ? (topBuyer[1] / totalVolume) * 100 : 0;
    const topPair = Object.entries(pairCount).sort((a, b) => b[1] - a[1])[0];
    const repeatPairCount = topPair ? topPair[1] : 0;
    const sortedSizes = tradeSizes.slice().sort((a, b) => a - b);
    const median = sortedSizes.length ? sortedSizes[Math.floor(sortedSizes.length / 2)] : 0;
    const max = sortedSizes.length ? sortedSizes[sortedSizes.length - 1] : 0;
    const outlierFactor = median > 0 ? (max / median) : 0;

    return {
      selfMatchCount,
      topBuyerPct,
      repeatPairCount,
      outlierFactor,
      tradeCount: trades.length,
      uniqueParticipants: Object.keys(userVolume).length,
    };
  }

  function buildFraudGrid(container, trades) {
    const sig = computeFraudSignals(trades);
    if (!sig) {
      renderMessage(container, 'No trade data');
      return;
    }
    const cards = [
      {
        title: 'Self-Match Trades',
        value: fmtNum(sig.selfMatchCount),
        meta: 'Buyer === seller',
        flagged: sig.selfMatchCount > 0,
      },
      {
        title: 'Top Buyer Concentration',
        value: sig.topBuyerPct.toFixed(1) + '%',
        meta: `of ${sig.tradeCount} trades volume`,
        flagged: sig.topBuyerPct > 50,
      },
      {
        title: 'Repeat Buyer→Seller Pair',
        value: fmtNum(sig.repeatPairCount),
        meta: 'Same buyer + seller',
        flagged: sig.repeatPairCount >= 5,
      },
      {
        title: 'Trade Size Outlier',
        value: sig.outlierFactor > 0 ? sig.outlierFactor.toFixed(1) + '×' : '—',
        meta: 'Max vs median',
        flagged: sig.outlierFactor > 20,
      },
      {
        title: 'Unique Participants',
        value: fmtNum(sig.uniqueParticipants),
        meta: `over ${sig.tradeCount} trades`,
        flagged: sig.tradeCount > 0 && sig.uniqueParticipants <= 3,
      },
    ];

    container.innerHTML = cards.map(c => `
      <div class="mp-fraud-card${c.flagged ? ' flagged' : ''}">
        <div class="mp-fraud-card-title">${c.title}</div>
        <div class="mp-fraud-card-value">${c.value}</div>
        <div class="mp-fraud-card-meta">${c.meta}</div>
      </div>
    `).join('');
  }

  // ─── HALT MARKET ─────────────────────────────────────

  async function toggleTrading(enabled) {
    const reason = enabled
      ? 'Resumed via analytics quick-action'
      : window.prompt('Reason for halting trading? (required)', '');
    if (!enabled && !reason) return;
    const verb = enabled ? 'resume' : 'HALT';
    if (!enabled && !window.confirm(`Are you sure you want to ${verb} all marketplace trading?\n\nReason: ${reason}`)) return;

    try {
      const res = await fetch(`${API_BASE}/toggle-trading`, {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled, reason: reason || undefined }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      if (window.MpToast) window.MpToast.success(enabled ? 'Trading resumed' : 'Trading halted');
      else alert(enabled ? 'Trading resumed' : 'Trading halted');
      await refresh();
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Toggle failed';
      if (window.MpToast) window.MpToast.error(`Toggle failed: ${msg}`);
      else alert(`Toggle failed: ${msg}`);
    }
  }

  function syncHaltButtons(stats) {
    const haltBtn = document.getElementById('btn-halt-trading');
    const resumeBtn = document.getElementById('btn-resume-trading');
    if (!haltBtn || !resumeBtn) return;
    const status = (stats && stats.trading_status) || 'UNKNOWN';
    if (status === 'LIVE') {
      haltBtn.style.display = '';
      resumeBtn.style.display = 'none';
    } else if (status === 'HALTED') {
      haltBtn.style.display = 'none';
      resumeBtn.style.display = '';
    } else {
      haltBtn.style.display = '';
      resumeBtn.style.display = '';
    }
  }

  // ─── METABASE ────────────────────────────────────────

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

    const isJinja = s => /^\{\{.*\}\}$/.test((s || '').trim());
    const quicklinks = document.getElementById('analytics-quick-links');
    if (!baseUrl || isJinja(baseUrl) || !publicDashboardPath || isJinja(publicDashboardPath) || !frame) {
      // Hide Metabase card entirely when not configured (saves vertical space).
      card.style.display = 'none';
      if (quicklinks) quicklinks.style.display = 'none';
      return;
    }
    if (quicklinks) quicklinks.style.display = '';

    const frameUrl = `${baseUrl}${publicDashboardPath}`;
    frame.src = frameUrl;
    frame.style.display = 'block';
    if (empty) empty.style.display = 'none';
    if (openBtn && dashboardId) {
      openBtn.disabled = false;
      openBtn.addEventListener('click', () => window.open(`${baseUrl}/dashboard/${dashboardId}`, '_blank', 'noopener,noreferrer'));
    }
    if (refreshBtn) {
      refreshBtn.disabled = false;
      refreshBtn.addEventListener('click', () => { frame.src = frame.src; });
    }
  }

  // ─── REFRESH / TIMERS ────────────────────────────────

  function tickUpdated() {
    const el = document.getElementById('analytics-updated-text');
    if (el && state.lastUpdated) {
      el.textContent = 'Updated ' + fmtRelative(state.lastUpdated);
    }
  }

  function attachPeriodToggles() {
    document.querySelectorAll('.mp-period-toggle').forEach(group => {
      const target = group.dataset.target;
      // Restore active state from persisted prefs.
      const persisted = state.period[target];
      group.querySelectorAll('button').forEach(b => {
        const days = parseInt(b.dataset.period, 10);
        b.classList.toggle('active', days === persisted);
      });
      group.querySelectorAll('button').forEach(btn => {
        btn.addEventListener('click', () => {
          group.querySelectorAll('button').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          const days = parseInt(btn.dataset.period, 10) || 7;
          state.period[target] = days;
          const p = loadPrefs();
          if (target === 'volume') p.periodVolume = days;
          if (target === 'assets') p.periodAssets = days;
          savePrefs(p);
          renderCharts();
        });
      });
    });
  }

  function renderCharts() {
    const volEl = document.getElementById('analytics-volume-chart');
    const assetsEl = document.getElementById('analytics-assets-chart');
    if (volEl)    buildVolumeChart(volEl, state.trades, state.period.volume);
    if (assetsEl) buildAssetsChart(assetsEl, state.trades, state.period.assets);
  }

  async function refresh() {
    const refreshBtn = document.getElementById('analytics-refresh-btn');
    if (refreshBtn) {
      refreshBtn.disabled = true;
      refreshBtn.classList.add('mp-spinning');
    }
    try {
      const [statsResult, tradesResult] = await Promise.all([fetchStats(), fetchTrades(500)]);
      if (statsResult.ok)  state.stats  = statsResult.data;
      if (tradesResult.ok) state.trades = tradesResult.data;

      state.lastUpdated = new Date().toISOString();

      const actionEl = document.getElementById('analytics-action-zone');
      const statsEl = document.getElementById('analytics-stats-grid');
      const tradesEl = document.getElementById('analytics-trades-table');
      const fraudEl = document.getElementById('analytics-fraud-grid');

      if (actionEl) {
        if (statsResult.ok) buildActionZone(actionEl, state.stats);
        else renderMessage(actionEl, `Action data unavailable: ${statsResult.message}`, true);
      }
      if (statsEl) {
        if (statsResult.ok) buildStatsCards(statsEl, state.stats);
        else renderMessage(statsEl, `Stats unavailable: ${statsResult.message}`, true);
      }
      if (statsResult.ok) syncHaltButtons(state.stats);

      if (tradesResult.ok) renderCharts();
      else {
        const volEl = document.getElementById('analytics-volume-chart');
        const assetsEl = document.getElementById('analytics-assets-chart');
        if (volEl)    renderMessage(volEl,    `Trade data unavailable: ${tradesResult.message}`, true);
        if (assetsEl) renderMessage(assetsEl, `Asset data unavailable: ${tradesResult.message}`, true);
      }
      if (tradesEl) {
        if (tradesResult.ok) buildTradesTable(tradesEl, state.trades);
        else renderMessage(tradesEl, `Trades unavailable: ${tradesResult.message}`, true);
      }
      if (fraudEl) {
        if (tradesResult.ok) buildFraudGrid(fraudEl, state.trades);
        else renderMessage(fraudEl, `Risk signals unavailable: ${tradesResult.message}`, true);
      }
      tickUpdated();
    } finally {
      if (refreshBtn) {
        refreshBtn.disabled = false;
        refreshBtn.classList.remove('mp-spinning');
      }
    }
  }

  // ─── INIT ────────────────────────────────────────────

  async function init() {
    const fallbackContainer = document.getElementById('analytics-fallback');
    if (!fallbackContainer) return;
    fallbackContainer.style.display = 'block';
    initMetabase();
    attachPeriodToggles();

    const refreshBtn = document.getElementById('analytics-refresh-btn');
    if (refreshBtn) refreshBtn.addEventListener('click', refresh);

    const exportBtn = document.getElementById('analytics-export-btn');
    if (exportBtn) exportBtn.addEventListener('click', exportTradesCsv);

    // Auto-refresh interval picker.
    const intervalPicker = document.getElementById('analytics-interval-picker');
    if (intervalPicker) {
      intervalPicker.value = String(state.refreshIntervalMs);
      intervalPicker.addEventListener('change', () => {
        const ms = parseInt(intervalPicker.value, 10) || 0;
        state.refreshIntervalMs = ms;
        const p = loadPrefs(); p.refreshIntervalMs = ms; savePrefs(p);
        if (state.refreshTimer) { clearInterval(state.refreshTimer); state.refreshTimer = null; }
        const stateLabel = document.getElementById('analytics-autorefresh-state');
        if (ms > 0) {
          state.refreshTimer = setInterval(refresh, ms);
          if (stateLabel) stateLabel.textContent = ms < 60000 ? `${ms / 1000}s` : `${ms / 60000}m`;
        } else if (stateLabel) {
          stateLabel.textContent = 'off';
        }
      });
    }

    // Custom popover (replaces native title attr where data-pop attribute set).
    initPopover();
    // KPI card drag-reorder.
    initStatsDrag();
    // Custom date-range picker.
    initDateRange();
    // Bulk-action wiring (selection checkboxes + bar).
    initBulkActions();

    const haltBtn = document.getElementById('btn-halt-trading');
    if (haltBtn) haltBtn.addEventListener('click', () => toggleTrading(false));
    const resumeBtn = document.getElementById('btn-resume-trading');
    if (resumeBtn) resumeBtn.addEventListener('click', () => toggleTrading(true));

    const kbdBtn = document.getElementById('btn-kbd-help');
    if (kbdBtn) kbdBtn.addEventListener('click', () => {
      const o = document.getElementById('mp-kbd-overlay');
      if (o) o.classList.toggle('open');
    });

    // Keyboard shortcuts: R=refresh, E=export, ?=help, G+T/O/A=goto.
    const kbdOverlay = document.getElementById('mp-kbd-overlay');
    const closeKbd = () => kbdOverlay && kbdOverlay.classList.remove('open');
    const toggleKbd = () => kbdOverlay && kbdOverlay.classList.toggle('open');
    if (kbdOverlay) {
      kbdOverlay.addEventListener('click', (e) => { if (e.target === kbdOverlay) closeKbd(); });
    }

    let gPending = false;
    let gTimer = null;
    document.addEventListener('keydown', (e) => {
      if (e.target && /input|textarea|select/i.test(e.target.tagName)) return;
      if (e.key === 'Escape')                                  { closeKbd(); gPending = false; return; }
      if (e.key === '?' || (e.shiftKey && e.key === '/'))      { e.preventDefault(); toggleKbd(); return; }
      if (e.key === 'r' || e.key === 'R')                      { e.preventDefault(); refresh(); return; }
      if (e.key === 'e' || e.key === 'E')                      { e.preventDefault(); exportTradesCsv(); return; }
      if (gPending) {
        const dest = { t: '/admin/marketplace/trades.html', o: '/admin/marketplace/orders.html', a: '/admin/marketplace/approvals.html' }[e.key.toLowerCase()];
        gPending = false;
        if (gTimer) { clearTimeout(gTimer); gTimer = null; }
        if (dest) { e.preventDefault(); window.location.href = dest; }
        return;
      }
      if (e.key === 'g' || e.key === 'G') {
        gPending = true;
        gTimer = setTimeout(() => { gPending = false; }, 1500);
      }
    });

    // Pause auto-refresh when tab is hidden, resume on visible.
    document.addEventListener('visibilitychange', () => {
      const stateLabel = document.getElementById('analytics-autorefresh-state');
      if (document.hidden) {
        if (state.refreshTimer) { clearInterval(state.refreshTimer); state.refreshTimer = null; }
        if (stateLabel) stateLabel.textContent = 'paused';
      } else {
        refresh();
        if (!state.refreshTimer) state.refreshTimer = setInterval(refresh, state.refreshIntervalMs);
        if (stateLabel) stateLabel.textContent = 'on';
      }
    });

    await refresh();
    state.refreshTimer = setInterval(refresh, state.refreshIntervalMs);
    state.updatedTimer = setInterval(tickUpdated, 15_000);
  }

  // ─── CSV EXPORT ──────────────────────────────────────

  function csvCell(v) {
    const s = v == null ? '' : String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  }

  function exportTradesCsv() {
    const rows = state.trades || [];
    if (!rows.length) {
      if (window.mpToast) window.mpToast.error('No trades to export');
      return;
    }
    const headers = ['executed_at', 'asset_id', 'asset', 'buyer_email', 'seller_email',
                     'price_usd', 'quantity', 'total_usd', 'fee_usd', 'on_chain_status'];
    const lines = [headers.join(',')];
    rows.forEach(t => {
      const total = t.total_cents != null ? t.total_cents : (t.price_cents || 0) * (t.quantity || 0);
      lines.push([
        t.executed_at || t.created_at || '',
        t.asset_id || '',
        t.asset_title || t.asset_name || '',
        t.buyer_email || '',
        t.seller_email || '',
        ((t.price_cents || 0) / 100).toFixed(2),
        t.quantity || 0,
        (total / 100).toFixed(2),
        ((t.fee_cents || 0) / 100).toFixed(2),
        t.on_chain_status || '',
      ].map(csvCell).join(','));
    });
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    a.href = url;
    a.download = `marketplace-trades-${stamp}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  // ─── KPI CARD CUSTOMIZATION ─────────────────────────

  const HIDDEN_KEY = 'mp.analytics.hiddenCards.v1';

  function getHiddenCards() {
    try {
      const raw = localStorage.getItem(HIDDEN_KEY);
      const arr = raw ? JSON.parse(raw) : [];
      return new Set(Array.isArray(arr) ? arr : []);
    } catch (_) { return new Set(); }
  }
  function saveHiddenCards(set) {
    try { localStorage.setItem(HIDDEN_KEY, JSON.stringify([...set])); } catch (_) { /* noop */ }
  }
  function hideCard(label) {
    const s = getHiddenCards(); s.add(label); saveHiddenCards(s);
  }
  function unhideCard(label) {
    const s = getHiddenCards(); s.delete(label); saveHiddenCards(s);
  }
  function resetHiddenCards() { saveHiddenCards(new Set()); }

  function renderHiddenBar(allItems, hidden) {
    let bar = document.getElementById('mp-hidden-cards-bar');
    if (!bar) {
      const grid = document.getElementById('analytics-stats-grid');
      if (!grid || !grid.parentElement) return;
      bar = document.createElement('div');
      bar.id = 'mp-hidden-cards-bar';
      bar.className = 'mp-hidden-cards-bar';
      grid.parentElement.insertBefore(bar, grid);
    }
    bar.replaceChildren();
    if (!hidden.size) {
      bar.classList.remove('has-hidden');
      return;
    }
    bar.classList.add('has-hidden');
    const lbl = document.createElement('span');
    lbl.textContent = `Hidden cards: ${hidden.size}`;
    bar.appendChild(lbl);
    [...hidden].forEach(name => {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = `+ ${name}`;
      b.title = `Show ${name}`;
      b.addEventListener('click', () => {
        unhideCard(name);
        // Re-render: caller (buildStatsCards) reads fresh state on next refresh — trigger explicit refresh.
        const grid = document.getElementById('analytics-stats-grid');
        if (grid && state.stats) buildStatsCards(grid, state.stats);
      });
      bar.appendChild(b);
    });
    const reset = document.createElement('button');
    reset.type = 'button';
    reset.textContent = 'Show all';
    reset.addEventListener('click', () => {
      resetHiddenCards();
      const grid = document.getElementById('analytics-stats-grid');
      if (grid && state.stats) buildStatsCards(grid, state.stats);
    });
    bar.appendChild(reset);
  }

  // ─── CUSTOM POPOVER (replaces native title attr) ───

  let popEl = null;
  function initPopover() {
    popEl = document.createElement('div');
    popEl.className = 'mp-popover';
    document.body.appendChild(popEl);

    document.addEventListener('mouseover', (e) => {
      const t = e.target.closest('[data-pop]');
      if (!t) return;
      popEl.textContent = t.dataset.pop;
      popEl.classList.add('open');
      const r = t.getBoundingClientRect();
      popEl.style.left = (window.scrollX + r.left) + 'px';
      popEl.style.top  = (window.scrollY + r.top - popEl.offsetHeight - 8) + 'px';
    });
    document.addEventListener('mouseout', (e) => {
      if (!e.target.closest('[data-pop]')) return;
      popEl.classList.remove('open');
    });
    document.addEventListener('scroll', () => popEl.classList.remove('open'), { passive: true });
  }

  // ─── KPI CARD DRAG REORDER ─────────────────────────

  const ORDER_KEY = 'mp.analytics.cardOrder.v1';
  function loadOrder() {
    try { return JSON.parse(localStorage.getItem(ORDER_KEY)) || []; }
    catch (_) { return []; }
  }
  function saveOrder(arr) {
    try { localStorage.setItem(ORDER_KEY, JSON.stringify(arr)); } catch (_) { /* noop */ }
  }
  // Expose for buildStatsCards consumer.
  window._mpApplyCardOrder = function (items) {
    const order = loadOrder();
    if (!order.length) return items;
    const idx = (label) => {
      const i = order.indexOf(label);
      return i === -1 ? 999 : i;
    };
    return [...items].sort((a, b) => idx(a.label) - idx(b.label));
  };

  function initStatsDrag() {
    const grid = document.getElementById('analytics-stats-grid');
    if (!grid) return;
    let dragging = null;
    let scrollRaf = null;
    const SCROLL_EDGE_PX = 60;
    const SCROLL_SPEED = 12;

    function autoScroll(clientY) {
      cancelAnimationFrame(scrollRaf);
      const vh = window.innerHeight;
      let dy = 0;
      if (clientY < SCROLL_EDGE_PX) dy = -SCROLL_SPEED * (1 - clientY / SCROLL_EDGE_PX);
      else if (clientY > vh - SCROLL_EDGE_PX) dy = SCROLL_SPEED * (1 - (vh - clientY) / SCROLL_EDGE_PX);
      if (dy === 0) return;
      const tick = () => {
        window.scrollBy(0, dy);
        scrollRaf = requestAnimationFrame(tick);
      };
      scrollRaf = requestAnimationFrame(tick);
    }
    function stopScroll() {
      if (scrollRaf) { cancelAnimationFrame(scrollRaf); scrollRaf = null; }
    }

    grid.addEventListener('dragstart', (e) => {
      const card = e.target.closest('.mp-analytics-stat');
      if (!card) return;
      dragging = card;
      card.classList.add('dragging');
      try { e.dataTransfer.effectAllowed = 'move'; e.dataTransfer.setData('text/plain', card.dataset.cardKey || ''); } catch (_) { /* noop */ }
    });
    grid.addEventListener('dragend', () => {
      if (dragging) dragging.classList.remove('dragging');
      grid.querySelectorAll('.drop-target').forEach(el => el.classList.remove('drop-target'));
      stopScroll();
      dragging = null;
    });
    grid.addEventListener('dragover', (e) => {
      const target = e.target.closest('.mp-analytics-stat');
      autoScroll(e.clientY);
      if (!target || target === dragging) return;
      e.preventDefault();
      grid.querySelectorAll('.drop-target').forEach(el => el.classList.remove('drop-target'));
      target.classList.add('drop-target');
    });
    grid.addEventListener('drop', (e) => {
      e.preventDefault();
      stopScroll();
      const target = e.target.closest('.mp-analytics-stat');
      if (!target || !dragging || target === dragging) return;
      const rect = target.getBoundingClientRect();
      const before = (e.clientY - rect.top) < rect.height / 2;
      grid.insertBefore(dragging, before ? target : target.nextSibling);
      const order = [...grid.querySelectorAll('.mp-analytics-stat')]
        .map(el => el.dataset.cardKey)
        .filter(Boolean);
      saveOrder(order);
      // Pinned items reorder among themselves; preserve unpinned order.
      // Re-derive pinned set from new DOM order so pin-set stays in-sync.
      const pinned = getPinned();
      const newPinnedOrder = order.filter(k => pinned.has(k));
      // Persist pin-order as a parallel ordered set (extension key).
      try { localStorage.setItem('mp.analytics.pinnedOrder.v1', JSON.stringify(newPinnedOrder)); } catch (_) { /* noop */ }
      grid.querySelectorAll('.drop-target').forEach(el => el.classList.remove('drop-target'));
    });
    document.addEventListener('dragend', stopScroll);
  }

  // ─── VOLUME CHART SUBTITLE ─────────────────────────

  function renderVolumeSubtitle(days) {
    const card = document.getElementById('analytics-volume-chart');
    if (!card || !card.parentElement) return;
    // When custom date range active, period-delta is meaningless — hide subtitle.
    if (state.customRange && state.customRange.start && state.customRange.end) {
      const existing = card.parentElement.querySelector('.mp-chart-subtitle');
      if (existing) existing.remove();
      return;
    }
    const cutoff = Date.now() - days * 86_400_000;
    const prevCutoff = cutoff - days * 86_400_000;
    let cur = 0, prev = 0;
    (state.trades || []).forEach(t => {
      const ts = Date.parse(t.executed_at || t.created_at || '');
      if (!Number.isFinite(ts)) return;
      const v = (t.price_cents || 0) * (t.quantity || 0);
      if (ts >= cutoff) cur += v;
      else if (ts >= prevCutoff) prev += v;
    });
    const d = pctDelta(cur, prev);
    const cls = d.dir === 'up' ? 'mp-delta-up' : d.dir === 'down' ? 'mp-delta-down' : 'mp-delta-flat';
    let sub = card.parentElement.querySelector('.mp-chart-subtitle');
    if (!sub) {
      sub = document.createElement('div');
      sub.className = 'mp-chart-subtitle';
      card.parentElement.insertBefore(sub, card);
    }
    sub.innerHTML = `<span>Total ${days}d: <strong>${fmtUSD(cur)}</strong></span> <span class="${cls}">${deltaArrow(d.dir)} ${d.text} vs prev ${days}d (${fmtUSD(prev)})</span>`;
  }

  // Hook into renderCharts via DOM mutation observer is overkill — just re-call after render.
  const _origRenderCharts = (typeof renderCharts === 'function') ? renderCharts : null;
  if (_origRenderCharts) {
    // Wrap in a delayed call after charts paint.
    document.addEventListener('readystatechange', () => {
      // No-op; renderVolumeSubtitle invoked manually below at every refresh tick.
    });
  }

  // Patch refresh to also draw subtitle.
  const _origRefresh = (typeof refresh === 'function') ? refresh : null;
  // Direct hook is unsafe (closure reference) — instead, observe stats grid changes via MutationObserver.
  const obs = new MutationObserver(() => {
    if (state.trades && state.trades.length) renderVolumeSubtitle(state.period.volume);
  });
  document.addEventListener('DOMContentLoaded', () => {
    const grid = document.getElementById('analytics-stats-grid');
    if (grid) obs.observe(grid, { childList: true });
  });

  // ─── RECENT TRADES SORT + FILTER ───────────────────

  // Wrap buildTradesTable post-render to attach click handlers + apply filter.
  const _origBuildTrades = buildTradesTable;
  buildTradesTable = function (container, trades) {
    // Apply status filter.
    let filtered = trades;
    if (state.tradesFilter.status !== 'all') {
      filtered = trades.filter(t => String(t.on_chain_status || '').toLowerCase() === state.tradesFilter.status);
    }
    // Apply sort.
    const dir = state.tradesFilter.sortDir === 'asc' ? 1 : -1;
    const cmp = {
      time:   (a, b) => (Date.parse(a.executed_at || a.created_at || 0) - Date.parse(b.executed_at || b.created_at || 0)) * dir,
      asset:  (a, b) => String(a.asset_title || '').localeCompare(String(b.asset_title || '')) * dir,
      qty:    (a, b) => ((a.quantity || 0) - (b.quantity || 0)) * dir,
      price:  (a, b) => ((a.price_cents || 0) - (b.price_cents || 0)) * dir,
      total:  (a, b) => {
        const ta = a.total_cents != null ? a.total_cents : (a.price_cents || 0) * (a.quantity || 0);
        const tb = b.total_cents != null ? b.total_cents : (b.price_cents || 0) * (b.quantity || 0);
        return (ta - tb) * dir;
      },
      fee:    (a, b) => ((a.fee_cents || 0) - (b.fee_cents || 0)) * dir,
      status: (a, b) => String(a.on_chain_status || '').localeCompare(String(b.on_chain_status || '')) * dir,
    }[state.tradesFilter.sortKey] || ((a, b) => 0);
    filtered = [...filtered].sort(cmp);

    // Inject filter dropdown into the table-header if not present.
    const wrap = container.closest('.mp-trades-table-wrap');
    if (wrap) {
      const header = wrap.querySelector('.mp-trades-table-header');
      if (header && !header.querySelector('.mp-trades-filter-bar')) {
        const bar = document.createElement('div');
        bar.className = 'mp-trades-filter-bar';
        bar.innerHTML = `
          <select id="mp-trades-status-filter" aria-label="Filter by status">
            <option value="all">All statuses</option>
            <option value="confirmed">Confirmed</option>
            <option value="settled">Settled</option>
            <option value="completed">Completed</option>
            <option value="pending">Pending</option>
            <option value="failed">Failed</option>
            <option value="reverted">Reverted</option>
          </select>`;
        // Insert before "View all"
        header.insertBefore(bar, header.lastElementChild);
        bar.querySelector('select').addEventListener('change', (e) => {
          state.tradesFilter.status = e.target.value;
          if (state.trades) buildTradesTable(container, state.trades);
        });
      }
      // Sync select value.
      const sel = header.querySelector('#mp-trades-status-filter');
      if (sel) sel.value = state.tradesFilter.status;
    }

    _origBuildTrades(container, filtered);

    // Wire sort click on headers.
    const ths = container.querySelectorAll('.mp-trades-table th');
    const SORT_KEYS = [null, 'time', 'asset', 'qty', 'price', 'total', 'fee', 'status'];
    ths.forEach((th, i) => {
      const key = SORT_KEYS[i];
      if (!key) return;
      th.dataset.sort = key;
      th.classList.toggle('asc', state.tradesFilter.sortKey === key && state.tradesFilter.sortDir === 'asc');
      th.classList.toggle('desc', state.tradesFilter.sortKey === key && state.tradesFilter.sortDir === 'desc');
      th.addEventListener('click', () => {
        if (state.tradesFilter.sortKey === key) {
          state.tradesFilter.sortDir = state.tradesFilter.sortDir === 'asc' ? 'desc' : 'asc';
        } else {
          state.tradesFilter.sortKey = key;
          state.tradesFilter.sortDir = 'desc';
        }
        if (state.trades) buildTradesTable(container, state.trades);
      });
    });
  };

  // ─── CUSTOM DATE RANGE ─────────────────────────────

  function initDateRange() {
    const startEl = document.getElementById('mp-daterange-start');
    const endEl = document.getElementById('mp-daterange-end');
    const clearBtn = document.getElementById('mp-daterange-clear');
    if (!startEl || !endEl) return;
    state.customRange = state.customRange || { start: null, end: null };
    const apply = () => {
      state.customRange.start = startEl.value || null;
      state.customRange.end = endEl.value || null;
      const p = loadPrefs(); p.customRange = state.customRange; savePrefs(p);
      renderCharts();
    };
    // Restore from prefs.
    if (prefs.customRange) {
      state.customRange = prefs.customRange;
      if (prefs.customRange.start) startEl.value = prefs.customRange.start;
      if (prefs.customRange.end) endEl.value = prefs.customRange.end;
    }
    startEl.addEventListener('change', apply);
    endEl.addEventListener('change', apply);
    if (clearBtn) clearBtn.addEventListener('click', () => {
      startEl.value = ''; endEl.value = '';
      state.customRange = { start: null, end: null };
      const p = loadPrefs(); p.customRange = null; savePrefs(p);
      renderCharts();
    });
  }

  // ─── BULK ACTIONS ──────────────────────────────────

  function initBulkActions() {
    const bar = document.getElementById('mp-trades-bulk-bar');
    const tbl = document.getElementById('analytics-trades-table');
    if (!bar || !tbl) return;
    state.bulkSelection = new Set();

    const refreshSelection = () => {
      const count = state.bulkSelection.size;
      bar.classList.toggle('active', count > 0);
      const c = bar.querySelector('.mp-bulk-count');
      if (c) c.textContent = `${count} selected`;
    };

    tbl.addEventListener('change', (e) => {
      const cb = e.target.closest('input[type="checkbox"][data-trade-id]');
      if (!cb) return;
      if (cb.checked) state.bulkSelection.add(cb.dataset.tradeId);
      else state.bulkSelection.delete(cb.dataset.tradeId);
      refreshSelection();
    });

    bar.addEventListener('click', async (e) => {
      const btn = e.target.closest('button[data-bulk-action]');
      if (!btn) return;
      const action = btn.dataset.bulkAction;
      const ids = [...state.bulkSelection];
      if (action === 'clear') {
        state.bulkSelection.clear();
        tbl.querySelectorAll('input[type="checkbox"][data-trade-id]').forEach(c => c.checked = false);
        refreshSelection();
        return;
      }
      if (action === 'export') {
        const sel = (state.trades || []).filter(t => ids.includes(String(t.id)));
        if (!sel.length) return;
        const orig = state.trades; state.trades = sel; exportTradesCsv(); state.trades = orig;
        return;
      }
      if (action === 'retry') {
        if (!ids.length) return;
        if (!confirm(`Retry on-chain settlement for ${ids.length} trade(s)?\n\nOnly trades in failed/reverted/timeout state will be reset to pending.`)) return;
        try {
          const res = await fetch(`${API_BASE}/trades/bulk-retry-onchain`, {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ trade_ids: ids, reason: 'Admin bulk retry from analytics' }),
          });
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const out = await res.json();
          if (window.mpToast) window.mpToast.success(`Retried ${out.reset}/${out.requested} trades (${out.eligible} eligible).`);
          else alert(`Retried ${out.reset}/${out.requested} (${out.eligible} eligible).`);
          state.bulkSelection.clear();
          tbl.querySelectorAll('input[type="checkbox"][data-trade-id]').forEach(c => c.checked = false);
          refreshSelection();
          // Trigger fresh data fetch to reflect updated statuses.
          const refreshBtn = document.getElementById('analytics-refresh-btn');
          if (refreshBtn) refreshBtn.click();
        } catch (err) {
          const msg = err && err.message ? err.message : String(err);
          if (window.mpToast) window.mpToast.error(`Retry failed: ${msg}`);
          else alert(`Retry failed: ${msg}`);
        }
        return;
      }
      if (action === 'cancel') {
        if (window.mpToast) window.mpToast.info('Cancel not applicable to settled trades. Use the Orders page to cancel open orders.');
        else alert('Cancel not applicable to settled trades.');
      }
    });
  }

  // ─── PIN SUPPORT ───────────────────────────────────

  const PIN_KEY = 'mp.analytics.pinnedCards.v1';
  function getPinned() {
    try { return new Set(JSON.parse(localStorage.getItem(PIN_KEY)) || []); }
    catch (_) { return new Set(); }
  }
  function savePinned(set) {
    try { localStorage.setItem(PIN_KEY, JSON.stringify([...set])); } catch (_) { /* noop */ }
  }
  // Expose for buildStatsCards.
  window._mpApplyPinning = function (items) {
    const pinned = getPinned();
    if (!pinned.size) return items;
    let pinOrder = [];
    try { pinOrder = JSON.parse(localStorage.getItem('mp.analytics.pinnedOrder.v1')) || []; }
    catch (_) { pinOrder = []; }
    const idx = (label) => {
      const i = pinOrder.indexOf(label);
      return i === -1 ? 999 : i;
    };
    const pinnedItems = items.filter(it => pinned.has(it.label)).sort((a, b) => idx(a.label) - idx(b.label));
    const unpinned = items.filter(it => !pinned.has(it.label));
    return [...pinnedItems, ...unpinned];
  };
  window._mpAttachPinButton = function (card, label) {
    const pinned = getPinned();
    const isP = pinned.has(label);
    card.classList.toggle('pinned', isP);
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'mp-stat-pin-btn';
    btn.title = isP ? 'Unpin' : 'Pin to top';
    btn.setAttribute('aria-pressed', String(isP));
    btn.innerHTML = isP
      ? '<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l2 6h6l-5 4 2 7-5-4-5 4 2-7-5-4h6z"/></svg>'
      : '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l2 6h6l-5 4 2 7-5-4-5 4 2-7-5-4h6z"/></svg>';
    btn.addEventListener('click', (ev) => {
      ev.preventDefault(); ev.stopPropagation();
      const s = getPinned();
      if (s.has(label)) s.delete(label); else s.add(label);
      savePinned(s);
      const grid = document.getElementById('analytics-stats-grid');
      if (grid && state.stats) buildStatsCards(grid, state.stats);
    });
    card.appendChild(btn);
  };

  // ─── GLOBAL RANGE PICKER ─────────────────────────────

  const RANGE_KEY = 'mp.analytics.rangeDays.v1';
  function applyRange(days, custom) {
    state.rangeDays = days;
    state.customRangePill = custom || null;
    try { localStorage.setItem(RANGE_KEY, JSON.stringify({ days, custom })); } catch (_) { /* noop */ }
    state.period.volume = days;
    state.period.assets = days;
    if (state.trades) renderCharts();
  }
  function loadRangePref() {
    try {
      const raw = localStorage.getItem(RANGE_KEY);
      if (!raw) return;
      const v = JSON.parse(raw);
      if (typeof v.days === 'number') state.rangeDays = v.days;
      if (v.custom) state.customRangePill = v.custom;
    } catch (_) { /* noop */ }
  }
  function initRangePicker() {
    loadRangePref();
    const pills = document.getElementById('mp-range-pills');
    const customWrap = document.getElementById('mp-range-custom');
    const fromEl = document.getElementById('mp-range-from');
    const toEl   = document.getElementById('mp-range-to');
    const apply  = document.getElementById('mp-range-apply');
    if (!pills) return;
    const active = state.customRangePill ? 'custom' : String(state.rangeDays || 1);
    pills.querySelectorAll('button').forEach(b => b.classList.toggle('active', b.dataset.range === active));
    if (customWrap && active === 'custom') customWrap.classList.add('open');
    pills.addEventListener('click', (e) => {
      const btn = e.target.closest('button[data-range]');
      if (!btn) return;
      pills.querySelectorAll('button').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      if (btn.dataset.range === 'custom') {
        if (customWrap) customWrap.classList.add('open');
        return;
      }
      if (customWrap) customWrap.classList.remove('open');
      applyRange(parseInt(btn.dataset.range, 10) || 1, null);
    });
    if (apply && fromEl && toEl) {
      apply.addEventListener('click', () => {
        if (!fromEl.value || !toEl.value) return;
        const fromMs = Date.parse(fromEl.value);
        const toMs   = Date.parse(toEl.value);
        if (!Number.isFinite(fromMs) || !Number.isFinite(toMs) || toMs < fromMs) return;
        const days = Math.max(1, Math.ceil((toMs - fromMs) / 86_400_000));
        applyRange(days, { from: fromEl.value, to: toEl.value, days });
      });
    }
  }

  // ─── ASSET SEGMENT FILTER ────────────────────────────

  function populateAssetFilter() {
    const sel = document.getElementById('mp-asset-filter-volume');
    if (!sel) return;
    const seen = new Set();
    const opts = [];
    (state.trades || []).forEach(t => {
      const id = t.asset_id || '';
      const name = t.asset_title || t.asset_name || (id ? `Asset ${String(id).slice(0,8)}` : null);
      if (!id || !name || seen.has(id)) return;
      seen.add(id);
      opts.push({ id, name });
    });
    opts.sort((a, b) => a.name.localeCompare(b.name));
    const currentValue = sel.value || '';
    sel.replaceChildren();
    sel.appendChild(new Option('All assets', ''));
    opts.forEach(o => sel.appendChild(new Option(o.name, o.id)));
    if ([...sel.options].some(o => o.value === currentValue)) sel.value = currentValue;
  }
  function initAssetFilter() {
    const sel = document.getElementById('mp-asset-filter-volume');
    if (!sel) return;
    sel.addEventListener('change', () => {
      state.assetFilter = sel.value || null;
      renderCharts();
    });
  }
  const _origRenderCharts2 = renderCharts;
  renderCharts = function () {
    const volEl = document.getElementById('analytics-volume-chart');
    const assetsEl = document.getElementById('analytics-assets-chart');
    const filtered = state.assetFilter
      ? (state.trades || []).filter(t => t.asset_id === state.assetFilter)
      : (state.trades || []);
    if (volEl)    buildVolumeChart(volEl, filtered, state.period.volume);
    if (assetsEl) buildAssetsChart(assetsEl, state.trades, state.period.assets);
  };

  // ─── RECENT ORDERS ───────────────────────────────────

  async function fetchOrders(limit = 10) {
    const r = await fetchJson(`${API_BASE}/orders?status=open&per_page=${limit}`);
    if (!r.ok) return r;
    const p = r.data;
    if (Array.isArray(p)) return { ok: true, data: p };
    if (Array.isArray(p.data)) return { ok: true, data: p.data };
    if (Array.isArray(p.orders)) return { ok: true, data: p.orders };
    return { ok: true, data: [] };
  }
  function buildOrdersTable(container, orders) {
    if (!orders.length) return renderMessage(container, 'No open orders');
    const top = orders.slice(0, 10);
    const rows = top.map(o => {
      const total = (o.price_cents || 0) * (o.quantity || 0);
      const filledPct = o.quantity > 0 ? Math.round(((o.quantity_filled || 0) / o.quantity) * 100) : 0;
      const sideClass = (o.side === 'buy') ? 'mp-pill--ok' : 'mp-pill--fail';
      return `
        <tr>
          <td><span title="${o.created_at || ''}">${fmtRelative(o.created_at)}</span></td>
          <td class="truncate" title="${o.asset_name || ''}">${o.asset_name || 'Unknown'}</td>
          <td><span class="mp-pill ${sideClass}">${(o.side || '').toUpperCase()}</span></td>
          <td>${o.order_type || '—'}</td>
          <td class="num">${fmtNum(o.quantity || 0)}</td>
          <td class="num">${fmtUSDfull(o.price_cents || 0)}</td>
          <td class="num"><strong>${fmtUSDfull(total)}</strong></td>
          <td class="num">${filledPct}%</td>
          <td class="truncate" title="${o.user_email || ''}">${o.user_email || '—'}</td>
        </tr>`;
    }).join('');
    container.innerHTML = `
      <table class="mp-trades-table">
        <thead><tr>
          <th>Created</th><th>Asset</th><th>Side</th><th>Type</th>
          <th class="num">Qty</th><th class="num">Price</th><th class="num">Total</th>
          <th class="num">Filled</th><th>User</th>
        </tr></thead>
        <tbody>${rows}</tbody>
      </table>`;
  }

  // ─── SYSTEM HEALTH BAR ───────────────────────────────

  async function fetchHealth() { return fetchJson(`${API_BASE}/health`); }
  function renderHealthBar(h) {
    const bar = document.getElementById('mp-health-bar');
    if (!bar) return;
    bar.hidden = false;
    const set = (id, txt) => { const el = document.getElementById(id); if (el) el.textContent = txt; };
    const dot = (id, tier) => { const el = document.getElementById(id); if (el) el.className = `mp-health-dot mp-health-dot--${tier}`; };
    const dbTier = h.database_connected
      ? (h.database_latency_ms < 50 ? 'ok' : h.database_latency_ms < 200 ? 'warn' : 'alert')
      : 'alert';
    set('mp-health-db', `${(h.database_latency_ms || 0).toFixed(1)}ms`);
    dot('mp-health-db-dot', dbTier);
    const redisTier = h.redis_connected ? 'ok' : 'warn';
    set('mp-health-redis', h.redis_connected ? `${(h.redis_latency_ms || 0).toFixed(1)}ms` : 'offline');
    dot('mp-health-redis-dot', redisTier);
    const engTier = h.matching_engine_status === 'healthy' ? 'ok' : h.matching_engine_status === 'degraded' ? 'warn' : 'alert';
    set('mp-health-engine', h.matching_engine_status || '—');
    dot('mp-health-engine-dot', engTier);
    set('mp-health-queue', fmtNum(h.order_queue_depth || 0));
    set('mp-health-last-trade', h.last_trade_at ? fmtRelative(h.last_trade_at) : '—');
  }

  // ─── COMMAND PALETTE ─────────────────────────────────

  const CMDK_ITEMS_STATIC = [
    { label: 'Go to Marketplace Overview', meta: 'Page',   action: () => location.href = '/admin/marketplace/' },
    { label: 'Go to Orders',               meta: 'Page',   action: () => location.href = '/admin/marketplace/orders.html' },
    { label: 'Go to Trades',               meta: 'Page',   action: () => location.href = '/admin/marketplace/trades.html' },
    { label: 'Go to Reconciliation',       meta: 'Page',   action: () => location.href = '/admin/marketplace/reconciliation.html' },
    { label: 'Go to Alerts',               meta: 'Page',   action: () => location.href = '/admin/marketplace/alerts.html' },
    { label: 'Go to Settings',             meta: 'Page',   action: () => location.href = '/admin/marketplace/settings.html' },
    { label: 'Refresh data',               meta: 'Action', action: () => refresh() },
    { label: 'Export trades CSV',          meta: 'Action', action: () => exportTradesCsv() },
    { label: 'Halt Market',                meta: 'Action', action: () => toggleTrading(false) },
    { label: 'Resume Trading',             meta: 'Action', action: () => toggleTrading(true) },
    { label: 'Show keyboard shortcuts',    meta: 'Help',   action: () => { const o = document.getElementById('mp-kbd-overlay'); if (o) o.classList.add('open'); } },
    { label: 'Reset hidden cards',         meta: 'Action', action: () => { resetHiddenCards(); const g = document.getElementById('analytics-stats-grid'); if (g && state.stats) buildStatsCards(g, state.stats); } },
  ];
  function getCmdkItems() {
    const seen = new Set();
    const dyn = [];
    (state.trades || []).forEach(t => {
      const id = t.asset_id;
      const name = t.asset_title || t.asset_name;
      if (!id || !name || seen.has(id)) return;
      seen.add(id);
      dyn.push({
        label: `Filter charts by: ${name}`,
        meta: 'Asset',
        action: () => {
          state.assetFilter = id;
          const sel = document.getElementById('mp-asset-filter-volume');
          if (sel) sel.value = id;
          renderCharts();
        },
      });
    });
    return [...CMDK_ITEMS_STATIC, ...dyn];
  }
  function openCmdk() {
    const m = document.getElementById('mp-cmdk');
    const input = document.getElementById('mp-cmdk-input');
    if (!m || !input) return;
    m.classList.add('open');
    input.value = '';
    renderCmdkList('');
    setTimeout(() => input.focus(), 50);
  }
  function closeCmdk() {
    const m = document.getElementById('mp-cmdk');
    if (m) m.classList.remove('open');
  }
  function renderCmdkList(query) {
    const list = document.getElementById('mp-cmdk-list');
    if (!list) return;
    const q = query.trim().toLowerCase();
    const items = getCmdkItems().filter(it => !q || it.label.toLowerCase().includes(q));
    if (!items.length) {
      list.innerHTML = '<div class="mp-cmdk-empty">No matches</div>';
      return;
    }
    list.replaceChildren();
    items.slice(0, 30).forEach((it, i) => {
      const el = document.createElement('div');
      el.className = 'mp-cmdk-item' + (i === 0 ? ' active' : '');
      el.innerHTML = `<span>${it.label}</span><span class="mp-cmdk-item-meta">${it.meta}</span>`;
      el.addEventListener('click', () => { closeCmdk(); it.action(); });
      list.appendChild(el);
    });
  }
  function initCmdk() {
    const input = document.getElementById('mp-cmdk-input');
    const m = document.getElementById('mp-cmdk');
    const btn = document.getElementById('btn-cmdk');
    if (btn) btn.addEventListener('click', openCmdk);
    if (m) m.addEventListener('click', (e) => { if (e.target === m) closeCmdk(); });
    if (input) {
      input.addEventListener('input', (e) => renderCmdkList(e.target.value));
      input.addEventListener('keydown', (e) => {
        const list = document.getElementById('mp-cmdk-list');
        const items = list ? [...list.querySelectorAll('.mp-cmdk-item')] : [];
        const idx = items.findIndex(it => it.classList.contains('active'));
        if (e.key === 'ArrowDown') { e.preventDefault(); if (items[idx]) items[idx].classList.remove('active'); const n = items[Math.min(idx + 1, items.length - 1)]; if (n) { n.classList.add('active'); n.scrollIntoView({ block: 'nearest' }); } }
        if (e.key === 'ArrowUp')   { e.preventDefault(); if (items[idx]) items[idx].classList.remove('active'); const n = items[Math.max(idx - 1, 0)]; if (n) { n.classList.add('active'); n.scrollIntoView({ block: 'nearest' }); } }
        if (e.key === 'Enter')     { e.preventDefault(); if (items[idx]) items[idx].click(); }
        if (e.key === 'Escape')    { closeCmdk(); }
      });
    }
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); openCmdk(); }
    });
  }

  // ─── SKELETON LOADERS ────────────────────────────────

  function injectSkeletons() {
    const grid = document.getElementById('analytics-stats-grid');
    if (grid && grid.querySelector('.mp-analytics-empty')) {
      grid.innerHTML = Array(6).fill('<div class="mp-skeleton mp-skeleton-card"></div>').join('');
    }
    const action = document.getElementById('analytics-action-zone');
    if (action && action.querySelector('.mp-analytics-empty')) {
      action.innerHTML = Array(3).fill('<div class="mp-skeleton mp-skeleton-card" style="height:84px;"></div>').join('');
    }
    const fraud = document.getElementById('analytics-fraud-grid');
    if (fraud && fraud.querySelector('.mp-analytics-empty')) {
      fraud.innerHTML = Array(5).fill('<div class="mp-skeleton mp-skeleton-card" style="height:80px;"></div>').join('');
    }
  }

  // ─── HOOK INTO REFRESH FOR ORDERS+HEALTH+ASSETS ──────

  const _origRefresh2 = refresh;
  refresh = async function () {
    await _origRefresh2();
    populateAssetFilter();
    const ordersEl = document.getElementById('analytics-orders-table');
    if (ordersEl) {
      const r = await fetchOrders(10);
      if (r.ok) buildOrdersTable(ordersEl, r.data);
      else renderMessage(ordersEl, `Orders unavailable: ${r.message}`, true);
    }
    const h = await fetchHealth();
    if (h.ok) renderHealthBar(h.data);
  };

  // ─── SINGLE-ASSET TRADING ASSETS DRILL ───────────────

  const _origBuildStatsCards2 = buildStatsCards;
  buildStatsCards = function (container, stats) {
    _origBuildStatsCards2(container, stats);
    if ((stats.total_assets_trading ?? 0) === 1 && state.trades && state.trades.length) {
      const oneAssetId = state.trades[0].asset_id;
      if (oneAssetId) {
        container.querySelectorAll('a.mp-analytics-stat').forEach(a => {
          const lbl = a.querySelector('.mp-analytics-stat-label');
          if (lbl && lbl.textContent === 'Trading Assets') {
            a.href = `/admin/marketplace/orders.html?asset_id=${oneAssetId}`;
            a.title = 'Open the only active asset';
          }
        });
      }
    }
  };

  // ─── LEGEND TOGGLE PERSISTENCE ───────────────────────

  const LEGEND_KEY = 'mp.analytics.legendHidden.v1';
  function loadHiddenSeries() {
    try { return new Set(JSON.parse(localStorage.getItem(LEGEND_KEY)) || []); }
    catch (_) { return new Set(); }
  }
  function saveHiddenSeries(set) {
    try { localStorage.setItem(LEGEND_KEY, JSON.stringify([...set])); } catch (_) { /* noop */ }
  }
  state.hiddenSeries = loadHiddenSeries();
  const _origBuildVolumeChart2 = buildVolumeChart;
  buildVolumeChart = function (container, trades, days) {
    _origBuildVolumeChart2(container, trades, days);
    const chart = state.charts.volume;
    if (!chart) return;
    state.hiddenSeries.forEach(name => { try { chart.hideSeries(name); } catch (_) { /* noop */ } });
    try {
      chart.updateOptions({
        chart: {
          events: {
            legendClick: function (_ctx, seriesIdx) {
              const name = chart.w.config.series[seriesIdx]?.name;
              if (!name) return;
              if (state.hiddenSeries.has(name)) state.hiddenSeries.delete(name);
              else state.hiddenSeries.add(name);
              saveHiddenSeries(state.hiddenSeries);
            },
          },
        },
      }, false, false);
    } catch (_) { /* noop */ }
  };

  document.addEventListener('DOMContentLoaded', () => {
    injectSkeletons();
    initRangePicker();
    initAssetFilter();
    initCmdk();
    const reset = document.getElementById('mp-hidden-reset');
    if (reset) reset.addEventListener('click', () => {
      resetHiddenCards();
      const g = document.getElementById('analytics-stats-grid');
      if (g && state.stats) buildStatsCards(g, state.stats);
    });
  });

  document.addEventListener('DOMContentLoaded', init);
})();
