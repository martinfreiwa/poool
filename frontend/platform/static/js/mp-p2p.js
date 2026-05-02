/**
 * P2P Offers Oversight — mp-p2p.js
 *
 * Client-side filter, search, sort, anomaly threshold, aging,
 * auto-refresh, drilldown drawer, bulk-cancel and CSV export over
 * /api/admin/marketplace/p2p (LIMIT 200).
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/p2p';
  const REFRESH_MS = 30_000;
  const STALE_SECONDS = 24 * 3600;        // 24h pending → warn
  const VERY_STALE_SECONDS = 72 * 3600;   // 72h pending → danger
  const STORAGE_THRESHOLD = 'p2p.threshold.pct';
  const STORAGE_COLUMNS = 'p2p.columns.hidden';
  const STORAGE_PAGE_SIZE = 'p2p.page_size';
  const STORAGE_ASSET_THRESHOLDS = 'p2p.threshold.per_asset';

  const COLUMNS = [
    { key: 'id', label: 'Offer ID', sticky: true },
    { key: 'side', label: 'Side' },
    { key: 'maker', label: 'Maker' },
    { key: 'taker', label: 'Taker' },
    { key: 'asset', label: 'Asset' },
    { key: 'quantity', label: 'Qty' },
    { key: 'price', label: 'Offer Price' },
    { key: 'market_price', label: 'Market Price' },
    { key: 'deviation', label: 'Deviation' },
    { key: 'status', label: 'Status' },
    { key: 'age', label: 'Age' },
    { key: 'expires', label: 'Expires' },
    { key: 'created', label: 'Created' },
    { key: 'action', label: 'Action', sticky: true },
  ];

  const state = {
    offers: [],
    total: 0,
    page: 1,
    pageSize: loadPageSize(),
    threshold: loadThreshold(),
    assetThresholds: loadAssetThresholds(),
    hiddenCols: loadHiddenCols(),
    filters: {
      search: '',
      status: '',
      side: '',
      asset: '',
      flaggedOnly: false,
      range: '',
    },
    sort: { key: 'created_at', dir: 'desc' },
    selected: new Set(),
    autoRefresh: true,
    refreshTimer: null,
    lastUpdated: null,
  };

  /* ───────────── helpers ───────────── */

  function csrfToken() {
    return document.cookie
      .split(';')
      .map((p) => p.trim())
      .find((p) => p.startsWith('csrf_token='))
      ?.split('=')
      .slice(1)
      .join('=') || '';
  }

  function loadThreshold() {
    const raw = parseFloat(localStorage.getItem(STORAGE_THRESHOLD));
    return Number.isFinite(raw) && raw >= 0 ? raw : 5;
  }

  function saveThreshold(value) {
    localStorage.setItem(STORAGE_THRESHOLD, String(value));
  }

  function loadPageSize() {
    const v = parseInt(localStorage.getItem(STORAGE_PAGE_SIZE), 10);
    return [10, 25, 50, 100, 200].includes(v) ? v : 50;
  }
  function savePageSize(v) {
    localStorage.setItem(STORAGE_PAGE_SIZE, String(v));
  }

  function loadAssetThresholds() {
    try {
      const raw = JSON.parse(localStorage.getItem(STORAGE_ASSET_THRESHOLDS) || '{}');
      return raw && typeof raw === 'object' ? raw : {};
    } catch {
      return {};
    }
  }
  function saveAssetThresholds() {
    localStorage.setItem(STORAGE_ASSET_THRESHOLDS, JSON.stringify(state.assetThresholds));
  }
  function effectiveThreshold(offer) {
    const per = offer && state.assetThresholds[offer.asset_id];
    return Number.isFinite(per) ? per : state.threshold;
  }

  function loadHiddenCols() {
    try {
      const raw = JSON.parse(localStorage.getItem(STORAGE_COLUMNS) || '[]');
      return new Set(Array.isArray(raw) ? raw : []);
    } catch {
      return new Set();
    }
  }

  function saveHiddenCols() {
    localStorage.setItem(STORAGE_COLUMNS, JSON.stringify(Array.from(state.hiddenCols)));
  }

  const RANGE_SECONDS = { '24h': 86400, '7d': 7 * 86400, '30d': 30 * 86400 };

  const fmtMoney = (cents) =>
    typeof cents === 'number' ? `$${(cents / 100).toFixed(2)}` : '--';
  const fmtEmail = (e) => (e ? e.split('@')[0] || e : '--');
  const shortId = (id) => (typeof id === 'string' && id.length > 8 ? id.substring(0, 8) : id || '--');

  function ageSeconds(offer) {
    const t = new Date(offer.created_at).getTime();
    if (!Number.isFinite(t)) return 0;
    return Math.max(0, (Date.now() - t) / 1000);
  }

  function fmtAge(seconds) {
    if (!seconds || seconds < 60) return `${Math.floor(seconds)}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    return `${Math.floor(seconds / 86400)}d`;
  }

  function fmtDateTime(s) {
    const d = new Date(s);
    return Number.isNaN(d.getTime()) ? '--' : d.toLocaleString();
  }

  function isFlagged(offer) {
    const dev = offer.price_deviation_pct;
    return typeof dev === 'number' && Math.abs(dev) > effectiveThreshold(offer);
  }

  function deviationVariant(offer) {
    const dev = offer.price_deviation_pct;
    if (typeof dev !== 'number') return 'neutral';
    const abs = Math.abs(dev);
    const t = effectiveThreshold(offer);
    if (abs > t * 4) return 'critical';
    if (abs > t) return 'warning';
    return 'success';
  }

  function statusVariant(s) {
    if (s === 'pending') return 'warning';
    if (s === 'accepted') return 'success';
    if (s === 'admin_cancelled') return 'admin-cancelled';
    if (s === 'expired') return 'expired';
    if (['cancelled', 'declined'].includes(s)) return 'danger';
    return 'neutral';
  }

  /* ───────────── filter + sort ───────────── */

  function applyFilters() {
    const f = state.filters;
    const q = f.search.trim().toLowerCase();
    const rangeSec = RANGE_SECONDS[f.range];
    let rows = state.offers.filter((o) => {
      if (f.status && o.status !== f.status) return false;
      if (f.side && (o.side || '').toLowerCase() !== f.side) return false;
      if (f.asset && o.asset_id !== f.asset) return false;
      if (f.flaggedOnly && !isFlagged(o)) return false;
      if (rangeSec && ageSeconds(o) > rangeSec) return false;
      if (q) {
        const hay = [
          o.id,
          o.asset_id,
          o.asset_name,
          o.maker_email,
          o.taker_email,
        ].filter(Boolean).join(' ').toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });

    const { key, dir } = state.sort;
    const mul = dir === 'asc' ? 1 : -1;
    rows.sort((a, b) => {
      let av = key === 'age_seconds' ? ageSeconds(a) : a[key];
      let bv = key === 'age_seconds' ? ageSeconds(b) : b[key];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      if (typeof av === 'string') return av.localeCompare(bv) * mul;
      return (av - bv) * mul;
    });
    return rows;
  }

  /* ───────────── KPI strip ───────────── */

  function setKpis(rows) {
    const total = state.offers.length;
    const pending = state.offers.filter((o) => o.status === 'pending').length;
    const flagged = state.offers.filter(isFlagged).length;
    const volume24h = state.offers
      .filter((o) => o.status === 'accepted' && ageSeconds(o) <= 86400)
      .reduce((sum, o) => sum + (o.price_cents || 0) * (o.quantity || 0), 0);

    setText('kpi-p2p-total', total);
    setText('kpi-p2p-total-sub', `${rows.length} match${rows.length === 1 ? '' : 'es'}`);
    setText('kpi-p2p-pending', pending);
    setText('kpi-p2p-flagged', flagged);
    setText('kpi-p2p-threshold', state.threshold);
    setText('kpi-p2p-volume', fmtMoney(volume24h));
  }

  function setText(id, v) {
    const el = document.getElementById(id);
    if (el) el.textContent = String(v);
  }

  /* ───────────── render table ───────────── */

  function renderEmpty(message, detail, retryable, action) {
    const body = document.getElementById('p2p-body');
    if (!body) return;
    body.replaceChildren();
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 15;
    cell.className = 'admin-table-empty';
    const title = document.createElement('div');
    title.style.fontWeight = '600';
    title.textContent = message;
    cell.appendChild(title);
    if (detail) {
      const sub = document.createElement('div');
      sub.style.marginTop = '4px';
      sub.style.color = 'var(--admin-text-muted)';
      sub.textContent = detail;
      cell.appendChild(sub);
    }
    if (retryable) {
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      retry.style.marginTop = '12px';
      retry.textContent = 'Retry';
      retry.addEventListener('click', loadP2P);
      cell.appendChild(retry);
    }
    if (action) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      btn.style.marginTop = '12px';
      btn.textContent = action.label;
      btn.addEventListener('click', action.onClick);
      cell.appendChild(btn);
    }
    row.appendChild(cell);
    body.appendChild(row);
  }

  function kycBadge(status) {
    const allowed = ['approved', 'pending', 'in_review', 'rejected', 'expired'];
    const v = allowed.includes(status) ? status : 'unknown';
    const label = v === 'approved' ? '✓ KYC' : (status || 'no KYC');
    const span = document.createElement('span');
    span.className = `mp-kyc-badge mp-kyc-badge--${v}`;
    span.textContent = label;
    span.title = `KYC status: ${status || 'unknown'}`;
    return span;
  }

  function badge(text, variant) {
    const span = document.createElement('span');
    const allowed = new Set(['neutral', 'warning', 'success', 'danger', 'critical', 'admin-cancelled', 'expired']);
    const v = allowed.has(variant) ? variant : 'neutral';
    const cls = v === 'critical' ? 'danger' : v;
    span.className = `admin-badge admin-badge--${cls}`;
    span.textContent = text;
    return span;
  }

  function buildRow(offer) {
    const row = document.createElement('tr');
    row.dataset.offerId = offer.id;
    row.className = 'mp-p2p-row';

    const age = ageSeconds(offer);
    if (offer.status === 'pending' && age > VERY_STALE_SECONDS) row.classList.add('is-very-stale');
    else if (offer.status === 'pending' && age > STALE_SECONDS) row.classList.add('is-stale');
    if (state.selected.has(offer.id)) row.classList.add('is-selected');

    // checkbox cell
    const checkCell = document.createElement('td');
    checkCell.className = 'mp-p2p-col-check';
    if (offer.status === 'pending') {
      const cb = document.createElement('input');
      cb.type = 'checkbox';
      cb.checked = state.selected.has(offer.id);
      cb.setAttribute('aria-label', `Select offer ${shortId(offer.id)}`);
      cb.addEventListener('click', (e) => e.stopPropagation());
      cb.addEventListener('change', () => toggleSelected(offer.id, cb.checked));
      checkCell.appendChild(cb);
    }
    row.appendChild(checkCell);

    appendCode(row, shortId(offer.id));
    appendText(row, (offer.side || '--').toUpperCase(), { bold: true });
    appendKycCell(row, offer.maker_email, offer.maker_kyc_status);
    appendKycCell(row, offer.taker_email, offer.taker_kyc_status);
    appendText(row, offer.asset_name || shortId(offer.asset_id), { bold: true });
    appendText(row, Number(offer.quantity || 0).toLocaleString(), { right: true, mono: true });
    appendText(row, fmtMoney(offer.price_cents), { right: true, mono: true, bold: true });
    appendText(row, fmtMoney(offer.market_price_cents), { right: true, mono: true, muted: true });

    // deviation
    const devCell = document.createElement('td');
    const dev = offer.price_deviation_pct;
    if (typeof dev !== 'number' || offer.market_price_cents == null) {
      devCell.appendChild(badge('N/A', 'neutral'));
    } else {
      const sign = dev >= 0 ? '+' : '';
      const variant = deviationVariant(offer);
      if (variant === 'critical') {
        const w = document.createElement('span');
        w.className = 'mp-price-warning';
        w.textContent = `${sign}${dev.toFixed(2)}%`;
        devCell.appendChild(w);
      } else {
        devCell.appendChild(badge(`${sign}${dev.toFixed(2)}%`, variant));
      }
      if (isFlagged(offer)) {
        const chip = document.createElement('span');
        chip.className = 'mp-anomaly-chip';
        chip.textContent = '⚑ Anomaly';
        chip.title = `Exceeds threshold ±${effectiveThreshold(offer)}%`;
        devCell.appendChild(chip);
      }
    }
    row.appendChild(devCell);

    // status
    const statusCell = document.createElement('td');
    statusCell.appendChild(badge(offer.status || '--', statusVariant(offer.status)));
    row.appendChild(statusCell);

    // age
    const ageCell = appendText(row, fmtAge(age), { mono: true });
    ageCell.classList.add('mp-p2p-age');

    appendText(row, fmtDateTime(offer.expires_at), { muted: true, small: true });
    appendText(row, fmtDateTime(offer.created_at), { muted: true, small: true });

    // action
    const actionCell = document.createElement('td');
    actionCell.style.textAlign = 'center';
    if (offer.status === 'pending') {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'admin-btn admin-btn--danger admin-btn--sm btn-cancel-p2p';
      btn.textContent = 'Admin Cancel';
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        openCancelModal(offer, btn);
      });
      actionCell.appendChild(btn);
    } else {
      actionCell.textContent = '--';
      actionCell.style.color = 'var(--admin-text-muted)';
    }
    row.appendChild(actionCell);

    row.addEventListener('click', () => openDrawer(offer));
    return row;
  }

  function appendText(row, text, opt = {}) {
    const cell = document.createElement('td');
    if (opt.right) cell.style.textAlign = 'right';
    if (opt.muted) cell.style.color = 'var(--admin-text-muted)';
    if (opt.bold) cell.style.fontWeight = '600';
    if (opt.mono) cell.style.fontVariantNumeric = 'tabular-nums';
    if (opt.small) cell.style.fontSize = '12px';
    cell.textContent = text;
    row.appendChild(cell);
    return cell;
  }

  function appendKycCell(row, email, kycStatus) {
    const cell = document.createElement('td');
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = fmtEmail(email);
    cell.appendChild(code);
    cell.appendChild(kycBadge(kycStatus));
    row.appendChild(cell);
    return cell;
  }

  function appendCode(row, text) {
    const cell = document.createElement('td');
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = text;
    cell.appendChild(code);
    row.appendChild(cell);
    return cell;
  }

  function render() {
    const body = document.getElementById('p2p-body');
    if (!body) return;
    const rows = applyFilters();
    setKpis(rows);
    setText('p2p-rowcount', `${rows.length} of ${state.offers.length} offer${state.offers.length === 1 ? '' : 's'}`);

    if (!rows.length) {
      const filtersActive = !!(state.filters.search || state.filters.status || state.filters.side || state.filters.asset || state.filters.flaggedOnly || state.filters.range);
      renderEmpty(
        filtersActive ? 'No matching offers' : 'No P2P offers found',
        filtersActive ? 'Try clearing filters or expanding the threshold.' : 'Pending, accepted, and cancelled P2P offers will appear here.',
        false,
        filtersActive ? { label: 'Clear filters', onClick: clearFilters } : null,
      );
    } else {
      body.replaceChildren();
      rows.forEach((o) => body.appendChild(buildRow(o)));
    }

    // sync sort indicators
    document.querySelectorAll('#p2p-table th[data-sort]').forEach((th) => {
      const k = th.dataset.sort;
      th.setAttribute('aria-sort', k === state.sort.key ? (state.sort.dir === 'asc' ? 'ascending' : 'descending') : 'none');
    });

    syncBulkbar();
    syncCheckAll();
    applyColumnVisibility();
    highlightSearchHits();
  }

  function highlightSearchHits() {
    const q = (state.filters.search || '').trim();
    if (!q) return;
    const tbody = document.getElementById('p2p-body');
    if (!tbody) return;
    const re = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    tbody.querySelectorAll('td code').forEach((node) => {
      const t = node.textContent;
      if (!t || !re.test(t)) return;
      node.innerHTML = t.replace(re, '<mark class="mp-p2p-hit">$1</mark>');
    });
  }

  /* ───────────── selection / bulk ───────────── */

  function toggleSelected(id, on) {
    if (on) state.selected.add(id);
    else state.selected.delete(id);
    render();
  }

  function syncBulkbar() {
    const bar = document.getElementById('p2p-bulkbar');
    const count = state.selected.size;
    if (!bar) return;
    if (count === 0) {
      bar.hidden = true;
    } else {
      bar.hidden = false;
      setText('p2p-bulkbar-count', `${count} selected`);
    }
  }

  function syncCheckAll() {
    const cb = document.getElementById('p2p-check-all');
    if (!cb) return;
    const visiblePending = applyFilters().filter((o) => o.status === 'pending');
    const allOn = visiblePending.length > 0 && visiblePending.every((o) => state.selected.has(o.id));
    cb.checked = allOn;
    cb.indeterminate = !allOn && visiblePending.some((o) => state.selected.has(o.id));
  }

  async function bulkCancel() {
    if (!state.selected.size) return;
    const reason = window.prompt(`Cancellation reason for ${state.selected.size} offer${state.selected.size === 1 ? '' : 's'} (required, max 500 chars):`);
    if (!reason || !reason.trim()) return;
    const ids = Array.from(state.selected);
    let ok = 0;
    let fail = 0;
    for (const id of ids) {
      try {
        const res = await fetch(`${API}/${encodeURIComponent(id)}/cancel`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken() },
          body: JSON.stringify({ reason: reason.trim() }),
        });
        if (res.ok) ok += 1;
        else fail += 1;
      } catch {
        fail += 1;
      }
    }
    state.selected.clear();
    if (typeof mpToast === 'function') {
      mpToast(`Bulk cancel: ${ok} succeeded, ${fail} failed`, fail ? 'error' : 'success');
    }
    await loadP2P();
  }

  /* ───────────── cancel modal ───────────── */

  function buildCancelBody() {
    const wrap = document.createElement('div');
    wrap.className = 'admin-form-group';
    const label = document.createElement('label');
    label.className = 'admin-form-label';
    label.setAttribute('for', 'p2p-cancel-reason');
    label.textContent = 'Cancellation Reason *';
    const ta = document.createElement('textarea');
    ta.className = 'admin-textarea';
    ta.id = 'p2p-cancel-reason';
    ta.maxLength = 500;
    ta.placeholder = 'e.g. Price significantly deviates from market';
    ta.setAttribute('aria-describedby', 'p2p-cancel-error');
    const err = document.createElement('div');
    err.id = 'p2p-cancel-error';
    err.style.marginTop = '6px';
    err.style.color = 'var(--admin-danger)';
    err.style.fontSize = '12px';
    err.setAttribute('role', 'alert');
    wrap.append(label, ta, err);
    return wrap;
  }

  function openCancelModal(offer, triggerButton) {
    if (typeof mpModal !== 'function') return;
    mpModal({
      title: 'Cancel P2P Offer',
      subtitle: `${shortId(offer.id)} - ${offer.asset_name || shortId(offer.asset_id)} @ ${fmtMoney(offer.price_cents)}`,
      bodyNode: buildCancelBody(),
      confirmLabel: 'Cancel Offer',
      onConfirm: async (overlay) => {
        const ta = overlay.querySelector('#p2p-cancel-reason');
        const err = overlay.querySelector('#p2p-cancel-error');
        const reason = ta?.value?.trim() || '';
        if (!reason) {
          if (err) err.textContent = 'Please provide a cancellation reason.';
          ta?.focus();
          return false;
        }
        if (triggerButton) {
          triggerButton.disabled = true;
          triggerButton.setAttribute('aria-busy', 'true');
        }
        try {
          const res = await fetch(`${API}/${encodeURIComponent(offer.id)}/cancel`, {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken() },
            body: JSON.stringify({ reason }),
          });
          const payload = await res.json().catch(() => ({}));
          if (!res.ok) throw new Error(payload.error || `HTTP ${res.status}`);
          if (typeof mpToast === 'function') mpToast(`P2P offer ${shortId(offer.id)} cancelled`, 'success');
          await loadP2P();
          return true;
        } catch (e) {
          if (triggerButton) {
            triggerButton.disabled = false;
            triggerButton.removeAttribute('aria-busy');
          }
          if (err) err.textContent = e.message || 'Cancellation failed.';
          if (typeof mpToast === 'function') mpToast(e.message || 'Cancellation failed', 'error');
          return false;
        }
      },
    });
  }

  /* ───────────── detail drawer ───────────── */

  function openDrawer(offer) {
    const drawer = document.getElementById('p2p-drawer');
    const body = document.getElementById('p2p-drawer-body');
    const title = document.getElementById('p2p-drawer-title');
    if (!drawer || !body || !title) return;

    title.textContent = `Offer ${shortId(offer.id)}`;
    body.replaceChildren();

    const grid = document.createElement('dl');
    grid.className = 'mp-p2p-detail-grid';
    const fields = [
      ['Offer ID', offer.id],
      ['Asset', `${offer.asset_name || '--'} (${shortId(offer.asset_id)})`],
      ['Side', (offer.side || '--').toUpperCase()],
      ['Status', offer.status || '--'],
      ['Maker', offer.maker_email || '--'],
      ['Taker', offer.taker_email || '--'],
      ['Quantity', Number(offer.quantity || 0).toLocaleString()],
      ['Offer price', fmtMoney(offer.price_cents)],
      ['Market price', fmtMoney(offer.market_price_cents)],
      ['Deviation', typeof offer.price_deviation_pct === 'number' ? `${offer.price_deviation_pct.toFixed(2)}%` : '--'],
      ['Notional', fmtMoney(offer.total_value_cents)],
      ['Age', fmtAge(ageSeconds(offer))],
      ['Created', fmtDateTime(offer.created_at)],
      ['Expires', fmtDateTime(offer.expires_at)],
    ];
    fields.forEach(([k, v]) => {
      const dt = document.createElement('dt');
      dt.textContent = k;
      const dd = document.createElement('dd');
      dd.textContent = v == null ? '--' : String(v);
      grid.append(dt, dd);
    });
    body.appendChild(grid);

    if (isFlagged(offer)) {
      const note = document.createElement('div');
      note.className = 'mp-p2p-detail-section';
      note.innerHTML = '<h3>Anomaly</h3>';
      const p = document.createElement('p');
      p.style.margin = '0';
      p.style.color = 'var(--admin-warning)';
      p.textContent = `Deviation exceeds threshold of ±${state.threshold}%. Review counterparty KYC before approving.`;
      note.appendChild(p);
      body.appendChild(note);
    }

    // Action footer
    const oldFooter = drawer.querySelector('.mp-p2p-drawer-actions');
    if (oldFooter) oldFooter.remove();
    const footer = document.createElement('div');
    footer.className = 'mp-p2p-drawer-actions';
    if (offer.status === 'pending') {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'admin-btn admin-btn--danger admin-btn--sm';
      btn.textContent = 'Admin Cancel';
      btn.addEventListener('click', () => openCancelModal(offer, btn));
      footer.appendChild(btn);
    }
    const auditLink = document.createElement('a');
    auditLink.href = `/admin/audit-logs?entity_type=p2p_offer&entity_id=${encodeURIComponent(offer.id)}`;
    auditLink.className = 'admin-btn admin-btn--secondary admin-btn--sm';
    auditLink.textContent = 'Audit trail';
    auditLink.target = '_blank';
    auditLink.rel = 'noopener';
    footer.appendChild(auditLink);
    drawer.appendChild(footer);

    drawer.hidden = false;
    requestAnimationFrame(() => drawer.classList.add('is-open'));
  }

  function closeDrawer() {
    const drawer = document.getElementById('p2p-drawer');
    if (!drawer) return;
    drawer.classList.remove('is-open');
    setTimeout(() => { drawer.hidden = true; }, 200);
  }

  /* ───────────── CSV export ───────────── */

  function exportCsv() {
    const rows = applyFilters();
    const headers = ['offer_id', 'side', 'maker_email', 'taker_email', 'asset_id', 'asset_name', 'quantity', 'price_cents', 'market_price_cents', 'deviation_pct', 'status', 'age_seconds', 'created_at', 'expires_at'];
    const escape = (v) => {
      if (v == null) return '';
      const s = String(v);
      return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const lines = [headers.join(',')];
    rows.forEach((o) => {
      lines.push([
        o.id,
        o.side,
        o.maker_email,
        o.taker_email,
        o.asset_id,
        o.asset_name,
        o.quantity,
        o.price_cents,
        o.market_price_cents,
        o.price_deviation_pct,
        o.status,
        Math.round(ageSeconds(o)),
        o.created_at,
        o.expires_at,
      ].map(escape).join(','));
    });
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const ts = new Date().toISOString().replace(/[:.]/g, '-');
    a.download = `p2p-offers-${ts}.csv`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  /* ───────────── load + auto-refresh ───────────── */

  function setLastUpdated() {
    state.lastUpdated = new Date();
    setText('p2p-last-updated', state.lastUpdated.toLocaleTimeString());
  }

  function populateAssetFilter() {
    const sel = document.getElementById('filter-asset');
    if (!sel) return;
    const seen = new Map();
    state.offers.forEach((o) => {
      if (o.asset_id && !seen.has(o.asset_id)) seen.set(o.asset_id, o.asset_name || o.asset_id);
    });
    const current = sel.value;
    sel.replaceChildren();
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = 'All assets';
    sel.appendChild(opt);
    Array.from(seen.entries())
      .sort((a, b) => String(a[1]).localeCompare(String(b[1])))
      .forEach(([id, name]) => {
        const o = document.createElement('option');
        o.value = id;
        o.textContent = name;
        sel.appendChild(o);
      });
    sel.value = seen.has(current) ? current : '';
  }

  async function loadP2P() {
    try {
      const params = new URLSearchParams({ page: String(state.page), page_size: String(state.pageSize) });
      const res = await fetch(`${API}?${params}`, { credentials: 'same-origin' });
      const payload = await res.json().catch(() => null);
      if (!res.ok) throw new Error(payload?.error || `HTTP ${res.status}`);
      // Accept envelope or legacy array
      let items;
      let total;
      if (Array.isArray(payload)) { items = payload; total = payload.length; }
      else if (payload && Array.isArray(payload.items)) { items = payload.items; total = payload.total ?? items.length; }
      else throw new Error('Unexpected P2P response format.');
      state.offers = items;
      state.total = total;
      const live = new Set(items.map((o) => o.id));
      Array.from(state.selected).forEach((id) => { if (!live.has(id)) state.selected.delete(id); });
      populateAssetFilter();
      setLastUpdated();
      render();
      renderPagination();
    } catch (e) {
      state.offers = [];
      state.total = 0;
      render();
      renderEmpty('Unable to load P2P offers', e.message || 'Please try again.', true);
    }
  }

  function renderPagination() {
    const el = document.getElementById('p2p-pagination');
    if (!el) return;
    el.replaceChildren();
    const totalPages = Math.max(1, Math.ceil(state.total / state.pageSize));
    if (state.total <= state.pageSize) { el.style.display = 'none'; return; }
    el.style.display = '';

    const sizeWrap = document.createElement('span');
    sizeWrap.className = 'mp-pagination-info';
    sizeWrap.textContent = 'Page size:';
    const sizeSel = document.createElement('select');
    sizeSel.className = 'admin-select admin-select--sm';
    [10, 25, 50, 100, 200].forEach((n) => {
      const opt = document.createElement('option');
      opt.value = String(n);
      opt.textContent = String(n);
      if (n === state.pageSize) opt.selected = true;
      sizeSel.appendChild(opt);
    });
    sizeSel.addEventListener('change', () => {
      state.pageSize = parseInt(sizeSel.value, 10);
      savePageSize(state.pageSize);
      state.page = 1;
      loadP2P();
    });
    sizeWrap.appendChild(sizeSel);

    const prev = document.createElement('button');
    prev.className = 'mp-pagination-btn';
    prev.textContent = '◀ Prev';
    prev.disabled = state.page <= 1;
    prev.addEventListener('click', () => { state.page = Math.max(1, state.page - 1); loadP2P(); });

    const info = document.createElement('span');
    info.className = 'mp-pagination-info';
    const start = (state.page - 1) * state.pageSize + 1;
    const end = Math.min(state.total, state.page * state.pageSize);
    info.textContent = `${start}–${end} of ${state.total} (page ${state.page}/${totalPages})`;

    const next = document.createElement('button');
    next.className = 'mp-pagination-btn';
    next.textContent = 'Next ▶';
    next.disabled = state.page >= totalPages;
    next.addEventListener('click', () => { state.page = Math.min(totalPages, state.page + 1); loadP2P(); });

    el.append(sizeWrap, prev, info, next);
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    if (!state.autoRefresh) return;
    state.refreshTimer = setInterval(loadP2P, REFRESH_MS);
  }
  function stopAutoRefresh() {
    if (state.refreshTimer) {
      clearInterval(state.refreshTimer);
      state.refreshTimer = null;
    }
  }

  /* ───────────── wiring ───────────── */

  function wireEvents() {
    // Search
    const search = document.getElementById('p2p-search');
    search?.addEventListener('input', (e) => {
      state.filters.search = e.target.value;
      render();
    });

    // Selects
    const status = document.getElementById('filter-status');
    status?.addEventListener('change', (e) => {
      state.filters.status = e.target.value;
      syncKpiPressed();
      render();
    });
    document.getElementById('filter-side')?.addEventListener('change', (e) => {
      state.filters.side = e.target.value;
      render();
    });
    document.getElementById('filter-asset')?.addEventListener('change', (e) => {
      state.filters.asset = e.target.value;
      render();
    });
    document.getElementById('filter-range')?.addEventListener('change', (e) => {
      state.filters.range = e.target.value;
      render();
    });
    const flagged = document.getElementById('filter-flagged');
    flagged?.addEventListener('change', (e) => {
      state.filters.flaggedOnly = e.target.checked;
      syncKpiPressed();
      render();
    });

    // KPI cards as filters
    document.getElementById('kpi-card-pending')?.addEventListener('click', () => {
      const next = state.filters.status === 'pending' ? '' : 'pending';
      state.filters.status = next;
      if (status) status.value = next;
      syncKpiPressed();
      render();
    });
    document.getElementById('kpi-card-flagged')?.addEventListener('click', () => {
      state.filters.flaggedOnly = !state.filters.flaggedOnly;
      if (flagged) flagged.checked = state.filters.flaggedOnly;
      syncKpiPressed();
      render();
    });

    // Threshold
    const threshold = document.getElementById('threshold-input');
    if (threshold) {
      threshold.value = state.threshold;
      threshold.addEventListener('change', (e) => {
        const v = parseFloat(e.target.value);
        if (Number.isFinite(v) && v >= 0) {
          state.threshold = v;
          saveThreshold(v);
          render();
        }
      });
    }

    // Sort
    document.querySelectorAll('#p2p-table th[data-sort]').forEach((th) => {
      th.addEventListener('click', () => {
        const k = th.dataset.sort;
        if (state.sort.key === k) {
          state.sort.dir = state.sort.dir === 'asc' ? 'desc' : 'asc';
        } else {
          state.sort.key = k;
          state.sort.dir = 'asc';
        }
        render();
      });
    });

    // Check-all
    document.getElementById('p2p-check-all')?.addEventListener('change', (e) => {
      const visible = applyFilters().filter((o) => o.status === 'pending');
      if (e.target.checked) visible.forEach((o) => state.selected.add(o.id));
      else visible.forEach((o) => state.selected.delete(o.id));
      render();
    });

    // Bulk
    document.getElementById('btn-bulk-cancel')?.addEventListener('click', bulkCancel);
    document.getElementById('btn-bulk-clear')?.addEventListener('click', () => {
      state.selected.clear();
      render();
    });

    // CSV
    document.getElementById('btn-export-csv')?.addEventListener('click', exportCsv);

    // Refresh + auto
    document.getElementById('btn-refresh-p2p')?.addEventListener('click', loadP2P);
    const auto = document.getElementById('p2p-auto-refresh');
    if (auto) {
      auto.checked = state.autoRefresh;
      auto.addEventListener('change', (e) => {
        state.autoRefresh = e.target.checked;
        if (state.autoRefresh) startAutoRefresh();
        else stopAutoRefresh();
      });
    }

    // Drawer
    document.getElementById('btn-drawer-close')?.addEventListener('click', closeDrawer);
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') closeDrawer();
    });

    // Pause auto-refresh while tab hidden
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) stopAutoRefresh();
      else if (state.autoRefresh) startAutoRefresh();
    });
  }

  function syncKpiPressed() {
    const pendingCard = document.getElementById('kpi-card-pending');
    if (pendingCard) pendingCard.setAttribute('aria-pressed', state.filters.status === 'pending' ? 'true' : 'false');
    const flagCard = document.getElementById('kpi-card-flagged');
    if (flagCard) flagCard.setAttribute('aria-pressed', state.filters.flaggedOnly ? 'true' : 'false');
  }

  /* ───────────── clear filters ───────────── */

  function clearFilters() {
    state.filters = { search: '', status: '', side: '', asset: '', flaggedOnly: false, range: '' };
    const ids = ['p2p-search', 'filter-status', 'filter-side', 'filter-asset', 'filter-range'];
    ids.forEach((id) => { const el = document.getElementById(id); if (el) el.value = ''; });
    const flagged = document.getElementById('filter-flagged');
    if (flagged) flagged.checked = false;
    syncKpiPressed();
    render();
  }

  /* ───────────── column visibility ───────────── */

  function applyColumnVisibility() {
    const table = document.getElementById('p2p-table');
    if (!table) return;
    COLUMNS.forEach((c) => {
      const hidden = state.hiddenCols.has(c.key);
      table.querySelectorAll(`[data-col="${c.key}"]`).forEach((cell) => {
        cell.style.display = hidden ? 'none' : '';
      });
    });
  }

  function populateColumnsMenu() {
    const panel = document.getElementById('p2p-cols-panel');
    if (!panel) return;
    panel.replaceChildren();
    COLUMNS.forEach((c) => {
      if (c.sticky) return;
      const label = document.createElement('label');
      label.className = 'mp-p2p-cols-menu__item';
      const cb = document.createElement('input');
      cb.type = 'checkbox';
      cb.checked = !state.hiddenCols.has(c.key);
      cb.addEventListener('change', () => {
        if (cb.checked) state.hiddenCols.delete(c.key);
        else state.hiddenCols.add(c.key);
        saveHiddenCols();
        applyColumnVisibility();
      });
      const txt = document.createElement('span');
      txt.textContent = c.label;
      label.append(cb, txt);
      panel.appendChild(label);
    });
  }

  function setupColumnsMenu() {
    const menu = document.getElementById('p2p-cols-menu');
    const btn = document.getElementById('btn-cols-toggle');
    if (!menu || !btn) return;
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const open = menu.classList.toggle('is-open');
      btn.setAttribute('aria-expanded', open ? 'true' : 'false');
    });
    document.addEventListener('click', (e) => {
      if (!menu.contains(e.target)) {
        menu.classList.remove('is-open');
        btn.setAttribute('aria-expanded', 'false');
      }
    });
  }

  /* ───────────── scroll shadow ───────────── */

  function setupScrollShadow() {
    const wrap = document.getElementById('p2p-table-wrap');
    if (!wrap) return;
    const update = () => {
      wrap.classList.toggle('is-scrolled', wrap.scrollLeft > 4);
    };
    wrap.addEventListener('scroll', update, { passive: true });
    update();
  }

  /* ───────────── per-asset thresholds dialog ───────────── */

  function openThresholdsPanel() {
    const panel = document.getElementById('p2p-thresholds-panel');
    const list = document.getElementById('p2p-thresholds-list');
    if (!panel || !list) return;
    list.replaceChildren();

    const seen = new Map();
    state.offers.forEach((o) => {
      if (o.asset_id && !seen.has(o.asset_id)) seen.set(o.asset_id, o.asset_name || o.asset_id);
    });
    if (seen.size === 0) {
      const empty = document.createElement('div');
      empty.style.color = 'var(--admin-text-muted)';
      empty.style.fontSize = '13px';
      empty.textContent = 'No assets in current page. Override applies once asset visible.';
      list.appendChild(empty);
    }
    seen.forEach((name, id) => {
      const row = document.createElement('div');
      row.className = 'mp-p2p-thresholds__row';
      const label = document.createElement('div');
      label.textContent = name;
      label.title = id;
      const inp = document.createElement('input');
      inp.type = 'number';
      inp.className = 'admin-input';
      inp.min = '0';
      inp.max = '100';
      inp.step = '0.5';
      inp.placeholder = String(state.threshold);
      const cur = state.assetThresholds[id];
      if (Number.isFinite(cur)) inp.value = String(cur);
      inp.addEventListener('change', () => {
        const v = parseFloat(inp.value);
        if (Number.isFinite(v) && v >= 0) state.assetThresholds[id] = v;
        else delete state.assetThresholds[id];
        saveAssetThresholds();
        render();
      });
      const clear = document.createElement('button');
      clear.type = 'button';
      clear.className = 'admin-btn admin-btn--ghost admin-btn--sm';
      clear.textContent = '×';
      clear.title = 'Reset to global';
      clear.addEventListener('click', () => {
        delete state.assetThresholds[id];
        saveAssetThresholds();
        inp.value = '';
        render();
      });
      row.append(label, inp, clear);
      list.appendChild(row);
    });
    panel.hidden = false;
  }

  function closeThresholdsPanel() {
    const panel = document.getElementById('p2p-thresholds-panel');
    if (panel) panel.hidden = true;
  }

  /* ───────────── command palette ───────────── */

  function paletteCommands() {
    return [
      { id: 'refresh', label: 'Refresh now', hint: 'R', run: loadP2P },
      { id: 'csv', label: 'Export CSV', hint: 'X', run: exportCsv },
      { id: 'clear', label: 'Clear filters', hint: 'C', run: clearFilters },
      { id: 'flag', label: 'Toggle flagged-only', run: () => {
        state.filters.flaggedOnly = !state.filters.flaggedOnly;
        const cb = document.getElementById('filter-flagged');
        if (cb) cb.checked = state.filters.flaggedOnly;
        syncKpiPressed(); render();
      } },
      { id: 'pending', label: 'Filter: pending', run: () => {
        state.filters.status = state.filters.status === 'pending' ? '' : 'pending';
        const sel = document.getElementById('filter-status');
        if (sel) sel.value = state.filters.status;
        syncKpiPressed(); render();
      } },
      { id: 'auto', label: 'Toggle auto-refresh', run: () => {
        state.autoRefresh = !state.autoRefresh;
        const cb = document.getElementById('p2p-auto-refresh');
        if (cb) cb.checked = state.autoRefresh;
        if (state.autoRefresh) startAutoRefresh(); else stopAutoRefresh();
      } },
      { id: 'thresholds', label: 'Per-asset thresholds…', run: openThresholdsPanel },
      { id: 'orderbook', label: 'Go to Orderbook', run: () => { location.href = '/admin/marketplace/orderbook'; } },
      { id: 'orders', label: 'Go to Open Orders', run: () => { location.href = '/admin/marketplace/orders'; } },
      { id: 'trades', label: 'Go to Trades', run: () => { location.href = '/admin/marketplace/trades'; } },
      ...viewCommands(),
    ];
  }

  function openPalette() {
    const root = document.getElementById('p2p-palette');
    const input = document.getElementById('p2p-palette-input');
    if (!root || !input) return;
    root.hidden = false;
    input.value = '';
    renderPaletteList('');
    input.focus();
  }
  function closePalette() {
    const root = document.getElementById('p2p-palette');
    if (root) root.hidden = true;
  }
  function renderPaletteList(query) {
    const list = document.getElementById('p2p-palette-list');
    if (!list) return;
    list.replaceChildren();
    const q = query.trim().toLowerCase();
    const items = paletteCommands().filter((c) => !q || c.label.toLowerCase().includes(q));
    items.forEach((c, idx) => {
      const li = document.createElement('li');
      li.dataset.cmdId = c.id;
      li.setAttribute('role', 'option');
      if (idx === 0) li.classList.add('is-active');
      const label = document.createElement('span');
      label.textContent = c.label;
      const hint = document.createElement('small');
      hint.textContent = c.hint || '';
      li.append(label, hint);
      li.addEventListener('click', () => { closePalette(); c.run(); });
      list.appendChild(li);
    });
  }
  function paletteMove(delta) {
    const list = document.getElementById('p2p-palette-list');
    if (!list) return;
    const items = Array.from(list.querySelectorAll('li'));
    if (!items.length) return;
    const cur = items.findIndex((li) => li.classList.contains('is-active'));
    const next = (cur + delta + items.length) % items.length;
    items.forEach((li, i) => li.classList.toggle('is-active', i === next));
    items[next].scrollIntoView({ block: 'nearest' });
  }
  function paletteRunActive() {
    const list = document.getElementById('p2p-palette-list');
    const active = list?.querySelector('li.is-active');
    if (!active) return;
    const cmd = paletteCommands().find((c) => c.id === active.dataset.cmdId);
    if (cmd) { closePalette(); cmd.run(); }
  }
  function setupPalette() {
    const input = document.getElementById('p2p-palette-input');
    document.getElementById('btn-palette')?.addEventListener('click', openPalette);
    document.getElementById('btn-thresholds-close')?.addEventListener('click', closeThresholdsPanel);
    document.getElementById('btn-per-asset-thresholds')?.addEventListener('click', openThresholdsPanel);
    input?.addEventListener('input', () => renderPaletteList(input.value));
    input?.addEventListener('keydown', (e) => {
      if (e.key === 'ArrowDown') { e.preventDefault(); paletteMove(1); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); paletteMove(-1); }
      else if (e.key === 'Enter') { e.preventDefault(); paletteRunActive(); }
      else if (e.key === 'Escape') { e.preventDefault(); closePalette(); }
    });
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        const root = document.getElementById('p2p-palette');
        if (root && root.hidden) openPalette(); else closePalette();
      } else if (e.key === '/' && !['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) {
        e.preventDefault();
        document.getElementById('p2p-search')?.focus();
      } else if (e.key === 'Escape') {
        closePalette();
        closeThresholdsPanel();
      }
    });
    // backdrop click to close palette
    document.getElementById('p2p-palette')?.addEventListener('click', (e) => {
      if (e.target === e.currentTarget) closePalette();
    });
  }

  /* ───────────── demo data ───────────── */

  function isDevHost() {
    const h = location.hostname;
    return h === 'localhost' || h === '127.0.0.1' || h.endsWith('.local') || location.search.includes('demo=1');
  }
  const DEMO_OFFERS = [
    { id: 'demo-0001-pending-stale', asset_id: 'demo-asset-1', asset_name: 'Demo Tower NYC', maker_email: 'alice@demo.app', taker_email: 'bob@demo.app', side: 'sell', price_cents: 12500, quantity: 250, total_value_cents: 12500 * 250, status: 'pending', market_price_cents: 12000, price_deviation_pct: 4.17, maker_kyc_status: 'approved', taker_kyc_status: 'pending', created_at: new Date(Date.now() - 26*3600*1000).toISOString(), expires_at: new Date(Date.now() + 22*3600*1000).toISOString() },
    { id: 'demo-0002-anomaly', asset_id: 'demo-asset-2', asset_name: 'Demo Vineyard', maker_email: 'mallory@demo.app', taker_email: 'eve@demo.app', side: 'buy', price_cents: 7000, quantity: 80, total_value_cents: 7000*80, status: 'pending', market_price_cents: 10000, price_deviation_pct: -30, maker_kyc_status: 'rejected', taker_kyc_status: 'approved', created_at: new Date(Date.now() - 5*60*1000).toISOString(), expires_at: new Date(Date.now() + 86400000).toISOString() },
    { id: 'demo-0003-accepted', asset_id: 'demo-asset-1', asset_name: 'Demo Tower NYC', maker_email: 'carol@demo.app', taker_email: 'dan@demo.app', side: 'sell', price_cents: 12100, quantity: 100, total_value_cents: 12100*100, status: 'accepted', market_price_cents: 12000, price_deviation_pct: 0.83, maker_kyc_status: 'approved', taker_kyc_status: 'approved', created_at: new Date(Date.now() - 3*3600*1000).toISOString(), expires_at: new Date(Date.now() + 86400000).toISOString() },
    { id: 'demo-0004-cancelled', asset_id: 'demo-asset-3', asset_name: 'Demo Solar Farm', maker_email: 'frank@demo.app', taker_email: 'gail@demo.app', side: 'buy', price_cents: 4400, quantity: 1500, total_value_cents: 4400*1500, status: 'admin_cancelled', market_price_cents: 4500, price_deviation_pct: -2.22, maker_kyc_status: 'in_review', taker_kyc_status: 'approved', created_at: new Date(Date.now() - 75*3600*1000).toISOString(), expires_at: new Date(Date.now() - 24*3600*1000).toISOString() },
    { id: 'demo-0005-expired', asset_id: 'demo-asset-2', asset_name: 'Demo Vineyard', maker_email: 'helen@demo.app', taker_email: 'ivan@demo.app', side: 'sell', price_cents: 9800, quantity: 25, total_value_cents: 9800*25, status: 'expired', market_price_cents: 10000, price_deviation_pct: -2, maker_kyc_status: 'expired', taker_kyc_status: null, created_at: new Date(Date.now() - 9*86400*1000).toISOString(), expires_at: new Date(Date.now() - 86400*1000).toISOString() },
  ];
  let demoMode = false;
  function setupDemoToggle() {
    const btn = document.getElementById('btn-demo-data');
    if (!btn) return;
    if (!isDevHost()) return;
    btn.hidden = false;
    btn.addEventListener('click', () => {
      demoMode = !demoMode;
      btn.classList.toggle('admin-btn--primary', demoMode);
      if (demoMode) {
        state.offers = DEMO_OFFERS.slice();
        state.total = state.offers.length;
        populateAssetFilter();
        setLastUpdated();
        render();
        renderPagination();
        if (typeof mpToast === 'function') mpToast('Demo data loaded (5 fixture offers)', 'info');
      } else {
        loadP2P();
      }
    });
  }

  /* ───────────── saved view presets ───────────── */

  const STORAGE_VIEWS = 'p2p.views';
  function loadViews() {
    try { return JSON.parse(localStorage.getItem(STORAGE_VIEWS) || '{}') || {}; }
    catch { return {}; }
  }
  function saveViews(v) { localStorage.setItem(STORAGE_VIEWS, JSON.stringify(v)); }

  function snapshotView() {
    return {
      filters: { ...state.filters },
      sort: { ...state.sort },
      threshold: state.threshold,
      hiddenCols: Array.from(state.hiddenCols),
    };
  }
  function applyView(v) {
    state.filters = { search: '', status: '', side: '', asset: '', flaggedOnly: false, range: '', ...(v.filters || {}) };
    state.sort = { ...state.sort, ...(v.sort || {}) };
    if (Number.isFinite(v.threshold)) {
      state.threshold = v.threshold;
      saveThreshold(v.threshold);
      const inp = document.getElementById('threshold-input');
      if (inp) inp.value = v.threshold;
    }
    if (Array.isArray(v.hiddenCols)) {
      state.hiddenCols = new Set(v.hiddenCols);
      saveHiddenCols();
      populateColumnsMenu();
    }
    // sync inputs
    const map = { 'p2p-search': 'search', 'filter-status': 'status', 'filter-side': 'side', 'filter-asset': 'asset', 'filter-range': 'range' };
    Object.entries(map).forEach(([id, k]) => { const el = document.getElementById(id); if (el) el.value = state.filters[k] || ''; });
    const flagged = document.getElementById('filter-flagged');
    if (flagged) flagged.checked = !!state.filters.flaggedOnly;
    syncKpiPressed();
    render();
  }

  function viewCommands() {
    const v = loadViews();
    const cmds = [
      { id: 'view-save', label: 'Save current view…', run: () => {
        const name = prompt('View name:');
        if (!name) return;
        const all = loadViews();
        all[name] = snapshotView();
        saveViews(all);
        if (typeof mpToast === 'function') mpToast(`View "${name}" saved`, 'success');
      } },
    ];
    Object.keys(v).forEach((name) => {
      cmds.push({ id: `view-load-${name}`, label: `View: ${name}`, hint: 'load', run: () => applyView(v[name]) });
      cmds.push({ id: `view-del-${name}`, label: `Delete view: ${name}`, run: () => {
        const all = loadViews(); delete all[name]; saveViews(all);
        if (typeof mpToast === 'function') mpToast(`View "${name}" deleted`, 'info');
      } });
    });
    return cmds;
  }

  /* ───────────── resizable columns ───────────── */

  const STORAGE_COL_WIDTHS = 'p2p.col_widths';
  function loadColWidths() {
    try { return JSON.parse(localStorage.getItem(STORAGE_COL_WIDTHS) || '{}') || {}; }
    catch { return {}; }
  }
  function saveColWidths(w) { localStorage.setItem(STORAGE_COL_WIDTHS, JSON.stringify(w)); }
  function applyColWidths() {
    const widths = loadColWidths();
    Object.entries(widths).forEach(([col, w]) => {
      document.querySelectorAll(`#p2p-table [data-col="${col}"]`).forEach((cell) => {
        cell.style.width = `${w}px`;
        cell.style.minWidth = `${w}px`;
      });
    });
  }
  function setupResizable() {
    const ths = document.querySelectorAll('#p2p-table thead th[data-col]');
    ths.forEach((th) => {
      if (th.querySelector('.mp-p2p-col-grip')) return;
      const grip = document.createElement('span');
      grip.className = 'mp-p2p-col-grip';
      grip.setAttribute('aria-hidden', 'true');
      th.style.position = 'relative';
      th.appendChild(grip);
      grip.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();
        const startX = e.clientX;
        const startW = th.getBoundingClientRect().width;
        const col = th.dataset.col;
        const onMove = (ev) => {
          const w = Math.max(60, Math.round(startW + (ev.clientX - startX)));
          th.style.width = `${w}px`;
          th.style.minWidth = `${w}px`;
        };
        const onUp = () => {
          document.removeEventListener('mousemove', onMove);
          document.removeEventListener('mouseup', onUp);
          const w = parseInt(th.style.width, 10);
          if (Number.isFinite(w)) {
            const all = loadColWidths();
            all[col] = w;
            saveColWidths(all);
            applyColWidths();
          }
        };
        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onUp);
      });
    });
    applyColWidths();
  }

  /* ───────────── keyboard nav (j/k/x/Enter) ───────────── */

  let keyboardCursor = -1;
  function moveCursor(delta) {
    const rows = Array.from(document.querySelectorAll('#p2p-body tr.mp-p2p-row'));
    if (!rows.length) return;
    keyboardCursor = Math.max(0, Math.min(rows.length - 1, keyboardCursor + delta));
    rows.forEach((r, i) => r.classList.toggle('is-cursor', i === keyboardCursor));
    rows[keyboardCursor]?.scrollIntoView({ block: 'nearest' });
  }
  function cursorRow() {
    const rows = Array.from(document.querySelectorAll('#p2p-body tr.mp-p2p-row'));
    return rows[keyboardCursor] || null;
  }
  function setupKeyboardNav() {
    document.addEventListener('keydown', (e) => {
      if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) return;
      const palette = document.getElementById('p2p-palette');
      if (palette && !palette.hidden) return;
      if (e.key === 'j') { e.preventDefault(); moveCursor(1); }
      else if (e.key === 'k') { e.preventDefault(); moveCursor(-1); }
      else if (e.key === 'Enter' && keyboardCursor >= 0) {
        e.preventDefault();
        const row = cursorRow();
        const id = row?.dataset.offerId;
        const offer = state.offers.find((o) => o.id === id);
        if (offer) openDrawer(offer);
      } else if (e.key === 'x' && keyboardCursor >= 0) {
        e.preventDefault();
        const row = cursorRow();
        const id = row?.dataset.offerId;
        const offer = state.offers.find((o) => o.id === id);
        if (offer && offer.status === 'pending') openCancelModal(offer, null);
      } else if (e.key === '?') {
        e.preventDefault();
        if (typeof mpToast === 'function') {
          mpToast('Shortcuts: ⌘K palette · / search · j/k navigate · Enter detail · x cancel · Esc close', 'info');
        }
      }
    });
  }

  document.addEventListener('DOMContentLoaded', async () => {
    wireEvents();
    syncKpiPressed();
    populateColumnsMenu();
    setupColumnsMenu();
    setupScrollShadow();
    setupPalette();
    setupDemoToggle();
    setupKeyboardNav();
    applyColumnVisibility();
    await loadP2P();
    setupResizable();
    startAutoRefresh();
  });
})();
