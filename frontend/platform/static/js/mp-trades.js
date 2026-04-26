/**
 * Trade History — mp-trades.js
 * Fetches executed trades from the backend API with filtering and pagination.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/trades';
  const ASSETS_API = '/api/admin/marketplace/trades/assets';
  const EXPORT_API = '/api/admin/marketplace/trades/export.csv';
  const PAGE_SIZE = 15;

  let currentPage = 1;
  let totalPages = 1;
  let totalTrades = 0;

  function $(id) {
    return document.getElementById(id);
  }

  function cents(value) {
    const amount = Number(value || 0) / 100;
    return amount.toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  }

  function compactId(value) {
    return value ? String(value).slice(0, 8) : '—';
  }

  function userLabel(email, id) {
    if (email) return String(email).split('@')[0];
    return compactId(id);
  }

  function formatDate(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '—';
    return date.toLocaleString('en-US', { dateStyle: 'short', timeStyle: 'short' });
  }

  function statusMeta(status) {
    const map = {
      confirmed: { cls: 'admin-badge--success', label: 'Confirmed' },
      pending: { cls: 'admin-badge--warning', label: 'Pending' },
      submitted: { cls: 'admin-badge--info', label: 'Submitted' },
      failed: { cls: 'admin-badge--danger', label: 'Failed' },
    };
    return map[status] || { cls: 'admin-badge--warning', label: status || 'Unknown' };
  }

  function appendTextCell(row, text, className) {
    const cell = document.createElement('td');
    if (className) cell.className = className;
    cell.textContent = text;
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
    return cell;
  }

  function appendMoneyCell(row, text, { muted = false, strong = false } = {}) {
    const cell = appendTextCell(row, `$${text}`);
    cell.style.textAlign = 'right';
    cell.style.fontVariantNumeric = 'tabular-nums';
    if (muted) cell.style.color = 'var(--admin-text-muted)';
    if (strong) cell.style.fontWeight = '600';
    return cell;
  }

  function appendStatusCell(row, status) {
    const cell = document.createElement('td');
    const meta = statusMeta(status);
    const badge = document.createElement('span');
    badge.className = `admin-badge ${meta.cls}`;
    const dot = document.createElement('span');
    dot.className = 'admin-badge-dot';
    badge.appendChild(dot);
    badge.appendChild(document.createTextNode(meta.label));
    cell.appendChild(badge);
    row.appendChild(cell);
  }

  function renderMessage(message, { error = false } = {}) {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.replaceChildren();
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 11;
    cell.style.textAlign = 'center';
    cell.style.color = error ? 'var(--admin-danger)' : 'var(--admin-text-muted)';
    cell.style.padding = '24px';
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function renderError(message) {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.replaceChildren();

    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 11;
    cell.style.textAlign = 'center';
    cell.style.color = 'var(--admin-danger)';
    cell.style.padding = '24px';

    const text = document.createElement('div');
    text.textContent = message;
    const retry = document.createElement('button');
    retry.type = 'button';
    retry.className = 'admin-btn admin-btn--secondary admin-btn--sm';
    retry.style.marginTop = '12px';
    retry.textContent = 'Retry';
    retry.addEventListener('click', loadTrades);

    cell.append(text, retry);
    row.appendChild(cell);
    tbody.appendChild(row);

    const pagination = $('pagination');
    if (pagination) pagination.replaceChildren();
  }

  function renderTrades(trades) {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.replaceChildren();

    if (!Array.isArray(trades) || trades.length === 0) {
      renderMessage('No trades found');
      return;
    }

    const fragment = document.createDocumentFragment();
    trades.forEach((trade) => {
      const row = document.createElement('tr');
      row.dataset.tradeId = trade.id || '';
      row.dataset.assetId = trade.asset_id || '';

      appendCodeCell(row, compactId(trade.id));

      const dateCell = appendTextCell(row, formatDate(trade.executed_at));
      dateCell.style.fontVariantNumeric = 'tabular-nums';
      dateCell.style.fontSize = '12px';
      dateCell.style.color = 'var(--admin-text-muted)';

      const assetCell = appendTextCell(row, trade.asset_name || compactId(trade.asset_id));
      assetCell.style.fontWeight = '600';
      assetCell.style.color = 'var(--admin-text-primary)';

      const sideCell = appendTextCell(row, 'TRADE', 'mp-side-buy');
      sideCell.setAttribute('aria-label', 'Executed trade');

      appendCodeCell(row, userLabel(trade.buyer_email, trade.buyer_id));
      appendCodeCell(row, userLabel(trade.seller_email, trade.seller_id));

      const quantityCell = appendTextCell(row, Number(trade.quantity || 0).toLocaleString('en-US'));
      quantityCell.style.textAlign = 'right';
      quantityCell.style.fontVariantNumeric = 'tabular-nums';

      appendMoneyCell(row, cents(trade.price_cents));
      appendMoneyCell(row, cents(trade.fee_cents), { muted: true });
      appendMoneyCell(row, cents(trade.total_cents), { strong: true });
      appendStatusCell(row, trade.on_chain_status);

      fragment.appendChild(row);
    });
    tbody.appendChild(fragment);
  }

  function filterParams({ includePagination = true } = {}) {
    const params = new URLSearchParams();
    if (includePagination) {
      params.set('page', String(currentPage));
      params.set('per_page', String(PAGE_SIZE));
    }

    const fromDate = $('filter-start')?.value;
    const toDate = $('filter-end')?.value;
    const assetId = $('filter-asset')?.value;
    const status = $('filter-status')?.value;

    if (fromDate) params.set('from_date', fromDate);
    if (toDate) params.set('to_date', toDate);
    if (assetId) params.set('asset_id', assetId);
    if (status) params.set('on_chain_status', status);

    return params;
  }

  async function loadTradeAssets() {
    const select = $('filter-asset');
    if (!select) return;

    try {
      const res = await fetch(ASSETS_API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const assets = await res.json();
      assets.forEach((asset) => {
        const option = document.createElement('option');
        option.value = asset.id;
        option.textContent = `${asset.title} (${asset.trade_count})`;
        select.appendChild(option);
      });
    } catch (err) {
      console.warn('[mp-trades] Unable to load trade assets:', err);
      if (typeof mpToast === 'function') mpToast('Asset filter unavailable', 'warning');
    }
  }

  function renderPagination() {
    const pagination = $('pagination');
    if (!pagination) return;
    pagination.replaceChildren();

    const previous = document.createElement('button');
    previous.className = 'mp-pagination-btn';
    previous.id = 'pg-prev';
    previous.disabled = currentPage <= 1;
    previous.textContent = 'Previous';
    previous.addEventListener('click', () => {
      currentPage = Math.max(1, currentPage - 1);
      loadTrades();
    });

    const info = document.createElement('span');
    info.className = 'mp-pagination-info';
    info.textContent = `Page ${currentPage} of ${totalPages} (${totalTrades} trades)`;

    const next = document.createElement('button');
    next.className = 'mp-pagination-btn';
    next.id = 'pg-next';
    next.disabled = currentPage >= totalPages;
    next.textContent = 'Next';
    next.addEventListener('click', () => {
      currentPage += 1;
      loadTrades();
    });

    pagination.append(previous, info, next);
  }

  async function loadTrades() {
    renderMessage('Loading trades...');
    try {
      const res = await fetch(`${API}?${filterParams().toString()}`, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      totalTrades = Number(data.total || 0);
      totalPages = Number(data.total_pages || 1) || 1;
      currentPage = Number(data.page || currentPage);
      renderTrades(data.data);
      renderPagination();
    } catch (err) {
      console.warn('[mp-trades] Trade history failed:', err);
      totalTrades = 0;
      totalPages = 1;
      renderError('Unable to load trade history. Try again after refreshing.');
    }
  }

  async function exportCsv() {
    const button = $('btn-export-csv');
    const params = filterParams({ includePagination: false });
    const originalText = button?.textContent;
    if (button) {
      button.disabled = true;
      button.textContent = 'Exporting...';
    }
    try {
      const res = await fetch(`${EXPORT_API}?${params.toString()}`, { credentials: 'same-origin' });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || `HTTP ${res.status}`);
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = 'marketplace_trades.csv';
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      if (typeof mpToast === 'function') mpToast('CSV export downloaded', 'success');
    } catch (err) {
      console.warn('[mp-trades] CSV export failed:', err);
      if (typeof mpToast === 'function') mpToast(err.message || 'CSV export failed', 'error');
    } finally {
      if (button) {
        button.disabled = false;
        button.textContent = originalText || 'CSV';
      }
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    loadTradeAssets();
    loadTrades();

    $('btn-apply-filter')?.addEventListener('click', () => {
      currentPage = 1;
      loadTrades();
    });

    $('btn-export-csv')?.addEventListener('click', exportCsv);
    const pdfButton = $('btn-export-pdf');
    if (pdfButton) {
      pdfButton.disabled = true;
      pdfButton.setAttribute('aria-disabled', 'true');
      pdfButton.title = 'PDF export is not available yet. Use CSV export.';
    }
  });
})();
