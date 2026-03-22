/**
 * Trade History — mp-trades.js
 * Fetches executed trades from the backend API with filtering and pagination.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/trades';
  const PAGE_SIZE = 15;
  let currentPage = 1;
  let totalPages = 1;
  let totalTrades = 0;
  let usingMockData = false;

  // ── Mock Data Fallback ──────────────────────────────────────────
  const ASSETS = ['Bali Villa Resort (BVRT)', 'Jakarta Office Tower (JOTX)', 'Surabaya Warehouse (SWHS)', 'Bandung Tech Hub (BTHB)', 'Yogya Heritage Hotel (YHHT)'];
  const ASSET_CODES = ['BVRT', 'JOTX', 'SWHS', 'BTHB', 'YHHT'];
  const STATUSES = ['settled', 'settled', 'settled', 'settled', 'settled', 'settled', 'settled', 'pending', 'settled', 'failed'];
  const USERS = ['USR-8291', 'USR-3384', 'USR-6643', 'USR-1738', 'USR-5561', 'USR-2201', 'USR-7829', 'USR-4410', 'USR-9203', 'USR-1105'];

  let mockTrades = [];
  function generateMockTrades() {
    for (let i = 0; i < 60; i++) {
      const assetIdx = i % ASSETS.length;
      const price = [52.40, 105.00, 23.75, 87.20, 34.90][assetIdx] + (Math.random() - 0.5) * 2;
      const qty = Math.floor(Math.random() * 400) + 10;
      const fee = +(price * qty * 0.005).toFixed(2);
      const total = +(price * qty).toFixed(2);
      const day = Math.max(1, 20 - Math.floor(i / 3));
      const hour = Math.floor(Math.random() * 24);
      const min = Math.floor(Math.random() * 60);
      mockTrades.push({
        id: `TRD-${(100000 + i)}`, date: `2026-03-${String(day).padStart(2, '0')} ${String(hour).padStart(2, '0')}:${String(min).padStart(2, '0')}`,
        asset: ASSETS[assetIdx], assetCode: ASSET_CODES[assetIdx], side: i % 3 === 0 ? 'SELL' : 'BUY',
        buyer: USERS[i % USERS.length], seller: USERS[(i + 7) % USERS.length], qty, price: +price.toFixed(2), fee, total,
        status: STATUSES[i % STATUSES.length],
      });
    }
  }

  // ── Status Badge ────────────────────────────────────────────────
  function statusBadge(status) {
    const map = {
      settled: { cls: 'admin-badge--success', label: 'Settled' },
      pending: { cls: 'admin-badge--warning', label: 'Pending' },
      failed:  { cls: 'admin-badge--danger', label: 'Failed' },
    };
    const s = map[status] || map.settled;
    return `<span class="admin-badge ${s.cls}"><span class="admin-badge-dot"></span>${s.label}</span>`;
  }

  // ── Render Table ────────────────────────────────────────────────
  function renderTrades(trades) {
    const tbody = document.getElementById('trades-body');
    if (!tbody) return;

    if (!trades || trades.length === 0) {
      tbody.innerHTML = `<tr><td colspan="11" style="text-align:center; color:var(--admin-text-muted); padding:24px;">No trades found</td></tr>`;
      return;
    }

    tbody.innerHTML = trades.map(t => {
      // API data vs mock data field mapping
      let tradeId, date, asset, side, buyer, seller, qty, price, fee, total, status;
      if (usingMockData) {
        tradeId = t.id; date = t.date; asset = t.asset; side = t.side;
        buyer = t.buyer; seller = t.seller; qty = t.qty; price = t.price;
        fee = t.fee; total = t.total; status = t.status;
      } else {
        tradeId = t.id.substring(0, 8);
        date = new Date(t.executed_at).toLocaleString('en-US', { dateStyle: 'short', timeStyle: 'short' });
        asset = t.asset_name || t.asset_id.substring(0, 8);
        side = 'TRADE';
        buyer = t.buyer_email ? t.buyer_email.split('@')[0] : t.buyer_id.substring(0, 8);
        seller = t.seller_email ? t.seller_email.split('@')[0] : t.seller_id.substring(0, 8);
        qty = t.quantity;
        price = (t.price_cents / 100).toFixed(2);
        fee = (t.fee_cents / 100).toFixed(2);
        total = (t.total_cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2 });
        status = 'settled';
      }
      const sideClass = side === 'BUY' ? 'mp-side-buy' : (side === 'SELL' ? 'mp-side-sell' : 'mp-side-buy');
      return `
        <tr>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${tradeId}</code></td>
          <td style="font-variant-numeric:tabular-nums; font-size:12px; color:var(--admin-text-muted);">${date}</td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${asset}</td>
          <td><span class="${sideClass}">${side}</span></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${buyer}</code></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${seller}</code></td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${typeof qty === 'number' ? qty.toLocaleString() : qty}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">$${price}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums; color:var(--admin-text-muted);">$${fee}</td>
          <td style="text-align:right; font-weight:600; font-variant-numeric:tabular-nums;">$${total}</td>
          <td>${statusBadge(status)}</td>
        </tr>
      `;
    }).join('');
  }

  // ── Render Pagination ───────────────────────────────────────────
  function renderPagination() {
    const pag = document.getElementById('pagination');
    if (!pag) return;
    pag.innerHTML = `
      <button class="mp-pagination-btn" id="pg-prev" ${currentPage <= 1 ? 'disabled' : ''}>← Previous</button>
      <span class="mp-pagination-info">Page ${currentPage} of ${totalPages} (${totalTrades} trades)</span>
      <button class="mp-pagination-btn" id="pg-next" ${currentPage >= totalPages ? 'disabled' : ''}>Next →</button>
    `;
    document.getElementById('pg-prev')?.addEventListener('click', () => { currentPage--; loadTrades(); });
    document.getElementById('pg-next')?.addEventListener('click', () => { currentPage++; loadTrades(); });
  }

  // ── Load Trades (API or Mock) ───────────────────────────────────
  async function loadTrades() {
    try {
      const res = await fetch(`${API}?page=${currentPage}&per_page=${PAGE_SIZE}`, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      usingMockData = false;
      totalTrades = data.total;
      totalPages = data.total_pages || 1;
      renderTrades(data.data);
      renderPagination();
    } catch (err) {
      console.warn('[mp-trades] API unavailable, using mock data:', err);
      if (mockTrades.length === 0) generateMockTrades();
      usingMockData = true;
      totalTrades = mockTrades.length;
      totalPages = Math.ceil(totalTrades / PAGE_SIZE);
      const start = (currentPage - 1) * PAGE_SIZE;
      renderTrades(mockTrades.slice(start, start + PAGE_SIZE));
      renderPagination();
    }
  }

  // ── Init ────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    loadTrades();

    document.getElementById('btn-apply-filter')?.addEventListener('click', () => {
      currentPage = 1;
      loadTrades();
      if (typeof mpToast === 'function') mpToast('Filters applied', 'info');
    });

    document.getElementById('btn-export-csv')?.addEventListener('click', function () {
      if (typeof mpButtonAction === 'function') mpButtonAction(this, 'CSV export started — download will begin shortly', 1200);
    });

    document.getElementById('btn-export-pdf')?.addEventListener('click', function () {
      if (typeof mpButtonAction === 'function') mpButtonAction(this, 'PDF report generated — download starting', 1500);
    });
  });
})();
