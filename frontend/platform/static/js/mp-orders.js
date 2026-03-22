/**
 * Open Orders — mp-orders.js
 * Fetches open orders from the backend API with admin-cancel via DELETE.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/orders';
  const PAGE_SIZE = 25;
  let currentPage = 1;
  let totalPages = 1;
  let totalOrders = 0;
  let ordersData = [];
  let usingMockData = false;

  // ── Mock Data Fallback ──────────────────────────────────────────
  const ASSETS = ['Bali Villa Resort (BVRT)', 'Jakarta Office Tower (JOTX)', 'Surabaya Warehouse (SWHS)', 'Bandung Tech Hub (BTHB)', 'Yogya Heritage Hotel (YHHT)'];
  const USERS = ['USR-8291', 'USR-3384', 'USR-6643', 'USR-1738', 'USR-5561', 'USR-2201', 'USR-7829', 'USR-4410', 'USR-9203', 'USR-1105', 'USR-7712', 'USR-2290'];
  const PRICES = [52.40, 105.00, 23.75, 87.20, 34.90];

  function generateMockOrders() {
    const orders = [];
    for (let i = 0; i < 18; i++) {
      const assetIdx = i % ASSETS.length;
      const price = PRICES[assetIdx] + (Math.random() - 0.5) * 3;
      const qty = Math.floor(Math.random() * 300) + 20;
      const held = +(price * qty).toFixed(2);
      const hoursAgo = Math.floor(Math.random() * 72) + 1;
      orders.push({
        id: `ORD-${(200000 + i)}`, user: USERS[i % USERS.length], asset: ASSETS[assetIdx],
        side: i % 3 === 0 ? 'SELL' : 'BUY', type: i % 4 === 0 ? 'Market' : 'Limit',
        qty, price: +price.toFixed(2), held, created: `${hoursAgo}h ago`, hoursAgo, status: 'open',
      });
    }
    return orders;
  }

  // ── Render Table ────────────────────────────────────────────────
  function renderOrders(orders) {
    ordersData = orders;

    // KPIs
    let totalHeld, avgAge;
    if (usingMockData) {
      totalHeld = orders.reduce((s, o) => s + o.held, 0);
      avgAge = orders.length > 0 ? (orders.reduce((s, o) => s + o.hoursAgo, 0) / orders.length).toFixed(1) + 'h' : '0h';
    } else {
      totalHeld = orders.reduce((s, o) => s + (o.price_cents * (o.quantity - o.quantity_filled)), 0) / 100;
      // Calculate average age from created_at
      const now = Date.now();
      const ages = orders.map(o => (now - new Date(o.created_at).getTime()) / 3600000);
      avgAge = orders.length > 0 ? (ages.reduce((a, b) => a + b, 0) / ages.length).toFixed(1) + 'h' : '0h';
    }

    const kpiTotal = document.getElementById('kpi-total-open');
    const kpiHeld = document.getElementById('kpi-held-balance');
    const kpiAge = document.getElementById('kpi-avg-age');
    if (kpiTotal) kpiTotal.textContent = totalOrders || orders.length;
    if (kpiHeld) kpiHeld.textContent = '$' + totalHeld.toLocaleString(undefined, { minimumFractionDigits: 2 });
    if (kpiAge) kpiAge.textContent = avgAge;

    // Table
    const tbody = document.getElementById('orders-body');
    if (!tbody) return;

    if (!orders || orders.length === 0) {
      tbody.innerHTML = `<tr><td colspan="11" style="text-align:center; color:var(--admin-text-muted); padding:24px;">No open orders</td></tr>`;
      return;
    }

    tbody.innerHTML = orders.map((o, idx) => {
      let orderId, user, asset, side, type, qty, price, held, created, status;
      if (usingMockData) {
        orderId = o.id; user = o.user; asset = o.asset; side = o.side; type = o.type;
        qty = o.qty; price = o.price; held = o.held; created = o.created; status = o.status;
      } else {
        orderId = o.id.substring(0, 8);
        user = o.user_email ? o.user_email.split('@')[0] : o.user_id.substring(0, 8);
        asset = o.asset_name || o.asset_id.substring(0, 8);
        side = o.side.toUpperCase();
        type = o.order_type ? o.order_type.charAt(0).toUpperCase() + o.order_type.slice(1) : 'Limit';
        qty = o.quantity;
        price = (o.price_cents / 100).toFixed(2);
        const remaining = o.quantity - o.quantity_filled;
        held = ((o.price_cents * remaining) / 100).toFixed(2);
        created = timeAgo(o.created_at);
        status = o.status;
      }
      const sideClass = side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      const statusBadge = status === 'partially_filled'
        ? '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Partial</span>'
        : '<span class="admin-badge admin-badge--info"><span class="admin-badge-dot"></span>Open</span>';

      return `
        <tr data-order-idx="${idx}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${orderId}</code></td>
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${user}</code></td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${asset}</td>
          <td><span class="${sideClass}">${side}</span></td>
          <td><span class="admin-badge admin-badge--neutral">${type}</span></td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${typeof qty === 'number' ? qty.toLocaleString() : qty}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">$${price}</td>
          <td style="text-align:right;">
            <span class="admin-badge admin-badge--warning" style="font-variant-numeric:tabular-nums;">
              <span class="admin-badge-dot"></span>$${held}
            </span>
          </td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${created}</td>
          <td>${statusBadge}</td>
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
        openCancelModal(ordersData[idx], idx);
      });
    });
  }

  // ── Time Ago Helper ─────────────────────────────────────────────
  function timeAgo(dateStr) {
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  // ── Cancel Modal ────────────────────────────────────────────────
  function openCancelModal(order, idx) {
    const orderId = usingMockData ? order.id : order.id.substring(0, 8);
    const asset = usingMockData ? order.asset : (order.asset_name || 'Asset');
    const side = usingMockData ? order.side : order.side.toUpperCase();
    const qty = usingMockData ? order.qty : order.quantity;
    const price = usingMockData ? order.price.toFixed(2) : (order.price_cents / 100).toFixed(2);
    const held = usingMockData ? order.held : ((order.price_cents * (order.quantity - order.quantity_filled)) / 100).toFixed(2);

    mpModal({
      title: 'Cancel Order',
      subtitle: `Order ${orderId} — ${asset} (${side} ${qty} @ $${price})`,
      bodyHTML: `
        <div class="admin-form-group">
          <label class="admin-form-label">Reason for Cancellation *</label>
          <textarea class="admin-textarea" id="cancel-reason" placeholder="Enter the legal reason for this cancellation…" rows="3" style="min-height:80px;"></textarea>
        </div>
        <div style="display:flex; align-items:center; gap:8px; padding:10px 14px; background:var(--admin-danger-bg); border-radius:var(--admin-radius-sm); margin-top:8px;">
          <span style="color:var(--admin-danger); font-size:13px; font-weight:500;">⚠️ This will release the held balance of $${held} back to the user's wallet.</span>
        </div>
      `,
      confirmLabel: 'Cancel Order',
      confirmClass: 'admin-btn--danger',
      onConfirm: async (overlay) => {
        const reason = overlay.querySelector('#cancel-reason')?.value?.trim();
        if (!reason) {
          mpToast('Please provide a cancellation reason', 'error');
          return;
        }

        if (usingMockData) {
          ordersData.splice(idx, 1);
          renderOrders(ordersData);
          mpToast(`Order ${orderId} cancelled — "${reason}"`, 'success');
          return;
        }

        // Real API call
        try {
          const res = await fetch(`/api/admin/marketplace/orders/${order.id}`, {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ reason }),
          });
          if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || `HTTP ${res.status}`);
          }
          mpToast(`Order ${orderId} cancelled — "${reason}"`, 'success');
          loadOrders(); // Reload
        } catch (err) {
          mpToast(`Failed to cancel: ${err.message}`, 'error');
        }
      }
    });
  }

  // ── Load Orders ─────────────────────────────────────────────────
  async function loadOrders() {
    try {
      const res = await fetch(`${API}?page=${currentPage}&per_page=${PAGE_SIZE}`, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      usingMockData = false;
      totalOrders = data.total;
      totalPages = data.total_pages || 1;
      renderOrders(data.data);
    } catch (err) {
      console.warn('[mp-orders] API unavailable, using mock data:', err);
      usingMockData = true;
      const mocks = generateMockOrders();
      totalOrders = mocks.length;
      totalPages = 1;
      renderOrders(mocks);
    }
  }

  document.addEventListener('DOMContentLoaded', loadOrders);
})();
