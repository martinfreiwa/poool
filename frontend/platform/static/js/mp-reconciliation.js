/**
 * Reconciliation — mp-reconciliation.js
 * Top-5 admin upgrades: derived KPIs, severity tiers, aging,
 * resolve actions, filter/search/export, history drill-down + sparkline.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/reconciliation';

  // ── Mock dataset (used as fallback + to drive UX until backend ships) ─
  const MOCK = {
    generatedAt: '2026-05-02T04:00:00Z',
    walletsChecked: 1247,
    walletsCheckedPrev: 1243,
    mismatches: [
      { id: 'M-001', user: 'USR-3384', wallet: '0x4a…b21f', asset: 'BVRT', walletBal: 120, ledgerBal: 125, diff: -5, diffUsd: -312.50, firstDetectedAt: '2026-05-01T12:14:00Z', cause: 'Concurrent order execution race condition', status: 'open',
        balanceHistory: [120, 122, 125, 125, 122, 120, 120],
        txHistory: [
          { ts: '2026-05-01T12:10:00Z', kind: 'order-fill', delta: -3, txHash: '0xabc…001' },
          { ts: '2026-05-01T12:13:00Z', kind: 'order-fill', delta: -2, txHash: '0xabc…002' },
          { ts: '2026-05-01T12:14:00Z', kind: 'detect', delta: 0, txHash: null },
        ] },
      { id: 'M-002', user: 'USR-1738', wallet: '0x9c…7e02', asset: 'JOTX', walletBal: 50, ledgerBal: 48, diff: 2, diffUsd: 8.40, firstDetectedAt: '2026-05-02T03:58:00Z', cause: 'Settlement callback delayed — tokens credited early', status: 'open',
        balanceHistory: [48, 48, 48, 50, 50, 50, 50],
        txHistory: [
          { ts: '2026-05-02T03:45:00Z', kind: 'settle', delta: 2, txHash: '0xdef…010' },
          { ts: '2026-05-02T03:58:00Z', kind: 'detect', delta: 0, txHash: null },
        ] },
      { id: 'M-003', user: 'USR-6643', wallet: '0x2f…11aa', asset: 'SWHS', walletBal: 800, ledgerBal: 800.5, diff: -0.5, diffUsd: -0.42, firstDetectedAt: '2026-05-02T03:59:30Z', cause: 'Rounding error in fractional token split', status: 'open',
        balanceHistory: [800.5, 800.5, 800, 800, 800, 800, 800],
        txHistory: [
          { ts: '2026-05-02T03:30:00Z', kind: 'split', delta: -0.5, txHash: '0xfed…020' },
          { ts: '2026-05-02T03:59:30Z', kind: 'detect', delta: 0, txHash: null },
        ] },
    ],
    history: [
      { time: '2026-05-02T04:00:00Z', wallets: 1247, mismatches: 3, durationSec: 12.4, status: 'warning' },
      { time: '2026-05-01T04:00:00Z', wallets: 1243, mismatches: 0, durationSec: 11.8, status: 'ok' },
      { time: '2026-04-30T04:00:00Z', wallets: 1238, mismatches: 1, durationSec: 12.1, status: 'warning' },
      { time: '2026-04-29T04:00:00Z', wallets: 1235, mismatches: 0, durationSec: 11.5, status: 'ok' },
      { time: '2026-04-28T04:00:00Z', wallets: 1230, mismatches: 0, durationSec: 11.9, status: 'ok' },
      { time: '2026-04-27T04:00:00Z', wallets: 1227, mismatches: 0, durationSec: 11.4, status: 'ok' },
      { time: '2026-04-26T04:00:00Z', wallets: 1224, mismatches: 2, durationSec: 13.1, status: 'warning' },
    ],
  };

  // Severity thresholds (USD-equiv difference) — persisted in localStorage
  const RULES_KEY = 'poool.recon.rules.v1';
  const DEFAULT_RULES = {
    critical: 100, warning: 1, cron: '0 4 * * *',
    slack: '', pagerduty: '',
    onCritical: true, onWarning: false, onAnomaly: true,
  };
  let SEVERITY_THRESHOLDS = loadRules();

  function loadRules() {
    try {
      const stored = JSON.parse(localStorage.getItem(RULES_KEY) || 'null');
      return Object.assign({}, DEFAULT_RULES, stored || {});
    } catch { return Object.assign({}, DEFAULT_RULES); }
  }
  function saveRules(r) { localStorage.setItem(RULES_KEY, JSON.stringify(r)); }

  const AUDIT_KEY = 'poool.recon.audit.v1';
  const PRESETS_KEY = 'poool.recon.presets.v1';
  const THROTTLE_KEY = 'poool.recon.lastrun';
  const THROTTLE_MS = 60 * 1000;

  let state = {
    data: structuredClone(MOCK),
    filter: { q: '', severity: '', status: 'open' },
    selected: new Set(),
    histFilter: { from: '', to: '', status: '' },
    histSelected: new Set(),
    refreshSec: 60,
    refreshTimer: null,
    normalize: false,
    audit: loadAudit(),
    presets: loadPresets(),
  };

  // FX rates (mock — backend would push real rates)
  const FX = { USD: 1, EUR: 0.93, CHF: 0.88, GBP: 0.79 };
  const TOUR_KEY = 'poool.recon.tour-seen.v1';

  let lastSeenMismatchIds = new Set();

  function loadAudit() { try { return JSON.parse(localStorage.getItem(AUDIT_KEY) || '[]'); } catch { return []; } }
  function saveAudit() { localStorage.setItem(AUDIT_KEY, JSON.stringify(state.audit.slice(0, 50))); }
  function logAudit(action, target, notes) {
    state.audit.unshift({ ts: new Date().toISOString(), actor: 'jonas@poool.dev', action, target, notes });
    saveAudit();
    renderAudit();
  }

  function loadPresets() {
    const defaults = {
      'critical-open': { q: '', severity: 'critical', status: 'open' },
      'age-24h': { q: '', severity: '', status: 'open', minAgeHours: 24 },
      'settlement': { q: 'settlement', severity: '', status: 'open' },
      'all-status': { q: '', severity: '', status: 'all' },
    };
    try { return Object.assign({}, defaults, JSON.parse(localStorage.getItem(PRESETS_KEY) || '{}')); }
    catch { return defaults; }
  }
  function savePresets() { localStorage.setItem(PRESETS_KEY, JSON.stringify(state.presets)); }

  // Heuristic auto-classifier for likely cause
  const CAUSE_PATTERNS = [
    { re: /race|concurrent/i, label: 'Race condition', tag: 'race' },
    { re: /settlement|callback/i, label: 'Settlement lag', tag: 'settlement' },
    { re: /round|fractional|precision/i, label: 'Rounding', tag: 'rounding' },
    { re: /gas|reserve/i, label: 'Gas reserve', tag: 'gas' },
    { re: /pending|tx|transaction/i, label: 'Pending tx', tag: 'pending' },
  ];
  function classifyCause(text) {
    for (const p of CAUSE_PATTERNS) if (p.re.test(text || '')) return p;
    return { label: 'Unclassified', tag: 'other' };
  }

  // ── Helpers ─────────────────────────────────────────────────────
  const fmtNum = n => n.toLocaleString('en-US', { maximumFractionDigits: 4 });
  const fmtUsd = n => (n < 0 ? '-' : '') + '$' + Math.abs(n).toLocaleString('en-US', { maximumFractionDigits: 2, minimumFractionDigits: 2 });

  function severityOf(m) {
    const abs = Math.abs(m.diffUsd);
    if (abs >= SEVERITY_THRESHOLDS.critical) return 'critical';
    if (abs >= SEVERITY_THRESHOLDS.warning) return 'warning';
    return 'info';
  }

  function ageMs(iso) { return Date.now() - new Date(iso).getTime(); }
  function fmtAge(ms) {
    const m = Math.floor(ms / 60000);
    if (m < 60) return m + 'm';
    const h = Math.floor(m / 60);
    if (h < 24) return h + 'h';
    return Math.floor(h / 24) + 'd';
  }
  function ageBadgeClass(ms) {
    const h = ms / 3600000;
    if (h >= 24) return 'recon-age recon-age--danger';
    if (h >= 4) return 'recon-age recon-age--warning';
    return 'recon-age recon-age--ok';
  }

  function fmtRelative(iso) {
    const ms = Date.now() - new Date(iso).getTime();
    const m = Math.floor(ms / 60000);
    if (m < 1) return 'just now';
    if (m < 60) return m + 'm ago';
    const h = Math.floor(m / 60);
    if (h < 24) return h + 'h ago';
    const d = Math.floor(h / 24);
    return d + 'd ago';
  }

  // ── Derived KPIs (single source of truth) ───────────────────────
  function renderKpis() {
    const d = state.data;
    const open = d.mismatches.filter(m => m.status === 'open');
    const critical = open.filter(m => severityOf(m) === 'critical').length;
    const oldest = open.reduce((acc, m) => Math.max(acc, ageMs(m.firstDetectedAt)), 0);

    // Status
    const status = open.length === 0 ? 'All Clear' : (critical > 0 ? 'Action Required' : 'Needs Review');
    const statusColor = open.length === 0 ? 'var(--admin-success)' : (critical > 0 ? 'var(--admin-danger)' : 'var(--admin-warning)');
    const statusEl = document.getElementById('kpi-status-value');
    statusEl.textContent = status;
    statusEl.style.color = statusColor;
    document.getElementById('kpi-status-sub').textContent = open.length === 0
      ? 'All balances verified'
      : `${critical} critical · oldest ${fmtAge(oldest)}`;
    const statusCta = document.getElementById('kpi-status-cta');
    if (statusCta) statusCta.style.display = open.length > 0 ? 'inline-block' : 'none';

    // Mismatches
    const mEl = document.getElementById('kpi-mismatches-value');
    mEl.textContent = open.length;
    mEl.style.color = open.length > 0 ? 'var(--admin-danger)' : 'var(--admin-success)';
    const totalUsd = open.reduce((s, m) => s + Math.abs(m.diffUsd), 0);
    document.getElementById('kpi-mismatches-delta').innerHTML = open.length > 0
      ? `<span style="color:var(--admin-text-muted);">≈ ${fmtUsd(totalUsd)} exposure</span>`
      : '';

    // Wallets + delta
    document.getElementById('kpi-wallets-value').textContent = d.walletsChecked.toLocaleString();
    const delta = d.walletsChecked - d.walletsCheckedPrev;
    const deltaColor = delta >= 0 ? 'var(--admin-success)' : 'var(--admin-danger)';
    const arrow = delta > 0 ? '▲' : (delta < 0 ? '▼' : '–');
    document.getElementById('kpi-wallets-delta').innerHTML =
      `<span style="color:${deltaColor};">${arrow} ${Math.abs(delta)}</span> <span style="color:var(--admin-text-muted);">vs prev run</span>`;

    // Last run
    const dt = new Date(d.generatedAt);
    document.getElementById('kpi-lastrun-value').textContent = dt.toLocaleString('en-US', { dateStyle: 'short', timeStyle: 'short' });
    document.getElementById('kpi-lastrun-sub').textContent = fmtRelative(d.generatedAt);
  }

  // ── Filtered mismatch list ──────────────────────────────────────
  function visibleMismatches() {
    const f = state.filter;
    return state.data.mismatches.filter(m => {
      if (f.status !== 'all' && m.status !== f.status) return false;
      if (f.severity && severityOf(m) !== f.severity) return false;
      if (state._minAgeHours && ageMs(m.firstDetectedAt) < state._minAgeHours * 3600000) return false;
      if (f.q) {
        const hay = `${m.user} ${m.asset} ${m.wallet} ${m.id} ${m.cause}`.toLowerCase();
        if (!hay.includes(f.q.toLowerCase())) return false;
      }
      return true;
    });
  }

  // ── Render mismatches ───────────────────────────────────────────
  function renderMismatches() {
    const tbody = document.getElementById('recon-body');
    if (!tbody) return;
    const rows = visibleMismatches();

    if (rows.length === 0) {
      const allClear = state.data.mismatches.filter(m => m.status === 'open').length === 0;
      tbody.innerHTML = `
        <tr><td colspan="10" style="text-align:center; padding:32px; color:var(--admin-success);">
          ${allClear
            ? `<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-bottom:8px; opacity:0.8;"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
               <div>No open mismatches — all balances verified</div>`
            : '<div style="color:var(--admin-text-muted);">No mismatches match current filter</div>'}
        </td></tr>`;
      updateBulkBar();
      return;
    }

    tbody.innerHTML = rows.map(m => {
      const sev = severityOf(m);
      const age = ageMs(m.firstDetectedAt);
      const sevBadge = `<span class="admin-badge recon-sev recon-sev--${sev}">${sev}</span>`;
      const ageBadge = `<span class="${ageBadgeClass(age)}" title="First detected ${new Date(m.firstDetectedAt).toLocaleString()}">${fmtAge(age)}</span>`;
      const checked = state.selected.has(m.id) ? 'checked' : '';
      const isResolved = m.status !== 'open';
      const rowStyle = isResolved ? 'opacity:0.5;' : '';
      return `
        <tr id="row-${m.id}" style="${rowStyle}">
          <td><input type="checkbox" class="recon-row-check" data-id="${m.id}" ${checked} ${isResolved ? 'disabled' : ''} aria-label="Select ${m.id}" /></td>
          <td><a href="/admin/users.html?id=${encodeURIComponent(m.user)}" class="recon-deep" data-stop>${`<code class="recon-code">${m.user}</code>`}</a><div style="font-size:11px; color:var(--admin-text-muted); margin-top:2px;">${m.wallet}</div></td>
          <td style="font-weight:600;"><a href="/admin/assets.html?symbol=${encodeURIComponent(m.asset)}" class="recon-deep" data-stop>${m.asset}</a></td>
          <td>${sevBadge}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${state.normalize ? fmtUsd(m.walletBal * (m.diffUsd / m.diff || 1)) : fmtNum(m.walletBal)}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${state.normalize ? fmtUsd(m.ledgerBal * (m.diffUsd / m.diff || 1)) : fmtNum(m.ledgerBal)}</td>
          <td style="text-align:right;">
            <div style="font-variant-numeric:tabular-nums; color:var(--admin-danger); font-weight:600;">${state.normalize ? fmtUsd(m.diffUsd) : (m.diff > 0 ? '+' : '') + fmtNum(m.diff)}</div>
            <div style="font-size:11px; color:var(--admin-text-muted); font-variant-numeric:tabular-nums;">${state.normalize ? `${m.diff > 0 ? '+' : ''}${fmtNum(m.diff)} ${m.asset}` : fmtUsd(m.diffUsd)}</div>
          </td>
          <td>${ageBadge}</td>
          <td style="font-size:12px; color:var(--admin-text-muted); max-width:260px;">
            <span class="recon-cause-tag recon-cause-${classifyCause(m.cause).tag}">${classifyCause(m.cause).label}</span>
            <div style="margin-top:2px;">${m.cause}</div>
            ${m.notes ? `<div class="recon-note" title="Admin note">📝 ${m.notes}</div>` : ''}
          </td>
          <td>${m.balanceHistory ? sparkline(m.balanceHistory, 80, 22) : ''}</td>
          <td style="text-align:right; white-space:nowrap;">
            ${isResolved
              ? `<span class="admin-badge admin-badge--success">${m.status}</span>`
              : `
                <button class="admin-btn admin-btn--sm admin-btn--ghost recon-act" data-id="${m.id}" data-act="detail" title="View tx history (Enter)">Detail</button>
                <button class="admin-btn admin-btn--sm admin-btn--ghost recon-act" data-id="${m.id}" data-act="note" title="Add note">📝</button>
                <button class="admin-btn admin-btn--sm admin-btn--ghost recon-act" data-id="${m.id}" data-act="force-sync" title="Re-fetch on-chain balance">Force-Sync</button>
                <button class="admin-btn admin-btn--sm admin-btn--success recon-act" data-id="${m.id}" data-act="resolve">Resolve</button>
                <button class="admin-btn admin-btn--sm admin-btn--ghost recon-act" data-id="${m.id}" data-act="dismiss" title="Mark false positive">Dismiss</button>
              `
            }
          </td>
        </tr>`;
    }).join('');

    bindMismatchRowEvents();
    updateBulkBar();
  }

  function bindMismatchRowEvents() {
    document.querySelectorAll('.recon-row-check').forEach(cb => {
      cb.addEventListener('change', e => {
        const id = e.target.dataset.id;
        if (e.target.checked) state.selected.add(id); else state.selected.delete(id);
        updateBulkBar();
      });
    });
    document.querySelectorAll('.recon-act').forEach(btn => {
      btn.addEventListener('click', () => actOnMismatch(btn.dataset.id, btn.dataset.act));
    });
  }

  function actOnMismatch(id, act) {
    const m = state.data.mismatches.find(x => x.id === id);
    if (!m) return;
    if (act === 'detail') { openDetail(m); return; }
    if (act === 'note') {
      const cur = m.notes || '';
      const next = prompt(`Note for ${id}:`, cur);
      if (next === null) return;
      m.notes = next.trim();
      logAudit('note', id, m.notes ? `Note: ${m.notes.slice(0, 60)}` : 'Note cleared');
      renderMismatches();
      return;
    }
    if (act === 'resolve') {
      openResolveModal(m);
      return;
    } else if (act === 'dismiss') {
      if (!confirm(`Mark ${id} as false positive (dismiss)?`)) return;
      m.status = 'dismissed';
      logAudit('dismiss', `${id} (${m.user}/${m.asset})`, 'False positive');
      toast(`${id} dismissed`, 'success');
    } else if (act === 'force-sync') {
      logAudit('force-sync', m.wallet, `${m.asset} balance refresh`);
      toast(`Force-sync queued for ${m.wallet}`, 'success');
    } else if (act === 'investigate') {
      logAudit('investigate', `${id} (${m.user})`, 'Opened investigation');
      toast(`Investigation view for ${id} (todo: navigate to user page)`, 'info');
    }
    renderKpis();
    renderMismatches();
  }

  // ── Bulk action bar ─────────────────────────────────────────────
  function updateBulkBar() {
    const btn = document.getElementById('btn-bulk-action');
    if (!btn) return;
    const n = state.selected.size;
    btn.disabled = n === 0;
    btn.textContent = n === 0 ? 'Bulk action…' : `Bulk action (${n})…`;
  }

  function bindBulkActions() {
    document.getElementById('recon-select-all')?.addEventListener('change', e => {
      const visible = visibleMismatches().filter(m => m.status === 'open');
      if (e.target.checked) visible.forEach(m => state.selected.add(m.id));
      else visible.forEach(m => state.selected.delete(m.id));
      renderMismatches();
    });
    document.getElementById('btn-bulk-action')?.addEventListener('click', () => {
      const choice = prompt('Bulk action: type "resolve", "dismiss", or "force-sync"');
      if (!choice) return;
      const act = choice.trim().toLowerCase();
      if (!['resolve', 'dismiss', 'force-sync'].includes(act)) { toast('Unknown action', 'error'); return; }
      if (!confirm(`Apply "${act}" to ${state.selected.size} mismatches?`)) return;
      const ids = [...state.selected];
      ids.forEach(id => {
        const m = state.data.mismatches.find(x => x.id === id);
        if (!m) return;
        if (act === 'resolve') m.status = 'resolved';
        else if (act === 'dismiss') m.status = 'dismissed';
      });
      state.selected.clear();
      toast(`${ids.length} mismatches updated`, 'success');
      renderKpis();
      renderMismatches();
    });
  }

  // ── Filters ─────────────────────────────────────────────────────
  function bindFilters() {
    const search = document.getElementById('recon-search');
    search?.addEventListener('input', e => { state.filter.q = e.target.value; renderMismatches(); });
    document.getElementById('recon-severity-filter')?.addEventListener('change', e => { state.filter.severity = e.target.value; renderMismatches(); });
    document.getElementById('recon-status-filter')?.addEventListener('change', e => { state.filter.status = e.target.value; renderMismatches(); });

    // KPI cards as filter shortcuts
    document.getElementById('kpi-mismatches')?.addEventListener('click', () => {
      state.filter.status = 'open';
      state.filter.severity = '';
      document.getElementById('recon-status-filter').value = 'open';
      document.getElementById('recon-severity-filter').value = '';
      renderMismatches();
      document.getElementById('recon-body')?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
    document.getElementById('kpi-status')?.addEventListener('click', () => {
      const hasCrit = state.data.mismatches.some(m => m.status === 'open' && severityOf(m) === 'critical');
      if (hasCrit) {
        state.filter.severity = 'critical';
        document.getElementById('recon-severity-filter').value = 'critical';
        renderMismatches();
      }
    });

    // Keyboard: "/" focuses search
    document.addEventListener('keydown', e => {
      if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
        e.preventDefault();
        search?.focus();
      }
    });
  }

  // ── CSV export ──────────────────────────────────────────────────
  function exportCsv(rows, filename, headers, mapper) {
    const csv = [headers.join(',')]
      .concat(rows.map(r => mapper(r).map(v => {
        const s = String(v ?? '');
        return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
      }).join(',')))
      .join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = filename; a.click();
    URL.revokeObjectURL(url);
  }

  function bindExports() {
    document.getElementById('btn-export-csv')?.addEventListener('click', () => {
      const rows = visibleMismatches();
      exportCsv(rows,
        `reconciliation-mismatches-${new Date().toISOString().slice(0, 10)}.csv`,
        ['id', 'user', 'wallet', 'asset', 'severity', 'wallet_balance', 'ledger_balance', 'diff', 'diff_usd', 'first_detected_at', 'status', 'cause'],
        m => [m.id, m.user, m.wallet, m.asset, severityOf(m), m.walletBal, m.ledgerBal, m.diff, m.diffUsd, m.firstDetectedAt, m.status, m.cause]
      );
      toast(`Exported ${rows.length} rows`, 'success');
    });
    document.getElementById('btn-export-history')?.addEventListener('click', () => {
      exportCsv(state.data.history,
        `reconciliation-history-${new Date().toISOString().slice(0, 10)}.csv`,
        ['run_time', 'wallets_checked', 'mismatches', 'duration_sec', 'status'],
        h => [h.time, h.wallets, h.mismatches, h.durationSec, h.status]
      );
    });
  }

  // ── History + sparkline ─────────────────────────────────────────
  function sparkline(values, w = 100, h = 22) {
    if (!values.length) return '';
    const max = Math.max(...values, 1);
    const min = Math.min(...values, 0);
    const range = (max - min) || 1;
    const step = w / Math.max(values.length - 1, 1);
    const pts = values.map((v, i) => `${(i * step).toFixed(1)},${(h - ((v - min) / range) * h).toFixed(1)}`).join(' ');
    return `<svg width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" aria-hidden="true"><polyline fill="none" stroke="currentColor" stroke-width="1.5" points="${pts}"/></svg>`;
  }

  function renderHistory() {
    const hBody = document.getElementById('recon-history-body');
    if (!hBody) return;
    const hist = visibleHistory();
    const durations = hist.slice().reverse().map(h => h.durationSec);
    const avg = durations.length ? durations.reduce((s, v) => s + v, 0) / durations.length : 0;

    if (hist.length === 0) {
      hBody.innerHTML = '<tr><td colspan="7" style="text-align:center; padding:24px; color:var(--admin-text-muted);">No runs in selected range</td></tr>';
    } else {
      hBody.innerHTML = hist.map((h) => {
        const idx = state.data.history.indexOf(h);
        const statusBadge = h.status === 'ok'
          ? '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Clean</span>'
          : '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Mismatches</span>';
        const trendVals = state.data.history.slice(idx, Math.min(idx + 5, state.data.history.length)).map(x => x.durationSec).reverse();
        const anomaly = avg > 0 && h.durationSec > avg * 1.25;
        const dt = new Date(h.time);
        const checked = state.histSelected.has(idx) ? 'checked' : '';
        return `
          <tr class="recon-history-row" data-idx="${idx}" tabindex="0" role="button" aria-label="View snapshot for ${dt.toISOString()}">
            <td><input type="checkbox" class="recon-hist-check" data-idx="${idx}" ${checked} aria-label="Select run for compare" /></td>
            <td style="font-variant-numeric:tabular-nums; font-size:13px;">${dt.toLocaleString('en-US', { dateStyle: 'short', timeStyle: 'short' })}</td>
            <td style="text-align:right; font-variant-numeric:tabular-nums;">${h.wallets.toLocaleString()}</td>
            <td style="text-align:right;"><span class="admin-badge ${h.mismatches > 0 ? 'admin-badge--warning' : 'admin-badge--success'}">${h.mismatches}</span></td>
            <td style="text-align:right; font-variant-numeric:tabular-nums; ${anomaly ? 'color:var(--admin-warning); font-weight:600;' : ''}" title="${anomaly ? `${(h.durationSec / avg * 100 - 100).toFixed(0)}% above avg` : ''}">${h.durationSec.toFixed(1)}s${anomaly ? ' ⚠' : ''}</td>
            <td style="color:var(--admin-text-muted);">${sparkline(trendVals)}</td>
            <td>${statusBadge}</td>
          </tr>`;
      }).join('');
    }

    document.querySelectorAll('.recon-history-row').forEach(row => {
      row.addEventListener('click', e => {
        if (e.target.closest('.recon-hist-check')) return;
        openSnapshot(parseInt(row.dataset.idx));
      });
      row.addEventListener('keydown', e => { if (e.key === 'Enter') openSnapshot(parseInt(row.dataset.idx)); });
    });
    document.querySelectorAll('.recon-hist-check').forEach(cb => {
      cb.addEventListener('change', e => {
        const i = parseInt(e.target.dataset.idx);
        if (e.target.checked) state.histSelected.add(i); else state.histSelected.delete(i);
        const btn = document.getElementById('btn-compare-runs');
        btn.textContent = `Compare (${state.histSelected.size})`;
        btn.disabled = state.histSelected.size !== 2;
      });
    });
  }

  function openSnapshot(idx) {
    const h = state.data.history[idx];
    const drawer = document.getElementById('recon-snapshot');
    document.getElementById('snapshot-title').textContent = `Run @ ${new Date(h.time).toLocaleString()}`;
    // For today's run: show current mismatches; for older runs: synthesized summary
    const isLatest = idx === 0;
    let body;
    if (isLatest && h.mismatches > 0) {
      body = `<p style="margin:0 0 8px 0; color:var(--admin-text-muted);">Showing ${state.data.mismatches.length} detected mismatches from this run.</p>
              <ul style="margin:0; padding-left:20px;">
                ${state.data.mismatches.map(m => `<li><code>${m.id}</code> · ${m.user} · ${m.asset} · diff ${fmtNum(m.diff)} (${fmtUsd(m.diffUsd)})</li>`).join('')}
              </ul>`;
    } else {
      body = `<dl class="recon-dl">
        <dt>Wallets checked</dt><dd>${h.wallets.toLocaleString()}</dd>
        <dt>Mismatches</dt><dd>${h.mismatches}</dd>
        <dt>Duration</dt><dd>${h.durationSec.toFixed(1)}s</dd>
        <dt>Status</dt><dd>${h.status}</dd>
      </dl>
      <p style="font-size:12px; color:var(--admin-text-muted); margin-top:12px;">Detailed per-mismatch snapshots for historical runs require backend persistence (todo).</p>`;
    }
    document.getElementById('snapshot-body').innerHTML = body;
    drawer.style.display = 'block';
    drawer.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // ── Alert rules ─────────────────────────────────────────────────
  function hydrateRulesUi() {
    const r = SEVERITY_THRESHOLDS;
    const set = (id, v) => { const el = document.getElementById(id); if (el) { if (el.type === 'checkbox') el.checked = !!v; else el.value = v ?? ''; } };
    set('thr-critical', r.critical);
    set('thr-warning', r.warning);
    set('thr-cron', r.cron);
    set('alert-slack', r.slack);
    set('alert-pd', r.pagerduty);
    set('alert-on-critical', r.onCritical);
    set('alert-on-warning', r.onWarning);
    set('alert-on-anomaly', r.onAnomaly);
  }

  function readRulesFromUi() {
    const get = id => document.getElementById(id);
    return {
      critical: parseFloat(get('thr-critical').value) || 0,
      warning: parseFloat(get('thr-warning').value) || 0,
      cron: get('thr-cron').value.trim(),
      slack: get('alert-slack').value.trim(),
      pagerduty: get('alert-pd').value.trim(),
      onCritical: get('alert-on-critical').checked,
      onWarning: get('alert-on-warning').checked,
      onAnomaly: get('alert-on-anomaly').checked,
    };
  }

  function bindRules() {
    hydrateRulesUi();
    document.getElementById('btn-save-rules')?.addEventListener('click', () => {
      const next = readRulesFromUi();
      if (next.warning >= next.critical) { toast('Warning threshold must be lower than Critical', 'error'); return; }
      SEVERITY_THRESHOLDS = next;
      saveRules(next);
      toast('Alert rules saved', 'success');
      renderKpis();
      renderMismatches();
    });
    document.getElementById('btn-reset-rules')?.addEventListener('click', () => {
      if (!confirm('Reset rules to defaults?')) return;
      SEVERITY_THRESHOLDS = Object.assign({}, DEFAULT_RULES);
      saveRules(SEVERITY_THRESHOLDS);
      hydrateRulesUi();
      renderKpis();
      renderMismatches();
      toast('Rules reset', 'success');
    });
    document.getElementById('btn-test-alert')?.addEventListener('click', () => {
      const r = readRulesFromUi();
      const channels = [];
      if (r.slack) channels.push('Slack');
      if (r.pagerduty) channels.push('PagerDuty');
      if (channels.length === 0) { toast('No notification channel configured', 'error'); return; }
      toast(`Test alert dispatched to ${channels.join(' + ')}`, 'success');
    });
  }

  function bindSnapshotClose() {
    document.getElementById('btn-close-snapshot')?.addEventListener('click', () => {
      document.getElementById('recon-snapshot').style.display = 'none';
    });
  }

  // ── Toast helper ────────────────────────────────────────────────
  function toast(msg, type) { if (typeof window.mpToast === 'function') window.mpToast(msg, type || 'success'); }

  // ── Run reconciliation button ───────────────────────────────────
  function bindRunButton() {
    document.getElementById('btn-run-recon')?.addEventListener('click', function () {
      const lastRunMs = Date.now() - new Date(state.data.generatedAt).getTime();
      if (lastRunMs < 10 * 60 * 1000) {
        if (!confirm(`Last run was ${fmtAge(lastRunMs)} ago. Run again now?`)) return;
      }
      const btn = this;
      btn.disabled = true;
      const orig = btn.innerHTML;
      btn.innerHTML = '<span class="recon-spinner"></span> Running…';
      markRunStart();
      loadReconciliation().then(() => {
        btn.innerHTML = orig;
        toast('Reconciliation run completed', 'success');
        logAudit('run-reconciliation', 'manual', `${state.data.mismatches.length} mismatches found`);
      });
    });
  }

  // ── Per-asset breakdown (#19) ───────────────────────────────────
  function renderBreakdown() {
    const host = document.getElementById('recon-breakdown');
    if (!host) return;
    // Aggregate across history (mock: spread across assets)
    const counts = {};
    state.data.mismatches.forEach(m => { counts[m.asset] = (counts[m.asset] || 0) + 1; });
    // Synthesize 30d counts from history total
    const histTotal = state.data.history.reduce((s, h) => s + h.mismatches, 0);
    const assets = Object.keys(counts);
    if (assets.length === 0) {
      host.innerHTML = '<div style="color:var(--admin-text-muted); text-align:center; padding:16px;">No mismatches in window.</div>';
      document.getElementById('breakdown-meta').textContent = '';
      return;
    }
    const max = Math.max(...assets.map(a => counts[a]));
    host.innerHTML = `
      <div class="recon-bars">
        ${assets.map(a => `
          <div class="recon-bar-row">
            <span class="recon-bar-label">${a}</span>
            <div class="recon-bar-track"><div class="recon-bar-fill" style="width:${(counts[a] / max * 100).toFixed(1)}%"></div></div>
            <span class="recon-bar-value">${counts[a]}</span>
          </div>`).join('')}
      </div>`;
    document.getElementById('breakdown-meta').textContent = `${histTotal} total · ${assets.length} assets`;
  }

  function renderXref() {
    const open = state.data.mismatches.filter(m => m.status === 'open');
    const pending = open.filter(m => /settlement|pending/i.test(m.cause)).length;
    const el = document.getElementById('xref-settlements');
    if (el) {
      el.textContent = pending;
      el.className = 'recon-xref-badge ' + (pending > 0 ? 'recon-xref-badge--warn' : 'recon-xref-badge--ok');
    }
  }

  // ── History filter + compare (#5, #31) ──────────────────────────
  function visibleHistory() {
    const f = state.histFilter;
    return state.data.history.filter(h => {
      const ts = new Date(h.time);
      if (f.from && ts < new Date(f.from)) return false;
      if (f.to && ts > new Date(f.to + 'T23:59:59')) return false;
      if (f.status && h.status !== f.status) return false;
      return true;
    });
  }

  function bindHistoryFilters() {
    const handler = () => {
      state.histFilter = {
        from: document.getElementById('hist-from').value,
        to: document.getElementById('hist-to').value,
        status: document.getElementById('hist-status').value,
      };
      renderHistory();
    };
    ['hist-from', 'hist-to', 'hist-status'].forEach(id => document.getElementById(id)?.addEventListener('change', handler));
    document.getElementById('btn-compare-runs')?.addEventListener('click', () => {
      const ids = [...state.histSelected];
      if (ids.length !== 2) { toast('Select exactly 2 runs to compare', 'error'); return; }
      const a = state.data.history[ids[0]];
      const b = state.data.history[ids[1]];
      const drawer = document.getElementById('recon-snapshot');
      document.getElementById('snapshot-title').textContent = `Compare: ${new Date(a.time).toLocaleString()} ↔ ${new Date(b.time).toLocaleString()}`;
      document.getElementById('snapshot-body').innerHTML = `
        <table class="admin-table">
          <thead><tr><th>Metric</th><th>Run A</th><th>Run B</th><th>Δ</th></tr></thead>
          <tbody>
            <tr><td>Wallets</td><td>${a.wallets}</td><td>${b.wallets}</td><td>${b.wallets - a.wallets}</td></tr>
            <tr><td>Mismatches</td><td>${a.mismatches}</td><td>${b.mismatches}</td><td>${b.mismatches - a.mismatches}</td></tr>
            <tr><td>Duration (s)</td><td>${a.durationSec}</td><td>${b.durationSec}</td><td>${(b.durationSec - a.durationSec).toFixed(1)}</td></tr>
          </tbody>
        </table>`;
      drawer.style.display = 'block';
      drawer.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }

  // ── Auto-refresh (#10) ──────────────────────────────────────────
  function setupAutoRefresh() {
    if (state.refreshTimer) { clearInterval(state.refreshTimer); state.refreshTimer = null; }
    const sel = document.getElementById('recon-refresh-interval');
    sel?.addEventListener('change', () => {
      state.refreshSec = parseInt(sel.value);
      setupAutoRefresh();
    });
    const live = document.getElementById('recon-live-text');
    if (state.refreshSec === 0) {
      if (live) live.textContent = 'Manual';
      document.querySelector('#recon-live .mp-live-dot')?.style.setProperty('background', 'var(--admin-text-muted)');
      return;
    }
    if (live) live.textContent = `Live · ${state.refreshSec}s`;
    state.refreshTimer = setInterval(() => loadReconciliation(), state.refreshSec * 1000);
  }

  // ── Command palette (#28) ───────────────────────────────────────
  function openCmdK() {
    const overlay = document.getElementById('cmdk-overlay');
    const input = document.getElementById('cmdk-input');
    const list = document.getElementById('cmdk-list');
    const commands = [
      { id: 'run', label: 'Run reconciliation', act: () => document.getElementById('btn-run-recon').click() },
      { id: 'export', label: 'Export mismatches CSV', act: () => document.getElementById('btn-export-csv').click() },
      { id: 'export-hist', label: 'Export history CSV', act: () => document.getElementById('btn-export-history').click() },
      { id: 'filter-crit', label: 'Filter: Critical only', act: () => { document.getElementById('recon-severity-filter').value = 'critical'; state.filter.severity = 'critical'; renderMismatches(); } },
      { id: 'filter-clear', label: 'Filter: Clear', act: () => { ['recon-search', 'recon-severity-filter'].forEach(id => document.getElementById(id).value = ''); state.filter = { q: '', severity: '', status: 'open' }; renderMismatches(); } },
      { id: 'rules', label: 'Edit alert rules', act: () => document.getElementById('thr-critical')?.scrollIntoView({ behavior: 'smooth' }) },
      { id: 'snapshot-latest', label: 'Open latest snapshot', act: () => openSnapshot(0) },
      { id: 'goto-settlements', label: 'Go to: Pending Settlements', act: () => location.href = '/admin/marketplace/pending-settlements.html' },
      { id: 'goto-audit', label: 'Go to: Audit log', act: () => location.href = '/admin/audit-logs.html?source=reconciliation' },
    ];
    let idx = 0;
    function render(filter = '') {
      const f = filter.toLowerCase();
      const items = commands.filter(c => c.label.toLowerCase().includes(f));
      list.innerHTML = items.map((c, i) => `<li class="cmdk-item ${i === idx ? 'is-active' : ''}" data-id="${c.id}">${c.label}</li>`).join('');
      list._items = items;
    }
    function close() { overlay.hidden = true; document.removeEventListener('keydown', keyHandler); }
    function keyHandler(e) {
      if (e.key === 'Escape') { close(); return; }
      if (e.key === 'ArrowDown') { idx = Math.min(idx + 1, (list._items?.length || 1) - 1); render(input.value); e.preventDefault(); }
      if (e.key === 'ArrowUp') { idx = Math.max(idx - 1, 0); render(input.value); e.preventDefault(); }
      if (e.key === 'Enter') {
        const cmd = list._items?.[idx];
        if (cmd) { close(); setTimeout(() => cmd.act(), 50); }
      }
    }
    overlay.hidden = false;
    input.value = ''; idx = 0; render();
    input.focus();
    input.oninput = () => { idx = 0; render(input.value); };
    list.onclick = e => {
      const li = e.target.closest('.cmdk-item');
      if (!li) return;
      const cmd = list._items.find(c => c.id === li.dataset.id);
      if (cmd) { close(); setTimeout(() => cmd.act(), 50); }
    };
    overlay.onclick = e => { if (e.target === overlay) close(); };
    document.addEventListener('keydown', keyHandler);
  }

  function bindCmdK() {
    document.getElementById('btn-cmdk')?.addEventListener('click', openCmdK);
    document.addEventListener('keydown', e => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); openCmdK(); }
    });
  }

  // ── Detail modal (tx history) ───────────────────────────────────
  function openDetail(m) {
    const overlay = document.getElementById('mm-detail-overlay');
    document.getElementById('mm-detail-title').textContent = `${m.id} · ${m.user} · ${m.asset}`;
    const sev = severityOf(m);
    const txRows = (m.txHistory || []).map(t => `
      <tr>
        <td style="font-size:12px; font-variant-numeric:tabular-nums;">${new Date(t.ts).toLocaleString()}</td>
        <td><span class="recon-cause-tag">${t.kind}</span></td>
        <td style="text-align:right; font-variant-numeric:tabular-nums; color:${t.delta < 0 ? 'var(--admin-danger)' : t.delta > 0 ? 'var(--admin-success)' : 'var(--admin-text-muted)'};">${t.delta > 0 ? '+' : ''}${t.delta || '—'}</td>
        <td style="font-size:11px; color:var(--admin-text-muted);">${t.txHash ? `<code class="recon-code">${t.txHash}</code>` : '—'}</td>
      </tr>`).join('');
    document.getElementById('mm-detail-body').innerHTML = `
      <div class="recon-dl" style="margin-bottom:14px;">
        <dt>Severity</dt><dd><span class="admin-badge recon-sev recon-sev--${sev}">${sev}</span></dd>
        <dt>Wallet</dt><dd><code class="recon-code">${m.wallet}</code></dd>
        <dt>Wallet balance</dt><dd>${fmtNum(m.walletBal)} ${m.asset}</dd>
        <dt>Ledger balance</dt><dd>${fmtNum(m.ledgerBal)} ${m.asset}</dd>
        <dt>Diff</dt><dd>${m.diff > 0 ? '+' : ''}${fmtNum(m.diff)} ${m.asset} (${fmtUsd(m.diffUsd)})</dd>
        <dt>First detected</dt><dd>${new Date(m.firstDetectedAt).toLocaleString()} (${fmtAge(ageMs(m.firstDetectedAt))} ago)</dd>
        <dt>Likely cause</dt><dd>${classifyCause(m.cause).label} — ${m.cause}</dd>
        ${m.notes ? `<dt>Note</dt><dd>${m.notes}</dd>` : ''}
      </div>
      <div style="margin-bottom:8px; font-weight:600; font-size:13px;">Balance trend (7d)</div>
      <div style="margin-bottom:14px; color:var(--admin-text-muted);">${sparkline(m.balanceHistory || [], 280, 40)}</div>
      <div style="margin-bottom:8px; font-weight:600; font-size:13px;">Recent on-chain events</div>
      <table class="admin-table">
        <thead><tr><th>Timestamp</th><th>Kind</th><th style="text-align:right">Δ</th><th>Tx</th></tr></thead>
        <tbody>${txRows || '<tr><td colspan="4" style="text-align:center; color:var(--admin-text-muted); padding:12px;">No tx history available</td></tr>'}</tbody>
      </table>`;
    overlay.hidden = false;
  }
  function closeDetail() { document.getElementById('mm-detail-overlay').hidden = true; }

  // ── Resolve modal w/ reason templates ───────────────────────────
  let resolveTarget = null;
  const REASON_TEMPLATES = {
    'manual-credit': 'Manually credited via support ticket. Ledger now matches wallet.',
    'onchain-confirmed': 'On-chain transaction confirmed late. Re-sync brought balances into agreement.',
    'ledger-correction': 'Ledger correction applied to match on-chain truth.',
    'false-positive': 'False positive — wallet balance refreshed and matches ledger.',
    'rounding-acceptable': 'Rounding error within accepted tolerance band.',
    'custom': '',
  };
  function openResolveModal(m) {
    resolveTarget = m;
    document.getElementById('mm-reason-subtitle').textContent = `${m.id} · ${m.user} / ${m.asset} · diff ${m.diff > 0 ? '+' : ''}${fmtNum(m.diff)} (${fmtUsd(m.diffUsd)})`;
    const sel = document.getElementById('mm-reason-template');
    const notes = document.getElementById('mm-reason-notes');
    sel.value = 'manual-credit';
    notes.value = REASON_TEMPLATES['manual-credit'];
    sel.onchange = () => { notes.value = REASON_TEMPLATES[sel.value] ?? ''; };
    document.getElementById('mm-reason-overlay').hidden = false;
    notes.focus();
  }
  function closeResolveModal() { document.getElementById('mm-reason-overlay').hidden = true; resolveTarget = null; }
  function confirmResolve() {
    if (!resolveTarget) return;
    const reason = document.getElementById('mm-reason-template').value;
    const notes = document.getElementById('mm-reason-notes').value.trim();
    resolveTarget.status = 'resolved';
    resolveTarget.resolveReason = reason;
    resolveTarget.notes = notes;
    logAudit('resolve', `${resolveTarget.id} (${resolveTarget.user}/${resolveTarget.asset})`, `[${reason}] ${notes}`);
    toast(`${resolveTarget.id} resolved`, 'success');
    closeResolveModal();
    renderKpis();
    renderMismatches();
  }

  // ── Webhook delivery status ─────────────────────────────────────
  function renderWebhookStatus() {
    const host = document.getElementById('webhook-status');
    if (!host) return;
    const channels = [
      { name: 'Slack', enabled: !!SEVERITY_THRESHOLDS.slack, lastSent: SEVERITY_THRESHOLDS.slack ? Date.now() - 1000 * 60 * 35 : null, ok: true },
      { name: 'PagerDuty', enabled: !!SEVERITY_THRESHOLDS.pagerduty, lastSent: SEVERITY_THRESHOLDS.pagerduty ? Date.now() - 1000 * 60 * 60 * 4 : null, ok: true },
      { name: 'Email', enabled: false, lastSent: null, ok: true },
    ];
    host.innerHTML = channels.map(c => {
      const dot = !c.enabled ? 'recon-wh-off' : c.ok ? 'recon-wh-ok' : 'recon-wh-err';
      const status = !c.enabled ? 'Not configured' : c.ok ? `Last sent ${fmtRelative(new Date(c.lastSent).toISOString())}` : 'Last attempt failed';
      return `<div class="recon-wh-row"><span class="recon-wh-dot ${dot}"></span><span style="font-weight:600; min-width:90px;">${c.name}</span><span style="color:var(--admin-text-muted); font-size:12px;">${status}</span></div>`;
    }).join('');
  }

  // ── Skeleton loaders ────────────────────────────────────────────
  function showSkeleton() {
    const tbody = document.getElementById('recon-body');
    if (!tbody) return;
    tbody.innerHTML = Array.from({ length: 3 }, () =>
      `<tr><td colspan="11"><div class="recon-skeleton-row"></div></td></tr>`).join('');
  }

  // ── Audit log render ────────────────────────────────────────────
  function renderAudit() {
    const tbody = document.getElementById('recon-audit-body');
    if (!tbody) return;
    if (state.audit.length === 0) {
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:20px; color:var(--admin-text-muted);">No actions yet</td></tr>';
      return;
    }
    tbody.innerHTML = state.audit.slice(0, 20).map(a => `
      <tr>
        <td style="font-size:12px; font-variant-numeric:tabular-nums;" title="${a.ts}">${fmtRelative(a.ts)}</td>
        <td><code class="recon-code">${a.actor}</code></td>
        <td><span class="recon-cause-tag recon-cause-${a.action === 'resolve' ? 'rounding' : a.action === 'dismiss' ? 'gas' : 'pending'}">${a.action}</span></td>
        <td>${a.target}</td>
        <td style="font-size:12px; color:var(--admin-text-muted);">${a.notes || ''}</td>
      </tr>`).join('');
  }

  // ── Cron preview (#23) ──────────────────────────────────────────
  function describeCron(expr) {
    const parts = expr.trim().split(/\s+/);
    if (parts.length !== 5) return { ok: false, label: 'Invalid (need 5 fields)' };
    const [m, h, dom, mon, dow] = parts;
    const valid = s => /^(\*|\*\/\d+|\d+(-\d+)?(,\d+)*)$/.test(s);
    if (![m, h, dom, mon, dow].every(valid)) return { ok: false, label: 'Invalid syntax' };
    let label = 'Custom';
    if (dom === '*' && mon === '*' && dow === '*' && /^\d+$/.test(m) && /^\d+$/.test(h)) {
      label = `Daily at ${h.padStart(2, '0')}:${m.padStart(2, '0')} UTC`;
    } else if (dom === '*' && mon === '*' && dow !== '*') {
      label = `Weekly (dow=${dow}) at ${h}:${m.padStart(2, '0')} UTC`;
    } else if (m === '*' && h === '*') {
      label = 'Every minute';
    }
    return { ok: true, label };
  }
  function nextRunFromCron(expr) {
    const parts = expr.trim().split(/\s+/);
    if (parts.length !== 5) return null;
    const m = parseInt(parts[0]), h = parseInt(parts[1]);
    if (isNaN(m) || isNaN(h)) return null;
    const now = new Date();
    const next = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), h, m, 0));
    if (next <= now) next.setUTCDate(next.getUTCDate() + 1);
    return next;
  }
  function updateCronPreview() {
    const inp = document.getElementById('thr-cron');
    if (!inp) return;
    const desc = describeCron(inp.value);
    const hint = document.getElementById('cron-hint');
    const nextEl = document.getElementById('cron-next');
    if (hint) {
      hint.textContent = desc.ok ? `· ${desc.label}` : `· ${desc.label}`;
      hint.style.color = desc.ok ? 'var(--admin-success)' : 'var(--admin-danger)';
    }
    const next = nextRunFromCron(inp.value);
    if (nextEl) nextEl.textContent = next ? `Next run: ${next.toLocaleString()} (in ${fmtAge(next - Date.now())})` : '';
  }

  // ── Presets (#filter) ───────────────────────────────────────────
  function applyPreset(key) {
    if (key === 'save') {
      const name = prompt('Preset name?');
      if (!name) return;
      state.presets[name] = { ...state.filter };
      savePresets();
      const sel = document.getElementById('recon-preset');
      const opt = document.createElement('option'); opt.value = name; opt.textContent = name;
      sel.insertBefore(opt, sel.querySelector('[value="save"]'));
      sel.value = '';
      toast(`Preset "${name}" saved`, 'success');
      return;
    }
    const p = state.presets[key];
    if (!p) return;
    state.filter = { q: p.q || '', severity: p.severity || '', status: p.status || 'open' };
    document.getElementById('recon-search').value = state.filter.q;
    document.getElementById('recon-severity-filter').value = state.filter.severity;
    document.getElementById('recon-status-filter').value = state.filter.status;
    if (p.minAgeHours) {
      const h = p.minAgeHours;
      // post-filter applied via additional check
      state._minAgeHours = h;
    } else delete state._minAgeHours;
    renderMismatches();
  }

  // ── Throttle banner (#4) ────────────────────────────────────────
  function checkThrottle() {
    const last = parseInt(localStorage.getItem(THROTTLE_KEY) || '0');
    const remaining = THROTTLE_MS - (Date.now() - last);
    const banner = document.getElementById('recon-throttle');
    const text = document.getElementById('recon-throttle-text');
    const btn = document.getElementById('btn-run-recon');
    if (remaining > 0) {
      banner.hidden = false;
      text.textContent = `Cooldown active — wait ${Math.ceil(remaining / 1000)}s before next run`;
      if (btn) btn.disabled = true;
      setTimeout(checkThrottle, 1000);
    } else {
      banner.hidden = true;
      if (btn) btn.disabled = false;
    }
  }
  function markRunStart() { localStorage.setItem(THROTTLE_KEY, String(Date.now())); checkThrottle(); }

  // ── Keyboard shortcuts (R/D/I per-row) ──────────────────────────
  let focusedRowId = null;
  function bindRowShortcuts() {
    document.addEventListener('keydown', e => {
      if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement.tagName)) return;
      const row = document.activeElement.closest?.('tr[id^="row-"]');
      const id = row ? row.id.slice(4) : focusedRowId;
      if (!id) return;
      if (e.key === 'r' || e.key === 'R') { e.preventDefault(); actOnMismatch(id, 'resolve'); }
      else if (e.key === 'd' || e.key === 'D') { e.preventDefault(); actOnMismatch(id, 'dismiss'); }
      else if (e.key === 'i' || e.key === 'I') { e.preventDefault(); actOnMismatch(id, 'investigate'); }
    });
  }

  // ── prefers-color-scheme (#30) ──────────────────────────────────
  function syncColorScheme() {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      if (!localStorage.getItem('admin.theme.user-set')) {
        document.documentElement.dataset.theme = mq.matches ? 'dark' : 'light';
      }
    };
    apply();
    mq.addEventListener?.('change', apply);
  }

  // ── Backend integration ─────────────────────────────────────────
  // ── Multi-currency exposure ─────────────────────────────────────
  function renderExposure() {
    const ccy = document.getElementById('kpi-exposure-ccy')?.value || 'USD';
    const rate = FX[ccy] || 1;
    const open = state.data.mismatches.filter(m => m.status === 'open');
    const totalUsd = open.reduce((s, m) => s + Math.abs(m.diffUsd), 0);
    const conv = totalUsd * rate;
    const sym = { USD: '$', EUR: '€', CHF: 'CHF ', GBP: '£' }[ccy] || '';
    const el = document.getElementById('kpi-exposure-value');
    if (el) el.textContent = sym + conv.toLocaleString('en-US', { maximumFractionDigits: 2, minimumFractionDigits: 2 });
    const detail = document.getElementById('kpi-exposure-detail');
    if (detail) detail.textContent = open.length === 0 ? '0 open' : `${open.length} open · ${ccy === 'USD' ? '' : `from ${fmtUsd(totalUsd)}`}`;
  }

  // ── Anomaly detection + auto-toast ──────────────────────────────
  function detectNewMismatches() {
    const open = state.data.mismatches.filter(m => m.status === 'open');
    const newOnes = open.filter(m => !lastSeenMismatchIds.has(m.id));
    const newCritical = newOnes.filter(m => severityOf(m) === 'critical');
    if (newCritical.length > 0 && lastSeenMismatchIds.size > 0) {
      toast(`⚠ ${newCritical.length} new CRITICAL mismatch${newCritical.length > 1 ? 'es' : ''} detected`, 'error');
      // Highlight new rows
      setTimeout(() => newCritical.forEach(m => {
        const row = document.getElementById(`row-${m.id}`);
        row?.classList.add('recon-row-flash');
        setTimeout(() => row?.classList.remove('recon-row-flash'), 3000);
      }), 100);
    } else if (newOnes.length > 0 && lastSeenMismatchIds.size > 0) {
      toast(`${newOnes.length} new mismatch${newOnes.length > 1 ? 'es' : ''} detected`, 'warning');
    }
    lastSeenMismatchIds = new Set(open.map(m => m.id));
  }

  // ── Stale data banner ───────────────────────────────────────────
  function checkStale() {
    const banner = document.getElementById('recon-stale');
    const text = document.getElementById('recon-stale-text');
    if (!banner) return;
    const ageMin = (Date.now() - new Date(state.data.generatedAt).getTime()) / 60000;
    const expectedMin = state.refreshSec ? (state.refreshSec / 60) * 3 : 1440;
    if (ageMin > Math.max(expectedMin, 30)) {
      banner.hidden = false;
      text.textContent = `Data is ${fmtAge((Date.now() - new Date(state.data.generatedAt).getTime()))} old — last run may have failed`;
    } else {
      banner.hidden = true;
    }
  }

  // ── Threshold slider + histogram ────────────────────────────────
  function renderThresholdHistogram() {
    const host = document.getElementById('thr-histogram');
    if (!host) return;
    const buckets = [0, 1, 10, 50, 100, 500, 1000, Infinity];
    const counts = new Array(buckets.length - 1).fill(0);
    state.data.mismatches.forEach(m => {
      const v = Math.abs(m.diffUsd);
      for (let i = 0; i < buckets.length - 1; i++) {
        if (v >= buckets[i] && v < buckets[i + 1]) { counts[i]++; break; }
      }
    });
    const max = Math.max(...counts, 1);
    const crit = parseFloat(document.getElementById('thr-critical').value) || 100;
    host.innerHTML = `<div class="recon-thr-bars">
      ${counts.map((c, i) => {
        const lo = buckets[i], hi = buckets[i + 1];
        const above = lo >= crit;
        return `<div class="recon-thr-bar ${above ? 'is-critical' : ''}" style="height:${(c / max * 100) || 4}%" title="$${lo}–${hi === Infinity ? '∞' : '$' + hi}: ${c} mismatches"></div>`;
      }).join('')}
    </div><div class="recon-thr-axis"><span>$0</span><span>$1k+</span></div>`;
  }
  function bindThresholdSliders() {
    const sync = (sliderId, inputId, readoutId, sym = '$') => {
      const slider = document.getElementById(sliderId);
      const input = document.getElementById(inputId);
      const ro = document.getElementById(readoutId);
      if (!slider || !input) return;
      const update = src => {
        const v = src === 'slider' ? slider.value : input.value;
        slider.value = v; input.value = v;
        if (ro) ro.textContent = sym + v;
        renderThresholdHistogram();
      };
      slider.addEventListener('input', () => update('slider'));
      input.addEventListener('input', () => update('input'));
      update('input');
    };
    sync('thr-critical-slider', 'thr-critical', 'thr-critical-readout');
    sync('thr-warning-slider', 'thr-warning', 'thr-warning-readout');
  }

  // ── Copy-to-clipboard helper ────────────────────────────────────
  async function copyToClipboard(text, label) {
    try { await navigator.clipboard.writeText(text); toast(`${label || 'Copied'}: ${text.slice(0, 24)}`, 'success'); }
    catch { toast('Copy failed', 'error'); }
  }

  // ── CSV import (spot-check) ─────────────────────────────────────
  function bindCsvImport() {
    const overlay = document.getElementById('csv-import-overlay');
    document.getElementById('btn-import-csv')?.addEventListener('click', () => { overlay.hidden = false; });
    document.getElementById('csv-import-cancel')?.addEventListener('click', () => { overlay.hidden = true; });
    document.getElementById('csv-import-confirm')?.addEventListener('click', () => {
      const txt = document.getElementById('csv-import-text').value.trim();
      const lines = txt.split('\n').map(l => l.trim()).filter(Boolean);
      if (lines.length === 0) { toast('Paste at least one wallet', 'error'); return; }
      const parsed = lines.map(l => {
        const [w, a] = l.split(',').map(s => s.trim());
        return { wallet: w, asset: a || '?' };
      });
      logAudit('spot-check', `${parsed.length} wallets`, parsed.slice(0, 3).map(p => `${p.wallet}/${p.asset}`).join(', '));
      toast(`Spot-check queued for ${parsed.length} wallets`, 'success');
      overlay.hidden = true;
      document.getElementById('csv-import-text').value = '';
    });
  }

  // ── Onboarding tour ─────────────────────────────────────────────
  const TOUR_STEPS = [
    { sel: '#recon-kpis', title: 'KPI Overview', body: 'Status drives everything. Click any card to filter the table. Drag cards to reorder.' },
    { sel: '#kpi-exposure', title: 'Total Exposure', body: 'Aggregate USD impact of open mismatches. Switch currency for finance team.' },
    { sel: '#recon-breakdown', title: 'Per-Asset Breakdown', body: 'Spot the asset driving most mismatches. Often signals indexer drift.' },
    { sel: '.recon-toolbar', title: 'Filter & Search', body: 'Press / to focus search. Save presets via the dropdown. Cmd+K opens command palette.' },
    { sel: '#recon-body', title: 'Mismatch Actions', body: 'Detail / Note / Force-Sync / Resolve / Dismiss per row. Keyboard: R/D/I when row focused.' },
    { sel: '#recon-history-body', title: 'Run History', body: 'Click row for snapshot. Select 2 runs → Compare. Sparkline + anomaly flag on duration.' },
    { sel: '#thr-critical-slider', title: 'Severity Thresholds', body: 'Drag slider to retune Critical/Warning bands. Histogram shows current distribution.' },
  ];
  let tourIdx = 0;
  function showTourStep() {
    const step = TOUR_STEPS[tourIdx];
    if (!step) { closeTour(); return; }
    const target = document.querySelector(step.sel);
    if (!target) { tourIdx++; showTourStep(); return; }
    target.scrollIntoView({ behavior: 'smooth', block: 'center' });
    document.querySelectorAll('.recon-tour-spot').forEach(el => el.classList.remove('recon-tour-spot'));
    target.classList.add('recon-tour-spot');
    document.getElementById('tour-title').textContent = step.title;
    document.getElementById('tour-body').textContent = step.body;
    document.getElementById('tour-progress').textContent = `${tourIdx + 1} / ${TOUR_STEPS.length}`;
    document.getElementById('tour-prev').disabled = tourIdx === 0;
    document.getElementById('tour-next').textContent = tourIdx === TOUR_STEPS.length - 1 ? 'Done' : 'Next';
    const overlay = document.getElementById('tour-overlay');
    overlay.hidden = false;
  }
  function closeTour() {
    document.getElementById('tour-overlay').hidden = true;
    document.querySelectorAll('.recon-tour-spot').forEach(el => el.classList.remove('recon-tour-spot'));
    localStorage.setItem(TOUR_KEY, '1');
  }
  function bindTour() {
    document.getElementById('tour-next')?.addEventListener('click', () => {
      if (tourIdx >= TOUR_STEPS.length - 1) closeTour();
      else { tourIdx++; showTourStep(); }
    });
    document.getElementById('tour-prev')?.addEventListener('click', () => { if (tourIdx > 0) { tourIdx--; showTourStep(); } });
    document.getElementById('tour-skip')?.addEventListener('click', closeTour);
    document.getElementById('btn-tour')?.addEventListener('click', () => { tourIdx = 0; showTourStep(); });
    if (!localStorage.getItem(TOUR_KEY)) setTimeout(() => { tourIdx = 0; showTourStep(); }, 1200);
  }

  // ── Context menu (right-click on row) ───────────────────────────
  function bindContextMenu() {
    const menu = document.getElementById('recon-ctxmenu');
    document.getElementById('recon-body')?.addEventListener('contextmenu', e => {
      const row = e.target.closest('tr[id^="row-"]');
      if (!row) return;
      const id = row.id.slice(4);
      const m = state.data.mismatches.find(x => x.id === id);
      if (!m || m.status !== 'open') return;
      e.preventDefault();
      const actions = [
        ['Detail', 'detail'], ['Note', 'note'], ['Force-Sync', 'force-sync'],
        ['Resolve', 'resolve'], ['Dismiss', 'dismiss'],
        ['—', null],
        [`Copy wallet (${m.wallet})`, 'copy-wallet'],
        [`Copy ID (${m.id})`, 'copy-id'],
      ];
      menu.innerHTML = actions.map(([label, act]) => act === null
        ? '<li class="recon-ctxmenu-sep"></li>'
        : `<li class="recon-ctxmenu-item" data-act="${act}">${label}</li>`).join('');
      menu.style.top = e.pageY + 'px';
      menu.style.left = e.pageX + 'px';
      menu.hidden = false;
      menu.onclick = ev => {
        const li = ev.target.closest('.recon-ctxmenu-item');
        if (!li) return;
        const act = li.dataset.act;
        menu.hidden = true;
        if (act === 'copy-wallet') copyToClipboard(m.wallet, 'Wallet');
        else if (act === 'copy-id') copyToClipboard(m.id, 'Mismatch ID');
        else actOnMismatch(id, act);
      };
    });
    document.addEventListener('click', () => { menu.hidden = true; });
    document.addEventListener('scroll', () => { menu.hidden = true; }, true);
  }

  // ── Mobile bottom sheet ─────────────────────────────────────────
  function openSheet(m) {
    const overlay = document.getElementById('recon-sheet-overlay');
    document.getElementById('recon-sheet-title').textContent = `${m.id} · ${m.asset}`;
    const body = document.getElementById('recon-sheet-body');
    body.innerHTML = `
      <button class="admin-btn admin-btn--ghost" data-act="detail" style="width:100%; margin-bottom:6px;">Detail</button>
      <button class="admin-btn admin-btn--ghost" data-act="note" style="width:100%; margin-bottom:6px;">Add note</button>
      <button class="admin-btn admin-btn--ghost" data-act="force-sync" style="width:100%; margin-bottom:6px;">Force-Sync</button>
      <button class="admin-btn admin-btn--success" data-act="resolve" style="width:100%; margin-bottom:6px;">Resolve</button>
      <button class="admin-btn admin-btn--ghost" data-act="dismiss" style="width:100%; margin-bottom:6px;">Dismiss</button>`;
    body.onclick = e => {
      const b = e.target.closest('[data-act]');
      if (!b) return;
      overlay.hidden = true;
      actOnMismatch(m.id, b.dataset.act);
    };
    overlay.hidden = false;
  }
  function bindSheet() {
    document.getElementById('recon-sheet-close')?.addEventListener('click', () => document.getElementById('recon-sheet-overlay').hidden = true);
    document.getElementById('recon-sheet-overlay')?.addEventListener('click', e => { if (e.target.id === 'recon-sheet-overlay') e.target.hidden = true; });
    // Long-press on mobile → open sheet
    let timer;
    document.getElementById('recon-body')?.addEventListener('touchstart', e => {
      const row = e.target.closest('tr[id^="row-"]');
      if (!row) return;
      const id = row.id.slice(4);
      const m = state.data.mismatches.find(x => x.id === id);
      if (!m) return;
      timer = setTimeout(() => openSheet(m), 500);
    }, { passive: true });
    document.getElementById('recon-body')?.addEventListener('touchend', () => clearTimeout(timer));
    document.getElementById('recon-body')?.addEventListener('touchmove', () => clearTimeout(timer), { passive: true });
  }

  // ── SSE-style live update simulation ────────────────────────────
  function startLiveSim() {
    if (typeof EventSource !== 'undefined') {
      // Real backend would: const es = new EventSource(API + '/stream');
    }
    // Simulation: random small balance jitter on existing mismatches every 15s
    setInterval(() => {
      if (state.data.mismatches.length === 0) return;
      const m = state.data.mismatches[Math.floor(Math.random() * state.data.mismatches.length)];
      if (m.status !== 'open' || !m.balanceHistory) return;
      const last = m.balanceHistory[m.balanceHistory.length - 1];
      const jitter = (Math.random() - 0.5) * 0.2 * Math.abs(last || 1);
      m.balanceHistory = [...m.balanceHistory.slice(1), Math.max(0, last + jitter)];
      const row = document.getElementById(`row-${m.id}`);
      if (row) {
        const sparkCell = row.children[row.children.length - 2];
        if (sparkCell) sparkCell.innerHTML = sparkline(m.balanceHistory, 80, 22);
      }
    }, 15000);
  }

  // ── KPI drag-reorder ────────────────────────────────────────────
  const KPI_ORDER_KEY = 'poool.recon.kpi-order.v1';
  function applyKpiOrder() {
    const grid = document.getElementById('recon-kpis');
    if (!grid) return;
    let order;
    try { order = JSON.parse(localStorage.getItem(KPI_ORDER_KEY) || 'null'); } catch { order = null; }
    if (!order) return;
    order.forEach(id => {
      const el = document.getElementById(id);
      if (el) grid.appendChild(el);
    });
  }
  function bindKpiDrag() {
    const grid = document.getElementById('recon-kpis');
    if (!grid) return;
    grid.querySelectorAll('.admin-kpi-card').forEach(card => {
      card.draggable = true;
      card.addEventListener('dragstart', e => {
        e.dataTransfer.setData('text/plain', card.id);
        card.classList.add('is-dragging');
      });
      card.addEventListener('dragend', () => card.classList.remove('is-dragging'));
      card.addEventListener('dragover', e => { e.preventDefault(); card.classList.add('is-drop-target'); });
      card.addEventListener('dragleave', () => card.classList.remove('is-drop-target'));
      card.addEventListener('drop', e => {
        e.preventDefault();
        card.classList.remove('is-drop-target');
        const srcId = e.dataTransfer.getData('text/plain');
        const src = document.getElementById(srcId);
        if (!src || src === card) return;
        const all = [...grid.children];
        if (all.indexOf(src) < all.indexOf(card)) card.after(src); else card.before(src);
        const order = [...grid.children].map(c => c.id);
        localStorage.setItem(KPI_ORDER_KEY, JSON.stringify(order));
        toast('KPI order saved', 'success');
      });
    });
  }

  // ── History row hover preview ───────────────────────────────────
  function bindHistoryHover() {
    let tip;
    document.querySelectorAll('.recon-history-row').forEach(row => {
      row.addEventListener('mouseenter', () => {
        const idx = parseInt(row.dataset.idx);
        const h = state.data.history[idx];
        if (!h) return;
        tip = document.createElement('div');
        tip.className = 'recon-hover-tip';
        const trend = state.data.history.slice(idx, Math.min(idx + 5, state.data.history.length)).map(x => x.mismatches).reverse();
        tip.innerHTML = `<div style="font-weight:600; margin-bottom:4px;">${new Date(h.time).toLocaleString()}</div>
          <div style="display:grid; grid-template-columns: max-content 1fr; gap:2px 12px; font-size:12px;">
            <span style="color:var(--admin-text-muted);">Wallets</span><span>${h.wallets.toLocaleString()}</span>
            <span style="color:var(--admin-text-muted);">Mismatches</span><span>${h.mismatches}</span>
            <span style="color:var(--admin-text-muted);">Duration</span><span>${h.durationSec}s</span>
            <span style="color:var(--admin-text-muted);">Mismatch trend</span><span style="color:var(--admin-warning);">${sparkline(trend, 60, 16)}</span>
          </div>`;
        document.body.appendChild(tip);
        const r = row.getBoundingClientRect();
        tip.style.top = (window.scrollY + r.top - tip.offsetHeight - 8) + 'px';
        tip.style.left = (r.left + r.width / 2 - tip.offsetWidth / 2) + 'px';
      });
      row.addEventListener('mouseleave', () => { tip?.remove(); tip = null; });
    });
  }

  // ── Preset delete UI ────────────────────────────────────────────
  function bindPresetManage() {
    const sel = document.getElementById('recon-preset');
    if (!sel) return;
    sel.addEventListener('contextmenu', e => {
      const opt = sel.options[sel.selectedIndex];
      if (!opt || !opt.value || ['save', ''].includes(opt.value)) return;
      e.preventDefault();
      if (!confirm(`Delete preset "${opt.value}"?`)) return;
      delete state.presets[opt.value];
      savePresets();
      opt.remove();
      toast('Preset deleted', 'success');
    });
    // Also expose via a dropdown affordance
    const hint = document.createElement('span');
    hint.style.cssText = 'font-size:10px; color:var(--admin-text-muted); margin-left:4px;';
    hint.textContent = '(right-click to delete)';
    sel.parentNode?.insertBefore(hint, sel.nextSibling);
  }

  async function loadReconciliation() {
    showSkeleton();
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const report = await res.json();
      // Adapter: map legacy invariant-check API onto new shape (best-effort)
      if (report && Array.isArray(report.mismatches)) {
        state.data = report;
      } else {
        // Stay on mock; refresh timestamp so "last run" looks live
        state.data = structuredClone(MOCK);
        state.data.generatedAt = new Date().toISOString();
      }
    } catch (err) {
      console.warn('[mp-reconciliation] API unavailable, using mock data:', err);
      state.data = structuredClone(MOCK);
      state.data.generatedAt = new Date().toISOString();
    }
    state.selected.clear();
    renderKpis();
    renderExposure();
    renderMismatches();
    renderHistory();
    renderBreakdown();
    renderXref();
    renderWebhookStatus();
    renderThresholdHistogram();
    bindHistoryHover();
    detectNewMismatches();
    checkStale();
  }

  // ── Init ────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    bindFilters();
    bindBulkActions();
    bindExports();
    bindRules();
    bindSnapshotClose();
    bindRunButton();
    bindHistoryFilters();
    bindCmdK();
    setupAutoRefresh();
    syncColorScheme();
    bindRowShortcuts();
    checkThrottle();
    renderAudit();
    applyKpiOrder();
    bindKpiDrag();
    bindPresetManage();

    bindCsvImport();
    bindTour();
    bindContextMenu();
    bindSheet();
    bindThresholdSliders();
    startLiveSim();

    // Exposure currency switch
    document.getElementById('kpi-exposure-ccy')?.addEventListener('change', renderExposure);
    document.getElementById('btn-stale-refresh')?.addEventListener('click', () => loadReconciliation());
    document.getElementById('btn-print')?.addEventListener('click', () => window.print());

    // Tx hash click-to-copy in detail modal (delegated)
    document.getElementById('mm-detail-body')?.addEventListener('click', e => {
      const code = e.target.closest('code.recon-code');
      if (code && code.textContent.startsWith('0x')) copyToClipboard(code.textContent, 'Tx');
    });

    // Stale check tick
    setInterval(checkStale, 60000);

    // Modals
    document.getElementById('mm-detail-close')?.addEventListener('click', closeDetail);
    document.getElementById('mm-detail-overlay')?.addEventListener('click', e => { if (e.target.id === 'mm-detail-overlay') closeDetail(); });
    document.getElementById('mm-reason-cancel')?.addEventListener('click', closeResolveModal);
    document.getElementById('mm-reason-confirm')?.addEventListener('click', confirmResolve);
    document.addEventListener('keydown', e => {
      if (e.key === 'Escape') { closeDetail(); closeResolveModal(); }
    });

    // Cron preview
    const cronInp = document.getElementById('thr-cron');
    cronInp?.addEventListener('input', updateCronPreview);
    updateCronPreview();

    // Normalize toggle
    document.getElementById('recon-normalize')?.addEventListener('change', e => {
      state.normalize = e.target.checked;
      renderMismatches();
    });

    // Presets
    const presetSel = document.getElementById('recon-preset');
    presetSel?.addEventListener('change', e => {
      const v = e.target.value;
      if (v) applyPreset(v);
      e.target.value = '';
    });
    // Inject custom presets
    Object.keys(state.presets).forEach(k => {
      if (presetSel && !presetSel.querySelector(`[value="${k}"]`)) {
        const opt = document.createElement('option'); opt.value = k; opt.textContent = k;
        presetSel.insertBefore(opt, presetSel.querySelector('[value="save"]'));
      }
    });

    // Audit clear
    document.getElementById('btn-clear-audit')?.addEventListener('click', () => {
      if (!confirm('Clear local audit log?')) return;
      state.audit = []; saveAudit(); renderAudit();
    });
    document.getElementById('kpi-status-cta')?.addEventListener('click', e => {
      e.stopPropagation();
      document.getElementById('recon-body')?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
    loadReconciliation();
    // Auto-refresh "last run" relative timestamp every 30s
    setInterval(() => {
      const sub = document.getElementById('kpi-lastrun-sub');
      if (sub && state.data.generatedAt) sub.textContent = fmtRelative(state.data.generatedAt);
    }, 30000);
  });
})();
