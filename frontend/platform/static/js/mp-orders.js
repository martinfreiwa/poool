/**
 * Open Orders - live admin order list with filters, sort, drill-downs,
 * partial-fill progress, SLA aging, auto-refresh, CSV export.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/orders';
  const AUTO_REFRESH_MS = 30000;
  const SEARCH_DEBOUNCE_MS = 300;

  const state = {
    page: 1,
    perPage: 25,
    totalPages: 1,
    totalOrders: 0,
    status: 'open,partially_filled',
    side: '',
    q: '',
    sort: 'created_at',
    order: 'desc',
    isLoading: false,
    autoRefresh: false,
    autoTimer: null,
    searchTimer: null,
    selectedIds: new Set(),
    userCounts: new Map(),    // user_id -> count of orders in current page
    statsCache: null,
  };

  // Anomaly thresholds
  const HELD_WARN_CENTS = 1_000_000;     // $10k
  const HELD_DANGER_CENTS = 10_000_000;  // $100k
  const ANOMALY_USER_ORDERS = 3;
  const ANOMALY_HELD_CENTS = 5_000_000;  // $50k

  // ── Helpers ────────────────────────────────────────────────────────
  const $ = (id) => document.getElementById(id);
  const setText = (id, v) => { const el = $(id); if (el) el.textContent = v; };

  function formatMoney(cents) {
    return '$' + (Number(cents || 0) / 100).toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  }

  function formatQuantity(value) {
    return Number(value || 0).toLocaleString();
  }

  function shortId(value) {
    return String(value || '').substring(0, 8);
  }

  function userLabel(order) {
    if (order.user_email) return String(order.user_email).split('@')[0];
    return shortId(order.user_id);
  }

  function sideLabel(order) {
    return String(order.side || '').toUpperCase();
  }

  function typeLabel(order) {
    const type = String(order.order_type || 'limit');
    return type.charAt(0).toUpperCase() + type.slice(1);
  }

  function remainingQuantity(order) {
    return Number(order.quantity || 0) - Number(order.quantity_filled || 0);
  }

  function heldCents(order) {
    return Number(order.price_cents || 0) * remainingQuantity(order);
  }

  function ageHours(dateStr) {
    const t = new Date(dateStr).getTime();
    if (!Number.isFinite(t)) return null;
    return Math.max(0, (Date.now() - t) / 3600000);
  }

  function ageBucket(hours) {
    if (hours == null) return 'unknown';
    if (hours < 6) return 'fresh';
    if (hours < 24) return 'aging';
    return 'stale';
  }

  function timeAgo(dateStr) {
    const t = new Date(dateStr).getTime();
    if (!Number.isFinite(t)) return '-';
    const diff = Math.max(0, (Date.now() - t) / 1000);
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  function isoTooltip(dateStr) {
    const d = new Date(dateStr);
    return Number.isFinite(d.getTime()) ? d.toISOString() : '';
  }

  function csrfToken() {
    return document.cookie
      .split(';')
      .map((p) => p.trim())
      .find((p) => p.startsWith('csrf_token='))
      ?.split('=')
      .slice(1)
      .join('=') || '';
  }

  function clearTable() {
    const tbody = $('orders-body');
    if (tbody) tbody.replaceChildren();
    return tbody;
  }

  function renderStateRow(message, tone) {
    const tbody = clearTable();
    if (!tbody) return;
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 11;
    cell.style.textAlign = 'center';
    cell.style.color = tone === 'error' ? 'var(--admin-danger)' : 'var(--admin-text-muted)';
    cell.style.padding = '24px';
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function renderBadge(text, modifier) {
    const badge = document.createElement('span');
    badge.className = `admin-badge ${modifier}`;
    const dot = document.createElement('span');
    dot.className = 'admin-badge-dot';
    badge.appendChild(dot);
    badge.append(document.createTextNode(text));
    return badge;
  }

  function statusBadge(status) {
    switch (status) {
      case 'partially_filled': return renderBadge('Partial', 'admin-badge--warning');
      case 'filled':            return renderBadge('Filled', 'admin-badge--success');
      case 'cancelled':         return renderBadge('Cancelled', 'admin-badge--neutral');
      case 'admin_cancelled':   return renderBadge('Admin Cancelled', 'admin-badge--danger');
      default:                  return renderBadge('Open', 'admin-badge--info');
    }
  }

  // ── KPIs ───────────────────────────────────────────────────────────
  function updateKpis(orders) {
    const totalHeld = orders.reduce((s, o) => s + heldCents(o), 0);
    const ages = orders.map((o) => ageHours(o.created_at)).filter((h) => h != null);
    const avg = ages.length ? ages.reduce((s, h) => s + h, 0) / ages.length : 0;

    setText('kpi-total-open', state.totalOrders.toLocaleString());
    setText('kpi-held-balance', formatMoney(totalHeld));

    const ageEl = $('kpi-avg-age');
    if (ageEl) {
      ageEl.textContent = ages.length ? avg.toFixed(1) + 'h' : '0h';
      ageEl.dataset.bucket = ageBucket(ages.length ? avg : null);
    }
  }

  // ── Pagination ─────────────────────────────────────────────────────
  function updatePagination() {
    const info = $('orders-page-info');
    const prev = $('orders-prev-page');
    const next = $('orders-next-page');
    const pageCount = Math.max(state.totalPages, 1);
    if (info) {
      info.textContent = `${state.totalOrders.toLocaleString()} orders · page ${state.page} of ${pageCount}`;
    }
    if (prev) prev.disabled = state.isLoading || state.page <= 1;
    if (next) next.disabled = state.isLoading || state.page >= pageCount;
  }

  // ── Last-updated ───────────────────────────────────────────────────
  let lastLoadedAt = null;
  function updateLastUpdated() {
    const el = $('orders-last-updated');
    if (!el) return;
    if (!lastLoadedAt) { el.textContent = 'Updated —'; return; }
    el.textContent = 'Updated ' + timeAgo(lastLoadedAt.toISOString());
    el.title = lastLoadedAt.toISOString();
  }
  setInterval(updateLastUpdated, 5000);

  // ── Anomaly detection ──────────────────────────────────────────────
  function isInternalUser(order) {
    const email = String(order.user_email || '').toLowerCase();
    return email === 'admin' ||
      email.startsWith('admin@') ||
      email.includes('+admin@') ||
      email.endsWith('@poool.app') ||
      email.endsWith('@poool.internal');
  }

  function anomalyFlags(order) {
    const flags = [];
    const held = heldCents(order);
    const userOrders = state.userCounts.get(order.user_id) || 0;
    if (held >= ANOMALY_HELD_CENTS) flags.push({ tone: 'danger', label: 'Large hold', title: `Held ${formatMoney(held)} ≥ $50k` });
    if (userOrders >= ANOMALY_USER_ORDERS) flags.push({ tone: 'warning', label: `${userOrders}× user`, title: `User has ${userOrders} orders on this page` });
    if (isInternalUser(order)) flags.push({ tone: 'danger', label: 'Internal', title: 'Internal/admin user — verify before cancel' });
    if (ageHours(order.created_at) >= 48) flags.push({ tone: 'warning', label: 'Stale 48h+', title: 'Order older than 48 hours' });
    return flags;
  }

  function renderAnomalyBadges(flags) {
    if (!flags.length) return null;
    const wrap = document.createElement('span');
    wrap.className = 'mp-anomaly-wrap';
    flags.forEach((f) => {
      const b = document.createElement('span');
      b.className = `mp-anomaly mp-anomaly--${f.tone}`;
      b.textContent = '⚠ ' + f.label;
      b.title = f.title;
      wrap.appendChild(b);
    });
    return wrap;
  }

  function heldThresholdClass(cents) {
    if (cents >= HELD_DANGER_CENTS) return 'admin-badge--danger';
    if (cents >= HELD_WARN_CENTS) return 'admin-badge--warning';
    return 'admin-badge--neutral';
  }

  // ── Render ─────────────────────────────────────────────────────────
  function renderRow(order) {
    const row = document.createElement('tr');
    row.dataset.orderId = order.id;

    const flags = anomalyFlags(order);
    if (flags.length) row.classList.add('mp-row-anomaly');

    // Selection checkbox
    {
      const cell = document.createElement('td');
      cell.className = 'mp-td-checkbox';
      const isCancellable = order.status === 'open' || order.status === 'partially_filled';
      if (isCancellable) {
        const cb = document.createElement('input');
        cb.type = 'checkbox';
        cb.className = 'mp-row-check';
        cb.dataset.orderId = order.id;
        cb.checked = state.selectedIds.has(order.id);
        cb.setAttribute('aria-label', `Select order ${shortId(order.id)}`);
        cb.addEventListener('change', () => {
          if (cb.checked) state.selectedIds.add(order.id);
          else state.selectedIds.delete(order.id);
          updateBulkBar();
          updateSelectAllState();
        });
        cell.appendChild(cb);
      }
      row.appendChild(cell);
    }

    // Order ID — copyable, truncated
    {
      const cell = document.createElement('td');
      const code = document.createElement('code');
      code.className = 'mp-id-copy';
      code.textContent = shortId(order.id);
      code.title = `${order.id} (click to copy)`;
      code.addEventListener('click', async () => {
        try {
          await navigator.clipboard.writeText(order.id);
          mpToast('Order ID copied', 'success');
        } catch (_) {
          mpToast('Copy failed', 'error');
        }
      });
      cell.appendChild(code);
      const badges = renderAnomalyBadges(flags);
      if (badges) cell.appendChild(badges);
      row.appendChild(cell);
    }

    // User → drill-down
    {
      const cell = document.createElement('td');
      if (order.user_id) {
        const a = document.createElement('a');
        a.className = 'admin-link';
        a.href = `/admin/user-details?id=${encodeURIComponent(order.user_id)}`;
        a.textContent = userLabel(order);
        a.title = order.user_email || order.user_id;
        cell.appendChild(a);
      } else {
        cell.textContent = userLabel(order);
      }
      if (isInternalUser(order)) {
        const tag = document.createElement('span');
        tag.className = 'mp-internal-tag';
        tag.textContent = 'INTERNAL';
        tag.title = 'Internal/admin user';
        cell.appendChild(tag);
      }
      row.appendChild(cell);
    }

    // Asset → drill-down
    {
      const cell = document.createElement('td');
      cell.style.fontWeight = '600';
      if (order.asset_id) {
        const a = document.createElement('a');
        a.className = 'admin-link';
        a.href = `/admin/asset-details.html?id=${encodeURIComponent(order.asset_id)}`;
        a.textContent = order.asset_name || shortId(order.asset_id);
        cell.appendChild(a);
      } else {
        cell.textContent = order.asset_name || '—';
      }
      row.appendChild(cell);
    }

    // Side
    {
      const cell = document.createElement('td');
      const span = document.createElement('span');
      const side = sideLabel(order);
      span.className = side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      span.textContent = side;
      cell.appendChild(span);
      row.appendChild(cell);
    }

    // Type
    {
      const cell = document.createElement('td');
      cell.appendChild(renderBadge(typeLabel(order), 'admin-badge--neutral'));
      row.appendChild(cell);
    }

    // Qty / Filled with progress bar when partial
    {
      const cell = document.createElement('td');
      cell.style.textAlign = 'right';
      cell.style.fontVariantNumeric = 'tabular-nums';
      const total = Number(order.quantity || 0);
      const filled = Number(order.quantity_filled || 0);
      const pct = total > 0 ? (filled / total) * 100 : 0;
      const wrap = document.createElement('div');
      wrap.className = 'mp-qty-cell';
      const label = document.createElement('div');
      label.className = 'mp-qty-label';
      label.textContent = filled > 0
        ? `${formatQuantity(filled)} / ${formatQuantity(total)}`
        : formatQuantity(total);
      wrap.appendChild(label);
      if (filled > 0) {
        const bar = document.createElement('div');
        bar.className = 'mp-qty-bar';
        const fill = document.createElement('div');
        fill.className = 'mp-qty-bar-fill';
        fill.style.width = pct.toFixed(1) + '%';
        fill.title = `${pct.toFixed(1)}% filled`;
        bar.appendChild(fill);
        wrap.appendChild(bar);
      }
      cell.appendChild(wrap);
      row.appendChild(cell);
    }

    // Price
    {
      const cell = document.createElement('td');
      cell.style.textAlign = 'right';
      cell.style.fontVariantNumeric = 'tabular-nums';
      cell.textContent = formatMoney(order.price_cents);
      row.appendChild(cell);
    }

    // Held balance — threshold-colored
    {
      const cell = document.createElement('td');
      cell.style.textAlign = 'right';
      const held = heldCents(order);
      cell.appendChild(renderBadge(formatMoney(held), heldThresholdClass(held)));
      row.appendChild(cell);
    }

    // Created — SLA aging color + ISO tooltip
    {
      const cell = document.createElement('td');
      cell.className = 'mp-age-cell';
      cell.dataset.bucket = ageBucket(ageHours(order.created_at));
      cell.textContent = timeAgo(order.created_at);
      cell.title = isoTooltip(order.created_at);
      row.appendChild(cell);
    }

    // Status
    {
      const cell = document.createElement('td');
      cell.appendChild(statusBadge(order.status));
      row.appendChild(cell);
    }

    // Actions: Orderbook + Cancel
    {
      const cell = document.createElement('td');
      cell.style.textAlign = 'center';
      cell.style.whiteSpace = 'nowrap';

      if (order.asset_id) {
        const ob = document.createElement('a');
        ob.className = 'admin-btn admin-btn--secondary admin-btn--sm';
        ob.href = `/admin/marketplace/orderbook.html?asset=${encodeURIComponent(order.asset_id)}`;
        ob.textContent = 'Book';
        ob.title = 'View orderbook for this asset';
        ob.style.marginRight = '6px';
        cell.appendChild(ob);
      }

      const isCancellable = order.status === 'open' || order.status === 'partially_filled';
      if (isCancellable) {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'admin-btn admin-btn--danger admin-btn--sm btn-cancel-order';
        btn.textContent = 'Cancel';
        btn.setAttribute(
          'aria-label',
          `Cancel order ${shortId(order.id)} — ${sideLabel(order)} ${formatQuantity(remainingQuantity(order))} of ${order.asset_name || 'asset'} for ${formatMoney(heldCents(order))}`
        );
        btn.addEventListener('click', () => openCancelModal(order));
        cell.appendChild(btn);
      }
      row.appendChild(cell);
    }

    return row;
  }

  // Track previously-seen order IDs to flash new rows
  const previouslySeenIds = new Set();
  let firstLoad = true;

  function renderOrders(orders) {
    // Build user-order count for anomaly detection on this page
    state.userCounts = new Map();
    let buyN = 0, sellN = 0;
    orders.forEach((o) => {
      if (!o.user_id) return;
      state.userCounts.set(o.user_id, (state.userCounts.get(o.user_id) || 0) + 1);
      if (o.side === 'buy') buyN++;
      else if (o.side === 'sell') sellN++;
    });
    // Drop selections no longer visible (different page/filter)
    const visibleIds = new Set(orders.map((o) => o.id));
    for (const id of Array.from(state.selectedIds)) {
      if (!visibleIds.has(id)) state.selectedIds.delete(id);
    }

    // Side counters
    const cb = $('orders-count-buy');
    const cs = $('orders-count-sell');
    if (cb) cb.textContent = `BUY ${buyN}`;
    if (cs) cs.textContent = `SELL ${sellN}`;
    if (cb) cb.classList.toggle('mp-side-counter--zero', buyN === 0);
    if (cs) cs.classList.toggle('mp-side-counter--zero', sellN === 0);

    updateKpis(orders);
    updatePagination();
    const tbody = clearTable();
    if (!tbody) return;
    if (orders.length === 0) {
      renderStateRow('No orders match the current filters', 'empty');
      updateBulkBar();
      updateSelectAllState();
      return;
    }
    orders.forEach((o) => {
      const tr = renderRow(o);
      // Flash new rows since last refresh (skip on first load)
      if (!firstLoad && !previouslySeenIds.has(o.id)) {
        tr.classList.add('mp-row-new');
        setTimeout(() => tr.classList.remove('mp-row-new'), 2500);
      }
      tbody.appendChild(tr);
    });
    // Update seen set for next refresh
    previouslySeenIds.clear();
    orders.forEach((o) => previouslySeenIds.add(o.id));
    firstLoad = false;
    updateBulkBar();
    updateSelectAllState();
  }

  // ── Bulk selection ─────────────────────────────────────────────────
  function updateBulkBar() {
    const bar = $('orders-bulk-bar');
    const count = $('orders-bulk-count');
    const n = state.selectedIds.size;
    if (count) count.textContent = String(n);
    if (bar) bar.hidden = n === 0;
  }

  function updateSelectAllState() {
    const all = $('orders-select-all');
    if (!all) return;
    const checks = document.querySelectorAll('.mp-row-check');
    if (!checks.length) {
      all.checked = false;
      all.indeterminate = false;
      return;
    }
    const checked = Array.from(checks).filter((c) => c.checked).length;
    all.checked = checked === checks.length;
    all.indeterminate = checked > 0 && checked < checks.length;
  }

  function bindBulkSelection() {
    $('orders-select-all')?.addEventListener('change', (e) => {
      const on = !!e.target.checked;
      document.querySelectorAll('.mp-row-check').forEach((cb) => {
        cb.checked = on;
        const id = cb.dataset.orderId;
        if (!id) return;
        if (on) state.selectedIds.add(id);
        else state.selectedIds.delete(id);
      });
      updateBulkBar();
    });

    $('orders-bulk-clear')?.addEventListener('click', () => {
      state.selectedIds.clear();
      document.querySelectorAll('.mp-row-check').forEach((cb) => { cb.checked = false; });
      updateBulkBar();
      updateSelectAllState();
    });

    $('orders-bulk-cancel')?.addEventListener('click', () => openBulkCancelModal());
    $('orders-bulk-export')?.addEventListener('click', () => bulkExport());
  }

  function buildBulkCancelBody(n) {
    const body = document.createElement('div');
    const group = document.createElement('div');
    group.className = 'admin-form-group';
    const label = document.createElement('label');
    label.className = 'admin-form-label';
    label.setAttribute('for', 'bulk-cancel-reason');
    label.textContent = `Reason for cancelling ${n} orders *`;
    const textarea = document.createElement('textarea');
    textarea.className = 'admin-textarea';
    textarea.id = 'bulk-cancel-reason';
    textarea.placeholder = 'Enter the legal reason — applied to all selected orders…';
    textarea.rows = 3;
    textarea.style.minHeight = '80px';
    group.append(label, textarea);
    body.appendChild(group);

    const warning = document.createElement('div');
    warning.style.cssText = 'display:flex; align-items:center; gap:8px; padding:10px 14px; background:var(--admin-danger-bg); border-radius:var(--admin-radius-sm); margin-top:8px;';
    const wt = document.createElement('span');
    wt.style.cssText = 'color:var(--admin-danger); font-size:13px; font-weight:500;';
    wt.textContent = `This will release the held balance/tokens on all ${n} orders. Each cancellation is audit-logged.`;
    warning.appendChild(wt);
    body.appendChild(warning);
    return body;
  }

  function openBulkCancelModal() {
    const ids = Array.from(state.selectedIds);
    if (!ids.length) { mpToast('No orders selected', 'error'); return; }
    const n = ids.length;
    mpModal({
      title: `Cancel ${n} Orders`,
      subtitle: 'Bulk admin cancellation — audit-logged per order',
      bodyNode: buildBulkCancelBody(n),
      confirmLabel: `Cancel ${n} Orders`,
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#bulk-cancel-reason')?.value?.trim();
        const confirmBtn = overlay.querySelector('.mp-modal-confirm');
        if (!reason) { mpToast('Please provide a cancellation reason', 'error'); return false; }
        try {
          if (confirmBtn) { confirmBtn.disabled = true; confirmBtn.textContent = 'Cancelling…'; }
          const token = csrfToken();
          const res = await fetch(`${API}/bulk-cancel`, {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
            body: JSON.stringify({ order_ids: ids, reason }),
          });
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || err.message || `HTTP ${res.status}`);
          }
          const data = await res.json();
          const ok = data.succeeded_count || 0;
          const bad = data.failed_count || 0;
          mpToast(`Cancelled ${ok}, failed ${bad}`, bad ? 'warning' : 'success');
          state.selectedIds.clear();
          await loadOrders();
          return true;
        } catch (err) {
          mpToast(`Bulk cancel failed: ${err.message}`, 'error');
          if (confirmBtn) { confirmBtn.disabled = false; confirmBtn.textContent = `Cancel ${n} Orders`; }
          return false;
        }
      },
    });
  }

  function bulkExport() {
    const ids = Array.from(state.selectedIds);
    if (!ids.length) { mpToast('No orders selected', 'error'); return; }
    // Export as client-side CSV from current page rows
    const rows = document.querySelectorAll('#orders-body tr[data-order-id]');
    const visible = Array.from(rows).filter((r) => state.selectedIds.has(r.dataset.orderId));
    const header = ['Order_ID', 'Created', 'Side', 'Status'];
    const lines = [header.join(',')];
    visible.forEach((r) => {
      const id = r.dataset.orderId;
      const tds = r.querySelectorAll('td');
      const created = tds[9]?.title || tds[9]?.textContent || '';
      const side = tds[4]?.textContent?.trim() || '';
      const status = tds[10]?.textContent?.trim() || '';
      lines.push([id, created, side, status].map((v) => `"${String(v).replace(/"/g, '""')}"`).join(','));
    });
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'selected_orders.csv';
    a.click();
    URL.revokeObjectURL(url);
  }

  // ── Cancel modal ───────────────────────────────────────────────────
  const CANCEL_REASON_PRESETS = [
    'User request via support ticket',
    'KYC/AML — account flagged',
    'Stale order — older than 30 days',
    'Mispriced order — significant deviation from market',
    'Compliance hold — regulator request',
    'Suspected fraud / market manipulation',
    'Duplicate order',
    'Asset delisted',
  ];

  function buildCancelBody(order) {
    const body = document.createElement('div');
    const group = document.createElement('div');
    group.className = 'admin-form-group';
    const label = document.createElement('label');
    label.className = 'admin-form-label';
    label.setAttribute('for', 'cancel-reason');
    label.textContent = 'Reason for Cancellation *';

    const presetWrap = document.createElement('div');
    presetWrap.className = 'mp-reason-presets';
    CANCEL_REASON_PRESETS.forEach((r) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'mp-chip mp-chip--reason';
      chip.textContent = r;
      chip.addEventListener('click', () => {
        const ta = body.querySelector('#cancel-reason');
        if (ta) { ta.value = r; ta.focus(); }
      });
      presetWrap.appendChild(chip);
    });

    const textarea = document.createElement('textarea');
    textarea.className = 'admin-textarea';
    textarea.id = 'cancel-reason';
    textarea.placeholder = 'Enter the legal reason for this cancellation…';
    textarea.rows = 3;
    textarea.style.minHeight = '80px';
    group.append(label, presetWrap, textarea);
    body.appendChild(group);

    const warning = document.createElement('div');
    warning.style.cssText = 'display:flex; align-items:center; gap:8px; padding:10px 14px; background:var(--admin-danger-bg); border-radius:var(--admin-radius-sm); margin-top:8px;';
    const wt = document.createElement('span');
    wt.style.cssText = 'color:var(--admin-danger); font-size:13px; font-weight:500;';
    const heldText = sideLabel(order) === 'BUY'
      ? `${formatMoney(heldCents(order))} from the user's wallet hold`
      : `${formatQuantity(remainingQuantity(order))} held tokens`;
    wt.textContent = `This will release ${heldText}.`;
    warning.appendChild(wt);
    body.appendChild(warning);
    return body;
  }

  function openCancelModal(order) {
    const orderId = shortId(order.id);
    const asset = order.asset_name || 'Asset';
    const side = sideLabel(order);
    const qty = formatQuantity(remainingQuantity(order));
    const price = formatMoney(order.price_cents);

    mpModal({
      title: 'Cancel Order',
      subtitle: `Order ${orderId} · ${asset} (${side} ${qty} @ ${price})`,
      bodyNode: buildCancelBody(order),
      confirmLabel: 'Cancel Order',
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#cancel-reason')?.value?.trim();
        const confirmBtn = overlay.querySelector('.mp-modal-confirm');
        if (!reason) { mpToast('Please provide a cancellation reason', 'error'); return false; }
        try {
          if (confirmBtn) { confirmBtn.disabled = true; confirmBtn.textContent = 'Cancelling…'; }
          const token = csrfToken();
          const res = await fetch(`${API}/${encodeURIComponent(order.id)}`, {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
            body: JSON.stringify({ reason }),
          });
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || err.message || `HTTP ${res.status}`);
          }
          mpToast(`Order ${orderId} cancelled`, 'success');
          await loadOrders();
          return true;
        } catch (err) {
          mpToast(`Failed to cancel: ${err.message}`, 'error');
          if (confirmBtn) { confirmBtn.disabled = false; confirmBtn.textContent = 'Cancel Order'; }
          return false;
        }
      },
    });
  }

  // ── Query string ───────────────────────────────────────────────────
  function buildQuery(extra) {
    const params = new URLSearchParams();
    params.set('page', String(state.page));
    params.set('per_page', String(state.perPage));
    if (state.status) params.set('status', state.status);
    if (state.side) params.set('side', state.side);
    if (state.q) params.set('q', state.q);
    if (state.sort) params.set('sort', state.sort);
    if (state.order) params.set('order', state.order);
    if (extra) Object.entries(extra).forEach(([k, v]) => params.set(k, v));
    return params.toString();
  }

  // ── Load ───────────────────────────────────────────────────────────
  async function loadOrders() {
    if (state.isLoading) return;
    state.isLoading = true;
    updatePagination();
    if (!$('orders-body')?.querySelector('tr[data-order-id]')) {
      renderStateRow('Loading orders…', 'empty');
    }
    try {
      const res = await fetch(`${API}?${buildQuery()}`, { credentials: 'same-origin' });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || err.message || `HTTP ${res.status}`);
      }
      const data = await res.json();
      state.totalOrders = Number(data.total || 0);
      state.totalPages = Math.max(Number(data.total_pages || 0), 1);
      if (state.page > state.totalPages) state.page = state.totalPages;
      lastLoadedAt = new Date();
      updateLastUpdated();
      renderOrders(data.data || []);
      // Refresh stats in parallel; not awaited
      loadStats();
    } catch (err) {
      console.warn('[mp-orders] Failed to load:', err);
      state.totalOrders = 0;
      state.totalPages = 1;
      setText('kpi-total-open', '-');
      setText('kpi-held-balance', '-');
      setText('kpi-avg-age', '-');
      renderStateRow(`Could not load orders: ${err.message}`, 'error');
      mpToast('Could not load open orders', 'error');
    } finally {
      state.isLoading = false;
      updatePagination();
    }
  }

  // ── Auto-refresh ───────────────────────────────────────────────────
  function setAutoRefresh(on) {
    state.autoRefresh = on;
    if (state.autoTimer) { clearInterval(state.autoTimer); state.autoTimer = null; }
    if (on) {
      state.autoTimer = setInterval(() => {
        if (!document.hidden && !state.isLoading) loadOrders();
      }, AUTO_REFRESH_MS);
    }
    try { localStorage.setItem('mp_orders_autorefresh', on ? '1' : '0'); } catch (_) {}
  }

  // ── Sort headers ───────────────────────────────────────────────────
  function applySortIndicator() {
    document.querySelectorAll('.mp-th-sort').forEach((th) => {
      th.classList.remove('active', 'is-asc', 'is-desc');
      if (th.dataset.sort === state.sort) {
        th.classList.add('active', state.order === 'asc' ? 'is-asc' : 'is-desc');
      }
    });
  }

  function bindSortHeaders() {
    document.querySelectorAll('.mp-th-sort').forEach((th) => {
      th.style.cursor = 'pointer';
      th.addEventListener('click', () => {
        const col = th.dataset.sort;
        if (!col) return;
        if (state.sort === col) {
          state.order = state.order === 'asc' ? 'desc' : 'asc';
        } else {
          state.sort = col;
          state.order = 'desc';
        }
        state.page = 1;
        applySortIndicator();
        loadOrders();
      });
    });
  }

  // ── Tabs ───────────────────────────────────────────────────────────
  function bindTabs() {
    document.querySelectorAll('.admin-tab').forEach((tab) => {
      tab.addEventListener('click', () => {
        document.querySelectorAll('.admin-tab').forEach((t) => {
          t.classList.remove('active');
          t.setAttribute('aria-selected', 'false');
        });
        tab.classList.add('active');
        tab.setAttribute('aria-selected', 'true');
        state.status = tab.dataset.status || '';
        state.page = 1;
        loadOrders();
      });
    });
  }

  // ── Filters ────────────────────────────────────────────────────────
  function bindFilters() {
    $('orders-search')?.addEventListener('input', (e) => {
      clearTimeout(state.searchTimer);
      const v = e.target.value.trim();
      state.searchTimer = setTimeout(() => {
        state.q = v;
        state.page = 1;
        loadOrders();
      }, SEARCH_DEBOUNCE_MS);
    });

    $('orders-side')?.addEventListener('change', (e) => {
      state.side = e.target.value || '';
      state.page = 1;
      loadOrders();
    });

    $('orders-page-size')?.addEventListener('change', (e) => {
      const n = Math.max(1, parseInt(e.target.value, 10) || 25);
      state.perPage = n;
      state.page = 1;
      try { localStorage.setItem('mp_orders_page_size', String(n)); } catch (_) {}
      loadOrders();
    });

    $('orders-refresh')?.addEventListener('click', () => loadOrders());

    $('orders-autorefresh')?.addEventListener('change', (e) => setAutoRefresh(!!e.target.checked));

    $('orders-export-csv')?.addEventListener('click', () => {
      const params = new URLSearchParams();
      if (state.status) params.set('status', state.status);
      if (state.side) params.set('side', state.side);
      if (state.q) params.set('q', state.q);
      window.location.href = `${API}/export.csv?${params.toString()}`;
    });
  }

  // ── Pagination buttons ─────────────────────────────────────────────
  function bindPagination() {
    $('orders-prev-page')?.addEventListener('click', () => {
      if (state.page <= 1) return;
      state.page -= 1;
      loadOrders();
    });
    $('orders-next-page')?.addEventListener('click', () => {
      if (state.page >= state.totalPages) return;
      state.page += 1;
      loadOrders();
    });
  }

  // ── Stats: deltas, P50/P90, sparkline ──────────────────────────────
  async function loadStats() {
    try {
      const res = await fetch(`${API}/stats`, { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      state.statsCache = data;
      applyStats(data);
    } catch (e) {
      console.warn('[mp-orders] stats fetch failed:', e);
    }
  }

  function fmtSec(s) {
    if (s == null || !Number.isFinite(s)) return '—';
    if (s < 60) return Math.round(s) + 's';
    if (s < 3600) return Math.round(s / 60) + 'm';
    if (s < 86400) return (s / 3600).toFixed(1) + 'h';
    return (s / 86400).toFixed(1) + 'd';
  }

  function applyStats(data) {
    // Delta vs yesterday on Total Open
    const today = Number(data.today_count || 0);
    const yest = Number(data.yesterday_count || 0);
    const deltaEl = $('kpi-total-delta');
    if (deltaEl) {
      if (yest > 0) {
        const pct = ((today - yest) / yest) * 100;
        const arrow = pct > 0 ? '↑' : pct < 0 ? '↓' : '→';
        const tone = pct > 10 ? 'up' : pct < -10 ? 'down' : 'flat';
        deltaEl.dataset.tone = tone;
        deltaEl.textContent = `${arrow} ${Math.abs(pct).toFixed(0)}% vs 24h`;
      } else if (today > 0) {
        deltaEl.dataset.tone = 'up';
        deltaEl.textContent = `+${today} new (24h)`;
      } else {
        deltaEl.textContent = '';
      }
    }

    // Age distribution
    const dist = $('kpi-age-distribution');
    if (dist) {
      dist.textContent = `P50 ${fmtSec(data.p50_age_sec)} · P90 ${fmtSec(data.p90_age_sec)}`;
    }

    // Sparkline
    drawSparkline($('kpi-total-spark'), data.sparkline || []);
  }

  function drawSparkline(svg, points) {
    if (!svg) return;
    while (svg.firstChild) svg.removeChild(svg.firstChild);
    if (!points.length) return;
    const max = Math.max(...points, 1);
    const w = 100, h = 24;
    const step = points.length > 1 ? w / (points.length - 1) : w;
    let d = '';
    points.forEach((p, i) => {
      const x = (i * step).toFixed(2);
      const y = (h - (p / max) * (h - 2) - 1).toFixed(2);
      d += (i === 0 ? 'M' : 'L') + x + ',' + y + ' ';
    });
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', d.trim());
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke', 'currentColor');
    path.setAttribute('stroke-width', '1.5');
    path.setAttribute('stroke-linejoin', 'round');
    svg.appendChild(path);
  }

  // ── KPI cards click → drill-down ───────────────────────────────────
  function bindKpiCards() {
    document.querySelectorAll('.mp-kpi-clickable').forEach((card) => {
      card.addEventListener('click', () => {
        const status = card.dataset.filterStatus;
        const sort = card.dataset.sort;
        const order = card.dataset.order;
        if (status) state.status = status;
        if (sort) state.sort = sort;
        if (order) state.order = order;
        state.page = 1;
        applySortIndicator();
        // Sync tab selection if relevant
        if (status) {
          document.querySelectorAll('.admin-tab').forEach((t) => {
            const isMatch = (t.dataset.status || '') === status;
            t.classList.toggle('active', isMatch);
            t.setAttribute('aria-selected', isMatch ? 'true' : 'false');
          });
        }
        loadOrders();
      });
    });
  }

  // ── Saved views ────────────────────────────────────────────────────
  function applyPreset(preset) {
    if (!preset || typeof preset !== 'object') return;
    if ('status' in preset) state.status = preset.status || '';
    if ('side' in preset) {
      state.side = preset.side || '';
      const sel = $('orders-side');
      if (sel) sel.value = state.side;
    }
    if ('q' in preset) {
      state.q = preset.q || '';
      const inp = $('orders-search');
      if (inp) inp.value = state.q;
    }
    if ('sort' in preset) state.sort = preset.sort || 'created_at';
    if ('order' in preset) state.order = preset.order || 'desc';
    state.page = 1;
    document.querySelectorAll('.admin-tab').forEach((t) => {
      const isMatch = (t.dataset.status || '') === state.status;
      t.classList.toggle('active', isMatch);
      t.setAttribute('aria-selected', isMatch ? 'true' : 'false');
    });
    applySortIndicator();
    loadOrders();
  }

  function readCustomViews() {
    try { return JSON.parse(localStorage.getItem('mp_orders_views') || '[]'); } catch (_) { return []; }
  }
  function writeCustomViews(arr) {
    try { localStorage.setItem('mp_orders_views', JSON.stringify(arr)); } catch (_) {}
  }
  function renderCustomViews() {
    const host = $('orders-saved-views-custom');
    if (!host) return;
    host.replaceChildren();
    readCustomViews().forEach((v, idx) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'mp-chip mp-chip--custom';
      chip.textContent = v.name;
      chip.title = `Load: ${v.name}`;
      chip.addEventListener('click', () => applyPreset(v.preset));
      const del = document.createElement('span');
      del.className = 'mp-chip-del';
      del.textContent = '×';
      del.title = 'Delete view';
      del.setAttribute('role', 'button');
      del.tabIndex = 0;
      del.addEventListener('click', (e) => {
        e.stopPropagation();
        const arr = readCustomViews();
        arr.splice(idx, 1);
        writeCustomViews(arr);
        renderCustomViews();
      });
      chip.appendChild(del);
      host.appendChild(chip);
    });
  }
  function bindSavedViews() {
    document.querySelectorAll('.mp-chip[data-preset]').forEach((c) => {
      c.addEventListener('click', () => {
        try { applyPreset(JSON.parse(c.dataset.preset)); } catch (_) {}
      });
    });
    $('orders-save-view')?.addEventListener('click', () => {
      const name = window.prompt('Name this view:');
      if (!name) return;
      const arr = readCustomViews();
      arr.push({
        name: name.trim().substring(0, 40),
        preset: { status: state.status, side: state.side, q: state.q, sort: state.sort, order: state.order },
      });
      writeCustomViews(arr);
      renderCustomViews();
      mpToast('View saved', 'success');
    });
    renderCustomViews();
  }

  // ── Keyboard shortcuts ─────────────────────────────────────────────
  function bindKeys() {
    document.addEventListener('keydown', (e) => {
      const target = e.target;
      const isInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
      if (e.key === '/' && !isInput) {
        e.preventDefault();
        $('orders-search')?.focus();
      } else if (e.key === 'r' && !isInput && !e.metaKey && !e.ctrlKey) {
        loadOrders();
      }
    });
  }

  // ── Restore prefs ──────────────────────────────────────────────────
  function restorePrefs() {
    try {
      const ps = parseInt(localStorage.getItem('mp_orders_page_size') || '', 10);
      if (Number.isFinite(ps) && ps > 0) {
        state.perPage = ps;
        const sel = $('orders-page-size');
        if (sel) sel.value = String(ps);
      }
      const ar = localStorage.getItem('mp_orders_autorefresh') === '1';
      if (ar) {
        const cb = $('orders-autorefresh');
        if (cb) cb.checked = true;
        setAutoRefresh(true);
      }
    } catch (_) {}
  }

  // ── Rebuild orderbook (admin recovery) ─────────────────────────────
  function bindRebuild() {
    $('orders-rebuild-orderbook')?.addEventListener('click', () => {
      mpModal({
        title: 'Rebuild Orderbook',
        subtitle: 'Forces Redis orderbook to be rebuilt from PostgreSQL — use only if matching engine is out of sync',
        confirmLabel: 'Rebuild Now',
        confirmClass: 'admin-btn--danger',
        onConfirm: async (overlay) => {
          const btn = overlay.querySelector('.mp-modal-confirm');
          try {
            if (btn) { btn.disabled = true; btn.textContent = 'Rebuilding…'; }
            const token = csrfToken();
            const res = await fetch('/api/admin/marketplace/orderbook/rebuild', {
              method: 'POST',
              credentials: 'same-origin',
              headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
            });
            if (!res.ok) {
              const err = await res.json().catch(() => ({}));
              throw new Error(err.error || err.message || `HTTP ${res.status}`);
            }
            mpToast('Orderbook rebuilt', 'success');
            await loadOrders();
            return true;
          } catch (err) {
            mpToast(`Rebuild failed: ${err.message}`, 'error');
            if (btn) { btn.disabled = false; btn.textContent = 'Rebuild Now'; }
            return false;
          }
        },
      });
    });
  }

  // ── Pending Settlements badge ──────────────────────────────────────
  async function loadSettlementsBadge() {
    try {
      const res = await fetch('/api/admin/marketplace/orders/stats?_=settlements', { credentials: 'same-origin' });
      if (!res.ok) return;
      // Reuse stats endpoint as a low-cost ping; real settlement count would have its own endpoint
      // We just ensure the link is reachable; counter pulled from settlements API if available
      const sCount = await fetch('/api/admin/pending-settlements?count=1', { credentials: 'same-origin' })
        .then((r) => r.ok ? r.json() : null)
        .catch(() => null);
      const badge = $('orders-settlements-count');
      if (badge && sCount && typeof sCount.total === 'number' && sCount.total > 0) {
        badge.textContent = sCount.total > 99 ? '99+' : String(sCount.total);
        badge.hidden = false;
      }
    } catch (_) {}
  }

  // ── Order Detail Drawer ────────────────────────────────────────────
  function openDrawer(order) {
    const root = $('orders-drawer');
    const title = $('orders-drawer-title');
    const body = $('orders-drawer-body');
    if (!root || !body) return;
    if (title) title.textContent = `Order ${shortId(order.id)} — ${order.asset_name || 'Asset'}`;
    body.innerHTML = '';
    const tbl = document.createElement('table');
    tbl.className = 'mp-drawer-kv';
    const rows = [
      ['Order ID', order.id],
      ['Created', new Date(order.created_at).toISOString()],
      ['User', order.user_email || order.user_id],
      ['Asset', order.asset_name || order.asset_id],
      ['Side', sideLabel(order)],
      ['Type', typeLabel(order)],
      ['Price', formatMoney(order.price_cents)],
      ['Quantity', `${formatQuantity(order.quantity_filled)} filled / ${formatQuantity(order.quantity)} total`],
      ['Held', formatMoney(heldCents(order))],
      ['Status', order.status],
    ];
    rows.forEach(([k, v]) => {
      const tr = document.createElement('tr');
      const th = document.createElement('th'); th.textContent = k;
      const td = document.createElement('td'); td.textContent = String(v);
      tr.append(th, td); tbl.appendChild(tr);
    });
    body.appendChild(tbl);

    // Action footer
    const actions = document.createElement('div');
    actions.className = 'mp-drawer-actions';
    if (order.user_id) {
      const u = document.createElement('a');
      u.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      u.href = `/admin/user-details?id=${encodeURIComponent(order.user_id)}`;
      u.textContent = 'View User';
      actions.appendChild(u);
    }
    if (order.asset_id) {
      const a = document.createElement('a');
      a.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      a.href = `/admin/asset-details.html?id=${encodeURIComponent(order.asset_id)}`;
      a.textContent = 'View Asset';
      actions.appendChild(a);
    }
    if (order.status === 'open' || order.status === 'partially_filled') {
      const c = document.createElement('button');
      c.type = 'button';
      c.className = 'admin-btn admin-btn--danger admin-btn--sm';
      c.textContent = 'Cancel Order';
      c.addEventListener('click', () => { closeDrawer(); openCancelModal(order); });
      actions.appendChild(c);
    }
    body.appendChild(actions);

    // Audit-log placeholder — best-effort fetch
    const auditH = document.createElement('h3');
    auditH.className = 'mp-drawer-section-title';
    auditH.textContent = 'Audit Log';
    body.appendChild(auditH);
    const auditList = document.createElement('div');
    auditList.className = 'mp-drawer-audit';
    auditList.textContent = 'Loading audit entries…';
    body.appendChild(auditList);
    fetch(`/api/admin/audit-logs?entity_type=market_order&entity_id=${encodeURIComponent(order.id)}&per_page=20`, { credentials: 'same-origin' })
      .then((r) => r.ok ? r.json() : null)
      .then((data) => {
        if (!data || !Array.isArray(data.data) || data.data.length === 0) {
          auditList.textContent = 'No audit entries found.';
          return;
        }
        auditList.innerHTML = '';
        data.data.forEach((e) => {
          const row = document.createElement('div');
          row.className = 'mp-drawer-audit-row';
          row.innerHTML = `<strong>${e.action || ''}</strong> <span class="mp-meta">${e.created_at || ''}${e.actor_email ? ' · ' + e.actor_email : ''}</span>`;
          auditList.appendChild(row);
        });
      })
      .catch(() => { auditList.textContent = 'Audit log unavailable.'; });

    root.hidden = false;
    document.body.classList.add('mp-no-scroll');
  }

  function closeDrawer() {
    const root = $('orders-drawer');
    if (root) root.hidden = true;
    document.body.classList.remove('mp-no-scroll');
  }

  function bindDrawer() {
    document.querySelectorAll('[data-drawer-close]').forEach((el) => el.addEventListener('click', closeDrawer));
    // Click on row code → drawer; double-click row → drawer
    document.addEventListener('dblclick', (e) => {
      const tr = e.target.closest && e.target.closest('tr[data-order-id]');
      if (!tr) return;
      const id = tr.dataset.orderId;
      const order = currentOrders().find((o) => o.id === id);
      if (order) openDrawer(order);
    });
  }

  // Last-rendered orders cache for drawer/palette lookups
  function currentOrders() {
    const out = [];
    document.querySelectorAll('#orders-body tr[data-order-id]').forEach((tr) => {
      // The order objects aren't stored on rows; refetch by ID via drawer's needs is fine.
      // For simplicity, drawer uses the latest-loaded set held by closure below.
    });
    return loadedOrdersCache;
  }
  let loadedOrdersCache = [];
  // Wrap renderOrders to capture
  const _origRender = renderOrders;
  renderOrders = function (orders) { loadedOrdersCache = orders.slice(); return _origRender(orders); };

  // ── Command Palette ────────────────────────────────────────────────
  const PALETTE_COMMANDS = [
    { id: 'tab-active',  label: 'Show Active orders',     run: () => { state.status = 'open,partially_filled'; syncTabs(); state.page = 1; loadOrders(); } },
    { id: 'tab-open',    label: 'Show Open only',         run: () => { state.status = 'open'; syncTabs(); state.page = 1; loadOrders(); } },
    { id: 'tab-partial', label: 'Show Partial only',      run: () => { state.status = 'partially_filled'; syncTabs(); state.page = 1; loadOrders(); } },
    { id: 'tab-filled',  label: 'Show Filled',            run: () => { state.status = 'filled'; syncTabs(); state.page = 1; loadOrders(); } },
    { id: 'tab-cancel',  label: 'Show Cancelled',         run: () => { state.status = 'cancelled,admin_cancelled'; syncTabs(); state.page = 1; loadOrders(); } },
    { id: 'side-buy',    label: 'Filter side: BUY',       run: () => { state.side = 'buy';  $('orders-side').value = 'buy';  state.page = 1; loadOrders(); } },
    { id: 'side-sell',   label: 'Filter side: SELL',      run: () => { state.side = 'sell'; $('orders-side').value = 'sell'; state.page = 1; loadOrders(); } },
    { id: 'side-all',    label: 'Filter side: All',       run: () => { state.side = '';     $('orders-side').value = '';     state.page = 1; loadOrders(); } },
    { id: 'sort-held',   label: 'Sort by Held Balance ↓', run: () => { state.sort = 'held'; state.order = 'desc'; applySortIndicator(); loadOrders(); } },
    { id: 'sort-old',    label: 'Sort by Oldest first',   run: () => { state.sort = 'created_at'; state.order = 'asc'; applySortIndicator(); loadOrders(); } },
    { id: 'sort-new',    label: 'Sort by Newest first',   run: () => { state.sort = 'created_at'; state.order = 'desc'; applySortIndicator(); loadOrders(); } },
    { id: 'export-csv',  label: 'Export current view → CSV', run: () => $('orders-export-csv')?.click() },
    { id: 'refresh',     label: 'Refresh now',            run: () => loadOrders() },
    { id: 'autorefresh', label: 'Toggle auto-refresh 30s', run: () => { const cb = $('orders-autorefresh'); if (cb) { cb.checked = !cb.checked; setAutoRefresh(cb.checked); } } },
    { id: 'rebuild',     label: 'Rebuild orderbook (recovery)', run: () => $('orders-rebuild-orderbook')?.click() },
    { id: 'bulk-cancel', label: 'Cancel selected orders', run: () => $('orders-bulk-cancel')?.click() },
    { id: 'bulk-clear',  label: 'Clear selection',        run: () => $('orders-bulk-clear')?.click() },
    { id: 'goto-trades', label: 'Go to Trade History',    run: () => { window.location.href = '/admin/marketplace/trades.html'; } },
    { id: 'goto-book',   label: 'Go to Orderbook',        run: () => { window.location.href = '/admin/marketplace/orderbook.html'; } },
    { id: 'goto-settle', label: 'Go to Pending Settlements', run: () => { window.location.href = '/admin/pending-settlements.html'; } },
  ];

  function syncTabs() {
    document.querySelectorAll('.admin-tab').forEach((t) => {
      const isMatch = (t.dataset.status || '') === state.status;
      t.classList.toggle('active', isMatch);
      t.setAttribute('aria-selected', isMatch ? 'true' : 'false');
    });
  }

  let paletteIdx = 0;
  function renderPalette(query) {
    const list = $('orders-palette-list');
    if (!list) return;
    list.innerHTML = '';
    const q = String(query || '').toLowerCase().trim();
    const matches = PALETTE_COMMANDS.filter((c) => !q || c.label.toLowerCase().includes(q));
    matches.forEach((c, i) => {
      const li = document.createElement('li');
      li.className = 'mp-palette-item' + (i === paletteIdx ? ' active' : '');
      li.dataset.idx = String(i);
      li.textContent = c.label;
      li.setAttribute('role', 'option');
      li.addEventListener('click', () => { runPaletteCommand(matches[i]); closePalette(); });
      list.appendChild(li);
    });
    list.dataset.matchCount = String(matches.length);
    list.__matches = matches;
  }
  function runPaletteCommand(cmd) { try { cmd?.run?.(); } catch (e) { console.warn(e); } }
  function openPalette() {
    const root = $('orders-palette');
    const input = $('orders-palette-input');
    if (!root || !input) return;
    paletteIdx = 0;
    root.hidden = false;
    input.value = '';
    renderPalette('');
    setTimeout(() => input.focus(), 0);
    document.body.classList.add('mp-no-scroll');
  }
  function closePalette() {
    const root = $('orders-palette');
    if (root) root.hidden = true;
    document.body.classList.remove('mp-no-scroll');
  }
  function bindPalette() {
    document.querySelectorAll('[data-palette-close]').forEach((e) => e.addEventListener('click', closePalette));
    const input = $('orders-palette-input');
    const list = $('orders-palette-list');
    input?.addEventListener('input', (e) => { paletteIdx = 0; renderPalette(e.target.value); });
    input?.addEventListener('keydown', (e) => {
      const matches = list?.__matches || [];
      if (e.key === 'ArrowDown') { e.preventDefault(); paletteIdx = Math.min(paletteIdx + 1, matches.length - 1); renderPalette(input.value); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); paletteIdx = Math.max(paletteIdx - 1, 0); renderPalette(input.value); }
      else if (e.key === 'Enter')   { e.preventDefault(); runPaletteCommand(matches[paletteIdx]); closePalette(); }
      else if (e.key === 'Escape')  { closePalette(); }
    });
  }

  // ── Row keyboard navigation: j/k cursor, x cancel, Enter open drawer
  let rowCursor = -1;
  function focusRow(idx) {
    const rows = document.querySelectorAll('#orders-body tr[data-order-id]');
    if (!rows.length) return;
    rowCursor = Math.max(0, Math.min(idx, rows.length - 1));
    rows.forEach((r, i) => r.classList.toggle('mp-row-focused', i === rowCursor));
    rows[rowCursor].scrollIntoView({ block: 'nearest' });
  }
  function bindRowNav() {
    document.addEventListener('keydown', (e) => {
      const target = e.target;
      const isInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
      if (isInput) return;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); return; }
      if (e.key === 'j') { e.preventDefault(); focusRow(rowCursor < 0 ? 0 : rowCursor + 1); }
      else if (e.key === 'k') { e.preventDefault(); focusRow(rowCursor < 0 ? 0 : rowCursor - 1); }
      else if (e.key === 'x') {
        const rows = document.querySelectorAll('#orders-body tr[data-order-id]');
        if (rowCursor >= 0 && rows[rowCursor]) {
          const id = rows[rowCursor].dataset.orderId;
          const order = loadedOrdersCache.find((o) => o.id === id);
          if (order) openCancelModal(order);
        }
      }
      else if (e.key === 'Enter' && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
        const rows = document.querySelectorAll('#orders-body tr[data-order-id]');
        if (rowCursor >= 0 && rows[rowCursor]) {
          const id = rows[rowCursor].dataset.orderId;
          const order = loadedOrdersCache.find((o) => o.id === id);
          if (order) openDrawer(order);
        }
      }
      else if (e.key === 'Escape') {
        if ($('orders-drawer') && !$('orders-drawer').hidden) closeDrawer();
        else state.selectedIds.clear();
        document.querySelectorAll('.mp-row-check').forEach((c) => { c.checked = false; });
        updateBulkBar();
        updateSelectAllState();
        document.querySelectorAll('.mp-row-focused').forEach((r) => r.classList.remove('mp-row-focused'));
        rowCursor = -1;
      }
    });
  }

  // ════════════════════════════════════════════════════════════════
  // Tier-2 Features
  // ════════════════════════════════════════════════════════════════

  // ── T-A: URL state sync ────────────────────────────────────────────
  let suppressUrlSync = false;
  function pushUrlState() {
    if (suppressUrlSync) return;
    const p = new URLSearchParams();
    if (state.status && state.status !== 'open,partially_filled') p.set('status', state.status);
    if (state.side) p.set('side', state.side);
    if (state.q) p.set('q', state.q);
    if (state.sort && state.sort !== 'created_at') p.set('sort', state.sort);
    if (state.order && state.order !== 'desc') p.set('order', state.order);
    if (state.page > 1) p.set('page', String(state.page));
    if (state.perPage !== 25) p.set('per_page', String(state.perPage));
    const qs = p.toString();
    const url = qs ? `${window.location.pathname}?${qs}${window.location.hash}` : window.location.pathname + window.location.hash;
    window.history.replaceState(null, '', url);
  }
  function readUrlState() {
    const p = new URLSearchParams(window.location.search);
    if (p.has('status')) state.status = p.get('status');
    if (p.has('side')) { state.side = p.get('side'); const s = $('orders-side'); if (s) s.value = state.side; }
    if (p.has('q'))    { state.q    = p.get('q');    const i = $('orders-search'); if (i) i.value = state.q; }
    if (p.has('sort'))  state.sort  = p.get('sort');
    if (p.has('order')) state.order = p.get('order');
    if (p.has('page'))  state.page  = Math.max(1, parseInt(p.get('page'), 10) || 1);
    if (p.has('per_page')) {
      state.perPage = Math.max(1, parseInt(p.get('per_page'), 10) || 25);
      const sel = $('orders-page-size'); if (sel) sel.value = String(state.perPage);
    }
    // Sync tab
    document.querySelectorAll('.admin-tab').forEach((t) => {
      const isMatch = (t.dataset.status || '') === state.status;
      t.classList.toggle('active', isMatch);
      t.setAttribute('aria-selected', isMatch ? 'true' : 'false');
    });
  }
  // Wrap loadOrders to push URL after each call
  const _loadOrdersOrig = loadOrders;
  loadOrders = async function () { pushUrlState(); return _loadOrdersOrig.apply(this, arguments); };

  // ── T-H: Share view via URL hash ───────────────────────────────────
  function bindShareView() {
    $('orders-share-view')?.addEventListener('click', async () => {
      pushUrlState();
      const url = window.location.href;
      try {
        await navigator.clipboard.writeText(url);
        mpToast('Shareable URL copied', 'success');
      } catch (_) {
        window.prompt('Copy URL:', url);
      }
    });
  }

  // ── T-F: Density toggle ───────────────────────────────────────────
  function applyDensity(density) {
    const tbl = $('orders-table');
    if (!tbl) return;
    tbl.classList.toggle('mp-density-compact', density === 'compact');
    document.querySelectorAll('.mp-view-btn[data-density]').forEach((b) => {
      b.classList.toggle('active', b.dataset.density === density);
    });
    try { localStorage.setItem('mp_orders_density', density); } catch (_) {}
  }
  function bindDensity() {
    document.querySelectorAll('.mp-view-btn[data-density]').forEach((b) => {
      b.addEventListener('click', () => applyDensity(b.dataset.density));
    });
    try {
      const d = localStorage.getItem('mp_orders_density') || 'comfortable';
      applyDensity(d);
    } catch (_) {}
  }

  // ── T-E: Column visibility toggle ─────────────────────────────────
  function applyColumns(visible) {
    document.querySelectorAll('[data-col]').forEach((el) => {
      const k = el.dataset.col;
      if (k && visible[k] === false) el.classList.add('mp-col-hidden');
      else el.classList.remove('mp-col-hidden');
    });
  }
  function readColumnState() {
    try { return JSON.parse(localStorage.getItem('mp_orders_columns') || '{}'); } catch (_) { return {}; }
  }
  function writeColumnState(v) {
    try { localStorage.setItem('mp_orders_columns', JSON.stringify(v)); } catch (_) {}
  }
  function bindColumns() {
    const btn = $('orders-columns-btn');
    const menu = $('orders-columns-menu');
    if (!btn || !menu) return;
    menu.hidden = true;
    menu.setAttribute('aria-hidden', 'true');
    btn.setAttribute('aria-expanded', 'false');
    const v = readColumnState();
    menu.querySelectorAll('input[data-col]').forEach((cb) => {
      const k = cb.dataset.col;
      if (k in v) cb.checked = v[k] !== false;
      cb.addEventListener('change', () => {
        v[k] = cb.checked;
        writeColumnState(v);
        applyColumns(v);
      });
    });
    applyColumns(v);
    btn.addEventListener('click', () => {
      const nextOpen = menu.hidden;
      menu.hidden = !nextOpen;
      menu.setAttribute('aria-hidden', String(!nextOpen));
      btn.setAttribute('aria-expanded', nextOpen ? 'true' : 'false');
    });
    document.addEventListener('click', (e) => {
      if (!menu.contains(e.target) && e.target !== btn) {
        menu.hidden = true;
        menu.setAttribute('aria-hidden', 'true');
        btn.setAttribute('aria-expanded', 'false');
      }
    });
  }

  // ── T-J: Typed-confirm cancel for orders > $50k ───────────────────
  const TYPED_CONFIRM_THRESHOLD_CENTS = 5_000_000;
  const _origOpenCancel = openCancelModal;
  openCancelModal = function (order) {
    if (heldCents(order) < TYPED_CONFIRM_THRESHOLD_CENTS) return _origOpenCancel(order);
    // High-value path — require typed order ID
    const orderId = shortId(order.id);
    const asset = order.asset_name || 'Asset';
    const side = sideLabel(order);
    const qty = formatQuantity(remainingQuantity(order));
    const price = formatMoney(order.price_cents);
    mpModal({
      title: 'Cancel HIGH-VALUE Order',
      subtitle: `Order ${orderId} · ${asset} (${side} ${qty} @ ${price}) — Held ${formatMoney(heldCents(order))}`,
      bodyNode: (function () {
        const body = document.createElement('div');
        const presetGroup = buildCancelBody(order);
        body.appendChild(presetGroup);
        const tcWrap = document.createElement('div');
        tcWrap.className = 'admin-form-group';
        tcWrap.innerHTML = `
          <label class="admin-form-label" for="cancel-typed">Type the order ID <code>${orderId}</code> to confirm *</label>
          <input class="admin-input" id="cancel-typed" autocomplete="off" placeholder="${orderId}">
        `;
        body.appendChild(tcWrap);
        return body;
      })(),
      confirmLabel: 'Cancel HIGH-VALUE Order',
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#cancel-reason')?.value?.trim();
        const typed = overlay.querySelector('#cancel-typed')?.value?.trim();
        const confirmBtn = overlay.querySelector('.mp-modal-confirm');
        if (!reason) { mpToast('Reason required', 'error'); return false; }
        if (typed !== orderId) { mpToast(`Type "${orderId}" exactly to confirm`, 'error'); return false; }
        try {
          if (confirmBtn) { confirmBtn.disabled = true; confirmBtn.textContent = 'Cancelling…'; }
          const token = csrfToken();
          const res = await fetch(`${API}/${encodeURIComponent(order.id)}`, {
            method: 'DELETE', credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
            body: JSON.stringify({ reason }),
          });
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || err.message || `HTTP ${res.status}`);
          }
          mpToast(`Order ${orderId} cancelled`, 'success');
          await loadOrders();
          return true;
        } catch (err) {
          mpToast(`Cancel failed: ${err.message}`, 'error');
          if (confirmBtn) { confirmBtn.disabled = false; confirmBtn.textContent = 'Cancel HIGH-VALUE Order'; }
          return false;
        }
      },
    });
  };

  // ── T-D: Held balance heatmap (mini distribution) ─────────────────
  function renderHeldHeatmap(orders) {
    const host = $('kpi-held-heatmap');
    if (!host) return;
    host.replaceChildren();
    if (!orders.length) return;
    const sorted = orders.slice().sort((a, b) => heldCents(b) - heldCents(a)).slice(0, 30);
    const max = Math.max(...sorted.map((o) => heldCents(o)), 1);
    sorted.forEach((o) => {
      const cell = document.createElement('span');
      cell.className = 'mp-heatcell';
      const intensity = heldCents(o) / max;
      cell.style.opacity = (0.25 + intensity * 0.75).toFixed(2);
      cell.title = `${formatMoney(heldCents(o))} — ${o.asset_name || ''}`;
      host.appendChild(cell);
    });
  }

  // ── T-C: Orderbook depth preview on Asset hover ───────────────────
  const obCache = new Map();
  let obTimer = null;
  function bindAssetHover() {
    document.addEventListener('mouseover', (e) => {
      const link = e.target.closest && e.target.closest('a.admin-link[href^="/admin/asset-details"]');
      if (!link) return;
      const url = new URL(link.href, window.location.origin);
      const assetId = url.searchParams.get('id');
      if (!assetId) return;
      clearTimeout(obTimer);
      obTimer = setTimeout(() => showOrderbookPreview(link, assetId), 350);
    }, true);
    document.addEventListener('mouseout', (e) => {
      if (e.target.closest && e.target.closest('a.admin-link[href^="/admin/asset-details"]')) {
        clearTimeout(obTimer);
        document.querySelectorAll('.mp-ob-preview').forEach((p) => p.remove());
      }
    }, true);
  }
  async function showOrderbookPreview(anchor, assetId) {
    let data = obCache.get(assetId);
    if (!data) {
      try {
        const res = await fetch(`/api/admin/marketplace/orderbook/${encodeURIComponent(assetId)}?levels=5`, { credentials: 'same-origin' });
        if (!res.ok) return;
        data = await res.json();
        obCache.set(assetId, data);
        setTimeout(() => obCache.delete(assetId), 30000);
      } catch (_) { return; }
    }
    document.querySelectorAll('.mp-ob-preview').forEach((p) => p.remove());
    const tip = document.createElement('div');
    tip.className = 'mp-ob-preview';
    const bids = (data.bids || []).slice(0, 5);
    const asks = (data.asks || []).slice(0, 5);
    tip.innerHTML = `
      <div class="mp-ob-preview-title">${data.asset?.title || 'Orderbook'}</div>
      <div class="mp-ob-preview-grid">
        <div>
          <div class="mp-ob-h">BIDS</div>
          ${bids.map((b) => `<div class="mp-ob-row mp-ob-bid">${formatMoney(b.price_cents)} <span>${b.quantity}</span></div>`).join('') || '<div class="mp-ob-empty">—</div>'}
        </div>
        <div>
          <div class="mp-ob-h">ASKS</div>
          ${asks.map((a) => `<div class="mp-ob-row mp-ob-ask">${formatMoney(a.price_cents)} <span>${a.quantity}</span></div>`).join('') || '<div class="mp-ob-empty">—</div>'}
        </div>
      </div>
    `;
    document.body.appendChild(tip);
    const r = anchor.getBoundingClientRect();
    tip.style.left = (r.left + window.scrollX) + 'px';
    tip.style.top = (r.bottom + window.scrollY + 6) + 'px';
  }

  // ── T-I: Per-user concentration view (group by user) ──────────────
  let groupByUser = false;
  function applyGroupByUser() {
    const tbody = document.getElementById('orders-body');
    if (!tbody) return;
    if (!groupByUser) {
      tbody.querySelectorAll('.mp-group-header').forEach((g) => g.remove());
      tbody.querySelectorAll('tr[data-order-id]').forEach((tr) => { tr.style.display = ''; });
      return;
    }
    // Sort orders by user_id grouping in DOM
    const rows = Array.from(tbody.querySelectorAll('tr[data-order-id]'));
    const byUser = new Map();
    rows.forEach((tr) => {
      const o = loadedOrdersCache.find((x) => x.id === tr.dataset.orderId);
      const uid = o?.user_id || 'unknown';
      if (!byUser.has(uid)) byUser.set(uid, []);
      byUser.get(uid).push({ tr, order: o });
    });
    tbody.replaceChildren();
    for (const [uid, items] of byUser.entries()) {
      const head = document.createElement('tr');
      head.className = 'mp-group-header';
      const td = document.createElement('td');
      td.colSpan = 12;
      const totalHeld = items.reduce((s, x) => s + heldCents(x.order || {}), 0);
      const email = items[0].order?.user_email || uid;
      td.innerHTML = `<strong>${email}</strong> · ${items.length} orders · ${formatMoney(totalHeld)} held`;
      head.appendChild(td);
      tbody.appendChild(head);
      items.forEach(({ tr }) => tbody.appendChild(tr));
    }
  }
  function bindGroupByUser() {
    $('orders-group-user')?.addEventListener('change', (e) => {
      groupByUser = !!e.target.checked;
      try { localStorage.setItem('mp_orders_group_user', groupByUser ? '1' : '0'); } catch (_) {}
      applyGroupByUser();
    });
    try {
      if (localStorage.getItem('mp_orders_group_user') === '1') {
        const cb = $('orders-group-user');
        if (cb) cb.checked = true;
        groupByUser = true;
      }
    } catch (_) {}
  }

  // ── T-G: Sticky table header ──────────────────────────────────────
  function applyStickyHeader() {
    const tbl = $('orders-table');
    if (tbl) tbl.classList.add('mp-sticky-header');
  }

  // ── T-L: Lifecycle timeline in detail drawer ──────────────────────
  function appendLifecycleTimeline(order) {
    const body = $('orders-drawer-body');
    if (!body) return;
    const h = document.createElement('h3');
    h.className = 'mp-drawer-section-title';
    h.textContent = 'Lifecycle';
    body.appendChild(h);
    const tl = document.createElement('ol');
    tl.className = 'mp-timeline';
    const created = new Date(order.created_at);
    const steps = [
      { label: 'Submitted', ts: created, done: true },
      { label: 'Open in book', ts: created, done: order.status !== 'cancelled' },
      { label: 'Partial fill', ts: null, done: order.quantity_filled > 0 },
      { label: 'Fully filled', ts: null, done: order.status === 'filled' },
      { label: 'Cancelled', ts: null, done: order.status === 'cancelled' || order.status === 'admin_cancelled', tone: 'danger' },
    ];
    steps.forEach((s) => {
      const li = document.createElement('li');
      li.className = 'mp-timeline-step' + (s.done ? ' done' : '') + (s.tone === 'danger' ? ' danger' : '');
      li.innerHTML = `<span class="mp-timeline-dot"></span><span class="mp-timeline-label">${s.label}</span>${s.ts ? `<span class="mp-timeline-ts">${s.ts.toISOString()}</span>` : ''}`;
      tl.appendChild(li);
    });
    body.appendChild(tl);
  }
  // Hook: after openDrawer finishes, append timeline
  const _origOpenDrawer = openDrawer;
  openDrawer = function (order) {
    _origOpenDrawer(order);
    setTimeout(() => appendLifecycleTimeline(order), 0);
  };

  // ── Wire heatmap into renderOrders ─────────────────────────────────
  const _renderOrdersWithHeatmap = renderOrders;
  renderOrders = function (orders) {
    _renderOrdersWithHeatmap(orders);
    renderHeldHeatmap(orders);
    if (groupByUser) applyGroupByUser();
  };

  document.addEventListener('DOMContentLoaded', () => {
    suppressUrlSync = true;
    restorePrefs();
    readUrlState();
    suppressUrlSync = false;

    bindTabs();
    bindFilters();
    bindPagination();
    bindSortHeaders();
    bindBulkSelection();
    bindKpiCards();
    bindSavedViews();
    bindKeys();
    bindRebuild();
    bindPalette();
    bindRowNav();
    bindDrawer();
    // Tier-2
    bindShareView();
    bindDensity();
    bindColumns();
    bindAssetHover();
    bindGroupByUser();
    applyStickyHeader();
    // Tier-3
    bindHelp();
    bindRowWindowing();
    bindWsLive();

    applySortIndicator();
    loadOrders();
    loadStats();
    loadSettlementsBadge();
  });

  // ════════════════════════════════════════════════════════════════
  // Tier-3 Features
  // ════════════════════════════════════════════════════════════════

  // ── T3-I: i18n dictionary + t() helper ────────────────────────────
  const I18N = {
    en: {
      'orders.empty.no_match':       'No orders match the current filters',
      'orders.empty.cta.clear':      'Clear filters',
      'orders.empty.cta.broaden':    'Show all statuses',
      'orders.help.title':           'Keyboard Shortcuts',
      'orders.cancel.rate_limit':    'Too many cancels — wait a moment',
      'orders.audit.no_more':        'No more entries',
      'orders.audit.next':           'Next →',
      'orders.audit.prev':           '← Prev',
      'orders.diff.title':           'Changes',
      'orders.diff.no_change':       'No diff available',
      'orders.ws.live':              'LIVE',
      'orders.ws.offline':           'POLLING',
      'orders.premium.over':         'over mid',
      'orders.premium.under':        'under mid',
    },
  };
  const I18N_LANG = (navigator.language || 'en').slice(0, 2);
  function t(key, fallback) {
    return (I18N[I18N_LANG] && I18N[I18N_LANG][key])
      || I18N.en[key]
      || fallback
      || key;
  }
  // Apply to known string sites — wrap renderStateRow
  const _origRenderStateRow = renderStateRow;
  renderStateRow = function (message, tone) {
    if (message === 'No orders match the current filters') {
      const tbody = clearTable();
      if (!tbody) return;
      const row = document.createElement('tr');
      const cell = document.createElement('td');
      cell.colSpan = 12;
      cell.style.textAlign = 'center';
      cell.style.padding = '32px 24px';
      cell.style.color = 'var(--admin-text-muted)';
      cell.innerHTML = `
        <div style="font-size:14px; margin-bottom:14px;">${t('orders.empty.no_match')}</div>
        <div style="display:inline-flex; gap:8px;">
          <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" id="orders-empty-clear">${t('orders.empty.cta.clear')}</button>
          <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" id="orders-empty-broaden">${t('orders.empty.cta.broaden')}</button>
        </div>
      `;
      row.appendChild(cell);
      tbody.appendChild(row);
      cell.querySelector('#orders-empty-clear')?.addEventListener('click', () => {
        state.q = ''; state.side = ''; state.page = 1;
        const s = $('orders-search'); if (s) s.value = '';
        const sd = $('orders-side'); if (sd) sd.value = '';
        loadOrders();
      });
      cell.querySelector('#orders-empty-broaden')?.addEventListener('click', () => {
        state.status = 'open,partially_filled,filled,cancelled,admin_cancelled';
        document.querySelectorAll('.admin-tab').forEach((tt) => {
          const isMatch = (tt.dataset.status || '') === state.status;
          tt.classList.toggle('active', isMatch);
          tt.setAttribute('aria-selected', isMatch ? 'true' : 'false');
        });
        state.page = 1;
        loadOrders();
      });
      return;
    }
    return _origRenderStateRow(message, tone);
  };

  // ── T3-B: Frontend rate-limit guard on cancel actions ─────────────
  const cancelRateBucket = { tokens: 20, last: Date.now() };
  function consumeCancelToken() {
    const now = Date.now();
    const elapsed = (now - cancelRateBucket.last) / 1000;
    cancelRateBucket.tokens = Math.min(20, cancelRateBucket.tokens + elapsed * (20 / 60)); // refill 20/min
    cancelRateBucket.last = now;
    if (cancelRateBucket.tokens < 1) return false;
    cancelRateBucket.tokens -= 1;
    return true;
  }
  // Wrap openCancelModal + bulk
  const _origOpenCancel2 = openCancelModal;
  openCancelModal = function (order) {
    if (!consumeCancelToken()) {
      mpToast(t('orders.cancel.rate_limit'), 'error');
      return;
    }
    return _origOpenCancel2(order);
  };
  document.addEventListener('click', (e) => {
    if (e.target?.id === 'orders-bulk-cancel') {
      if (!consumeCancelToken()) {
        e.stopImmediatePropagation();
        e.preventDefault();
        mpToast(t('orders.cancel.rate_limit'), 'error');
      }
    }
  }, true);

  // ── T3-E + T3-G: Audit log pagination + diff view in drawer ───────
  const auditState = { page: 1, perPage: 5, orderId: null };
  function renderAuditDiff(prev, next) {
    if (!prev && !next) return `<div class="mp-meta">${t('orders.diff.no_change')}</div>`;
    const keys = Array.from(new Set([...Object.keys(prev || {}), ...Object.keys(next || {})]));
    const rows = keys
      .filter((k) => JSON.stringify(prev?.[k]) !== JSON.stringify(next?.[k]))
      .map((k) => `
        <tr>
          <th>${k}</th>
          <td class="mp-diff-old">${prev?.[k] !== undefined ? JSON.stringify(prev[k]) : '∅'}</td>
          <td class="mp-diff-new">${next?.[k] !== undefined ? JSON.stringify(next[k]) : '∅'}</td>
        </tr>`).join('');
    if (!rows) return `<div class="mp-meta">${t('orders.diff.no_change')}</div>`;
    return `<table class="mp-diff-tbl"><thead><tr><th></th><th>before</th><th>after</th></tr></thead><tbody>${rows}</tbody></table>`;
  }
  async function loadAuditPage(orderId, container) {
    const offset = (auditState.page - 1) * auditState.perPage;
    container.innerHTML = `<div class="mp-meta">Loading audit page ${auditState.page}…</div>`;
    try {
      const res = await fetch(`/api/admin/audit-logs?entity_type=market_order&entity_id=${encodeURIComponent(orderId)}&per_page=${auditState.perPage}&offset=${offset}`, { credentials: 'same-origin' });
      const data = res.ok ? await res.json() : null;
      const items = data?.data || [];
      if (!items.length) {
        container.innerHTML = `<div class="mp-meta">${auditState.page === 1 ? 'No audit entries found.' : t('orders.audit.no_more')}</div>`;
        return;
      }
      container.innerHTML = '';
      items.forEach((e) => {
        const card = document.createElement('details');
        card.className = 'mp-drawer-audit-row mp-audit-card';
        const summary = document.createElement('summary');
        summary.innerHTML = `<strong>${e.action || ''}</strong> <span class="mp-meta">${e.created_at || ''}${e.actor_email ? ' · ' + e.actor_email : ''}</span>`;
        card.appendChild(summary);
        const diff = document.createElement('div');
        diff.className = 'mp-audit-diff';
        diff.innerHTML = `<div class="mp-drawer-section-title">${t('orders.diff.title')}</div>${renderAuditDiff(e.previous_state, e.new_state)}`;
        card.appendChild(diff);
        container.appendChild(card);
      });
      // Pagination row
      const ctrls = document.createElement('div');
      ctrls.className = 'mp-audit-pager';
      ctrls.innerHTML = `
        <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" data-audit-prev ${auditState.page <= 1 ? 'disabled' : ''}>${t('orders.audit.prev')}</button>
        <span class="mp-meta">page ${auditState.page}</span>
        <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" data-audit-next ${items.length < auditState.perPage ? 'disabled' : ''}>${t('orders.audit.next')}</button>
      `;
      ctrls.querySelector('[data-audit-prev]')?.addEventListener('click', () => {
        if (auditState.page > 1) { auditState.page--; loadAuditPage(orderId, container); }
      });
      ctrls.querySelector('[data-audit-next]')?.addEventListener('click', () => {
        auditState.page++; loadAuditPage(orderId, container);
      });
      container.appendChild(ctrls);
    } catch (_) {
      container.innerHTML = `<div class="mp-meta">Audit log unavailable.</div>`;
    }
  }
  // Replace existing one-shot audit fetch in drawer
  const _origOpenDrawer3 = openDrawer;
  openDrawer = function (order) {
    auditState.page = 1; auditState.orderId = order.id;
    _origOpenDrawer3(order);
    setTimeout(() => {
      const audit = document.querySelector('.mp-drawer-audit');
      if (audit) loadAuditPage(order.id, audit);
    }, 10);
  };

  // ── T3-A: WS live updates when single asset filtered ──────────────
  let wsAssetId = null;
  function inferSingleAsset() {
    // Heuristic: if all rendered orders share asset_id, treat as filtered
    if (!loadedOrdersCache.length) return null;
    const first = loadedOrdersCache[0].asset_id;
    return loadedOrdersCache.every((o) => o.asset_id === first) ? first : null;
  }
  function bindWsLive() {
    if (!window.MarketWS || !window.MarketBus) return;
    // Subscribe / resubscribe whenever orders re-render
    const _origRender2 = renderOrders;
    renderOrders = function (orders) {
      _origRender2(orders);
      const single = inferSingleAsset();
      if (single && single !== wsAssetId) {
        try { window.MarketWS.disconnect(); } catch (_) {}
        wsAssetId = single;
        window.MarketWS.connect(single);
      } else if (!single && wsAssetId) {
        try { window.MarketWS.disconnect(); } catch (_) {}
        wsAssetId = null;
      }
      updateLiveBadge();
    };
    // Update KPI value bg on trade events
    window.MarketBus.on('trade', () => {
      // Trigger fast refresh on trade (within 1s)
      setTimeout(() => loadOrders(), 800);
    });
    window.MarketBus.on('orderbook:update', () => {
      // Cheap: pulse last-updated indicator
      const el = $('orders-last-updated');
      if (el) {
        el.classList.add('mp-pulse');
        setTimeout(() => el.classList.remove('mp-pulse'), 600);
      }
    });
  }
  function updateLiveBadge() {
    const el = $('orders-last-updated');
    if (!el) return;
    const live = wsAssetId && window.MarketWS && window.MarketWS.getState() === 'open';
    let badge = el.querySelector('.mp-live-badge');
    if (!badge) {
      badge = document.createElement('span');
      badge.className = 'mp-live-badge';
      el.prepend(badge);
    }
    badge.textContent = live ? t('orders.ws.live') : t('orders.ws.offline');
    badge.dataset.state = live ? 'live' : 'poll';
  }

  // ── T3-F: Row windowing via IntersectionObserver + content-visibility
  let rowObserver = null;
  function bindRowWindowing() {
    if (!('IntersectionObserver' in window)) return;
    rowObserver = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        const tr = entry.target;
        if (entry.isIntersecting) tr.classList.remove('mp-row-offscreen');
        else tr.classList.add('mp-row-offscreen');
      });
    }, { root: null, rootMargin: '200px 0px', threshold: 0 });
    // Wrap renderRow to observe each appended row
    const _origRenderRow = renderRow;
    renderRow = function (order) {
      const tr = _origRenderRow(order);
      if (rowObserver) rowObserver.observe(tr);
      return tr;
    };
  }

  // ── Help overlay ───────────────────────────────────────────────────
  function openHelp() {
    const h = $('orders-help');
    if (h) { h.hidden = false; document.body.classList.add('mp-no-scroll'); }
  }
  function closeHelp() {
    const h = $('orders-help');
    if (h) { h.hidden = true; document.body.classList.remove('mp-no-scroll'); }
  }
  function bindHelp() {
    document.querySelectorAll('[data-help-close]').forEach((el) => el.addEventListener('click', closeHelp));
    document.addEventListener('keydown', (e) => {
      const isInput = e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable);
      if (e.key === '?' && !isInput) { e.preventDefault(); openHelp(); }
    });
  }

  // ── Premium/discount price indicator vs asset mid (lazy fetch) ────
  const midCache = new Map();
  async function getAssetMid(assetId) {
    if (midCache.has(assetId)) return midCache.get(assetId);
    try {
      const res = await fetch(`/api/admin/marketplace/orderbook/${encodeURIComponent(assetId)}?levels=1`, { credentials: 'same-origin' });
      const data = res.ok ? await res.json() : null;
      const bid = data?.bids?.[0]?.price_cents;
      const ask = data?.asks?.[0]?.price_cents;
      const mid = (bid && ask) ? (bid + ask) / 2 : (bid || ask || null);
      midCache.set(assetId, mid);
      setTimeout(() => midCache.delete(assetId), 30000);
      return mid;
    } catch (_) { return null; }
  }
  // After render, decorate each row's price cell with premium badge
  const _origRenderOrders = renderOrders;
  renderOrders = function (orders) {
    _origRenderOrders(orders);
    // Group by asset → fetch mid once per asset
    const byAsset = new Map();
    orders.forEach((o) => {
      if (!o.asset_id) return;
      if (!byAsset.has(o.asset_id)) byAsset.set(o.asset_id, []);
      byAsset.get(o.asset_id).push(o);
    });
    byAsset.forEach(async (group, assetId) => {
      const mid = await getAssetMid(assetId);
      if (!mid) return;
      group.forEach((o) => {
        const tr = document.querySelector(`tr[data-order-id="${o.id}"]`);
        if (!tr) return;
        const priceCell = tr.querySelectorAll('td')[7]; // Price column index after checkbox+id+user+asset+side+type+qty
        if (!priceCell) return;
        const diffPct = ((o.price_cents - mid) / mid) * 100;
        if (Math.abs(diffPct) < 0.5) return;
        const badge = document.createElement('span');
        badge.className = 'mp-premium ' + (diffPct > 0 ? 'mp-premium--up' : 'mp-premium--down');
        const word = diffPct > 0 ? t('orders.premium.over') : t('orders.premium.under');
        badge.textContent = `${diffPct > 0 ? '+' : ''}${diffPct.toFixed(1)}%`;
        badge.title = `${Math.abs(diffPct).toFixed(2)}% ${word} ${formatMoney(mid)}`;
        priceCell.appendChild(document.createElement('br'));
        priceCell.appendChild(badge);
      });
    });
  };

  // ════════════════════════════════════════════════════════════════
  // Tier-4 Features
  // ════════════════════════════════════════════════════════════════

  // ── T4-F: Audit search/filter inside drawer ───────────────────────
  // Inject filter input above audit list, send `q`/`actor` params to backend.
  const _origLoadAuditPage = loadAuditPage;
  loadAuditPage = async function (orderId, container) {
    if (!container.dataset.auditEnhanced) {
      container.dataset.auditEnhanced = '1';
      const filter = document.createElement('div');
      filter.className = 'mp-audit-filter';
      filter.innerHTML = `
        <input type="search" class="admin-input admin-input--sm" placeholder="Filter actions… (e.g. cancel)" id="mp-audit-q" style="margin-bottom:6px; width:100%;">
      `;
      container.parentElement?.insertBefore(filter, container);
      const inp = filter.querySelector('#mp-audit-q');
      let dt;
      inp.addEventListener('input', () => {
        clearTimeout(dt);
        dt = setTimeout(() => {
          auditState.page = 1;
          auditState.q = inp.value.trim();
          _origLoadAuditPage(orderId, container);
        }, 200);
      });
    }
    return _origLoadAuditPage(orderId, container);
  };
  // Augment fetch URL with q param when present
  const _origFetch = window.fetch;
  window.fetch = function (url, opts) {
    if (typeof url === 'string' && url.startsWith('/api/admin/audit-logs?') && auditState.q) {
      url = url + `&q=${encodeURIComponent(auditState.q)}`;
    }
    return _origFetch.call(this, url, opts);
  };

  // ── T4-G: Bulk-cancel preflight (preview before confirm) ──────────
  const _origOpenBulkCancel = openBulkCancelModal;
  openBulkCancelModal = function () {
    const ids = Array.from(state.selectedIds);
    if (!ids.length) { mpToast('No orders selected', 'error'); return; }
    const orders = loadedOrdersCache.filter((o) => ids.includes(o.id));
    if (!orders.length) return _origOpenBulkCancel();

    // Aggregate per-asset summary
    const totalHeld = orders.reduce((s, o) => s + heldCents(o), 0);
    const buyN = orders.filter((o) => o.side === 'buy').length;
    const sellN = orders.filter((o) => o.side === 'sell').length;
    const userN = new Set(orders.map((o) => o.user_id)).size;
    const assetN = new Set(orders.map((o) => o.asset_id)).size;
    const flagged = orders.filter((o) => anomalyFlags(o).length).length;

    const body = document.createElement('div');
    body.innerHTML = `
      <div class="mp-preflight-summary">
        <h4>Preflight Summary</h4>
        <ul>
          <li><strong>${orders.length}</strong> orders selected — ${buyN} BUY · ${sellN} SELL</li>
          <li>Across <strong>${userN}</strong> user(s) and <strong>${assetN}</strong> asset(s)</li>
          <li>Total held to release: <strong>${formatMoney(totalHeld)}</strong></li>
          ${flagged ? `<li class="mp-preflight-warn">⚠ <strong>${flagged}</strong> flagged for anomaly — review first</li>` : ''}
        </ul>
        <details class="mp-preflight-details">
          <summary>Show ${orders.length} order IDs</summary>
          <pre>${orders.map((o) => `${shortId(o.id)} · ${o.side.toUpperCase()} ${o.quantity} ${o.asset_name || ''} — ${formatMoney(heldCents(o))}`).join('\n')}</pre>
        </details>
      </div>
      <hr style="border:0; border-top:1px solid var(--admin-border); margin:12px 0;">
    `;
    const reasonGroup = buildBulkCancelBody(orders.length);
    body.appendChild(reasonGroup);

    mpModal({
      title: `Cancel ${orders.length} Orders — Preflight`,
      subtitle: 'Review aggregate impact before committing',
      bodyNode: body,
      confirmLabel: `Confirm Cancel ${orders.length}`,
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#bulk-cancel-reason')?.value?.trim();
        const confirmBtn = overlay.querySelector('.mp-modal-confirm');
        if (!reason) { mpToast('Please provide a cancellation reason', 'error'); return false; }
        try {
          if (confirmBtn) { confirmBtn.disabled = true; confirmBtn.textContent = 'Cancelling…'; }
          const token = csrfToken();
          const res = await fetch(`${API}/bulk-cancel`, {
            method: 'POST', credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
            body: JSON.stringify({ order_ids: ids, reason }),
          });
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || err.message || `HTTP ${res.status}`);
          }
          const data = await res.json();
          mpToast(`Cancelled ${data.succeeded_count}, failed ${data.failed_count}`, data.failed_count ? 'warning' : 'success');
          state.selectedIds.clear();
          await loadOrders();
          return true;
        } catch (err) {
          mpToast(`Bulk cancel failed: ${err.message}`, 'error');
          if (confirmBtn) { confirmBtn.disabled = false; confirmBtn.textContent = `Confirm Cancel ${orders.length}`; }
          return false;
        }
      },
    });
  };

  // ── T4-C: Saved views server-sync (with localStorage fallback) ────
  const VIEWS_API = '/api/admin/marketplace/saved-views';
  const VIEWS_SCOPE = 'marketplace_orders';
  let serverViewsAvailable = true;
  async function loadServerViews() {
    try {
      const res = await fetch(`${VIEWS_API}?scope=${VIEWS_SCOPE}`, { credentials: 'same-origin' });
      if (!res.ok) { serverViewsAvailable = false; return null; }
      return await res.json();
    } catch (_) { serverViewsAvailable = false; return null; }
  }
  async function saveServerView(name, preset) {
    if (!serverViewsAvailable) return false;
    try {
      const token = csrfToken();
      const res = await fetch(VIEWS_API, {
        method: 'POST', credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', ...(token ? { 'X-CSRF-Token': token } : {}) },
        body: JSON.stringify({ scope: VIEWS_SCOPE, name, preset }),
      });
      return res.ok;
    } catch (_) { return false; }
  }
  async function deleteServerView(id) {
    if (!serverViewsAvailable) return false;
    try {
      const token = csrfToken();
      const res = await fetch(`${VIEWS_API}/${encodeURIComponent(id)}`, {
        method: 'DELETE', credentials: 'same-origin',
        headers: token ? { 'X-CSRF-Token': token } : {},
      });
      return res.ok;
    } catch (_) { return false; }
  }
  // Augment renderCustomViews to merge server views
  const _origRenderCustomViews = renderCustomViews;
  renderCustomViews = async function () {
    _origRenderCustomViews();
    const host = document.getElementById('orders-saved-views-custom');
    if (!host) return;
    const server = await loadServerViews();
    if (!server || !server.length) return;
    server.forEach((v) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'mp-chip mp-chip--custom mp-chip--server';
      chip.textContent = v.name;
      chip.title = `Server-saved: ${v.name}`;
      chip.addEventListener('click', () => applyPreset(v.preset));
      const del = document.createElement('span');
      del.className = 'mp-chip-del';
      del.textContent = '×';
      del.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (await deleteServerView(v.id)) renderCustomViews();
      });
      chip.appendChild(del);
      host.appendChild(chip);
    });
  };
  // Augment save action to also push to server
  document.addEventListener('click', async (e) => {
    if (e.target?.id === 'orders-save-view') {
      // Wait one tick for prompt to fire then sync from localStorage to server
      setTimeout(async () => {
        try {
          const arr = JSON.parse(localStorage.getItem('mp_orders_views') || '[]');
          const last = arr[arr.length - 1];
          if (last) {
            const ok = await saveServerView(last.name, last.preset);
            if (ok) renderCustomViews();
          }
        } catch (_) {}
      }, 100);
    }
  });
  // Auto-load server views on init
  setTimeout(() => renderCustomViews(), 200);
})();
