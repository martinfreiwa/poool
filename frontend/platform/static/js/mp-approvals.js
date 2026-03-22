/**
 * Pending Approvals — mp-approvals.js
 * Fetches orders with status=pending_review from the backend API.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/approvals';
  let approvals = [];
  let usingMockData = false;

  // ── Mock Data Fallback ──────────────────────────────────────────
  const MOCK_APPROVALS = [
    {
      id: 'APR-001', user: 'USR-8291', userName: 'Daniel Hartono',
      asset: 'Bali Villa Resort (BVRT)', side: 'BUY', qty: 2500, price: 52.40,
      totalSupply: 10000, supplyPct: 25, reason: 'Exceeds 20% single-holder threshold', created: '2h ago',
    },
    {
      id: 'APR-002', user: 'USR-3384', userName: 'Sri Widodo',
      asset: 'Jakarta Office Tower (JOTX)', side: 'BUY', qty: 800, price: 105.00,
      totalSupply: 3000, supplyPct: 26.7, reason: 'Order value >$50,000 requires manual review', created: '5h ago',
    },
    {
      id: 'APR-003', user: 'USR-6643', userName: 'Rina Kusuma',
      asset: 'Surabaya Warehouse (SWHS)', side: 'SELL', qty: 4000, price: 23.75,
      totalSupply: 15000, supplyPct: 26.7, reason: 'Large sell — >20% of outstanding supply', created: '8h ago',
    },
  ];

  function render() {
    const grid = document.getElementById('approvals-grid');
    const empty = document.getElementById('approvals-empty');
    if (!grid) return;

    // Update KPI
    const kpi = document.getElementById('kpi-pending-count');
    if (kpi) kpi.textContent = approvals.length;

    if (approvals.length === 0) {
      grid.style.display = 'none';
      if (empty) empty.style.display = 'block';
      return;
    }
    grid.style.display = 'grid';
    if (empty) empty.style.display = 'none';

    grid.innerHTML = approvals.map((a, idx) => {
      // Normalize fields for API vs mock
      let orderId, user, userName, asset, side, qty, price, total, supplyPct, reason, created;
      if (usingMockData) {
        orderId = a.id; user = a.user; userName = a.userName; asset = a.asset;
        side = a.side; qty = a.qty; price = a.price;
        total = (a.qty * a.price).toLocaleString(undefined, { style: 'currency', currency: 'USD' });
        supplyPct = a.supplyPct; reason = a.reason; created = a.created;
      } else {
        orderId = a.id.substring(0, 8);
        user = a.user_id.substring(0, 8);
        userName = a.user_email ? a.user_email.split('@')[0] : user;
        asset = a.asset_name || a.asset_id.substring(0, 8);
        side = a.side.toUpperCase();
        qty = a.quantity;
        price = a.price_cents / 100;
        total = (a.total_value_cents / 100).toLocaleString(undefined, { style: 'currency', currency: 'USD' });
        supplyPct = '—';
        reason = 'Flagged for admin review';
        created = timeAgo(a.created_at);
      }
      const sideClass = side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      return `
        <div class="mp-approval-card" data-idx="${idx}" id="approval-${idx}" style="animation-delay:${idx * 0.08}s;">
          <div style="display:flex; align-items:flex-start; justify-content:space-between; gap:16px; flex-wrap:wrap;">
            <div>
              <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
                <code style="font-size:12px; padding:3px 8px; background:var(--admin-code-bg); border-radius:4px;">${orderId}</code>
                <span class="admin-badge admin-badge--info"><span class="admin-badge-dot"></span>Pending Review</span>
                <span style="font-size:12px; color:var(--admin-text-muted);">${created}</span>
              </div>
              <h3 style="font-size:18px; font-weight:700; color:var(--admin-text-primary); margin:0 0 4px;">${asset}</h3>
              <p style="font-size:13px; color:var(--admin-text-secondary); margin:0;">
                <code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${user}</code>
                ${userName} wants to <span class="${sideClass}" style="font-weight:700;">${side}</span>
                <strong>${qty.toLocaleString()}</strong> tokens @ <strong>$${price.toFixed(2)}</strong>
                = <strong>${total}</strong>
              </p>
            </div>
            <div style="display:flex; gap:10px; align-items:flex-start;">
              <button class="admin-btn admin-btn--success btn-approve" data-idx="${idx}" style="padding:10px 24px; font-size:14px; font-weight:600;">
                ✓ Approve
              </button>
              <button class="admin-btn admin-btn--danger btn-reject" data-idx="${idx}" style="padding:10px 24px; font-size:14px; font-weight:600;">
                ✕ Reject
              </button>
            </div>
          </div>
          <div class="mp-approval-warning">
            ⚠️ ${reason}.
          </div>
          <div style="display:grid; grid-template-columns:repeat(4,1fr); gap:12px; margin-top:12px;">
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Quantity</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">${qty.toLocaleString()}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Unit Price</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">$${price.toFixed(2)}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Total Value</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">${total}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Supply Impact</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-danger); margin-top:2px;">${supplyPct}%</div>
            </div>
          </div>
        </div>
      `;
    }).join('');

    // Bind buttons
    document.querySelectorAll('.btn-approve').forEach(btn => {
      btn.addEventListener('click', () => handleAction(parseInt(btn.dataset.idx), 'approve'));
    });
    document.querySelectorAll('.btn-reject').forEach(btn => {
      btn.addEventListener('click', () => handleAction(parseInt(btn.dataset.idx), 'reject'));
    });
  }

  function timeAgo(dateStr) {
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  async function handleAction(idx, action) {
    const card = document.getElementById(`approval-${idx}`);
    const btn = card?.querySelector(action === 'approve' ? '.btn-approve' : '.btn-reject');
    if (!btn || btn.classList.contains('mp-btn-loading')) return;

    const originalHTML = btn.innerHTML;
    btn.classList.add('mp-btn-loading');
    btn.innerHTML = `<span class="mp-btn-text">${originalHTML}</span>`;

    const approval = approvals[idx];

    // Try real API call
    if (!usingMockData) {
      try {
        const realId = approval.id; // UUID from backend
        const res = await fetch(`${API}/${realId}/${action}`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ reason: `Admin ${action}ed` }),
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.error || `HTTP ${res.status}`);
        }
        // Success — animate out
        animateRemoval(card, idx, action);
        return;
      } catch (err) {
        btn.classList.remove('mp-btn-loading');
        btn.innerHTML = originalHTML;
        mpToast(`Failed: ${err.message}`, 'error');
        return;
      }
    }

    // Mock path
    setTimeout(() => animateRemoval(card, idx, action), 1000);
  }

  function animateRemoval(card, idx, action) {
    card.style.transition = 'all 0.4s ease';
    card.style.opacity = '0';
    card.style.transform = 'translateX(40px)';
    card.style.maxHeight = card.scrollHeight + 'px';

    setTimeout(() => {
      card.style.maxHeight = '0';
      card.style.padding = '0';
      card.style.margin = '0';
      card.style.overflow = 'hidden';

      setTimeout(() => {
        approvals.splice(idx, 1);
        render();
      }, 300);
    }, 300);

    if (action === 'approve') {
      mpToast(`Order approved — execution queued`, 'success');
    } else {
      mpToast(`Order rejected — user will be notified`, 'warning');
    }
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadApprovals() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      approvals = await res.json();
      usingMockData = false;
    } catch (err) {
      console.warn('[mp-approvals] API unavailable, using mock data:', err);
      approvals = [...MOCK_APPROVALS];
      usingMockData = true;
    }
    render();
  }

  document.addEventListener('DOMContentLoaded', loadApprovals);
})();
