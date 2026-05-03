/**
 * Marketplace Overview — mp-index.js
 * Fetches KPIs, Live Trades, and compact topbar health from backend APIs.
 */
(function () {
  'use strict';

  const API_BASE = '/api/admin/marketplace';
  const REFRESH_INTERVAL_MS = 30_000;

  // Anomaly thresholds
  const WASH_WINDOW_SECONDS = 5;          // Cluster trades within this many seconds
  const WASH_MIN_COUNT = 5;               // Minimum identical trades to flag
  const SLA_PENDING_REVIEW_SECONDS = 4 * 3600;   // 4h → flag pending approval
  const SLA_OPEN_ORDER_SECONDS = 7 * 86400;       // 7d → flag stale open order
  const DB_LATENCY_WARN_MS = 50;
  const DB_LATENCY_CRIT_MS = 200;
  const REDIS_LATENCY_WARN_MS = 20;
  const QUEUE_DEPTH_WARN = 1000;
  const QUEUE_DEPTH_CRIT = 5000;
  const TRADES_DISPLAY_LIMIT = 25;
  const SYSTEM_HANDLES = new Set(['support', 'system', 'liquidity', 'poool-system']);
  const LATENCY_BUFFER_SIZE = 30;
  const STALE_REFRESH_MS = REFRESH_INTERVAL_MS * 2.5;
  const RANGE_WINDOWS_MS = { '1h': 3600 * 1000, '24h': 86400 * 1000, '7d': 7 * 86400 * 1000 };

  let filterText = '';
  let filterAsset = '';
  let filterFlagged = 'all';
  let activeRange = '24h';
  const latencyBuffer = [];
  let lastSuccessAt = null;

  let lastStatsError = null;
  let lastTradesError = null;
  let lastHealthError = null;
  let lastTrades = [];
  let refreshTimer = null;
  let refreshInFlight = false;

  // ===== UTIL =====

  async function fetchJSON(url) {
    const res = await fetch(url, { credentials: 'same-origin' });
    if (!res.ok) {
      let message = `HTTP ${res.status}`;
      try {
        const body = await res.json();
        message = body.error || body.message || message;
      } catch (_) {}
      throw new Error(message);
    }
    return res.json();
  }

  function $(id) { return document.getElementById(id); }

  function setText(id, value) {
    const el = $(id);
    if (el) el.textContent = value;
  }

  function setHidden(id, hidden) {
    const el = $(id);
    if (el) el.hidden = hidden;
  }

  function formatCurrency(cents) {
    return (Number(cents || 0) / 100).toLocaleString('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    });
  }

  function formatAge(seconds) {
    if (seconds == null) return '';
    const s = Math.max(0, Math.floor(seconds));
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    if (s < 86400) return `${Math.floor(s / 3600)}h`;
    return `${Math.floor(s / 86400)}d`;
  }

  function formatDelta(current, prior) {
    const cur = Number(current || 0);
    const prev = Number(prior || 0);
    if (prev === 0 && cur === 0) return { text: '— vs prior 24h', tone: 'neutral' };
    if (prev === 0) return { text: 'new vs prior 24h', tone: 'up' };
    const pct = ((cur - prev) / prev) * 100;
    const sign = pct > 0 ? '▲' : pct < 0 ? '▼' : '–';
    const tone = pct > 0 ? 'up' : pct < 0 ? 'down' : 'neutral';
    return { text: `${sign} ${Math.abs(pct).toFixed(1)}% vs prior 24h`, tone };
  }

  function firstSegment(value, fallback) {
    const text = String(value || fallback || '');
    return text.includes('@') ? text.split('@')[0] : text;
  }

  function shortId(value) {
    return String(value || '').substring(0, 8) || 'unknown';
  }

  function clearElement(el) {
    while (el.firstChild) el.removeChild(el.firstChild);
  }

  function makeCell(text, className) {
    const td = document.createElement('td');
    if (className) td.className = className;
    td.textContent = text;
    return td;
  }

  function makeCodeCell(text) {
    const td = document.createElement('td');
    const code = document.createElement('code');
    code.className = 'mp-inline-code';
    code.textContent = text;
    td.appendChild(code);
    return td;
  }

  function renderTableMessage(message, tone) {
    const tbody = $('live-trades-body');
    if (!tbody) return;
    clearElement(tbody);
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 8;
    cell.className = `mp-table-message${tone ? ` mp-table-message--${tone}` : ''}`;
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  // ===== KPI RENDER =====

  function renderStatsError(error) {
    const message = error && error.message ? error.message : 'Stats unavailable';
    setText('kpi-trading-status', 'Unavailable');
    setText('kpi-trading-subtext', message);
    setText('kpi-open-orders', '--');
    setText('kpi-open-orders-subtext', 'Open order count unavailable');
    setText('kpi-volume', '--');
    setText('kpi-volume-subtext', '24h volume unavailable');
    setText('kpi-pending', '--');
    setText('kpi-pending-subtext', 'Review queue unavailable');
    const statusEl = $('kpi-trading-status');
    if (statusEl) statusEl.style.color = 'var(--admin-danger, #ef4444)';
    setHidden('kpi-volume-delta', true);
    setHidden('kpi-pending-sla', true);
    setHidden('kpi-open-orders-sla', true);
  }

  function applyDelta(elId, current, prior) {
    const el = $(elId);
    if (!el) return;
    const { text, tone } = formatDelta(current, prior);
    el.textContent = text;
    el.dataset.tone = tone;
    el.hidden = false;
  }

  function applySla(elId, ageSeconds, slaSeconds, label) {
    const el = $(elId);
    if (!el) return;
    if (ageSeconds == null) {
      el.hidden = true;
      return;
    }
    const overdue = ageSeconds > slaSeconds;
    el.textContent = `${label}: ${formatAge(ageSeconds)}${overdue ? ' (SLA breach)' : ''}`;
    el.dataset.tone = overdue ? 'crit' : 'ok';
    el.hidden = false;
  }

  function renderKPIs(stats) {
    const tradingStatus = stats.trading_status || 'UNKNOWN';
    const statusEl = $('kpi-trading-status');
    const pendingEl = $('kpi-pending');
    const pendingCard = $('kpi-card-pending');

    if (statusEl) {
      statusEl.textContent = tradingStatus;
      statusEl.style.color = tradingStatus === 'LIVE'
        ? 'var(--admin-success)'
        : tradingStatus === 'HALTED'
          ? 'var(--admin-warning)'
          : 'var(--admin-danger, #ef4444)';
    }
    updateHaltControls(tradingStatus);

    setText('kpi-open-orders', Number(stats.open_orders || 0).toLocaleString());
    setText('kpi-volume', formatCurrency(stats.volume_24h_cents));

    const pending = Number(stats.pending_reviews || 0);
    if (pendingEl) {
      pendingEl.textContent = pending.toLocaleString();
      pendingEl.style.color = pending > 0 ? 'var(--admin-warning)' : 'var(--admin-text-primary)';
    }
    if (pendingCard) {
      pendingCard.classList.toggle('mp-kpi-card--alert', pending > 0);
    }

    setText('kpi-trading-subtext',
      tradingStatus === 'LIVE' ? 'Trading enabled'
      : tradingStatus === 'HALTED' ? 'Trading halted by kill-switch'
      : 'Trading status could not be verified');

    const totalAssets = Number(stats.total_assets_trading || 0);
    setText('kpi-open-orders-subtext',
      `${totalAssets.toLocaleString()} active asset${totalAssets === 1 ? '' : 's'}`);

    setText('kpi-volume-subtext',
      `${Number(stats.trades_24h || 0).toLocaleString()} trades in the last 24h`);

    setText('kpi-pending-subtext',
      pending > 0 ? `${pending.toLocaleString()} order${pending === 1 ? '' : 's'} awaiting review`
                  : 'No orders awaiting review');

    // Trend deltas
    applyDelta('kpi-volume-delta', stats.volume_24h_cents, stats.volume_prev_24h_cents);

    // SLA / aging
    applySla('kpi-pending-sla', stats.oldest_pending_review_age_seconds,
             SLA_PENDING_REVIEW_SECONDS, 'Oldest');
    applySla('kpi-open-orders-sla', stats.oldest_open_order_age_seconds,
             SLA_OPEN_ORDER_SECONDS, 'Oldest');

    // Last-updated chip
    const updated = stats.generated_at ? new Date(stats.generated_at) : new Date();
    setText('mp-last-updated',
      `Updated ${updated.toLocaleTimeString('en-US', { hour12: false })}`);
  }

  // ===== TRADE RENDER + ANOMALY DETECTION =====

  /**
   * Flag suspicious trade clusters: >=WASH_MIN_COUNT trades on the same asset
   * at the same price within a WASH_WINDOW_SECONDS window. Returns a Set of
   * trade IDs that should be highlighted, plus a count of distinct clusters.
   */
  function detectSuspiciousTrades(trades) {
    const flagged = new Set();
    if (!Array.isArray(trades) || trades.length === 0) return { flagged, clusters: 0 };

    // Group by (asset_id, price_cents)
    const groups = new Map();
    for (const t of trades) {
      const key = `${t.asset_id}|${t.price_cents}`;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(t);
    }

    let clusters = 0;
    for (const list of groups.values()) {
      if (list.length < WASH_MIN_COUNT) continue;
      // Sort by executed_at ascending
      list.sort((a, b) => new Date(a.executed_at) - new Date(b.executed_at));
      // Sliding window
      let start = 0;
      for (let end = 0; end < list.length; end++) {
        const endTime = new Date(list[end].executed_at).getTime();
        while (start < end &&
               (endTime - new Date(list[start].executed_at).getTime()) > WASH_WINDOW_SECONDS * 1000) {
          start++;
        }
        if (end - start + 1 >= WASH_MIN_COUNT) {
          let isNewCluster = true;
          for (let i = start; i <= end; i++) {
            if (flagged.has(list[i].id)) isNewCluster = false;
            flagged.add(list[i].id);
          }
          if (isNewCluster) clusters++;
        }
      }
    }
    return { flagged, clusters };
  }

  function isSystemHandle(label) {
    return SYSTEM_HANDLES.has(String(label || '').toLowerCase());
  }

  function relativeTime(date) {
    if (!date) return '--';
    const now = Date.now();
    const t = date.getTime();
    if (Number.isNaN(t)) return '--';
    const diff = Math.max(0, Math.floor((now - t) / 1000));
    if (diff < 5) return 'just now';
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function makeUserCell(handleLabel, fullEmail, userId) {
    const td = document.createElement('td');
    td.className = 'mp-user-cell';

    const code = document.createElement('code');
    code.className = 'mp-inline-code';
    code.textContent = handleLabel;
    td.appendChild(code);

    if (isSystemHandle(handleLabel)) {
      const sys = document.createElement('span');
      sys.className = 'mp-sys-badge';
      sys.textContent = 'SYSTEM';
      sys.title = 'POOOL system liquidity account';
      td.appendChild(sys);
    }

    const tipParts = [];
    if (fullEmail && fullEmail !== handleLabel) tipParts.push(fullEmail);
    if (userId) tipParts.push(`id: ${userId}`);
    if (tipParts.length) td.title = tipParts.join('\n');

    return td;
  }

  function applyTradeFilters(trades, flagged) {
    const text = filterText.trim().toLowerCase();
    const windowMs = RANGE_WINDOWS_MS[activeRange] || RANGE_WINDOWS_MS['24h'];
    const cutoff = Date.now() - windowMs;
    return trades.filter((t) => {
      const ts = new Date(t.executed_at).getTime();
      if (Number.isFinite(ts) && ts < cutoff) return false;
      if (filterAsset && t.asset_id !== filterAsset) return false;
      if (filterFlagged === 'suspicious' && !flagged.has(t.id)) return false;
      if (text) {
        const hay = [
          t.asset_name, t.buyer_email, t.seller_email,
          t.buyer_id, t.seller_id, t.id,
        ].join(' ').toLowerCase();
        if (!hay.includes(text)) return false;
      }
      return true;
    });
  }

  function populateAssetFilter(trades) {
    const sel = $('mp-filter-asset');
    if (!sel) return;
    const seen = new Map();
    for (const t of trades) {
      if (!seen.has(t.asset_id)) {
        seen.set(t.asset_id, t.asset_name || shortId(t.asset_id));
      }
    }
    const current = sel.value;
    while (sel.options.length > 1) sel.remove(1);
    Array.from(seen.entries())
      .sort((a, b) => String(a[1]).localeCompare(String(b[1])))
      .forEach(([id, name]) => {
        const opt = document.createElement('option');
        opt.value = id;
        opt.textContent = name;
        sel.appendChild(opt);
      });
    if (current && seen.has(current)) sel.value = current;
  }

  function renderEmptyState(message, kind) {
    const tbody = $('live-trades-body');
    if (!tbody) return;
    clearElement(tbody);
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 8;
    cell.className = `mp-empty-state mp-empty-state--${kind || 'idle'}`;
    cell.innerHTML = `
      <svg class="mp-empty-svg" viewBox="0 0 96 64" aria-hidden="true">
        <rect x="8" y="14" width="80" height="40" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"/>
        <line x1="8" y1="26" x2="88" y2="26" stroke="currentColor" stroke-width="1" opacity="0.3"/>
        <line x1="20" y1="36" x2="60" y2="36" stroke="currentColor" stroke-width="1" opacity="0.3"/>
        <line x1="20" y1="44" x2="44" y2="44" stroke="currentColor" stroke-width="1" opacity="0.3"/>
        <circle cx="76" cy="44" r="6" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.5"/>
        <line x1="80" y1="48" x2="84" y2="52" stroke="currentColor" stroke-width="1.5" opacity="0.5"/>
      </svg>
      <div class="mp-empty-msg">${message}</div>
    `;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function renderTrades(trades) {
    const tbody = $('live-trades-body');
    const badge = $('mp-anomaly-badge');
    const footer = $('mp-trades-footer');
    const countEl = $('mp-trades-count');
    if (!tbody) return;

    if (Array.isArray(trades)) {
      lastTrades = trades;
      populateAssetFilter(lastTrades);
    }

    clearElement(tbody);

    if (lastTrades.length === 0) {
      renderEmptyState('No trades yet — the feed will populate as orders match.', 'idle');
      if (badge) badge.hidden = true;
      if (footer) footer.hidden = true;
      return;
    }

    const { flagged, clusters } = detectSuspiciousTrades(lastTrades);
    const filtered = applyTradeFilters(lastTrades, flagged);

    if (badge) {
      if (flagged.size > 0) {
        badge.textContent = `⚠ ${flagged.size} suspicious trade${flagged.size === 1 ? '' : 's'} (${clusters} cluster${clusters === 1 ? '' : 's'})`;
        badge.hidden = false;
      } else {
        badge.hidden = true;
      }
    }

    if (filtered.length === 0) {
      renderEmptyState('No trades match the current filters.', 'filtered');
      if (footer) footer.hidden = true;
      return;
    }

    const visible = filtered.slice(0, TRADES_DISPLAY_LIMIT);

    visible.forEach((trade) => {
      const row = document.createElement('tr');
      row.className = 'mp-trade-row';
      row.tabIndex = 0;
      row.dataset.tradeId = trade.id;

      if (flagged.has(trade.id)) {
        row.classList.add('mp-trade-suspicious');
        row.title = 'Suspicious cluster: same price, same asset, rapid succession';
      }

      const executed = trade.executed_at ? new Date(trade.executed_at) : null;
      const isoTime = executed && !Number.isNaN(executed.getTime())
        ? executed.toLocaleTimeString('en-US', { hour12: false })
        : '--';
      const relTime = relativeTime(executed);

      const timeCell = document.createElement('td');
      timeCell.className = 'mp-cell-muted mp-cell-tabular';
      timeCell.title = executed ? executed.toISOString() : '';
      timeCell.innerHTML = '';
      const timeMain = document.createElement('span');
      timeMain.textContent = relTime;
      timeMain.className = 'mp-time-rel';
      const timeAbs = document.createElement('span');
      timeAbs.textContent = isoTime;
      timeAbs.className = 'mp-time-abs';
      timeCell.appendChild(timeMain);
      timeCell.appendChild(document.createTextNode(' '));
      timeCell.appendChild(timeAbs);
      row.appendChild(timeCell);

      const assetName = trade.asset_name || shortId(trade.asset_id);
      row.appendChild(makeCell(assetName, 'mp-cell-strong'));

      // Type chip — uses on_chain_status as sub-label when present
      const sideCell = document.createElement('td');
      const chip = document.createElement('span');
      chip.className = 'mp-type-chip';
      chip.textContent = 'EXEC';
      sideCell.appendChild(chip);
      if (trade.on_chain_status && trade.on_chain_status !== 'pending') {
        const sub = document.createElement('span');
        sub.className = `mp-chain-sub mp-chain-sub--${String(trade.on_chain_status).toLowerCase()}`;
        sub.textContent = trade.on_chain_status;
        sideCell.appendChild(sub);
      }
      row.appendChild(sideCell);

      row.appendChild(makeCell(formatCurrency(trade.price_cents), 'mp-num mp-cell-tabular'));
      row.appendChild(makeCell(Number(trade.quantity || 0).toLocaleString(), 'mp-num mp-cell-tabular'));
      row.appendChild(makeCell(formatCurrency(trade.total_cents), 'mp-num mp-cell-strong mp-cell-tabular'));

      row.appendChild(makeUserCell(
        firstSegment(trade.buyer_email, shortId(trade.buyer_id)),
        trade.buyer_email, trade.buyer_id));
      row.appendChild(makeUserCell(
        firstSegment(trade.seller_email, shortId(trade.seller_id)),
        trade.seller_email, trade.seller_id));

      row.addEventListener('click', () => openTradeDetail(trade));
      row.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          openTradeDetail(trade);
        }
      });
      tbody.appendChild(row);
    });

    if (footer && countEl) {
      footer.hidden = false;
      const total = lastTrades.length;
      const filteredCount = filtered.length;
      const filteredHint = filteredCount !== total ? ` (filtered from ${total})` : '';
      countEl.textContent = filteredCount > TRADES_DISPLAY_LIMIT
        ? `Showing ${visible.length} of ${filteredCount}${filteredHint}`
        : `Showing ${visible.length} trade${visible.length === 1 ? '' : 's'}${filteredHint}`;
    }
  }

  function reapplyFilters() {
    renderTrades(null);
  }

  // ===== TOP ASSETS (derived from recent-trades) =====

  function renderTopAssets(trades) {
    const tbody = $('mp-top-assets-body');
    if (!tbody) return;
    clearElement(tbody);

    if (!Array.isArray(trades) || trades.length === 0) {
      const tr = document.createElement('tr');
      tr.innerHTML = '<td colspan="4" class="mp-table-message">No recent trades</td>';
      tbody.appendChild(tr);
      return null;
    }

    const agg = new Map();
    for (const t of trades) {
      const key = t.asset_id;
      if (!agg.has(key)) {
        agg.set(key, {
          asset_id: t.asset_id,
          asset_name: t.asset_name || shortId(t.asset_id),
          trades: 0,
          volume_cents: 0,
          last_price_cents: 0,
          last_time: 0,
        });
      }
      const row = agg.get(key);
      row.trades += 1;
      row.volume_cents += Number(t.total_cents || 0);
      const t_ms = new Date(t.executed_at).getTime();
      if (t_ms >= row.last_time) {
        row.last_time = t_ms;
        row.last_price_cents = Number(t.price_cents || 0);
      }
    }

    const sorted = Array.from(agg.values())
      .sort((a, b) => b.volume_cents - a.volume_cents)
      .slice(0, 5);

    sorted.forEach((row) => {
      const tr = document.createElement('tr');
      const nameTd = document.createElement('td');
      const link = document.createElement('a');
      link.href = `/admin/marketplace/orderbook.html?asset_id=${encodeURIComponent(row.asset_id)}`;
      link.textContent = row.asset_name;
      link.className = 'mp-cell-strong';
      nameTd.appendChild(link);
      tr.appendChild(nameTd);
      tr.appendChild(makeCell(row.trades.toLocaleString(), 'mp-num mp-cell-tabular'));
      tr.appendChild(makeCell(formatCurrency(row.volume_cents), 'mp-num mp-cell-tabular mp-cell-strong'));
      tr.appendChild(makeCell(formatCurrency(row.last_price_cents), 'mp-num mp-cell-tabular'));
      tbody.appendChild(tr);
    });

    return sorted[0] || null;
  }

  // ===== MINI ORDERBOOK =====

  let lastOrderbookAssetId = null;

  async function loadMiniOrderbook(assetId, assetName) {
    const root = $('mp-orderbook-mini');
    const titleEl = $('mp-orderbook-asset');
    const spreadEl = $('mp-orderbook-spread');
    if (!root) return;

    if (!assetId) {
      root.innerHTML = '<div class="mp-table-message">No active asset</div>';
      if (titleEl) titleEl.textContent = '';
      if (spreadEl) spreadEl.hidden = true;
      return;
    }

    if (titleEl) titleEl.textContent = `· ${assetName || ''}`;
    lastOrderbookAssetId = assetId;

    try {
      const ob = await fetchJSON(`${API_BASE}/orderbook/${encodeURIComponent(assetId)}`);
      renderMiniOrderbook(ob);
    } catch (err) {
      root.innerHTML = `<div class="mp-table-message mp-table-message--error">Orderbook unavailable: ${err.message || 'request failed'}</div>`;
      if (spreadEl) spreadEl.hidden = true;
    }
  }

  function renderMiniOrderbook(ob) {
    const root = $('mp-orderbook-mini');
    const spreadEl = $('mp-orderbook-spread');
    if (!root) return;

    const bids = (ob.bids || []).slice(0, 5);
    const asks = (ob.asks || []).slice(0, 5).reverse();

    if (bids.length === 0 && asks.length === 0) {
      root.innerHTML = '<div class="mp-table-message">Orderbook empty</div>';
      if (spreadEl) spreadEl.hidden = true;
      return;
    }

    if (spreadEl) {
      if (ob.spread_cents != null) {
        spreadEl.textContent = `Spread ${formatCurrency(ob.spread_cents)}`;
        spreadEl.hidden = false;
      } else {
        spreadEl.hidden = true;
      }
    }

    const renderRow = (level, side) => {
      const cents = level.price_cents ?? level.price ?? 0;
      const qty = level.quantity ?? level.total_quantity ?? level.qty ?? 0;
      return `
        <tr class="mp-ob-row mp-ob-row--${side}">
          <td class="mp-num mp-cell-tabular">${formatCurrency(cents)}</td>
          <td class="mp-num mp-cell-tabular">${Number(qty).toLocaleString()}</td>
        </tr>`;
    };

    root.innerHTML = `
      <table class="admin-table mp-ob-table">
        <thead><tr><th class="mp-num">Price</th><th class="mp-num">Qty</th></tr></thead>
        <tbody>
          ${asks.map((l) => renderRow(l, 'ask')).join('')}
          <tr class="mp-ob-spread"><td colspan="2">${ob.mid_price_cents != null ? `Mid ${formatCurrency(ob.mid_price_cents)}` : '—'}</td></tr>
          ${bids.map((l) => renderRow(l, 'bid')).join('')}
        </tbody>
      </table>`;
  }

  // ===== TRADE DETAIL SLIDE-OUT =====

  function openTradeDetail(trade) {
    const panel = $('mp-trade-detail');
    const backdrop = $('mp-slideout-backdrop');
    const body = $('mp-trade-detail-body');
    if (!panel || !body) return;

    const executed = trade.executed_at ? new Date(trade.executed_at) : null;
    const rows = [
      ['Trade ID', trade.id || '—'],
      ['Asset', `${trade.asset_name || ''} (${trade.asset_id})`],
      ['Executed', executed ? `${executed.toISOString()} (${relativeTime(executed)})` : '—'],
      ['Price', formatCurrency(trade.price_cents)],
      ['Quantity', Number(trade.quantity || 0).toLocaleString()],
      ['Total', formatCurrency(trade.total_cents)],
      ['Fee', formatCurrency(trade.fee_cents)],
      ['Buyer', `${trade.buyer_email || '—'} (${trade.buyer_id})`],
      ['Seller', `${trade.seller_email || '—'} (${trade.seller_id})`],
      ['On-chain', trade.on_chain_status || '—'],
    ];

    body.innerHTML = `
      <dl class="mp-detail-list">
        ${rows.map(([k, v]) => `
          <dt>${k}</dt>
          <dd><code class="mp-inline-code">${String(v).replace(/[<>&]/g, (c) => ({'<':'&lt;','>':'&gt;','&':'&amp;'}[c]))}</code></dd>
        `).join('')}
      </dl>
      <div class="mp-notes-mount" id="mp-notes-mount-${trade.id}"></div>`;

    if (trade.id) loadTradeNotes(trade.id, body.querySelector(`#mp-notes-mount-${trade.id}`));

    panel.hidden = false;
    panel.setAttribute('aria-hidden', 'false');
    if (backdrop) backdrop.hidden = false;
    requestAnimationFrame(() => panel.classList.add('is-open'));
  }

  function closeTradeDetail() {
    const panel = $('mp-trade-detail');
    const backdrop = $('mp-slideout-backdrop');
    if (!panel) return;
    panel.classList.remove('is-open');
    panel.setAttribute('aria-hidden', 'true');
    setTimeout(() => {
      panel.hidden = true;
      if (backdrop) backdrop.hidden = true;
    }, 200);
  }

  // ===== HALT / RESUME TRADING =====

  function updateHaltControls(tradingStatus) {
    const halt = $('mp-halt-btn');
    const resume = $('mp-resume-btn');
    if (halt) halt.hidden = tradingStatus !== 'LIVE';
    if (resume) resume.hidden = tradingStatus !== 'HALTED';
  }

  async function toggleTrading(enable) {
    const action = enable ? 'resume' : 'halt';
    const reason = window.prompt(
      `${enable ? 'Resume' : 'HALT'} trading?\n\nThis affects ALL users immediately.\nEnter reason for audit log:`,
      ''
    );
    if (reason === null) return;
    if (!enable && reason.trim().length < 3) {
      alert('A reason (min 3 chars) is required to halt trading.');
      return;
    }

    try {
      const res = await fetch(`${API_BASE}/toggle-trading`, {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: enable, reason: reason || null }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error || body.message || `HTTP ${res.status}`);
      }
      if (typeof mpToast === 'function') mpToast(`Trading ${action}ed`, 'success');
      await refreshAll();
    } catch (err) {
      if (typeof mpToast === 'function') mpToast(`Failed to ${action} trading: ${err.message}`, 'error');
      else alert(`Failed to ${action} trading: ${err.message}`);
    }
  }

  // ===== CSV EXPORT =====

  function exportTradesCsv() {
    if (!lastTrades.length) return;
    const headers = ['executed_at', 'trade_id', 'asset_id', 'asset_name', 'price_cents',
                     'quantity', 'total_cents', 'fee_cents', 'buyer_id', 'buyer_email',
                     'seller_id', 'seller_email', 'on_chain_status', 'suspicious'];
    const { flagged } = detectSuspiciousTrades(lastTrades);
    const escape = (v) => {
      if (v == null) return '';
      const s = String(v);
      return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const rows = [headers.join(',')];
    for (const t of lastTrades) {
      rows.push([
        t.executed_at, t.id, t.asset_id, t.asset_name, t.price_cents, t.quantity,
        t.total_cents, t.fee_cents, t.buyer_id, t.buyer_email, t.seller_id,
        t.seller_email, t.on_chain_status, flagged.has(t.id) ? 'yes' : 'no',
      ].map(escape).join(','));
    }
    const blob = new Blob([rows.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `recent-trades-${new Date().toISOString().replace(/[:.]/g, '-')}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  // ===== HEALTH =====

  function dbLatencyStatus(ms) {
    if (ms >= DB_LATENCY_CRIT_MS) return 'error';
    if (ms >= DB_LATENCY_WARN_MS) return 'warn';
    return 'ok';
  }

  function redisLatencyStatus(ms) {
    if (ms == null) return 'warn';
    if (ms >= REDIS_LATENCY_WARN_MS * 5) return 'error';
    if (ms >= REDIS_LATENCY_WARN_MS) return 'warn';
    return 'ok';
  }

  function queueStatus(depth) {
    if (depth >= QUEUE_DEPTH_CRIT) return 'error';
    if (depth >= QUEUE_DEPTH_WARN) return 'warn';
    return 'ok';
  }

  function setTopbarHealth(id, status, label) {
    const dot = $(id);
    if (!dot) return;
    dot.className = `admin-health-dot admin-health-dot--${status}`;
    dot.title = label;
  }

  function updateTopbarHealth(health) {
    const dbStatus = health.database_connected ? dbLatencyStatus(health.database_latency_ms) : 'error';
    const redisStatus = health.redis_connected ? redisLatencyStatus(health.redis_latency_ms) : 'error';
    const wsStatus = health.websocket_status === 'healthy' ? 'ok'
                    : health.websocket_status === 'not_tracked' ? 'warn'
                    : 'error';

    setTopbarHealth('health-dot-db', dbStatus, `Database: ${health.database_latency_ms?.toFixed(1)}ms`);
    setTopbarHealth('health-dot-matching', redisStatus,
      `Matching engine: ${health.matching_engine_status || 'unknown'}`);
    setTopbarHealth('health-dot-ws', wsStatus,
      `WebSocket gateway: ${health.websocket_status || 'unknown'}`);
  }

  // ===== CONNECTION STATE PILL =====

  function setConnectionState(state, message) {
    const pill = $('mp-conn-pill');
    if (!pill) return;
    pill.dataset.state = state;
    const labelEl = pill.querySelector('.mp-conn-label');
    if (labelEl) labelEl.textContent = message;
    pill.title = message;
  }

  function checkStaleConnection() {
    if (!lastSuccessAt) return;
    const ago = Date.now() - lastSuccessAt;
    if (ago > STALE_REFRESH_MS) {
      setConnectionState('stale', `Stale (${Math.round(ago / 1000)}s since last update)`);
    }
  }

  // ===== LATENCY SPARKLINE =====

  function pushLatencySample(ms) {
    if (!Number.isFinite(ms)) return;
    latencyBuffer.push({ t: Date.now(), ms });
    if (latencyBuffer.length > LATENCY_BUFFER_SIZE) latencyBuffer.shift();
    renderLatencySparkline();
  }

  function renderLatencySparkline() {
    const svg = $('mp-latency-spark');
    if (!svg) return;
    const samples = latencyBuffer.slice();
    if (samples.length < 2) {
      svg.innerHTML = '<text x="150" y="22" text-anchor="middle" font-size="10" fill="currentColor" opacity="0.5">Collecting samples…</text>';
      return;
    }
    const max = Math.max(...samples.map((s) => s.ms), DB_LATENCY_WARN_MS);
    const w = 300, h = 40;
    const stepX = w / (LATENCY_BUFFER_SIZE - 1);
    const points = samples.map((s, i) => {
      const x = i * stepX;
      const y = h - (s.ms / max) * (h - 4) - 2;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    const lastY = h - (samples[samples.length - 1].ms / max) * (h - 4) - 2;
    const lastX = (samples.length - 1) * stepX;
    const warnY = h - (DB_LATENCY_WARN_MS / max) * (h - 4) - 2;
    svg.innerHTML = `
      <line x1="0" y1="${warnY.toFixed(1)}" x2="${w}" y2="${warnY.toFixed(1)}"
            stroke="var(--admin-warning)" stroke-width="0.5" stroke-dasharray="3 3" opacity="0.6"/>
      <polyline points="${points.join(' ')}"
                fill="none" stroke="var(--admin-primary, #4f46e5)" stroke-width="1.5"/>
      <circle cx="${lastX.toFixed(1)}" cy="${lastY.toFixed(1)}" r="2.5" fill="var(--admin-primary, #4f46e5)"/>
      <text x="${w - 4}" y="10" text-anchor="end" font-size="9" fill="currentColor" opacity="0.6">${samples[samples.length - 1].ms.toFixed(1)}ms · max ${max.toFixed(0)}ms</text>
    `;
  }

  function renderHealth(health) {
    updateTopbarHealth(health);
  }

  // ===== INIT / REFRESH ORCHESTRATION =====

  async function refreshAll() {
    if (refreshInFlight) return;
    refreshInFlight = true;
    const refreshBtn = $('mp-refresh-btn');
    if (refreshBtn) refreshBtn.classList.add('is-loading');

    try {
      const [statsResult, tradesResult, healthResult] = await Promise.allSettled([
        fetchJSON(`${API_BASE}/stats`),
        fetchJSON(`${API_BASE}/recent-trades`),
        fetchJSON(`${API_BASE}/health`),
      ]);

      if (statsResult.status === 'fulfilled') {
        lastStatsError = null;
        renderKPIs(statsResult.value);
      } else {
        lastStatsError = statsResult.reason;
        console.warn('[mp-index] Stats unavailable', lastStatsError);
        renderStatsError(lastStatsError);
      }

      if (tradesResult.status === 'fulfilled') {
        lastTradesError = null;
        renderTrades(tradesResult.value);
        renderFraudSignals(tradesResult.value);
        const top = renderTopAssets(tradesResult.value);
        if (top && top.asset_id !== lastOrderbookAssetId) {
          loadMiniOrderbook(top.asset_id, top.asset_name);
        } else if (top) {
          loadMiniOrderbook(top.asset_id, top.asset_name);
        } else {
          loadMiniOrderbook(null);
        }
      } else {
        lastTradesError = tradesResult.reason;
        console.warn('[mp-index] Recent trades unavailable', lastTradesError);
        renderTableMessage(`Recent trades unavailable: ${lastTradesError.message || 'request failed'}`, 'error');
      }

      if (healthResult.status === 'fulfilled') {
        lastHealthError = null;
        renderHealth(healthResult.value);
      } else {
        lastHealthError = healthResult.reason;
        console.warn('[mp-index] Health unavailable', lastHealthError);
        setTopbarHealth('health-dot-db', 'error', 'Database: health check unavailable');
        setTopbarHealth('health-dot-matching', 'error', 'Matching Engine: health check unavailable');
        setTopbarHealth('health-dot-ws', 'error', 'WebSocket Gateway: health check unavailable');
      }
      const anyOk = [statsResult, tradesResult, healthResult].some((r) => r.status === 'fulfilled');
      const allFail = [statsResult, tradesResult, healthResult].every((r) => r.status !== 'fulfilled');
      if (allFail) {
        setConnectionState('error', 'All endpoints failed');
      } else if (anyOk) {
        lastSuccessAt = Date.now();
        setConnectionState('live', 'Live · last refresh OK');
      }
    } finally {
      refreshInFlight = false;
      if (refreshBtn) refreshBtn.classList.remove('is-loading');
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    refreshTimer = setInterval(refreshAll, REFRESH_INTERVAL_MS);
  }

  function stopAutoRefresh() {
    if (refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }

  function bindControls() {
    const toggle = $('mp-autorefresh');
    if (toggle) {
      toggle.addEventListener('change', () => {
        if (toggle.checked) startAutoRefresh();
        else stopAutoRefresh();
      });
    }
    const refreshBtn = $('mp-refresh-btn');
    if (refreshBtn) refreshBtn.addEventListener('click', refreshAll);

    const exportBtn = $('mp-export-trades');
    if (exportBtn) exportBtn.addEventListener('click', exportTradesCsv);

    const haltBtn = $('mp-halt-btn');
    if (haltBtn) haltBtn.addEventListener('click', () => toggleTrading(false));
    const resumeBtn = $('mp-resume-btn');
    if (resumeBtn) resumeBtn.addEventListener('click', () => toggleTrading(true));

    const filterTextInput = $('mp-filter-text');
    if (filterTextInput) filterTextInput.addEventListener('input', (e) => {
      filterText = e.target.value;
      reapplyFilters();
    });
    const filterAssetSel = $('mp-filter-asset');
    if (filterAssetSel) filterAssetSel.addEventListener('change', (e) => {
      filterAsset = e.target.value;
      reapplyFilters();
    });
    const filterFlaggedSel = $('mp-filter-flagged');
    if (filterFlaggedSel) filterFlaggedSel.addEventListener('change', (e) => {
      filterFlagged = e.target.value;
      reapplyFilters();
    });
    const filterClear = $('mp-filter-clear');
    if (filterClear) filterClear.addEventListener('click', () => {
      if (filterTextInput) filterTextInput.value = '';
      if (filterAssetSel) filterAssetSel.value = '';
      if (filterFlaggedSel) filterFlaggedSel.value = 'all';
      filterText = '';
      filterAsset = '';
      filterFlagged = 'all';
      reapplyFilters();
    });

    document.querySelectorAll('.mp-range-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.mp-range-btn').forEach((b) => {
          b.classList.remove('is-active');
          b.removeAttribute('aria-pressed');
        });
        btn.classList.add('is-active');
        btn.setAttribute('aria-pressed', 'true');
        activeRange = btn.dataset.range;
        reapplyFilters();
      });
    });

    const closeBtn = $('mp-trade-detail-close');
    if (closeBtn) closeBtn.addEventListener('click', closeTradeDetail);
    const backdrop = $('mp-slideout-backdrop');
    if (backdrop) backdrop.addEventListener('click', closeTradeDetail);
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') closeTradeDetail();
    });

    // Pause auto-refresh while tab hidden
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) {
        stopAutoRefresh();
      } else if (toggle?.checked !== false) {
        refreshAll();
        startAutoRefresh();
      }
    });
  }

  // ===== COMMAND PALETTE (Cmd+K) =====

  const PALETTE_COMMANDS = [
    { label: 'Marketplace Overview', kind: 'page', href: '/admin/marketplace/' },
    { label: 'Orderbook', kind: 'page', href: '/admin/marketplace/orderbook.html' },
    { label: 'Trades', kind: 'page', href: '/admin/marketplace/trades.html' },
    { label: 'Open Orders', kind: 'page', href: '/admin/marketplace/orders.html?status=open' },
    { label: 'Approvals queue', kind: 'page', href: '/admin/marketplace/approvals.html' },
    { label: 'Primary Escrow', kind: 'page', href: '/admin/marketplace/primary-escrow.html' },
    { label: 'Fees', kind: 'page', href: '/admin/marketplace/fees.html' },
    { label: 'Alerts', kind: 'page', href: '/admin/marketplace/alerts.html' },
    { label: 'P2P Offers', kind: 'page', href: '/admin/marketplace/p2p.html' },
    { label: 'Reconciliation', kind: 'page', href: '/admin/marketplace/reconciliation.html' },
    { label: 'Compliance', kind: 'page', href: '/admin/marketplace/compliance.html' },
    { label: 'Analytics', kind: 'page', href: '/admin/marketplace/analytics.html' },
    { label: 'Marketplace Settings', kind: 'page', href: '/admin/marketplace/settings.html' },
    { label: 'Users', kind: 'page', href: '/admin/users.html' },
    { label: 'KYC queue', kind: 'page', href: '/admin/kyc.html' },
    { label: 'Deposits', kind: 'page', href: '/admin/deposits.html' },
    { label: 'Audit logs', kind: 'page', href: '/admin/audit-logs.html' },
    { label: 'Refresh now', kind: 'action', run: () => refreshAll() },
    { label: 'Export trades CSV', kind: 'action', run: () => exportTradesCsv() },
    { label: 'Halt trading (kill-switch)', kind: 'action', run: () => toggleTrading(false) },
    { label: 'Resume trading', kind: 'action', run: () => toggleTrading(true) },
    { label: 'Toggle dark mode', kind: 'action', run: () => document.getElementById('admin-theme-toggle')?.click() },
  ];
  let paletteCursor = 0;

  function openPalette() {
    const root = $('mp-palette');
    const backdrop = $('mp-palette-backdrop');
    const input = $('mp-palette-input');
    if (!root || !input) return;
    root.hidden = false;
    if (backdrop) backdrop.hidden = false;
    input.value = '';
    paletteCursor = 0;
    renderPaletteResults('');
    setTimeout(() => input.focus(), 10);
  }

  function closePalette() {
    const root = $('mp-palette');
    const backdrop = $('mp-palette-backdrop');
    if (root) root.hidden = true;
    if (backdrop) backdrop.hidden = true;
  }

  function renderPaletteResults(query) {
    const list = $('mp-palette-results');
    if (!list) return;
    const q = query.trim().toLowerCase();
    const matches = !q
      ? PALETTE_COMMANDS.slice(0, 10)
      : PALETTE_COMMANDS.filter((c) => c.label.toLowerCase().includes(q));
    if (paletteCursor >= matches.length) paletteCursor = 0;
    list.innerHTML = matches.length === 0
      ? '<li class="mp-palette-empty">No matches</li>'
      : matches.map((c, i) => `
          <li class="mp-palette-item ${i === paletteCursor ? 'is-active' : ''}"
              data-idx="${i}" role="option">
            <span class="mp-palette-kind mp-palette-kind--${c.kind}">${c.kind === 'page' ? '↗' : '⚡'}</span>
            <span class="mp-palette-label">${c.label}</span>
          </li>`).join('');
    list.querySelectorAll('.mp-palette-item').forEach((el) => {
      el.addEventListener('click', () => runPaletteAt(matches, Number(el.dataset.idx)));
      el.addEventListener('mouseenter', () => {
        paletteCursor = Number(el.dataset.idx);
        renderPaletteResults(query);
      });
    });
    return matches;
  }

  function runPaletteAt(matches, idx) {
    const cmd = matches[idx];
    if (!cmd) return;
    closePalette();
    if (cmd.kind === 'page') window.location.href = cmd.href;
    else if (typeof cmd.run === 'function') cmd.run();
  }

  function bindPalette() {
    const input = $('mp-palette-input');
    const backdrop = $('mp-palette-backdrop');
    let cachedMatches = PALETTE_COMMANDS.slice(0, 10);

    document.addEventListener('keydown', (e) => {
      const isMod = e.ctrlKey || e.metaKey;
      if (isMod && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        openPalette();
        return;
      }
      const root = $('mp-palette');
      if (!root || root.hidden) return;
      if (e.key === 'Escape') { e.preventDefault(); closePalette(); }
      else if (e.key === 'ArrowDown') {
        e.preventDefault();
        paletteCursor = Math.min(cachedMatches.length - 1, paletteCursor + 1);
        cachedMatches = renderPaletteResults(input?.value || '');
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        paletteCursor = Math.max(0, paletteCursor - 1);
        cachedMatches = renderPaletteResults(input?.value || '');
      } else if (e.key === 'Enter') {
        e.preventDefault();
        runPaletteAt(cachedMatches, paletteCursor);
      }
    });

    if (input) input.addEventListener('input', (e) => {
      paletteCursor = 0;
      cachedMatches = renderPaletteResults(e.target.value);
    });
    if (backdrop) backdrop.addEventListener('click', closePalette);
  }

  // ===== FRAUD SIGNALS (#26) =====

  function detectFraudSignals(trades) {
    if (!Array.isArray(trades) || trades.length === 0) {
      return { selfTrades: 0, velocitySpikes: 0, pingPong: 0, totalScore: 0, examples: [] };
    }
    const examples = [];

    // 1) Self-trades — same user as buyer and seller
    const selfTrades = trades.filter((t) => t.buyer_id && t.seller_id && t.buyer_id === t.seller_id);
    if (selfTrades.length) {
      examples.push({
        kind: 'self',
        msg: `${selfTrades.length} self-trade${selfTrades.length === 1 ? '' : 's'} (buyer = seller)`,
        ids: selfTrades.slice(0, 3).map((t) => t.id),
      });
    }

    // 2) Velocity — single user >10 trades / 60s
    const userBuckets = new Map();
    for (const t of trades) {
      const ts = new Date(t.executed_at).getTime();
      [t.buyer_id, t.seller_id].forEach((uid) => {
        if (!uid) return;
        if (!userBuckets.has(uid)) userBuckets.set(uid, []);
        userBuckets.get(uid).push(ts);
      });
    }
    let velocitySpikes = 0;
    for (const [uid, times] of userBuckets.entries()) {
      times.sort();
      let start = 0;
      for (let end = 0; end < times.length; end++) {
        while (times[end] - times[start] > 60000) start++;
        if (end - start + 1 >= 10) {
          velocitySpikes++;
          examples.push({ kind: 'velocity', msg: `User ${shortId(uid)} traded ${end - start + 1}× in 60s`, ids: [] });
          break;
        }
      }
    }

    // 3) Ping-pong — A→B then B→A within 30s for same asset
    let pingPong = 0;
    const pairKey = (a, b, asset) => [a, b].sort().join('|') + '|' + asset;
    const pairWindows = new Map();
    for (const t of trades) {
      if (!t.buyer_id || !t.seller_id) continue;
      const key = pairKey(t.buyer_id, t.seller_id, t.asset_id);
      if (!pairWindows.has(key)) pairWindows.set(key, []);
      pairWindows.get(key).push({ ts: new Date(t.executed_at).getTime(), buyer: t.buyer_id, seller: t.seller_id, id: t.id });
    }
    for (const list of pairWindows.values()) {
      list.sort((a, b) => a.ts - b.ts);
      for (let i = 1; i < list.length; i++) {
        if (list[i].ts - list[i - 1].ts > 30000) continue;
        if (list[i].buyer === list[i - 1].seller && list[i].seller === list[i - 1].buyer) {
          pingPong++;
        }
      }
    }
    if (pingPong) {
      examples.push({ kind: 'pingpong', msg: `${pingPong} reversal${pingPong === 1 ? '' : 's'} (A→B then B→A within 30s)`, ids: [] });
    }

    const totalScore = selfTrades.length * 10 + velocitySpikes * 5 + pingPong * 7;
    return { selfTrades: selfTrades.length, velocitySpikes, pingPong, totalScore, examples };
  }

  function renderFraudSignals(trades) {
    const root = $('mp-fraud-card');
    if (!root) return;
    const sig = detectFraudSignals(trades);
    const scoreEl = $('mp-fraud-score');
    const listEl = $('mp-fraud-list');
    if (scoreEl) {
      scoreEl.textContent = sig.totalScore;
      scoreEl.dataset.tone = sig.totalScore === 0 ? 'ok' : sig.totalScore < 20 ? 'warn' : 'crit';
    }
    if (listEl) {
      listEl.innerHTML = sig.examples.length === 0
        ? '<li class="mp-fraud-empty">No suspicious patterns detected</li>'
        : sig.examples.map((e) => `
            <li class="mp-fraud-item mp-fraud-item--${e.kind}">
              <span class="mp-fraud-kind">${e.kind.toUpperCase()}</span>
              <span class="mp-fraud-msg">${e.msg}</span>
            </li>`).join('');
    }
  }

  // ===== TRADE NOTES (#35) =====

  async function loadTradeNotes(tradeId, mountEl) {
    if (!mountEl) return;
    mountEl.innerHTML = '<div class="mp-notes-loading">Loading notes…</div>';
    try {
      const notes = await fetchJSON(`${API_BASE}/trade-notes/${encodeURIComponent(tradeId)}`);
      renderTradeNotes(tradeId, notes, mountEl);
    } catch (err) {
      mountEl.innerHTML = `<div class="mp-notes-error">Notes unavailable: ${err.message}</div>`;
    }
  }

  function renderTradeNotes(tradeId, notes, mountEl) {
    const list = (notes || []).map((n) => `
      <li class="mp-note">
        <div class="mp-note-meta">
          <span class="mp-note-author">${(n.author_email || n.author_id || '—').replace(/[<>&]/g, '')}</span>
          <span class="mp-note-time">${new Date(n.created_at).toLocaleString()}</span>
        </div>
        <div class="mp-note-content">${String(n.content || '').replace(/[<>&]/g, (c) => ({'<':'&lt;','>':'&gt;','&':'&amp;'}[c]))}</div>
      </li>`).join('');
    mountEl.innerHTML = `
      <h4 class="mp-notes-title">Audit notes (${(notes || []).length})</h4>
      <ul class="mp-notes-list">${list || '<li class="mp-notes-empty">No notes yet</li>'}</ul>
      <form class="mp-note-form" data-trade-id="${tradeId}">
        <textarea name="content" rows="2" placeholder="Add a note (visible to admins only)…" required maxlength="2000"></textarea>
        <button type="submit" class="admin-btn admin-btn--primary admin-btn--sm">Save note</button>
      </form>`;
    const form = mountEl.querySelector('.mp-note-form');
    if (form) form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const ta = form.querySelector('textarea');
      const content = (ta.value || '').trim();
      if (content.length < 1) return;
      const btn = form.querySelector('button');
      btn.disabled = true;
      try {
        const res = await fetch(`${API_BASE}/trade-notes/${encodeURIComponent(tradeId)}`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content }),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        ta.value = '';
        await loadTradeNotes(tradeId, mountEl);
      } catch (err) {
        if (typeof mpToast === 'function') mpToast(`Failed to save note: ${err.message}`, 'error');
      } finally {
        btn.disabled = false;
      }
    });
  }

  document.addEventListener('DOMContentLoaded', async () => {
    setConnectionState('connecting', 'Connecting…');
    renderEmptyState('Loading recent trades…', 'idle');
    bindControls();
    bindPalette();
    await refreshAll();
    if ($('mp-autorefresh')?.checked !== false) startAutoRefresh();
    setInterval(checkStaleConnection, 5000);
  });

  window.PooolMarketplaceOverview = {
    refreshAll,
    detectSuspiciousTrades,
    getLastErrors() {
      return { stats: lastStatsError, trades: lastTradesError, health: lastHealthError };
    },
  };
})();
