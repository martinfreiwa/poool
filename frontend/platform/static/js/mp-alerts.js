/**
 * Alerts & Watchlist — mp-alerts.js
 * Alerts pane + Watchlist pane + Rules pane.
 * Filters, KPI-as-filter, auto-refresh, bulk, aging, sparklines, donut, category tabs,
 * expandable rows, snooze (per-row + bulk picker), saved views, browser notifications,
 * favicon badge, assignee + claim, audit trail, command palette, rule editor with test+escalation.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/alerts';
  const API_RULES = '/api/admin/marketplace/alert-rules';
  const API_WATCH = '/api/admin/marketplace/watchlist/v2';
  const API_VIEWS = '/api/admin/marketplace/alert-views';
  const API_HISTORY = '/api/admin/marketplace/alerts/history';
  const REFRESH_MS = 30000;

  async function jfetch(url, opts) {
    const res = await fetch(url, { credentials: 'same-origin', ...opts });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.status === 204 ? null : res.json();
  }
  const LS_SNOOZE = 'poool.alerts.snooze.v1';
  const LS_VIEWS = 'poool.alerts.views.v1';
  const LS_SEEN = 'poool.alerts.seenIds.v1';
  const LS_ASSIGN = 'poool.alerts.assignees.v1';
  const LS_AUDIT = 'poool.alerts.audit.v1';
  const LS_WATCH = 'poool.alerts.watchlist.v1';
  const LS_RULES = 'poool.alerts.rules.v1';

  // Current user (best-effort)
  const ME = (window.userData && (window.userData.email || window.userData.name)) || 'admin@poool';
  const ME_INITIALS = ME.split(/[@.\s]/)[0].slice(0, 2).toUpperCase();

  let alerts = [];
  let usingMockData = false;
  let lastFetchedAt = null;
  let refreshTimer = null;
  let tickTimer = null;
  let selected = new Set();
  let expanded = new Set();
  let snoozeMap = loadJSON(LS_SNOOZE, {});
  let savedViews = loadJSON(LS_VIEWS, []);
  let seenIds = new Set(loadJSON(LS_SEEN, []));
  let assignees = loadJSON(LS_ASSIGN, {}); // { alertId: "user@x" }
  let audit = loadJSON(LS_AUDIT, {});       // { alertId: [{at, by, action}] }
  let watchlist = loadJSON(LS_WATCH, []);
  let rules = loadJSON(LS_RULES, []);
  let baseFavicon = null;
  let editingRuleIdx = -1;

  const state = {
    pane: 'alerts',
    search: '',
    severity: '',
    type: '',
    status: '',
    assignee: '',
    chip: 'all',
    kpi: 'all',
    category: 'all',
  };

  const CATEGORY_RULES = {
    trading: ['wash', 'price manipulation', 'large order', 'unusual pattern', 'spoofing', 'volume'],
    compliance: ['kyc', 'aml', 'sanction', 'compliance'],
    system: ['settlement', 'negative balance', 'api abuse', 'rate limit', 'system'],
  };

  const MOCK_ALERTS = [
    { id: 'ALR-001', severity: 'critical', alert_type: 'Wash Trading', message: 'Same IP address executing buy/sell on BVRT within 30s window', user_id: 'USR-8291', created_at: new Date(Date.now() - 12 * 60 * 1000).toISOString(), status: 'new', metadata: { rule: 'IP_REUSE_30S', baseline_volume: 1200, observed_volume: 4800, deviation_pct: 300 } },
    { id: 'ALR-002', severity: 'critical', alert_type: 'Negative Balance', message: 'User wallet balance went negative after concurrent order execution', user_id: 'USR-3384', created_at: new Date(Date.now() - 45 * 60 * 1000).toISOString(), status: 'new', metadata: { wallet: '0xab12…ff90', delta: -42.18 } },
    { id: 'ALR-003', severity: 'warning', alert_type: 'Price Manipulation', message: 'Rapid price movement on SWHS — 15% spike in 5 minutes', user_id: 'USR-6643', created_at: new Date(Date.now() - 2 * 3600 * 1000).toISOString(), status: 'new', metadata: { asset: 'SWHS', spike_pct: 15, window_min: 5 } },
    { id: 'ALR-004', severity: 'warning', alert_type: 'Large Order', message: 'Single order exceeds 10% of daily volume on JOTX', user_id: 'USR-1738', created_at: new Date(Date.now() - 3 * 3600 * 1000).toISOString(), status: 'acknowledged', metadata: { asset: 'JOTX', order_pct_of_dv: 11.4 } },
    { id: 'ALR-005', severity: 'warning', alert_type: 'Unusual Pattern', message: 'User placing and cancelling orders repeatedly (potential spoofing)', user_id: 'USR-5561', created_at: new Date(Date.now() - 5 * 3600 * 1000).toISOString(), status: 'acknowledged' },
    { id: 'ALR-006', severity: 'critical', alert_type: 'Settlement Failure', message: 'On-chain settlement for trade TRD-100042 failed after 3 retries', user_id: 'USR-2201', created_at: new Date(Date.now() - 6 * 3600 * 1000).toISOString(), status: 'new', metadata: { trade: 'TRD-100042', retries: 3 } },
    { id: 'ALR-007', severity: 'warning', alert_type: 'KYC Mismatch', message: 'Trading user has pending KYC re-verification — tier 2 required for volume', user_id: 'USR-7829', created_at: new Date(Date.now() - 8 * 3600 * 1000).toISOString(), status: 'resolved' },
    { id: 'ALR-008', severity: 'warning', alert_type: 'API Abuse', message: 'Rate limit exceeded — 500+ API calls in 1 minute from same session', user_id: 'USR-4410', created_at: new Date(Date.now() - 12 * 3600 * 1000).toISOString(), status: 'resolved' },
  ];

  // ── Persistence ─────────────────────────────────────────────────
  function loadJSON(key, fallback) {
    try { const v = localStorage.getItem(key); return v ? JSON.parse(v) : fallback; }
    catch { return fallback; }
  }
  function saveJSON(key, val) { try { localStorage.setItem(key, JSON.stringify(val)); } catch { /* quota */ } }
  function pruneSnoozes() {
    const now = Date.now();
    let changed = false;
    for (const id of Object.keys(snoozeMap)) {
      if (snoozeMap[id] !== -1 && snoozeMap[id] <= now) { delete snoozeMap[id]; changed = true; }
    }
    if (changed) saveJSON(LS_SNOOZE, snoozeMap);
  }

  // ── Helpers ─────────────────────────────────────────────────────
  function ageSeconds(a) {
    const t = a.created_at ? new Date(a.created_at).getTime() : Date.now();
    return Math.max(0, (Date.now() - t) / 1000);
  }
  function timeAgo(dateStr) {
    if (!dateStr) return '—';
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }
  function isAging(a) {
    if (['resolved', 'false_positive'].includes(a.status)) return false;
    const sec = ageSeconds(a);
    if ((a.severity || '').toLowerCase() === 'critical' && sec > 3600) return true;
    if ((a.severity || '').toLowerCase() === 'warning' && sec > 86400) return true;
    return false;
  }
  function isCritAged(a) {
    return (a.severity || '').toLowerCase() === 'critical'
      && !['resolved', 'false_positive'].includes(a.status)
      && ageSeconds(a) > 3600;
  }
  function isSnoozed(a) {
    const exp = snoozeMap[a.id];
    if (exp == null) return false;
    if (exp === -1) return !['resolved', 'false_positive'].includes(a.status);
    return exp > Date.now();
  }
  function categorize(a) {
    const t = (a.alert_type || '').toLowerCase();
    for (const [cat, keys] of Object.entries(CATEGORY_RULES)) {
      if (keys.some(k => t.includes(k))) return cat;
    }
    return 'anomaly';
  }
  function severityIcon(sev) {
    const s = (sev || 'info').toLowerCase();
    if (s === 'critical') return '<span aria-hidden="true" style="font-size:10px;">●</span>';
    if (s === 'warning') return '<span aria-hidden="true" style="font-size:10px;">▲</span>';
    return '<span aria-hidden="true" style="font-size:10px;">■</span>';
  }
  function severityBadge(sev) {
    const s = (sev || 'info').toLowerCase();
    const label = s === 'critical' ? 'Critical' : s === 'warning' ? 'Warning' : 'Info';
    const cls = s === 'critical' ? 'admin-badge--danger' : s === 'warning' ? 'admin-badge--warning' : 'admin-badge--info';
    return `<span class="admin-badge ${cls}" role="img" aria-label="${label} severity">${severityIcon(sev)} ${label}</span>`;
  }
  function statusBadge(status) {
    const map = {
      new: ['admin-badge--danger', 'New', '✱'],
      acknowledged: ['admin-badge--warning', 'Acknowledged', '◐'],
      resolved: ['admin-badge--success', 'Resolved', '✓'],
      false_positive: ['admin-badge--neutral', 'False Pos', '⊘'],
    };
    const [cls, label, icon] = map[status] || map.resolved;
    return `<span class="admin-badge ${cls}" role="img" aria-label="Status ${label}"><span aria-hidden="true">${icon}</span> ${label}</span>`;
  }
  function escapeHtml(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }
  function setText(id, val) { const el = document.getElementById(id); if (el) el.textContent = val; }
  function on(id, ev, fn) { const el = document.getElementById(id); if (el) el.addEventListener(ev, fn); }
  function formatDuration(sec) {
    if (sec < 60) return Math.floor(sec) + 's';
    if (sec < 3600) return Math.floor(sec / 60) + 'm';
    if (sec < 86400) return Math.floor(sec / 3600) + 'h';
    return Math.floor(sec / 86400) + 'd';
  }
  function logAudit(id, action) {
    if (!audit[id]) audit[id] = [];
    audit[id].unshift({ at: new Date().toISOString(), by: ME, action });
    audit[id] = audit[id].slice(0, 20);
    saveJSON(LS_AUDIT, audit);
  }

  // ── Filtering ───────────────────────────────────────────────────
  function applyFilters(list, opts = {}) {
    const q = state.search.trim().toLowerCase();
    return list.filter(a => {
      const sev = (a.severity || '').toLowerCase();
      const status = a.status || '';
      const type = a.alert_type || '';

      if (state.severity && sev !== state.severity) return false;
      if (state.type && type !== state.type) return false;
      if (state.status && status !== state.status) return false;

      if (state.assignee === '__me' && assignees[a.id] !== ME) return false;
      if (state.assignee === '__unassigned' && assignees[a.id]) return false;

      if (state.kpi === 'critical' && sev !== 'critical') return false;
      if (state.kpi === 'unresolved' && ['resolved', 'false_positive'].includes(status)) return false;

      if (state.chip === 'unresolved' && ['resolved', 'false_positive'].includes(status)) return false;
      if (state.chip === 'critical' && sev !== 'critical') return false;
      if (state.chip === 'new' && status !== 'new') return false;
      if (state.chip === 'aging' && !isAging(a)) return false;
      if (state.chip === 'mine' && assignees[a.id] !== ME) return false;

      if (!opts.skipCategory && state.category !== 'all' && categorize(a) !== state.category) return false;
      if (!opts.includeSnoozed && isSnoozed(a)) return false;

      if (q) {
        const hay = `${a.id} ${type} ${a.message || ''} ${a.user_id || ''}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    }).sort((a, b) => {
      const sevRank = s => ({ critical: 0, warning: 1, info: 2 }[(s || '').toLowerCase()] ?? 3);
      const r = sevRank(a.severity) - sevRank(b.severity);
      if (r !== 0) return r;
      return new Date(b.created_at || 0) - new Date(a.created_at || 0);
    });
  }

  // ── Sparklines ──────────────────────────────────────────────────
  // History from server-side matview, falls back to in-memory bucketing.
  let historyCache = null;
  async function loadHistory() {
    try {
      const rows = await jfetch(`${API_HISTORY}?days=7`);
      if (Array.isArray(rows)) historyCache = rows;
    } catch (e) { historyCache = null; }
  }
  function bucketsFromHistory(severityFilter) {
    if (!historyCache) return null;
    const buckets = new Array(7).fill(0);
    const now = new Date(); now.setHours(0, 0, 0, 0);
    historyCache.forEach(r => {
      if (severityFilter && r.severity !== severityFilter) return;
      const day = new Date(r.day);
      const idx = 6 - Math.floor((now - day) / 86400000);
      if (idx >= 0 && idx < 7) buckets[idx] += r.count;
    });
    return buckets;
  }
  function dailyBuckets(filterFn) {
    const buckets = new Array(7).fill(0);
    const now = Date.now();
    const day = 86400 * 1000;
    alerts.forEach(a => {
      if (filterFn && !filterFn(a)) return;
      const t = a.created_at ? new Date(a.created_at).getTime() : 0;
      if (!t) return;
      const idx = 6 - Math.floor((now - t) / day);
      if (idx >= 0 && idx < 7) buckets[idx]++;
    });
    return buckets;
  }
  function renderSpark(svgId, values, color) {
    const svg = document.getElementById(svgId);
    if (!svg) return;
    const max = Math.max(1, ...values);
    const w = 100, h = 28, n = values.length;
    const stepX = w / Math.max(1, n - 1);
    const points = values.map((v, i) => `${(i * stepX).toFixed(1)},${(h - (v / max) * (h - 4) - 2).toFixed(1)}`).join(' ');
    const area = `M0,${h} L${points.split(' ').join(' L')} L${w},${h} Z`;
    svg.innerHTML = `<path d="${area}" fill="${color}" fill-opacity="0.15" /><polyline points="${points}" fill="none" stroke="${color}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />`;
  }

  // ── Donut ───────────────────────────────────────────────────────
  const DONUT_COLORS = ['#001dca', '#ff5252', '#ffaa00', '#22c55e', '#a855f7', '#06b6d4', '#f97316', '#64748b'];
  function renderDonut() {
    const svg = document.getElementById('alerts-donut');
    const legend = document.getElementById('alerts-breakdown-legend');
    if (!svg || !legend) return;
    const counts = {};
    alerts.forEach(a => { const t = a.alert_type || 'Unknown'; counts[t] = (counts[t] || 0) + 1; });
    const entries = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    const total = entries.reduce((s, [, v]) => s + v, 0);
    if (total === 0) {
      svg.innerHTML = '<circle cx="50" cy="50" r="40" fill="none" stroke="var(--admin-border)" stroke-width="14"/>';
      legend.innerHTML = '<li style="color:var(--admin-text-muted);">No data yet</li>';
      return;
    }
    const cx = 50, cy = 50, r = 40, c = 2 * Math.PI * r;
    let offset = 0;
    let segs = '<circle cx="50" cy="50" r="40" fill="none" stroke="var(--admin-border)" stroke-width="14"/>';
    entries.forEach(([, v], i) => {
      const frac = v / total;
      const dash = c * frac;
      segs += `<circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="${DONUT_COLORS[i % DONUT_COLORS.length]}" stroke-width="14" stroke-dasharray="${dash.toFixed(2)} ${(c - dash).toFixed(2)}" stroke-dashoffset="${(-offset).toFixed(2)}" transform="rotate(-90 ${cx} ${cy})"/>`;
      offset += dash;
    });
    segs += `<text x="50" y="48" text-anchor="middle" font-size="14" font-weight="700" fill="var(--admin-text-primary)">${total}</text>`;
    segs += `<text x="50" y="60" text-anchor="middle" font-size="8" fill="var(--admin-text-muted)">total</text>`;
    svg.innerHTML = segs;
    legend.innerHTML = entries.map(([t, v], i) =>
      `<li><span class="swatch" style="background:${DONUT_COLORS[i % DONUT_COLORS.length]};"></span>${escapeHtml(t)} <span class="pct">${v} · ${Math.round(v / total * 100)}%</span></li>`
    ).join('');
  }

  // ── Render alerts pane ─────────────────────────────────────────
  function render() {
    pruneSnoozes();

    const total = alerts.length;
    const critical = alerts.filter(a => (a.severity || '').toLowerCase() === 'critical').length;
    const unresolved = alerts.filter(a => !['resolved', 'false_positive'].includes(a.status)).length;
    const critUnack = alerts.filter(a => (a.severity || '').toLowerCase() === 'critical' && !['resolved', 'false_positive'].includes(a.status)).length;
    const oldestUnack = alerts.filter(a => !['resolved', 'false_positive'].includes(a.status))
      .reduce((max, a) => Math.max(max, ageSeconds(a)), 0);

    setText('kpi-total-alerts', total);
    setText('kpi-critical', critical);
    setText('kpi-unresolved', unresolved);
    setText('kpi-critical-sub', critUnack > 0 ? `${critUnack} unresolved` : 'all clear');
    setText('kpi-unresolved-sub', oldestUnack > 0 ? `oldest: ${formatDuration(oldestUnack)}` : '—');

    const critCard = document.querySelector('.admin-kpi-card[data-filter="critical"]');
    if (critCard) critCard.classList.toggle('admin-kpi-card--alarm', critUnack > 0);

    renderSpark('spark-total', bucketsFromHistory(null) || dailyBuckets(() => true), '#001dca');
    renderSpark('spark-critical', bucketsFromHistory('critical') || dailyBuckets(a => (a.severity || '').toLowerCase() === 'critical'), '#ff5252');
    renderSpark('spark-unresolved', dailyBuckets(a => !['resolved', 'false_positive'].includes(a.status)), '#ffaa00');
    renderDonut();
    populateTypeFilter();
    populateAssigneeFilter();

    document.querySelectorAll('.admin-kpi-card[data-filter]').forEach(el => {
      const active = el.dataset.filter === state.kpi;
      el.classList.toggle('admin-kpi-card--active', active);
      el.setAttribute('aria-pressed', active ? 'true' : 'false');
    });
    document.querySelectorAll('#alerts-quick-chips .admin-chip').forEach(el => {
      el.classList.toggle('is-active', el.dataset.chip === state.chip);
    });
    document.querySelectorAll('.alerts-cat-tab').forEach(el => {
      const active = el.dataset.cat === state.category;
      el.classList.toggle('is-active', active);
      el.setAttribute('aria-selected', active ? 'true' : 'false');
    });

    const preCat = applyFilters(alerts, { skipCategory: true });
    const catCounts = { all: preCat.length, trading: 0, compliance: 0, system: 0, anomaly: 0 };
    preCat.forEach(a => { catCounts[categorize(a)]++; });
    Object.entries(catCounts).forEach(([k, v]) => setText(`cat-count-${k}`, v));

    const filtered = applyFilters(alerts);
    const snoozedCount = alerts.filter(isSnoozed).length;
    setText('alerts-count-label', `${filtered.length} of ${total}${snoozedCount ? ` · ${snoozedCount} snoozed` : ''}`);

    setText('tab-count-alerts', unresolved);
    setText('tab-count-watchlist', watchlist.length);
    setText('tab-count-rules', rules.length);

    // Syndicate fraud cross-link count: critical+trading
    const fraudCount = alerts.filter(a => categorize(a) === 'trading' && (a.severity || '').toLowerCase() === 'critical').length;
    setText('fraud-link-count', fraudCount);

    const tbody = document.getElementById('alerts-body');
    if (!tbody) return;
    if (filtered.length === 0) {
      const isEmptyDataset = total === 0;
      tbody.innerHTML = `
        <tr><td colspan="11" style="padding:0;">
          <div class="admin-empty-state">
            <div class="admin-empty-state-icon">${isEmptyDataset ? '✓' : '🔍'}</div>
            <div class="admin-empty-state-title">${isEmptyDataset ? 'No alerts detected' : 'No alerts match filters'}</div>
            <div class="admin-empty-state-text">
              ${isEmptyDataset ? 'Detection is active. Last scan: ' + (lastFetchedAt ? lastFetchedAt.toLocaleTimeString() : '—') + '.'
          : 'Try clearing filters, switching category tab, or selecting a different KPI card.'}
            </div>
          </div>
        </td></tr>`;
      // Hide actions col header when empty
      const thAct = document.getElementById('th-actions');
      if (thAct) thAct.style.opacity = '0.4';
      updateBulkBar();
      updateFavicon();
      return;
    }
    const thAct = document.getElementById('th-actions');
    if (thAct) thAct.style.opacity = '1';

    tbody.innerHTML = filtered.map(rowHtml).join('');
    bindRowActions();
    updateBulkBar();
    updateFavicon();
    renderSavedViews();
  }

  function rowHtml(a) {
    const id = a.id || '';
    const shortId = usingMockData ? id : (id.length > 8 ? id.substring(0, 8) : id);
    const userHTML = a.user_id
      ? `<code style="font-size:10px;padding:1px 5px;background:var(--admin-code-bg);border-radius:3px;">${escapeHtml(usingMockData ? a.user_id : a.user_id.substring(0, 8))}</code>`
      : '—';
    const detected = timeAgo(a.created_at);
    const isResolved = ['resolved', 'false_positive'].includes(a.status);
    const agingClass = isCritAged(a) ? 'alerts-row--aging-crit' : (isAging(a) ? 'alerts-row--aging-warn' : '');
    const checked = selected.has(id) ? 'checked' : '';
    const disabledCheck = isResolved ? 'disabled' : '';
    const isExp = expanded.has(id);
    const snoozedTag = isSnoozed(a)
      ? `<span class="alerts-snooze-badge" title="Snoozed${snoozeMap[id] === -1 ? ' until resolved' : ' until ' + new Date(snoozeMap[id]).toLocaleTimeString()}">snoozed</span>`
      : '';
    const watchedTag = isWatched(a) ? '<span class="alerts-snooze-badge" style="background:var(--admin-warning-bg);color:var(--admin-warning);" title="Entity is on watchlist">👁 watched</span>' : '';
    const assignee = assignees[id];
    const assigneeHTML = assignee
      ? `<span class="alerts-assignee" title="${escapeHtml(assignee)}"><span class="alerts-avatar">${escapeHtml(assignee.slice(0, 2).toUpperCase())}</span>${escapeHtml(assignee.split('@')[0])}</span>`
      : `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-claim" data-id="${escapeHtml(id)}" style="font-size:11px;padding:2px 8px;">Claim</button>`;

    const main = `
      <tr data-alert-id="${escapeHtml(id)}" class="${agingClass}" style="${isResolved ? 'opacity:0.55;' : ''}">
        <td style="text-align:center;"><input type="checkbox" class="alerts-checkbox alerts-row-check" data-id="${escapeHtml(id)}" ${checked} ${disabledCheck} aria-label="Select alert ${escapeHtml(shortId)}" /></td>
        <td><button class="alerts-expand-btn" data-id="${escapeHtml(id)}" aria-expanded="${isExp}" aria-label="${isExp ? 'Collapse' : 'Expand'} details">${isExp ? '▼' : '▶'}</button></td>
        <td><code style="font-size:11px;padding:2px 6px;background:var(--admin-code-bg);border-radius:4px;">${escapeHtml(shortId)}</code></td>
        <td>${severityBadge(a.severity)}</td>
        <td style="font-weight:600;color:var(--admin-text-primary);">${escapeHtml(a.alert_type || '')}${snoozedTag}${watchedTag}</td>
        <td style="max-width:300px;font-size:12px;">${escapeHtml(a.message || '')}</td>
        <td>${userHTML}</td>
        <td style="font-size:12px;" title="${escapeHtml(a.created_at || '')}">${detected}</td>
        <td>${statusBadge(a.status)}</td>
        <td>${assigneeHTML}</td>
        <td style="text-align:center;">
          <div style="display:flex;gap:6px;justify-content:center;flex-wrap:wrap;">
            ${a.status === 'new' ? `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-ack" data-id="${escapeHtml(id)}">Ack</button>` : ''}
            ${!isResolved ? `<button class="admin-btn admin-btn--success admin-btn--sm btn-resolve" data-id="${escapeHtml(id)}">Resolve</button>` : '<span style="font-size:11px;color:var(--admin-text-muted);">Done</span>'}
            ${!isResolved && !isSnoozed(a) ? `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-snooze-menu" data-id="${escapeHtml(id)}" title="Snooze">💤</button>` : ''}
            ${isSnoozed(a) ? `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-unsnooze" data-id="${escapeHtml(id)}">Unsnooze</button>` : ''}
          </div>
        </td>
      </tr>`;

    if (!isExp) return main;

    const meta = a.metadata || {};
    const metaRows = Object.keys(meta).length
      ? Object.entries(meta).map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(typeof v === 'object' ? JSON.stringify(v) : v)}</dd>`).join('')
      : '<dt>Metadata</dt><dd style="color:var(--admin-text-muted);">No additional metadata available</dd>';
    const trail = (audit[id] || []);
    const trailHTML = trail.length
      ? `<ul class="alerts-audit-list">${trail.map(e => `<li><strong>${escapeHtml(e.action)}</strong> · ${escapeHtml(e.by)} · ${escapeHtml(timeAgo(e.at))}</li>`).join('')}</ul>`
      : '<p style="font-size:11px;color:var(--admin-text-muted);margin:6px 0 0;">No actions logged yet.</p>';

    const detail = `
      <tr class="alerts-row-detail">
        <td colspan="11">
          <div style="display:grid;grid-template-columns:1fr 240px;gap:24px;">
            <div>
              <strong style="font-size:11px;color:var(--admin-text-secondary);text-transform:uppercase;letter-spacing:0.04em;">Detection details</strong>
              <dl class="alerts-detail-grid" style="margin-top:8px;">
                <dt>Alert ID</dt><dd><code>${escapeHtml(id)}</code></dd>
                <dt>Category</dt><dd>${escapeHtml(categorize(a))}</dd>
                <dt>Detected</dt><dd>${escapeHtml(a.created_at || '—')}</dd>
                <dt>User</dt><dd>${a.user_id ? `<code>${escapeHtml(a.user_id)}</code> <button class="admin-btn admin-btn--secondary admin-btn--sm btn-watch-from-alert" data-type="user" data-id="${escapeHtml(a.user_id)}" style="font-size:10px;padding:1px 6px;margin-left:6px;">+ Watch</button>` : '—'}</dd>
                <dt>Full message</dt><dd>${escapeHtml(a.message || '')}</dd>
                ${metaRows}
              </dl>
            </div>
            <div>
              <strong style="font-size:11px;color:var(--admin-text-secondary);text-transform:uppercase;letter-spacing:0.04em;">Audit trail</strong>
              ${trailHTML}
            </div>
          </div>
        </td>
      </tr>`;
    return main + detail;
  }

  let typeFilterPopulated = false;
  function populateTypeFilter() {
    if (typeFilterPopulated) return;
    const sel = document.getElementById('alerts-filter-type');
    if (!sel) return;
    const types = [...new Set(alerts.map(a => a.alert_type).filter(Boolean))].sort();
    if (types.length === 0) return;
    types.forEach(t => { const o = document.createElement('option'); o.value = t; o.textContent = t; sel.appendChild(o); });
    typeFilterPopulated = true;
  }
  function populateAssigneeFilter() {
    const sel = document.getElementById('alerts-filter-assignee');
    if (!sel) return;
    const existing = new Set([...sel.options].map(o => o.value));
    const users = [...new Set(Object.values(assignees))].filter(Boolean);
    users.forEach(u => {
      if (!existing.has(u)) {
        const o = document.createElement('option'); o.value = u; o.textContent = u; sel.appendChild(o);
      }
    });
  }

  // ── Bulk ────────────────────────────────────────────────────────
  function updateBulkBar() {
    const bar = document.getElementById('alerts-bulk-bar');
    const count = document.getElementById('alerts-bulk-count');
    if (!bar || !count) return;
    if (selected.size === 0) bar.classList.remove('is-visible');
    else { bar.classList.add('is-visible'); count.textContent = `${selected.size} selected`; }
    const all = document.getElementById('alerts-select-all');
    if (all) {
      const visibleIds = applyFilters(alerts).filter(a => !['resolved', 'false_positive'].includes(a.status)).map(a => a.id);
      const allChecked = visibleIds.length > 0 && visibleIds.every(id => selected.has(id));
      all.checked = allChecked;
      all.indeterminate = !allChecked && visibleIds.some(id => selected.has(id));
    }
  }

  function bindRowActions() {
    document.querySelectorAll('.btn-ack').forEach(btn => btn.addEventListener('click', () => handleAlertAction(btn.dataset.id, 'acknowledge')));
    document.querySelectorAll('.btn-resolve').forEach(btn => btn.addEventListener('click', () => handleAlertAction(btn.dataset.id, 'resolve')));
    document.querySelectorAll('.btn-snooze-menu').forEach(btn => btn.addEventListener('click', e => openSnoozeMenu(e, btn.dataset.id)));
    document.querySelectorAll('.btn-unsnooze').forEach(btn => btn.addEventListener('click', () => unsnoozeAlert(btn.dataset.id)));
    document.querySelectorAll('.btn-claim').forEach(btn => btn.addEventListener('click', () => claimAlert(btn.dataset.id)));
    document.querySelectorAll('.btn-watch-from-alert').forEach(btn => btn.addEventListener('click', () => quickWatch(btn.dataset.type, btn.dataset.id)));
    document.querySelectorAll('.alerts-row-check').forEach(cb => {
      cb.addEventListener('change', () => {
        if (cb.checked) selected.add(cb.dataset.id); else selected.delete(cb.dataset.id);
        updateBulkBar();
      });
    });
    document.querySelectorAll('.alerts-expand-btn').forEach(btn => {
      btn.addEventListener('click', async () => {
        const id = btn.dataset.id;
        if (expanded.has(id)) {
          expanded.delete(id);
        } else {
          expanded.add(id);
          // Fetch server audit trail when expanding
          if (!usingMockData) {
            try {
              const trail = await jfetch(`${API}/${id}/audit`);
              if (Array.isArray(trail)) {
                audit[id] = trail.map(t => ({
                  at: t.created_at,
                  by: t.by_user_email || t.by_user_id,
                  action: t.action,
                }));
                saveJSON(LS_AUDIT, audit);
              }
            } catch (e) { /* keep local */ }
          }
        }
        render();
      });
    });
  }

  // ── Snooze ──────────────────────────────────────────────────────
  function openSnoozeMenu(ev, id) {
    document.querySelectorAll('.alerts-snooze-menu').forEach(m => m.remove());
    const menu = document.createElement('div');
    menu.className = 'alerts-snooze-menu';
    menu.innerHTML = `
      <button data-ms="3600000">1 hour</button>
      <button data-ms="14400000">4 hours</button>
      <button data-ms="86400000">24 hours</button>
      <button data-ms="-1">Until resolved</button>
    `;
    document.body.appendChild(menu);
    const r = ev.currentTarget.getBoundingClientRect();
    menu.style.top = `${r.bottom + window.scrollY + 4}px`;
    menu.style.left = `${r.left + window.scrollX - 80}px`;
    menu.querySelectorAll('button').forEach(b => {
      b.addEventListener('click', () => {
        const ms = parseInt(b.dataset.ms);
        snoozeAlert(id, ms);
        menu.remove();
      });
    });
    setTimeout(() => {
      const close = e => { if (!menu.contains(e.target)) { menu.remove(); document.removeEventListener('click', close); } };
      document.addEventListener('click', close);
    }, 0);
  }
  async function snoozeAlert(id, ms) {
    snoozeMap[id] = ms === -1 ? -1 : Date.now() + ms;
    saveJSON(LS_SNOOZE, snoozeMap);
    selected.delete(id);
    logAudit(id, ms === -1 ? 'snoozed (until resolved)' : `snoozed ${formatDuration(ms / 1000)}`);
    if (!usingMockData) {
      try {
        await jfetch(`${API}/${id}/snooze`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ minutes: ms === -1 ? -1 : Math.round(ms / 60000) }),
        });
      } catch (e) { console.warn('snooze API failed, kept local:', e); }
    }
    if (typeof mpToast === 'function') mpToast(`Alert snoozed`, 'info');
    render();
  }
  async function unsnoozeAlert(id) {
    delete snoozeMap[id];
    saveJSON(LS_SNOOZE, snoozeMap);
    logAudit(id, 'unsnoozed');
    if (!usingMockData) {
      try {
        await jfetch(`${API}/${id}/snooze`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ minutes: 0 }),
        });
      } catch (e) { console.warn('unsnooze API failed, kept local:', e); }
    }
    render();
  }

  // ── Assign ──────────────────────────────────────────────────────
  async function claimAlert(id) {
    assignees[id] = ME;
    saveJSON(LS_ASSIGN, assignees);
    logAudit(id, `claimed by ${ME}`);
    if (!usingMockData) {
      try { await jfetch(`${API}/${id}/claim`, { method: 'POST' }); }
      catch (e) { console.warn('claim API failed, kept local:', e); }
    }
    if (typeof mpToast === 'function') mpToast('Alert claimed', 'success');
    render();
  }
  async function bulkClaim() {
    const ids = [...selected];
    ids.forEach(id => { assignees[id] = ME; logAudit(id, `claimed by ${ME}`); });
    saveJSON(LS_ASSIGN, assignees);
    if (!usingMockData) {
      for (const id of ids) {
        try { await jfetch(`${API}/${id}/claim`, { method: 'POST' }); } catch { }
      }
    }
    selected.clear();
    if (typeof mpToast === 'function') mpToast('Claimed', 'success');
    render();
  }

  // ── Actions ─────────────────────────────────────────────────────
  async function handleAlertAction(id, action) {
    const idx = alerts.findIndex(a => a.id === id);
    if (idx === -1) return;
    if (!usingMockData) {
      try {
        const res = await fetch(`${API}/${id}`, {
          method: 'POST', credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action }),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const result = await res.json();
        alerts[idx].status = result.status;
        selected.delete(id);
        logAudit(id, action);
        render();
        if (typeof mpToast === 'function') mpToast(`Alert ${action}d`, 'success');
        return;
      } catch (err) {
        if (typeof mpToast === 'function') mpToast(`Failed: ${err.message}`, 'error');
        return;
      }
    }
    alerts[idx].status = action === 'acknowledge' ? 'acknowledged' : 'resolved';
    selected.delete(id);
    logAudit(id, action);
    render();
    if (typeof mpToast === 'function') mpToast(`Alert ${id} ${action}d`, 'success');
  }
  async function bulkAction(action) {
    const ids = [...selected];
    if (ids.length === 0) return;
    if (!usingMockData && ids.length > 1) {
      try {
        await jfetch(`${API}/bulk`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ ids, action }),
        });
        ids.forEach(id => {
          const idx = alerts.findIndex(a => a.id === id);
          if (idx !== -1) alerts[idx].status = action === 'acknowledge' ? 'acknowledged' : (action === 'resolve' ? 'resolved' : 'false_positive');
          logAudit(id, `bulk_${action}`);
        });
        selected.clear();
        if (typeof mpToast === 'function') mpToast(`${ids.length} alerts ${action}d`, 'success');
        render();
        return;
      } catch (e) {
        console.warn('bulk API failed, falling back per-row:', e);
      }
    }
    for (const id of ids) await handleAlertAction(id, action);
    selected.clear();
    render();
  }
  function bulkSnooze(ms) {
    [...selected].forEach(id => {
      snoozeMap[id] = ms === -1 ? -1 : Date.now() + ms;
      logAudit(id, ms === -1 ? 'snoozed (until resolved)' : `snoozed ${formatDuration(ms / 1000)}`);
    });
    saveJSON(LS_SNOOZE, snoozeMap);
    if (typeof mpToast === 'function') mpToast(`Snoozed ${selected.size} alerts`, 'info');
    selected.clear();
    render();
  }

  // ── Export ──────────────────────────────────────────────────────
  function exportCsv() {
    const rows = applyFilters(alerts);
    const headers = ['id', 'severity', 'type', 'message', 'user_id', 'created_at', 'status', 'category', 'assignee'];
    const csv = [headers.join(',')].concat(
      rows.map(a => headers.map(h => {
        let v = h === 'type' ? a.alert_type : h === 'category' ? categorize(a) : h === 'assignee' ? (assignees[a.id] || '') : a[h];
        v = v == null ? '' : String(v).replace(/"/g, '""');
        return `"${v}"`;
      }).join(','))
    ).join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = `alerts-${new Date().toISOString().slice(0, 10)}.csv`; a.click();
    URL.revokeObjectURL(url);
    if (typeof mpToast === 'function') mpToast(`Exported ${rows.length} alerts`, 'success');
  }

  // ── Saved views ─────────────────────────────────────────────────
  function renderSavedViews() {
    const list = document.getElementById('alerts-views-list');
    if (!list) return;
    if (savedViews.length === 0) { list.innerHTML = '<span style="font-size:11px;color:var(--admin-text-muted);font-style:italic;">none yet</span>'; return; }
    list.innerHTML = savedViews.map((v, i) => `
      <span class="alerts-view-chip" data-view-idx="${i}" title="${escapeHtml(viewSummary(v))}">
        <span class="alerts-view-load">${escapeHtml(v.name)}</span>
        <button class="alerts-view-chip-x" data-del-idx="${i}" aria-label="Delete view">✕</button>
      </span>`).join('');
    list.querySelectorAll('.alerts-view-load').forEach(el => {
      el.addEventListener('click', () => loadView(savedViews[parseInt(el.parentElement.dataset.viewIdx)]));
    });
    list.querySelectorAll('.alerts-view-chip-x').forEach(el => {
      el.addEventListener('click', async e => {
        e.stopPropagation();
        const idx = parseInt(el.dataset.delIdx);
        const v = savedViews[idx];
        if (v && v.id) {
          try { await jfetch(`${API_VIEWS}/${v.id}`, { method: 'DELETE' }); }
          catch (err) { console.warn('view delete API failed, removing locally:', err); }
        }
        savedViews.splice(idx, 1);
        saveJSON(LS_VIEWS, savedViews);
        renderSavedViews();
      });
    });
  }
  function viewSummary(v) {
    return Object.entries(v.state).filter(([, val]) => val && val !== 'all').map(([k, val]) => `${k}=${val}`).join(', ') || 'all';
  }
  function loadView(v) { Object.assign(state, v.state); syncFiltersToUI(); render(); }
  function syncFiltersToUI() {
    const get = id => document.getElementById(id);
    if (get('alerts-search')) get('alerts-search').value = state.search;
    if (get('alerts-filter-severity')) get('alerts-filter-severity').value = state.severity;
    if (get('alerts-filter-type')) get('alerts-filter-type').value = state.type;
    if (get('alerts-filter-status')) get('alerts-filter-status').value = state.status;
    if (get('alerts-filter-assignee')) get('alerts-filter-assignee').value = state.assignee;
  }
  async function saveCurrentView() {
    const name = prompt('Name this view:', '');
    if (!name) return;
    const entry = { name: name.slice(0, 40), state: { ...state } };
    try {
      const r = await jfetch(API_VIEWS, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: entry.name, state: entry.state }),
      });
      if (r && r.id) entry.id = r.id;
    } catch (e) { console.warn('save-view API failed, kept local:', e); }
    savedViews.push(entry);
    saveJSON(LS_VIEWS, savedViews);
    renderSavedViews();
    if (typeof mpToast === 'function') mpToast(`Saved view "${name}"`, 'success');
  }
  async function loadViewsFromApi() {
    try {
      const list = await jfetch(API_VIEWS);
      if (Array.isArray(list)) {
        savedViews = list.map(v => ({ id: v.id, name: v.name, state: v.state }));
        saveJSON(LS_VIEWS, savedViews);
        renderSavedViews();
      }
    } catch (e) { console.warn('views API unavailable, using local:', e); }
  }

  // ── Notifications + favicon ─────────────────────────────────────
  function checkNewCriticals(prevIds) {
    // Watchlist banner: any new alert hitting a watched entity
    const newWatched = alerts.filter(a => !prevIds.has(a.id) && isWatched(a));
    if (newWatched.length > 0) {
      const sample = newWatched[0];
      if (typeof mpToast === 'function') {
        mpToast(`👁 Watched entity in new alert: ${sample.alert_type}`, 'warning', 6000);
      }
    }
    const newCrits = alerts.filter(a =>
      (a.severity || '').toLowerCase() === 'critical'
      && !['resolved', 'false_positive'].includes(a.status)
      && !prevIds.has(a.id));
    if (newCrits.length === 0) return;
    if ('Notification' in window && Notification.permission === 'granted') {
      const sample = newCrits[0];
      try {
        new Notification(`${newCrits.length} new critical alert${newCrits.length > 1 ? 's' : ''}`, {
          body: `${sample.alert_type}: ${sample.message || ''}`.slice(0, 120),
          tag: 'poool-alerts',
        });
      } catch { /* blocked */ }
    }
    const chime = document.getElementById('alerts-chime');
    if (chime) {
      try {
        const vol = parseInt(localStorage.getItem('poool.alerts.volume') || '60');
        chime.volume = Math.max(0, Math.min(100, vol)) / 100;
        if (chime.volume > 0) { chime.currentTime = 0; chime.play().catch(() => { }); }
      } catch { }
    }
    newCrits.forEach(a => seenIds.add(a.id));
    saveJSON(LS_SEEN, [...seenIds]);
  }
  async function requestNotifyPermission() {
    if (!('Notification' in window)) { mpToast && mpToast('Notifications not supported', 'error'); return; }
    if (Notification.permission === 'granted') { mpToast && mpToast('Already enabled', 'info'); return; }
    const p = await Notification.requestPermission();
    if (p === 'granted') {
      mpToast && mpToast('Notifications enabled', 'success');
      const btn = document.getElementById('alerts-btn-notify');
      if (btn) btn.style.display = 'none';
      subscribeWebPush();
    }
  }
  async function subscribeWebPush() {
    if (!('serviceWorker' in navigator) || !('PushManager' in window)) return;
    try {
      const reg = await navigator.serviceWorker.register('/static/sw-alerts.js');
      // VAPID public key env-injected at build/deploy. Fetch from server endpoint.
      let vapid = null;
      try { vapid = (await jfetch('/api/admin/marketplace/push-vapid-key')).key; } catch { /* not configured */ }
      if (!vapid) { console.info('[mp-alerts] web-push not configured server-side; SW registered for future use'); return; }
      const sub = await reg.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(vapid),
      });
      await jfetch('/api/admin/marketplace/push-subscriptions', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(sub),
      });
    } catch (e) { console.warn('[mp-alerts] web-push subscribe failed:', e); }
  }
  function urlBase64ToUint8Array(b64) {
    const padding = '='.repeat((4 - b64.length % 4) % 4);
    const base64 = (b64 + padding).replace(/-/g, '+').replace(/_/g, '/');
    const raw = atob(base64);
    return Uint8Array.from([...raw].map(c => c.charCodeAt(0)));
  }
  function updateFavicon() {
    const unresolved = alerts.filter(a => !['resolved', 'false_positive'].includes(a.status) && !isSnoozed(a)).length;
    const link = document.querySelector('link[rel="icon"][type="image/svg+xml"]') || document.querySelector('link[rel="icon"]');
    if (!link) return;
    if (!baseFavicon) baseFavicon = link.href;
    if (unresolved === 0) {
      link.href = baseFavicon;
      document.title = 'Alerts & Watchlist | Admin | POOOL';
      return;
    }
    const canvas = document.createElement('canvas');
    canvas.width = 32; canvas.height = 32;
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#001dca';
    ctx.beginPath(); ctx.arc(16, 16, 14, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#ff5252';
    ctx.beginPath(); ctx.arc(24, 8, 8, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#fff';
    ctx.font = 'bold 10px sans-serif';
    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText(unresolved > 9 ? '9+' : String(unresolved), 24, 8);
    link.href = canvas.toDataURL('image/png');
    document.title = `(${unresolved}) Alerts & Watchlist | Admin | POOOL`;
  }

  // ── Live status ─────────────────────────────────────────────────
  function tickLastUpdated() {
    const el = document.getElementById('alerts-last-updated');
    const dot = document.getElementById('alerts-live-dot');
    if (!el || !lastFetchedAt) return;
    const sec = (Date.now() - lastFetchedAt.getTime()) / 1000;
    el.textContent = sec < 5 ? 'Live · just now'
      : sec < 60 ? `Updated ${Math.floor(sec)}s ago`
        : sec < 3600 ? `Updated ${Math.floor(sec / 60)}m ago`
          : `Updated ${Math.floor(sec / 3600)}h ago`;
    if (dot) dot.style.background = sec > 90 ? 'var(--admin-warning)' : 'var(--admin-success)';
  }

  // ── Test alert ──────────────────────────────────────────────────
  function fireTestAlert() {
    const id = 'TEST-' + Date.now().toString(36);
    alerts.unshift({
      id, severity: 'warning', alert_type: 'Test Alert',
      message: 'Synthetic test alert — does not represent a real incident',
      user_id: 'TEST', created_at: new Date().toISOString(), status: 'new',
      metadata: { source: 'manual-test', triggered_by: ME },
    });
    if (typeof mpToast === 'function') mpToast('Test alert fired', 'success');
    render();
  }

  // ── Watchlist ───────────────────────────────────────────────────
  function isWatched(a) {
    return watchlist.some(w =>
      (w.type === 'user' && a.user_id === w.identifier)
      || (w.type === 'asset' && (a.alert_type || '').includes(w.identifier))
      || (w.type === 'wallet' && JSON.stringify(a.metadata || {}).includes(w.identifier))
      || (w.type === 'ip' && JSON.stringify(a.metadata || {}).includes(w.identifier))
    );
  }
  function quickWatch(type, identifier) {
    if (!identifier) return;
    if (watchlist.some(w => w.type === type && w.identifier === identifier)) {
      mpToast && mpToast('Already on watchlist', 'info');
      return;
    }
    watchlist.push({ type, identifier, reason: 'Added from alert', added_at: new Date().toISOString(), added_by: ME });
    saveJSON(LS_WATCH, watchlist);
    mpToast && mpToast(`${identifier} watched`, 'success');
    renderWatchlist();
    render();
  }
  function renderWatchlist() {
    const tbody = document.getElementById('watchlist-body');
    if (!tbody) return;
    setText('tab-count-watchlist', watchlist.length);
    if (watchlist.length === 0) {
      tbody.innerHTML = `<tr><td colspan="6"><div class="alerts-watchlist-empty">
        <div style="font-size:32px;margin-bottom:8px;">👁</div>
        <div style="font-weight:600;color:var(--admin-text-secondary);margin-bottom:4px;">No entities on watchlist</div>
        <div style="font-size:12px;">Add users, wallets, assets, or IPs that need ongoing monitoring.</div>
      </div></td></tr>`;
      return;
    }
    tbody.innerHTML = watchlist.map((w, i) => {
      const linked = w.linked_alerts != null ? w.linked_alerts : alerts.filter(a => isWatched(a) && (
        (w.type === 'user' && a.user_id === w.identifier)
        || (w.type === 'asset' && (a.alert_type || '').includes(w.identifier))
        || (w.type === 'wallet' && JSON.stringify(a.metadata || {}).includes(w.identifier))
        || (w.type === 'ip' && JSON.stringify(a.metadata || {}).includes(w.identifier))
      )).length;
      return `<tr>
        <td><span class="admin-badge admin-badge--info">${escapeHtml(w.type)}</span></td>
        <td><code>${escapeHtml(w.identifier)}</code></td>
        <td style="font-size:12px;color:var(--admin-text-secondary);">${escapeHtml(w.reason || '—')}</td>
        <td style="font-size:12px;color:var(--admin-text-muted);">${escapeHtml(timeAgo(w.added_at))}</td>
        <td><strong>${linked}</strong></td>
        <td style="text-align:center;">
          <button class="admin-btn admin-btn--secondary admin-btn--sm watch-remove" data-idx="${i}">Remove</button>
        </td>
      </tr>`;
    }).join('');
    tbody.querySelectorAll('.watch-remove').forEach(b => {
      b.addEventListener('click', async () => {
        const idx = parseInt(b.dataset.idx);
        const w = watchlist[idx];
        if (w && w.id) {
          try { await jfetch(`${API_WATCH}/${w.id}`, { method: 'DELETE' }); }
          catch (e) { console.warn('watch delete API failed, removing locally:', e); }
        }
        watchlist.splice(idx, 1);
        saveJSON(LS_WATCH, watchlist);
        renderWatchlist();
        render();
      });
    });
  }
  async function loadRulesAndWatchFromApi() {
    try {
      const r = await jfetch(API_RULES);
      if (Array.isArray(r) && r.length) {
        rules = r.map(x => ({ ...x, threshold: x.threshold_text, escalate: x.escalate_after_min }));
        saveJSON(LS_RULES, rules);
      }
    } catch (e) { console.warn('rules API unavailable, using local:', e); }
    try {
      // Prefer enriched endpoint (linked-alerts count + added_by_email)
      let w = null;
      try { w = await jfetch(`${API_WATCH}/enriched`); } catch { w = await jfetch(API_WATCH); }
      if (Array.isArray(w) && w.length) {
        watchlist = w.map(x => ({
          id: x.id,
          type: x.entity_type,
          identifier: x.entity_identifier,
          reason: x.reason,
          added_at: x.created_at,
          added_by: x.added_by_email || x.user_email || '',
          linked_alerts: x.linked_alerts || 0,
        }));
        saveJSON(LS_WATCH, watchlist);
      }
    } catch (e) { console.warn('watchlist API unavailable, using local:', e); }
    renderRules(); renderWatchlist();
  }

  // ── Rules ───────────────────────────────────────────────────────
  function renderRules() {
    const tbody = document.getElementById('rules-body');
    if (!tbody) return;
    setText('tab-count-rules', rules.length);
    if (rules.length === 0) {
      tbody.innerHTML = `<tr><td colspan="8"><div class="alerts-watchlist-empty">
        <div style="font-size:32px;margin-bottom:8px;">⚙</div>
        <div style="font-weight:600;color:var(--admin-text-secondary);margin-bottom:4px;">No detection rules yet</div>
        <div style="font-size:12px;">Click "+ New rule" to define detection thresholds.</div>
      </div></td></tr>`;
      return;
    }
    tbody.innerHTML = rules.map((r, i) => `<tr>
      <td>${r.enabled ? '<span class="admin-badge admin-badge--success">Enabled</span>' : '<span class="admin-badge admin-badge--neutral">Disabled</span>'}</td>
      <td><strong>${escapeHtml(r.name)}</strong></td>
      <td>${escapeHtml(r.category)}</td>
      <td>${severityBadge(r.severity)}</td>
      <td style="font-size:12px;">${escapeHtml(r.threshold || '—')}</td>
      <td style="font-size:12px;">${r.escalate ? r.escalate + ' min' : '—'}</td>
      <td style="font-size:12px;">${escapeHtml(r.channel || 'none')}</td>
      <td style="text-align:center;">
        <div style="display:flex;gap:6px;justify-content:center;">
          <button class="admin-btn admin-btn--secondary admin-btn--sm rule-test" data-idx="${i}">Test</button>
          <button class="admin-btn admin-btn--secondary admin-btn--sm rule-edit" data-idx="${i}">Edit</button>
          <button class="admin-btn admin-btn--danger admin-btn--sm rule-del" data-idx="${i}">Delete</button>
        </div>
      </td>
    </tr>`).join('');
    tbody.querySelectorAll('.rule-test').forEach(b => b.addEventListener('click', () => testRule(parseInt(b.dataset.idx))));
    tbody.querySelectorAll('.rule-edit').forEach(b => b.addEventListener('click', () => openRuleModal(parseInt(b.dataset.idx))));
    tbody.querySelectorAll('.rule-del').forEach(b => b.addEventListener('click', async () => {
      if (!confirm('Delete this rule?')) return;
      const idx = parseInt(b.dataset.idx);
      const r = rules[idx];
      if (r && r.id) {
        try { await jfetch(`${API_RULES}/${r.id}`, { method: 'DELETE' }); }
        catch (e) { console.warn('rule delete API failed, removing locally:', e); }
      }
      rules.splice(idx, 1);
      saveJSON(LS_RULES, rules);
      renderRules();
    }));
  }
  async function testRule(idx) {
    const r = rules[idx];
    if (!r) return;
    if (r.id) {
      try {
        await jfetch(`${API_RULES}/${r.id}/test`, { method: 'POST' });
        mpToast && mpToast(`Rule "${r.name}" test fired`, 'success');
        await loadAlerts();
        state.pane = 'alerts'; switchPane();
        return;
      } catch (e) { console.warn('rule test API failed, faking locally:', e); }
    }
    const id = 'RULE-TEST-' + Date.now().toString(36);
    alerts.unshift({
      id, severity: r.severity, alert_type: r.name + ' (test)',
      message: `Test fire of rule "${r.name}" — threshold: ${r.threshold || 'n/a'}`,
      user_id: 'TEST', created_at: new Date().toISOString(), status: 'new',
      metadata: { source: 'rule-test', rule: r.name, channel: r.channel },
    });
    mpToast && mpToast(`Rule "${r.name}" test fired`, 'success');
    state.pane = 'alerts'; switchPane(); render();
  }
  function expandHourRange(s) {
    if (!s || !s.trim()) return [];
    const out = new Set();
    s.split(',').forEach(part => {
      part = part.trim();
      if (!part) return;
      if (part.includes('-')) {
        const [a, b] = part.split('-').map(n => parseInt(n));
        if (!isNaN(a) && !isNaN(b)) {
          if (a <= b) for (let i = a; i <= b; i++) out.add(i);
          else { for (let i = a; i < 24; i++) out.add(i); for (let i = 0; i <= b; i++) out.add(i); }
        }
      } else {
        const n = parseInt(part);
        if (!isNaN(n)) out.add(n);
      }
    });
    return [...out].filter(h => h >= 0 && h < 24).sort((a, b) => a - b);
  }
  function compactHours(arr) { return (arr || []).join(','); }

  function openRuleModal(idx) {
    editingRuleIdx = idx == null ? -1 : idx;
    const r = idx != null && idx >= 0 ? rules[idx] : { name: '', category: 'trading', severity: 'warning', threshold: '', escalate: '', channel: 'none', enabled: true, mute_schedule: null };
    document.getElementById('rule-modal-title').textContent = idx >= 0 ? 'Edit detection rule' : 'New detection rule';
    document.getElementById('rule-name').value = r.name || '';
    document.getElementById('rule-category').value = r.category || 'trading';
    document.getElementById('rule-severity').value = r.severity || 'warning';
    document.getElementById('rule-threshold').value = r.threshold || r.threshold_text || '';
    document.getElementById('rule-escalate').value = r.escalate || r.escalate_after_min || '';
    document.getElementById('rule-channel').value = r.channel || 'none';
    document.getElementById('rule-enabled').checked = r.enabled !== false;
    const m = r.mute_schedule || {};
    document.getElementById('rule-mute-weekends').checked = !!m.weekends;
    document.getElementById('rule-mute-hours').value = compactHours(m.hours);
    document.getElementById('rule-modal').classList.add('is-open');
  }
  function closeRuleModal() { document.getElementById('rule-modal').classList.remove('is-open'); editingRuleIdx = -1; }
  async function saveRule() {
    const muteHours = expandHourRange(document.getElementById('rule-mute-hours').value);
    const muteWeekends = document.getElementById('rule-mute-weekends').checked;
    const mute_schedule = (muteWeekends || muteHours.length) ? { weekends: muteWeekends, hours: muteHours } : null;
    const ui = {
      name: document.getElementById('rule-name').value.trim(),
      category: document.getElementById('rule-category').value,
      severity: document.getElementById('rule-severity').value,
      threshold_text: document.getElementById('rule-threshold').value.trim(),
      escalate_after_min: parseInt(document.getElementById('rule-escalate').value) || 0,
      channel: document.getElementById('rule-channel').value,
      enabled: document.getElementById('rule-enabled').checked,
      mute_schedule,
    };
    if (!ui.name) { mpToast && mpToast('Rule name required', 'error'); return; }

    const existing = editingRuleIdx >= 0 ? rules[editingRuleIdx] : null;
    let saved = null;
    try {
      if (existing && existing.id) {
        saved = await jfetch(`${API_RULES}/${existing.id}`, {
          method: 'PUT', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(ui),
        });
      } else {
        saved = await jfetch(API_RULES, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(ui),
        });
      }
    } catch (e) {
      console.warn('rule API failed, kept local:', e);
    }

    const r = saved || { ...ui, threshold: ui.threshold_text, escalate: ui.escalate_after_min };
    if (editingRuleIdx >= 0) rules[editingRuleIdx] = r; else rules.push(r);
    saveJSON(LS_RULES, rules);
    closeRuleModal();
    renderRules();
    mpToast && mpToast(saved ? 'Rule saved' : 'Rule saved (local only)', 'success');
  }

  // ── Watchlist modal ─────────────────────────────────────────────
  function openWatchModal() { document.getElementById('watch-modal').classList.add('is-open'); }
  function closeWatchModal() { document.getElementById('watch-modal').classList.remove('is-open'); }
  async function saveWatchEntry() {
    const type = document.getElementById('watch-type').value;
    const id = document.getElementById('watch-id').value.trim();
    const reason = document.getElementById('watch-reason').value.trim();
    if (!id) { mpToast && mpToast('Identifier required', 'error'); return; }
    let serverId = null;
    try {
      const r = await jfetch(API_WATCH, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ entity_type: type, identifier: id, reason }),
      });
      serverId = r && r.id;
    } catch (e) { console.warn('watchlist API failed, kept local:', e); }
    watchlist.push({ id: serverId, type, identifier: id, reason, added_at: new Date().toISOString(), added_by: ME });
    saveJSON(LS_WATCH, watchlist);
    document.getElementById('watch-id').value = '';
    document.getElementById('watch-reason').value = '';
    closeWatchModal();
    renderWatchlist();
    render();
    mpToast && mpToast('Added to watchlist', 'success');
  }

  // ── Pane switching ──────────────────────────────────────────────
  function switchPane() {
    document.querySelectorAll('.alerts-page-tab').forEach(t => {
      const active = t.dataset.pane === state.pane;
      t.classList.toggle('is-active', active);
      t.setAttribute('aria-selected', active ? 'true' : 'false');
    });
    document.querySelectorAll('.alerts-pane').forEach(p => {
      p.classList.toggle('is-active', p.id === `pane-${state.pane}`);
    });
    if (state.pane === 'watchlist') renderWatchlist();
    if (state.pane === 'rules') renderRules();
  }

  // ── Command palette (Cmd+K) ─────────────────────────────────────
  const PALETTE_CMDS = [
    { id: 'refresh', label: 'Refresh alerts', kbd: 'R', run: () => loadAlerts() },
    { id: 'export', label: 'Export CSV', kbd: 'E', run: exportCsv },
    { id: 'test', label: 'Fire test alert', kbd: 'T', run: fireTestAlert },
    { id: 'save-view', label: 'Save current view', run: saveCurrentView },
    { id: 'new-rule', label: 'New detection rule', run: () => { state.pane = 'rules'; switchPane(); openRuleModal(-1); } },
    { id: 'add-watch', label: 'Add to watchlist', run: () => { state.pane = 'watchlist'; switchPane(); openWatchModal(); } },
    { id: 'pane-alerts', label: 'Go to Alerts', run: () => { state.pane = 'alerts'; switchPane(); } },
    { id: 'pane-watchlist', label: 'Go to Watchlist', run: () => { state.pane = 'watchlist'; switchPane(); } },
    { id: 'pane-rules', label: 'Go to Detection Rules', run: () => { state.pane = 'rules'; switchPane(); } },
    { id: 'filter-critical', label: 'Filter: Critical only', run: () => { state.kpi = 'critical'; render(); } },
    { id: 'filter-mine', label: 'Filter: My alerts', run: () => { state.chip = 'mine'; render(); } },
    { id: 'filter-aging', label: 'Filter: Aging >1h', run: () => { state.chip = 'aging'; render(); } },
    { id: 'clear-filters', label: 'Clear all filters', run: () => { Object.assign(state, { search: '', severity: '', type: '', status: '', assignee: '', chip: 'all', kpi: 'all', category: 'all' }); syncFiltersToUI(); render(); } },
    { id: 'enable-notify', label: 'Enable notifications', run: requestNotifyPermission },
  ];
  let paletteIdx = 0;
  function openPalette() {
    document.getElementById('palette').classList.add('is-open');
    const inp = document.getElementById('palette-input');
    inp.value = ''; paletteIdx = 0;
    renderPalette('');
    setTimeout(() => inp.focus(), 30);
  }
  function closePalette() { document.getElementById('palette').classList.remove('is-open'); }
  function renderPalette(q) {
    const list = document.getElementById('palette-list');
    const filtered = PALETTE_CMDS.filter(c => c.label.toLowerCase().includes(q.toLowerCase()));
    if (filtered.length === 0) { list.innerHTML = '<div class="alerts-palette-item" style="color:var(--admin-text-muted);">No matches</div>'; return; }
    if (paletteIdx >= filtered.length) paletteIdx = 0;
    list.innerHTML = filtered.map((c, i) => `
      <div class="alerts-palette-item ${i === paletteIdx ? 'is-active' : ''}" data-cmd="${escapeHtml(c.id)}">
        <span>${escapeHtml(c.label)}</span>${c.kbd ? `<span class="kbd">${escapeHtml(c.kbd)}</span>` : ''}
      </div>`).join('');
    list.querySelectorAll('.alerts-palette-item[data-cmd]').forEach((el, i) => {
      el.addEventListener('mouseenter', () => { paletteIdx = i; renderPalette(q); });
      el.addEventListener('click', () => runPaletteCmd(filtered[i]));
    });
  }
  function runPaletteCmd(cmd) { closePalette(); cmd.run(); }

  // ── Load ────────────────────────────────────────────────────────
  async function loadAlerts() {
    const prevCriticalIds = new Set(seenIds);
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      alerts = await res.json();
      usingMockData = false;
    } catch (err) {
      console.warn('[mp-alerts] API unavailable, using mock data:', err);
      alerts = [...MOCK_ALERTS];
      usingMockData = true;
    }
    lastFetchedAt = new Date();
    typeFilterPopulated = false;
    selected = new Set([...selected].filter(id => {
      const a = alerts.find(x => x.id === id);
      return a && !['resolved', 'false_positive'].includes(a.status);
    }));
    checkNewCriticals(prevCriticalIds);
    render();
    tickLastUpdated();
  }
  function startAutoRefresh() { stopAutoRefresh(); refreshTimer = setInterval(loadAlerts, REFRESH_MS); }
  function stopAutoRefresh() { if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null; } }

  // ── Wiring ──────────────────────────────────────────────────────
  function wire() {
    // Pane tabs
    document.querySelectorAll('.alerts-page-tab').forEach(t => {
      t.addEventListener('click', () => { state.pane = t.dataset.pane; switchPane(); });
    });

    document.querySelectorAll('.admin-kpi-card[data-filter]').forEach(card => {
      const handler = () => { state.kpi = card.dataset.filter || 'all'; render(); };
      card.addEventListener('click', handler);
      card.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handler(); } });
    });
    document.querySelectorAll('#alerts-quick-chips .admin-chip').forEach(chip => {
      chip.addEventListener('click', () => { state.chip = chip.dataset.chip; render(); });
    });
    document.querySelectorAll('.alerts-cat-tab').forEach(tab => {
      tab.addEventListener('click', () => { state.category = tab.dataset.cat; render(); });
    });

    const search = document.getElementById('alerts-search');
    if (search) {
      let t;
      search.addEventListener('input', () => {
        clearTimeout(t); t = setTimeout(() => { state.search = search.value; render(); }, 150);
      });
    }
    on('alerts-filter-severity', 'change', e => { state.severity = e.target.value; render(); });
    on('alerts-filter-type', 'change', e => { state.type = e.target.value; render(); });
    on('alerts-filter-status', 'change', e => { state.status = e.target.value; render(); });
    on('alerts-filter-assignee', 'change', e => { state.assignee = e.target.value; render(); });

    on('alerts-btn-refresh', 'click', () => loadAlerts());
    on('alerts-btn-export', 'click', exportCsv);
    on('alerts-btn-test', 'click', fireTestAlert);
    on('alerts-auto-refresh', 'change', e => { if (e.target.checked) startAutoRefresh(); else stopAutoRefresh(); });
    on('alerts-btn-notify', 'click', requestNotifyPermission);
    const vol = document.getElementById('alerts-volume');
    if (vol) {
      vol.value = localStorage.getItem('poool.alerts.volume') || '60';
      vol.addEventListener('input', () => { try { localStorage.setItem('poool.alerts.volume', vol.value); } catch { } });
    }
    on('alerts-btn-save-view', 'click', saveCurrentView);
    on('alerts-btn-cmdk', 'click', openPalette);

    on('alerts-select-all', 'change', e => {
      const checked = e.target.checked;
      const visible = applyFilters(alerts).filter(a => !['resolved', 'false_positive'].includes(a.status));
      if (checked) visible.forEach(a => selected.add(a.id)); else visible.forEach(a => selected.delete(a.id));
      render();
    });
    on('alerts-bulk-ack', 'click', () => bulkAction('acknowledge'));
    on('alerts-bulk-resolve', 'click', () => bulkAction('resolve'));
    on('alerts-bulk-claim', 'click', bulkClaim);
    on('alerts-bulk-snooze', 'change', e => {
      if (!e.target.value) return;
      bulkSnooze(parseInt(e.target.value));
      e.target.value = '';
    });
    on('alerts-bulk-clear', 'click', () => { selected.clear(); render(); });

    // Watchlist + Rules
    on('watchlist-btn-add', 'click', openWatchModal);
    on('watch-modal-close', 'click', closeWatchModal);
    on('watch-modal-cancel', 'click', closeWatchModal);
    on('watch-modal-save', 'click', saveWatchEntry);
    on('rules-btn-add', 'click', () => openRuleModal(-1));
    on('rule-modal-close', 'click', closeRuleModal);
    on('rule-modal-cancel', 'click', closeRuleModal);
    on('rule-modal-save', 'click', saveRule);
    on('rule-modal-backtest', 'click', async () => {
      const r = editingRuleIdx >= 0 ? rules[editingRuleIdx] : null;
      if (!r || !r.id) { mpToast && mpToast('Save rule first to backtest', 'info'); return; }
      try {
        const res = await jfetch(`${API_RULES}/${r.id}/backtest?days=30`, { method: 'POST' });
        mpToast && mpToast(`Backtest: ${res.matched_alerts} matches in ${res.days}d (${res.critical_matches} critical, avg ${res.avg_per_day.toFixed(2)}/day)`, 'info', 8000);
      } catch (e) {
        mpToast && mpToast('Backtest endpoint unavailable', 'error');
      }
    });
    on('rule-modal-test', 'click', () => {
      // fire a one-shot test using current form state
      const r = {
        name: document.getElementById('rule-name').value || 'Untitled',
        severity: document.getElementById('rule-severity').value,
        threshold: document.getElementById('rule-threshold').value,
        channel: document.getElementById('rule-channel').value,
      };
      const id = 'RULE-DRAFT-' + Date.now().toString(36);
      alerts.unshift({
        id, severity: r.severity, alert_type: r.name + ' (draft test)',
        message: `Draft rule test — threshold: ${r.threshold || 'n/a'}`,
        user_id: 'TEST', created_at: new Date().toISOString(), status: 'new',
        metadata: { source: 'rule-draft-test', channel: r.channel },
      });
      mpToast && mpToast('Test fired (draft rule)', 'success');
      closeRuleModal(); state.pane = 'alerts'; switchPane(); render();
    });

    // Cmd+K + global keys
    document.addEventListener('keydown', e => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault(); openPalette(); return;
      }
      if (e.key === 'Escape') {
        if (document.getElementById('palette').classList.contains('is-open')) closePalette();
        if (document.getElementById('rule-modal').classList.contains('is-open')) closeRuleModal();
        if (document.getElementById('watch-modal').classList.contains('is-open')) closeWatchModal();
      }
      if (document.getElementById('palette').classList.contains('is-open')) {
        const q = document.getElementById('palette-input').value;
        const filtered = PALETTE_CMDS.filter(c => c.label.toLowerCase().includes(q.toLowerCase()));
        if (e.key === 'ArrowDown') { e.preventDefault(); paletteIdx = Math.min(filtered.length - 1, paletteIdx + 1); renderPalette(q); }
        if (e.key === 'ArrowUp') { e.preventDefault(); paletteIdx = Math.max(0, paletteIdx - 1); renderPalette(q); }
        if (e.key === 'Enter' && filtered[paletteIdx]) { e.preventDefault(); runPaletteCmd(filtered[paletteIdx]); }
      }
    });
    on('palette-input', 'input', e => { paletteIdx = 0; renderPalette(e.target.value); });

    document.addEventListener('visibilitychange', () => {
      const cb = document.getElementById('alerts-auto-refresh');
      if (document.hidden) stopAutoRefresh();
      else if (cb && cb.checked) { startAutoRefresh(); loadAlerts(); }
    });

    if ('Notification' in window && Notification.permission === 'granted') {
      const btn = document.getElementById('alerts-btn-notify');
      if (btn) btn.style.display = 'none';
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    wire();
    loadAlerts();
    loadRulesAndWatchFromApi();
    loadViewsFromApi();
    loadHistory().then(render);
    startAutoRefresh();
    tickTimer = setInterval(tickLastUpdated, 5000);
    // Refresh history every 5 min
    setInterval(() => loadHistory().then(render), 5 * 60 * 1000);
  });
})();
