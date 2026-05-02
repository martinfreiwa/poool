/**
 * Trade History — mp-trades.js
 * Fetches executed trades from the backend API with filtering, sort,
 * search, summary aggregates, SLA banner, and auto-refresh.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/trades';
  const ASSETS_API = '/api/admin/marketplace/trades/assets';
  const EXPORT_API = '/api/admin/marketplace/trades/export.csv';
  const DEFAULT_PAGE_SIZE = 15;
  const PAGE_SIZE_OPTIONS = [15, 25, 50, 100];
  const WHALE_THRESHOLD_CENTS = 1_000_000; // $10,000
  const SLA_SECONDS = 3600; // 1 hour
  const AUTO_REFRESH_MS = 30_000;
  const SEARCH_DEBOUNCE_MS = 300;
  const COL_PREFS_KEY = 'mp-trades:column-prefs';
  const PAGE_SIZE_KEY = 'mp-trades:page-size';
  const ALL_COLUMNS = [
    { key: 'id', label: 'Trade ID', alwaysOn: true },
    { key: 'date', label: 'Date/Time' },
    { key: 'asset', label: 'Asset' },
    { key: 'age', label: 'Age' },
    { key: 'buyer', label: 'Buyer' },
    { key: 'seller', label: 'Seller' },
    { key: 'qty', label: 'Qty' },
    { key: 'price', label: 'Price' },
    { key: 'fee', label: 'Fee' },
    { key: 'total', label: 'Total' },
    { key: 'status', label: 'Status', alwaysOn: true },
  ];

  let currentPage = 1;
  let totalPages = 1;
  let totalTrades = 0;
  let pageSize = readPageSize();
  let sortBy = 'executed_at';
  let sortDir = 'desc';
  let lastLoadedAt = null;
  let stampTimer = null;
  let refreshTimer = null;
  let searchTimer = null;
  let selectedTradeIds = new Set();
  let lastSummary = null;
  let groupByPair = readGroupByPair();
  let filtersDirty = false;
  const GROUP_BY_PAIR_KEY = 'mp-trades:group-by-pair';

  function readGroupByPair() {
    try { return localStorage.getItem(GROUP_BY_PAIR_KEY) === '1'; } catch (_) { return false; }
  }
  function writeGroupByPair(on) {
    try { localStorage.setItem(GROUP_BY_PAIR_KEY, on ? '1' : '0'); } catch (_) { /* noop */ }
  }

  function readPageSize() {
    try {
      const saved = parseInt(localStorage.getItem(PAGE_SIZE_KEY) || '', 10);
      if (PAGE_SIZE_OPTIONS.includes(saved)) return saved;
    } catch (_) { /* noop */ }
    return DEFAULT_PAGE_SIZE;
  }
  function writePageSize(value) {
    try { localStorage.setItem(PAGE_SIZE_KEY, String(value)); } catch (_) { /* noop */ }
  }
  function readColumnPrefs() {
    try {
      const raw = localStorage.getItem(COL_PREFS_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed : null;
    } catch (_) { return null; }
  }
  function writeColumnPrefs(hidden) {
    try { localStorage.setItem(COL_PREFS_KEY, JSON.stringify(hidden)); } catch (_) { /* noop */ }
  }

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
    return date.toLocaleString('en-US', { dateStyle: 'short', timeStyle: 'medium' });
  }

  function formatIsoTooltip(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '';
    return date.toISOString();
  }

  function ageSeconds(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return null;
    return Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  }

  function formatAge(seconds) {
    if (seconds == null) return '—';
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) {
      const h = Math.floor(seconds / 3600);
      const m = Math.floor((seconds % 3600) / 60);
      return m > 0 ? `${h}h ${m}m` : `${h}h`;
    }
    return `${Math.floor(seconds / 86400)}d`;
  }

  function formatRelative(seconds) {
    if (seconds == null) return '—';
    if (seconds < 5) return 'just now';
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    return `${Math.floor(seconds / 3600)}h ago`;
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

  function appendCodeCell(row, text, fullValue) {
    const cell = document.createElement('td');
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = text;
    if (fullValue && fullValue !== text) {
      code.title = fullValue;
    }
    cell.appendChild(code);
    row.appendChild(cell);
    return cell;
  }

  function appendUserCell(row, email, id, role) {
    const cell = document.createElement('td');
    const label = userLabel(email, id);
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = label;
    if (email) code.title = email;
    cell.appendChild(code);

    const flags = riskScore({ buyer_email: role === 'buyer' ? email : null, seller_email: role === 'seller' ? email : null }, role);
    if (flags.length > 0) {
      const flag = document.createElement('span');
      flag.className = `mp-risk-flag mp-risk-flag--${flags.length >= 2 ? 'high' : 'low'}`;
      flag.textContent = flags.length >= 2 ? '⚠' : '!';
      flag.title = `Risk signals: ${flags.join(', ')}`;
      cell.appendChild(flag);
    }
    row.appendChild(cell);
    return cell;
  }

  function feeBps(trade) {
    const total = Number(trade.total_cents || 0);
    const fee = Number(trade.fee_cents || 0);
    if (total <= 0) return null;
    return Math.round((fee / total) * 10000);
  }

  function appendFeeCell(row, trade) {
    const cell = document.createElement('td');
    cell.style.textAlign = 'right';
    cell.style.fontVariantNumeric = 'tabular-nums';
    const dollar = document.createElement('div');
    dollar.textContent = `$${cents(trade.fee_cents)}`;
    dollar.style.color = 'var(--admin-text-muted)';
    cell.appendChild(dollar);
    const bps = feeBps(trade);
    if (bps != null) {
      const sub = document.createElement('div');
      sub.className = 'mp-fee-bps';
      sub.textContent = `${bps} bps`;
      sub.title = `${(bps / 100).toFixed(2)}% of total`;
      cell.appendChild(sub);
    }
    row.appendChild(cell);
    return cell;
  }

  function appendMoneyCell(row, text, { muted = false, strong = false, whale = false } = {}) {
    const cell = appendTextCell(row, `$${text}`);
    cell.style.textAlign = 'right';
    cell.style.fontVariantNumeric = 'tabular-nums';
    if (muted) cell.style.color = 'var(--admin-text-muted)';
    if (strong) cell.style.fontWeight = '600';
    if (whale) cell.classList.add('mp-money-whale');
    return cell;
  }

  function appendAgeCell(row, executedAt, status) {
    const cell = document.createElement('td');
    const seconds = ageSeconds(executedAt);
    cell.textContent = formatAge(seconds);
    cell.style.fontVariantNumeric = 'tabular-nums';
    cell.style.fontSize = '12px';
    cell.dataset.ageSeconds = seconds == null ? '' : String(seconds);
    if (status === 'pending' && seconds != null && seconds >= SLA_SECONDS) {
      cell.classList.add('mp-age-over-sla');
      cell.title = `Pending for over ${Math.floor(SLA_SECONDS / 60)} minutes — exceeds SLA`;
    } else {
      cell.style.color = 'var(--admin-text-muted)';
    }
    row.appendChild(cell);
    return cell;
  }

  function statusIconSvg(status) {
    const ns = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('class', 'mp-status-icon');
    svg.setAttribute('width', '12');
    svg.setAttribute('height', '12');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', '2.5');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    svg.setAttribute('aria-hidden', 'true');

    const paths = {
      confirmed: ['<polyline points="20 6 9 17 4 12"/>'],
      pending:   ['<circle cx="12" cy="12" r="10"/>', '<polyline points="12 6 12 12 16 14"/>'],
      submitted: ['<path d="M22 2L11 13"/>', '<path d="M22 2l-7 20-4-9-9-4 20-7z"/>'],
      failed:    ['<line x1="18" y1="6" x2="6" y2="18"/>', '<line x1="6" y1="6" x2="18" y2="18"/>'],
    };
    const list = paths[status] || paths.pending;
    svg.innerHTML = list.join('');
    return svg;
  }

  function appendStatusCell(row, status) {
    const cell = document.createElement('td');
    const meta = statusMeta(status);
    const badge = document.createElement('span');
    badge.className = `admin-badge ${meta.cls}`;
    badge.appendChild(statusIconSvg(status));
    badge.appendChild(document.createTextNode(meta.label));
    cell.appendChild(badge);
    row.appendChild(cell);
  }

  function appendIdWithKebabCell(row, trade) {
    const cell = document.createElement('td');
    cell.className = 'mp-id-cell';
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = compactId(trade.id);
    if (trade.id) code.title = trade.id;
    cell.appendChild(code);

    const kebab = document.createElement('button');
    kebab.type = 'button';
    kebab.className = 'mp-kebab-btn';
    kebab.setAttribute('aria-haspopup', 'menu');
    kebab.setAttribute('aria-expanded', 'false');
    kebab.setAttribute('aria-label', `Actions for trade ${compactId(trade.id)}`);
    kebab.innerHTML = '<span aria-hidden="true">⋮</span>';
    kebab.dataset.tradeId = trade.id || '';
    kebab.addEventListener('click', (e) => {
      e.stopPropagation();
      openKebabMenu(kebab, trade);
    });
    cell.appendChild(kebab);
    row.appendChild(cell);
    return cell;
  }

  function openKebabMenu(anchor, trade) {
    closeKebabMenu();
    const menu = document.createElement('div');
    menu.className = 'mp-kebab-menu';
    menu.id = 'mp-kebab-menu';
    menu.setAttribute('role', 'menu');

    const items = [
      { label: 'View detail', fn: () => openTradeDrawer(trade) },
      { label: 'Copy trade ID', fn: () => copyToClipboard(trade.id) },
      { label: 'Copy as JSON', fn: () => copyToClipboard(JSON.stringify(trade, null, 2)) },
      { label: 'Open buyer', fn: () => trade.buyer_id && (window.location.href = `/admin/user-details.html?id=${encodeURIComponent(trade.buyer_id)}`), disabled: !trade.buyer_id },
      { label: 'Open seller', fn: () => trade.seller_id && (window.location.href = `/admin/user-details.html?id=${encodeURIComponent(trade.seller_id)}`), disabled: !trade.seller_id },
      { label: 'Open buy order', fn: () => trade.buy_order_id && (window.location.href = `/admin/marketplace/orders.html?order_id=${encodeURIComponent(trade.buy_order_id)}`), disabled: !trade.buy_order_id },
      { label: 'Open sell order', fn: () => trade.sell_order_id && (window.location.href = `/admin/marketplace/orders.html?order_id=${encodeURIComponent(trade.sell_order_id)}`), disabled: !trade.sell_order_id },
    ];

    items.forEach((item) => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'mp-kebab-item';
      btn.textContent = item.label;
      btn.setAttribute('role', 'menuitem');
      if (item.disabled) {
        btn.disabled = true;
      } else {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          closeKebabMenu();
          item.fn();
        });
      }
      menu.appendChild(btn);
    });

    document.body.appendChild(menu);
    const rect = anchor.getBoundingClientRect();
    menu.style.top = `${window.scrollY + rect.bottom + 4}px`;
    menu.style.left = `${window.scrollX + rect.left}px`;
    anchor.setAttribute('aria-expanded', 'true');

    const closeOnOutside = (e) => {
      if (!menu.contains(e.target) && e.target !== anchor) closeKebabMenu();
    };
    setTimeout(() => document.addEventListener('click', closeOnOutside, { once: true }), 0);
  }

  function closeKebabMenu() {
    const existing = document.getElementById('mp-kebab-menu');
    if (existing) existing.remove();
    document.querySelectorAll('.mp-kebab-btn[aria-expanded="true"]').forEach((b) => {
      b.setAttribute('aria-expanded', 'false');
    });
  }

  function appendCheckboxCell(row, tradeId) {
    const cell = document.createElement('td');
    cell.className = 'mp-td-checkbox';
    const wrap = document.createElement('label');
    wrap.className = 'mp-check-wrap';
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.className = 'mp-row-check';
    cb.dataset.tradeId = tradeId || '';
    cb.checked = selectedTradeIds.has(String(tradeId));
    cb.setAttribute('aria-label', `Select trade ${compactId(tradeId)}`);
    cb.addEventListener('click', (e) => e.stopPropagation());
    cb.addEventListener('change', () => {
      const id = String(tradeId);
      if (cb.checked) selectedTradeIds.add(id);
      else selectedTradeIds.delete(id);
      renderBulkBar();
      syncSelectAllCheckbox();
    });
    const indicator = document.createElement('span');
    wrap.append(cb, indicator);
    cell.appendChild(wrap);
    row.appendChild(cell);
  }

  function renderMessage(message, { error = false } = {}) {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.replaceChildren();
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 12;
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
    cell.colSpan = 12;
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

    const foot = $('trades-foot');
    if (foot) foot.hidden = true;

    const banner = $('trades-sla-banner');
    if (banner) banner.hidden = true;
    const recon = $('trades-recon-banner');
    if (recon) recon.hidden = true;
    const summary = $('trades-filter-summary');
    if (summary) summary.replaceChildren();
  }

  let tradesById = new Map();

  function renderTrades(trades) {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.replaceChildren();

    if (!Array.isArray(trades) || trades.length === 0) {
      renderMessage('No trades found');
      tradesById = new Map();
      return;
    }

    tradesById = new Map(trades.map((t) => [String(t.id), t]));

    let prevPairKey = null;
    const fragment = document.createDocumentFragment();
    trades.forEach((trade) => {
      const pairKey = `${trade.buyer_id || ''}::${trade.seller_id || ''}`;
      const isRepeatPair = groupByPair && pairKey === prevPairKey;
      prevPairKey = pairKey;
      const row = document.createElement('tr');
      row.dataset.tradeId = trade.id || '';
      row.dataset.assetId = trade.asset_id || '';
      row.dataset.status = trade.on_chain_status || '';
      row.classList.add('mp-trade-row');
      if (isRepeatPair) row.classList.add('mp-trade-row--repeat-pair');
      row.tabIndex = 0;
      row.setAttribute('role', 'button');
      row.setAttribute('aria-label', `Open detail for trade ${compactId(trade.id)}`);

      appendCheckboxCell(row, trade.id);
      appendIdWithKebabCell(row, trade);

      const dateCell = appendTextCell(row, formatDate(trade.executed_at));
      dateCell.style.fontVariantNumeric = 'tabular-nums';
      dateCell.style.fontSize = '12px';
      dateCell.style.color = 'var(--admin-text-muted)';
      dateCell.title = formatIsoTooltip(trade.executed_at);

      const assetCell = appendTextCell(row, trade.asset_name || compactId(trade.asset_id));
      assetCell.style.fontWeight = '600';
      assetCell.style.color = 'var(--admin-text-primary)';

      appendAgeCell(row, trade.executed_at, trade.on_chain_status);

      appendUserCell(row, trade.buyer_email, trade.buyer_id, 'buyer');
      appendUserCell(row, trade.seller_email, trade.seller_id, 'seller');

      const quantityCell = appendTextCell(row, Number(trade.quantity || 0).toLocaleString('en-US'));
      quantityCell.style.textAlign = 'right';
      quantityCell.style.fontVariantNumeric = 'tabular-nums';

      appendMoneyCell(row, cents(trade.price_cents));
      appendFeeCell(row, trade);

      const isWhale = Number(trade.total_cents || 0) >= WHALE_THRESHOLD_CENTS;
      appendMoneyCell(row, cents(trade.total_cents), { strong: true, whale: isWhale });

      appendStatusCell(row, trade.on_chain_status);

      fragment.appendChild(row);
    });
    tbody.appendChild(fragment);
  }

  function renderSummary(summary) {
    const foot = $('trades-foot');
    if (!foot) return;
    if (!summary) {
      foot.hidden = true;
      return;
    }
    const qty = $('totals-qty');
    const fee = $('totals-fee');
    const volume = $('totals-volume');
    if (qty) qty.textContent = Number(summary.total_quantity || 0).toLocaleString('en-US');
    if (fee) fee.textContent = `$${cents(summary.total_fee_cents)}`;
    if (volume) volume.textContent = `$${cents(summary.total_volume_cents)}`;
    foot.hidden = totalTrades <= 0;
  }

  function renderBanner(summary) {
    const banner = $('trades-sla-banner');
    if (!banner) return;
    const overSla = Number(summary?.over_sla_count || 0);
    const oldest = summary?.oldest_pending_age_seconds;
    if (overSla <= 0) {
      banner.hidden = true;
      banner.replaceChildren();
      return;
    }
    banner.replaceChildren();

    const icon = document.createElement('span');
    icon.className = 'mp-sla-banner-icon';
    icon.setAttribute('aria-hidden', 'true');
    icon.textContent = '⚠';

    const text = document.createElement('div');
    text.className = 'mp-sla-banner-text';
    const headline = document.createElement('strong');
    headline.textContent = `${overSla} pending trade${overSla === 1 ? '' : 's'} exceed${overSla === 1 ? 's' : ''} the 1h SLA`;
    const detail = document.createElement('span');
    detail.style.marginLeft = '8px';
    detail.style.color = 'var(--admin-text-muted)';
    if (oldest != null) {
      detail.textContent = `Oldest pending: ${formatAge(Number(oldest))}`;
    }
    text.appendChild(headline);
    if (oldest != null) text.appendChild(detail);

    const action = document.createElement('button');
    action.type = 'button';
    action.className = 'admin-btn admin-btn--secondary admin-btn--sm';
    action.textContent = 'Show pending only';
    action.addEventListener('click', () => {
      const sel = $('filter-status');
      if (sel) sel.value = 'pending';
      currentPage = 1;
      loadTrades();
    });

    banner.append(icon, text, action);
    banner.hidden = false;
  }

  function updateUpdatedStamp() {
    const stamp = $('trades-updated-stamp');
    if (!stamp) return;
    if (!lastLoadedAt) {
      stamp.textContent = '—';
      return;
    }
    const seconds = Math.max(0, Math.floor((Date.now() - lastLoadedAt) / 1000));
    stamp.textContent = `Updated ${formatRelative(seconds)}`;
  }

  function startTimers() {
    if (stampTimer) clearInterval(stampTimer);
    stampTimer = setInterval(updateUpdatedStamp, 1000);
    if (refreshTimer) clearInterval(refreshTimer);
    refreshTimer = setInterval(() => {
      if (document.visibilityState === 'visible') loadTrades({ silent: true });
    }, AUTO_REFRESH_MS);
  }

  function filterParams({ includePagination = true } = {}) {
    const params = new URLSearchParams();
    if (includePagination) {
      params.set('page', String(currentPage));
      params.set('per_page', String(pageSize));
    }

    const fromDate = $('filter-start')?.value;
    const toDate = $('filter-end')?.value;
    const assetId = $('filter-asset')?.value;
    const status = $('filter-status')?.value;
    const search = $('filter-search')?.value?.trim();

    if (fromDate) params.set('from_date', fromDate);
    if (toDate) params.set('to_date', toDate);
    if (assetId) params.set('asset_id', assetId);
    if (status) params.set('on_chain_status', status);
    if (search) params.set('q', search);
    if (sortBy) params.set('sort_by', sortBy);
    if (sortDir) params.set('sort_dir', sortDir);

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
      maybeHideAssetFilter(assets.length);
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

    // Page size selector
    const sizeWrap = document.createElement('label');
    sizeWrap.className = 'mp-pagination-size';
    sizeWrap.textContent = 'Per page: ';
    const sizeSel = document.createElement('select');
    sizeSel.id = 'pg-size';
    sizeSel.setAttribute('aria-label', 'Rows per page');
    PAGE_SIZE_OPTIONS.forEach((n) => {
      const opt = document.createElement('option');
      opt.value = String(n);
      opt.textContent = String(n);
      if (n === pageSize) opt.selected = true;
      sizeSel.appendChild(opt);
    });
    sizeSel.addEventListener('change', () => {
      const next = parseInt(sizeSel.value, 10);
      if (PAGE_SIZE_OPTIONS.includes(next)) {
        pageSize = next;
        writePageSize(next);
        currentPage = 1;
        loadTrades();
      }
    });
    sizeWrap.appendChild(sizeSel);

    // Jump to page input
    const jumpWrap = document.createElement('label');
    jumpWrap.className = 'mp-pagination-jump';
    jumpWrap.textContent = 'Jump: ';
    const jumpInput = document.createElement('input');
    jumpInput.type = 'number';
    jumpInput.id = 'pg-jump';
    jumpInput.min = '1';
    jumpInput.max = String(Math.max(totalPages, 1));
    jumpInput.value = String(currentPage);
    jumpInput.setAttribute('aria-label', 'Jump to page');
    const goPage = () => {
      const v = parseInt(jumpInput.value, 10);
      if (Number.isFinite(v) && v >= 1 && v <= totalPages && v !== currentPage) {
        currentPage = v;
        loadTrades();
      } else {
        jumpInput.value = String(currentPage);
      }
    };
    jumpInput.addEventListener('change', goPage);
    jumpInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        goPage();
      }
    });
    jumpWrap.appendChild(jumpInput);

    pagination.append(previous, info, next, sizeWrap, jumpWrap);
  }

  function updateSortIndicators() {
    document.querySelectorAll('.mp-sortable').forEach((th) => {
      const key = th.dataset.sortKey;
      if (key === sortBy) {
        th.setAttribute('aria-sort', sortDir === 'asc' ? 'ascending' : 'descending');
      } else {
        th.setAttribute('aria-sort', 'none');
      }
    });
  }

  async function loadTrades({ silent = false } = {}) {
    if (!silent) renderMessage('Loading trades...');
    try {
      const res = await fetch(`${API}?${filterParams().toString()}`, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      totalTrades = Number(data.total || 0);
      totalPages = Number(data.total_pages || 1) || 1;
      currentPage = Number(data.page || currentPage);
      lastSummary = data.summary || null;
      clearFiltersDirty();
      renderTrades(data.data);
      renderSummary(data.summary);
      renderBanner(data.summary);
      renderReconBanner(data.summary);
      renderFilterSummary();
      renderPagination();
      applyColumnVisibility();
      syncSelectAllCheckbox();
      lastLoadedAt = Date.now();
      updateUpdatedStamp();
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

  function setQuickRange(range) {
    const start = $('filter-start');
    const end = $('filter-end');
    if (!start || !end) return;
    if (range === 'all') {
      start.value = '';
      end.value = '';
    } else {
      const today = new Date();
      const iso = (d) => d.toISOString().slice(0, 10);
      end.value = iso(today);
      const from = new Date(today);
      if (range === 'today') {
        // already today
      } else if (range === '24h') {
        from.setDate(from.getDate() - 1);
      } else if (range === '7d') {
        from.setDate(from.getDate() - 7);
      } else if (range === '30d') {
        from.setDate(from.getDate() - 30);
      }
      start.value = iso(from);
    }
    currentPage = 1;
    loadTrades();
  }

  function bindSortableHeaders() {
    document.querySelectorAll('.mp-sortable').forEach((th) => {
      th.tabIndex = 0;
      th.style.cursor = 'pointer';
      const trigger = () => {
        const key = th.dataset.sortKey;
        if (!key) return;
        if (sortBy === key) {
          sortDir = sortDir === 'asc' ? 'desc' : 'asc';
        } else {
          sortBy = key;
          sortDir = 'desc';
        }
        updateSortIndicators();
        currentPage = 1;
        loadTrades();
      };
      th.addEventListener('click', trigger);
      th.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          trigger();
        }
      });
    });
  }

  /* ───────────── Detail drawer ───────────── */

  function copyToClipboard(value) {
    if (!value) return;
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(value).then(
        () => { if (typeof mpToast === 'function') mpToast('Copied to clipboard', 'success'); },
        () => { if (typeof mpToast === 'function') mpToast('Copy failed', 'error'); },
      );
    } else {
      const ta = document.createElement('textarea');
      ta.value = value;
      ta.setAttribute('readonly', '');
      ta.style.position = 'absolute';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand('copy'); } catch (_) { /* noop */ }
      ta.remove();
    }
  }

  function detailRow(label, valueNode, { mono = false } = {}) {
    const dt = document.createElement('dt');
    dt.textContent = label;
    const dd = document.createElement('dd');
    if (mono) dd.style.fontVariantNumeric = 'tabular-nums';
    if (typeof valueNode === 'string') dd.textContent = valueNode;
    else if (valueNode) dd.appendChild(valueNode);
    return [dt, dd];
  }

  function copyableId(value) {
    const wrap = document.createElement('span');
    wrap.style.display = 'inline-flex';
    wrap.style.alignItems = 'center';
    wrap.style.gap = '6px';
    const code = document.createElement('code');
    code.textContent = value || '—';
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.style.wordBreak = 'break-all';
    wrap.appendChild(code);
    if (value) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'mp-trade-copy-btn';
      btn.textContent = 'Copy';
      btn.addEventListener('click', () => copyToClipboard(value));
      wrap.appendChild(btn);
    }
    return wrap;
  }

  function userLink(email, id) {
    if (!id) {
      const span = document.createElement('span');
      span.textContent = '—';
      return span;
    }
    const a = document.createElement('a');
    a.href = `/admin/user-details.html?id=${encodeURIComponent(id)}`;
    a.textContent = email || compactId(id);
    if (email) a.title = email;
    return a;
  }

  function assetLink(name, id) {
    if (!id) {
      const span = document.createElement('span');
      span.textContent = name || '—';
      return span;
    }
    const a = document.createElement('a');
    a.href = `/admin/asset-details.html?id=${encodeURIComponent(id)}`;
    a.textContent = name || compactId(id);
    return a;
  }

  function orderLink(orderId) {
    if (!orderId) {
      const span = document.createElement('span');
      span.textContent = '—';
      return span;
    }
    const wrap = document.createElement('span');
    wrap.style.display = 'inline-flex';
    wrap.style.alignItems = 'center';
    wrap.style.gap = '6px';
    const a = document.createElement('a');
    a.href = `/admin/marketplace/orders.html?order_id=${encodeURIComponent(orderId)}`;
    a.textContent = compactId(orderId);
    a.title = orderId;
    wrap.appendChild(a);
    const copy = document.createElement('button');
    copy.type = 'button';
    copy.className = 'mp-trade-copy-btn';
    copy.textContent = 'Copy';
    copy.addEventListener('click', () => copyToClipboard(orderId));
    wrap.appendChild(copy);
    return wrap;
  }

  function txHashNode(txHash) {
    if (!txHash) {
      const span = document.createElement('span');
      span.className = 'mp-tx-pending';
      span.textContent = 'Awaiting submission';
      return span;
    }
    const wrap = document.createElement('span');
    wrap.style.display = 'inline-flex';
    wrap.style.alignItems = 'center';
    wrap.style.gap = '6px';
    const code = document.createElement('code');
    code.textContent = `${txHash.slice(0, 10)}…${txHash.slice(-6)}`;
    code.title = txHash;
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    wrap.appendChild(code);
    const copy = document.createElement('button');
    copy.type = 'button';
    copy.className = 'mp-trade-copy-btn';
    copy.textContent = 'Copy';
    copy.addEventListener('click', () => copyToClipboard(txHash));
    wrap.appendChild(copy);
    return wrap;
  }

  function batchLink(batchId) {
    if (!batchId) {
      const span = document.createElement('span');
      span.textContent = '—';
      return span;
    }
    const a = document.createElement('a');
    a.href = `/admin/marketplace/reconciliation.html?batch_id=${encodeURIComponent(batchId)}`;
    a.textContent = compactId(batchId);
    a.title = batchId;
    return a;
  }

  function riskScore(trade, role) {
    const email = role === 'buyer' ? trade.buyer_email : trade.seller_email;
    const flags = [];
    if (email && email.includes('+')) flags.push('plus-alias');
    if (email && /^test|^e2e|^demo/i.test(email)) flags.push('test-account');
    if (!email) flags.push('no-email');
    return flags;
  }

  function feeBpsFromTrade(trade) {
    const total = Number(trade.total_cents || 0);
    const fee = Number(trade.fee_cents || 0);
    if (total <= 0) return null;
    return (fee / total) * 100;
  }

  function openTradeDrawer(trade) {
    const drawer = $('trade-drawer');
    const backdrop = $('trade-drawer-backdrop');
    const body = $('trade-drawer-body');
    const title = $('trade-drawer-title');
    if (!drawer || !body || !title) return;

    title.textContent = `Trade ${compactId(trade.id)}`;

    body.replaceChildren();

    const dl = document.createElement('dl');
    dl.className = 'mp-p2p-detail-grid';

    const seconds = ageSeconds(trade.executed_at);
    const ageText = formatAge(seconds);
    const overSla = trade.on_chain_status === 'pending' && seconds != null && seconds >= SLA_SECONDS;

    const meta = statusMeta(trade.on_chain_status);
    const statusBadge = document.createElement('span');
    statusBadge.className = `admin-badge ${meta.cls}`;
    const dot = document.createElement('span');
    dot.className = 'admin-badge-dot';
    statusBadge.appendChild(dot);
    statusBadge.appendChild(document.createTextNode(meta.label));

    const ageNode = document.createElement('span');
    ageNode.textContent = ageText;
    if (overSla) {
      ageNode.classList.add('mp-age-over-sla');
      ageNode.title = `Pending for over ${Math.floor(SLA_SECONDS / 60)} minutes — exceeds SLA`;
    }

    const feePct = feeBpsFromTrade(trade);
    const feeText = feePct == null ? `$${cents(trade.fee_cents)}` : `$${cents(trade.fee_cents)} (${feePct.toFixed(2)}%)`;

    const executedDate = new Date(trade.executed_at);
    const executedDisplay = Number.isNaN(executedDate.getTime())
      ? '—'
      : executedDate.toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'medium' });

    const rows = [
      detailRow('Trade ID', copyableId(trade.id)),
      detailRow('Status', statusBadge),
      detailRow('Age', ageNode),
      detailRow('Executed at', executedDisplay, { mono: true }),
      detailRow('Asset', assetLink(trade.asset_name, trade.asset_id)),
      detailRow('Asset ID', copyableId(trade.asset_id)),
      detailRow('Buyer', userLink(trade.buyer_email, trade.buyer_id)),
      detailRow('Buyer ID', copyableId(trade.buyer_id)),
      detailRow('Seller', userLink(trade.seller_email, trade.seller_id)),
      detailRow('Seller ID', copyableId(trade.seller_id)),
      detailRow('Quantity', String(Number(trade.quantity || 0).toLocaleString('en-US')), { mono: true }),
      detailRow('Price', `$${cents(trade.price_cents)}`, { mono: true }),
      detailRow('Fee', feeText, { mono: true }),
      detailRow('Total', `$${cents(trade.total_cents)}`, { mono: true }),
      detailRow('Buy order', orderLink(trade.buy_order_id)),
      detailRow('Sell order', orderLink(trade.sell_order_id)),
      detailRow('Onchain TX', txHashNode(trade.on_chain_tx_hash)),
      detailRow('Settlement batch', batchLink(trade.on_chain_batch_id)),
    ];
    rows.forEach(([dt, dd]) => dl.append(dt, dd));

    body.appendChild(dl);

    drawer.hidden = false;
    if (backdrop) backdrop.hidden = false;
    requestAnimationFrame(() => drawer.classList.add('is-open'));
    $('trade-drawer-close')?.focus();
  }

  function closeTradeDrawer() {
    const drawer = $('trade-drawer');
    const backdrop = $('trade-drawer-backdrop');
    if (!drawer) return;
    drawer.classList.remove('is-open');
    setTimeout(() => {
      drawer.hidden = true;
      if (backdrop) backdrop.hidden = true;
    }, 180);
  }

  function bindRowDrilldown() {
    const tbody = $('trades-body');
    if (!tbody) return;
    tbody.addEventListener('click', (e) => {
      // Don't open drawer when clicking interactive children (copy btn, alias flag, links).
      if (e.target.closest('button, a, input, label, .mp-trade-copy-btn, .mp-check-wrap')) return;
      const row = e.target.closest('tr.mp-trade-row');
      if (!row) return;
      const id = row.dataset.tradeId;
      const trade = id ? tradesById.get(String(id)) : null;
      if (trade) openTradeDrawer(trade);
    });
    tbody.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter' && e.key !== ' ') return;
      const row = e.target.closest('tr.mp-trade-row');
      if (!row) return;
      if (e.target.closest('input, label')) return;
      e.preventDefault();
      const id = row.dataset.tradeId;
      const trade = id ? tradesById.get(String(id)) : null;
      if (trade) openTradeDrawer(trade);
    });
  }

  /* ───────────── Filter dirty-state ───────────── */
  function markFiltersDirty() {
    filtersDirty = true;
    $('btn-apply-filter')?.classList.add('mp-btn-dirty');
  }
  function clearFiltersDirty() {
    filtersDirty = false;
    $('btn-apply-filter')?.classList.remove('mp-btn-dirty');
  }

  /* ───────────── Filter summary bar ───────────── */
  function renderFilterSummary() {
    const wrap = $('trades-filter-summary');
    if (!wrap) return;
    wrap.replaceChildren();

    const fromDate = $('filter-start')?.value;
    const toDate = $('filter-end')?.value;
    const assetSel = $('filter-asset');
    const status = $('filter-status')?.value;
    const search = $('filter-search')?.value?.trim();
    const assetText = assetSel && assetSel.value
      ? assetSel.options[assetSel.selectedIndex]?.text || ''
      : '';

    const chips = [];
    if (status) chips.push({ label: 'Status', value: status });
    if (assetSel?.value) chips.push({ label: 'Asset', value: assetText });
    if (fromDate || toDate) chips.push({ label: 'Date', value: `${fromDate || '…'} → ${toDate || '…'}` });
    if (search) chips.push({ label: 'Search', value: `"${search}"` });

    const summary = document.createElement('span');
    summary.className = 'mp-filter-summary-text';
    if (totalTrades === 0) {
      summary.textContent = 'No trades match the current filters';
    } else {
      summary.textContent = `Showing ${totalTrades.toLocaleString('en-US')} trade${totalTrades === 1 ? '' : 's'}`;
    }
    wrap.appendChild(summary);

    if (chips.length === 0) return;

    const filterLabel = document.createElement('span');
    filterLabel.className = 'mp-filter-summary-label';
    filterLabel.textContent = '· Filters:';
    wrap.appendChild(filterLabel);

    chips.forEach((c) => {
      const chip = document.createElement('span');
      chip.className = 'mp-filter-summary-chip';
      chip.textContent = `${c.label}: ${c.value}`;
      wrap.appendChild(chip);
    });

    const clearBtn = document.createElement('button');
    clearBtn.type = 'button';
    clearBtn.className = 'mp-filter-summary-clear';
    clearBtn.textContent = 'Clear all';
    clearBtn.addEventListener('click', clearAllFilters);
    wrap.appendChild(clearBtn);
  }

  function clearAllFilters() {
    const fields = ['filter-start', 'filter-end', 'filter-asset', 'filter-status', 'filter-search'];
    fields.forEach((id) => {
      const el = $(id);
      if (el) el.value = '';
    });
    currentPage = 1;
    loadTrades();
  }

  /* ───────────── Reconciliation banner ───────────── */
  function renderReconBanner(summary) {
    const banner = $('trades-recon-banner');
    if (!banner) return;
    const oldest = summary?.oldest_pending_age_seconds;
    const overSla = Number(summary?.over_sla_count || 0);
    // Banner is shown for any pending workload — distinct from SLA banner
    // which only appears when SLA is exceeded.
    if (oldest == null || overSla > 0) {
      banner.hidden = true;
      banner.replaceChildren();
      return;
    }
    banner.replaceChildren();
    const text = document.createElement('span');
    text.textContent = `Pending settlements detected (oldest ${formatAge(Number(oldest))}). Reconciliation may have related work.`;
    const link = document.createElement('a');
    link.href = '/admin/marketplace/reconciliation.html';
    link.className = 'mp-recon-banner-link';
    link.textContent = 'Open Reconciliation →';
    banner.append(text, link);
    banner.hidden = false;
  }

  /* ───────────── Status preset chips ───────────── */
  function applyStatusPreset(preset) {
    const sel = $('filter-status');
    if (!sel) return;
    if (preset === 'action_required') {
      sel.value = 'pending';
    } else if (preset === 'pending') {
      sel.value = 'pending';
    } else if (preset === 'failed') {
      sel.value = 'failed';
    } else {
      sel.value = '';
    }
    document.querySelectorAll('.mp-status-preset').forEach((c) => {
      c.setAttribute('aria-pressed', c.dataset.statusPreset === preset ? 'true' : 'false');
    });
    currentPage = 1;
    loadTrades();
  }

  /* ───────────── Bulk-action bar ───────────── */
  function renderBulkBar() {
    const bar = $('trades-bulk-bar');
    const countEl = $('trades-bulk-count');
    if (!bar || !countEl) return;
    const n = selectedTradeIds.size;
    if (n <= 0) {
      bar.hidden = true;
      return;
    }
    countEl.textContent = `${n} selected`;
    bar.hidden = false;
  }
  function syncSelectAllCheckbox() {
    const all = $('select-all-trades');
    if (!all) return;
    const visible = Array.from(document.querySelectorAll('.mp-row-check'));
    if (visible.length === 0) {
      all.checked = false;
      all.indeterminate = false;
      return;
    }
    const checkedCount = visible.filter((cb) => cb.checked).length;
    all.checked = checkedCount === visible.length;
    all.indeterminate = checkedCount > 0 && checkedCount < visible.length;
  }
  function bindBulkActions() {
    const all = $('select-all-trades');
    if (all) {
      all.addEventListener('change', () => {
        const visible = Array.from(document.querySelectorAll('.mp-row-check'));
        visible.forEach((cb) => {
          cb.checked = all.checked;
          const id = String(cb.dataset.tradeId || '');
          if (all.checked) selectedTradeIds.add(id);
          else selectedTradeIds.delete(id);
        });
        renderBulkBar();
      });
    }
    $('btn-bulk-clear')?.addEventListener('click', () => {
      selectedTradeIds.clear();
      document.querySelectorAll('.mp-row-check').forEach((cb) => { cb.checked = false; });
      syncSelectAllCheckbox();
      renderBulkBar();
    });
    $('btn-bulk-copy-ids')?.addEventListener('click', () => {
      if (selectedTradeIds.size === 0) return;
      const text = Array.from(selectedTradeIds).join('\n');
      copyToClipboard(text);
    });
    $('btn-bulk-export')?.addEventListener('click', () => {
      if (selectedTradeIds.size === 0) return;
      // Bulk export is currently client-side: just trigger a CSV from the
      // visible filtered set. Server doesn't accept a list-of-ids yet.
      const visible = Array.from(document.querySelectorAll('.mp-row-check'))
        .filter((cb) => cb.checked)
        .map((cb) => {
          const id = String(cb.dataset.tradeId);
          return tradesById.get(id);
        })
        .filter(Boolean);
      const header = 'Trade_ID,Executed_At,Asset,Buyer,Seller,Quantity,Price_Cents,Fee_Cents,Total_Cents,Status\n';
      const rows = visible.map((t) => [
        t.id,
        t.executed_at,
        (t.asset_name || '').replace(/"/g, '""'),
        t.buyer_email || '',
        t.seller_email || '',
        t.quantity,
        t.price_cents,
        t.fee_cents,
        t.total_cents,
        t.on_chain_status,
      ].map((v) => {
        const s = String(v ?? '');
        return /[,"\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
      }).join(',')).join('\n');
      const blob = new Blob([header + rows + '\n'], { type: 'text/csv;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `marketplace_trades_selected_${selectedTradeIds.size}.csv`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      if (typeof mpToast === 'function') mpToast(`Exported ${selectedTradeIds.size} trades`, 'success');
    });
  }

  /* ───────────── Column visibility toggle ───────────── */
  function getHiddenColumns() {
    const stored = readColumnPrefs();
    return new Set(stored || []);
  }
  function applyColumnVisibility() {
    const hidden = getHiddenColumns();
    ALL_COLUMNS.forEach((c) => {
      if (c.alwaysOn) return;
      const isHidden = hidden.has(c.key);
      document
        .querySelectorAll(`[data-col="${c.key}"]`)
        .forEach((el) => { el.classList.toggle('mp-col-hidden', isHidden); });
    });
  }
  function buildColumnToggleMenu() {
    const menu = $('col-toggle-menu');
    if (!menu) return;
    menu.replaceChildren();
    const hidden = getHiddenColumns();
    ALL_COLUMNS.forEach((c) => {
      if (c.alwaysOn) return;
      const item = document.createElement('label');
      item.className = 'mp-col-toggle-item';
      const cb = document.createElement('input');
      cb.type = 'checkbox';
      cb.checked = !hidden.has(c.key);
      cb.addEventListener('change', () => {
        const set = getHiddenColumns();
        if (cb.checked) set.delete(c.key); else set.add(c.key);
        writeColumnPrefs(Array.from(set));
        applyColumnVisibility();
      });
      const span = document.createElement('span');
      span.textContent = c.label;
      item.append(cb, span);
      menu.appendChild(item);
    });
  }
  function bindColumnToggle() {
    const btn = $('btn-col-toggle');
    const menu = $('col-toggle-menu');
    if (!btn || !menu) return;
    buildColumnToggleMenu();
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const expanded = btn.getAttribute('aria-expanded') === 'true';
      btn.setAttribute('aria-expanded', String(!expanded));
      menu.hidden = expanded;
    });
    document.addEventListener('click', (e) => {
      if (!menu.contains(e.target) && e.target !== btn) {
        menu.hidden = true;
        btn.setAttribute('aria-expanded', 'false');
      }
    });
  }

  /* ───────────── Keyboard shortcuts ───────────── */
  function bindKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const tag = (e.target?.tagName || '').toLowerCase();
      const inField = tag === 'input' || tag === 'textarea' || tag === 'select' || e.target?.isContentEditable;
      if (e.key === '/' && !inField) {
        e.preventDefault();
        $('filter-search')?.focus();
      } else if (e.key === 'r' && !inField) {
        e.preventDefault();
        loadTrades();
      } else if (e.key === 'Escape') {
        const search = $('filter-search');
        if (search && search === document.activeElement && search.value) {
          search.value = '';
          currentPage = 1;
          loadTrades();
        }
      }
    });
  }

  /* ───────────── Asset filter auto-hide ───────────── */
  function maybeHideAssetFilter(assetCount) {
    const sel = $('filter-asset');
    if (!sel) return;
    if (assetCount <= 1) {
      sel.hidden = true;
      sel.setAttribute('aria-hidden', 'true');
      sel.disabled = true;
    } else {
      sel.hidden = false;
      sel.removeAttribute('aria-hidden');
      sel.disabled = false;
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    loadTradeAssets();
    bindSortableHeaders();
    updateSortIndicators();
    bindRowDrilldown();
    bindBulkActions();
    bindColumnToggle();
    bindKeyboardShortcuts();
    applyColumnVisibility();
    loadTrades();
    startTimers();

    document.querySelectorAll('.mp-status-preset').forEach((chip) => {
      chip.addEventListener('click', () => applyStatusPreset(chip.dataset.statusPreset));
    });

    // Group-by-pair toggle
    const groupBtn = $('btn-group-by-pair');
    if (groupBtn) {
      groupBtn.setAttribute('aria-pressed', String(groupByPair));
      groupBtn.addEventListener('click', () => {
        groupByPair = !groupByPair;
        writeGroupByPair(groupByPair);
        groupBtn.setAttribute('aria-pressed', String(groupByPair));
        loadTrades();
      });
    }

    // Dirty-state on filter dropdowns + dates → highlight Apply button
    ['filter-start', 'filter-end', 'filter-asset', 'filter-status'].forEach((id) => {
      $(id)?.addEventListener('change', markFiltersDirty);
    });

    $('btn-apply-filter')?.addEventListener('click', () => {
      currentPage = 1;
      loadTrades();
    });

    $('btn-refresh-trades')?.addEventListener('click', () => loadTrades());

    $('btn-export-csv')?.addEventListener('click', exportCsv);
    const pdfButton = $('btn-export-pdf');
    if (pdfButton) {
      pdfButton.disabled = true;
      pdfButton.setAttribute('aria-disabled', 'true');
      pdfButton.title = 'PDF export is not available yet. Use CSV export.';
    }

    $('filter-search')?.addEventListener('input', () => {
      if (searchTimer) clearTimeout(searchTimer);
      searchTimer = setTimeout(() => {
        currentPage = 1;
        loadTrades();
      }, SEARCH_DEBOUNCE_MS);
    });

    document.querySelectorAll('.mp-chip[data-range]').forEach((chip) => {
      chip.addEventListener('click', () => setQuickRange(chip.dataset.range));
    });

    $('trade-drawer-close')?.addEventListener('click', closeTradeDrawer);
    $('trade-drawer-backdrop')?.addEventListener('click', closeTradeDrawer);
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        const drawer = $('trade-drawer');
        if (drawer && !drawer.hidden) closeTradeDrawer();
      }
    });
  });
})();
