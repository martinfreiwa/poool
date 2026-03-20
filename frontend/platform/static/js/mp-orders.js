/**
 * Open Orders — mp-orders.js
 * Mock data for open orders with cancel modal (reason required).
 */
(function () {
  'use strict';

  const ASSETS = ['Bali Villa Resort (BVRT)', 'Jakarta Office Tower (JOTX)', 'Surabaya Warehouse (SWHS)', 'Bandung Tech Hub (BTHB)', 'Yogya Heritage Hotel (YHHT)'];
  const USERS = ['USR-8291', 'USR-3384', 'USR-6643', 'USR-1738', 'USR-5561', 'USR-2201', 'USR-7829', 'USR-4410', 'USR-9203', 'USR-1105', 'USR-7712', 'USR-2290'];
  const PRICES = [52.40, 105.00, 23.75, 87.20, 34.90];

  let orders = [];
  for (let i = 0; i < 18; i++) {
    const assetIdx = i % ASSETS.length;
    const price = PRICES[assetIdx] + (Math.random() - 0.5) * 3;
    const qty = Math.floor(Math.random() * 300) + 20;
    const held = +(price * qty).toFixed(2);
    const hoursAgo = Math.floor(Math.random() * 72) + 1;
    const side = i % 3 === 0 ? 'SELL' : 'BUY';

    orders.push({
      id: `ORD-${(200000 + i).toString()}`,
      user: USERS[i % USERS.length],
      asset: ASSETS[assetIdx],
      side,
      type: i % 4 === 0 ? 'Market' : 'Limit',
      qty,
      price: +price.toFixed(2),
      held,
      created: `${hoursAgo}h ago`,
      hoursAgo,
      status: 'open',
    });
  }

  function render() {
    // KPIs
    const totalHeld = orders.reduce((s, o) => s + o.held, 0);
    const avgAge = orders.length > 0 ? (orders.reduce((s, o) => s + o.hoursAgo, 0) / orders.length).toFixed(1) : 0;
    document.getElementById('kpi-total-open').textContent = orders.length;
    document.getElementById('kpi-held-balance').textContent = '$' + totalHeld.toLocaleString(undefined, { minimumFractionDigits: 2 });
    document.getElementById('kpi-avg-age').textContent = avgAge + 'h';

    // Table
    const tbody = document.getElementById('orders-body');
    if (!tbody) return;

    tbody.innerHTML = orders.map((o, idx) => {
      const sideClass = o.side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      return `
        <tr data-order-idx="${idx}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${o.id}</code></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${o.user}</code></td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${o.asset}</td>
          <td><span class="${sideClass}">${o.side}</span></td>
          <td><span class="admin-badge admin-badge--neutral">${o.type}</span></td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${o.qty.toLocaleString()}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">$${o.price.toFixed(2)}</td>
          <td style="text-align:right;">
            <span class="admin-badge admin-badge--warning" style="font-variant-numeric:tabular-nums;">
              <span class="admin-badge-dot"></span>
              $${o.held.toLocaleString()}
            </span>
          </td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${o.created}</td>
          <td><span class="admin-badge admin-badge--info"><span class="admin-badge-dot"></span>Open</span></td>
          <td style="text-align:center;">
            <button class="admin-btn admin-btn--danger admin-btn--sm btn-cancel-order" data-idx="${idx}">Cancel Order</button>
          </td>
        </tr>
      `;
    }).join('');

    // Bind cancel buttons
    document.querySelectorAll('.btn-cancel-order').forEach(btn => {
      btn.addEventListener('click', () => {
        const idx = parseInt(btn.dataset.idx);
        const order = orders[idx];
        openCancelModal(order, idx);
      });
    });
  }

  function openCancelModal(order, idx) {
    mpModal({
      title: 'Cancel Order',
      subtitle: `Order ${order.id} — ${order.asset} (${order.side} ${order.qty} @ $${order.price.toFixed(2)})`,
      bodyHTML: `
        <div class="admin-form-group">
          <label class="admin-form-label">Reason for Cancellation *</label>
          <textarea class="admin-textarea" id="cancel-reason" placeholder="Enter the legal reason for this cancellation…" rows="3" style="min-height:80px;"></textarea>
        </div>
        <div style="display:flex; align-items:center; gap:8px; padding:10px 14px; background:var(--admin-danger-bg); border-radius:var(--admin-radius-sm); margin-top:8px;">
          <span style="color:var(--admin-danger); font-size:13px; font-weight:500;">⚠️ This will release the held balance of $${order.held.toLocaleString()} back to the user's wallet.</span>
        </div>
      `,
      confirmLabel: 'Cancel Order',
      confirmClass: 'admin-btn--danger',
      onConfirm: (overlay) => {
        const reason = overlay.querySelector('#cancel-reason')?.value?.trim();
        if (!reason) {
          mpToast('Please provide a cancellation reason', 'error');
          return;
        }
        // Remove order from list
        orders.splice(idx, 1);
        render();
        mpToast(`Order ${order.id} cancelled — "${reason}"`, 'success');
      }
    });
  }

  document.addEventListener('DOMContentLoaded', render);
})();
