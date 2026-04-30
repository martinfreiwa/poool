/**
 * Fee Management — mp-fees.js
 * Fetches fee configurations and promotions from backend API.
 * Shows an error state if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/fees';
  const REWARDS_API = '/api/admin/rewards';

  function esc(s) {
    return String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

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

    tbody.innerHTML = configs.map((c) => {
      const takerPct = (c.taker_fee_bps / 100).toFixed(2) + '%';
      const makerPct = (c.maker_fee_bps / 100).toFixed(2) + '%';
      const scope = esc(c.scope.charAt(0).toUpperCase() + c.scope.slice(1));
      const label = c.asset_id ? esc(c.asset_id.substring(0, 8)) : scope;
      const actionCell = c.is_active
        ? `<button class="admin-btn admin-btn--danger admin-btn--sm btn-remove-fee" data-fee-id="${esc(c.id)}">Deactivate</button>`
        : `<span class="admin-badge admin-badge--neutral">Inactive</span>`;
      return `
        <tr>
          <td style="font-weight:600; color:var(--admin-text-primary);">${label}</td>
          <td><span class="admin-badge admin-badge--info">${takerPct}</span></td>
          <td><span class="admin-badge admin-badge--success">${makerPct}</span></td>
          <td><span class="admin-badge admin-badge--neutral">${scope}</span></td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${esc(c.reason || '—')}</td>
          <td style="text-align:center;">${actionCell}</td>
        </tr>
      `;
    }).join('');

    bindRemoveButtons();
  }

  function bindRemoveButtons() {
    document.querySelectorAll('.btn-remove-fee').forEach(btn => {
      btn.addEventListener('click', async function () {
        const feeId = this.dataset.feeId;
        if (!feeId) return;
        this.disabled = true;
        this.textContent = '…';
        try {
          const res = await fetch(`/api/admin/marketplace/fees/${feeId}`, {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: { 'X-CSRF-Token': document.cookie.match(/csrf_token=([^;]+)/)?.[1] || '' },
          });
          if (res.ok) {
            if (typeof mpToast === 'function') mpToast('Fee configuration deactivated.', 'success');
            const tr = this.closest('tr');
            tr.style.transition = 'opacity 0.3s';
            tr.style.opacity = '0';
            setTimeout(() => tr.remove(), 300);
          } else {
            const err = await res.json().catch(() => ({}));
            if (typeof mpToast === 'function') mpToast(err.error || `Error (${res.status}).`, 'error');
            this.disabled = false;
            this.textContent = 'Deactivate';
          }
        } catch (e) {
          if (typeof mpToast === 'function') mpToast('Network error.', 'error');
          this.disabled = false;
          this.textContent = 'Deactivate';
        }
      });
    });
  }

  // ── Render Promotions ───────────────────────────────────────────
  function renderPromotions(promos) {
    const grid = document.getElementById('promos-grid');
    if (!grid) return;

    grid.innerHTML = promos.map((p, i) => {
        const takerPct = (p.taker_fee_bps / 100).toFixed(2) + '%';
        const makerPct = (p.maker_fee_bps / 100).toFixed(2) + '%';
        const endDate = new Date(p.ends_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
        const isActive = p.is_active && new Date(p.ends_at) > new Date();
        const deactivateBtn = isActive
          ? `<button class="admin-btn admin-btn--danger admin-btn--sm btn-deactivate-promo" data-promo-id="${esc(p.id)}" data-idx="${i}">Deactivate</button>`
          : `<span class="admin-badge admin-badge--neutral">Expired</span>`;
        return `
          <div class="mp-promo-card" id="promo-${i}">
            <div class="mp-promo-badge">● ${isActive ? 'Active' : 'Expired'}</div>
            <h4 style="font-size:16px; font-weight:700; color:var(--admin-text-primary); margin:0 0 6px;">${esc(p.name)}</h4>
            <p style="font-size:13px; color:var(--admin-text-secondary); margin:0 0 12px;">Taker: ${takerPct} · Maker: ${makerPct}</p>
            <div style="display:flex; align-items:center; justify-content:space-between;">
              <div>
                <span style="font-size:11px; color:var(--admin-text-muted); text-transform:uppercase; letter-spacing:0.5px; font-weight:600;">Valid until</span>
                <div style="font-size:13px; font-weight:600; color:var(--admin-text-primary);">${endDate}</div>
              </div>
              ${deactivateBtn}
            </div>
          </div>
        `;
    }).join('');

    document.querySelectorAll('.btn-deactivate-promo').forEach(btn => {
      btn.addEventListener('click', async function () {
        const promoId = this.dataset.promoId;
        const idx = this.dataset.idx;
        if (!promoId) return;
        this.disabled = true;
        this.textContent = '…';
        try {
          const res = await fetch(`/api/admin/marketplace/promotions/${promoId}`, {
            method: 'DELETE',
            credentials: 'same-origin',
            headers: { 'X-CSRF-Token': document.cookie.match(/csrf_token=([^;]+)/)?.[1] || '' },
          });
          if (res.ok) {
            if (typeof mpToast === 'function') mpToast('Promotion deactivated.', 'success');
            const card = document.getElementById(`promo-${idx}`);
            if (card) {
              card.style.transition = 'all 0.3s ease';
              card.style.opacity = '0';
              card.style.transform = 'scale(0.95)';
              setTimeout(() => card.remove(), 300);
            }
          } else {
            const err = await res.json().catch(() => ({}));
            if (typeof mpToast === 'function') mpToast(err.error || `Error (${res.status}).`, 'error');
            this.disabled = false;
            this.textContent = 'Deactivate';
          }
        } catch (e) {
          if (typeof mpToast === 'function') mpToast('Network error.', 'error');
          this.disabled = false;
          this.textContent = 'Deactivate';
        }
      });
    });
  }

  // ── Render Tiers ────────────────────────────────────────────────
  function renderTiers(tiers) {
    const tbody = document.getElementById('tier-fees-body');
    if (!tbody) return;

    if (!tiers || tiers.length === 0) return;

    const defaultTakerFee = 5.00; // From "Platform Default Fees"

    tbody.innerHTML = tiers.map((t) => {
      const minInvestStr = t.min_invest > 0 ? '$' + (t.min_invest / 100).toLocaleString('en-US') : '$0';
      const discountVal = (t.cashback_pct || 0) / 100;
      const effectiveFee = Math.max(0, defaultTakerFee - discountVal).toFixed(2);
      const rawColor = t.badge_color || '#9ca3af';
      const badgeColor = /^#[0-9a-fA-F]{3,6}$/.test(rawColor) ? rawColor : '#9ca3af';

      return `
        <tr>
          <td>
            <span style="display:inline-flex;align-items:center;gap:6px;">
              <span style="width:10px;height:10px;border-radius:50%;background:${badgeColor};display:inline-block;"></span>
              ${esc(t.name)}
            </span>
          </td>
          <td><input type="text" class="admin-input admin-input--sm" value="${minInvestStr}" style="width:100px;"></td>
          <td><input type="number" class="admin-input admin-input--sm" value="${discountVal.toFixed(2)}" step="0.1" min="0" max="5" style="width:80px;"> %</td>
          <td><strong>${effectiveFee}%</strong></td>
          <td style="text-align:center"><span class="admin-badge admin-badge--active">Active</span></td>
        </tr>
      `;
    }).join('');
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadTiers() {
    try {
      const res = await fetch(REWARDS_API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderTiers(data.tiers || []);
    } catch (err) {
      console.warn('[mp-fees] Tiers API unavailable:', err);
      const tbody = document.getElementById('tier-fees-body');
      if (tbody) tbody.innerHTML = `<tr><td colspan="5" style="text-align:center;color:#c00;">Failed to load tier data.</td></tr>`;
    }
  }

  async function loadFees() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderAssetFees(data.configurations);
      renderPromotions(data.promotions);
    } catch (err) {
      console.warn('[mp-fees] API unavailable:', err);
      const tbody = document.getElementById('asset-fees-body');
      if (tbody) tbody.innerHTML = `<tr><td colspan="6" style="text-align:center;color:#c00;">Failed to load fee configurations.</td></tr>`;
      const grid = document.getElementById('promos-grid');
      if (grid) grid.innerHTML = `<p style="color:#c00;">Failed to load promotions.</p>`;
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    loadFees();
    loadTiers();

    document.getElementById('btn-save-defaults')?.addEventListener('click', async function () {
      const takerBps = Math.round(parseFloat(document.getElementById('fee-taker')?.value || '0') * 100);
      const makerBps = Math.round(parseFloat(document.getElementById('fee-maker')?.value || '0') * 100);

      if (isNaN(takerBps) || isNaN(makerBps)) {
        if (typeof mpToast === 'function') mpToast('Invalid fee values.', 'error');
        return;
      }

      this.disabled = true;
      const label = this.textContent;
      this.textContent = 'Saving…';

      try {
        const res = await fetch(API, {
          method: 'POST',
          credentials: 'same-origin',
          headers: {
            'Content-Type': 'application/json',
            'X-CSRF-Token': document.cookie.match(/csrf_token=([^;]+)/)?.[1] || '',
          },
          body: JSON.stringify({ scope: 'platform', taker_fee_bps: takerBps, maker_fee_bps: makerBps }),
        });
        if (res.ok) {
          if (typeof mpToast === 'function') mpToast('Default fees saved.', 'success');
          loadFees();
        } else {
          const err = await res.json().catch(() => ({}));
          if (typeof mpToast === 'function') mpToast(err.error || `Save failed (${res.status}).`, 'error');
        }
      } catch (e) {
        if (typeof mpToast === 'function') mpToast('Network error. Changes not saved.', 'error');
      } finally {
        this.disabled = false;
        this.textContent = label;
      }
    });

    document.getElementById('btn-add-override')?.addEventListener('click', () => {
      if (typeof mpToast === 'function') mpToast('Fee override creation is not yet available.', 'info');
    });

    document.getElementById('btn-save-tiers')?.addEventListener('click', () => {
      if (typeof mpToast === 'function') mpToast('Tier discount configuration is read-only. Contact engineering to adjust tier thresholds.', 'info');
    });

    // Settlement and min fee fields are informational — no backend column yet
    ['fee-settlement', 'fee-min'].forEach(id => {
      const el = document.getElementById(id);
      if (el) {
        el.disabled = true;
        el.title = 'Not yet configurable via this interface';
        el.style.opacity = '0.5';
      }
    });
  });
})();
