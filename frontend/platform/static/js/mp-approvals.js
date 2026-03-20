/**
 * Pending Approvals — mp-approvals.js
 * Card-based large order approvals with loading + disappear animation.
 */
(function () {
  'use strict';

  let approvals = [
    {
      id: 'APR-001',
      user: 'USR-8291',
      userName: 'Daniel Hartono',
      asset: 'Bali Villa Resort (BVRT)',
      side: 'BUY',
      qty: 2500,
      price: 52.40,
      totalSupply: 10000,
      supplyPct: 25,
      reason: 'Exceeds 20% single-holder threshold',
      created: '2h ago',
    },
    {
      id: 'APR-002',
      user: 'USR-3384',
      userName: 'Sri Widodo',
      asset: 'Jakarta Office Tower (JOTX)',
      side: 'BUY',
      qty: 800,
      price: 105.00,
      totalSupply: 3000,
      supplyPct: 26.7,
      reason: 'Order value >$50,000 requires manual review',
      created: '5h ago',
    },
    {
      id: 'APR-003',
      user: 'USR-6643',
      userName: 'Rina Kusuma',
      asset: 'Surabaya Warehouse (SWHS)',
      side: 'SELL',
      qty: 4000,
      price: 23.75,
      totalSupply: 15000,
      supplyPct: 26.7,
      reason: 'Large sell — >20% of outstanding supply',
      created: '8h ago',
    },
  ];

  function render() {
    const grid = document.getElementById('approvals-grid');
    const empty = document.getElementById('approvals-empty');
    if (!grid) return;

    if (approvals.length === 0) {
      grid.style.display = 'none';
      empty.style.display = 'block';
      return;
    }
    grid.style.display = 'grid';
    empty.style.display = 'none';

    grid.innerHTML = approvals.map((a, idx) => {
      const total = (a.qty * a.price).toLocaleString(undefined, { style: 'currency', currency: 'USD' });
      const sideClass = a.side === 'BUY' ? 'mp-side-buy' : 'mp-side-sell';
      return `
        <div class="mp-approval-card" data-idx="${idx}" id="approval-${idx}" style="animation-delay:${idx * 0.08}s;">
          <div style="display:flex; align-items:flex-start; justify-content:space-between; gap:16px; flex-wrap:wrap;">
            <div>
              <div style="display:flex; align-items:center; gap:10px; margin-bottom:8px;">
                <code style="font-size:12px; padding:3px 8px; background:var(--admin-code-bg); border-radius:4px;">${a.id}</code>
                <span class="admin-badge admin-badge--info"><span class="admin-badge-dot"></span>Pending Review</span>
                <span style="font-size:12px; color:var(--admin-text-muted);">${a.created}</span>
              </div>
              <h3 style="font-size:18px; font-weight:700; color:var(--admin-text-primary); margin:0 0 4px;">${a.asset}</h3>
              <p style="font-size:13px; color:var(--admin-text-secondary); margin:0;">
                <code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${a.user}</code>
                ${a.userName} wants to <span class="${sideClass}" style="font-weight:700;">${a.side}</span>
                <strong>${a.qty.toLocaleString()}</strong> tokens @ <strong>$${a.price.toFixed(2)}</strong>
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
            ⚠️ This order accounts for ${a.supplyPct}% of total supply (${a.totalSupply.toLocaleString()} tokens). ${a.reason}.
          </div>
          <div style="display:grid; grid-template-columns:repeat(4,1fr); gap:12px; margin-top:12px;">
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Quantity</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">${a.qty.toLocaleString()}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Unit Price</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">$${a.price.toFixed(2)}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Total Value</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin-top:2px;">${total}</div>
            </div>
            <div>
              <div style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Supply Impact</div>
              <div style="font-size:16px; font-weight:700; color:var(--admin-danger); margin-top:2px;">${a.supplyPct}%</div>
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

  function handleAction(idx, action) {
    const card = document.getElementById(`approval-${idx}`);
    const btn = card?.querySelector(action === 'approve' ? '.btn-approve' : '.btn-reject');
    if (!btn || btn.classList.contains('mp-btn-loading')) return;

    // Show spinner
    const originalHTML = btn.innerHTML;
    btn.classList.add('mp-btn-loading');
    btn.innerHTML = `<span class="mp-btn-text">${originalHTML}</span>`;

    setTimeout(() => {
      // Fade out card
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
    }, 1000);
  }

  document.addEventListener('DOMContentLoaded', render);
})();
