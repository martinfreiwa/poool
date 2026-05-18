/* global window, document */

/** Customers sub-page — referred users with sort/search/pagination.
 *
 *  Phase-1 upgrades:
 *    - Status chip-bar (multi-select) above the table.
 *    - Date-range filter (presets + custom from/to) on `acquired`.
 *    - Lifecycle column (New / Active / Dormant / Churned) derived from
 *      last_activity_at.
 *    - "Days since last order" column with traffic-light coloring.
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  let _lastRows = [];
  let _table = null;

  // ── Persistence keys ────────────────────────────────────────────────────
  const RANGE_LS = 'dat:dateRange:customers';

  // ── Date helpers (local-tz YYYY-MM-DD) ──────────────────────────────────
  function isoDay(d) {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  function todayIso() { return isoDay(new Date()); }
  function daysAgoIso(n) {
    const d = new Date(); d.setDate(d.getDate() - n); return isoDay(d);
  }
  function monthStartIso() {
    const d = new Date(); return isoDay(new Date(d.getFullYear(), d.getMonth(), 1));
  }
  function yearStartIso() {
    const d = new Date(); return isoDay(new Date(d.getFullYear(), 0, 1));
  }

  function daysSince(iso) {
    if (!iso) return null;
    const t = new Date(iso).getTime();
    if (Number.isNaN(t)) return null;
    return Math.floor((Date.now() - t) / 86_400_000);
  }

  // ── Lifecycle stage from last_activity_at ───────────────────────────────
  // "New":     no activity recorded yet (account exists, no orders)
  // "Active":  last activity within 30 days
  // "Dormant": 31..90 days since last activity
  // "Churned": > 90 days
  function lifecycleStage(row) {
    const ds = daysSince(row.last_activity_at);
    if (ds == null) return { key: 'new', label: 'New' };
    if (ds <= 30)   return { key: 'active', label: 'Active' };
    if (ds <= 90)   return { key: 'dormant', label: 'Dormant' };
    return { key: 'churned', label: 'Churned' };
  }

  function lifecycleCell(row) {
    const { key, label } = lifecycleStage(row);
    return DAT.el('span', {
      class: `dat-lifecycle dat-lifecycle--${key}`,
      title: `Lifecycle: ${label}`,
    }, label);
  }

  function daysSinceCell(row) {
    const ds = daysSince(row.last_activity_at);
    if (ds == null) return DAT.el('span', { class: 'dat-muted' }, '—');
    let tone = 'good';      // ≤30
    if (ds > 90) tone = 'bad';
    else if (ds > 30) tone = 'warn';
    return DAT.el('span', {
      class: `dat-days dat-days--${tone}`,
      title: `Last activity ${ds} day${ds === 1 ? '' : 's'} ago`,
    }, `${ds}d`);
  }

  // ── Existing cell renderers ─────────────────────────────────────────────
  function customerCell(row) {
    const wrap = DAT.el('div', { class: 'dat-cell-stack' });
    wrap.appendChild(DAT.el('span', { class: 'dat-cell-strong' }, row.full_name || row.email || '—'));
    if (row.full_name && row.email) {
      wrap.appendChild(DAT.el('span', { class: 'dat-cell-sub' }, row.email));
    }
    return wrap;
  }

  function statusPill(status) {
    return DAT.el(
      'span',
      { class: `dat-status dat-status--${status}`, title: DAT.humanize(status) },
      DAT.humanize(status),
    );
  }

  // ── CSV export ──────────────────────────────────────────────────────────
  function exportCsv() {
    if (!_lastRows.length) {
      DAT.toast('Nothing to export', 'No customers in the current view.', 'info');
      return;
    }
    const header = ['Customer', 'Email', 'Via member', 'Status', 'Invested (EUR)', 'Commission (EUR)', 'Orders', 'Lifecycle', 'Last activity', 'Days since', 'Acquired'];
    const body = _lastRows.map((c) => [
      c.full_name || '',
      c.email || '',
      c.attribution_user_name || '',
      DAT.humanize(c.referral_status),
      ((c.gross_invested_cents || 0) / 100).toFixed(2),
      ((c.commission_earned_cents || 0) / 100).toFixed(2),
      c.n_purchases || 0,
      lifecycleStage(c).label,
      c.last_activity_at || '',
      (daysSince(c.last_activity_at) ?? ''),
      c.created_at || '',
    ]);
    const stamp = new Date().toISOString().slice(0, 10);
    DAT.downloadCsv(`team-customers-${stamp}.csv`, [header, ...body]);
  }

  // ── Date-range wiring ───────────────────────────────────────────────────
  function loadRange() {
    // URL > localStorage > empty (= all time)
    const url = new URL(location.href);
    const fromUrl = url.searchParams.get('from') || '';
    const toUrl   = url.searchParams.get('to')   || '';
    if (fromUrl || toUrl) return { from: fromUrl, to: toUrl, preset: '' };
    try {
      const p = JSON.parse(localStorage.getItem(RANGE_LS) || 'null');
      if (p && typeof p === 'object') return { from: p.from || '', to: p.to || '', preset: p.preset || '' };
    } catch {}
    return { from: '', to: '', preset: '' };
  }
  function saveRange(state) {
    try { localStorage.setItem(RANGE_LS, JSON.stringify(state)); } catch {}
  }

  // ── Boot ────────────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    const tbody     = DAT.$('#dat-customers-tbody');
    const theadRow  = DAT.$('#dat-customers-thead-row');
    const pagerHost = DAT.$('#dat-customers-pager-host');
    const pagerFooterHost = DAT.$('#dat-customers-pager-footer');
    if (!tbody || !theadRow || !pagerHost) return;

    // Topbar date-range owns from/to; reload the table on each change.
    DAT.topbarDateRange({ onChange: () => _table?.reload() });

    _table = DAT.dataTable({
      pageKey: 'customers',
      endpoint: '/api/developer/affiliate/team/customers',
      tbody, theadRow, pagerHost, pagerFooterHost,
      extraParams: () => {
        const r = DAT.currentRange();
        return { from: r.from || '', to: r.to || '' };
      },
      emptyText: 'No customers match your search. Try a different keyword or date range.',
      columns: [
        { key: 'full_name', label: 'Customer', sortable: true, render: customerCell },
        { key: 'via_member', label: 'Via member', sortable: true,
          render: (r) => r.attribution_user_name || DAT.el('span', { class: 'dat-muted' }, 'Unattributed') },
        { key: 'status', label: 'Status', sortable: true, render: (r) => statusPill(r.referral_status) },
        { key: 'invested', label: 'Invested', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.gross_invested_cents) },
        { key: 'commission', label: 'Commission', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.commission_earned_cents) },
        { key: 'n_purchases', label: 'Orders', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => (r.n_purchases || 0).toLocaleString() },
        { key: 'lifecycle', label: 'Lifecycle', sortable: false, render: lifecycleCell },
        { key: 'last_activity', label: 'Last activity', sortable: true, defaultDir: 'desc',
          render: (r) => r.last_activity_at ? DAT.fmtDate(r.last_activity_at) : DAT.el('span', { class: 'dat-muted' }, '—') },
        { key: 'days_since', label: 'Days since', sortable: false, numeric: true, render: daysSinceCell },
        { key: 'acquired', label: 'Acquired', sortable: true, defaultDir: 'desc',
          render: (r) => DAT.fmtDate(r.created_at) },
      ],
      onRowsLoaded: (rows) => { _lastRows = rows; },
    });

    const exp = DAT.$('#dat-customers-export');
    if (exp) exp.addEventListener('click', exportCsv);
  });
})();
