/**
 * Fee Management — mp-fees.js
 * Fetches fee configurations and promotions from backend API.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/fees';

  // ── Mock Data ───────────────────────────────────────────────────
  const MOCK_ASSET_FEES = [
    { asset: 'Bali Villa Resort (BVRT)', taker: '0.30%', maker: '0.00%', settlement: '0.05%', reason: 'High liquidity asset — reduced fees' },
    { asset: 'Jakarta Office Tower (JOTX)', taker: '0.50%', maker: '0.00%', settlement: '0.10%', reason: 'Standard' },
    { asset: 'Surabaya Warehouse (SWHS)', taker: '0.75%', maker: '0.10%', settlement: '0.15%', reason: 'Low liquidity — higher spread compensation' },
  ];
  const MOCK_PROMOS = [
    { name: 'Launch Special', desc: '0% trading fees for all assets', discount: '100%', badge: 'Active', validUntil: 'Apr 30, 2026', color: 'success' },
    { name: 'BVRT Liquidity Boost', desc: 'Reduced taker fee to 0.1% for Bali Villa Resort', discount: '80%', badge: 'Active', validUntil: 'May 15, 2026', color: 'info' },
    { name: 'New User Welcome', desc: 'First 3 trades free for new marketplace users', discount: 'First 3 free', badge: 'Active', validUntil: 'Jun 01, 2026', color: 'success' },
  ];

  let usingMockData = false;

  // ── Tab Switching ───────────────────────────────────────────────
  function initTabs() {
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
  }

  // ── Render Asset Fee Configs ────────────────────────────────────
  function renderAssetFees(configs) {
    const tbody = document.getElementById('asset-fees-body');
    if (!tbody) return;

    if (usingMockData) {
      tbody.innerHTML = MOCK_ASSET_FEES.map((f, i) => `
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
    } else {
      tbody.innerHTML = configs.map((c, i) => {
        const takerPct = (c.taker_fee_bps / 100).toFixed(2) + '%';
        const makerPct = (c.maker_fee_bps / 100).toFixed(2) + '%';
        const scope = c.scope.charAt(0).toUpperCase() + c.scope.slice(1);
        const label = c.asset_id ? c.asset_id.substring(0, 8) : scope;
        return `
          <tr>
            <td style="font-weight:600; color:var(--admin-text-primary);">${label}</td>
            <td><span class="admin-badge admin-badge--info">${takerPct}</span></td>
            <td><span class="admin-badge admin-badge--success">${makerPct}</span></td>
            <td><span class="admin-badge admin-badge--neutral">${scope}</span></td>
            <td style="font-size:12px; color:var(--admin-text-muted);">${c.reason || '—'}</td>
            <td style="text-align:center;">
              <span class="admin-badge ${c.is_active ? 'admin-badge--success' : 'admin-badge--neutral'}">${c.is_active ? 'Active' : 'Inactive'}</span>
            </td>
          </tr>
        `;
      }).join('');
    }

    bindRemoveButtons();
  }

  function bindRemoveButtons() {
    document.querySelectorAll('.btn-remove-fee').forEach(btn => {
      btn.addEventListener('click', function () {
        if (typeof mpButtonAction === 'function') {
          mpButtonAction(this, 'Fee override removed', 800, () => {
            const tr = this.closest('tr');
            tr.style.transition = 'opacity 0.3s';
            tr.style.opacity = '0';
            setTimeout(() => tr.remove(), 300);
          });
        }
      });
    });
  }

  // ── Render Promotions ───────────────────────────────────────────
  function renderPromotions(promos) {
    const grid = document.getElementById('promos-grid');
    if (!grid) return;

    if (usingMockData) {
      grid.innerHTML = MOCK_PROMOS.map((p, i) => `
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
    } else {
      grid.innerHTML = promos.map((p, i) => {
        const takerPct = (p.taker_fee_bps / 100).toFixed(2) + '%';
        const makerPct = (p.maker_fee_bps / 100).toFixed(2) + '%';
        const endDate = new Date(p.ends_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
        const isActive = p.is_active && new Date(p.ends_at) > new Date();
        return `
          <div class="mp-promo-card" id="promo-${i}">
            <div class="mp-promo-badge">● ${isActive ? 'Active' : 'Expired'}</div>
            <h4 style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin:0 0 6px;">${p.name}</h4>
            <p style="font-size:13px; color:var(--admin-text-secondary); margin:0 0 12px;">Taker: ${takerPct} · Maker: ${makerPct}</p>
            <div style="display:flex; align-items:center; justify-content:space-between;">
              <div>
                <span style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Valid until</span>
                <div style="font-size:13px; font-weight:600; color:var(--admin-text-primary);">${endDate}</div>
              </div>
              <button class="admin-btn admin-btn--danger admin-btn--sm btn-deactivate-promo" data-idx="${i}">Deactivate</button>
            </div>
          </div>
        `;
      }).join('');
    }

    document.querySelectorAll('.btn-deactivate-promo').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        if (typeof mpButtonAction === 'function') {
          mpButtonAction(this, 'Promotion deactivated', 1000, () => {
            const card = document.getElementById(`promo-${idx}`);
            if (card) {
              card.style.transition = 'all 0.3s ease';
              card.style.opacity = '0';
              card.style.transform = 'scale(0.95)';
              setTimeout(() => card.remove(), 300);
            }
          });
        }
      });
    });
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadFees() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      usingMockData = false;
      renderAssetFees(data.configurations);
      renderPromotions(data.promotions);
    } catch (err) {
      console.warn('[mp-fees] API unavailable, using mock data:', err);
      usingMockData = true;
      renderAssetFees([]);
      renderPromotions([]);
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    loadFees();

    document.getElementById('btn-save-defaults')?.addEventListener('click', function () {
      if (typeof mpButtonAction === 'function') {
        mpButtonAction(this, 'Default fees saved successfully', 1000);
      }
    });

    document.getElementById('btn-add-override')?.addEventListener('click', () => {
      if (typeof mpToast === 'function') mpToast('Override form — connect to backend', 'info');
    });
  });
})();
