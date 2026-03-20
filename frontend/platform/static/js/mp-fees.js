/**
 * Fee Management — mp-fees.js
 * Tab switching, asset-specific overrides, and promotion management.
 */
(function () {
  'use strict';

  // ===== TAB SWITCHING =====
  document.addEventListener('DOMContentLoaded', () => {
    const tabs = document.querySelectorAll('.admin-tab[data-tab]');
    tabs.forEach(tab => {
      tab.addEventListener('click', () => {
        tabs.forEach(t => t.classList.remove('active'));
        tab.classList.add('active');
        document.querySelectorAll('.mp-tab-content').forEach(c => c.classList.remove('active'));
        const target = document.getElementById(`tab-${tab.dataset.tab}`);
        if (target) target.classList.add('active');
      });
    });

    renderAssetFees();
    renderPromotions();

    // Save default fees
    document.getElementById('btn-save-defaults')?.addEventListener('click', function () {
      mpButtonAction(this, 'Default fees saved successfully', 1000);
    });

    // Add override
    document.getElementById('btn-add-override')?.addEventListener('click', () => {
      mpToast('Override form — connect to backend', 'info');
    });
  });

  // ===== ASSET-SPECIFIC FEES =====
  const assetFees = [
    { asset: 'Bali Villa Resort (BVRT)', taker: '0.30%', maker: '0.00%', settlement: '0.05%', reason: 'High liquidity asset — reduced fees' },
    { asset: 'Jakarta Office Tower (JOTX)', taker: '0.50%', maker: '0.00%', settlement: '0.10%', reason: 'Standard' },
    { asset: 'Surabaya Warehouse (SWHS)', taker: '0.75%', maker: '0.10%', settlement: '0.15%', reason: 'Low liquidity — higher spread compensation' },
  ];

  function renderAssetFees() {
    const tbody = document.getElementById('asset-fees-body');
    if (!tbody) return;
    tbody.innerHTML = assetFees.map((f, i) => `
      <tr>
        <td style="font-weight:600; color:var(--admin-text-primary);">${f.asset}</td>
        <td><span class="admin-badge admin-badge--info">${f.taker}</span></td>
        <td><span class="admin-badge admin-badge--success">${f.maker}</span></td>
        <td><span class="admin-badge admin-badge--neutral">${f.settlement}</span></td>
        <td style="font-size:12px; color:var(--admin-text-muted);">${f.reason}</td>
        <td style="text-align:center;">
          <button class="admin-btn admin-btn--danger admin-btn--sm btn-remove-fee" data-idx="${i}">Remove</button>
        </td>
      </tr>
    `).join('');

    document.querySelectorAll('.btn-remove-fee').forEach(btn => {
      btn.addEventListener('click', function () {
        mpButtonAction(this, 'Fee override removed', 800, () => {
          const tr = this.closest('tr');
          tr.style.transition = 'opacity 0.3s';
          tr.style.opacity = '0';
          setTimeout(() => tr.remove(), 300);
        });
      });
    });
  }

  // ===== PROMOTIONS =====
  let promos = [
    { name: 'Launch Special', desc: '0% trading fees for all assets', discount: '100%', badge: 'Active', validUntil: 'Apr 30, 2026', color: 'success' },
    { name: 'BVRT Liquidity Boost', desc: 'Reduced taker fee to 0.1% for Bali Villa Resort', discount: '80%', badge: 'Active', validUntil: 'May 15, 2026', color: 'info' },
    { name: 'New User Welcome', desc: 'First 3 trades free for new marketplace users', discount: 'First 3 free', badge: 'Active', validUntil: 'Jun 01, 2026', color: 'success' },
  ];

  function renderPromotions() {
    const grid = document.getElementById('promos-grid');
    if (!grid) return;

    grid.innerHTML = promos.map((p, i) => `
      <div class="mp-promo-card" id="promo-${i}">
        <div class="mp-promo-badge">● ${p.badge}</div>
        <h4 style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin:0 0 6px;">${p.name}</h4>
        <p style="font-size:13px; color:var(--admin-text-secondary); margin:0 0 12px;">${p.desc}</p>
        <div style="display:flex; align-items:center; justify-content:space-between;">
          <div>
            <span style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Valid until</span>
            <div style="font-size:13px; font-weight:600; color:var(--admin-text-primary);">${p.validUntil}</div>
          </div>
          <button class="admin-btn admin-btn--danger admin-btn--sm btn-deactivate-promo" data-idx="${i}">Deactivate</button>
        </div>
      </div>
    `).join('');

    document.querySelectorAll('.btn-deactivate-promo').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        mpButtonAction(this, `"${promos[idx].name}" promotion deactivated`, 1000, () => {
          const card = document.getElementById(`promo-${idx}`);
          if (card) {
            card.style.transition = 'all 0.3s ease';
            card.style.opacity = '0';
            card.style.transform = 'scale(0.95)';
            setTimeout(() => card.remove(), 300);
          }
        });
      });
    });
  }
})();
