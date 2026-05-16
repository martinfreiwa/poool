/* global window, document */

/** Products sub-page — assets sold via team links.
 *
 *  Phase-1: replace the hard-coded 5-year window with a real preset-bar
 *  (7d / 30d / this-month / YTD / all + custom from/to). State persists to
 *  localStorage + URL so the user's last window survives reloads.
 *
 *  Backend contract: `from` / `to` are inclusive ISO dates (YYYY-MM-DD); when
 *  both are empty the route returns all-time data.
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  let _lastRows = [];
  let _table = null;

  const RANGE_LS = 'dat:dateRange:products';

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

  function exportCsv() {
    if (!_lastRows.length) {
      DAT.toast('Nothing to export', 'No product sales in the current view.', 'info');
      return;
    }
    const header = ['Asset', 'Units sold', 'Unique buyers', 'Gross revenue (EUR)', 'Avg sale (EUR)', 'Commission (EUR)', 'Last sale'];
    const body = _lastRows.map((p) => [
      p.asset_name || p.asset_id || '',
      p.units_sold,
      p.n_buyers,
      ((p.gross_revenue_cents || 0) / 100).toFixed(2),
      ((p.avg_sale_cents || 0) / 100).toFixed(2),
      ((p.commission_cents || 0) / 100).toFixed(2),
      p.last_sale_at || '',
    ]);
    const stamp = new Date().toISOString().slice(0, 10);
    DAT.downloadCsv(`team-product-sales-${stamp}.csv`, [header, ...body]);
  }

  // ── Date-range wiring ───────────────────────────────────────────────────
  function loadRange() {
    const url = new URL(location.href);
    const fromUrl = url.searchParams.get('from') || '';
    const toUrl   = url.searchParams.get('to')   || '';
    if (fromUrl || toUrl) return { from: fromUrl, to: toUrl, preset: '' };
    try {
      const p = JSON.parse(localStorage.getItem(RANGE_LS) || 'null');
      if (p && typeof p === 'object') return { from: p.from || '', to: p.to || '', preset: p.preset || '' };
    } catch {}
    // Default: last 30 days. Matches Analytics page default.
    return { from: daysAgoIso(30), to: todayIso(), preset: '30d' };
  }
  function saveRange(state) {
    try { localStorage.setItem(RANGE_LS, JSON.stringify(state)); } catch {}
  }
  function paintPresets(activePreset) {
    DAT.$$('#dat-products-date-range .dat-preset').forEach((btn) => {
      const on = btn.dataset.preset === activePreset;
      btn.setAttribute('aria-pressed', on ? 'true' : 'false');
      btn.classList.toggle('dat-preset--active', on);
    });
  }
  function applyPreset(preset, fromInp, toInp) {
    if (preset === 'all') {
      // Backend defaults missing `from` to "30 days ago" — send an epoch
      // sentinel so "All time" actually returns all-time data.
      fromInp.value = '2000-01-01'; toInp.value = todayIso();
    } else if (preset === '7d') {
      fromInp.value = daysAgoIso(7);  toInp.value = todayIso();
    } else if (preset === '30d') {
      fromInp.value = daysAgoIso(30); toInp.value = todayIso();
    } else if (preset === 'this-month') {
      fromInp.value = monthStartIso(); toInp.value = todayIso();
    } else if (preset === 'ytd') {
      fromInp.value = yearStartIso(); toInp.value = todayIso();
    }
  }

  let _rangeState = { from: '', to: '', preset: '' };

  document.addEventListener('DOMContentLoaded', () => {
    const tbody = DAT.$('#dat-products-tbody');
    const theadRow = DAT.$('#dat-products-thead-row');
    const pagerHost = DAT.$('#dat-products-pager-host');
    if (!tbody || !theadRow || !pagerHost) return;

    const fromInp = DAT.$('#dat-products-from');
    const toInp   = DAT.$('#dat-products-to');

    _rangeState = loadRange();
    if (fromInp) fromInp.value = _rangeState.from;
    if (toInp)   toInp.value   = _rangeState.to;
    paintPresets(_rangeState.preset);

    DAT.$$('#dat-products-date-range .dat-preset').forEach((btn) => {
      btn.addEventListener('click', () => {
        const p = btn.dataset.preset;
        applyPreset(p, fromInp, toInp);
        _rangeState = { from: fromInp.value, to: toInp.value, preset: p };
        paintPresets(p);
        saveRange(_rangeState);
        _table?.reload();
      });
    });
    const onCustom = () => {
      _rangeState = { from: fromInp ? fromInp.value : '', to: toInp ? toInp.value : '', preset: '' };
      paintPresets('');
      saveRange(_rangeState);
      _table?.reload();
    };
    if (fromInp) fromInp.addEventListener('change', onCustom);
    if (toInp)   toInp.addEventListener('change', onCustom);

    _table = DAT.dataTable({
      pageKey: 'products',
      endpoint: '/api/developer/affiliate/team/products',
      tbody, theadRow, pagerHost,
      extraParams: () => ({
        from: _rangeState.from || '',
        to:   _rangeState.to   || '',
      }),
      emptyText: 'No assets sold via your team yet. Once customers your team referred make purchases, each asset rolls up here.',
      columns: [
        { key: 'asset_name', label: 'Asset', sortable: true,
          render: (r) => DAT.el('span', { class: 'dat-cell-strong' }, r.asset_name || r.asset_id || '—') },
        { key: 'units_sold', label: 'Units sold', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => Number(r.units_sold || 0).toLocaleString() },
        { key: 'n_buyers', label: 'Unique buyers', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => Number(r.n_buyers || 0).toLocaleString() },
        { key: 'gross_revenue', label: 'Gross revenue', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.gross_revenue_cents) },
        { key: 'avg_sale', label: 'Avg sale', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.avg_sale_cents) },
        { key: 'commission', label: 'Commission', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.commission_cents) },
        { key: 'last_sale', label: 'Last sale', sortable: true, defaultDir: 'desc',
          render: (r) => r.last_sale_at ? DAT.fmtDate(r.last_sale_at) : DAT.el('span', { class: 'dat-muted' }, '—') },
      ],
      onRowsLoaded: (rows) => { _lastRows = rows; },
    });

    const exp = DAT.$('#dat-products-export');
    if (exp) exp.addEventListener('click', exportCsv);
  });
})();
