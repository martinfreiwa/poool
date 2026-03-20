/**
 * Trade History — mp-trades.js
 * Mock data for executed trades with filtering, export buttons, and pagination.
 */
(function () {
  'use strict';

  const ASSETS = ['Bali Villa Resort (BVRT)', 'Jakarta Office Tower (JOTX)', 'Surabaya Warehouse (SWHS)', 'Bandung Tech Hub (BTHB)', 'Yogya Heritage Hotel (YHHT)'];
  const ASSET_CODES = ['BVRT', 'JOTX', 'SWHS', 'BTHB', 'YHHT'];
  const STATUSES = ['settled', 'settled', 'settled', 'settled', 'settled', 'settled', 'settled', 'pending', 'settled', 'failed'];
  const USERS = ['USR-8291', 'USR-3384', 'USR-6643', 'USR-1738', 'USR-5561', 'USR-2201', 'USR-7829', 'USR-4410', 'USR-9203', 'USR-1105', 'USR-7712', 'USR-2290', 'USR-9987', 'USR-3344', 'USR-6632', 'USR-5518', 'USR-1234', 'USR-5678', 'USR-8845', 'USR-4455'];

  // Generate 60 mock trades
  const ALL_TRADES = [];
  for (let i = 0; i < 60; i++) {
    const assetIdx = i % ASSETS.length;
    const price = [52.40, 105.00, 23.75, 87.20, 34.90][assetIdx] + (Math.random() - 0.5) * 2;
    const qty = Math.floor(Math.random() * 400) + 10;
    const fee = +(price * qty * 0.005).toFixed(2);
    const total = +(price * qty).toFixed(2);
    const day = Math.max(1, 20 - Math.floor(i / 3));
    const hour = Math.floor(Math.random() * 24);
    const min = Math.floor(Math.random() * 60);

    ALL_TRADES.push({
      id: `TRD-${(100000 + i).toString()}`,
      date: `2026-03-${String(day).padStart(2, '0')} ${String(hour).padStart(2, '0')}:${String(min).padStart(2, '0')}`,
      asset: ASSETS[assetIdx],
      assetCode: ASSET_CODES[assetIdx],
      side: i % 3 === 0 ? 'SELL' : 'BUY',
      buyer: USERS[i % USERS.length],
      seller: USERS[(i + 7) % USERS.length],
      qty,
      price: +price.toFixed(2),
      fee,
      total,
      status: STATUSES[i % STATUSES.length]
    });
  }

  const PAGE_SIZE = 15;
  let currentPage = 1;
  let filteredTrades = [...ALL_TRADES];

  function applyFilters() {
    const assetFilter = document.getElementById('filter-asset').value;
    const statusFilter = document.getElementById('filter-status').value;

    filteredTrades = ALL_TRADES.filter(t => {
      if (assetFilter && t.assetCode !== assetFilter) return false;
      if (statusFilter && t.status !== statusFilter) return false;
      return true;
    });
    currentPage = 1;
    render();
  }

  function statusBadge(status) {
    const map = {
      settled: { cls: 'admin-badge--success', label: 'Settled' },
      pending: { cls: 'admin-badge--warning', label: 'Pending' },
      failed:  { cls: 'admin-badge--danger', label: 'Failed' },
    };
    const s = map[status] || map.settled;
    return `<span class="admin-badge ${s.cls}"><span class="admin-badge-dot"></span>${s.label}</span>`;
  }

  function render() {
    const start = (currentPage - 1) * PAGE_SIZE;
    const page = filteredTrades.slice(start, start + PAGE_SIZE);
    const totalPages = Math.ceil(filteredTrades.length / PAGE_SIZE);

    const tbody = document.getElementById('trades-body');
    if (tbody) {
      tbody.innerHTML = page.map(t => {
        const sideClass = t.side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
        return `
          <tr>
            <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${t.id}</code></td>
            <td style="font-variant-numeric:tabular-nums; font-size:12px; color:var(--admin-text-muted);">${t.date}</td>
            <td style="font-weight:600; color:var(--admin-text-primary);">${t.asset}</td>
            <td><span class="${sideClass}">${t.side}</span></td>
            <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${t.buyer}</code></td>
            <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${t.seller}</code></td>
            <td style="text-align:right; font-variant-numeric:tabular-nums;">${t.qty.toLocaleString()}</td>
            <td style="text-align:right; font-variant-numeric:tabular-nums;">$${t.price.toFixed(2)}</td>
            <td style="text-align:right; font-variant-numeric:tabular-nums; color:var(--admin-text-muted);">$${t.fee.toFixed(2)}</td>
            <td style="text-align:right; font-weight:600; font-variant-numeric:tabular-nums;">$${t.total.toLocaleString()}</td>
            <td>${statusBadge(t.status)}</td>
          </tr>
        `;
      }).join('');
    }

    // Pagination
    const pag = document.getElementById('pagination');
    if (pag) {
      pag.innerHTML = `
        <button class="mp-pagination-btn" id="pg-prev" ${currentPage <= 1 ? 'disabled' : ''}>← Previous</button>
        <span class="mp-pagination-info">Page ${currentPage} of ${totalPages} (${filteredTrades.length} trades)</span>
        <button class="mp-pagination-btn" id="pg-next" ${currentPage >= totalPages ? 'disabled' : ''}>Next →</button>
      `;
      document.getElementById('pg-prev')?.addEventListener('click', () => { currentPage--; render(); });
      document.getElementById('pg-next')?.addEventListener('click', () => { currentPage++; render(); });
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    render();

    document.getElementById('btn-apply-filter')?.addEventListener('click', () => {
      applyFilters();
      mpToast('Filters applied', 'info');
    });

    document.getElementById('btn-export-csv')?.addEventListener('click', function () {
      mpButtonAction(this, 'CSV export started — download will begin shortly', 1200);
    });

    document.getElementById('btn-export-pdf')?.addEventListener('click', function () {
      mpButtonAction(this, 'PDF report generated — download starting', 1500);
    });
  });
})();
