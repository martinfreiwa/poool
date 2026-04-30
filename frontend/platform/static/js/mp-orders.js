/**
 * Open Orders - live admin order list with audited cancel action.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/orders';
  const PAGE_SIZE = 25;
  let currentPage = 1;
  let totalPages = 1;
  let totalOrders = 0;
  let ordersData = [];
  let isLoading = false;

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  }

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

  function timeAgo(dateStr) {
    const timestamp = new Date(dateStr).getTime();
    if (!Number.isFinite(timestamp)) return '-';

    const diff = Math.max(0, (Date.now() - timestamp) / 1000);
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  function csrfToken() {
    return document.cookie
      .split(';')
      .map((part) => part.trim())
      .find((part) => part.startsWith('csrf_token='))
      ?.split('=')
      .slice(1)
      .join('=') || '';
  }

  function clearTable() {
    const tbody = document.getElementById('orders-body');
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

  function appendTextCell(row, text, options = {}) {
    const cell = document.createElement('td');
    cell.textContent = text;
    if (options.alignRight) cell.style.textAlign = 'right';
    if (options.muted) {
      cell.style.fontSize = '12px';
      cell.style.color = 'var(--admin-text-muted)';
    }
    if (options.bold) {
      cell.style.fontWeight = '600';
      cell.style.color = 'var(--admin-text-primary)';
    }
    if (options.numeric) cell.style.fontVariantNumeric = 'tabular-nums';
    row.appendChild(cell);
    return cell;
  }

  function appendCodeCell(row, text) {
    const cell = document.createElement('td');
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = text;
    cell.appendChild(code);
    row.appendChild(cell);
  }

  function appendBadgeCell(row, badge, options = {}) {
    const cell = document.createElement('td');
    if (options.alignRight) cell.style.textAlign = 'right';
    cell.appendChild(badge);
    row.appendChild(cell);
  }

  function updateKpis(orders) {
    const totalHeld = orders.reduce((sum, order) => sum + heldCents(order), 0);
    const now = Date.now();
    const ages = orders
      .map((order) => (now - new Date(order.created_at).getTime()) / 3600000)
      .filter(Number.isFinite);
    const avgAge = ages.length
      ? (ages.reduce((sum, age) => sum + age, 0) / ages.length).toFixed(1) + 'h'
      : '0h';

    setText('kpi-total-open', totalOrders.toLocaleString());
    setText('kpi-held-balance', formatMoney(totalHeld));
    setText('kpi-avg-age', avgAge);
  }

  function updatePagination() {
    const info = document.getElementById('orders-page-info');
    const prev = document.getElementById('orders-prev-page');
    const next = document.getElementById('orders-next-page');
    const pageCount = Math.max(totalPages, 1);

    if (info) {
      info.textContent = `${totalOrders.toLocaleString()} orders - page ${currentPage} of ${pageCount}`;
    }
    if (prev) prev.disabled = isLoading || currentPage <= 1;
    if (next) next.disabled = isLoading || currentPage >= pageCount;
  }

  function renderOrders(orders) {
    ordersData = Array.isArray(orders) ? orders : [];
    updateKpis(ordersData);
    updatePagination();

    const tbody = clearTable();
    if (!tbody) return;

    if (ordersData.length === 0) {
      renderStateRow('No open orders', 'empty');
      return;
    }

    ordersData.forEach((order, idx) => {
      const row = document.createElement('tr');
      row.dataset.orderIdx = String(idx);
      row.dataset.orderId = order.id;

      appendCodeCell(row, shortId(order.id));
      appendCodeCell(row, userLabel(order));
      appendTextCell(row, order.asset_name || shortId(order.asset_id), { bold: true });

      const side = sideLabel(order);
      const sideCell = document.createElement('td');
      const sideSpan = document.createElement('span');
      sideSpan.className = side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      sideSpan.textContent = side;
      sideCell.appendChild(sideSpan);
      row.appendChild(sideCell);

      appendBadgeCell(row, renderBadge(typeLabel(order), 'admin-badge--neutral'));
      appendTextCell(row, formatQuantity(order.quantity), { alignRight: true, numeric: true });
      appendTextCell(row, formatMoney(order.price_cents), { alignRight: true, numeric: true });
      appendBadgeCell(row, renderBadge(formatMoney(heldCents(order)), 'admin-badge--warning'), { alignRight: true });
      appendTextCell(row, timeAgo(order.created_at), { muted: true });

      const statusBadge = order.status === 'partially_filled'
        ? renderBadge('Partial', 'admin-badge--warning')
        : renderBadge('Open', 'admin-badge--info');
      appendBadgeCell(row, statusBadge);

      const actionCell = document.createElement('td');
      actionCell.style.textAlign = 'center';
      const cancelBtn = document.createElement('button');
      cancelBtn.type = 'button';
      cancelBtn.className = 'admin-btn admin-btn--danger admin-btn--sm btn-cancel-order';
      cancelBtn.textContent = 'Cancel Order';
      cancelBtn.addEventListener('click', () => openCancelModal(order));
      actionCell.appendChild(cancelBtn);
      row.appendChild(actionCell);

      tbody.appendChild(row);
    });
  }

  function buildCancelBody(order) {
    const body = document.createElement('div');

    const group = document.createElement('div');
    group.className = 'admin-form-group';

    const label = document.createElement('label');
    label.className = 'admin-form-label';
    label.setAttribute('for', 'cancel-reason');
    label.textContent = 'Reason for Cancellation *';

    const textarea = document.createElement('textarea');
    textarea.className = 'admin-textarea';
    textarea.id = 'cancel-reason';
    textarea.placeholder = 'Enter the legal reason for this cancellation...';
    textarea.rows = 3;
    textarea.style.minHeight = '80px';

    group.append(label, textarea);
    body.appendChild(group);

    const warning = document.createElement('div');
    warning.style.display = 'flex';
    warning.style.alignItems = 'center';
    warning.style.gap = '8px';
    warning.style.padding = '10px 14px';
    warning.style.background = 'var(--admin-danger-bg)';
    warning.style.borderRadius = 'var(--admin-radius-sm)';
    warning.style.marginTop = '8px';

    const warningText = document.createElement('span');
    warningText.style.color = 'var(--admin-danger)';
    warningText.style.fontSize = '13px';
    warningText.style.fontWeight = '500';
    const heldText = sideLabel(order) === 'BUY'
      ? `${formatMoney(heldCents(order))} from the user's wallet hold`
      : `${formatQuantity(remainingQuantity(order))} held tokens`;
    warningText.textContent = `This will release ${heldText}.`;
    warning.appendChild(warningText);
    body.appendChild(warning);

    return body;
  }

  function openCancelModal(order) {
    const orderId = shortId(order.id);
    const asset = order.asset_name || 'Asset';
    const side = sideLabel(order);
    const qty = formatQuantity(order.quantity);
    const price = formatMoney(order.price_cents);

    mpModal({
      title: 'Cancel Order',
      subtitle: `Order ${orderId} - ${asset} (${side} ${qty} @ ${price})`,
      bodyNode: buildCancelBody(order),
      confirmLabel: 'Cancel Order',
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#cancel-reason')?.value?.trim();
        const confirmBtn = overlay.querySelector('.mp-modal-confirm');
        if (!reason) {
          mpToast('Please provide a cancellation reason', 'error');
          return false;
        }

        try {
          if (confirmBtn) {
            confirmBtn.disabled = true;
            confirmBtn.textContent = 'Cancelling...';
          }
          const token = csrfToken();
          const res = await fetch(`${API}/${encodeURIComponent(order.id)}`, {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: {
              'Content-Type': 'application/json',
              ...(token ? { 'X-CSRF-Token': token } : {}),
            },
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
          if (confirmBtn) {
            confirmBtn.disabled = false;
            confirmBtn.textContent = 'Cancel Order';
          }
          return false;
        }
      }
    });
  }

  async function loadOrders() {
    if (isLoading) return;
    isLoading = true;
    updatePagination();
    renderStateRow('Loading orders...', 'empty');

    try {
      const res = await fetch(`${API}?page=${currentPage}&per_page=${PAGE_SIZE}`, { credentials: 'same-origin' });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || err.message || `HTTP ${res.status}`);
      }
      const data = await res.json();
      totalOrders = Number(data.total || 0);
      totalPages = Math.max(Number(data.total_pages || 0), 1);
      if (currentPage > totalPages) currentPage = totalPages;
      renderOrders(data.data || []);
    } catch (err) {
      console.warn('[mp-orders] Failed to load live order data:', err);
      ordersData = [];
      totalOrders = 0;
      totalPages = 1;
      setText('kpi-total-open', '-');
      setText('kpi-held-balance', '-');
      setText('kpi-avg-age', '-');
      renderStateRow(`Could not load open orders: ${err.message}`, 'error');
      mpToast('Could not load open orders', 'error');
    } finally {
      isLoading = false;
      updatePagination();
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('orders-prev-page')?.addEventListener('click', () => {
      if (currentPage <= 1) return;
      currentPage -= 1;
      loadOrders();
    });
    document.getElementById('orders-next-page')?.addEventListener('click', () => {
      if (currentPage >= totalPages) return;
      currentPage += 1;
      loadOrders();
    });
    loadOrders();
  });
})();
