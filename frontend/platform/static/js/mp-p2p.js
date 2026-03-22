/**
 * P2P Offers — mp-p2p.js
 * Fetches P2P offers from the backend API with price deviation warnings.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/p2p';
  let offers = [];
  let usingMockData = false;

  // ── Mock Data ───────────────────────────────────────────────────
  const MOCK_OFFERS = [
    { id: 'P2P-001', seller: 'USR-8291', asset: 'Bali Villa Resort (BVRT)', qty: 200, offerPrice: 58.00, marketPrice: 52.40, created: '1h ago' },
    { id: 'P2P-002', seller: 'USR-3384', asset: 'Jakarta Office Tower (JOTX)', qty: 50, offerPrice: 108.00, marketPrice: 105.00, created: '3h ago' },
    { id: 'P2P-003', seller: 'USR-6643', asset: 'Surabaya Warehouse (SWHS)', qty: 800, offerPrice: 30.00, marketPrice: 23.75, created: '5h ago' },
    { id: 'P2P-004', seller: 'USR-1738', asset: 'Bandung Tech Hub (BTHB)', qty: 100, offerPrice: 88.00, marketPrice: 87.20, created: '6h ago' },
    { id: 'P2P-005', seller: 'USR-5561', asset: 'Yogya Heritage Hotel (YHHT)', qty: 300, offerPrice: 42.00, marketPrice: 34.90, created: '8h ago' },
    { id: 'P2P-006', seller: 'USR-2201', asset: 'Bali Villa Resort (BVRT)', qty: 150, offerPrice: 51.00, marketPrice: 52.40, created: '12h ago' },
    { id: 'P2P-007', seller: 'USR-7829', asset: 'Jakarta Office Tower (JOTX)', qty: 40, offerPrice: 130.00, marketPrice: 105.00, created: '14h ago' },
    { id: 'P2P-008', seller: 'USR-4410', asset: 'Surabaya Warehouse (SWHS)', qty: 600, offerPrice: 24.00, marketPrice: 23.75, created: '1d ago' },
  ];

  function timeAgo(dateStr) {
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  function render() {
    const tbody = document.getElementById('p2p-body');
    if (!tbody) return;

    // KPIs
    const kTotal = document.getElementById('kpi-p2p-total');
    const kWarning = document.getElementById('kpi-p2p-warnings');
    if (kTotal) kTotal.textContent = offers.length;

    let warningCount = 0;

    tbody.innerHTML = offers.map((o, i) => {
      let offerId, seller, asset, qty, offerPrice, marketPrice, devPct, created;

      if (usingMockData) {
        offerId = o.id; seller = o.seller; asset = o.asset; qty = o.qty;
        offerPrice = o.offerPrice; marketPrice = o.marketPrice;
        devPct = (((o.offerPrice - o.marketPrice) / o.marketPrice) * 100).toFixed(1);
        created = o.created;
      } else {
        offerId = o.id.substring(0, 8);
        seller = o.maker_email ? o.maker_email.split('@')[0] : '—';
        asset = o.asset_name || o.asset_id.substring(0, 8);
        qty = o.quantity;
        offerPrice = o.price_cents / 100;
        marketPrice = o.market_price_cents ? o.market_price_cents / 100 : null;
        devPct = o.price_deviation_pct;
        created = timeAgo(o.created_at);
      }

      const devAbs = devPct !== null ? Math.abs(parseFloat(devPct)) : 0;
      const isWarning = devAbs > 20;
      if (isWarning) warningCount++;
      const devSign = devPct !== null && parseFloat(devPct) >= 0 ? '+' : '';

      let devHTML;
      if (devPct === null || marketPrice === null) {
        devHTML = '<span class="admin-badge admin-badge--neutral">N/A</span>';
      } else if (isWarning) {
        devHTML = `<span class="mp-price-warning">⚠️ ${devSign}${devPct}%</span>`;
      } else if (devAbs > 5) {
        devHTML = `<span class="admin-badge admin-badge--warning">${devSign}${devPct}%</span>`;
      } else {
        devHTML = `<span class="admin-badge admin-badge--success">${devSign}${devPct}%</span>`;
      }

      const statusBadge = o.status
        ? `<span class="admin-badge admin-badge--${o.status === 'pending' ? 'warning' : (o.status === 'accepted' ? 'success' : 'neutral')}"><span class="admin-badge-dot"></span>${o.status}</span>`
        : '';

      return `
        <tr id="p2p-row-${i}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${offerId}</code></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${seller}</code></td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${asset}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${qty.toLocaleString()}</td>
          <td style="text-align:right; font-weight:600; font-variant-numeric:tabular-nums;">$${offerPrice.toFixed(2)}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums; color:var(--admin-text-muted);">${marketPrice !== null ? '$' + marketPrice.toFixed(2) : '—'}</td>
          <td>${devHTML}</td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${created}</td>
          <td style="text-align:center;">
            <button class="admin-btn admin-btn--danger admin-btn--sm btn-cancel-p2p" data-idx="${i}">Admin Cancel</button>
          </td>
        </tr>
      `;
    }).join('');

    if (kWarning) kWarning.textContent = warningCount;

    document.querySelectorAll('.btn-cancel-p2p').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        const offer = offers[idx];
        const label = usingMockData ? offer.id : offer.id.substring(0, 8);
        const assetLabel = usingMockData ? offer.asset : (offer.asset_name || 'Asset');
        const priceLabel = usingMockData ? offer.offerPrice.toFixed(2) : (offer.price_cents / 100).toFixed(2);

        if (typeof mpModal === 'function') {
          mpModal({
            title: 'Cancel P2P Offer',
            subtitle: `${label} — ${assetLabel} @ $${priceLabel}`,
            bodyHTML: `
              <div class="admin-form-group">
                <label class="admin-form-label">Cancellation Reason *</label>
                <textarea class="admin-textarea" id="p2p-cancel-reason" placeholder="e.g. Price significantly deviates from market…"></textarea>
              </div>
            `,
            confirmLabel: 'Cancel Offer',
            onConfirm: (overlay) => {
              const reason = overlay.querySelector('#p2p-cancel-reason')?.value?.trim();
              if (!reason) {
                mpToast('Please provide a reason', 'error');
                return;
              }
              const row = document.getElementById(`p2p-row-${idx}`);
              if (row) {
                row.style.transition = 'opacity 0.3s';
                row.style.opacity = '0';
                setTimeout(() => row.remove(), 300);
              }
              mpToast(`P2P offer ${label} cancelled`, 'success');
            }
          });
        }
      });
    });
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadP2P() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      offers = await res.json();
      usingMockData = false;
    } catch (err) {
      console.warn('[mp-p2p] API unavailable, using mock data:', err);
      offers = [...MOCK_OFFERS];
      usingMockData = true;
    }
    render();
  }

  document.addEventListener('DOMContentLoaded', loadP2P);
})();
