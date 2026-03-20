/**
 * Reconciliation — mp-reconciliation.js
 */
(function () {
  'use strict';

  const mismatches = [
    { user: 'USR-3384', asset: 'BVRT', wallet: 120, ledger: 125, diff: -5, cause: 'Concurrent order execution race condition' },
    { user: 'USR-1738', asset: 'JOTX', wallet: 50, ledger: 48, diff: 2, cause: 'Settlement callback delayed — tokens credited early' },
    { user: 'USR-6643', asset: 'SWHS', wallet: 800, ledger: 800.5, diff: -0.5, cause: 'Rounding error in fractional token split' },
  ];

  const history = [
    { time: '2026-03-20 04:00', wallets: 1247, mismatches: 3, duration: '12.4s', status: 'warning' },
    { time: '2026-03-19 04:00', wallets: 1243, mismatches: 0, duration: '11.8s', status: 'ok' },
    { time: '2026-03-18 04:00', wallets: 1238, mismatches: 1, duration: '12.1s', status: 'warning' },
    { time: '2026-03-17 04:00', wallets: 1235, mismatches: 0, duration: '11.5s', status: 'ok' },
    { time: '2026-03-16 04:00', wallets: 1230, mismatches: 0, duration: '11.9s', status: 'ok' },
  ];

  function render() {
    // Mismatches
    const tbody = document.getElementById('recon-body');
    if (tbody) {
      tbody.innerHTML = mismatches.map((m, i) => `
        <tr id="mismatch-${i}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${m.user}</code></td>
          <td style="font-weight:600;">${m.asset}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${m.wallet.toLocaleString()}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${m.ledger.toLocaleString()}</td>
          <td style="text-align:right;">
            <span class="admin-badge admin-badge--danger" style="font-variant-numeric:tabular-nums;">
              ${m.diff > 0 ? '+' : ''}${m.diff}
            </span>
          </td>
          <td style="font-size:12px; color:var(--admin-text-muted); max-width:250px;">${m.cause}</td>
          <td style="text-align:center;">
            <button class="admin-btn admin-btn--success admin-btn--sm btn-resolve-mismatch" data-idx="${i}">Resolve</button>
          </td>
        </tr>
      `).join('');

      document.querySelectorAll('.btn-resolve-mismatch').forEach(btn => {
        btn.addEventListener('click', function () {
          const idx = parseInt(this.dataset.idx);
          mpButtonAction(this, `Mismatch for ${mismatches[idx].user}/${mismatches[idx].asset} resolved`, 1000, () => {
            const row = document.getElementById(`mismatch-${idx}`);
            if (row) {
              row.style.transition = 'opacity 0.3s';
              row.style.opacity = '0.3';
            }
          });
        });
      });
    }

    // History
    const hBody = document.getElementById('recon-history-body');
    if (hBody) {
      hBody.innerHTML = history.map(h => {
        const statusBadge = h.status === 'ok'
          ? '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Clean</span>'
          : '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Mismatches</span>';
        return `
          <tr>
            <td style="font-variant-numeric:tabular-nums; font-size:13px;">${h.time}</td>
            <td style="font-variant-numeric:tabular-nums;">${h.wallets.toLocaleString()}</td>
            <td><span class="admin-badge ${h.mismatches > 0 ? 'admin-badge--warning' : 'admin-badge--success'}">${h.mismatches}</span></td>
            <td style="font-variant-numeric:tabular-nums;">${h.duration}</td>
            <td>${statusBadge}</td>
          </tr>
        `;
      }).join('');
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    render();

    document.getElementById('btn-run-recon')?.addEventListener('click', function () {
      mpButtonAction(this, 'Reconciliation run completed — 1,247 wallets checked', 3000);
    });
  });
})();
