/**
 * P2P Offers — mp-p2p.js
 * Mock P2P offers with price deviation warnings.
 */
(function () {
  'use strict';

  const offers = [
    { id: 'P2P-001', seller: 'USR-8291', asset: 'Bali Villa Resort (BVRT)', qty: 200, offerPrice: 58.00, marketPrice: 52.40, created: '1h ago' },
    { id: 'P2P-002', seller: 'USR-3384', asset: 'Jakarta Office Tower (JOTX)', qty: 50,  offerPrice: 108.00, marketPrice: 105.00, created: '3h ago' },
    { id: 'P2P-003', seller: 'USR-6643', asset: 'Surabaya Warehouse (SWHS)', qty: 800, offerPrice: 30.00, marketPrice: 23.75, created: '5h ago' },
    { id: 'P2P-004', seller: 'USR-1738', asset: 'Bandung Tech Hub (BTHB)', qty: 100, offerPrice: 88.00, marketPrice: 87.20, created: '6h ago' },
    { id: 'P2P-005', seller: 'USR-5561', asset: 'Yogya Heritage Hotel (YHHT)', qty: 300, offerPrice: 42.00, marketPrice: 34.90, created: '8h ago' },
    { id: 'P2P-006', seller: 'USR-2201', asset: 'Bali Villa Resort (BVRT)', qty: 150, offerPrice: 51.00, marketPrice: 52.40, created: '12h ago' },
    { id: 'P2P-007', seller: 'USR-7829', asset: 'Jakarta Office Tower (JOTX)', qty: 40,  offerPrice: 130.00, marketPrice: 105.00, created: '14h ago' },
    { id: 'P2P-008', seller: 'USR-4410', asset: 'Surabaya Warehouse (SWHS)', qty: 600, offerPrice: 24.00, marketPrice: 23.75, created: '1d ago' },
  ];

  function render() {
    const tbody = document.getElementById('p2p-body');
    if (!tbody) return;

    tbody.innerHTML = offers.map((o, i) => {
      const devPct = (((o.offerPrice - o.marketPrice) / o.marketPrice) * 100).toFixed(1);
      const devAbs = Math.abs(parseFloat(devPct));
      const isWarning = devAbs > 20;
      const devSign = parseFloat(devPct) >= 0 ? '+' : '';

      let devHTML;
      if (isWarning) {
        devHTML = `<span class="mp-price-warning">⚠️ ${devSign}${devPct}%</span>`;
      } else if (devAbs > 5) {
        devHTML = `<span class="admin-badge admin-badge--warning">${devSign}${devPct}%</span>`;
      } else {
        devHTML = `<span class="admin-badge admin-badge--success">${devSign}${devPct}%</span>`;
      }

      return `
        <tr id="p2p-row-${i}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${o.id}</code></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${o.seller}</code></td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${o.asset}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${o.qty.toLocaleString()}</td>
          <td style="text-align:right; font-weight:600; font-variant-numeric:tabular-nums;">$${o.offerPrice.toFixed(2)}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums; color:var(--admin-text-muted);">$${o.marketPrice.toFixed(2)}</td>
          <td>${devHTML}</td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${o.created}</td>
          <td style="text-align:center;">
            <button class="admin-btn admin-btn--danger admin-btn--sm btn-cancel-p2p" data-idx="${i}">Admin Cancel</button>
          </td>
        </tr>
      `;
    }).join('');

    document.querySelectorAll('.btn-cancel-p2p').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        const offer = offers[idx];
        mpModal({
          title: 'Cancel P2P Offer',
          subtitle: `${offer.id} — ${offer.asset} @ $${offer.offerPrice.toFixed(2)}`,
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
            mpToast(`P2P offer ${offer.id} cancelled`, 'success');
          }
        });
      });
    });
  }

  document.addEventListener('DOMContentLoaded', render);
})();
