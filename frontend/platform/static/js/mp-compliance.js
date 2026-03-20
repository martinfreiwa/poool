/**
 * Compliance — mp-compliance.js
 */
(function () {
  'use strict';

  const limits = [
    { tier: 'Tier 0 (Unverified)', dailyBuy: '$0', dailySell: '$0', maxOrder: '$0', maxHolding: '0%', p2p: false, highlight: false },
    { tier: 'Tier 1 (Basic)', dailyBuy: '$5,000', dailySell: '$2,000', maxOrder: '$1,000', maxHolding: '10%', p2p: false, highlight: false },
    { tier: 'Tier 2 (Enhanced)', dailyBuy: '$50,000', dailySell: '$25,000', maxOrder: '$10,000', maxHolding: '20%', p2p: true, highlight: true },
    { tier: 'Tier 3 (Accredited)', dailyBuy: 'Unlimited', dailySell: 'Unlimited', maxOrder: '$100,000', maxHolding: '49%', p2p: true, highlight: false },
  ];

  const rules = [
    { label: 'Require KYC before trading', enabled: true, desc: 'Users must have at least Tier 1 verification to place any order' },
    { label: 'Enforce cooling period on large trades', enabled: true, desc: '24h delay on orders exceeding 5% of asset supply' },
    { label: 'Auto-flag wash trading patterns', enabled: true, desc: 'Detect and alert on same-wallet buy/sell within 60 seconds' },
    { label: 'Block sanctioned jurisdictions', enabled: true, desc: 'Prevent trading from OFAC-sanctioned countries' },
    { label: 'Require legal entity for >$100k holdings', enabled: false, desc: 'Force corporate account verification for large portfolios' },
    { label: 'Enable dividend reinvestment auto-buy', enabled: false, desc: 'Allow automatic purchase orders from dividend payouts' },
  ];

  function render() {
    // Limits table
    const tbody = document.getElementById('limits-body');
    if (tbody) {
      tbody.innerHTML = limits.map(l => `
        <tr${l.highlight ? ' style="background:rgba(99,102,241,0.06);"' : ''}>
          <td style="font-weight:600; color:var(--admin-text-primary);">${l.tier}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${l.dailyBuy}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${l.dailySell}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${l.maxOrder}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${l.maxHolding}</td>
          <td>${l.p2p ? '<span class="admin-badge admin-badge--success">Yes</span>' : '<span class="admin-badge admin-badge--neutral">No</span>'}</td>
          <td style="text-align:center;"><button class="admin-btn admin-btn--secondary admin-btn--sm btn-edit-limit">Edit</button></td>
        </tr>
      `).join('');

      document.querySelectorAll('.btn-edit-limit').forEach(btn => {
        btn.addEventListener('click', () => {
          mpToast('Limit editor — connect to backend', 'info');
        });
      });
    }

    // Rules toggles
    const rulesBody = document.getElementById('rules-body');
    if (rulesBody) {
      rulesBody.innerHTML = rules.map((r, i) => `
        <div style="display:flex; align-items:center; justify-content:space-between; gap:16px; padding:12px 0; ${i < rules.length - 1 ? 'border-bottom:1px solid var(--admin-border);' : ''}">
          <div style="flex:1;">
            <div style="font-size:14px; font-weight:600; color:var(--admin-text-primary);">${r.label}</div>
            <div style="font-size:12px; color:var(--admin-text-muted); margin-top:2px;">${r.desc}</div>
          </div>
          <label class="mp-toggle" style="flex-shrink:0;">
            <input type="checkbox" ${r.enabled ? 'checked' : ''} data-rule-idx="${i}">
            <span class="mp-toggle-slider"></span>
          </label>
        </div>
      `).join('');

      rulesBody.querySelectorAll('input[type="checkbox"]').forEach(input => {
        input.addEventListener('change', function () {
          const idx = parseInt(this.dataset.ruleIdx);
          rules[idx].enabled = this.checked;
          mpToast(`"${rules[idx].label}" ${this.checked ? 'enabled' : 'disabled'}`, this.checked ? 'success' : 'warning');
        });
      });
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    render();

    document.getElementById('btn-kill-switch')?.addEventListener('click', () => {
      mpModal({
        title: '🛑 HALT ALL TRADING',
        subtitle: 'This is an emergency action. Please confirm you understand the consequences.',
        bodyHTML: `
          <div style="padding:14px; background:var(--admin-danger-bg); border-radius:var(--admin-radius-sm); margin-bottom:12px;">
            <strong style="color:var(--admin-danger);">This will:</strong>
            <ul style="margin:8px 0 0; padding-left:20px; font-size:13px; color:var(--admin-text-secondary); line-height:1.8;">
              <li>Stop the matching engine immediately</li>
              <li>Cancel all pending orders (${18} currently open)</li>
              <li>Release all held balances back to users</li>
              <li>Push notification to all connected WebSocket clients</li>
              <li>Generate an incident report</li>
            </ul>
          </div>
          <div class="admin-form-group">
            <label class="admin-form-label">Type "HALT" to confirm</label>
            <input type="text" class="admin-input" id="kill-confirm" placeholder="HALT">
          </div>
        `,
        confirmLabel: '🛑 HALT TRADING NOW',
        confirmClass: 'admin-btn--danger',
        onConfirm: (overlay) => {
          const val = overlay.querySelector('#kill-confirm')?.value?.trim();
          if (val !== 'HALT') {
            mpToast('Please type HALT to confirm', 'error');
            return;
          }
          mpToast('⛔ TRADING HALTED — All orders cancelled, users notified', 'error');
        }
      });
    });
  });
})();
